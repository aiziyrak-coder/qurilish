//! «AI smeta tekshiruvi» ekrani (TZ III).
//!
//! Smeta pozitsiyalari kiritiladi yoki namoyish ma'lumotidan olinadi, so'ng
//! qoidalar arifmetika, birliklar, dublikatlar, hajmlar va narxlarni tekshiradi.

use super::issues;
use super::warehouse::{cell_l, cell_r};
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

    // Kunlik: pozitsiyalar, topilmalar va qiymat nazorati.
    super::tab_row(
        ui,
        &mut tab,
        &[
            (0, t("tab_estimate_items")),
            (2, t("tab_issues")),
            (1, t("tab_cost_control")),
        ],
        &[
            (3, t("tab_structure")),
            (4, t("tab_chain")),
            (5, t("tab_deep")),
            (6, t("tab_volumes")),
        ],
    );
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => cost_tab(ui, app),
        2 => issues_tab(ui, app),
        3 => structure_tab(ui, app),
        4 => chain_tab(ui, app),
        5 => deep_tab(ui, app),
        6 => volumes_tab(ui, app),
        _ => items(ui, app),
    }
}

// ================================================================ Hajmlar

/// Loyiha va smeta hajmlari, tushib qolgan ishlar (TZ III.7, 10, 31).
///
/// Hisob **qo'pol**: elementlarning asosiy o'lchovi bo'lim bo'yicha
/// yig'iladi. Bu aniq hajm emas — maqsad kattalik tartibini tekshirish:
/// smetada ikki barobar ko'p hajm turgan bo'lsa, buni sezish kerak.
fn volumes_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let volumes = app.project_volumes();
    let missing = app.missing_works();
    let cost = app.cost_summary();
    let started = missing.iter().filter(|m| m.started).count();

    ui.label(
        RichText::new(t("es_vol_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    // ---------- COST CONTROL (TZ III.31) ----------
    stat_row(
        ui,
        vec![
            stat(
                t("es_cc_total"),
                money(cost.total),
                t("es_cc_total_hint"),
                theme::text(),
            ),
            stat(
                t("es_cc_excess"),
                money(cost.volume_excess),
                t("es_cc_excess_hint"),
                if cost.volume_excess <= 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("es_cc_duplicate"),
                money(cost.duplicate_cost),
                &format!("{} {}", cost.duplicate_count, t("es_cc_positions")),
                if cost.duplicate_count == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("es_cc_saving"),
                money(cost.price_saving),
                t("es_cc_saving_hint"),
                theme::muted(),
            ),
            stat(
                t("es_cc_missing"),
                format!("{started} / {}", missing.len()),
                t("es_cc_missing_hint"),
                if started == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(14.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Hajmlar (TZ III.7) ----------
            ui.label(RichText::new(t("es_vol_title")).size(13.5).strong());
            ui.add_space(6.0);
            if volumes.is_empty() {
                ui.label(
                    RichText::new(t("es_vol_empty"))
                        .size(12.5)
                        .color(theme::muted()),
                );
            } else {
                egui::Grid::new("es_volumes")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 200.0, t("col_section"));
                        head_r(ui, 140.0, t("es_vol_project"));
                        head_r(ui, 140.0, t("es_vol_estimate"));
                        head_r(ui, 110.0, t("es_vol_diff"));
                        head_r(ui, 110.0, t("es_vol_elements"));
                        head_r(ui, 110.0, t("es_vol_items"));
                        ui.end_row();

                        for v in &volumes {
                            cell_l(ui, 200.0, RichText::new(v.section.label()).size(12.5));
                            cell_r(
                                ui,
                                140.0,
                                RichText::new(super::materials::trim_num(v.from_project))
                                    .size(12.0),
                            );
                            cell_r(
                                ui,
                                140.0,
                                RichText::new(super::materials::trim_num(v.from_estimate))
                                    .size(12.0),
                            );
                            cell_r(
                                ui,
                                110.0,
                                match v.diff_pct {
                                    None => RichText::new(t("dash")).color(theme::muted()),
                                    Some(d) => RichText::new(format!("{d:+.0}%")).size(12.5).color(
                                        if v.off() {
                                            theme::warn()
                                        } else {
                                            theme::muted()
                                        },
                                    ),
                                },
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(v.elements.to_string())
                                    .size(12.0)
                                    .color(theme::muted()),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(v.items.to_string())
                                    .size(12.0)
                                    .color(theme::muted()),
                            );
                            ui.end_row();
                        }
                    });
            }

            // ---------- Tushib qolgan ishlar (TZ III.10) ----------
            ui.add_space(16.0);
            ui.label(RichText::new(t("es_missing")).size(13.5).strong());
            ui.label(
                RichText::new(t("es_missing_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if missing.is_empty() {
                ui.label(
                    RichText::new(t("es_missing_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for m in missing.iter().take(15) {
                let name = app
                    .task(m.task_id)
                    .map(|x| format!("{} {}", x.wbs, x.name))
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!(
                        "· {} — {} {} {}",
                        super::issues::truncate(&name, 40),
                        m.section.code(),
                        super::materials::trim_num(m.volume),
                        if m.started {
                            t("es_missing_started")
                        } else {
                            ""
                        }
                    ))
                    .size(12.0)
                    .color(if m.started {
                        theme::danger()
                    } else {
                        theme::muted()
                    }),
                );
            }
            ui.add_space(16.0);
        });
}

// ================================================================ Chuqur tekshiruv

/// Kompleks rasenka, ketma-ketlik, marka va tijorat taklifi
/// (TZ III.9, 11, 13, 16).
fn deep_tab(ui: &mut egui::Ui, app: &mut App) {
    let list = app.estimate_deep();
    let money = list.iter().filter(|i| i.money()).count();

    ui.label(
        RichText::new(t("es_deep_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("es_deep_money"),
                money.to_string(),
                t("es_deep_money_hint"),
                if money == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("es_deep_total"),
                list.len().to_string(),
                t("es_deep_total_hint"),
                theme::text(),
            ),
        ],
    );
    ui.add_space(12.0);

    if list.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("es_deep_none"))
                    .color(theme::ok())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for i in &list {
                ui.label(
                    RichText::new(format!("· {}", deep_text(i)))
                        .size(12.0)
                        .color(if i.money() {
                            theme::danger()
                        } else {
                            theme::warn()
                        }),
                );
            }
            ui.add_space(12.0);
        });
}

/// Chuqur tekshiruv e'tirozini gapga aylantiradi.
fn deep_text(i: &crate::checks::DeepIssue) -> String {
    use crate::checks::DeepIssue as D;
    match i {
        D::DoubleCount {
            pos,
            inside,
            amount,
        } => format!(
            "{} {} ← {} ({})",
            t("ed_double"),
            pos,
            inside,
            money(*amount)
        ),
        D::SamePriceCode { code, low, high } => format!(
            "{} {}: {} … {}",
            t("ed_same_code"),
            code,
            money(*low),
            money(*high)
        ),
        D::MissingPredecessor { pos, task } => {
            format!("{} {} — {}", t("ed_predecessor"), pos, task)
        }
        D::NoMark { pos, name } => format!(
            "{} {} — {}",
            t("ed_no_mark"),
            pos,
            super::issues::truncate(name, 40)
        ),
        D::QuoteGap {
            pos,
            estimate,
            quote,
            pct,
        } => format!(
            "{} {}: {} / {} ({}{:.0}%)",
            t("ed_quote_gap"),
            pos,
            money(*estimate),
            money(*quote),
            if *pct > 0.0 { "+" } else { "" },
            pct
        ),
    }
}

// ================================================================ Zanjir

/// Smetadan faktgacha bo'lgan yo'l (TZ III.33).
///
/// Pul smetadan chiqib, ariza va xarid orqali omborga, u yerdan ishga
/// o'tadi va tannarxga aylanadi. Har bosqich alohida modulda yozilgan —
/// bu yerda ular bir qatorda, shuning uchun uzilish darhol ko'rinadi.
fn chain_tab(ui: &mut egui::Ui, app: &mut App) {
    let lines = app.estimate_chain();
    let totals = crate::checks::chain_totals(&lines);

    ui.label(
        RichText::new(t("es_chain_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if lines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("es_chain_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    stat_row(
        ui,
        vec![
            stat(
                t("es_chain_planned"),
                money(totals.planned),
                t("es_chain_planned_hint"),
                theme::text(),
            ),
            stat(
                t("es_chain_purchased"),
                money(totals.purchased),
                t("es_chain_purchased_hint"),
                theme::text(),
            ),
            stat(
                t("es_chain_actual"),
                money(totals.actual),
                &format!("{} {}", money(totals.earned), t("es_chain_earned")),
                theme::text(),
            ),
            stat(
                t("es_chain_diff"),
                money(totals.diff),
                t("es_chain_diff_hint"),
                if totals.diff >= 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("es_chain_gaps"),
                totals.gaps.to_string(),
                t("es_chain_gaps_hint"),
                if totals.gaps == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("es_chain_grid")
                .num_columns(9)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 240.0, t("col_task"));
                    head_r(ui, 80.0, t("col_progress"));
                    head_r(ui, 140.0, t("es_chain_c_plan"));
                    head_r(ui, 130.0, t("es_chain_c_request"));
                    head_r(ui, 130.0, t("es_chain_c_purchase"));
                    head_r(ui, 130.0, t("es_chain_c_issued"));
                    head_r(ui, 140.0, t("es_chain_c_actual"));
                    head_r(ui, 140.0, t("es_chain_c_diff"));
                    head_l(ui, 120.0, "");
                    ui.end_row();

                    for l in &lines {
                        let Some(task) = l.task_id.and_then(|id| app.task(id)) else {
                            continue;
                        };
                        cell_l(
                            ui,
                            240.0,
                            RichText::new(super::issues::truncate(
                                &format!("{} {}", task.wbs, task.name),
                                32,
                            ))
                            .size(12.5),
                        );
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(format!("{:.0}%", l.progress))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        let cell = |ui: &mut egui::Ui, w: f32, v: f64| {
                            cell_r(
                                ui,
                                w,
                                if v == 0.0 {
                                    RichText::new(t("dash")).color(theme::muted())
                                } else {
                                    RichText::new(money(v)).size(12.0)
                                },
                            );
                        };
                        cell(ui, 140.0, l.planned);
                        cell(ui, 130.0, l.requested);
                        cell(ui, 130.0, l.purchased);
                        cell(ui, 130.0, l.issued);
                        cell_r(
                            ui,
                            140.0,
                            RichText::new(money(l.actual)).size(12.5).strong(),
                        );
                        cell_r(
                            ui,
                            140.0,
                            RichText::new(money(l.diff))
                                .size(12.5)
                                .color(if l.diff >= 0.0 {
                                    theme::ok()
                                } else {
                                    theme::danger()
                                }),
                        );
                        cell_l(
                            ui,
                            120.0,
                            if l.has_gap() {
                                RichText::new(t("es_chain_gap"))
                                    .size(11.0)
                                    .color(theme::warn())
                            } else {
                                RichText::new("").size(11.0)
                            },
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Tuzilish

/// Smeta tuzilishi: koeffitsiyentlar, GPR bog'lanishi va byudjet (TZ III.17–26).
fn structure_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let Some(est) = app.estimate().cloned() else {
        no_estimate(ui);
        return;
    };
    let items: Vec<crate::domain::EstimateItem> = app
        .estimate_items
        .iter()
        .filter(|i| i.estimate_id == est.id)
        .cloned()
        .collect();
    if items.is_empty() {
        no_estimate(ui);
        return;
    }

    let direct: f64 = items.iter().map(|i| i.computed()).sum();
    let totals = est.totals(direct);
    let issues = crate::checks::estimate_issues(&est, &items);
    let mut edited: Option<crate::domain::Estimate> = None;

    // ---- Koeffitsiyentlar.
    let w = (ui.available_width() - 20.0).min(880.0);
    card_frame(ui, t("est_coefficients"), w, |ui| {
        ui.label(
            RichText::new(t("est_coefficients_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.add_space(6.0);
        let mut e = est.clone();
        let mut changed = false;
        let mut row = |ui: &mut egui::Ui, label: &str, v: &mut f64, hint: &str| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [200.0, 20.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                changed |= ui
                    .add_sized(
                        [90.0, 22.0],
                        egui::DragValue::new(v)
                            .speed(0.5)
                            .range(0.0..=200.0)
                            .max_decimals(2)
                            .suffix(" %"),
                    )
                    .changed();
                ui.label(RichText::new(hint).size(11.0).color(theme::muted()));
            });
        };
        row(
            ui,
            t("est_overhead"),
            &mut e.overhead_pct,
            t("est_overhead_hint"),
        );
        row(ui, t("est_profit"), &mut e.profit_pct, t("est_profit_hint"));
        row(ui, t("est_vat"), &mut e.vat_pct, t("est_vat_hint"));
        if changed {
            edited = Some(e);
        }

        ui.add_space(8.0);
        // Hisob ochiq turadi: qaysi son qayerdan chiqqani ko'rinsin.
        let line = |ui: &mut egui::Ui, label: &str, v: f64, strong: bool| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [200.0, 20.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                let text = RichText::new(money(v)).size(if strong { 13.5 } else { 12.5 });
                ui.label(if strong { text.strong() } else { text });
            });
        };
        line(ui, t("est_direct"), totals.direct, false);
        line(ui, t("est_overhead"), totals.overhead, false);
        line(ui, t("est_profit"), totals.profit, false);
        line(ui, t("est_before_vat"), totals.before_vat, false);
        line(ui, t("est_vat"), totals.vat, false);
        line(ui, t("est_total"), totals.total, true);
        if est.declared_total > 0.0 {
            line(ui, t("est_declared"), est.declared_total, false);
        }

        // Kamchiliklar ochiq aytiladi.
        for i in &issues {
            let (text, color) = issue_text(i);
            ui.label(RichText::new(text).size(11.5).color(color));
        }
    });
    ui.add_space(10.0);

    // ---- GPR bilan bog'lanish.
    let cov = crate::checks::estimate_coverage(&items, &app.tasks);
    card_frame(ui, t("est_link"), w, |ui| {
        ui.label(
            RichText::new(t("est_link_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("{:.0}%", cov.linked_pct))
                    .size(16.0)
                    .strong()
                    .color(if cov.linked_pct >= 80.0 {
                        theme::ok()
                    } else if cov.linked_pct >= 40.0 {
                        theme::warn()
                    } else {
                        theme::danger()
                    }),
            );
            ui.label(
                RichText::new(t("est_linked"))
                    .size(11.5)
                    .color(theme::muted()),
            );
        });
        if !cov.free_items.is_empty() {
            ui.label(
                RichText::new(format!(
                    "{} {} · {}",
                    t("est_free_items"),
                    cov.free_items.len(),
                    money(cov.free_cost)
                ))
                .size(11.5)
                .color(theme::warn()),
            );
        }
        if !cov.free_tasks.is_empty() {
            ui.label(
                RichText::new(format!("{} {}", t("est_free_tasks"), cov.free_tasks.len()))
                    .size(11.5)
                    .color(theme::muted()),
            );
        }
    });
    ui.add_space(10.0);

    // ---- Byudjet bilan solishtirish.
    let budget = crate::checks::estimate_vs_budget(&items, &app.purchase_budgets);
    card_frame(ui, t("est_budget"), w, |ui| {
        ui.label(
            RichText::new(t("est_budget_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.add_space(6.0);
        egui::Grid::new("est_budget_grid")
            .num_columns(4)
            .spacing([10.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for h in [
                    t("col_section"),
                    t("nav_estimate_short"),
                    t("col_planned"),
                    t("col_diff"),
                ] {
                    ui.label(RichText::new(h).size(11.0).color(theme::muted()));
                }
                ui.end_row();
                for l in &budget {
                    cell_l(ui, 140.0, RichText::new(l.section.label()).size(12.0));
                    cell_r(ui, 150.0, RichText::new(money(l.estimate)).size(12.0));
                    cell_r(
                        ui,
                        150.0,
                        RichText::new(if l.budget > 0.0 {
                            money(l.budget)
                        } else {
                            t("dash").to_string()
                        })
                        .size(12.0)
                        .color(theme::muted()),
                    );
                    // Byudjet smetadan kam bo'lsa — yetishmaydi.
                    cell_r(
                        ui,
                        150.0,
                        RichText::new(if l.budget > 0.0 {
                            money(l.gap)
                        } else {
                            t("dash").to_string()
                        })
                        .size(12.0)
                        .color(if l.budget > 0.0 && l.gap < 0.0 {
                            theme::danger()
                        } else if l.budget > 0.0 {
                            theme::ok()
                        } else {
                            theme::muted()
                        }),
                    );
                    ui.end_row();
                }
            });
    });

    // ---- Variantlarni solishtirish (TZ III.21).
    let others: Vec<&crate::domain::Estimate> =
        app.estimates.iter().filter(|x| x.id != est.id).collect();
    if !others.is_empty() {
        ui.add_space(10.0);
        let key = egui::Id::new("est_cmp");
        let mut other = ui
            .data(|d| d.get_temp::<i64>(key))
            .filter(|id| others.iter().any(|x| x.id == *id))
            .unwrap_or(others[0].id);
        card_frame(ui, t("est_compare"), w, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(t("est_compare_with"))
                        .size(11.5)
                        .color(theme::muted()),
                );
                egui::ComboBox::from_id_salt("est_cmp_pick")
                    .selected_text(
                        others
                            .iter()
                            .find(|x| x.id == other)
                            .map(|x| x.name.clone())
                            .unwrap_or_default(),
                    )
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for x in &others {
                            ui.selectable_value(&mut other, x.id, &x.name);
                        }
                    });
            });
            ui.add_space(6.0);
            let diff = crate::checks::compare_estimates(&app.estimate_items, est.id, other);
            egui::Grid::new("est_cmp_grid")
                .num_columns(5)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for h in [
                        t("col_section"),
                        t("est_current"),
                        t("est_other"),
                        t("col_diff"),
                        "%",
                    ] {
                        ui.label(RichText::new(h).size(11.0).color(theme::muted()));
                    }
                    ui.end_row();
                    for l in &diff {
                        cell_l(ui, 140.0, RichText::new(l.section.label()).size(12.0));
                        cell_r(ui, 150.0, RichText::new(money(l.left)).size(12.0));
                        cell_r(ui, 150.0, RichText::new(money(l.right)).size(12.0));
                        // Ikkinchi variant qimmatroq bo'lsa qizil.
                        let color = if l.diff > 0.0 {
                            theme::danger()
                        } else if l.diff < 0.0 {
                            theme::ok()
                        } else {
                            theme::muted()
                        };
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(l.diff)).size(12.0).color(color),
                        );
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(match l.diff_pct {
                                Some(v) => format!("{v:+.0}%"),
                                None => t("dash").to_string(),
                            })
                            .size(11.5)
                            .color(color),
                        );
                        ui.end_row();
                    }
                });
        });
        ui.data_mut(|d| d.insert_temp(key, other));
    }

    if let Some(e) = edited {
        app.db.update_estimate(&e);
        app.reload_modules();
    }
}

/// Smeta yo'q — nima qilish kerakligini aytamiz.
fn no_estimate(ui: &mut egui::Ui) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(t("estimate_empty"))
                .color(theme::muted())
                .size(15.0),
        );
    });
}

/// Kamchilikning o'qiladigan matni va rangi.
fn issue_text(i: &crate::checks::EstimateIssue) -> (String, Color32) {
    use crate::checks::EstimateIssue;
    match i {
        EstimateIssue::NoOverhead => (t("est_no_overhead").to_string(), theme::warn()),
        EstimateIssue::NoProfit => (t("est_no_profit").to_string(), theme::warn()),
        EstimateIssue::NoVat => (t("est_no_vat").to_string(), theme::warn()),
        EstimateIssue::Suspicious { name, pct } => (
            format!(
                "{} {}: {pct:.1}%",
                t("est_suspicious"),
                if *name == "overhead" {
                    t("est_overhead")
                } else {
                    t("est_profit")
                }
            ),
            theme::danger(),
        ),
        EstimateIssue::TotalMismatch { computed, declared } => (
            format!(
                "{} {} · {} {}",
                t("est_mismatch"),
                money(*computed),
                t("est_declared"),
                money(*declared)
            ),
            theme::danger(),
        ),
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
                    &["xlsx", "xlsm", "xls", "xlsb", "ods", "csv", "pdf"],
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
                overhead_pct: 0.0,
                profit_pct: 0.0,
                vat_pct: 0.0,
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
            task_id: None,
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

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
