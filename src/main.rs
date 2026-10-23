// Нативное десктоп-приложение: окно рисуется через wgpu/glow, без webview.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Brauzer yig'ilishida desktopga xos yo'llar (ip, disk, tashqi dastur)
// ishlatilmaydi — ular uchun «ishlatilmagan» ogohlantirishi kutilgan.
#![cfg_attr(
    target_arch = "wasm32",
    allow(dead_code, unreachable_code, unused_variables, unused_imports)
)]

mod actions;
mod aitake;
mod analytics;
mod app;
mod attend;
mod backup;
mod calc;
mod checks;
mod clash;
mod copilot;
mod cpm;
mod db;
mod docgen;
mod docx;
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
mod plat;
mod portfolio;
mod prices;
mod qr;
mod reports;
mod roles;
mod sales;
mod signlog;
mod smeta;
mod smeta_ai;
mod store;
mod sync;
mod takeoff;
mod theme;
mod ui;

use app::App;

#[cfg(not(target_arch = "wasm32"))]
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
            ui::apply_scale(&cc.egui_ctx, app.settings.ui_scale);
            Ok(Box::new(Root { app }))
        }),
    )
}

/// Brauzer kirish nuqtasi: baza IndexedDB da, ilova `#qurai_canvas` ga
/// chiziladi. Sahifa — `web/index.html`.
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    console_error_panic_hook_lite();
    wasm_bindgen_futures::spawn_local(async {
        let status = |msg: &str| {
            if let Some(el) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("qurai_status"))
            {
                el.set_text_content(Some(msg));
            }
        };
        // IndexedDB VFS — sukut VFS; baza fayli brauzerda saqlanadi.
        let cfg = sqlite_wasm_vfs::relaxed_idb::RelaxedIdbCfg::default();
        if let Err(e) =
            sqlite_wasm_vfs::relaxed_idb::install::<sqlite_wasm_rs::WasmOsCallback>(&cfg, true)
                .await
        {
            status(&format!("IndexedDB: {e:?}"));
            return;
        }
        let database = match db::Db::open(&std::path::PathBuf::from("qurai.db")) {
            Ok(d) => d,
            Err(e) => {
                status(&format!("Baza ochilmadi / База не открылась: {e}"));
                return;
            }
        };
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("qurai_canvas"))
            .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok());
        let Some(canvas) = canvas else {
            status("canvas topilmadi");
            return;
        };
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    ui::install_fonts(&cc.egui_ctx);
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    let app = App::new(database);
                    ui::install_theme(&cc.egui_ctx);
                    ui::apply_scale(&cc.egui_ctx, app.settings.ui_scale);
                    Ok(Box::new(Root { app }))
                }),
            )
            .await;
        match started {
            Ok(()) => {
                if let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("qurai_loading"))
                {
                    el.remove();
                }
            }
            Err(e) => status(&format!("Ishga tushmadi / Не запустилось: {e:?}")),
        }
    });
}

/// Vahima matnini brauzer konsoliga chiqaradi — aks holda xato
/// «unreachable» bo'lib ko'rinardi.
#[cfg(target_arch = "wasm32")]
fn console_error_panic_hook_lite() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("QURAi: {info}").into());
    }));
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
        crate::plat::fix_canvas_size();
        ui::draw(ctx, &mut self.app);
    }
}
