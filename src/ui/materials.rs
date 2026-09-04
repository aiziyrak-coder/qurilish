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

    // Modul yordamchisi (TZ: har modul uchun AI-yordamchi).
    ui.horizontal(|ui| {
        super::assistant_button(ui, app);
    });
    ui.add_space(6.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.materials.is_empty() {
        empty_screen(ui, t("materials_empty"));
    } else {
        // Uch ko'rinish: katalog, sarf normalari va normativ/fakt taqqoslash.
        let tab_key = egui::Id::new("mat_tab");
        let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            for (i, label) in [
                (0u8, t("mat_tab_catalog")),
                (1, t("mat_tab_norms")),
                (2, t("mat_tab_usage")),
                (3, t("mat_tab_alts")),
                (4, t("mat_tab_trace")),
                (5, t("mat_tab_ready")),
                (6, t("mat_tab_fit")),
                (7, t("mat_tab_kits")),
            ] {
                if ui.selectable_label(tab == i, label).clicked() {
                    tab = i;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(tab_key, tab));
        ui.add_space(8.0);
        match tab {
            1 => norms_tab(ui, app),
            2 => usage_tab(ui, app),
            3 => {
                if let Some(pid) = app.current {
                    alts_tab(ui, app, pid);
                }
            }
            4 => trace_tab(ui, app),
            5 => ready_tab(ui, app),
            6 => fit_tab(ui, app),
            7 => kits_tab(ui, app),
            _ => table(ui, app),
        }
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
                estimate_code: String::new(),
                spec_ref: String::new(),
                special: String::new(),
                banned: false,
                ban_reason: String::new(),
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

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_materials"),
                app.materials.len().to_string(),
                t("kpi_materials_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_stock_value"),
                money(total_value),
                t("kpi_stock_value_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_below_min"),
                below.to_string(),
                t("kpi_below_min_hint"),
                if below == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_cert_expired"),
                expired.to_string(),
                &format!("{soon} {}", t("kpi_cert_soon")),
                if expired == 0 && soon == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_no_cert"),
                no_cert.to_string(),
                t("kpi_no_cert_hint"),
                if no_cert == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
}

fn table(ui: &mut egui::Ui, app: &mut App) {
    let card_key = egui::Id::new("mat_card");
    let mut open_card: Option<i64> = None;
    let mut edited: Option<Material> = None;
    let mut removed: Option<i64> = None;
    let lines = app.stock();
    let today = app.today;
    // Tor ekranda loyiha havolalari yashiriladi: ular bir marta to'ldiriladi,
    // kunlik ish esa qoldiq va narx ustunlarida.
    let wide = ui.available_width() > 1500.0;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("materials_grid")
                .num_columns(if wide { 15 } else { 12 })
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
                    if wide {
                        head(ui, 100.0, t("col_estimate_code"));
                        head(ui, 120.0, t("col_spec_ref"));
                        head(ui, 150.0, t("col_special"));
                    }
                    head(ui, 150.0, t("col_banned"));
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

                        if wide {
                            // Smeta va loyiha bilan bog'lanish (TZ XII.5–7).
                            changed |= ui
                                .add_sized(
                                    [100.0, 22.0],
                                    egui::TextEdit::singleline(&mut m.estimate_code),
                                )
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [120.0, 22.0],
                                    egui::TextEdit::singleline(&mut m.spec_ref),
                                )
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [150.0, 22.0],
                                    egui::TextEdit::singleline(&mut m.special),
                                )
                                .changed();
                        }

                        // Taqiq (TZ XII.38): sababsiz taqiq bajarilmaydi,
                        // shuning uchun sabab maydoni yonida turadi.
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut m.banned, "").changed();
                            if m.banned {
                                let r = ui.add_sized(
                                    [120.0, 22.0],
                                    egui::TextEdit::singleline(&mut m.ban_reason)
                                        .hint_text(t("col_reason")),
                                );
                                changed |= r.changed();
                                if m.ban_reason.trim().is_empty() {
                                    r.on_hover_text(t("ban_reason_hint"));
                                    ui.label(RichText::new("!").color(theme::danger()).strong());
                                }
                            }
                        });

                        // Kartochka: material haqidagi hamma narsa bir
                        // joyda — beshta ekranni aylanish shart emas.
                        if ui
                            .small_button(t("mc_open"))
                            .on_hover_text(t("mc_open_hint"))
                            .clicked()
                        {
                            open_card = Some(m.id);
                        }

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

    // Kartochka jadval ostida ochiladi; o'sha tugma qayta bosilsa yopiladi.
    let mut shown = ui.data(|d| d.get_temp::<i64>(card_key));
    if let Some(id) = open_card {
        shown = if shown == Some(id) { None } else { Some(id) };
    }
    match shown {
        Some(id) => {
            ui.data_mut(|d| d.insert_temp(card_key, id));
            ui.add_space(10.0);
            card_panel(ui, app, id);
        }
        None => ui.data_mut(|d| d.remove::<i64>(card_key)),
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

// ================================================================ Normalar

/// Sarf normalari (TZ XI.15, XII.21): bir birlik ishga qancha material.
fn norms_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::cell_l;
    let pid = match app.current {
        Some(p) => p,
        None => return,
    };

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_norm")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("norms_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.material_norms.is_empty() {
        empty_screen(ui, t("norms_empty"));
    } else {
        let mut edited: Option<crate::domain::MaterialNorm> = None;
        let mut removed: Option<i64> = None;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("mat_norms")
                    .num_columns(7)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 250.0, t("col_task"));
                        head_l(ui, 220.0, t("col_material"));
                        head_r(ui, 110.0, t("col_per_unit"));
                        head_l(ui, 90.0, t("col_unit"));
                        head_r(ui, 90.0, t("col_tolerance"));
                        head_l(ui, 200.0, t("col_note"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.material_norms {
                            let mut n = src.clone();
                            let mut changed = false;

                            let mut task = Some(n.task_id);
                            if task_picker(ui, app, ("nm_task", n.id), &mut task, 250.0) {
                                // Bo'sh tanlov normani ma'nosiz qiladi — eskisi qoladi.
                                if let Some(id) = task {
                                    n.task_id = id;
                                    changed = true;
                                }
                            }
                            changed |= material_picker(
                                ui,
                                app,
                                ("nm_mat", n.id),
                                &mut n.material_id,
                                220.0,
                            );
                            changed |= num_edit(ui, 110.0, &mut n.per_unit, 0.001, 1_000_000.0);

                            // Norma o'lchov birligi: material birligi / ish birligi.
                            let mu = app
                                .materials
                                .iter()
                                .find(|m| m.id == n.material_id)
                                .map(|m| m.unit.clone())
                                .unwrap_or_default();
                            let tu = app
                                .task(n.task_id)
                                .map(|x| x.unit.clone())
                                .unwrap_or_default();
                            cell_l(
                                ui,
                                90.0,
                                RichText::new(if mu.is_empty() && tu.is_empty() {
                                    String::new()
                                } else {
                                    format!("{mu}/{tu}")
                                })
                                .size(11.0)
                                .color(theme::muted()),
                            );

                            ui.horizontal(|ui| {
                                changed |= num_edit(ui, 58.0, &mut n.tolerance, 0.1, 100.0);
                                ui.label(RichText::new("%").size(11.0).color(theme::muted()));
                            });
                            changed |= ui
                                .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut n.note))
                                .changed();
                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(n.id);
                            }
                            ui.end_row();
                            if changed {
                                edited = Some(n);
                            }
                        }
                    });
            });

        if let Some(n) = edited {
            app.db.update_material_norm(&n);
            app.reload_modules();
        }
        if let Some(id) = removed {
            app.db.del("material_norm", id);
            app.reload_modules();
        }
    }

    if add {
        // Yangi norma birinchi ish va birinchi materialdan boshlanadi —
        // foydalanuvchi keyin o'zgartiradi.
        if let (Some(task), Some(mat)) = (
            app.tasks.first().map(|x| x.id),
            app.materials.first().map(|m| m.id),
        ) {
            app.db.insert_material_norm(&crate::domain::MaterialNorm {
                id: 0,
                project_id: pid,
                task_id: task,
                material_id: mat,
                per_unit: 0.0,
                tolerance: 5.0,
                note: String::new(),
            });
            app.reload_modules();
        } else {
            app.notify(t("norm_needs_task").to_string());
        }
    }
}

// ================================================================ Sarf

/// Normativ va haqiqiy sarf taqqoslanadi (TZ XI.14, XII.22, III.28).
fn usage_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let mut make_m29 = false;
    let lines = app.consumption();
    if lines.is_empty() {
        empty_screen(ui, t("usage_empty"));
        return;
    }

    let over: Vec<_> = lines.iter().filter(|l| l.over).collect();
    let over_cost: f64 = over.iter().map(|l| l.over_cost).sum();
    stat_row(
        ui,
        vec![
            stat(
                t("kpi_norm_lines"),
                lines.len().to_string(),
                t("kpi_norm_lines_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_overuse"),
                over.len().to_string(),
                t("kpi_overuse_hint"),
                if over.is_empty() {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_overuse_cost"),
                money(over_cost),
                t("kpi_overuse_cost_hint"),
                if over_cost > 0.0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
        ],
    );
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        // M-29 — material sarfi hisoboti, shu jadvaldan chiqadi (TZ IV.6).
        if ui
            .button(t("doc_m29_short"))
            .on_hover_text(t("doc_m29_hint"))
            .clicked()
        {
            make_m29 = true;
        }
        ui.label(
            RichText::new(t("usage_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("mat_usage")
                .num_columns(8)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 250.0, t("col_task"));
                    head_l(ui, 200.0, t("col_material"));
                    head_r(ui, 100.0, t("col_done_volume"));
                    head_r(ui, 100.0, t("col_norm"));
                    head_r(ui, 100.0, t("col_fact"));
                    head_r(ui, 100.0, t("col_diff"));
                    head_r(ui, 80.0, "%");
                    head_r(ui, 120.0, t("col_over_cost"));
                    ui.end_row();

                    for l in &lines {
                        let task = app
                            .task(l.task_id)
                            .map(|x| format!("{} {}", x.wbs, x.name))
                            .unwrap_or_default();
                        cell_l(
                            ui,
                            250.0,
                            RichText::new(super::issues::truncate(&task, 36)).size(12.0),
                        );
                        cell_l(
                            ui,
                            200.0,
                            RichText::new(super::issues::truncate(
                                &material_label(app, l.material_id),
                                28,
                            ))
                            .size(12.0),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(trim_num(l.done_volume))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(ui, 100.0, RichText::new(trim_num(l.norm)).size(12.0));
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(trim_num(l.fact)).size(12.0).strong(),
                        );
                        // Ortiqcha — qizil, tejalgan — yashil.
                        let color = if l.over {
                            theme::danger()
                        } else if l.diff < 0.0 {
                            theme::ok()
                        } else {
                            theme::muted()
                        };
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(format!(
                                "{}{}",
                                if l.diff > 0.0 { "+" } else { "" },
                                trim_num(l.diff)
                            ))
                            .size(12.0)
                            .color(color),
                        );
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(if l.norm > 0.0 {
                                format!("{:+.0}%", l.diff_pct)
                            } else {
                                t("dash").to_string()
                            })
                            .size(11.5)
                            .color(color),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(if l.over_cost > 0.0 {
                                money(l.over_cost)
                            } else {
                                t("dash").to_string()
                            })
                            .size(11.5)
                            .color(if l.over_cost > 0.0 {
                                theme::danger()
                            } else {
                                theme::muted()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });

    if make_m29 {
        write_m29(app, &lines);
    }
}

/// M-29 hisobotini faylga yozadi (TZ IV.6).
fn write_m29(app: &mut App, lines: &[crate::checks::ConsumptionLine]) {
    let Some(project) = app.project().cloned() else {
        return;
    };
    let (from, to) = super::doc_period(app);
    let file = format!("M-29-{}.xlsx", to.format("%Y-%m"));
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("doc_save"))
        .set_file_name(&file)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };
    let inp = crate::docgen::DocInput {
        project: &project,
        parties: &app.parties,
        tasks: &app.tasks,
        today: app.today,
        from,
        to,
    };
    match crate::docgen::write_m29(&path, &inp, lines, &app.materials, &app.material_norms) {
        Ok(()) => app.notify(format!("{} {}", t("doc_saved"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("doc_failed"))),
    }
}

// ================================================================ Analoglar

/// Almashtiruvchi materiallar (TZ XII.9–11).
fn alts_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    use super::warehouse::{cell_l, cell_r};
    use crate::domain::MaterialAlt;

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_alt")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("alts_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.material_alts.is_empty() {
        empty_screen(ui, t("alts_empty"));
    } else {
        let stock = app.stock();
        let mut edited: Option<MaterialAlt> = None;
        let mut removed: Option<i64> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("mat_alts")
                    .num_columns(9)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 220.0, t("col_material"));
                        head_l(ui, 220.0, t("col_alt"));
                        head_r(ui, 130.0, t("col_price"));
                        head_r(ui, 110.0, t("col_alt_diff"));
                        head_r(ui, 110.0, t("col_balance"));
                        head_l(ui, 160.0, t("col_approved_by"));
                        head_l(ui, 130.0, t("col_approved_at"));
                        head_l(ui, 130.0, t("col_status"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.material_alts {
                            let mut a = src.clone();
                            let mut changed = false;

                            changed |=
                                material_picker(ui, app, ("al_m", a.id), &mut a.material_id, 220.0);
                            changed |=
                                material_picker(ui, app, ("al_a", a.id), &mut a.alt_id, 220.0);

                            let base = app.materials.iter().find(|m| m.id == a.material_id);
                            let alt = app.materials.iter().find(|m| m.id == a.alt_id);
                            let alt_price = alt.map_or(0.0, |m| m.price);
                            cell_r(ui, 130.0, RichText::new(money(alt_price)).size(12.0));

                            // Narx farqi: analog qimmatroqmi yoki arzonroq.
                            let base_price = base.map_or(0.0, |m| m.price);
                            let diff = if base_price > 0.0 {
                                (alt_price - base_price) / base_price * 100.0
                            } else {
                                0.0
                            };
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(if base_price > 0.0 && alt_price > 0.0 {
                                    format!("{diff:+.0}%")
                                } else {
                                    t("dash").to_string()
                                })
                                .size(11.5)
                                .color(if diff > 0.0 {
                                    theme::warn()
                                } else if diff < 0.0 {
                                    theme::ok()
                                } else {
                                    theme::muted()
                                }),
                            );
                            // Analog omborda bormi — almashtirish shu yerda hal bo'ladi.
                            let bal = stock
                                .iter()
                                .find(|l| l.material_id == a.alt_id)
                                .map_or(0.0, |l| l.available);
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(trim_num(bal)).size(12.0).color(if bal > 0.0 {
                                    theme::ok()
                                } else {
                                    theme::muted()
                                }),
                            );

                            changed |= ui
                                .add_sized(
                                    [160.0, 22.0],
                                    egui::TextEdit::singleline(&mut a.approved_by)
                                        .hint_text(t("col_approved_by")),
                                )
                                .changed();
                            ui.horizontal(|ui| {
                                let mut has = a.approved_at.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    a.approved_at = has.then_some(app.today);
                                    changed = true;
                                }
                                if let Some(mut d) = a.approved_at {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("ala{}", a.id),
                                        &mut d,
                                    ) {
                                        a.approved_at = Some(d);
                                        changed = true;
                                    }
                                }
                            });

                            // Tasdiqlanmagan analog ishlatishga asos emas.
                            let banned = alt.is_some_and(|m| m.banned);
                            let (text, color) = if banned {
                                (t("alt_banned"), theme::danger())
                            } else if a.approved() {
                                (t("alt_approved"), theme::ok())
                            } else {
                                (t("alt_not_approved"), theme::warn())
                            };
                            cell_l(ui, 130.0, RichText::new(text).size(11.0).color(color));

                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(a.id);
                            }
                            ui.end_row();
                            if changed {
                                edited = Some(a);
                            }
                        }
                    });
            });

        if let Some(a) = edited {
            app.db.update_material_alt(&a);
            app.reload_modules();
        }
        if let Some(id) = removed {
            app.db.del("material_alt", id);
            app.reload_modules();
        }
    }

    if add {
        if app.materials.len() < 2 {
            app.notify(t("alt_needs_two").to_string());
        } else {
            let first = app.materials[0].id;
            let second = app.materials[1].id;
            app.db.insert_material_alt(&MaterialAlt {
                id: 0,
                project_id: pid,
                material_id: first,
                alt_id: second,
                approved_by: String::new(),
                approved_at: None,
                note: String::new(),
            });
            app.reload_modules();
        }
    }
}

// ================================================================ Kuzatuvchanlik

/// Bitta material bo'yicha to'liq zanjir va narx tarixi (TZ XII.19, 29–30, 35–36).
fn trace_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::cell_l;

    let key = egui::Id::new("mat_trace_pick");
    let mut picked = ui
        .data(|d| d.get_temp::<i64>(key))
        .filter(|id| app.materials.iter().any(|m| m.id == *id))
        .or_else(|| app.materials.first().map(|m| m.id));

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(t("col_material"))
                .size(11.5)
                .color(theme::muted()),
        );
        if let Some(cur) = &mut picked {
            material_picker(ui, app, "tr_pick", cur, 260.0);
        }
        ui.label(
            RichText::new(t("trace_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    let Some(mid) = picked else {
        empty_screen(ui, t("materials_empty"));
        return;
    };
    ui.data_mut(|d| d.insert_temp(key, mid));
    ui.add_space(8.0);

    let trace = crate::checks::material_trace(mid, &app.stock_moves, &app.exec_docs, &app.quality);
    let hist = crate::checks::price_history(mid, &app.stock_moves);
    let rating = crate::checks::material_ratings(&app.materials, &app.stock_moves, &app.quality)
        .into_iter()
        .find(|r| r.material_id == mid);

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_trace_in"),
                trim_num(trace.received),
                t("kpi_trace_in_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_issued"),
                trim_num(trace.issued),
                &format!("{} {}", trace.tasks.len(), t("kpi_issued_hint")),
                theme::accent(),
            ),
            stat(
                t("kpi_deliveries"),
                rating.as_ref().map_or(0, |r| r.deliveries).to_string(),
                t("kpi_deliveries_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_price_change"),
                match rating.as_ref().filter(|r| r.deliveries > 1) {
                    Some(r) => format!("{:+.0}%", r.price_change_pct),
                    None => t("dash").to_string(),
                },
                &match rating.as_ref().filter(|r| r.last_price > 0.0) {
                    Some(r) => format!("{} {}", t("kpi_last_price"), money(r.last_price)),
                    None => t("kpi_price_change_hint").to_string(),
                },
                match rating.as_ref() {
                    Some(r) if r.price_change_pct > 10.0 => theme::danger(),
                    Some(r) if r.price_change_pct > 0.0 => theme::warn(),
                    _ => theme::muted(),
                },
            ),
            stat(
                t("kpi_input_pass"),
                match rating.as_ref().and_then(|r| r.pass_pct) {
                    Some(v) => format!("{v:.0}%"),
                    None => t("dash").to_string(),
                },
                &match rating.as_ref().filter(|r| r.checks > 0) {
                    Some(r) => format!(
                        "{} {} · {} {}",
                        r.checks,
                        t("kpi_checks"),
                        r.rejected,
                        t("kpi_rejected")
                    ),
                    None => t("kpi_input_pass_hint").to_string(),
                },
                match rating.as_ref().and_then(|r| r.pass_pct) {
                    Some(v) if v >= 99.9 => theme::ok(),
                    Some(_) => theme::danger(),
                    None => theme::muted(),
                },
            ),
        ],
    );
    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---- Zanjir: qayerdan → qayerga → qaysi hujjatga.
            card_frame(
                ui,
                t("trace_chain"),
                (ui.available_width() - 20.0).max(200.0),
                |ui| {
                    let line = |ui: &mut egui::Ui, label: &str, value: String| {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [190.0, 18.0],
                                egui::Label::new(
                                    RichText::new(label).size(11.5).color(theme::muted()),
                                ),
                            );
                            ui.label(RichText::new(value).size(12.0));
                        });
                    };
                    line(
                        ui,
                        t("trace_suppliers"),
                        if trace.suppliers.is_empty() {
                            t("dash").to_string()
                        } else {
                            trace.suppliers.join(", ")
                        },
                    );
                    line(
                        ui,
                        t("trace_batches"),
                        if trace.batches.is_empty() {
                            t("dash").to_string()
                        } else {
                            trace
                                .batches
                                .iter()
                                .filter_map(|id| app.batches.iter().find(|b| b.id == *id))
                                .map(|b| b.number.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        },
                    );
                    line(
                        ui,
                        t("trace_tasks"),
                        if trace.tasks.is_empty() {
                            t("dash").to_string()
                        } else {
                            trace
                                .tasks
                                .iter()
                                .filter_map(|id| app.task(*id))
                                .map(|x| format!("{} {}", x.wbs, x.name))
                                .collect::<Vec<_>>()
                                .join("; ")
                        },
                    );
                    line(
                        ui,
                        t("trace_docs"),
                        if trace.docs.is_empty() {
                            // Hujjat yo'qligi ham javob: zanjir uzilgan.
                            t("trace_no_docs").to_string()
                        } else {
                            trace
                                .docs
                                .iter()
                                .filter_map(|id| app.exec_docs.iter().find(|d| d.id == *id))
                                .map(|d| d.number.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        },
                    );
                    line(ui, t("trace_checks"), trace.checks.len().to_string());
                },
            );
            ui.add_space(10.0);

            // ---- Narx tarixi.
            card_frame(
                ui,
                t("trace_prices"),
                (ui.available_width() - 20.0).max(200.0),
                |ui| {
                    if hist.is_empty() {
                        ui.label(
                            RichText::new(t("trace_no_prices"))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        return;
                    }
                    egui::Grid::new("mat_prices")
                        .num_columns(6)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for h in [
                                t("col_date"),
                                t("col_price"),
                                t("col_change"),
                                t("col_qty"),
                                t("col_supplier"),
                                t("col_document"),
                            ] {
                                ui.label(RichText::new(h).size(11.0).color(theme::muted()));
                            }
                            ui.end_row();
                            for p in &hist {
                                ui.label(
                                    RichText::new(p.date.format("%d.%m.%Y").to_string())
                                        .size(11.5)
                                        .monospace(),
                                );
                                ui.label(RichText::new(money(p.price)).size(12.0));
                                match p.change_pct {
                                    Some(c) => ui.label(
                                        RichText::new(format!("{c:+.1}%")).size(11.5).color(
                                            if c > 10.0 {
                                                theme::danger()
                                            } else if c > 0.0 {
                                                theme::warn()
                                            } else {
                                                theme::ok()
                                            },
                                        ),
                                    ),
                                    None => ui.label(
                                        RichText::new(t("dash")).size(11.5).color(theme::muted()),
                                    ),
                                };
                                ui.label(RichText::new(trim_num(p.qty)).size(11.5));
                                ui.label(RichText::new(&p.supplier).size(11.5));
                                ui.label(
                                    RichText::new(&p.document).size(11.5).color(theme::muted()),
                                );
                                ui.end_row();
                            }
                        });
                },
            );
            ui.add_space(10.0);

            // ---- Brak.
            let defects =
                crate::checks::defect_lines(&app.materials, &app.stock_moves, &app.quality);
            if let Some(d) = defects.iter().find(|d| d.material_id == mid) {
                card_frame(
                    ui,
                    t("trace_defects"),
                    (ui.available_width() - 20.0).max(200.0),
                    |ui| {
                        ui.label(
                            RichText::new(format!("{} {}", t("trace_rejected"), d.rejected))
                                .size(12.0)
                                .color(theme::danger()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} {} · {} {}",
                                t("mk_writeoff"),
                                trim_num(d.written_off),
                                t("mk_to_supplier"),
                                trim_num(d.returned)
                            ))
                            .size(11.5)
                            .color(theme::muted()),
                        );
                        if d.unresolved {
                            ui.label(
                                RichText::new(t("trace_unresolved"))
                                    .size(11.5)
                                    .color(theme::danger()),
                            );
                        }
                        for r in &d.reasons {
                            cell_l(
                                ui,
                                (ui.available_width() - 10.0).max(80.0),
                                RichText::new(r).size(11.5),
                            );
                        }
                    },
                );
            }
        });
}

// ================================================================ Tayyorlik

/// Yaqinda boshlanadigan ishlar uchun material yetarlimi (TZ XII.28).
fn ready_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let lines = app.readiness();

    ui.label(
        RichText::new(t("ready_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    if lines.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("ready_ok")).color(theme::ok()).size(15.0));
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("mat_ready")
                .num_columns(7)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 280.0, t("col_task"));
                    head_r(ui, 90.0, t("col_days_left"));
                    head_l(ui, 220.0, t("col_material"));
                    head_r(ui, 110.0, t("col_needed"));
                    head_r(ui, 110.0, t("col_available"));
                    head_r(ui, 110.0, t("col_short"));
                    head_l(ui, 70.0, t("col_unit"));
                    ui.end_row();

                    for l in &lines {
                        let task = app.task(l.task_id);
                        cell_l(
                            ui,
                            280.0,
                            RichText::new(super::issues::truncate(
                                &task
                                    .map(|x| format!("{} {}", x.wbs, x.name))
                                    .unwrap_or_default(),
                                40,
                            ))
                            .size(12.0),
                        );
                        // Kam kun qolgani qanchalik shoshilinchligini ko'rsatadi.
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(l.days_left.to_string()).size(12.0).color(
                                if l.days_left <= 0 {
                                    theme::danger()
                                } else if l.days_left <= 3 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                },
                            ),
                        );
                        let m = app.materials.iter().find(|m| m.id == l.material_id);
                        cell_l(
                            ui,
                            220.0,
                            RichText::new(super::issues::truncate(
                                &material_label(app, l.material_id),
                                28,
                            ))
                            .size(12.0),
                        );
                        cell_r(ui, 110.0, RichText::new(trim_num(l.needed)).size(12.0));
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(trim_num(l.available))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(trim_num(l.short))
                                .size(12.5)
                                .strong()
                                .color(theme::danger()),
                        );
                        cell_l(
                            ui,
                            70.0,
                            RichText::new(m.map(|m| m.unit.clone()).unwrap_or_default())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Kartochka

/// Material kartochkasi (TZ XII.11, 18, 22, 25-27, XI.39).
///
/// Kartochka yangi hisob qilmaydi: har bo'lim o'z funksiyasidan olinadi.
/// Qiymati boshqa — material haqidagi hamma narsa bir joyda va uni
/// ko'rish uchun beshta ekranni aylanish shart emas.
fn card_panel(ui: &mut egui::Ui, app: &mut App, material_id: i64) {
    let Some(c) = app.material_card(material_id) else {
        return;
    };
    let name = material_label(app, material_id);
    let unit = app
        .materials
        .iter()
        .find(|m| m.id == material_id)
        .map(|m| m.unit.clone())
        .unwrap_or_default();

    egui::Frame::new()
        .fill(theme::card())
        .stroke(egui::Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&name).size(13.5).strong());
                ui.label(
                    RichText::new(format!("#{}", c.material_id))
                        .size(10.5)
                        .monospace()
                        .color(theme::muted()),
                );
            });
            ui.add_space(6.0);

            // ---------- Ombor va narx ----------
            ui.horizontal_wrapped(|ui| {
                let item = |ui: &mut egui::Ui, label: &str, value: String, colour| {
                    ui.label(
                        RichText::new(format!("{label}: "))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                    ui.label(RichText::new(value).size(12.5).color(colour));
                    ui.add_space(10.0);
                };
                item(
                    ui,
                    t("mc_available"),
                    format!("{} {}", trim_num(c.available), unit),
                    if c.below_min {
                        theme::danger()
                    } else {
                        theme::text()
                    },
                );
                item(
                    ui,
                    t("mc_balance"),
                    format!("{} {}", trim_num(c.balance), unit),
                    theme::muted(),
                );
                item(
                    ui,
                    t("mc_reserved"),
                    format!("{} {}", trim_num(c.reserved), unit),
                    theme::muted(),
                );
                item(ui, t("mc_price"), money(c.catalog_price), theme::text());
                if let Some(change) = c.price_change {
                    item(
                        ui,
                        t("mc_last_price"),
                        format!("{} ({change:+.0}%)", money(c.last_price)),
                        if change.abs() > 10.0 {
                            theme::warn()
                        } else {
                            theme::muted()
                        },
                    );
                }
            });

            // ---------- Sarf (TZ XII.22) ----------
            if c.norm_total > 0.0 || c.fact_total > 0.0 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "{}: {} / {} {} · {}: {}",
                        t("mc_use"),
                        trim_num(c.fact_total),
                        trim_num(c.norm_total),
                        unit,
                        t("mc_overuse"),
                        trim_num(c.overuse())
                    ))
                    .size(12.0)
                    .color(if c.overuse() > 0.0 {
                        theme::warn()
                    } else {
                        theme::muted()
                    }),
                );
            }

            // ---------- Yaqin ehtiyoj (TZ XII.25-27) ----------
            if c.needed_soon > 0.0 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "{}: {} {} · {}: {} · {}: {}",
                        t("mc_needed"),
                        trim_num(c.needed_soon),
                        unit,
                        t("mc_short"),
                        trim_num(c.short()),
                        t("mc_need_by"),
                        c.need_by
                            .map(|d| d.format("%d.%m.%Y").to_string())
                            .unwrap_or_else(|| t("dash").to_string())
                    ))
                    .size(12.0)
                    .color(if c.short() > 0.0 {
                        theme::danger()
                    } else {
                        theme::muted()
                    }),
                );
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        t("mc_waiting"),
                        c.waiting_tasks
                            .iter()
                            .filter_map(|id| app.task(*id))
                            .map(|x| super::issues::truncate(&x.name, 22))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .size(11.0)
                    .color(theme::muted()),
                );
            }

            // ---------- Yetkazib beruvchilar (TZ XII.18) ----------
            if !c.suppliers.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("mc_suppliers"))
                        .size(12.0)
                        .strong()
                        .color(theme::muted()),
                );
                for (name, deals, amount) in c.suppliers.iter().take(4) {
                    ui.label(
                        RichText::new(format!(
                            "· {} — {} {} · {}",
                            super::issues::truncate(name, 26),
                            deals,
                            t("mat_maker_deals"),
                            money(*amount)
                        ))
                        .size(11.5),
                    );
                }
            }

            // ---------- Almashtiruvchilar (TZ XII.11) ----------
            if !c.alternatives.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("mc_alternatives"))
                        .size(12.0)
                        .strong()
                        .color(theme::muted()),
                );
                for (id, price, free) in &c.alternatives {
                    let cheaper = *price > 0.0 && c.catalog_price > 0.0 && *price < c.catalog_price;
                    ui.label(
                        RichText::new(format!(
                            "· {} — {} · {}: {}",
                            super::issues::truncate(&material_label(app, *id), 26),
                            money(*price),
                            t("mc_free"),
                            trim_num(*free)
                        ))
                        .size(11.5)
                        .color(if cheaper {
                            theme::ok()
                        } else {
                            theme::text()
                        }),
                    );
                }
            }
        });
}

// ================================================================ Loyihaga moslik

/// Material kartochkasi loyiha talabiga javob beradimi (TZ XII.8, 15, 32).
///
/// Ishlatilgan material oldinda turadi: qog'ozdagi kamchilik bilan
/// devordagi kamchilik bir xil og'irlikda emas.
fn fit_tab(ui: &mut egui::Ui, app: &mut App) {
    let fits = app.material_fit();
    let critical = fits.iter().filter(|f| f.critical()).count();
    let used = fits.iter().filter(|f| f.used).count();

    ui.label(
        RichText::new(t("mat_fit_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("mat_fit_critical"),
                critical.to_string(),
                t("mat_fit_critical_hint"),
                if critical == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("mat_fit_used"),
                used.to_string(),
                t("mat_fit_used_hint"),
                theme::text(),
            ),
            stat(
                t("mat_fit_total"),
                fits.len().to_string(),
                t("mat_fit_total_hint"),
                theme::text(),
            ),
        ],
    );
    ui.add_space(12.0);

    if fits.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("mat_fit_ok")).color(theme::ok()).size(15.0));
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for f in &fits {
                let colour = if f.critical() {
                    theme::danger()
                } else if f.used {
                    theme::warn()
                } else {
                    theme::muted()
                };
                egui::Frame::new()
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(super::issues::truncate(&f.name, 44))
                                    .size(12.5)
                                    .strong()
                                    .color(colour),
                            );
                            if f.used {
                                ui.label(
                                    RichText::new(t("mat_fit_in_use"))
                                        .size(11.0)
                                        .color(theme::warn()),
                                );
                            }
                        });
                        for p in &f.problems {
                            ui.label(
                                RichText::new(format!("· {}", fit_text(p)))
                                    .size(12.0)
                                    .color(if p.severe() {
                                        theme::danger()
                                    } else {
                                        theme::muted()
                                    }),
                            );
                        }
                    });
                ui.add_space(6.0);
            }
        });
}

/// Moslik e'tirozini odam o'qiydigan gapga aylantiradi.
fn fit_text(p: &crate::checks::FitProblem) -> String {
    use crate::checks::FitProblem as P;
    match p {
        P::NoSpecRef => t("fp_no_spec_ref").to_string(),
        P::NoEstimateCode => t("fp_no_estimate_code").to_string(),
        P::NoSpec => t("fp_no_spec").to_string(),
        P::SpecialNotInSpec { special } => format!("{} — {}", t("fp_special"), special),
        P::NoCertificate => t("fp_no_cert").to_string(),
        P::CertExpired { days } => format!("{} ({})", t("fp_cert_expired"), days),
        P::CertExpiring { days } => format!("{} ({})", t("fp_cert_expiring"), days),
        P::BanWithoutReason => t("fp_ban_no_reason").to_string(),
        P::NoSection => t("fp_no_section").to_string(),
    }
}

// ================================================================ Komplekt

/// Ish uchun material komplekti va yetkazib beruvchilarni solishtirish
/// (TZ XII.33-34).
fn kits_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let kits = app.material_kits();
    let makers = app.maker_comparison();
    let incomplete = kits.iter().filter(|k| !k.complete()).count();

    ui.label(
        RichText::new(t("mat_kit_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("mat_kit_incomplete"),
                incomplete.to_string(),
                t("mat_kit_incomplete_hint"),
                if incomplete == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("mat_kit_total"),
                kits.len().to_string(),
                t("mat_kit_total_hint"),
                theme::text(),
            ),
            stat(
                t("mat_kit_makers"),
                makers.len().to_string(),
                t("mat_kit_makers_hint"),
                theme::text(),
            ),
        ],
    );
    ui.add_space(12.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if kits.is_empty() {
                ui.label(
                    RichText::new(t("mat_kit_empty"))
                        .color(theme::muted())
                        .size(12.5),
                );
            } else {
                egui::Grid::new("mat_kits")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 280.0, t("col_task"));
                        head_r(ui, 90.0, t("col_days_left"));
                        head_r(ui, 110.0, t("mat_kit_ready"));
                        head_r(ui, 90.0, t("mat_kit_count"));
                        head_r(ui, 90.0, t("mat_kit_missing"));
                        head_l(ui, 220.0, t("mat_kit_worst"));
                        ui.end_row();

                        for k in &kits {
                            let name = app
                                .task(k.task_id)
                                .map(|x| format!("{} {}", x.wbs, x.name))
                                .unwrap_or_default();
                            cell_l(
                                ui,
                                280.0,
                                RichText::new(super::issues::truncate(&name, 38)).size(12.5),
                            );
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(k.days_left.to_string()).size(12.0).color(
                                    if k.days_left < 0 {
                                        theme::danger()
                                    } else {
                                        theme::muted()
                                    },
                                ),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(format!("{:.0}%", k.ready_pct))
                                    .size(12.5)
                                    .strong()
                                    .color(if k.complete() {
                                        theme::ok()
                                    } else {
                                        theme::warn()
                                    }),
                            );
                            cell_r(ui, 90.0, RichText::new(k.total.to_string()).size(12.0));
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(k.missing.to_string()).size(12.0).color(
                                    if k.missing == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_l(
                                ui,
                                220.0,
                                RichText::new(
                                    k.worst
                                        .map(|id| material_label(app, id))
                                        .unwrap_or_else(|| t("dash").to_string()),
                                )
                                .size(12.0)
                                .color(theme::muted()),
                            );
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(16.0);
            ui.label(RichText::new(t("mat_kit_makers")).size(13.5).strong());
            ui.label(
                RichText::new(t("mat_maker_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if makers.is_empty() {
                ui.label(
                    RichText::new(t("mat_maker_empty"))
                        .color(theme::muted())
                        .size(12.5),
                );
            }
            for c in &makers {
                egui::Frame::new()
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}, {} · {} {:.0}%",
                                super::issues::truncate(&c.title, 40),
                                c.unit,
                                t("mat_maker_spread"),
                                c.spread_pct
                            ))
                            .size(12.5)
                            .strong(),
                        );
                        ui.add_space(4.0);
                        egui::Grid::new(format!("mk_{}", c.title))
                            .num_columns(5)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                for o in &c.offers {
                                    cell_l(
                                        ui,
                                        200.0,
                                        RichText::new(super::issues::truncate(&o.supplier, 26))
                                            .size(12.0)
                                            .color(if o.chosen {
                                                theme::ok()
                                            } else {
                                                theme::text()
                                            }),
                                    );
                                    cell_r(ui, 130.0, RichText::new(money(o.price)).size(12.0));
                                    cell_r(
                                        ui,
                                        90.0,
                                        RichText::new(if o.over_pct < 0.01 {
                                            t("mat_maker_best").to_string()
                                        } else {
                                            format!("+{:.0}%", o.over_pct)
                                        })
                                        .size(12.0)
                                        .color(
                                            if o.over_pct < 0.01 {
                                                theme::ok()
                                            } else {
                                                theme::muted()
                                            },
                                        ),
                                    );
                                    cell_r(
                                        ui,
                                        90.0,
                                        RichText::new(format!("{} {}", o.delivery_days, t("days")))
                                            .size(12.0)
                                            .color(theme::muted()),
                                    );
                                    cell_r(
                                        ui,
                                        110.0,
                                        RichText::new(format!(
                                            "{} {}",
                                            o.purchases,
                                            t("mat_maker_deals")
                                        ))
                                        .size(11.5)
                                        .color(theme::muted()),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(6.0);
            }
        });
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

/// Son kiritish maydoni. `DragValue` ishlatilgan — sarf normasi 0.0235 kabi
/// kichik son bo'lishi mumkin, matnli maydon esa uni yaxlitlab yuborardi.
pub fn num_edit(ui: &mut egui::Ui, w: f32, v: &mut f64, speed: f64, max: f64) -> bool {
    ui.add_sized(
        [w, 22.0],
        egui::DragValue::new(v)
            .speed(speed)
            .range(0.0..=max)
            .max_decimals(4)
            // Milliardli summalar ajratkichsiz o'qilmaydi. Kiritishda esa
            // bo'shliqli ham, bo'shliqsiz ham raqam qabul qilinadi.
            .custom_formatter(|n, _| {
                if n.abs() >= 10_000.0 {
                    super::money(n)
                } else {
                    trim_num(n)
                }
            })
            .custom_parser(|s| s.replace([' ', '\u{00a0}'], "").parse().ok()),
    )
    .changed()
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
