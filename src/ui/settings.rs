//! Sozlamalar sahifasi: til, mavzu, masshtab va standart qiymatlar.

use super::*;
use crate::app::{GanttScale, Screen};
use crate::i18n::Lang;
use crate::theme::Theme;

/// Amallar tarixida shuncha oxirgi yozuv ko'rsatiladi.
const AUDIT_SHOWN: i64 = 25;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let mut changed = false;
    let mut make_demo = false;
    let mut clear_demo = false;
    let mut make_backup = false;
    let mut add_user = false;
    let mut export_pkg = false;
    let mut llm_changed = false;
    let mut import_pkg = false;
    let mut edited_user: Option<crate::roles::User> = None;
    let mut removed_user: Option<i64> = None;

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
                // Mijozga ko'rsatgandan keyin namuna kerak bo'lmaydi.
                // Tugma faqat namuna bazada turganda ko'rinadi.
                let has_demo = app
                    .projects
                    .iter()
                    .any(|p| crate::db::Db::DEMO_CODES.contains(&p.code.as_str()));
                if has_demo
                    && ui
                        .button(RichText::new(t("set_clear_demo")).color(theme::danger()))
                        .on_hover_text(t("set_clear_demo_hint"))
                        .clicked()
                {
                    clear_demo = true;
                }
            });
            // Qurilmalar orasida ma'lumot fayl orqali ko'chadi (server yo'q).
            field(ui, t("set_package"), |ui| {
                if ui
                    .button(t("set_package_export"))
                    .on_hover_text(t("set_package_export_hint"))
                    .clicked()
                {
                    export_pkg = true;
                }
                if ui
                    .button(t("set_package_import"))
                    .on_hover_text(t("set_package_import_hint"))
                    .clicked()
                {
                    import_pkg = true;
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

        // ---------- Foydalanuvchilar va rollar ----------
        card_frame(ui, t("set_group_roles"), w, |ui| {
            ui.label(
                RichText::new(t("set_roles_note"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(8.0);
            if ui.button(t("add_user")).clicked() {
                add_user = true;
            }
            ui.add_space(6.0);

            if app.users.is_empty() {
                ui.label(
                    RichText::new(t("role_no_users"))
                        .size(12.0)
                        .color(theme::muted()),
                );
            }
            egui::Grid::new("users_grid")
                .num_columns(4)
                .spacing([10.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    for src in &app.users {
                        let mut u = src.clone();
                        let mut changed = false;
                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut u.name))
                            .changed();
                        egui::ComboBox::from_id_salt(("user_role", u.id))
                            .selected_text(u.role.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for r in crate::roles::Role::ALL {
                                    changed |= ui
                                        .selectable_value(&mut u.role, *r, r.label())
                                        .on_hover_text(r.hint())
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut u.note))
                            .changed();
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed_user = Some(u.id);
                        }
                        ui.end_row();
                        if changed {
                            edited_user = Some(u);
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // ---------- Til modeli (ixtiyoriy) ----------
        card_frame(ui, t("set_group_llm"), w, |ui| {
            let built = cfg!(feature = "llm");
            if !built {
                // Tarmoq qismi yig'ilishga umuman kirmagan — buni ochiq aytamiz.
                ui.label(
                    RichText::new(t("set_llm_not_built"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
            }
            ui.label(
                RichText::new(t("set_llm_warning"))
                    .size(11.5)
                    .color(theme::warn()),
            );
            ui.add_space(8.0);

            ui.add_enabled_ui(built, |ui| {
                field(ui, t("set_llm_enabled"), |ui| {
                    if ui.checkbox(&mut app.llm.enabled, "").changed() {
                        llm_changed = true;
                    }
                });
                field(ui, t("set_llm_endpoint"), |ui| {
                    if ui
                        .add_sized(
                            [360.0, 22.0],
                            egui::TextEdit::singleline(&mut app.llm.endpoint)
                                .hint_text("https://…/v1/chat/completions"),
                        )
                        .changed()
                    {
                        llm_changed = true;
                    }
                });
                field(ui, t("set_llm_model"), |ui| {
                    if ui
                        .add_sized(
                            [220.0, 22.0],
                            egui::TextEdit::singleline(&mut app.llm.model),
                        )
                        .changed()
                    {
                        llm_changed = true;
                    }
                });
                field(ui, t("set_llm_key"), |ui| {
                    // Kalit ekranda ochiq turmasin.
                    if ui
                        .add_sized(
                            [260.0, 22.0],
                            egui::TextEdit::singleline(&mut app.llm.api_key).password(true),
                        )
                        .changed()
                    {
                        llm_changed = true;
                    }
                    let masked = app.llm.masked_key();
                    if !masked.is_empty() {
                        ui.label(RichText::new(masked).size(11.0).color(theme::muted()));
                    }
                });
            });

            ui.add_space(6.0);
            let ready = app.llm.is_ready();
            ui.label(
                RichText::new(if ready {
                    t("set_llm_ready")
                } else {
                    t("set_llm_off")
                })
                .size(11.5)
                .color(if ready { theme::ok() } else { theme::muted() }),
            );
        });

        ui.add_space(10.0);

        // ---------- Amallar tarixi ----------
        card_frame(ui, t("set_group_audit"), w, |ui| {
            ui.label(
                RichText::new(t("set_audit_hint"))
                    .size(11.5)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);

            let total = app.db.audit_count();
            field(ui, t("set_audit_count"), |ui| {
                ui.label(RichText::new(total.to_string()).color(theme::text()));
            });

            if total == 0 {
                ui.label(
                    RichText::new(t("set_audit_empty"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                return;
            }

            ui.add_space(6.0);
            // Ichma-ich aylantirish noqulay — sahifa o'zi suriladi, shuning
            // uchun bu yerda faqat oxirgi yozuvlar ko'rsatiladi.
            {
                egui::Grid::new("audit_grid")
                    .num_columns(5)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for h in [
                            t("col_when"),
                            t("col_who"),
                            t("col_what"),
                            t("col_table"),
                            t("col_row"),
                        ] {
                            ui.label(RichText::new(h).size(11.0).color(theme::muted()));
                        }
                        ui.end_row();

                        for e in app.db.audit_log(AUDIT_SHOWN) {
                            ui.label(
                                RichText::new(&e.at)
                                    .size(11.5)
                                    .monospace()
                                    .color(theme::muted()),
                            );
                            ui.label(
                                RichText::new(if e.user.is_empty() {
                                    t("no_user").to_string()
                                } else {
                                    e.user.clone()
                                })
                                .size(11.5),
                            );
                            // O'chirish ko'zga tashlanib tursin.
                            let color = match e.action {
                                crate::domain::AuditAction::Insert => theme::ok(),
                                crate::domain::AuditAction::Update => theme::text(),
                                crate::domain::AuditAction::Delete => theme::danger(),
                            };
                            ui.label(RichText::new(e.action.label()).size(11.5).color(color));
                            ui.label(RichText::new(&e.table_name).size(11.5).monospace());
                            ui.label(
                                RichText::new(if e.row_id > 0 {
                                    e.row_id.to_string()
                                } else {
                                    t("dash").to_string()
                                })
                                .size(11.5)
                                .color(theme::muted()),
                            );
                            ui.end_row();
                        }
                    });
            }
            if total > AUDIT_SHOWN {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("{} {AUDIT_SHOWN}", t("set_audit_shown")))
                        .size(11.0)
                        .color(theme::muted()),
                );
            }
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

    if add_user {
        let n = app.users.len() + 1;
        app.db.insert_user(&crate::roles::User {
            id: 0,
            name: format!("{} {n}", t("user_new_name")),
            role: crate::roles::Role::Foreman,
            note: String::new(),
        });
        app.reload_users();
    }
    if let Some(u) = edited_user {
        app.db.update_user(&u);
        app.reload_users();
    }
    if let Some(id) = removed_user {
        app.db.del("app_user", id);
        // Joriy foydalanuvchi o'chirilsa — rol to'liq huquqqa qaytadi.
        if app.current_user == Some(id) {
            app.set_user(None);
        }
        app.reload_users();
    }

    if llm_changed {
        app.save_llm();
    }

    if export_pkg {
        if app.current.is_none() {
            app.notify(t("no_object_selected").to_string());
        } else {
            let pkg = app.export_package();
            let file = format!("qurai-paket-{}.txt", app.today.format("%Y-%m-%d"));
            if let Some(path) = rfd::FileDialog::new()
                .set_title(t("set_package_export"))
                .set_file_name(&file)
                .add_filter("QURAi", &["txt"])
                .save_file()
            {
                match std::fs::write(&path, crate::package::write(&pkg)) {
                    Ok(()) => app.notify(format!(
                        "{} {} · {}",
                        t("set_package_saved"),
                        pkg.row_count(),
                        path.display()
                    )),
                    Err(e) => app.notify(format!("{}: {e}", t("set_package_failed"))),
                }
            }
        }
    }
    if import_pkg {
        if app.current.is_none() {
            app.notify(t("no_object_selected").to_string());
        } else if let Some(path) = rfd::FileDialog::new()
            .set_title(t("set_package_import"))
            .add_filter("QURAi", &["txt"])
            .pick_file()
        {
            match std::fs::read_to_string(&path).map_err(|e| e.to_string()) {
                Ok(text) => match crate::package::read(&text) {
                    Ok(pkg) => {
                        let (added, existing) = app.import_package(&pkg);
                        app.notify(format!(
                            "{} {added} · {} {existing}",
                            t("ifc_added"),
                            t("ifc_existing")
                        ));
                    }
                    Err(_) => app.notify(t("set_package_bad").to_string()),
                },
                Err(e) => app.notify(format!("{}: {e}", t("set_package_failed"))),
            }
        }
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

    if clear_demo {
        match app.db.clear_demo() {
            Ok(n) => {
                app.current = None;
                app.reload_projects();
                if let Some(p) = app.projects.first().map(|p| p.id) {
                    app.select_project(p);
                } else {
                    app.clear_modules();
                }
                app.notify(format!("{} {n}", t("set_demo_cleared")));
            }
            Err(e) => app.notify(format!("{}: {e}", t("err_delete"))),
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
