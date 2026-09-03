// Нативное десктоп-приложение: окно рисуется через wgpu/glow, без webview.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod analytics;
mod app;
mod backup;
mod checks;
mod copilot;
mod cpm;
mod db;
mod docgen;
mod domain;
mod i18n;
mod ifc;
mod import;
mod llm;
mod model;
mod notify;
mod package;
mod pdf;
mod portfolio;
mod roles;
mod sales;
mod store;
mod theme;
mod ui;

use app::App;

fn main() -> eframe::Result<()> {
    let path = db::default_db_path();
    let database = match db::Db::open(&path) {
        Ok(d) => d,
        Err(e) => {
            rfd::MessageDialog::new()
                .set_title("QURAi")
                .set_description(format!(
                    "Ma'lumotlar bazasini ochib bo'lmadi / Не удалось открыть базу данных:\n{}\n\n{e}",
                    path.display()
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
            std::process::exit(1);
        }
    };

    // Первый запуск: наполняем демонстрационным объектом, чтобы ГПР было на чем показать.
    // Namuna faqat birinchi ochilishda yaratiladi. Foydalanuvchi uni
    // tozalagan bo'lsa — qaytarib bermaymiz.
    if database.project_count().unwrap_or(0) == 0 && !database.demo_cleared() {
        let _ = database.seed_demo();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 900.0])
            .with_min_inner_size([1100.0, 650.0])
            .with_title("QURAi — Construction Intelligence Platform"),
        ..Default::default()
    };

    eframe::run_native(
        "QURAi",
        options,
        Box::new(|cc| {
            ui::install_fonts(&cc.egui_ctx);
            // Foto galereyasi uchun: `file://` manzillaridan rasm yuklash.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // App::new sozlamalarni o'qib, til va mavzuni global holatga qo'yadi,
            // shundan keyingina uslubni quramiz.
            let app = App::new(database);
            ui::install_theme(&cc.egui_ctx);
            cc.egui_ctx.set_pixels_per_point(app.settings.ui_scale);
            Ok(Box::new(Root { app }))
        }),
    )
}

struct Root {
    app: App,
}

impl eframe::App for Root {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Model javobi qaysi ekranda turgandan qat'i nazar olinadi: so'rov
        // berib boshqa bo'limga o'tilsa ham javob yo'qolmasin.
        if self.app.poll_llm() {
            ctx.request_repaint();
        }
        ui::draw(ctx, &mut self.app);
    }
}
