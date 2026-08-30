pub mod aliyun;
pub mod deepseek;
pub mod esxi;

use std::time::Duration;

use reqwest::blocking::Client;

/// 构造带超时（可选跳过 TLS 校验）的 blocking HTTP 客户端。
pub fn http_client(timeout: Duration, insecure: bool) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(timeout);
    if insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }
    builder.build().map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}
