//! «Rahbar» ekrani — modullar bo'yicha bosh sonlar bir joyda.
//!
//! TZ da har modul uchun alohida «rahbar paneli» talab qilinadi (IX.34,
//! X.45, XI.40, XII.40, XIII.41-42, XIV.39, XV.39, XVI.47). Har ekranga
//! alohida panel qo'yish o'rniga ular bitta joyga yig'ildi: rahbar
//! modullar bo'ylab yurib chiqmaydi — u bir ekranda ko'radi va faqat
//! kerakli joyga kiradi.
//!
//! **Ekran yangi hisob qilmaydi.** Har son o'z modulidagi funksiyadan
//! olinadi, shuning uchun bu yerdagi raqam modul ekranidagi bilan hech
//! qachon farq qilmaydi. Har blokda «Ochish» tugmasi bor — son qayerdan
//! kelganini tekshirib ko'rish uchun.

use super::*;
use crate::domain::{IssueStatus, PurchaseStatus, RequestStatus, Severity};

/// Bitta modul bloki: sarlavha, sonlar va manzil.
struct Card {
    title: String,
    numeral: &'static str,
    screen: Screen,
    rows: Vec<(String, String, egui::Color32)>,
}

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

    // Umumiy ball — modullar ustidagi bitta ko'rsatkich (TZ XVII.49).
    let score = app.executive_score();
    ui.label(RichText::new(t("dr_hint")).size(11.0).color(theme::muted()));
    ui.add_space(10.0);

    let paint = |v: f64| {
        if v >= 80.0 {
            theme::ok()
        } else if v >= 60.0 {
            theme::warn()
        } else {
            theme::danger()
        }
    };
    stat_row(
        ui,
        vec![
            stat(
                t("an_exec_total"),
                format!("{:.0}", score.total),
                t("an_exec_total_hint"),
                paint(score.total),
            ),
            stat(
                t("an_exec_safety"),
                format!("{:.0}", score.safety),
                t("an_exec_weight_25"),
                paint(score.safety),
            ),
            stat(
                t("an_exec_quality"),
                format!("{:.0}", score.quality),
                t("an_exec_weight_25"),
                paint(score.quality),
            ),
            stat(
                t("an_exec_schedule"),
                format!("{:.0}", score.schedule),
                t("an_exec_weight_20"),
                paint(score.schedule),
            ),
            stat(
                t("an_exec_money"),
                format!("{:.0}", score.money),
                t("an_exec_weight_15"),
                paint(score.money),
            ),
        ],
    );
    ui.add_space(14.0);

    let cards = cards(app);
    let decisions = app.decisions();
    let bench = crate::portfolio::benchmark(&app.db, app.today);
    let mut go: Option<Screen> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Qarorlar markazi (TZ XVII.42) ----------
            ui.label(RichText::new(t("dr_decisions")).size(14.0).strong());
            ui.label(
                RichText::new(t("dr_decisions_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if decisions.is_empty() {
                ui.label(
                    RichText::new(t("dr_decisions_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for d in decisions.iter().take(10) {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(52.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, theme::line());
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(rect.width() * (d.weight() / 100.0) as f32, rect.height()),
                        ),
                        2.0,
                        if d.weight() >= 60.0 {
                            theme::danger()
                        } else {
                            theme::warn()
                        },
                    );
                    ui.label(RichText::new(decision_text(d)).size(12.0));
                    if ui.small_button(t("an_open")).clicked() {
                        go = Some(d.screen());
                    }
                });
            }
            ui.add_space(16.0);

            // ---------- Obyektlar solishtiruvi (TZ XVII.27-28) ----------
            if bench.len() > 1 {
                ui.label(RichText::new(t("dr_bench")).size(14.0).strong());
                ui.label(
                    RichText::new(t("dr_bench_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                egui::Grid::new("dr_bench")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 220.0, t("col_object"));
                        head_r(ui, 130.0, t("dr_bench_gap"));
                        head_r(ui, 140.0, t("dr_bench_cost"));
                        head_r(ui, 130.0, t("dr_bench_hours"));
                        head_r(ui, 90.0, t("an_exec_quality"));
                        head_r(ui, 90.0, t("an_exec_safety"));
                        ui.end_row();

                        for b in &bench {
                            let name = app
                                .projects
                                .iter()
                                .find(|p| p.id == b.project_id)
                                .map(|p| p.name.clone())
                                .unwrap_or_default();
                            super::warehouse::cell_l(
                                ui,
                                220.0,
                                RichText::new(super::issues::truncate(&name, 28)).size(12.5),
                            );
                            super::warehouse::cell_r(
                                ui,
                                130.0,
                                RichText::new(format!("{:+.1}%", b.gap())).size(12.5).color(
                                    if b.gap() >= 0.0 {
                                        theme::ok()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            // Hajm noma'lum bo'lsa ustun bo'sh qoladi.
                            let opt = |v: Option<f64>, money_fmt: bool| match v {
                                None => RichText::new(t("dash")).color(theme::muted()),
                                Some(x) if money_fmt => RichText::new(money(x)).size(12.0),
                                Some(x) => RichText::new(format!("{x:.1}")).size(12.0),
                            };
                            super::warehouse::cell_r(ui, 140.0, opt(b.cost_per_volume, true));
                            super::warehouse::cell_r(ui, 130.0, opt(b.hours_per_volume, false));
                            super::warehouse::cell_r(
                                ui,
                                90.0,
                                RichText::new(format!("{:.0}", b.quality)).size(12.0),
                            );
                            super::warehouse::cell_r(
                                ui,
                                90.0,
                                RichText::new(format!("{:.0}", b.safety)).size(12.0),
                            );
                            ui.end_row();
                        }
                    });
                ui.add_space(16.0);
            }

            // Ustunlar soni oynaga qarab: tor oynada bitta ustun.
            let width = ui.available_width();
            let columns = ((width / 330.0).floor() as usize).clamp(1, 4);
            let card_w = (width - (columns as f32 - 1.0) * 10.0) / columns as f32;

            for chunk in cards.chunks(columns) {
                ui.horizontal_top(|ui| {
                    for c in chunk {
                        ui.allocate_ui(egui::vec2(card_w, 0.0), |ui| {
                            if card(ui, c) {
                                go = Some(c.screen);
                            }
                        });
                    }
                });
                ui.add_space(10.0);
            }
            ui.add_space(20.0);
        });

    if let Some(s) = go {
        app.screen = s;
    }
}

/// Qarorni odam o'qiydigan gapga aylantiradi.
fn decision_text(d: &crate::checks::Decision) -> String {
    use crate::checks::Decision as D;
    match d {
        D::Request { number, days } => {
            format!("{} {number} ({days} {})", t("de_request"), t("days"))
        }
        D::Change {
            number,
            days,
            amount,
        } => format!(
            "{} {number} — {} ({days} {})",
            t("de_change"),
            money(*amount),
            t("days")
        ),
        D::Acceptance { number, days } => {
            format!("{} {number} ({days} {})", t("de_acceptance"), t("days"))
        }
        D::TechApproval { number, amount } => {
            format!("{} {number} — {}", t("de_tech"), money(*amount))
        }
        D::DefectDeadline { count } => format!("{} ({count})", t("de_deadline")),
        D::DocSign { count } => format!("{} ({count})", t("de_doc")),
        D::MachineStop { count } => format!("{} ({count})", t("de_machine")),
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
}

/// Bitta blokni chizadi; «Ochish» bosilsa `true` qaytadi.
fn card(ui: &mut egui::Ui, c: &Card) -> bool {
    let mut open = false;
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(c.numeral)
                        .size(10.5)
                        .monospace()
                        .color(theme::muted()),
                );
                ui.label(RichText::new(&c.title).size(13.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(t("an_open")).clicked() {
                        open = true;
                    }
                });
            });
            ui.add_space(6.0);
            for (label, value, colour) in &c.rows {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [(ui.available_width() - 110.0).max(60.0), 18.0],
                        egui::Label::new(RichText::new(label).size(11.5).color(theme::muted())),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(value).size(12.5).strong().color(*colour));
                    });
                });
            }
        });
    open
}

/// Modullar bo'yicha bosh sonlar.
///
/// Har son o'z modulidagi funksiyadan olinadi — bu yerda hech narsa
/// qaytadan hisoblanmaydi.
fn cards(app: &App) -> Vec<Card> {
    let muted = theme::muted();
    let ok = theme::ok();
    let warn = theme::warn();
    let danger = theme::danger();
    let mark = |n: usize| if n == 0 { ok } else { warn };
    let hard = |n: usize| if n == 0 { ok } else { danger };

    let mut out = Vec::new();

    // ---------- I. Grafik ----------
    let overdue = app.progress.overdue.len();
    out.push(Card {
        title: t("screen_gantt").to_string(),
        numeral: "I.2",
        screen: Screen::Gantt,
        rows: vec![
            (
                t("dr_progress").to_string(),
                format!(
                    "{:.1}% / {:.1}%",
                    app.progress.fact_pct, app.progress.plan_pct
                ),
                if app.progress.fact_pct + 0.01 >= app.progress.plan_pct {
                    ok
                } else {
                    warn
                },
            ),
            (
                t("dr_overdue").to_string(),
                overdue.to_string(),
                hard(overdue),
            ),
            (
                t("dr_delay").to_string(),
                format!("{} {}", app.progress.delay_days, t("days")),
                if app.progress.delay_days <= 0 {
                    ok
                } else {
                    danger
                },
            ),
        ],
    });

    // ---------- IX. Arizalar (TZ IX.34) ----------
    let open_requests = app
        .requests
        .iter()
        .filter(|r| matches!(r.status, RequestStatus::New | RequestStatus::Approved))
        .count();
    let late_supply = app.supply().iter().filter(|s| s.late).count();
    out.push(Card {
        title: t("screen_requests").to_string(),
        numeral: "IX",
        screen: Screen::Requests,
        rows: vec![
            (t("dr_open").to_string(), open_requests.to_string(), muted),
            (
                t("dr_late_supply").to_string(),
                late_supply.to_string(),
                mark(late_supply),
            ),
        ],
    });

    // ---------- X. Xaridlar (TZ X.45) ----------
    let control = app.supply_control();
    let amount: f64 = app
        .purchases
        .iter()
        .filter(|p| p.status != PurchaseStatus::Draft)
        .map(|p| p.amount())
        .sum();
    let severe = control.iter().filter(|i| i.severe()).count();
    out.push(Card {
        title: t("screen_purchases").to_string(),
        numeral: "X",
        screen: Screen::Purchases,
        rows: vec![
            (t("dr_amount").to_string(), money(amount), muted),
            (
                t("dr_order_issues").to_string(),
                control.len().to_string(),
                mark(control.len()),
            ),
            (t("dr_severe").to_string(), severe.to_string(), hard(severe)),
        ],
    });

    // ---------- XI. Ombor (TZ XI.40) ----------
    let stock = app.stock();
    let stock_value: f64 = stock.iter().map(|l| l.value).sum();
    let below = stock.iter().filter(|l| l.below_min).count();
    let stock_issues = app.stock_control().len();
    out.push(Card {
        title: t("screen_warehouse").to_string(),
        numeral: "XI",
        screen: Screen::Warehouse,
        rows: vec![
            (t("dr_stock_value").to_string(), money(stock_value), muted),
            (
                t("dr_below_min").to_string(),
                below.to_string(),
                mark(below),
            ),
            (
                t("dr_stock_issues").to_string(),
                stock_issues.to_string(),
                mark(stock_issues),
            ),
        ],
    });

    // ---------- XII. Materiallar (TZ XII.40) ----------
    let fit = app.material_fit();
    let critical = fit.iter().filter(|f| f.critical()).count();
    let kits = app.material_kits();
    let incomplete = kits.iter().filter(|k| !k.complete()).count();
    out.push(Card {
        title: t("screen_materials").to_string(),
        numeral: "XII",
        screen: Screen::Materials,
        rows: vec![
            (
                t("dr_material_critical").to_string(),
                critical.to_string(),
                hard(critical),
            ),
            (
                t("dr_kits").to_string(),
                format!("{} / {}", incomplete, kits.len()),
                mark(incomplete),
            ),
        ],
    });

    // ---------- XIII. Tabel (TZ XIII.41-42) ----------
    let staff = app.staff_forecast();
    let anomalies = crate::checks::timesheet_anomalies(&app.timesheet).len();
    out.push(Card {
        title: t("screen_timesheet").to_string(),
        numeral: "XIII",
        screen: Screen::Timesheet,
        rows: vec![
            (t("dr_workers").to_string(), staff.have.to_string(), muted),
            (
                t("dr_staff_gap").to_string(),
                staff.gap.to_string(),
                if staff.gap <= 0 { ok } else { warn },
            ),
            (
                t("dr_anomalies").to_string(),
                anomalies.to_string(),
                mark(anomalies),
            ),
        ],
    });

    // ---------- XIV. Sifat (TZ XIV.39) ----------
    let week = app.quality_week();
    out.push(Card {
        title: t("screen_quality").to_string(),
        numeral: "XIV",
        screen: Screen::Quality,
        rows: vec![
            (
                t("dr_score").to_string(),
                format!("{:.0}", week.score),
                if week.score >= 80.0 {
                    ok
                } else if week.score >= 60.0 {
                    warn
                } else {
                    danger
                },
            ),
            (
                t("dr_defects").to_string(),
                week.defects_open.to_string(),
                mark(week.defects_open),
            ),
            (
                t("dr_defects_overdue").to_string(),
                week.overdue.to_string(),
                hard(week.overdue),
            ),
        ],
    });

    // ---------- XV. Xavfsizlik (TZ XV.39) ----------
    let safety_score = crate::checks::safety_score(
        &app.safety,
        &app.worker_safety(),
        &app.work_permits,
        app.today,
    );
    let open_safety = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    out.push(Card {
        title: t("screen_safety").to_string(),
        numeral: "XV",
        screen: Screen::Safety,
        rows: vec![
            (
                t("dr_score").to_string(),
                format!("{:.0}", safety_score.score),
                if safety_score.score >= 80.0 {
                    ok
                } else if safety_score.score >= 60.0 {
                    warn
                } else {
                    danger
                },
            ),
            (
                t("dr_open_cases").to_string(),
                open_safety.to_string(),
                mark(open_safety),
            ),
        ],
    });

    // ---------- XVI. Mashinalar (TZ XVI.47) ----------
    let mech = app.mech_issues();
    let stops = mech.iter().filter(|i| i.stop()).count();
    let (park, advice) = app.park_review();
    let idle = park
        .iter()
        .filter(|l| l.usage_pct < crate::checks::PARK_IDLE_PCT)
        .count();
    out.push(Card {
        title: t("screen_machines").to_string(),
        numeral: "XVI",
        screen: Screen::Machines,
        rows: vec![
            (t("dr_stops").to_string(), stops.to_string(), hard(stops)),
            (
                t("dr_idle_machines").to_string(),
                idle.to_string(),
                mark(idle),
            ),
            (
                t("dr_park_advice").to_string(),
                advice.len().to_string(),
                muted,
            ),
        ],
    });

    // ---------- II-III. Loyiha va smeta ----------
    let critical_issues = app
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Critical && i.status != IssueStatus::Fixed)
        .count();
    out.push(Card {
        title: t("screen_ai_check").to_string(),
        numeral: "II",
        screen: Screen::AiCheck,
        rows: vec![
            (
                t("dr_critical").to_string(),
                critical_issues.to_string(),
                hard(critical_issues),
            ),
            (
                t("dr_issues").to_string(),
                app.issues.len().to_string(),
                muted,
            ),
        ],
    });

    let deep = app.estimate_deep();
    let cost = app.cost_summary();
    out.push(Card {
        title: t("screen_estimate").to_string(),
        numeral: "III",
        screen: Screen::Estimate,
        rows: vec![
            (t("dr_estimate").to_string(), money(cost.total), muted),
            (
                t("dr_deep").to_string(),
                deep.len().to_string(),
                mark(deep.len()),
            ),
        ],
    });

    // ---------- IV, VII. Hujjatlar va nazorat ----------
    let readiness = app.final_readiness();
    out.push(Card {
        title: t("screen_exec_docs").to_string(),
        numeral: "IV",
        screen: Screen::ExecDocs,
        rows: vec![
            (
                t("dr_final_ready").to_string(),
                format!("{:.0}%", readiness.ready_pct),
                if readiness.ready() { ok } else { warn },
            ),
            (
                t("dr_final_blocks").to_string(),
                readiness.blocks.len().to_string(),
                mark(readiness.blocks.len()),
            ),
        ],
    });

    out
}
