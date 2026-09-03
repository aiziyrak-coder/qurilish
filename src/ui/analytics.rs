//! «AI analitika» ekrani (TZ XVII).
//!
//! Barcha modullar bo'yicha bitta ko'rinish: yo'nalishlar kesimidagi
//! ko'rsatkichlar va e'tibor talab qiladigan topilmalar ro'yxati. Topilmalar
//! `analytics` modulida hisoblanadi — ekran faqat ko'rsatadi, o'zi xulosa
//! chiqarmaydi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::analytics::{self, Area, Finding};
use crate::domain::Severity;
use egui::{vec2, Sense};

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

    // Hisoblarni bir marta bajaramiz — ekran bo'ylab bir xil sonlar ishlatiladi.
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, &sales);
    let metrics = analytics::metrics(&inp);
    let found = analytics::findings(&inp);
    let score = analytics::health(&found);

    // Yo'nalish filtri.
    let filter_key = egui::Id::new("an_filter");
    let mut filter = ui
        .data(|d| d.get_temp::<Option<Area>>(filter_key))
        .flatten();

    let mut go: Option<Screen> = None;

    let mut export = false;
    ui.horizontal(|ui| {
        if ui
            .button(t("an_export"))
            .on_hover_text(t("an_export_hint"))
            .clicked()
        {
            export = true;
        }
        ui.label(
            RichText::new(t("analytics_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    header(ui, score, &found);
    ui.add_space(10.0);

    let tab_key = egui::Id::new("an_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("an_tab_findings")),
            (1, t("an_tab_cash")),
            (2, t("an_tab_briefing")),
            (3, t("an_tab_forecast")),
            (4, t("an_tab_scenario")),
            (5, t("an_tab_losses")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(10.0);

    if tab == 1 {
        if let Some(screen) = cash_tab(ui, &inp) {
            go = Some(screen);
        }
        if let Some(screen) = go {
            app.screen = screen;
        }
        return;
    }
    if tab == 2 {
        if let Some(screen) = briefing_tab(ui, &inp) {
            app.screen = screen;
        }
        return;
    }
    if tab == 3 {
        forecast_tab(ui, app);
        return;
    }
    if tab == 4 {
        scenario_tab(ui, app);
        return;
    }
    if tab == 5 {
        if let Some(screen) = losses_tab(ui, app) {
            app.screen = screen;
        }
        return;
    }

    // Kartochka bosilsa — o'sha yo'nalish ekraniga o'tamiz.
    let cards: Vec<Stat> = metrics
        .iter()
        .map(|m| {
            stat(
                &m.title,
                m.value.clone(),
                &m.hint,
                severity_color(m.severity),
            )
            .link()
        })
        .collect();
    if let Some(i) = stat_row(ui, cards) {
        go = metrics.get(i).map(|m| m.area.screen());
    }
    ui.add_space(12.0);

    // Yo'nalish bo'yicha filtr tugmalari.
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(filter.is_none(), t("an_all")).clicked() {
            filter = None;
        }
        for area in Area::ALL {
            let n = found.iter().filter(|f| f.area == *area).count();
            let text = if n > 0 {
                format!("{} · {n}", area.label())
            } else {
                area.label().to_string()
            };
            let color = if n == 0 {
                theme::muted()
            } else {
                theme::text()
            };
            if ui
                .selectable_label(filter == Some(*area), RichText::new(text).color(color))
                .clicked()
            {
                filter = if filter == Some(*area) {
                    None
                } else {
                    Some(*area)
                };
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(filter_key, filter));
    ui.add_space(8.0);

    let list: Vec<&Finding> = found
        .iter()
        .filter(|f| filter.is_none_or(|a| f.area == a))
        .collect();

    if list.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("an_nothing")).color(theme::ok()).size(15.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("an_nothing_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for f in &list {
                    if card(ui, f) {
                        go = Some(f.screen);
                    }
                    ui.add_space(6.0);
                }
            });
    }

    if export {
        // Hisobot ekrandagi bilan bir xil hisobdan chiqadi.
        let name = app
            .project()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| t("dash").to_string());
        let text = analytics::report(&inp, &name);
        let file = format!("qurai-{}.txt", app.today.format("%Y-%m-%d"));
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("an_export"))
            .set_file_name(&file)
            .add_filter("Text", &["txt"])
            .save_file()
        {
            match std::fs::write(&path, text) {
                Ok(()) => app.notify(format!("{} {}", t("an_export_done"), path.display())),
                Err(e) => app.notify(format!("{}: {e}", t("an_export_failed"))),
            }
        }
    }

    if let Some(screen) = go {
        app.screen = screen;
    }
}

/// Sog'lomlik indeksi va daraja bo'yicha hisob.
fn header(ui: &mut egui::Ui, score: i64, found: &[Finding]) {
    let count = |s: Severity| found.iter().filter(|f| f.severity == s).count();
    let critical = count(Severity::Critical);
    let major = count(Severity::Major);
    let warning = count(Severity::Warning);

    let color = if score >= 85 {
        theme::ok()
    } else if score >= 60 {
        theme::warn()
    } else {
        theme::danger()
    };

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Halqa ko'rinishidagi indeks.
                let (rect, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                let p = ui.painter();
                let c = rect.center();
                p.circle_stroke(c, 30.0, Stroke::new(6.0_f32, theme::track()));
                // To'liq aylana 100 ballga to'g'ri keladi.
                let frac = score as f32 / 100.0;
                arc(p, c, 30.0, frac, color);
                p.text(
                    c,
                    Align2::CENTER_CENTER,
                    score.to_string(),
                    FontId::proportional(20.0),
                    color,
                );

                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(t("an_health")).size(14.0).strong());
                    ui.label(
                        RichText::new(t("an_health_hint"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        chip(ui, theme::danger(), critical, t("sev_critical"));
                        chip(ui, theme::warn(), major, t("sev_major"));
                        chip(ui, theme::accent(), warning, t("sev_warning"));
                    });
                });
            });
        });
}

/// Halqaning to'ldirilgan qismi — 12 soatdan boshlab soat yo'nalishida.
fn arc(p: &egui::Painter, c: egui::Pos2, r: f32, frac: f32, color: Color32) {
    let steps = 64;
    let n = (steps as f32 * frac.clamp(0.0, 1.0)).round() as usize;
    if n == 0 {
        return;
    }
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * (i as f32 / steps as f32);
        points.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
    }
    p.add(egui::Shape::line(points, Stroke::new(6.0_f32, color)));
}

fn chip(ui: &mut egui::Ui, color: Color32, n: usize, label: &str) {
    let text = format!("{n} {label}");
    let color = if n == 0 { theme::muted() } else { color };
    ui.label(RichText::new(text).size(11.5).color(color));
    ui.add_space(6.0);
}

// ================================================================ Pul oqimi

/// Oylar kesimida pul oqimi va kassa uzilishi (TZ XVII.30–31).
fn cash_tab(ui: &mut egui::Ui, inp: &analytics::Input) -> Option<Screen> {
    use super::warehouse::{cell_l, cell_r};
    let flow = analytics::cash_flow(inp, 5, 6);
    let gap = analytics::cash_gap(&flow, inp.today);
    let mut go = None;

    // Kassa uzilishi bo'lsa — birinchi navbatda shu.
    match &gap {
        Some(g) => {
            egui::Frame::group(ui.style())
                .fill(theme::card())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(t("cash_gap_title"))
                                .size(13.0)
                                .strong()
                                .color(theme::danger()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} ({} {}) · {} {}",
                                g.month.format("%m.%Y"),
                                g.months_ahead,
                                t("cash_gap_months"),
                                t("cash_gap_amount"),
                                money(g.amount)
                            ))
                            .size(12.5),
                        );
                    });
                    ui.label(
                        RichText::new(t("cash_gap_hint"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                });
        }
        None => {
            ui.label(
                RichText::new(t("cash_gap_none"))
                    .size(12.0)
                    .color(theme::ok()),
            );
        }
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new(t("cash_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("an_cash")
                .num_columns(9)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_month"));
                    head_l(ui, 80.0, t("col_basis"));
                    head_r(ui, 150.0, t("col_income"));
                    head_r(ui, 150.0, t("col_purchases"));
                    head_r(ui, 150.0, t("col_payroll"));
                    head_r(ui, 140.0, t("col_machine_cost"));
                    head_r(ui, 150.0, t("col_expense"));
                    head_r(ui, 150.0, t("col_net"));
                    head_r(ui, 160.0, t("col_cumulative"));
                    ui.end_row();

                    for m in &flow {
                        let now = m.month == analytics::month_of(inp.today);
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(m.month.format("%m.%Y").to_string())
                                .size(12.0)
                                .monospace()
                                .color(if now { theme::accent() } else { theme::text() }),
                        );
                        cell_l(
                            ui,
                            80.0,
                            RichText::new(if m.past { t("col_fact") } else { t("col_plan") })
                                .size(10.5)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(m.income))
                                .size(12.0)
                                .color(if m.income > 0.0 {
                                    theme::ok()
                                } else {
                                    theme::muted()
                                }),
                        );
                        // Chiqim tarkibi ochiq turadi — qayerdan chiqqani ko'rinsin.
                        for (v, screen, w) in [
                            (m.purchases, Screen::Purchases, 150.0),
                            (m.payroll, Screen::Timesheet, 150.0),
                            (m.machines, Screen::Machines, 140.0),
                        ] {
                            let r = cell_r(
                                ui,
                                w,
                                RichText::new(money(v)).size(12.0).color(if v > 0.0 {
                                    theme::text()
                                } else {
                                    theme::muted()
                                }),
                            );
                            if v > 0.0 && r.interact(egui::Sense::click()).clicked() {
                                go = Some(screen);
                            }
                        }
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(m.expense))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(m.net))
                                .size(12.5)
                                .strong()
                                .color(if m.net < 0.0 {
                                    theme::danger()
                                } else {
                                    theme::ok()
                                }),
                        );
                        cell_r(
                            ui,
                            160.0,
                            RichText::new(money(m.balance))
                                .size(12.0)
                                .color(if m.balance < 0.0 {
                                    theme::danger()
                                } else {
                                    theme::muted()
                                }),
                        );
                        ui.end_row();
                    }
                });
        });
    go
}

// ================================================================ Kunlik xulosa

/// Bugungi kun uchun qisqa xulosa (TZ XVII.37).
fn briefing_tab(ui: &mut egui::Ui, inp: &analytics::Input) -> Option<Screen> {
    let lines = analytics::briefing(inp);
    let mut go = None;

    ui.label(
        RichText::new(format!(
            "{} · {}",
            t("br_title"),
            inp.today.format("%d.%m.%Y")
        ))
        .size(14.0)
        .strong(),
    );
    ui.add_space(2.0);
    ui.label(RichText::new(t("br_hint")).size(11.0).color(theme::muted()));
    ui.add_space(10.0);

    if lines.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("br_empty")).color(theme::ok()).size(15.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("br_empty_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return None;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for l in &lines {
                let r = egui::Frame::group(ui.style())
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width() - 4.0);
                        ui.horizontal_wrapped(|ui| {
                            // Rangli nuqta — muhimlik darajasi.
                            let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                            ui.painter().circle_filled(
                                rect.center(),
                                4.0,
                                severity_color(l.severity),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(l.area.label())
                                    .size(11.0)
                                    .color(theme::muted()),
                            );
                            ui.label(RichText::new(&l.text).size(13.0));
                        });
                    })
                    .response;
                if r.interact(Sense::click()).clicked() {
                    go = Some(l.screen);
                }
                ui.add_space(6.0);
            }
        });
    go
}

// ================================================================ Prognoz

/// Obyektning moliyaviy prognozi (TZ XVII.13-14, 32-34).
fn forecast_tab(ui: &mut egui::Ui, app: &mut App) {
    let f = app.finance_forecast();

    ui.label(
        RichText::new(t("an_fc_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if f.progress_pct < 5.0 {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("an_fc_too_early"))
                    .size(15.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("an_fc_too_early_hint"))
                    .size(12.0)
                    .color(theme::muted()),
            );
        });
        return;
    }

    stat_row(
        ui,
        vec![
            stat(
                t("an_fc_contract"),
                money(f.contract),
                t("an_fc_contract_hint"),
                theme::text(),
            ),
            stat(
                t("an_fc_cost"),
                money(f.cost_forecast),
                &format!("{} {}", money(f.cost_now), t("an_fc_cost_now")),
                theme::text(),
            ),
            stat(
                t("an_fc_profit"),
                money(f.profit_forecast),
                &format!("{:.1}% {}", f.margin_pct, t("an_fc_margin")),
                if f.profit_forecast >= 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("an_fc_receivable"),
                money(f.receivable),
                &format!("{} {}", money(f.overdue), t("an_fc_overdue")),
                if f.overdue == 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("an_fc_revenue"),
                money(f.revenue_90),
                t("an_fc_revenue_hint"),
                theme::text(),
            ),
        ],
    );
    ui.add_space(14.0);

    // Hisob ochiq: qaysi son qayerdan chiqqani yozilgan.
    ui.label(RichText::new(t("an_fc_how")).size(13.5).strong());
    ui.add_space(4.0);
    for line in [
        format!("{} — {:.1}%", t("an_fc_how_progress"), f.progress_pct),
        format!("{} — {}", t("an_fc_how_cost"), money(f.cost_now)),
        format!("{} — {}", t("an_fc_how_earned"), money(f.earned)),
        t("an_fc_how_rule").to_string(),
    ] {
        ui.label(RichText::new(line).size(12.0).color(theme::muted()));
    }

    ui.add_space(16.0);
    productivity_block(ui, app);
}

/// Ishlar bo'yicha unumdorlik (TZ XVII.26-27).
///
/// Bir birlik ish qancha soat va qancha pulga tushgani — tannarx
/// prognozining asosidagi son. Eng qimmat ishlar yuqorida.
fn productivity_block(ui: &mut egui::Ui, app: &App) {
    let rows = app.productivity();
    if rows.is_empty() {
        return;
    }
    ui.label(RichText::new(t("an_pr_title")).size(13.5).strong());
    ui.label(
        RichText::new(t("an_pr_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    // O'rtacha bilan solishtirish: qaysi ish qimmatga tushayotgani ko'rinsin.
    let avg: f64 = rows.iter().map(|r| r.cost_per_unit).sum::<f64>() / rows.len() as f64;

    egui::Grid::new("an_pr_grid")
        .num_columns(6)
        .spacing([10.0, 5.0])
        .striped(true)
        .show(ui, |ui| {
            head_l(ui, 260.0, t("col_task"));
            head_r(ui, 110.0, t("an_pr_done"));
            head_r(ui, 90.0, t("an_pr_hours"));
            head_r(ui, 120.0, t("an_pr_per_unit"));
            head_r(ui, 140.0, t("an_pr_cost_unit"));
            head_l(ui, 110.0, t("an_pr_vs_avg"));
            ui.end_row();

            for r in rows.iter().take(12) {
                let Some(task) = app.task(r.task_id) else {
                    continue;
                };
                cell_l(
                    ui,
                    260.0,
                    RichText::new(super::issues::truncate(
                        &format!("{} {}", task.wbs, task.name),
                        34,
                    ))
                    .size(12.0),
                );
                cell_r(
                    ui,
                    110.0,
                    RichText::new(format!(
                        "{} {}",
                        super::materials::trim_num(r.done_volume),
                        r.unit
                    ))
                    .size(12.0),
                );
                cell_r(
                    ui,
                    90.0,
                    RichText::new(super::materials::trim_num(r.hours)).size(12.0),
                );
                cell_r(
                    ui,
                    120.0,
                    RichText::new(format!("{:.2}", r.hours_per_unit)).size(12.0),
                );
                cell_r(ui, 140.0, RichText::new(money(r.cost_per_unit)).size(12.0));
                let diff = if avg > 0.0 {
                    (r.cost_per_unit - avg) / avg * 100.0
                } else {
                    0.0
                };
                cell_l(
                    ui,
                    110.0,
                    RichText::new(format!("{diff:+.0}%"))
                        .size(12.0)
                        .color(if diff > 20.0 {
                            theme::danger()
                        } else if diff < -20.0 {
                            theme::ok()
                        } else {
                            theme::muted()
                        }),
                );
                ui.end_row();
            }
        });
}

// ================================================================ Ssenariy

/// «Nima bo'ladi, agar?» (TZ XVII.36, 43).
fn scenario_tab(ui: &mut egui::Ui, app: &mut App) {
    let key = egui::Id::new("an_scenario");
    let mut sc = ui
        .data(|d| d.get_temp::<crate::checks::Scenario>(key))
        .unwrap_or_default();

    ui.label(
        RichText::new(t("an_sc_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    // Uch tugma: muddat, material narxi, ish haqi.
    egui::Grid::new("an_sc_inputs")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(RichText::new(t("an_sc_delay")).size(12.5));
            let mut days = sc.delay_days as f64;
            if super::materials::num_edit(ui, 120.0, &mut days, 1.0, 365.0) {
                sc.delay_days = days.round() as i64;
            }
            ui.end_row();

            ui.label(RichText::new(t("an_sc_price")).size(12.5));
            ui.add(
                egui::Slider::new(&mut sc.price_pct, -30.0..=50.0)
                    .suffix(" %")
                    .step_by(1.0),
            );
            ui.end_row();

            ui.label(RichText::new(t("an_sc_wage")).size(12.5));
            ui.add(
                egui::Slider::new(&mut sc.wage_pct, -30.0..=50.0)
                    .suffix(" %")
                    .step_by(1.0),
            );
            ui.end_row();
        });
    ui.data_mut(|d| d.insert_temp(key, sc.clone()));
    ui.add_space(14.0);

    let Some(project) = app.project().cloned() else {
        return;
    };
    let base = app.finance_forecast();
    let costs = app.task_costs();
    let r = crate::checks::scenario(
        &base,
        &costs,
        app.progress.forecast_end,
        project.planned_end,
        &sc,
    );

    // Natija: ikki ustun — hozir va ssenariy bilan.
    egui::Grid::new("an_sc_result")
        .num_columns(3)
        .spacing([16.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new("").size(12.0));
            ui.label(
                RichText::new(t("an_sc_now"))
                    .size(12.0)
                    .color(theme::muted()),
            );
            ui.label(
                RichText::new(t("an_sc_after"))
                    .size(12.0)
                    .color(theme::accent()),
            );
            ui.end_row();

            let date = |d: Option<chrono::NaiveDate>| match d {
                Some(d) => d.format("%d.%m.%Y").to_string(),
                None => t("dash").to_string(),
            };
            ui.label(RichText::new(t("an_sc_finish")).size(12.5));
            ui.label(RichText::new(date(r.finish)).size(13.0));
            ui.label(
                RichText::new(date(r.finish_after))
                    .size(13.0)
                    .strong()
                    .color(if r.over_deadline {
                        theme::danger()
                    } else {
                        theme::ok()
                    }),
            );
            ui.end_row();

            ui.label(RichText::new(t("an_sc_cost")).size(12.5));
            ui.label(RichText::new(money(r.cost)).size(13.0));
            ui.label(
                RichText::new(money(r.cost_after))
                    .size(13.0)
                    .strong()
                    .color(if r.cost_after > r.cost {
                        theme::warn()
                    } else {
                        theme::ok()
                    }),
            );
            ui.end_row();

            ui.label(RichText::new(t("an_sc_profit")).size(12.5));
            ui.label(RichText::new(money(r.profit)).size(13.0));
            ui.label(
                RichText::new(money(r.profit_after))
                    .size(13.0)
                    .strong()
                    .color(if r.profit_after >= 0.0 {
                        theme::ok()
                    } else {
                        theme::danger()
                    }),
            );
            ui.end_row();
        });

    if r.over_deadline {
        ui.add_space(10.0);
        ui.label(
            RichText::new(t("an_sc_over"))
                .size(12.5)
                .color(theme::danger()),
        );
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new(t("an_sc_note"))
            .size(11.0)
            .color(theme::muted()),
    );
}

// ================================================================ Yo'qotishlar

/// Yashirin yo'qotishlar va tejash imkoniyatlari (TZ XVII.41, 44).
fn losses_tab(ui: &mut egui::Ui, app: &mut App) -> Option<Screen> {
    let list = app.opportunities();

    ui.label(
        RichText::new(t("an_op_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if list.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("an_op_none")).size(15.0).color(theme::ok()));
        });
        return None;
    }

    let losses: f64 = list
        .iter()
        .filter(|o| o.amount < 0.0)
        .map(|o| -o.amount)
        .sum();
    let frozen: f64 = list
        .iter()
        .filter(|o| o.amount > 0.0)
        .map(|o| o.amount)
        .sum();
    stat_row(
        ui,
        vec![
            stat(
                t("an_op_losses"),
                money(losses),
                t("an_op_losses_hint"),
                if losses == 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("an_op_frozen"),
                money(frozen),
                t("an_op_frozen_hint"),
                theme::warn(),
            ),
        ],
    );
    ui.add_space(12.0);

    let mut go = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for o in &list {
                let color = if o.amount < 0.0 {
                    theme::danger()
                } else {
                    theme::warn()
                };
                egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(4.0, 30.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, color);
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&o.title).size(13.0).strong().color(color),
                                    );
                                    ui.label(
                                        RichText::new(money(o.amount.abs()))
                                            .size(13.0)
                                            .strong()
                                            .color(color),
                                    );
                                });
                                ui.label(RichText::new(&o.detail).size(11.5).color(theme::muted()));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button(RichText::new(t("nt_open")).size(11.5)).clicked() {
                                        go = Some(o.screen);
                                    }
                                    ui.label(
                                        RichText::new(o.code)
                                            .size(10.0)
                                            .color(theme::muted())
                                            .monospace(),
                                    );
                                },
                            );
                        });
                    });
                ui.add_space(6.0);
            }
        });
    go
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn severity_color(s: Severity) -> Color32 {
    match s {
        Severity::Critical => theme::danger(),
        Severity::Major => theme::warn(),
        Severity::Warning => theme::accent(),
        Severity::Info => theme::muted(),
        Severity::Ok => theme::ok(),
    }
}

/// Bitta topilma kartochkasi. «O'tish» bosilsa `true`.
fn card(ui: &mut egui::Ui, f: &Finding) -> bool {
    let color = severity_color(f.severity);
    let mut open = false;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    // Chapdagi rangli chiziq — muhimlik darajasi.
                    let (r, _) = ui.allocate_exact_size(vec2(4.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(r, 2.0, color);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(f.severity.label())
                            .size(11.0)
                            .color(color)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(f.area.label())
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.label(
                        RichText::new(f.code)
                            .size(10.5)
                            .monospace()
                            .color(theme::muted()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(t("an_open")).clicked() {
                            open = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(RichText::new(&f.fact).size(13.5));
                if !f.evidence.is_empty() {
                    ui.label(
                        RichText::new(format!("{}: {}", t("an_evidence"), f.evidence))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                }
                if !f.action.is_empty() {
                    ui.label(
                        RichText::new(format!("{}: {}", t("an_action"), f.action))
                            .size(11.5)
                            .color(theme::accent()),
                    );
                }
            });
        });

    open
}
