//! 阿里云 ECS：RPC 风格 API，使用 HMAC-SHA1 正确签名。
//! 修复了原 Go 版直接把明文 AK/SK 放进 Authorization 头导致指标永远失败的 bug。

use std::collections::BTreeMap;
use std::time::Duration;

use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use uuid::Uuid;

use super::http_client;

type HmacSha1 = Hmac<Sha1>;

const API_VERSION: &str = "2014-05-26";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct EcsInstance {
    #[allow(dead_code)] // 保留：后续可扩展显示
    pub instance_id: String,
    pub instance_name: String,
    pub status: String,
    pub public_ip: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    #[allow(dead_code)] // 保留：后续可扩展多 region 显示
    pub region: String,
}

pub struct AliyunCollector {
    region: String,
    ak: String,
    sk: String,
}

impl AliyunCollector {
    pub fn new(region: String, access_key_id: String, access_key_secret: String) -> Self {
        Self {
            region: if region.is_empty() {
                "cn-hangzhou".into()
            } else {
                region
            },
            ak: access_key_id,
            sk: access_key_secret,
        }
    }

    pub fn collect(&self) -> Result<Vec<EcsInstance>, String> {
        let url = build_signed_url(
            &self.region,
            &self.ak,
            &self.sk,
            "DescribeInstances",
            &[
                ("PageSize", "100".to_string()),
                ("PageNumber", "1".to_string()),
            ],
        )?;

        let body = get_text(&url)?;
        let v: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;

        if let Some(code) = v.get("Code").and_then(|c| c.as_str())
            && !code.is_empty() && code != "Success" {
                let msg = v
                    .get("Message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("未知错误");
                return Err(format!("API 错误 [{code}]: {msg}"));
            }

        let mut instances = Vec::new();
        if let Some(list) = v
            .get("Instances")
            .and_then(|x| x.get("Instance"))
            .and_then(|x| x.as_array())
        {
            for item in list {
                let public_ip = item
                    .get("PublicIpAddress")
                    .and_then(|x| x.get("IpAddress"))
                    .and_then(|x| x.as_array())
                    .and_then(|ips| ips.first())
                    .and_then(|ip| ip.as_str())
                    .unwrap_or("")
                    .to_string();

                instances.push(EcsInstance {
                    instance_id: get_str(item, "InstanceId"),
                    instance_name: get_str(item, "InstanceName"),
                    status: get_str(item, "Status"),
                    public_ip,
                    cpu_usage: self.query_metric(&get_str(item, "InstanceId"), "CPUUtilization"),
                    memory_usage: self.query_metric(&get_str(item, "InstanceId"), "MemoryUtilization"),
                    region: self.region.clone(),
                });
            }
        }
        Ok(instances)
    }

    fn query_metric(&self, instance_id: &str, metric: &str) -> f64 {
        if instance_id.is_empty() {
            return 0.0;
        }
        let url = match build_signed_url(
            &self.region,
            &self.ak,
            &self.sk,
            "DescribeInstanceMonitorData",
            &[
                ("InstanceId", instance_id.to_string()),
                ("MetricName", metric.to_string()),
                ("Period", "60".to_string()),
                ("Length", "1".to_string()),
            ],
        ) {
            Ok(u) => u,
            Err(_) => return 0.0,
        };
        let Ok(body) = get_text(&url) else {
            return 0.0;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
            return 0.0;
        };
        v.pointer("/MonitorData/MonitorData/0/Average")
            .and_then(|a| a.as_f64())
            .unwrap_or(0.0)
    }
}

fn get_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn get_text(url: &str) -> Result<String, String> {
    let client = http_client(REQUEST_TIMEOUT, false)?;
    let resp = client.get(url).send().map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("读取响应失败: {e}"))?;
    if !status.is_success() {
        return Err(format!("HTTP 错误 ({status}): {body}"));
    }
    Ok(body)
}

/// 阿里云 RPC 签名（SignatureMethod=HMAC-SHA1, SignatureVersion=1.0）。
/// 返回带完整签名的请求 URL。
fn build_signed_url(
    region: &str,
    ak: &str,
    sk: &str,
    action: &str,
    extra: &[(&str, String)],
) -> Result<String, String> {
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    params.insert("Action".into(), action.into());
    params.insert("Version".into(), API_VERSION.into());
    params.insert("RegionId".into(), region.into());
    params.insert("Format".into(), "JSON".into());
    params.insert("AccessKeyId".into(), ak.into());
    params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
    params.insert("SignatureVersion".into(), "1.0".into());
    params.insert("SignatureNonce".into(), Uuid::new_v4().to_string());
    params.insert(
        "Timestamp".into(),
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    for (k, v) in extra {
        params.insert((*k).to_string(), v.clone());
    }

    let canonical = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let string_to_sign = format!("GET&{}&{}", percent_encode("/"), percent_encode(&canonical));

    let mut mac = HmacSha1::new_from_slice(format!("{sk}&").as_bytes())
        .map_err(|e| format!("初始化签名失败: {e}"))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    params.insert("Signature".into(), signature);
    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    Ok(format!("https://ecs.{region}.aliyuncs.com/?{query}"))
}

/// RFC 3986 百分号编码（空格 → %20）。
fn percent_encode(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}
