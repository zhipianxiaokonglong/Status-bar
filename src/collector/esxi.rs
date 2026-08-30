//! VMware ESXi：通过 vim_rs（SOAP/XML 传输）直连 ESXi 采集。
//!
//! ESXi 7.0 已移除 hostd 的 `/rest/` REST API（POST /rest/... 一律 400），
//! 必须走 SOAP `/sdk` 端点（与 govmomi 相同的协议）。
//! 本实现参考 PortalT（govmomi）与 vim_rs 的 SOAP/XML 传输模式。

use std::time::Duration;

use vim_macros::vim_retrievable;
use vim_rs::core::client::TransportMode;
use vim_rs::core::pc_retrieve::ObjectRetriever;
use vim_rs::core::ClientBuilder;
use vim_rs::types::enums::VirtualMachinePowerStateEnum;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

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

// 主机属性（一次 RetrievePropertiesEx 拉取）
vim_retrievable!(
    struct HostProps: HostSystem {
        name = "name",
        cpu_mhz = "summary.hardware.cpu_mhz"?,
        num_cpu_cores = "summary.hardware.num_cpu_cores"?,
        memory_size = "summary.hardware.memory_size"?,
        cpu_usage = "summary.quick_stats.overall_cpu_usage"?,
        memory_usage = "summary.quick_stats.overall_memory_usage"?,
    }
);

// 虚拟机属性（统计运行中数量）
vim_retrievable!(
    struct VmProps: VirtualMachine {
        power_state = "runtime.power_state",
    }
);

/// 采集 ESXi 主机信息与 VM 列表（CPU/内存使用率、运行中 VM 数）。
pub fn collect(raw_url: &str, user: &str, password: &str, insecure: bool) -> Result<EsxiInfo, String> {
    if raw_url.trim().is_empty() {
        return Err("ESXi URL 未配置".into());
    }

    let host = normalize_host(raw_url)?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("创建 tokio 运行时失败: {e}"))?;
    rt.block_on(async move { collect_async(&host, user, password, insecure).await })
}

async fn collect_async(
    host: &str,
    user: &str,
    password: &str,
    insecure: bool,
) -> Result<EsxiInfo, String> {
    // SOAP 会话基于 HTTP cookie（vmware_soap_session），必须启用 cookie_store
    let http = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .cookie_store(true)
        .danger_accept_invalid_certs(insecure)
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let client = ClientBuilder::new(host, http)
        .basic_authn(user, password)
        .app_details("statusbar", env!("CARGO_PKG_VERSION"))
        .transport(TransportMode::Soap)
        .build()
        .await
        .map_err(|e| {
            super::write_log(&format!("ESXi 连接失败: {host} (insecure={insecure}): {e}"));
            format!("连接 ESXi 失败: {e}")
        })?;

    let root_folder = client.service_content().root_folder.clone();
    let retriever = ObjectRetriever::new(client.clone()).map_err(|e| e.to_string())?;

    let mut info = EsxiInfo::default();

    // 主机信息（ESXi 单机：root folder 下第一台 HostSystem）
    let hosts: Vec<HostProps> = retriever
        .retrieve_objects_from_container(&root_folder)
        .await
        .map_err(|e| format!("获取主机信息失败: {e}"))?;

    if let Some(host) = hosts.first() {
        info.host_name = host.name.clone();

        if let Some(memory_size) = host.memory_size {
            info.total_memory = memory_size.max(0) as u64;
        }

        if let (Some(cpu_mhz), Some(num_cores)) = (host.cpu_mhz, host.num_cpu_cores) {
            let total_cpu_mhz = cpu_mhz as i64 * num_cores as i64;
            if total_cpu_mhz > 0 && let Some(usage_mhz) = host.cpu_usage {
                info.cpu_usage = usage_mhz as f64 / total_cpu_mhz as f64 * 100.0;
            }
        }

        // overall_memory_usage 单位为 MB；memory_size 单位为字节
        if let Some(memory_size) = host.memory_size
            && memory_size > 0 && let Some(used_mb) = host.memory_usage {
                info.used_memory = (used_mb as u64).saturating_mul(1024 * 1024);
                info.memory_usage = info.used_memory as f64 / memory_size as f64 * 100.0;
            }
    }

    // VM 列表与运行状态（失败不影响主机数据）
    if let Ok(vms) = retriever
        .retrieve_objects_from_container::<VmProps>(&root_folder)
        .await
    {
        info.total_vms = vms.len();
        info.running_vms = vms
            .iter()
            .filter(|vm| vm.power_state == VirtualMachinePowerStateEnum::PoweredOn)
            .count();
    }

    Ok(info)
}

/// 归一化用户输入的 URL 为 vim_rs 需要的 host[:port]（去掉 scheme 与路径）。
fn normalize_host(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    let without_scheme = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let host_port = without_scheme.split('/').next().unwrap_or("").trim();
    if host_port.is_empty() {
        return Err(format!("ESXi URL 无效: {raw}"));
    }
    Ok(host_port.to_string())
}
