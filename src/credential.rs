//! 凭证仅保存在内存中（App 持有的字段），绝不写入磁盘。
//! 进程退出后内存自然释放，无持久化痕迹。

#[derive(Debug, Clone, Default)]
pub struct AliyunCred {
    pub access_key_id: String,
    pub access_key_secret: String,
}

#[derive(Debug, Clone, Default)]
pub struct EsxiCred {
    pub url: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone, Default)]
pub struct DeepSeekCred {
    pub api_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct Credentials {
    pub aliyun: Option<AliyunCred>,
    pub esxi: Option<EsxiCred>,
    pub deepseek: Option<DeepSeekCred>,
}

impl Credentials {
    pub fn has_aliyun(&self) -> bool {
        self.aliyun
            .as_ref()
            .is_some_and(|c| !c.access_key_id.is_empty() && !c.access_key_secret.is_empty())
    }

    pub fn has_esxi(&self) -> bool {
        self.esxi
            .as_ref()
            .is_some_and(|c| !c.url.is_empty() && !c.user.is_empty() && !c.password.is_empty())
    }

    pub fn has_deepseek(&self) -> bool {
        self.deepseek.as_ref().is_some_and(|c| !c.api_key.is_empty())
    }
}
