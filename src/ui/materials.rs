//! «Materiallar» ekrani (TZ XII).
//!
//! Obyektda ishlatiladigan materiallarning yagona katalogi: texnik tavsif,
//! sertifikat va uning muddati, minimal zaxira va narx. Har bir qator yonida
//! ombordagi joriy qoldiq ko'rsatiladi — katalog va ombor bir-biridan ajralib
//! qolmasligi uchun.

use super::*;
use crate::domain::Material;
use crate::model::Section;
use egui::{vec2, Sense};

/// Sertifikat muddati shu kun ichida tugasa — ogohlantiramiz.
const CERT_WARN_DAYS: i64 = 30;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        empty_screen(ui, t("no_object_selected"));
        return;
    }

    let mut add = false;
    let mut seed = false;
    ui.horizontal(|ui| {
        if ui.button(t("add_material")).clicked() {
            add = true;
        }
        // Eski bazalarda katalog bo'sh qoladi — namunani bir bosishda yuklaymiz.
        if app.stock_moves.is_empty() && ui.button(t("load_demo_stock")).clicked() {
            seed = true;
        }
        ui.label(
            RichText::new(t("materials_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.materials.is_empty() {
        empty_screen(ui, t("materials_empty"));
    } else {
        table(ui, app);
    }

    if add {
        if let Some(pid) = app.current {
            let id = app.db.insert_material(&Material {
                id: 0,
                project_id: pid,
                code: String::new(),
                name: t("material_new_name").to_string(),
                unit: String::new(),
                section: Section::None,
                spec: String::new(),
                cert_no: String::new(),
                cert_until: None,
                min_stock: 0.0,
                price: 0.0,
                note: String::new(),
            });
            app.reload_modules();
            let _ = id;
        }
    }
    if seed {
        if let Some(pid) = app.current {
            app.db
                .seed_demo_stock(pid, crate::i18n::lang() == crate::i18n::Lang::Ru);
            app.reload_modules();
        }
    }
}

fn empty_screen(ui: &mut egui::Ui, msg: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.label(RichText::new(msg).color(theme::muted()).size(16.0));
    });
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let lines = app.stock();
    let total_value: f64 = lines.iter().map(|l| l.value).sum();
    let below = lines.iter().filter(|l| l.below_min).count();

    // Sertifikat holati: muddati o'tgan va yaqin orada tugaydiganlar.
    let mut expired = 0;
    let mut soon = 0;
    for m in &app.materials {
        let Some(until) = m.cert_until else { continue };
        let left = (until - app.today).num_days();
        if left < 0 {
            expired += 1;
        } else if left <= CERT_WARN_DAYS {
            soon += 1;
        }
    }
    let no_cert = app
        .materials
        .iter()
        .filter(|m| m.cert_no.trim().is_empty())
        .count();

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_materials"),
            app.materials.len().to_string(),
            t("kpi_materials_hint"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_stock_value"),
            money(total_value),
            t("kpi_stock_value_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_below_min"),
            below.to_string(),
            t("kpi_below_min_hint"),
            if below == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        stat_card(
            ui,
            t("kpi_cert_expired"),
            expired.to_string(),
            &format!("{soon} {}", t("kpi_cert_soon")),
            if expired == 0 && soon == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        stat_card(
            ui,
            t("kpi_no_cert"),
            no_cert.to_string(),
            t("kpi_no_cert_hint"),
            if no_cert == 0 {
                theme::ok()
            } else {
                theme::warn()
            },
        );
    });
}

fn table(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<Material> = None;
    let mut removed: Option<i64> = None;
    let lines = app.stock();
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("materials_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 90.0, t("col_code"));
                    head(ui, 210.0, t("col_name"));
                    head(ui, 56.0, t("col_unit"));
                    head(ui, 50.0, t("col_section_short"));
                    head(ui, 180.0, t("col_spec"));
                    head(ui, 110.0, t("col_cert"));
                    head(ui, 118.0, t("col_cert_until"));
                    head(ui, 80.0, t("col_min_stock"));
                    head(ui, 100.0, t("col_price"));
                    head(ui, 120.0, t("col_balance"));
                    head(ui, 24.0, "");
                    ui.end_row();

                    for material in &app.materials {
                        let mut m = material.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut m.code))
                            .changed();
                        changed |= ui
                            .add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut m.name))
                            .changed();
                        changed |= ui
                            .add_sized([56.0, 22.0], egui::TextEdit::singleline(&mut m.unit))
                            .changed();
                        egui::ComboBox::from_id_salt(("mat_sec", m.id))
                            .selected_text(m.section.label())
                            .width(50.0)
                            .show_ui(ui, |ui| {
                                for s in Section::ALL {
                                    changed |=
                                        ui.selectable_value(&mut m.section, s, s.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized(
                                [180.0, 22.0],
                                egui::TextEdit::singleline(&mut m.spec)
                                    .hint_text(t("col_spec_hint")),
                            )
                            .changed();
                        changed |= ui
                            .add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut m.cert_no))
                            .changed();

                        // Sertifikat muddati: belgilanmagan bo'lishi mumkin.
                        ui.horizontal(|ui| {
                            let mut has = m.cert_until.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                m.cert_until = has.then(|| today + chrono::Duration::days(365));
                                changed = true;
                            }
                            if let Some(mut d) = m.cert_until {
                                if super::passport::date_edit(ui, &format!("cert{}", m.id), &mut d)
                                {
                                    m.cert_until = Some(d);
                                    changed = true;
                                }
                                let left = (d - today).num_days();
                                if left < 0 {
                                    ui.label(
                                        RichText::new(t("cert_expired"))
                                            .size(10.5)
                                            .color(theme::danger()),
                                    );
                                } else if left <= CERT_WARN_DAYS {
                                    ui.label(
                                        RichText::new(format!("{left} {}", t("days_short")))
                                            .size(10.5)
                                            .color(theme::warn()),
                                    );
                                }
                            }
                        });

                        changed |= ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut m.min_stock)
                                    .speed(1.0)
                                    .range(0.0..=1e9),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [100.0, 22.0],
                                egui::DragValue::new(&mut m.price)
                                    .speed(100.0)
                                    .range(0.0..=1e12),
                            )
                            .changed();

                        // Ombordagi qoldiq — faqat ko'rsatiladi, bu yerda tahrirlanmaydi.
                        let line = lines.iter().find(|l| l.material_id == m.id);
                        ui.horizontal(|ui| match line {
                            Some(l) => {
                                let color = if l.negative {
                                    theme::danger()
                                } else if l.below_min {
                                    theme::warn()
                                } else {
                                    theme::text()
                                };
                                ui.add_sized(
                                    [76.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(trim_num(l.balance))
                                            .size(12.0)
                                            .color(color)
                                            .strong(),
                                    ),
                                );
                                if l.below_min {
                                    ui.label(
                                        RichText::new(t("below_min_short"))
                                            .size(10.0)
                                            .color(theme::warn()),
                                    );
                                }
                            }
                            None => {
                                ui.label(RichText::new(t("dash")).color(theme::muted()));
                            }
                        });

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(m.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(m);
                        }
                    }
                });
        });

    if let Some(m) = edited {
        app.db.update_material(&m);
        if let Some(slot) = app.materials.iter_mut().find(|x| x.id == m.id) {
            *slot = m;
        }
    }
    if let Some(id) = removed {
        app.db.del("material", id);
        app.reload_modules();
    }
}

/// 12.0 -> «12», 4.5 -> «4.5».
pub fn trim_num(v: f64) -> String {
    // Manfiy nol «-0» bo'lib chiqmasin.
    let v = if v == 0.0 { 0.0 } else { v };
    if (v - v.round()).abs() < 1e-6 {
        format!("{:.0}", v)
    } else {
        format!("{v:.2}")
    }
}

/// Material nomini id bo'yicha topadi — boshqa ekranlar ham ishlatadi.
pub fn material_label(app: &App, id: i64) -> String {
    app.materials
        .iter()
        .find(|m| m.id == id)
        .map(|m| {
            if m.code.trim().is_empty() {
                m.name.clone()
            } else {
                format!("{} · {}", m.code, m.name)
            }
        })
        .unwrap_or_else(|| t("dash").to_string())
}

/// Material tanlash ro'yxati. O'zgargan bo'lsa `true`.
pub fn material_picker(
    ui: &mut egui::Ui,
    app: &App,
    salt: impl std::hash::Hash,
    cur: &mut i64,
    width: f32,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(super::issues::truncate(&material_label(app, *cur), 30))
        .width(width)
        .show_ui(ui, |ui| {
            for m in &app.materials {
                changed |= ui
                    .selectable_value(cur, m.id, material_label(app, m.id))
                    .changed();
            }
        });
    changed
}

/// Qoldiq polosasi: minimal zaxiraga nisbatan joriy holat.
pub fn stock_bar(ui: &mut egui::Ui, balance: f64, min_stock: f64, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 10.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, theme::track());
    // Shkala: minimal zaxiraning ikki barobari to'liq kenglik deb olinadi.
    let scale = if min_stock > 0.0 {
        min_stock * 2.0
    } else {
        balance.max(1.0)
    };
    let frac = (balance.max(0.0) / scale).clamp(0.0, 1.0) as f32;
    let color = if balance < 0.0 {
        theme::danger()
    } else if min_stock > 0.0 && balance < min_stock {
        theme::warn()
    } else {
        theme::ok()
    };
    p.rect_filled(
        Rect::from_min_size(rect.min, vec2(rect.width() * frac, rect.height())),
        3.0,
        color,
    );
    // Minimal zaxira chizig'i.
    if min_stock > 0.0 {
        let x = rect.min.x + rect.width() * 0.5;
        p.line_segment(
            [pos2(x, rect.min.y - 2.0), pos2(x, rect.max.y + 2.0)],
            Stroke::new(1.5_f32, theme::text().gamma_multiply(0.6)),
        );
    }
}
