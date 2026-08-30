//! 主应用：egui 界面、5 秒自动刷新（后台线程采集，结果回传 UI 线程）、托盘事件。

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::collector::{aliyun, deepseek, esxi};
use crate::config::Config;
use crate::credential::Credentials;
use crate::settings::{self, SettingsDraft};
use crate::tray::{self, Tray, TrayCommand};

const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

enum RefreshMsg {
    Aliyun(Result<Vec<aliyun::EcsInstance>, String>),
    Esxi(Result<esxi::EsxiInfo, String>),
    DeepSeek(Result<deepseek::DeepSeekBalance, String>),
}

pub struct StatusBarApp {
    cfg: Config,
    creds: Credentials,

    open_settings: bool,
    settings_draft: Option<SettingsDraft>,

    aliyun_text: String,
    esxi_text: String,
    ds_text: String,
    status_text: String,

    last_refresh: Instant,
    aliyun_inflight: bool,
    esxi_inflight: bool,
    ds_inflight: bool,
    tx: Sender<RefreshMsg>,
    rx: Receiver<RefreshMsg>,

    tray: Option<Tray>,
    visible: bool,
}

impl StatusBarApp {
    pub fn new(cfg: Config) -> Self {
        let (tx, rx) = mpsc::channel();
        let tray = tray::create(&cfg).ok();
        Self {
            cfg,
            creds: Credentials::default(),
            open_settings: false,
            settings_draft: None,
            aliyun_text: "  阿里云: [未配置]".into(),
            esxi_text: "  ESXi: [未配置]".into(),
            ds_text: "  DeepSeek: [未配置]".into(),
            status_text: "  上次刷新: --:--:--".into(),
            last_refresh: Instant::now() - REFRESH_INTERVAL,
            aliyun_inflight: false,
            esxi_inflight: false,
            ds_inflight: false,
            tx,
            rx,
            tray,
            visible: true,
        }
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        // 先收集命令，避免托盘借用与 &mut self 冲突
        let mut cmds = Vec::new();
        if let Some(tray) = &self.tray {
            while let Some(cmd) = tray.poll_events() {
                cmds.push(cmd);
            }
        }
        for cmd in cmds {
            match cmd {
                TrayCommand::ToggleVisible => {
                    self.visible = !self.visible;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.visible));
                }
                TrayCommand::OpenSettings => self.open_settings(),
                TrayCommand::ToggleAliyun => self.toggle_module("aliyun"),
                TrayCommand::ToggleEsxi => self.toggle_module("esxi"),
                TrayCommand::ToggleDeepSeek => self.toggle_module("deepseek"),
                TrayCommand::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn open_settings(&mut self) {
        if self.settings_draft.is_none() {
            self.settings_draft = Some(SettingsDraft::from_creds(&self.creds));
        }
        self.open_settings = true;
    }

    fn toggle_module(&mut self, module: &str) {
        let new_val = match module {
            "aliyun" => {
                self.cfg.display.show_aliyun = !self.cfg.display.show_aliyun;
                self.cfg.display.show_aliyun
            }
            "esxi" => {
                self.cfg.display.show_esxi = !self.cfg.display.show_esxi;
                self.cfg.display.show_esxi
            }
            "deepseek" => {
                self.cfg.display.show_deepseek = !self.cfg.display.show_deepseek;
                self.cfg.display.show_deepseek
            }
            _ => return,
        };
        let _ = crate::config::save(&self.cfg);
        if let Some(tray) = &self.tray {
            tray.set_module_checked(module, new_val);
        }
        self.trigger_refresh();
    }

    /// 在后台线程执行采集；catch_unwind 保护，线程 panic 也会复位 in-flight 并回报错误。
    fn spawn_collect<T: Send + 'static>(
        &self,
        f: impl FnOnce() -> Result<T, String> + Send + 'static,
        wrap: impl FnOnce(Result<T, String>) -> RefreshMsg + Send + 'static,
    ) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
                    Err("采集线程发生内部错误".to_string())
                });
            let _ = tx.send(wrap(result));
        });
    }

    /// 按配置触发各模块的后台采集（防堆积：in-flight 时跳过）。
    fn trigger_refresh(&mut self) {
        let now = chrono::Local::now().format("%H:%M:%S").to_string();

        if self.cfg.display.show_aliyun {
            if self.creds.has_aliyun() && !self.aliyun_inflight {
                let Some(cred) = self.creds.aliyun.clone() else {
                    return;
                };
                let region = self.cfg.aliyun.region.clone();
                self.aliyun_inflight = true;
                self.spawn_collect(
                    move || {
                        let c = aliyun::AliyunCollector::new(region, cred.access_key_id, cred.access_key_secret)?;
                        c.collect()
                    },
                    RefreshMsg::Aliyun,
                );
            } else if !self.creds.has_aliyun() {
                self.aliyun_text = "  阿里云: [未配置]".to_string();
            }
        } else {
            self.aliyun_text = "  阿里云: [已停用]".to_string();
        }

        if self.cfg.display.show_esxi {
            if self.creds.has_esxi() && !self.esxi_inflight {
                let Some(cred) = self.creds.esxi.clone() else {
                    return;
                };
                let insecure = self.cfg.esxi.insecure;
                self.esxi_inflight = true;
                self.spawn_collect(
                    move || esxi::collect(&cred.url, &cred.user, &cred.password, insecure),
                    RefreshMsg::Esxi,
                );
            } else if !self.creds.has_esxi() {
                self.esxi_text = "  ESXi: [未配置]".to_string();
            }
        } else {
            self.esxi_text = "  ESXi: [已停用]".to_string();
        }

        if self.cfg.display.show_deepseek {
            if self.creds.has_deepseek() && !self.ds_inflight {
                let Some(cred) = self.creds.deepseek.clone() else {
                    return;
                };
                let base_url = self.cfg.deepseek.base_url.clone();
                self.ds_inflight = true;
                self.spawn_collect(
                    move || deepseek::collect(&cred.api_key, &base_url),
                    RefreshMsg::DeepSeek,
                );
            } else if !self.creds.has_deepseek() {
                self.ds_text = "  DeepSeek: [未配置]".to_string();
            }
        } else {
            self.ds_text = "  DeepSeek: [已停用]".to_string();
        }

        self.status_text = format!("  上次刷新: {now}");
    }

    fn collect_results(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                RefreshMsg::Aliyun(Ok(instances)) => {
                    self.aliyun_inflight = false;
                    self.aliyun_text = match instances.first() {
                        None => "  阿里云: 无实例".to_string(),
                        Some(inst) => {
                            let extra = if instances.len() > 1 {
                                format!(" (+{} 台)", instances.len() - 1)
                            } else {
                                String::new()
                            };
                            let ip = if inst.public_ip.is_empty() {
                                String::new()
                            } else {
                                format!(" ({})", inst.public_ip)
                            };
                            format!(
                                "  ECS: {}{} | CPU: {:.1}% | 内存: {:.1}% | {}{}",
                                inst.instance_name, ip, inst.cpu_usage, inst.memory_usage, inst.status, extra
                            )
                        }
                    };
                }
                RefreshMsg::Aliyun(Err(e)) => {
                    self.aliyun_inflight = false;
                    self.aliyun_text = format!("  阿里云: {e}");
                }
                RefreshMsg::Esxi(Ok(info)) => {
                    self.esxi_inflight = false;
                    self.esxi_text = format!(
                        "  ESXi: {} | CPU: {:.1}% | 内存: {:.1}% | VM: {}/{}",
                        info.host_name, info.cpu_usage, info.memory_usage, info.running_vms, info.total_vms
                    );
                }
                RefreshMsg::Esxi(Err(e)) => {
                    self.esxi_inflight = false;
                    self.esxi_text = format!("  ESXi: {e}");
                }
                RefreshMsg::DeepSeek(Ok(b)) => {
                    self.ds_inflight = false;
                    let unit = if b.currency == "CNY" { "¥" } else { "" };
                    // DeepSeek 官方 /user/balance 响应不含 total_consumed，
                    // 仅在有该字段（兼容其他网关）时才显示"已用"。
                    if b.total_used > 0.0 {
                        self.ds_text = format!(
                            "  DeepSeek: 余额 {unit}{:.2} | 已用 {unit}{:.2}",
                            b.balance, b.total_used
                        );
                    } else {
                        self.ds_text = format!("  DeepSeek: 余额 {unit}{:.2}", b.balance);
                    }
                }
                RefreshMsg::DeepSeek(Err(e)) => {
                    self.ds_inflight = false;
                    self.ds_text = format!("  DeepSeek: {e}");
                }
            }
        }
    }

}

impl eframe::App for StatusBarApp {
    /// 每帧之前调用（窗口隐藏时也调用）：处理托盘事件、收集后台采集结果、定时刷新。
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray_events(ctx);
        self.collect_results();

        if self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.last_refresh = Instant::now();
            self.trigger_refresh();
        }

        // 调度下一帧：空闲/窗口隐藏时也保持定时刷新与托盘事件轮询
        ctx.request_repaint_after(REFRESH_INTERVAL);
    }

    /// 绘制界面。
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(2.0);
            ui.vertical_centered(|ui| {
                ui.heading("Server Monitor");
            });
            ui.add_space(8.0);
            ui.label(&self.aliyun_text);
            ui.label(&self.esxi_text);
            ui.label(&self.ds_text);
            ui.label(&self.status_text);
            ui.add_space(6.0);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("设置").clicked() {
                    self.open_settings();
                }
            });
        });

        let ctx = ui.ctx().clone();
        let mut saved = false;
        if self.open_settings {
            if let Some(draft) = self.settings_draft.as_mut() {
                if settings::show(draft, &ctx, &mut self.open_settings) {
                    saved = true;
                }
            } else {
                self.open_settings = false;
            }
        }

        if saved {
            if let Some(draft) = &self.settings_draft {
                self.creds = draft.to_creds();
            }
            self.settings_draft = None;
            self.trigger_refresh();
        }
    }
}
