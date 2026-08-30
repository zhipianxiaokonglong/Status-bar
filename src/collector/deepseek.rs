//! DeepSeek 账户余额：GET {base_url}/user/balance。

use std::time::Duration;

use serde_json::Value;

use super::http_client;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Default)]
pub struct DeepSeekBalance {
    pub balance: f64,
    pub total_used: f64,
    pub currency: String,
}

pub fn collect(api_key: &str, base_url: &str) -> Result<DeepSeekBalance, String> {
    if api_key.is_empty() {
        return Err("DeepSeek API Key 未配置".into());
    }
    let base = base_url.trim_end_matches('/');
    let client = http_client(REQUEST_TIMEOUT, false)?;
    let resp = client
        .get(format!("{base}/user/balance"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .send()
        .map_err(|e| format!("请求余额失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("读取响应失败: {e}"))?;
    if !status.is_success() {
        return Err(format!("API 错误 (HTTP {status}): {body}"));
    }

    let v: Value = serde_json::from_str(&body).map_err(|e| format!("解析响应失败: {e}"))?;
    let infos = v
        .get("balance_infos")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "响应中缺少 balance_infos".to_string())?;

    // 优先取 CNY 或 quota_type=balance 的条目，回退到第一个
    for info in infos {
        let currency = info.get("currency").and_then(|c| c.as_str()).unwrap_or("");
        let quota_type = info.get("quota_type").and_then(|c| c.as_str()).unwrap_or("");
        if currency == "CNY" || quota_type == "balance" {
            return Ok(balance_from(info, currency));
        }
    }
    if let Some(first) = infos.first() {
        let currency = first.get("currency").and_then(|c| c.as_str()).unwrap_or("");
        return Ok(balance_from(first, currency));
    }
    Err("无余额数据".into())
}

fn balance_from(info: &Value, currency: &str) -> DeepSeekBalance {
    DeepSeekBalance {
        balance: parse_f64(info.get("total_balance")),
        total_used: parse_f64(info.get("total_consumed")),
        currency: currency.to_string(),
    }
}

fn parse_f64(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}
