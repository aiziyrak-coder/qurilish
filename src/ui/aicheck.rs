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
    let mut toggle: Option<(usize, bool)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, tb) in tk.tables.iter().enumerate() {
                let mut title = format!(
                    "{} {} · {} {}",
                    t("pdf_page"),
                    tb.page,
                    tb.rows.len(),
                    t("tk_rows")
                );
                // Varaq bitta konstruksiyaga bag'ishlangan bo'lsa — qaysi
                // songa ko'paytirilgani ko'rinib tursin.
                if let Some(c) = tk
                    .constructs
                    .iter()
                    .find(|c| !tb.owner.is_empty() && c.mark == tb.owner)
                {
                    title.push_str(&format!(" · {} ×{}", c.mark, num(c.count)));
                }
                if tb.off {
                    title.push_str(&format!(" · {}", t("tk_table_off")));
                }
                ui.horizontal_top(|ui| {
                    let mut on = !tb.off;
                    let ticked = ui
                        .checkbox(&mut on, "")
                        .on_hover_text(t("tk_table_on_hint"))
                        .changed();
                    if ticked {
                        toggle = Some((i, !on));
                    }
                    ui.vertical(|ui| {
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
                                            ui.label(
                                                RichText::new(t(h))
                                                    .size(11.0)
                                                    .color(theme::muted()),
                                            );
                                        }
                                        ui.end_row();
                                        for r in &tb.rows {
                                            for c in [
                                                &r.pos,
                                                &r.designation,
                                                &r.name,
                                                &r.qty,
                                                &r.mass,
                                                &r.note,
                                            ] {
                                                ui.label(RichText::new(c).size(12.0));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                });
            }
        });
    if let Some((i, off)) = toggle {
        app.set_table_off(i, off);
    }
}

fn kpi(ui: &mut egui::Ui, title: &str, value: &str, hint: &str) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            // Ichki joylashuv vertikal: tashqi qator gorizontal bo'lgani
            // uchun aks holda sarlavha, son va izoh bir qatorga siqilardi.
            ui.vertical(|ui| {
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
    let hinted = app
        .cost_rows
        .iter()
        .filter(|r| r.orientir.is_some())
        .count();

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
                format!("{} {}", money(cost), t("tk_sum_unit"))
            },
            &format!("{no_price} {}", t("tk_no_price")),
        );
    });
    if hinted > 0 {
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("{hinted} {}", t("tk_orientir_note")))
                .size(11.5)
                .color(theme::warn()),
        );
    }
    if unsure > 0 {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{unsure} {}", t("tk_unsure_note")))
                .size(11.5)
                .color(theme::warn()),
        );
    }
    if !app.takeoff_repeats.is_empty() {
        ui.add_space(4.0);
        let list = app
            .takeoff_repeats
            .iter()
            .map(|(name, page)| format!("{} ({} {page})", name.trim(), t("pdf_page")))
            .collect::<Vec<_>>()
            .join(", ");
        ui.label(
            RichText::new(format!("{}: {list}", t("tk_repeats")))
                .size(11.5)
                .color(theme::muted()),
        );
    }
    ui.add_space(10.0);

    let mut edit: Option<(String, f64)> = None;
    let mut open: Option<String> = None;
    let rows = app.cost_rows.clone();
    let shown = app.takeoff_open.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("tk_calc")
                .striped(true)
                .spacing([16.0, 5.0])
                .show(ui, |ui| {
                    for h in [
                        "tk_col_no",
                        "tk_col_material",
                        "tk_col_amount",
                        "tk_col_price",
                        "tk_col_sum",
                        "tk_col_source",
                    ] {
                        ui.label(RichText::new(t(h)).size(11.0).color(theme::muted()));
                    }
                    ui.end_row();

                    // Har tur — alohida bo'lim, o'z jami bilan.
                    let mut no = 0;
                    for kind in [Kind::Concrete, Kind::Rebar, Kind::Steel, Kind::Other] {
                        let part: Vec<&crate::app::CostRow> =
                            rows.iter().filter(|r| r.total.kind == kind).collect();
                        if part.is_empty() {
                            continue;
                        }
                        ui.label("");
                        ui.label(
                            RichText::new(kind_label(kind).to_uppercase())
                                .size(12.0)
                                .strong()
                                .color(theme::accent()),
                        );
                        for _ in 0..4 {
                            ui.label("");
                        }
                        ui.end_row();

                        for r in &part {
                            no += 1;
                            ui.label(
                                RichText::new(no.to_string())
                                    .size(11.0)
                                    .color(theme::muted()),
                            );
                            let is_open = shown.as_deref() == Some(r.total.material.as_str());
                            let mark = if is_open { "−" } else { "+" };
                            let name = ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(format!("{mark} {}", r.total.material))
                                            .size(12.5),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text(t("tk_detail_open"));
                            if name.clicked() {
                                open = Some(r.total.material.clone());
                            }
                            ui.label(
                                RichText::new(amount(r.total.amount, &r.total.unit))
                                    .size(12.5)
                                    .strong(),
                            );
                            let mut p = r.price.unwrap_or(0.0);
                            let resp = ui
                                .horizontal(|ui| {
                                    let resp = ui.add(
                                        egui::DragValue::new(&mut p)
                                            .speed(1000.0)
                                            .range(0.0..=1e12),
                                    );
                                    ui.label(
                                        RichText::new(format!("/{}", r.total.unit))
                                            .size(11.0)
                                            .color(theme::muted()),
                                    );
                                    resp
                                })
                                .inner;
                            if resp.changed() {
                                edit = Some((r.total.material.clone(), p));
                            }
                            match r.sum() {
                                Some(v) => ui.label(RichText::new(money(v)).size(12.5)),
                                None => ui.label(RichText::new(t("dash")).color(theme::muted())),
                            };
                            // Narx qayerdan kelgani har qatorda ko'rinadi.
                            let src = if r.manual {
                                RichText::new(t("tk_src_manual")).color(theme::muted())
                            } else if let Some(s) = r.orientir {
                                RichText::new(format!("{} · {s}", t("tk_src_orientir")))
                                    .color(theme::warn())
                            } else if r.price.is_some() {
                                RichText::new(t("tk_src_book")).color(theme::muted())
                            } else {
                                RichText::new(t("tk_src_none")).color(theme::warn())
                            };
                            ui.label(src.size(11.0));
                            ui.end_row();

                            if is_open {
                                detail_rows(ui, app, &r.total.material, &r.total.unit);
                            }
                        }

                        // Bo'lim jami: miqdor bir xil birlikda bo'lsagina
                        // qo'shiladi.
                        let unit = &part[0].total.unit;
                        let same = part.iter().all(|r| &r.total.unit == unit);
                        let qty: f64 = part.iter().map(|r| r.total.amount).sum();
                        let sums: Vec<f64> = part.iter().filter_map(|r| r.sum()).collect();
                        ui.label("");
                        ui.label(
                            RichText::new(format!("{} — {}", t("tk_subtotal"), kind_label(kind)))
                                .size(12.0)
                                .strong(),
                        );
                        ui.label(if same {
                            RichText::new(amount(qty, unit)).size(12.5).strong()
                        } else {
                            RichText::new("")
                        });
                        ui.label("");
                        ui.label(if sums.is_empty() {
                            RichText::new(t("dash")).color(theme::muted())
                        } else {
                            RichText::new(money(sums.iter().sum())).size(12.5).strong()
                        });
                        let missing = part.len() - sums.len();
                        ui.label(if missing > 0 {
                            RichText::new(format!("{missing} {}", t("tk_no_price")))
                                .size(11.0)
                                .color(theme::warn())
                        } else {
                            RichText::new("")
                        });
                        ui.end_row();
                    }
                });

            // ---- Umumiy jami.
            ui.add_space(10.0);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("tk_grand_total")).size(13.0).strong());
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(if priced.is_empty() {
                            t("dash").to_string()
                        } else {
                            format!("{} {}", money(cost), t("tk_sum_unit"))
                        })
                        .size(20.0)
                        .strong()
                        .color(theme::accent()),
                    );
                });
                // Jami to'liq emasligi yashirilmaydi.
                ui.label(if no_price > 0 {
                    RichText::new(format!("{no_price} {}", t("tk_total_partial")))
                        .size(11.5)
                        .color(theme::warn())
                } else {
                    RichText::new(t("tk_total_full"))
                        .size(11.5)
                        .color(theme::muted())
                });
            });
            ui.add_space(8.0);
        });
    if let Some((material, price)) = edit {
        app.set_takeoff_price(&material, price);
    }
    if let Some(material) = open {
        app.takeoff_open = if app.takeoff_open.as_deref() == Some(material.as_str()) {
            None
        } else {
            Some(material)
        };
    }
}

/// Material qayerdan yig'ilgani: har konstruksiya uchun hisob formulasi.
fn detail_rows(ui: &mut egui::Ui, app: &App, material: &str, unit: &str) {
    for l in app
        .takeoff_lines
        .iter()
        .filter(|l| l.item.material == material && l.item.unit == unit)
    {
        ui.label("");
        let group = if l.group.trim().is_empty() {
            t("dash").to_string()
        } else {
            super::issues::truncate(l.group.trim(), 48)
        };
        ui.label(
            RichText::new(format!("      {} {} · {group}", t("pdf_page"), l.page))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.label(
            RichText::new(amount(l.total(), unit))
                .size(11.0)
                .color(theme::muted()),
        );
        // Formula: bir donaga × yig'ma birlik × konstruksiya soni.
        let formula = match l.count {
            Some(c) => {
                let mut f = format!("{} {unit}", num(l.item.amount));
                if (l.sub - 1.0).abs() > 1e-9 {
                    f.push_str(&format!(" × {}", num(l.sub)));
                }
                format!("{f} × {}", num(c))
            }
            None => t("tk_detail_as_written").to_string(),
        };
        ui.label(
            RichText::new(formula)
                .size(11.0)
                .color(if l.count.is_some() {
                    theme::muted()
                } else {
                    theme::warn()
                }),
        );
        ui.label("");
        ui.label("");
        ui.end_row();
    }
}
