//! «AI smeta tekshiruvi» ekrani (TZ III).
//!
//! Smeta pozitsiyalari kiritiladi yoki namoyish ma'lumotidan olinadi, so'ng
//! qoidalar arifmetika, birliklar, dublikatlar, hajmlar va narxlarni tekshiradi.

use super::issues;
use super::*;
use crate::domain::{Estimate, EstimateItem, IssueModule};
use crate::model::Section;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(18.0),
            );
        });
        return;
    }

    app.auto_check(IssueModule::Estimate);

    // Ekran ichidagi bo'lim: pozitsiyalar yoki tekshiruv natijasi.
    let tab_key = egui::Id::new("estimate_tab_v2");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    header(ui, app);
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let tabs = [
            (0u8, t("tab_estimate_items")),
            (1, t("tab_cost_control")),
            (2, t("tab_issues")),
        ];
        for (i, label) in tabs {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => cost_tab(ui, app),
        2 => issues_tab(ui, app),
        _ => items(ui, app),
    }
}

// ================================================================ COST CONTROL

/// TZ III.31: smetaning pul ko'rinishidagi xulosasi — nima qancha turadi va
/// qayerda tekshirishga arziydigan summa bor.
fn cost_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(est) = app.estimate().cloned() else {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new(t("no_estimates")).color(theme::muted()));
        });
        return;
    };
    if app.estimate_items.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new(t("cc_no_items")).color(theme::muted()));
        });
        return;
    }

    let s = app.cost_summary();
    let cur = est.currency.clone();
    // Tekshirishga arziydigan umumiy summa: dublikat, hajm oshig'i va arifmetika.
    let attention = s.duplicate_cost + s.volume_excess + s.arithmetic_diff.max(0.0);
    let pct = if s.total > 0.0 {
        attention / s.total * 100.0
    } else {
        0.0
    };

    stat_row(
        ui,
        vec![
            stat(t("cc_total"), money(s.total), &cur, theme::text()),
            stat(
                t("cc_attention"),
                money(attention),
                &format!("{pct:.1} % {}", t("cc_of_total")),
                if attention > 0.0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("cc_duplicates"),
                s.duplicate_count.to_string(),
                &format!("{} {cur}", money(s.duplicate_cost)),
                if s.duplicate_count == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("cc_mismatch"),
                s.mismatch_count.to_string(),
                t("cc_mismatch_hint"),
                if s.mismatch_count == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("cc_saving"),
                money(s.price_saving),
                t("cc_saving_hint"),
                if s.price_saving > 0.0 {
                    theme::accent()
                } else {
                    theme::muted()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    let w = (ui.available_width() - 26.0).min(1000.0);

    // ---- Batafsil: har bir raqam qanday hisoblangani ----
    card_frame(ui, t("cc_breakdown"), w, |ui| {
        let row = |ui: &mut egui::Ui, label: &str, value: f64, hint: &str, danger: bool| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [300.0, 20.0],
                    egui::Label::new(RichText::new(label).size(13.0)),
                );
                ui.add_sized(
                    [180.0, 20.0],
                    egui::Label::new(
                        RichText::new(format!("{} {}", money(value), cur))
                            .size(13.0)
                            .strong()
                            .color(if danger && value.abs() > 0.5 {
                                theme::danger()
                            } else {
                                theme::text()
                            }),
                    ),
                );
                ui.label(RichText::new(hint).size(11.5).color(theme::muted()));
            });
            ui.add_space(2.0);
        };

        row(
            ui,
            t("cc_items_sum"),
            s.total,
            t("cc_items_sum_hint"),
            false,
        );
        row(
            ui,
            t("cc_declared_diff"),
            s.declared_diff,
            t("cc_declared_diff_hint"),
            true,
        );
        row(
            ui,
            t("cc_arithmetic"),
            s.arithmetic_diff,
            t("cc_arithmetic_hint"),
            true,
        );
        row(
            ui,
            t("cc_volume_excess"),
            s.volume_excess,
            t("cc_volume_excess_hint"),
            true,
        );
        row(
            ui,
            t("cc_duplicate_cost"),
            s.duplicate_cost,
            t("cc_duplicate_cost_hint"),
            true,
        );
        row(
            ui,
            t("cc_price_saving"),
            s.price_saving,
            t("cc_price_saving_hint"),
            false,
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new(t("cc_disclaimer"))
                .size(11.0)
                .color(theme::muted()),
        );
    });

    ui.add_space(10.0);

    // ---- Bo'limlar kesimida qiymat ----
    card_frame(ui, t("cc_by_section"), w, |ui| {
        use crate::model::Section;
        let mut rows: Vec<(Section, f64, usize)> = Vec::new();
        for sec in Section::ALL {
            let items: Vec<_> = app
                .estimate_items
                .iter()
                .filter(|i| i.section == sec)
                .collect();
            if items.is_empty() {
                continue;
            }
            rows.push((sec, items.iter().map(|i| i.cost).sum(), items.len()));
        }
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let max = rows.first().map(|r| r.1).unwrap_or(1.0).max(1.0);

        for (sec, sum, n) in &rows {
            let col = theme::section_color(sec.color());
            ui.horizontal(|ui| {
                ui.add_sized(
                    [46.0, 18.0],
                    egui::Label::new(RichText::new(sec.label()).color(col).strong().size(12.5)),
                );
                ui.add_sized(
                    [64.0, 18.0],
                    egui::Label::new(
                        RichText::new(format!("{n} {}", t("estimate_items_count")))
                            .size(11.0)
                            .color(theme::muted()),
                    ),
                );
                let bar_w = (ui.available_width() - 210.0).clamp(80.0, 420.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(bar_w, 12.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(rect, 3.0, theme::track());
                p.rect_filled(
                    egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(rect.width() * (*sum / max) as f32, rect.height()),
                    ),
                    3.0,
                    col,
                );
                ui.label(
                    RichText::new(format!("{} {}", money(*sum), cur))
                        .size(12.0)
                        .color(theme::text()),
                );
                if s.total > 0.0 {
                    ui.label(
                        RichText::new(format!("{:.0}%", sum / s.total * 100.0))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                }
            });
            ui.add_space(2.0);
        }
    });
    ui.add_space(20.0);
}

// ---------------------------------------------------------------- Sarlavha

fn header(ui: &mut egui::Ui, app: &mut App) {
    let mut add_estimate = false;
    let mut import_from: Option<std::path::PathBuf> = None;
    let mut pick: Option<i64> = None;
    let mut edited: Option<Estimate> = None;
    let mut delete: Option<i64> = None;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t("estimate_label")).color(theme::muted()));
        egui::ComboBox::from_id_salt("estimate_picker")
            .selected_text(
                app.estimate()
                    .map(|e| e.name.clone())
                    .unwrap_or_else(|| t("no_estimates").to_string()),
            )
            .width(320.0)
            .show_ui(ui, |ui| {
                for e in &app.estimates {
                    if ui
                        .selectable_label(app.current_estimate == Some(e.id), &e.name)
                        .clicked()
                    {
                        pick = Some(e.id);
                    }
                }
            });
        if ui
            .button(RichText::new(t("import_estimate")).strong())
            .on_hover_text(t("import_hint"))
            .clicked()
        {
            import_from = rfd::FileDialog::new()
                .add_filter(
                    t("import_file_filter"),
                    &["xlsx", "xlsm", "xls", "xlsb", "ods", "csv"],
                )
                .pick_file();
        }
        if ui.button(t("add_estimate")).clicked() {
            add_estimate = true;
        }
        if app.current_estimate.is_some()
            && ui
                .button(RichText::new(t("delete_estimate")).color(theme::danger()))
                .clicked()
        {
            delete = app.current_estimate;
        }
    });

    if let Some(est) = app.estimate().cloned() {
        let mut e = est.clone();
        let computed: f64 = app.estimate_items.iter().map(|i| i.cost).sum();
        let diff = computed - e.declared_total;
        let mut dirty = false;

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [120.0, 20.0],
                egui::Label::new(
                    RichText::new(t("estimate_name"))
                        .color(theme::muted())
                        .size(12.0),
                ),
            );
            dirty |= ui
                .add(egui::TextEdit::singleline(&mut e.name).desired_width(300.0))
                .changed();
            ui.add_space(12.0);
            ui.label(
                RichText::new(t("estimate_declared"))
                    .color(theme::muted())
                    .size(12.0),
            );
            dirty |= ui
                .add(
                    egui::DragValue::new(&mut e.declared_total)
                        .speed(100_000.0)
                        .range(0.0..=f64::MAX),
                )
                .changed();
            dirty |= ui
                .add(egui::TextEdit::singleline(&mut e.currency).desired_width(60.0))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.add_space(120.0);
            ui.label(
                RichText::new(format!(
                    "{}: {} {}   ·   {}: {} {}   ·   {}: {} {}",
                    t("estimate_computed"),
                    money(computed),
                    e.currency,
                    t("estimate_declared"),
                    money(e.declared_total),
                    e.currency,
                    t("chk_diff"),
                    money(diff),
                    e.currency
                ))
                .size(12.0)
                .color(if diff.abs() > 1.0 {
                    theme::danger()
                } else {
                    theme::ok()
                }),
            );
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    app.estimate_items.len(),
                    t("estimate_items_count")
                ))
                .size(12.0)
                .color(theme::muted()),
            );
        });

        if dirty {
            edited = Some(e);
        }
    }

    if let Some(path) = import_from {
        app.import_estimate(&path);
    }
    if let Some(id) = pick {
        app.current_estimate = Some(id);
        app.reload_estimate_items();
    }
    if let Some(e) = edited {
        app.db.update_estimate(&e);
        if let Some(slot) = app.estimates.iter_mut().find(|x| x.id == e.id) {
            *slot = e;
        }
    }
    if add_estimate {
        if let Some(pid) = app.current {
            let e = Estimate {
                id: 0,
                project_id: pid,
                name: t("estimate_new_name").to_string(),
                currency: app.settings.default_currency.clone(),
                declared_total: 0.0,
                added_at: String::new(),
            };
            let id = app.db.insert_estimate(&e);
            app.reload_modules();
            if id > 0 {
                app.current_estimate = Some(id);
                app.reload_estimate_items();
            }
        }
    }
    if let Some(id) = delete {
        app.db.del("estimate", id);
        app.current_estimate = None;
        app.reload_modules();
    }
}

// ---------------------------------------------------------------- Pozitsiyalar

fn items(ui: &mut egui::Ui, app: &mut App) {
    let Some(eid) = app.current_estimate else {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new(t("no_estimates")).color(theme::muted()));
        });
        return;
    };

    let mut edited: Option<EstimateItem> = None;
    let mut removed: Option<i64> = None;
    let mut add = false;

    ui.horizontal(|ui| {
        if ui.button(t("add_item")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("items_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("items_grid")
                .num_columns(11)
                .spacing([8.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 34.0, t("col_pos"));
                    head(ui, 90.0, t("col_code"));
                    head(ui, 260.0, t("col_name"));
                    head(ui, 46.0, t("col_section_short"));
                    head(ui, 64.0, t("col_unit"));
                    head(ui, 80.0, t("col_qty"));
                    head(ui, 100.0, t("col_price"));
                    head(ui, 110.0, t("col_cost"));
                    head(ui, 110.0, t("col_computed"));
                    head(ui, 100.0, t("col_diff"));
                    head(ui, 24.0, "");
                    ui.end_row();

                    for item in &app.estimate_items {
                        let mut it = item.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([34.0, 20.0], egui::DragValue::new(&mut it.pos))
                            .changed();
                        changed |= ui
                            .add_sized([90.0, 20.0], egui::TextEdit::singleline(&mut it.code))
                            .changed();
                        changed |= ui
                            .add_sized([260.0, 20.0], egui::TextEdit::singleline(&mut it.name))
                            .changed();
                        egui::ComboBox::from_id_salt(("it_sec", it.id))
                            .selected_text(it.section.label())
                            .width(46.0)
                            .show_ui(ui, |ui| {
                                for s in Section::ALL {
                                    changed |= ui
                                        .selectable_value(&mut it.section, s, s.label())
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add_sized([64.0, 20.0], egui::TextEdit::singleline(&mut it.unit))
                            .changed();
                        changed |= ui
                            .add_sized([80.0, 20.0], egui::DragValue::new(&mut it.qty).speed(0.1))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [100.0, 20.0],
                                egui::DragValue::new(&mut it.price).speed(100.0),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [110.0, 20.0],
                                egui::DragValue::new(&mut it.cost).speed(100.0),
                            )
                            .changed();

                        let calc = it.computed();
                        let diff = calc - it.cost;
                        ui.add_sized(
                            [110.0, 20.0],
                            egui::Label::new(
                                RichText::new(money(calc)).size(12.0).color(theme::muted()),
                            ),
                        );
                        ui.add_sized(
                            [100.0, 20.0],
                            egui::Label::new(
                                RichText::new(if diff.abs() < 0.5 {
                                    t("dash").to_string()
                                } else {
                                    money(diff)
                                })
                                .size(12.0)
                                .color(if diff.abs() < 0.5 {
                                    theme::muted()
                                } else {
                                    theme::danger()
                                }),
                            ),
                        );
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(it.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(it);
                        }
                    }
                });
        });

    if let Some(it) = edited {
        app.db.update_estimate_item(&it);
        if let Some(slot) = app.estimate_items.iter_mut().find(|x| x.id == it.id) {
            *slot = it;
        }
    }
    if let Some(id) = removed {
        app.db.del("estimate_item", id);
        app.reload_estimate_items();
    }
    if add {
        let pos = app.estimate_items.iter().map(|i| i.pos).max().unwrap_or(0) + 1;
        app.db.insert_estimate_item(&EstimateItem {
            id: 0,
            estimate_id: eid,
            pos,
            section: Section::None,
            code: String::new(),
            name: String::new(),
            unit: String::new(),
            qty: 0.0,
            price: 0.0,
            cost: 0.0,
            note: String::new(),
        });
        app.reload_estimate_items();
    }
}

// ---------------------------------------------------------------- Nomuvofiqliklar

fn issues_tab(ui: &mut egui::Ui, app: &mut App) {
    if issues::filter_bar(ui, app, t("run_estimate_check")) {
        app.run_estimate_check();
    }
    ui.add_space(6.0);

    let list = app.filtered_issues(IssueModule::Estimate);
    issues::issue_kpis(ui, &list, None);
    ui.add_space(6.0);
    issues::section_report(ui, &list);
    drop(list);
    ui.add_space(8.0);

    let h = (ui.available_height() - 12.0).max(180.0);
    let detail_w = (ui.available_width() * 0.34).clamp(300.0, 440.0);

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - detail_w - 16.0).max(260.0));
            if let Some(id) = issues::issue_table(ui, app, IssueModule::Estimate, h) {
                app.selected_issue = Some(id);
            }
        });
        ui.vertical(|ui| {
            ui.set_width(detail_w);
            issues::issue_detail(ui, app, h - 24.0);
        });
    });
}
