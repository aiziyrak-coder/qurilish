//! Sozlamalar sahifasi: til, mavzu, masshtab va standart qiymatlar.

use super::*;
use crate::app::{GanttScale, Screen};
use crate::i18n::Lang;
use crate::theme::Theme;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;
    let mut make_demo = false;
    let mut make_backup = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading(t("settings_title"));
        ui.add_space(10.0);

        let w = (ui.available_width() - 26.0).min(820.0);

        // ---------- Interfeys ----------
        card_frame(ui, t("set_group_ui"), w, |ui| {
            field(ui, t("set_language"), |ui| {
                for l in Lang::ALL {
                    let sel = app.settings.lang == l;
                    if ui.selectable_label(sel, l.native_name()).clicked() && !sel {
                        app.settings.lang = l;
                        changed = true;
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(210.0);
                ui.label(
                    RichText::new(t("set_language_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });

            ui.add_space(6.0);
            field(ui, t("set_theme"), |ui| {
                for th in Theme::ALL {
                    let sel = app.settings.theme == th;
                    if ui.selectable_label(sel, th.label()).clicked() && !sel {
                        app.settings.theme = th;
                        changed = true;
                    }
                }
            });

            ui.add_space(6.0);
            field(ui, t("set_scale"), |ui| {
                let mut pct = (app.settings.ui_scale * 100.0).round() as i32;
                if ui
                    .add(
                        egui::Slider::new(&mut pct, 80..=160)
                            .suffix(" %")
                            .step_by(5.0),
                    )
                    .changed()
                {
                    app.settings.ui_scale = pct as f32 / 100.0;
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(210.0);
                ui.label(
                    RichText::new(t("set_scale_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });
        });

        ui.add_space(10.0);

        // ---------- Ishlar grafigi ----------
        card_frame(ui, t("set_group_gantt"), w, |ui| {
            field(ui, t("set_default_scale"), |ui| {
                for sc in GanttScale::ALL {
                    let sel = app.settings.default_scale == sc;
                    if ui.selectable_label(sel, sc.label()).clicked() && !sel {
                        app.settings.default_scale = sc;
                        app.scale = sc;
                        app.px_per_day = sc.px_per_day();
                        changed = true;
                    }
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(210.0);
                if ui
                    .checkbox(&mut app.settings.show_weekends, t("set_show_weekends"))
                    .changed()
                {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(210.0);
                ui.label(
                    RichText::new(t("set_show_weekends_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });
        });

        ui.add_space(10.0);

        // ---------- Standart qiymatlar ----------
        card_frame(ui, t("set_group_defaults"), w, |ui| {
            field(ui, t("set_currency"), |ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut app.settings.default_currency)
                            .desired_width(100.0),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    RichText::new(t("set_currency_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });

            field(ui, t("set_task_duration"), |ui| {
                if ui
                    .add(egui::DragValue::new(&mut app.settings.default_task_days).range(1..=365))
                    .changed()
                {
                    changed = true;
                }
            });
        });

        ui.add_space(10.0);

        // ---------- Ma'lumotlar ----------
        card_frame(ui, t("set_group_data"), w, |ui| {
            let path = crate::db::default_db_path();
            field(ui, t("set_db_path"), |ui| {
                ui.label(
                    RichText::new(path.display().to_string())
                        .size(12.0)
                        .color(theme::text()),
                );
            });
            field(ui, "", |ui| {
                if ui.button(t("set_open_folder")).clicked() {
                    if let Some(dir) = path.parent() {
                        // Windows uchun explorer, boshqa tizimlarda xohishga ko'ra.
                        let _ = std::process::Command::new("explorer").arg(dir).spawn();
                    }
                }
            });
            field(ui, t("set_objects_count"), |ui| {
                ui.label(
                    RichText::new(app.projects.len().to_string())
                        .strong()
                        .color(theme::text()),
                );
            });
            field(ui, "", |ui| {
                if ui.button(t("set_create_demo")).clicked() {
                    make_demo = true;
                }
            });
            // Kod git bilan qaytadi, ma'lumot esa qaytmaydi — zaxira alohida.
            field(ui, t("set_backup"), |ui| {
                if ui
                    .button(t("set_backup_btn"))
                    .on_hover_text(t("set_backup_hint"))
                    .clicked()
                {
                    make_backup = true;
                }
            });
        });

        ui.add_space(10.0);

        // ---------- Dastur haqida ----------
        card_frame(ui, t("set_group_about"), w, |ui| {
            field(ui, t("set_version"), |ui| {
                ui.label(RichText::new(env!("CARGO_PKG_VERSION")).color(theme::text()));
            });
            field(ui, t("set_stack"), |ui| {
                ui.label(RichText::new(t("set_stack_value")).color(theme::text()));
            });
            field(ui, t("set_module"), |ui| {
                ui.label(RichText::new(t("set_module_value")).color(theme::text()));
            });
        });

        ui.add_space(20.0);
    });

    if changed {
        app.save_settings();
        app.notify(t("set_saved").to_string());
    }

    if make_backup {
        let name = crate::backup::suggested_name(chrono::Local::now().naive_local());
        if let Some(dest) = rfd::FileDialog::new()
            .set_title(t("set_backup_btn"))
            .set_file_name(&name)
            .add_filter("SQLite", &["db"])
            .save_file()
        {
            match crate::backup::create(app.db.conn(), &dest) {
                Ok(p) => app.notify(format!("{} {}", t("set_backup_done"), p.display())),
                Err(e) => app.notify(format!("{}: {e}", t("set_backup_failed"))),
            }
        }
    }

    if make_demo {
        match app.db.seed_demo() {
            Ok(id) => {
                app.reload_projects();
                app.select_project(id);
                app.screen = Screen::Dashboard;
                app.notify(t("set_demo_created").to_string());
            }
            Err(e) => app.notify(format!("{}: {e}", t("err_create_object"))),
        }
    }
}
