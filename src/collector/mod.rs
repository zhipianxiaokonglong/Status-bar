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

/// 追加诊断日志到 exe 同目录 statusbar.log（尽力而为，失败静默）。
/// UI 上的错误只显示截断摘要，完整细节写入此文件便于排查。
pub fn write_log(msg: &str) {
    use std::io::Write;

    let Some(mut path) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
    else {
        return;
    };
    path.push("statusbar.log");
    let line = format!(
        "[{}] {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        msg
    );
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}
