//! VMware ESXi：通过 Host Client REST API（/rest/com/vmware/cis/session）采集。
//! 支持 frp 内网穿透场景（任意 base URL），尊重 config 的 insecure 开关。

use std::time::Duration;

use serde::Deserialize;

use super::http_client;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Default)]
pub struct EsxiInfo {
    pub host_name: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub total_memory: u64,
    pub used_memory: u64,
    pub running_vms: usize,
    pub total_vms: usize,
}

#[derive(Deserialize)]
struct EsxiSession {
    value: String,
}

#[derive(Deserialize)]
struct EsxiHostSummary {
    #[serde(rename = "value")]
    value: EsxiHostSummaryValue,
}

#[derive(Deserialize)]
struct EsxiHostSummaryValue {
    #[serde(rename = "hostHardwareInfo")]
    host_hardware_info: EsxiHostHardwareInfo,
    #[serde(rename = "hostHardwareSummary")]
    host_hardware_summary: Option<EsxiHostHardwareSummary>,
    #[serde(rename = "hostSystemResourceInfo")]
    host_system_resource_info: EsxiHostSystemResourceInfo,
}

#[derive(Deserialize)]
struct EsxiHostHardwareInfo {
    #[serde(rename = "cpuMhz")]
    cpu_mhz: i64,
    #[serde(rename = "numCpuCores")]
    num_cpu_cores: i64,
    #[serde(rename = "memorySize")]
    memory_size: i64,
}

#[derive(Deserialize)]
struct EsxiHostHardwareSummary {
    name: Option<String>,
}

#[derive(Deserialize)]
struct EsxiHostSystemResourceInfo {
    #[serde(rename = "cpuUsage")]
    cpu_usage: Option<EsxiCpuUsage>,
    #[serde(rename = "memoryUsage")]
    memory_usage: Option<EsxiMemoryUsage>,
}

#[derive(Deserialize)]
struct EsxiCpuUsage {
    #[serde(rename = "usageMhz")]
    usage_mhz: Option<i64>,
}

#[derive(Deserialize)]
struct EsxiMemoryUsage {
    usage: Option<i64>,
}

#[derive(Deserialize)]
struct EsxiVmList {
    #[serde(rename = "value")]
    value: Vec<EsxiVmItem>,
}

#[derive(Deserialize)]
struct EsxiVmItem {
    #[serde(rename = "vm")]
    vm: EsxiVm,
}

#[derive(Deserialize)]
struct EsxiVm {
    #[serde(rename = "powerState")]
    power_state: Option<String>,
}

pub fn collect(raw_url: &str, user: &str, password: &str, insecure: bool) -> Result<EsxiInfo, String> {
    if raw_url.trim().is_empty() {
        return Err("ESXi URL 未配置".into());
    }

    let base = normalize_base(raw_url);
    let client = http_client(REQUEST_TIMEOUT, insecure)?;

    // 登录拿 session id
    // 注意：VMware vAPI 要求 POST 携带 Content-Type: application/json，
    // 缺失时 hostd 直接返回 400 Bad Request（原 Go 版有此头，Rust 版曾遗漏）。
    let session_url = format!("{base}/rest/com/vmware/cis/session");
    let resp = client
        .post(&session_url)
        .header("Content-Type", "application/json")
        .basic_auth(user, Some(password))
        .send()
        .map_err(|e| format!("登录请求失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        super::write_log(&format!(
            "ESXi 登录失败: {session_url} (HTTP {status}) body: {body}"
        ));
        return Err(format!("登录失败 (HTTP {status}): {}", truncate(&body)));
    }
    let session: EsxiSession =
        serde_json::from_str(&resp.text().map_err(|e| e.to_string())?)
            .map_err(|e| format!("解析登录响应失败: {e}"))?;

    let mut info = EsxiInfo::default();

    // 主机摘要
    let summary_body = api_get(&client, &base, &session.value, "/rest/vmware/host/summary")?;
    let summary: EsxiHostSummary = serde_json::from_str(&summary_body)
        .map_err(|e| format!("解析主机摘要失败: {e}"))?;

    let hw = &summary.value.host_hardware_info;
    info.total_memory = hw.memory_size.max(0) as u64;

    let total_cpu_mhz = hw.cpu_mhz.saturating_mul(hw.num_cpu_cores);
    if let Some(usage) = summary
        .value
        .host_system_resource_info
        .cpu_usage
        .as_ref()
        .and_then(|c| c.usage_mhz)
        && total_cpu_mhz > 0 {
            info.cpu_usage = usage as f64 / total_cpu_mhz as f64 * 100.0;
        }

    if let Some(mem) = summary.value.host_system_resource_info.memory_usage.as_ref()
        && let Some(used) = mem.usage {
            info.used_memory = used.max(0) as u64;
            if hw.memory_size > 0 {
                info.memory_usage = used as f64 / hw.memory_size as f64 * 100.0;
            }
        }

    if let Some(name) = summary
        .value
        .host_hardware_summary
        .as_ref()
        .and_then(|s| s.name.clone())
    {
        info.host_name = name;
    }

    // VM 列表（失败不影响主机数据）
    if let Ok(vm_body) = api_get(&client, &base, &session.value, "/rest/vmware/host/vm-list")
        && let Ok(list) = serde_json::from_str::<EsxiVmList>(&vm_body) {
            info.total_vms = list.value.len();
            for item in &list.value {
                if item
                    .vm
                    .power_state
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case("POWERED_ON"))
                {
                    info.running_vms += 1;
                }
            }
        }

    // 数据采集完成后登出（尽力而为，先采集后登出：登出会终止会话）
    let _ = client
        .delete(&session_url)
        .header("vmware-api-session-id", &session.value)
        .send();

    Ok(info)
}

fn api_get(client: &reqwest::blocking::Client, base: &str, session_id: &str, path: &str) -> Result<String, String> {
    let url = format!("{base}{path}");
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .header("vmware-api-session-id", session_id)
        .send()
        .map_err(|e| format!("请求 {path} 失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("读取响应失败: {e}"))?;
    if !status.is_success() {
        super::write_log(&format!(
            "ESXi 请求失败: {url} (HTTP {status}) body: {body}"
        ));
        return Err(format!("请求 {path} 失败 (HTTP {status}): {}", truncate(&body)));
    }
    Ok(body)
}

/// 截断错误响应体，避免超长/含敏感信息的响应直接回显到界面。
fn truncate(s: &str) -> String {
    let preview: String = s.chars().take(200).collect();
    if s.chars().count() > 200 {
        format!("{preview}…")
    } else {
        preview
    }
}

/// 归一化 base URL：补 https 前缀、去掉路径，只保留 scheme://host[:port]。
fn normalize_base(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if !s.starts_with("http://") && !s.starts_with("https://") {
        s = format!("https://{s}");
    }
    if let Some(scheme_end) = s.find("://") {
        let after = &s[scheme_end + 3..];
        if let Some(slash) = after.find('/') {
            s.truncate(scheme_end + 3 + slash);
        }
    }
    s.trim_end_matches('/').to_string()
}
