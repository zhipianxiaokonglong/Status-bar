mod app;
mod collector;
mod config;
mod credential;
mod settings;
mod tray;

use eframe::egui;

fn main() -> eframe::Result {
    let cfg = config::load().unwrap_or_else(|e| {
        eprintln!("加载配置失败: {e}，使用默认配置");
        config::Config::default()
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Server Status Bar")
            .with_inner_size([400.0, 230.0])
            .with_resizable(false)
            .with_always_on_top(),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Server Status Bar",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::StatusBarApp::new(cfg)))
        }),
    )
}

/// 加载系统中文字体（微软雅黑等），否则 egui 无法渲染中文。
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyhbd.ttc",
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(data).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}
