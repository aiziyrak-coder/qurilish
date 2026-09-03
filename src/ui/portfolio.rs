//! «Obyektlar» ekrani — bir necha obyekt bo'yicha konsolidatsiya (TZ XVII.4).
//!
//! Bu yagona ekran obyektga bog'lanmagan: u barcha obyektlarni yonma-yon
//! qo'yadi. Savol bitta — **qaysi obyektga birinchi qarash kerak**. Shuning
//! uchun ro'yxat e'tibor talab qiladiganlardan boshlanadi va sabab yonida
//! yoziladi.
//!
//! Qatorga bosilsa o'sha obyekt tanlanadi va ilova odatdagi rejimga qaytadi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::portfolio::{self, ProjectSummary};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.projects.is_empty() {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("no_objects_yet"))
                    .color(theme::muted())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("create_first_object"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    // Hisob har bir obyekt uchun bazadan o'qiladi, shuning uchun uni kadrga
    // bir marta bajarib, natijani saqlab turamiz.
    let list = portfolio::summaries(&app.db, app.today);
    let totals = portfolio::totals(&list);

    ui.label(RichText::new(t("pf_hint")).size(11.0).color(theme::muted()));
    ui.add_space(8.0);

    stat_row(
        ui,
        vec![
            stat(
                t("pf_projects"),
                totals.projects.to_string(),
                t("pf_projects_hint"),
                theme::accent(),
            ),
            stat(
                t("pf_attention"),
                totals.attention.to_string(),
                t("pf_attention_hint"),
                if totals.attention == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("pf_progress"),
                format!("{:.1} %", totals.weighted_pct),
                t("pf_progress_hint"),
                theme::text(),
            ),
            stat(
                t("pf_contract"),
                money(totals.contract_sum),
                t("pf_contract_hint"),
                theme::text(),
            ),
            stat(
                t("pf_paid"),
                money(totals.paid_total),
                &format!(
                    "{:.0}% {}",
                    if totals.contract_sum > 0.0 {
                        totals.paid_total / totals.contract_sum * 100.0
                    } else {
                        0.0
                    },
                    t("pf_paid_hint")
                ),
                theme::ok(),
            ),
            stat(
                t("pf_stock"),
                money(totals.stock_value),
                t("pf_stock_hint"),
                theme::muted(),
            ),
        ],
    );
    ui.add_space(12.0);

    // Tor ekranda ikkinchi darajali ustunlar (reja, chiziq, sotuv) yashiriladi:
    // asosiy savol — qaysi obyekt orqada va nega — baribir ko'rinib turadi.
    let wide = ui.available_width() > 1400.0;

    let mut pick: Option<i64> = None;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("pf_grid")
                .num_columns(if wide { 13 } else { 10 })
                .spacing([8.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 210.0, t("pf_object"));
                    head_l(ui, 90.0, t("col_status"));
                    head_r(ui, 80.0, t("pf_fact"));
                    if wide {
                        head_r(ui, 80.0, t("pf_plan"));
                    }
                    head_r(ui, 90.0, t("pf_deviation"));
                    if wide {
                        head_l(ui, 110.0, "");
                    }
                    head_r(ui, 140.0, t("pf_contract_short"));
                    head_r(ui, 85.0, t("pf_paid_short"));
                    head_r(ui, 70.0, t("pf_quality"));
                    head_r(ui, 85.0, t("pf_safety"));
                    if wide {
                        head_r(ui, 80.0, t("pf_sales"));
                    }
                    head_l(ui, 200.0, t("pf_reason"));
                    head_l(ui, 80.0, "");
                    ui.end_row();

                    for p in &list {
                        let current = app.current == Some(p.project_id);
                        cell_l(
                            ui,
                            210.0,
                            RichText::new(super::issues::truncate(&p.name, 28))
                                .size(12.5)
                                .color(if current {
                                    theme::accent()
                                } else {
                                    theme::text()
                                }),
                        )
                        // Shifr — hujjatlarda obyekt shu kod bilan yuritiladi.
                        .on_hover_text(if p.code.is_empty() {
                            p.name.clone()
                        } else {
                            format!("{} · {}", p.code, p.name)
                        });
                        cell_l(
                            ui,
                            90.0,
                            RichText::new(p.status.label())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(format!("{:.1}%", p.fact_pct))
                                .size(12.5)
                                .strong(),
                        );
                        if wide {
                            cell_r(
                                ui,
                                80.0,
                                RichText::new(format!("{:.1}%", p.plan_pct))
                                    .size(12.0)
                                    .color(theme::muted()),
                            );
                        }
                        // Chetlanish — asosiy ko'rsatkich, shuning uchun rangli.
                        let dev = p.deviation();
                        let dev_color = if dev < -5.0 {
                            theme::danger()
                        } else if dev < 0.0 {
                            theme::warn()
                        } else {
                            theme::ok()
                        };
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(format!("{dev:+.1}"))
                                .size(12.5)
                                .color(dev_color),
                        );
                        if wide {
                            bar(ui, 110.0, p.fact_pct / 100.0, dev_color);
                        }

                        cell_r(ui, 140.0, RichText::new(money(p.contract_sum)).size(12.0));
                        cell_r(
                            ui,
                            85.0,
                            RichText::new(format!("{:.0}%", p.paid_pct()))
                                .size(12.0)
                                .color(if p.paid_pct() + 5.0 < p.fact_pct {
                                    // Bajarilish moliyalashtirishdan oldinda.
                                    theme::warn()
                                } else {
                                    theme::muted()
                                }),
                        );
                        cell_r(
                            ui,
                            70.0,
                            RichText::new(match p.quality_score {
                                Some(v) => format!("{v:.0}"),
                                None => t("dash").to_string(),
                            })
                            .size(12.0)
                            .color(match p.quality_score {
                                Some(v) if v >= 85.0 => theme::ok(),
                                Some(v) if v >= 60.0 => theme::warn(),
                                Some(_) => theme::danger(),
                                None => theme::muted(),
                            }),
                        );
                        cell_r(
                            ui,
                            85.0,
                            RichText::new(format!("{:.0}", p.safety_score))
                                .size(12.0)
                                .color(if p.safety_score >= 85.0 {
                                    theme::ok()
                                } else if p.safety_score >= 60.0 {
                                    theme::warn()
                                } else {
                                    theme::danger()
                                }),
                        );
                        if wide {
                            cell_r(
                                ui,
                                80.0,
                                RichText::new(if p.units_total > 0 {
                                    format!("{:.0}%", p.sold_pct())
                                } else {
                                    t("dash").to_string()
                                })
                                .size(12.0)
                                .color(theme::muted()),
                            )
                            // Sotuv ortidagi ikki son: nechta xonadon va qancha pul kelgan.
                            .on_hover_text(if p.units_total > 0 {
                                format!(
                                    "{} / {} · {} {}",
                                    p.units_sold,
                                    p.units_total,
                                    money(p.sales_paid),
                                    t("pf_sales_paid")
                                )
                            } else {
                                t("pf_no_sales").to_string()
                            });
                        }
                        // Sabab: nima uchun bu obyekt e'tibor talab qiladi.
                        cell_l(
                            ui,
                            200.0,
                            RichText::new(reason(p))
                                .size(11.0)
                                // Uch holat: e'tibor kerak, kichik kamchilik
                                // bor, hammasi joyida.
                                .color(if p.needs_attention() {
                                    theme::danger()
                                } else if p.overdue_tasks > 0 || p.late_purchases > 0 {
                                    theme::warn()
                                } else {
                                    theme::ok()
                                }),
                        );
                        if ui
                            .add_enabled(
                                !current,
                                egui::Button::new(RichText::new(t("pf_open")).size(11.5)),
                            )
                            .on_disabled_hover_text(t("pf_current"))
                            .clicked()
                        {
                            pick = Some(p.project_id);
                        }
                        ui.end_row();
                    }
                });
            redistribution_block(ui, app);
            central_block(ui, app);
        });

    if let Some(id) = pick {
        app.select_project(id);
        app.screen = Screen::Dashboard;
    }
}

/// Obyektlar bo'yicha xaridlar va markazlashtirish imkoniyati
/// (TZ X.33, 37-38, 42).
///
/// Guruhlash nom bo'yicha: har obyektning o'z katalogi bor va kodlar mos
/// kelmasligi mumkin. Tejash — hammasi eng arzon narxda olinganda chiqadigan
/// **yuqori chegara**, kafolat emas: hajm va yetkazish sharti har xil.
fn central_block(ui: &mut egui::Ui, app: &App) {
    let lines = portfolio::central_purchases(&app.db);
    if lines.is_empty() {
        return;
    }
    let worth: Vec<&crate::checks::CentralLine> =
        lines.iter().filter(|l| l.worth_central()).collect();
    let saving: f64 = worth.iter().map(|l| l.saving).sum();

    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("pf_central")).size(14.0).strong());
        ui.label(
            RichText::new(t("pf_central_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(4.0);
    if saving > 0.0 {
        ui.label(
            RichText::new(format!("{}: {}", t("pf_central_saving"), money(saving)))
                .size(12.5)
                .color(theme::ok()),
        );
        ui.label(
            RichText::new(t("pf_central_saving_hint"))
                .size(10.5)
                .color(theme::muted()),
        );
    }
    ui.add_space(6.0);

    egui::Grid::new("pf_central")
        .num_columns(6)
        .spacing([10.0, 5.0])
        .striped(true)
        .show(ui, |ui| {
            head_l(ui, 260.0, t("col_material"));
            head_r(ui, 100.0, t("pf_central_objects"));
            head_r(ui, 110.0, t("col_qty"));
            head_r(ui, 140.0, t("pf_central_best"));
            head_r(ui, 100.0, t("mat_maker_spread"));
            head_r(ui, 140.0, t("pf_central_save"));
            ui.end_row();

            for l in lines.iter().take(12) {
                let name = super::issues::truncate(&l.title, 34);
                let resp = super::warehouse::cell_l(
                    ui,
                    260.0,
                    RichText::new(name).size(12.5).color(if l.worth_central() {
                        theme::text()
                    } else {
                        theme::muted()
                    }),
                );
                // Qaysi obyektda qancha olingani va markazlashtirilgan
                // buyurtma qanday bo'linishi — sichqoncha ostida (TZ X.42).
                let name_of = |id: i64| {
                    app.projects
                        .iter()
                        .find(|p| p.id == id)
                        .map(|p| super::issues::truncate(&p.name, 26))
                        .unwrap_or_default()
                };
                resp.on_hover_ui(|ui| {
                    ui.label(RichText::new(t("pf_central_now")).size(11.5).strong());
                    for o in &l.objects {
                        ui.label(
                            RichText::new(format!(
                                "{}: {} {} × {} = {}",
                                name_of(o.project_id),
                                super::materials::trim_num(o.qty),
                                l.unit,
                                money(o.price),
                                money(o.amount)
                            ))
                            .size(11.5),
                        );
                    }
                    ui.label(
                        RichText::new(format!("{}: {}", t("col_total"), money(l.total_amount)))
                            .size(11.5)
                            .color(theme::muted()),
                    );

                    if l.worth_central() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(t("pf_central_split")).size(11.5).strong());
                        let need: Vec<(i64, f64)> =
                            l.objects.iter().map(|o| (o.project_id, o.qty)).collect();
                        for sp in crate::checks::split_order(&need, l.total_qty, l.best_price) {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {} {} ({:.0}%) = {}",
                                    name_of(sp.project_id),
                                    super::materials::trim_num(sp.qty),
                                    l.unit,
                                    sp.share_pct,
                                    money(sp.amount)
                                ))
                                .size(11.5),
                            );
                        }
                    }
                });
                super::warehouse::cell_r(
                    ui,
                    100.0,
                    RichText::new(l.objects.len().to_string()).size(12.0),
                );
                super::warehouse::cell_r(
                    ui,
                    110.0,
                    RichText::new(format!(
                        "{} {}",
                        super::materials::trim_num(l.total_qty),
                        l.unit
                    ))
                    .size(12.0),
                );
                super::warehouse::cell_r(ui, 140.0, RichText::new(money(l.best_price)).size(12.0));
                super::warehouse::cell_r(
                    ui,
                    100.0,
                    RichText::new(format!("{:.0}%", l.spread_pct))
                        .size(12.0)
                        .color(if l.worth_central() {
                            theme::warn()
                        } else {
                            theme::muted()
                        }),
                );
                super::warehouse::cell_r(
                    ui,
                    140.0,
                    RichText::new(if l.saving > 0.0 {
                        money(l.saving)
                    } else {
                        t("dash").to_string()
                    })
                    .size(12.5)
                    .color(if l.worth_central() {
                        theme::ok()
                    } else {
                        theme::muted()
                    }),
                );
                ui.end_row();
            }
        });
}

/// Obyektlar orasida material ko'chirish takliflari (TZ XI.42).
fn redistribution_block(ui: &mut egui::Ui, app: &App) {
    let moves = portfolio::redistribution(&app.db, app.today);
    if moves.is_empty() {
        return;
    }
    let name = |id: i64| {
        app.projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.code.clone())
            .unwrap_or_default()
    };
    let total: f64 = moves.iter().map(|m| m.saving).sum();

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("pf_move_title")).size(14.0).strong());
        ui.label(
            RichText::new(format!("{} {}", money(total), t("pf_move_saving")))
                .size(12.5)
                .color(theme::ok()),
        );
    });
    ui.label(
        RichText::new(t("pf_move_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    egui::Grid::new("pf_move_grid")
        .num_columns(5)
        .spacing([10.0, 5.0])
        .striped(true)
        .show(ui, |ui| {
            head_l(ui, 220.0, t("col_material"));
            head_l(ui, 110.0, t("pf_move_from"));
            head_l(ui, 110.0, t("pf_move_to"));
            head_r(ui, 120.0, t("col_qty"));
            head_r(ui, 140.0, t("pf_move_value"));
            ui.end_row();
            for m in moves.iter().take(10) {
                cell_l(
                    ui,
                    220.0,
                    RichText::new(super::issues::truncate(&m.material_name, 28)).size(12.0),
                );
                cell_l(
                    ui,
                    110.0,
                    RichText::new(name(m.from_project))
                        .size(12.0)
                        .color(theme::muted()),
                );
                cell_l(
                    ui,
                    110.0,
                    RichText::new(name(m.to_project))
                        .size(12.0)
                        .color(theme::accent()),
                );
                cell_r(
                    ui,
                    120.0,
                    RichText::new(format!("{} {}", super::materials::trim_num(m.qty), m.unit))
                        .size(12.0),
                );
                cell_r(ui, 140.0, RichText::new(money(m.saving)).size(12.0));
                ui.end_row();
            }
        });
}

/// Obyekt nima uchun e'tibor talab qilishini bir qatorda aytadi.
///
/// Bir nechta sabab bo'lsa hammasi yoziladi: bittasini tanlab qolgani
/// yashirish noto'g'ri bo'lardi.
fn reason(p: &ProjectSummary) -> String {
    let mut out: Vec<String> = Vec::new();
    if p.deviation() < -5.0 {
        out.push(format!("{} {}", t("pf_r_delay"), p.delay_days));
    }
    if p.critical_issues > 0 {
        out.push(format!("{} {}", t("pf_r_critical"), p.critical_issues));
    }
    if p.overdue_tasks > 0 {
        out.push(format!("{} {}", t("pf_r_overdue"), p.overdue_tasks));
    }
    if p.safety_score < 60.0 {
        out.push(t("pf_r_safety").to_string());
    }
    if p.late_purchases > 0 {
        out.push(format!("{} {}", t("pf_r_late"), p.late_purchases));
    }
    if out.is_empty() {
        return t("pf_r_ok").to_string();
    }
    out.join(" · ")
}

/// Bajarilish chizig'i.
fn bar(ui: &mut egui::Ui, width: f32, v: f64, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, theme::track());
    let w = (v.clamp(0.0, 1.0) as f32) * rect.width();
    if w > 0.5 {
        let mut fill = rect;
        fill.set_width(w);
        p.rect_filled(fill, 3.0, color);
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
