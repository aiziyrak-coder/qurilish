//! «Hisobotlar» ekrani: modul, davr, ko'rish va saqlash.
//!
//! Ekranning vazifasi sodda: **hisobotni tanlash, ko'rish va berish**.
//! Hisobotning o'zi `reports` modulida tuziladi va u yerda hech narsa
//! qaytadan hisoblanmaydi — sonlar modullardan olinadi.
//!
//! Uchta chiqish yo'li bor:
//!
//! - **Excel** — ish uchun: son son bo'lib qoladi, saralanadi va yig'iladi;
//! - **PDF** — topshirish uchun: o'zgartirib bo'lmaydi, sahifalangan;
//! - **Hammasi bitta kitobda** — har modul o'z varag'ida, oy yakunida
//!   bitta fayl yuborish uchun.

use super::*;
use crate::reports::{self, Kind, Period, Preset};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(16.0),
            );
        });
        return;
    }

    // Tanlov ilova holatida turadi: Ctrl+E ham aynan shu hisobotni
    // saqlaydi va boshqa ekranga o'tib qaytganda tanlov joyida qoladi.
    let mut kind = Kind::ALL
        .get(app.report_kind)
        .copied()
        .unwrap_or(Kind::Journal);
    let mut preset = Preset::ALL
        .get(app.report_preset)
        .copied()
        .unwrap_or(Preset::Month);

    let start = app.project().map(|p| p.start_date).unwrap_or(app.today);
    let period = preset.period(app.today, start);

    // ---- Davr
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(t("rp_period"))
                .size(11.5)
                .color(theme::muted()),
        );
        for (i, p) in Preset::ALL.iter().enumerate() {
            if ui.selectable_label(preset == *p, p.label()).clicked() {
                preset = *p;
                app.report_preset = i;
            }
        }
        ui.separator();
        ui.label(
            RichText::new(period.label())
                .size(12.0)
                .color(theme::text()),
        );
    });
    ui.add_space(8.0);

    let wide = ui.available_width() > 1100.0;
    let list_w = if wide { 300.0 } else { ui.available_width() };

    ui.horizontal_top(|ui| {
        // ---- Hisobotlar ro'yxati
        ui.vertical(|ui| {
            ui.set_width(list_w);
            egui::ScrollArea::vertical()
                .id_salt("rp_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, k) in Kind::ALL.iter().enumerate() {
                        let selected = kind == *k;
                        let resp = ui.selectable_label(
                            selected,
                            RichText::new(format!("{:<5} {}", k.numeral(), k.label())).size(12.5),
                        );
                        if resp.clicked() {
                            kind = *k;
                            app.report_kind = i;
                        }
                        // Davrga bog'liq emasligi darrov ko'rinsin.
                        if !k.uses_period() {
                            resp.on_hover_text(t("rp_no_period_hint"));
                        }
                    }
                });
        });

        if wide {
            ui.separator();
        }

        // ---- Tanlangan hisobot
        ui.vertical(|ui| {
            let table = reports::build(app, kind, period);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(kind.label()).size(15.0).strong());
                ui.label(
                    RichText::new(if kind.uses_period() {
                        period.label()
                    } else {
                        t("rp_no_period").to_string()
                    })
                    .size(11.5)
                    .color(theme::muted()),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{} {}", t("rp_rows"), table.rows.len()))
                        .size(11.5)
                        .color(theme::muted()),
                );
                ui.separator();
                if ui
                    .button(t("rp_save_xlsx"))
                    .on_hover_text(t("rp_save_xlsx_hint"))
                    .clicked()
                {
                    save_one(app, &table, false);
                }
                if ui
                    .button(t("rp_save_pdf"))
                    .on_hover_text(t("rp_save_pdf_hint"))
                    .clicked()
                {
                    save_one(app, &table, true);
                }
                if ui
                    .button(RichText::new(t("rp_save_all")).strong())
                    .on_hover_text(t("rp_save_all_hint"))
                    .clicked()
                {
                    save_all(app, period);
                }
            });
            ui.add_space(8.0);

            if table.rows.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t("rp_empty"))
                            .size(14.0)
                            .color(theme::muted()),
                    );
                    ui.label(
                        RichText::new(t("rp_empty_hint"))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                });
                return;
            }
            preview(ui, &table);
        });
    });
}

/// Hisobotning ekrandagi ko'rinishi.
///
/// Ko'rinish **birinchi yuz qator** bilan cheklanadi: minglab qatorni
/// ekranda varaqlash foydasiz, u fayl uchun. Chegara ochiq yoziladi.
fn preview(ui: &mut egui::Ui, table: &crate::docgen::Table) {
    const LIMIT: usize = 100;
    let shown = table.rows.len().min(LIMIT);

    egui::ScrollArea::both()
        .id_salt("rp_preview")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("rp_grid")
                .num_columns(table.headers.len())
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for h in &table.headers {
                        ui.label(
                            RichText::new(super::issues::truncate(h, 22))
                                .size(11.0)
                                .strong()
                                .color(theme::muted()),
                        );
                    }
                    ui.end_row();

                    for row in table.rows.iter().take(LIMIT) {
                        for cell in row {
                            ui.label(RichText::new(cell_text(cell)).size(12.0));
                        }
                        ui.end_row();
                    }
                });
            if table.rows.len() > shown {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} {} / {}",
                        t("rp_shown"),
                        shown,
                        table.rows.len()
                    ))
                    .size(11.0)
                    .color(theme::muted()),
                );
            }
        });
}

/// Katak matni — ekranda ko'rsatish uchun.
fn cell_text(c: &crate::docgen::Cell) -> String {
    use crate::docgen::Cell;
    match c {
        Cell::Text(s) => super::issues::truncate(s, 40),
        Cell::Num(v) => super::materials::trim_num(*v),
        Cell::Money(v) => money(*v),
        Cell::Date(d) => d.format("%d.%m.%Y").to_string(),
        Cell::Empty => t("dash").to_string(),
    }
}

/// Fayl nomi: hisobot nomi, obyekt kodi va sana.
fn file_name(app: &App, table: &crate::docgen::Table, ext: &str) -> String {
    let base: String = table
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-').to_lowercase();
    let code = app
        .project()
        .map(|p| p.code.clone())
        .unwrap_or_default()
        .to_lowercase();
    format!("qurai-{code}-{base}.{ext}")
}

/// Bitta hisobotni saqlaydi.
fn save_one(app: &mut App, table: &crate::docgen::Table, pdf: bool) {
    if table.rows.is_empty() {
        app.notify(t("rp_empty").to_string());
        return;
    }
    let ext = if pdf { "pdf" } else { "xlsx" };
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("rp_save"))
        .set_file_name(file_name(app, table, ext))
        .add_filter(if pdf { "PDF" } else { "Excel" }, &[ext])
        .save_file()
    else {
        return;
    };

    let result = if pdf {
        let subtitle = app.project().map(|p| p.name.clone()).unwrap_or_default();
        crate::pdf::write_table(&path, table, &subtitle)
    } else {
        crate::docgen::write_table(&path, table).map_err(|e| format!("{e}"))
    };
    match result {
        Ok(()) => app.notify(format!(
            "{} {} · {}",
            t("export_done"),
            table.rows.len(),
            path.display()
        )),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

/// Barcha hisobotlarni bitta kitobga saqlaydi.
fn save_all(app: &mut App, period: Period) {
    let tables = reports::build_all(app, period);
    if tables.is_empty() {
        app.notify(t("rp_empty").to_string());
        return;
    }
    let code = app
        .project()
        .map(|p| p.code.clone())
        .unwrap_or_default()
        .to_lowercase();
    let name = format!(
        "qurai-{code}-hisobotlar-{}.xlsx",
        app.today.format("%Y-%m-%d")
    );
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("rp_save_all"))
        .set_file_name(name)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };
    let rows: usize = tables.iter().map(|t| t.rows.len()).sum();
    match crate::docgen::write_book(&path, &tables) {
        Ok(()) => app.notify(format!(
            "{} {} · {} {} · {}",
            t("rp_saved_sheets"),
            tables.len(),
            t("export_done"),
            rows,
            path.display()
        )),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}
