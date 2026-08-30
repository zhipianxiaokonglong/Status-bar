//! 系统托盘：图标 + 右键菜单（显示/隐藏、设置、各模块开关、退出）。
//! 菜单事件通过 muda 的全局 receiver 轮询。

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ToggleVisible,
    OpenSettings,
    ToggleAliyun,
    ToggleEsxi,
    ToggleDeepSeek,
    Quit,
}

pub struct Tray {
    _icon: TrayIcon,
    aliyun_item: CheckMenuItem,
    esxi_item: CheckMenuItem,
    ds_item: CheckMenuItem,
}

pub fn create(cfg: &Config) -> Result<Tray, String> {
    let menu = Menu::new();

    let toggle = MenuItem::with_id("toggle", "显示/隐藏", true, None);
    let settings = MenuItem::with_id("settings", "设置...", true, None);
    let aliyun_item = CheckMenuItem::with_id("aliyun", "阿里云", true, false, None);
    let esxi_item = CheckMenuItem::with_id("esxi", "ESXi", true, false, None);
    let ds_item = CheckMenuItem::with_id("deepseek", "DeepSeek", true, false, None);
    let quit = MenuItem::with_id("quit", "退出", true, None);

    aliyun_item.set_checked(cfg.display.show_aliyun);
    esxi_item.set_checked(cfg.display.show_esxi);
    ds_item.set_checked(cfg.display.show_deepseek);

    menu.append(&toggle).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&settings).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&aliyun_item).map_err(|e| e.to_string())?;
    menu.append(&esxi_item).map_err(|e| e.to_string())?;
    menu.append(&ds_item).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator())
        .map_err(|e| e.to_string())?;
    menu.append(&quit).map_err(|e| e.to_string())?;

    let icon = make_icon()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Server Status Bar")
        .with_icon(icon)
        .build()
        .map_err(|e| format!("创建托盘失败: {e}"))?;

    Ok(Tray {
        _icon: tray,
        aliyun_item,
        esxi_item,
        ds_item,
    })
}

impl Tray {
    pub fn set_module_checked(&self, module: &str, checked: bool) {
        let item = match module {
            "aliyun" => &self.aliyun_item,
            "esxi" => &self.esxi_item,
            "deepseek" => &self.ds_item,
            _ => return,
        };
        item.set_checked(checked);
    }

    /// 轮询托盘事件（右键菜单 + 左键单击图标），返回待处理的命令。
    pub fn poll_events(&self) -> Option<TrayCommand> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            let cmd = match event.id().0.as_str() {
                "toggle" => Some(TrayCommand::ToggleVisible),
                "settings" => Some(TrayCommand::OpenSettings),
                "aliyun" => Some(TrayCommand::ToggleAliyun),
                "esxi" => Some(TrayCommand::ToggleEsxi),
                "deepseek" => Some(TrayCommand::ToggleDeepSeek),
                "quit" => Some(TrayCommand::Quit),
                _ => None,
            };
            if cmd.is_some() {
                return cmd;
            }
        }
        if let Ok(event) = TrayIconEvent::receiver().try_recv()
            && let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                return Some(TrayCommand::ToggleVisible);
            }
        None
    }
}

/// 生成一个 16x16 的简单 RGBA 图标（无需外部资源文件）。
fn make_icon() -> Result<Icon, String> {
    const SIZE: u32 = 16;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _y in 0..SIZE {
        for x in 0..SIZE {
            // 左半蓝右半绿，模拟"状态栏"分栏
            let (r, g, b) = if x < SIZE / 2 {
                (0x2b, 0x87, 0xf6)
            } else {
                (0x00, 0xc8, 0x53)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).map_err(|e| format!("生成托盘图标失败: {e}"))
}
