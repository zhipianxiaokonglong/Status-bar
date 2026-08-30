//! 非敏感配置：仅存 region / insecure / base_url / 模块显隐开关。
//! 凭证永远不落盘，见 credential.rs。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub aliyun: AliyunConfig,
    pub esxi: EsxiConfig,
    pub deepseek: DeepSeekConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunConfig {
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsxiConfig {
    pub insecure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekConfig {
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub show_aliyun: bool,
    pub show_esxi: bool,
    pub show_deepseek: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aliyun: AliyunConfig {
                region: "cn-hangzhou".into(),
            },
            esxi: EsxiConfig { insecure: true },
            deepseek: DeepSeekConfig {
                base_url: "https://api.deepseek.com".into(),
            },
            display: DisplayConfig {
                show_aliyun: true,
                show_esxi: true,
                show_deepseek: true,
            },
        }
    }
}

/// config.json 位于可执行文件同目录。
pub fn config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("config.json")))
        .unwrap_or_else(|| PathBuf::from("config.json"))
}

pub fn load() -> Result<Config, String> {
    let path = config_path();
    if !path.exists() {
        let cfg = Config::default();
        // 首次运行生成默认配置（写入失败不致命）
        let _ = save(&cfg);
        return Ok(cfg);
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("解析配置失败: {e}"))
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let data = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), data).map_err(|e| format!("写入配置失败: {e}"))
}
