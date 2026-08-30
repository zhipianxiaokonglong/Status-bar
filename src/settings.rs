//! 设置窗口：多 Tab（阿里云 / ESXi / DeepSeek），密码字段支持粘贴。
//! 保存时仅更新内存中的凭证，不落盘。

use eframe::egui;

use crate::credential::Credentials;

const TABS: [&str; 3] = ["阿里云", "ESXi", "DeepSeek"];

/// 设置窗口的编辑缓冲区（打开时从现有凭证初始化，关闭时丢弃）。
#[derive(Debug, Default)]
pub struct SettingsDraft {
    pub tab: usize,
    pub aliyun_id: String,
    pub aliyun_secret: String,
    pub esxi_url: String,
    pub esxi_user: String,
    pub esxi_pass: String,
    pub ds_key: String,
}

impl SettingsDraft {
    pub fn from_creds(creds: &Credentials) -> Self {
        let mut d = Self::default();
        if let Some(c) = &creds.aliyun {
            d.aliyun_id = c.access_key_id.clone();
            d.aliyun_secret = c.access_key_secret.clone();
        }
        if let Some(c) = &creds.esxi {
            d.esxi_url = c.url.clone();
            d.esxi_user = c.user.clone();
            d.esxi_pass = c.password.clone();
        }
        if let Some(c) = &creds.deepseek {
            d.ds_key = c.api_key.clone();
        }
        d
    }

    pub fn to_creds(&self) -> Credentials {
        let aliyun = if !self.aliyun_id.trim().is_empty() && !self.aliyun_secret.trim().is_empty() {
            Some(crate::credential::AliyunCred {
                access_key_id: self.aliyun_id.trim().to_string(),
                access_key_secret: self.aliyun_secret.trim().to_string(),
            })
        } else {
            None
        };
        let esxi = if !self.esxi_url.trim().is_empty()
            && !self.esxi_user.trim().is_empty()
            && !self.esxi_pass.trim().is_empty()
        {
            Some(crate::credential::EsxiCred {
                url: self.esxi_url.trim().to_string(),
                user: self.esxi_user.trim().to_string(),
                password: self.esxi_pass.trim().to_string(),
            })
        } else {
            None
        };
        let deepseek = if !self.ds_key.trim().is_empty() {
            Some(crate::credential::DeepSeekCred {
                api_key: self.ds_key.trim().to_string(),
            })
        } else {
            None
        };
        Credentials {
            aliyun,
            esxi,
            deepseek,
        }
    }
}

/// 返回 true 表示用户点击了"保存"（调用方需更新凭证并触发刷新）。
pub fn show(
    draft: &mut SettingsDraft,
    ctx: &egui::Context,
    open: &mut bool,
) -> bool {
    let mut saved = false;

    egui::Window::new("账户与密钥设置")
        .collapsible(false)
        .resizable(false)
        .fixed_size([420.0, 320.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, tab) in TABS.iter().enumerate() {
                    if ui.selectable_label(draft.tab == i, *tab).clicked() {
                        draft.tab = i;
                    }
                }
            });
            ui.separator();

            match draft.tab {
                0 => {
                    ui.label("AccessKey ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.aliyun_id)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label("AccessKey Secret:");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.aliyun_secret)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                }
                1 => {
                    ui.label("访问地址 (如 https://192.168.1.100:443):");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.esxi_url)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label("用户名:");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.esxi_user)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label("密码:");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.esxi_pass)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                }
                _ => {
                    ui.label("API Key:");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.ds_key)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        saved = true;
                        *open = false;
                    }
                    if ui.button("取消").clicked() {
                        *open = false;
                    }
                });
            });
        });

    saved
}
