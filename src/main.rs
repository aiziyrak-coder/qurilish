// Нативное десктоп-приложение: окно рисуется через wgpu/glow, без webview.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod analytics;
mod app;
mod attend;
mod backup;
mod checks;
mod clash;
mod copilot;
mod cpm;
mod db;
mod docgen;
mod domain;
mod dxf;
mod exif;
mod geo;
mod hook;
mod i18n;
mod ifc;
mod import;
mod llm;
mod model;
mod notify;
mod ocr;
mod package;
mod pdf;
mod pdfread;
mod photocheck;
mod portfolio;
mod prices;
mod qr;
mod reports;
mod roles;
mod sales;
mod signlog;
mod store;
mod sync;
mod takeoff;
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

    // Namuna ma'lumoti **o'zi yaratilmaydi**: ilova bo'sh ochiladi va
    // birinchi obyektni foydalanuvchining o'zi kiritadi.
    //
    // Avval birinchi ochilishda namoyish obyekti yaratilardi. U ilovani
    // ko'rsatish uchun qulay edi, lekin haqiqiy ishda chalkashtirardi:
    // bazada o'ylab topilgan obyekt turar, uning sonlari hisobotlarga
    // tushar va «bu qayerdan keldi?» degan savol tug'ilardi.
    //
    // Namuna kerak bo'lsa — «Sozlamalar → Namuna obyektini yaratish».
    // Bu ataylab qilinadigan ish va uni bir bosishda qaytarib o'chirish
    // ham mumkin.

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
        // Dastur sutkalab ochiq turishi mumkin — ofisdagi kompyuter
        // o'chirilmaydi. «Bugun» faqat ishga tushishda olinganda, yarim
        // tundan keyin muddat, kechikish va kunlik hisobot kechagi kunda
        // qolib ketardi.
        self.app.refresh_today();
        ui::draw(ctx, &mut self.app);
    }
}
