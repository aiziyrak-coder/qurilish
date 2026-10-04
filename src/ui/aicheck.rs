//! «Loyiha kalkulyatsiyasi» ekrani (TZ II.12, II.13).
//!
//! Sahifa bitta savolga javob beradi: **loyihaga nimadan qancha ketadi va
//! bu qancha turadi.** Uch qadam, chapdan o'ngga:
//!
//! 1. **Loyiha** — PDF yuklanadi; spetsifikatsiya jadvallari varaqdagi
//!    joylashuvi bo'yicha o'qiladi.
//! 2. **Konstruksiyalar** — loyihadagi konstruksiyalar va ularning soni
//!    (`Фм1 — 20 dona`). Son shu yerda tuzatiladi.
//! 3. **Kalkulyatsiya** — material bo'yicha yig'ma: birlikka ketadigan
//!    miqdor konstruksiya soniga ko'paytirilgan, narx bilan.
//!
//! Hech bir son o'ylab topilmaydi. Konstruksiya soni topilmagan qator
//! birga ko'paytiriladi va «noaniq» deb belgilanadi; narxi yo'q qatorda
//! summa ko'rsatilmaydi.

use super::*;
use crate::app::CheckTab;
use crate::takeoff::Kind;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        empty(ui, t("no_object_selected"));
        return;
    }

    // Fon ipida o'qilayotgan fayl tugagan bo'lsa natijani olamiz.
    app.poll_pdf();

    ui.horizontal_wrapped(|ui| {
        for tab in CheckTab::ALL {
            let ready = match tab {
                CheckTab::Project => true,
                CheckTab::Constructs | CheckTab::Calc => app.takeoff.is_some(),
            };
            let text = if ready {
                RichText::new(tab.label())
            } else {
                RichText::new(tab.label()).color(theme::muted())
            };
            if ui.selectable_label(app.check_tab == tab, text).clicked() {
                app.check_tab = tab;
            }
            if tab != CheckTab::Calc {
                ui.label(RichText::new("→").size(11.0).color(theme::muted()));
            }
        }
    });
    ui.add_space(8.0);

    match app.check_tab {
        CheckTab::Project => project_tab(ui, app),
        CheckTab::Constructs => constructs_tab(ui, app),
        CheckTab::Calc => calc_tab(ui, app),
    }
}

fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(msg).color(theme::muted()).size(15.0));
    });
}

fn card(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
        .show(ui, body);
}

/// Sonni qisqa yozadi: butun bo'lsa kasrsiz.
fn num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Miqdorni o'qilishi oson birlikda ko'rsatadi: 1000 kg dan ortig'i —
/// tonnada.
fn amount(v: f64, unit: &str) -> String {
    if unit == "kg" && v >= 1000.0 {
        format!("{:.2} t", v / 1000.0)
    } else {
        format!("{} {unit}", num(v))
    }
}

fn kind_label(k: Kind) -> &'static str {
    match k {
        Kind::Concrete => t("tk_concrete"),
        Kind::Rebar => t("tk_rebar"),
        Kind::Steel => t("tk_steel"),
        Kind::Other => t("tk_other"),
    }
}

// ================================================================ 1. Loyiha

fn project_tab(ui: &mut egui::Ui, app: &mut App) {
    let busy = app.pdf_job.is_some();
    let mut pick = false;
    let mut clear = false;

    card(ui, |ui| {
        ui.label(RichText::new(t("tk_load_title")).strong().size(14.0));
        ui.add_space(2.0);
        ui.label(
            RichText::new(t("tk_load_hint"))
                .size(11.5)
                .color(theme::muted()),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!busy, egui::Button::new(t("tk_load_button")))
                .clicked()
            {
                pick = true;
            }
            if app.takeoff.is_some()
                && ui
                    .add_enabled(!busy, egui::Button::new(t("tk_clear")))
                    .on_hover_text(t("tk_clear_hint"))
                    .clicked()
            {
                clear = true;
            }
        });

        if let Some(job) = &app.pdf_job {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "{} · {} · {} s",
                    t("pdf_reading"),
                    job.file,
                    job.started.elapsed().as_secs()
                ));
            });
            ui.label(
                RichText::new(t("tk_reading_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            // Vaqt yurib tursin: fon ipi kadr so'ramaydi.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        } else if app.takeoff.is_none() && !app.pdf_note.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(&app.pdf_note).color(theme::danger()));
        }
    });

    if pick {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("tk_load_button"))
            .add_filter("PDF", &["pdf", "PDF"])
            .pick_file()
        {
            app.start_pdf(&path);
        }
    }
    if clear {
        app.clear_takeoff();
        return;
    }

    let Some(tk) = &app.takeoff else {
        return;
    };

    // ---- Nima o'qildi.
    ui.add_space(10.0);
    let lines = app.takeoff_lines.len();
    let unsure = app
        .takeoff_lines
        .iter()
        .filter(|l| l.count.is_none())
        .count();
    ui.horizontal_wrapped(|ui| {
        kpi(
            ui,
            t("tk_file"),
            &tk.file,
            &format!("{} {}", tk.pages, t("tk_pages")),
        );
        kpi(
            ui,
            t("tk_tables"),
            &tk.tables.len().to_string(),
            t("tk_tables_hint"),
        );
        kpi(
            ui,
            t("tab_constructs"),
            &tk.constructs.len().to_string(),
            t("tk_constructs_hint"),
        );
        kpi(
            ui,
            t("tk_lines"),
            &lines.to_string(),
            &format!("{unsure} {}", t("tk_unsure")),
        );
    });

    // ---- O'qilgan jadvallar: odam ularni varaq bilan solishtira olsin.
    ui.add_space(10.0);
    ui.label(RichText::new(t("tk_tables_read")).strong());
    ui.label(
        RichText::new(t("tk_tables_read_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, tb) in tk.tables.iter().enumerate() {
                let title = format!(
                    "{} {} · {} {}",
                    t("pdf_page"),
                    tb.page,
                    tb.rows.len(),
                    t("tk_rows")
                );
                egui::CollapsingHeader::new(title)
                    .id_salt(("tk_table", i))
                    .show(ui, |ui| {
                        egui::Grid::new(("tk_grid", i))
                            .striped(true)
                            .spacing([10.0, 3.0])
                            .show(ui, |ui| {
                                for h in [
                                    "tk_col_pos",
                                    "tk_col_designation",
                                    "tk_col_name",
                                    "tk_col_qty",
                                    "tk_col_mass",
                                    "tk_col_note",
                                ] {
                                    ui.label(RichText::new(t(h)).size(11.0).color(theme::muted()));
                                }
                                ui.end_row();
                                for r in &tb.rows {
                                    for c in
                                        [&r.pos, &r.designation, &r.name, &r.qty, &r.mass, &r.note]
                                    {
                                        ui.label(RichText::new(c).size(12.0));
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            }
        });
}

fn kpi(ui: &mut egui::Ui, title: &str, value: &str, hint: &str) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(170.0);
            ui.set_max_width(260.0);
            ui.label(RichText::new(title).size(11.0).color(theme::muted()));
            ui.label(
                RichText::new(super::issues::truncate(value, 30))
                    .size(18.0)
                    .color(theme::accent()),
            );
            ui.label(RichText::new(hint).size(11.0).color(theme::muted()));
        });
}

// ======================================================= 2. Konstruksiyalar

fn constructs_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(tk) = app.takeoff.clone() else {
        empty(ui, t("tk_not_loaded"));
        return;
    };
    if tk.constructs.is_empty() {
        empty(ui, t("tk_no_constructs"));
        return;
    }
    ui.label(
        RichText::new(t("tk_constructs_explain"))
            .size(11.5)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    let mut edit: Option<(String, f64)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for c in &tk.constructs {
                let key = crate::takeoff::mark_key(&c.mark);
                // Shu konstruksiyaga tegishli material qatorlari.
                let own: Vec<&crate::takeoff::Line> = app
                    .takeoff_lines
                    .iter()
                    .filter(|l| crate::takeoff::mark_key(&l.group).contains(&key))
                    .collect();
                card(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&c.mark).strong().size(15.0));
                        ui.label(RichText::new(&c.name).size(12.5));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{} {}", t("pdf_page"), c.page))
                                    .size(11.0)
                                    .color(theme::muted()),
                            );
                            ui.label(RichText::new(&c.unit).size(12.0));
                            let mut v = c.count;
                            if ui
                                .add(egui::DragValue::new(&mut v).speed(1.0).range(0.0..=1e6))
                                .on_hover_text(t("tk_count_hint"))
                                .changed()
                            {
                                edit = Some((c.mark.clone(), v));
                            }
                        });
                    });
                    if own.is_empty() {
                        ui.label(
                            RichText::new(t("tk_no_spec"))
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        return;
                    }
                    egui::CollapsingHeader::new(format!("{} · {}", t("tk_composition"), own.len()))
                        .id_salt(("tk_c", &c.mark))
                        .show(ui, |ui| {
                            egui::Grid::new(("tk_cg", &c.mark))
                                .striped(true)
                                .spacing([14.0, 3.0])
                                .show(ui, |ui| {
                                    for h in ["tk_col_material", "tk_per_unit", "tk_col_total"] {
                                        ui.label(
                                            RichText::new(t(h)).size(11.0).color(theme::muted()),
                                        );
                                    }
                                    ui.end_row();
                                    for l in &own {
                                        ui.label(RichText::new(&l.item.material).size(12.0));
                                        ui.label(
                                            RichText::new(amount(
                                                l.item.amount * l.sub,
                                                &l.item.unit,
                                            ))
                                            .size(12.0),
                                        );
                                        ui.label(
                                            RichText::new(amount(l.total(), &l.item.unit))
                                                .size(12.0)
                                                .strong(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
                ui.add_space(6.0);
            }
        });
    if let Some((mark, v)) = edit {
        app.set_construct_count(&mark, v);
    }
}

// ========================================================= 3. Kalkulyatsiya

fn calc_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.takeoff.is_none() {
        empty(ui, t("tk_not_loaded"));
        return;
    }
    if app.cost_rows.is_empty() {
        empty(ui, t("tk_no_materials"));
        return;
    }

    // ---- Yig'ma ko'rsatkichlar.
    let sum_of = |k: Kind, unit: &str| -> f64 {
        app.cost_rows
            .iter()
            .filter(|r| r.total.kind == k && r.total.unit == unit)
            .map(|r| r.total.amount)
            .sum()
    };
    let priced: Vec<f64> = app.cost_rows.iter().filter_map(|r| r.sum()).collect();
    let cost: f64 = priced.iter().sum();
    let no_price = app.cost_rows.len() - priced.len();
    let unsure: usize = app.cost_rows.iter().map(|r| r.total.unsure).sum();

    ui.horizontal_wrapped(|ui| {
        kpi(
            ui,
            t("tk_concrete"),
            &amount(sum_of(Kind::Concrete, "m3"), "m3"),
            t("tk_all_classes"),
        );
        kpi(
            ui,
            t("tk_rebar"),
            &amount(sum_of(Kind::Rebar, "kg"), "kg"),
            t("tk_all_diameters"),
        );
        kpi(
            ui,
            t("tk_steel"),
            &amount(sum_of(Kind::Steel, "kg"), "kg"),
            t("tk_steel_hint"),
        );
        kpi(
            ui,
            t("tk_cost"),
            &if priced.is_empty() {
                t("dash").to_string()
            } else {
                money(cost)
            },
            &format!("{no_price} {}", t("tk_no_price")),
        );
    });
    if unsure > 0 {
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("{unsure} {}", t("tk_unsure_note")))
                .size(11.5)
                .color(theme::warn()),
        );
    }
    ui.add_space(10.0);

    let mut edit: Option<(String, f64)> = None;
    let rows = app.cost_rows.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("tk_calc")
                .striped(true)
                .spacing([16.0, 5.0])
                .show(ui, |ui| {
                    for h in [
                        "tk_col_kind",
                        "tk_col_material",
                        "tk_col_amount",
                        "tk_col_lines",
                        "tk_col_price",
                        "tk_col_sum",
                    ] {
                        ui.label(RichText::new(t(h)).size(11.0).color(theme::muted()));
                    }
                    ui.end_row();

                    for r in &rows {
                        ui.label(
                            RichText::new(kind_label(r.total.kind))
                                .size(11.5)
                                .color(theme::muted()),
                        );
                        ui.label(RichText::new(&r.total.material).size(12.5));
                        ui.label(
                            RichText::new(amount(r.total.amount, &r.total.unit))
                                .size(12.5)
                                .strong(),
                        );
                        // Nechta qatordan yig'ilgani va nechtasi noaniq.
                        let src = if r.total.unsure > 0 {
                            RichText::new(format!(
                                "{} · {} {}",
                                r.total.lines,
                                r.total.unsure,
                                t("tk_unsure")
                            ))
                            .size(11.0)
                            .color(theme::warn())
                        } else {
                            RichText::new(r.total.lines.to_string())
                                .size(11.0)
                                .color(theme::muted())
                        };
                        ui.label(src);

                        let mut p = r.price.unwrap_or(0.0);
                        let resp = ui
                            .add(egui::DragValue::new(&mut p).speed(1000.0).range(0.0..=1e12))
                            .on_hover_text(if r.manual {
                                t("tk_price_manual")
                            } else if r.price.is_some() {
                                t("tk_price_book")
                            } else {
                                t("tk_price_none")
                            });
                        if resp.changed() {
                            edit = Some((r.total.material.clone(), p));
                        }
                        match r.sum() {
                            Some(v) => ui.label(RichText::new(money(v)).size(12.5)),
                            None => ui.label(RichText::new(t("dash")).color(theme::muted())),
                        };
                        ui.end_row();
                    }
                });
        });
    if let Some((material, price)) = edit {
        app.set_takeoff_price(&material, price);
    }
}
