//! «Shartnomalar» ekrani (TZ VIII.11-12, 21-22, 27-30).
//!
//! Obyekt pasportidagi shartnoma summasi bitta umumiy raqam. Bu yerda esa
//! shartnomalar alohida turadi va ularning **hayoti** ko'rinadi: qiymat qanday
//! o'zgardi, to'lov jadvali qanday bajarilyapti, qaysi ish topshirilgan.
//!
//! Ikki qoida:
//! 1. **Faqat tasdiqlangan o'zgarish** shartnoma summasini siljitadi. Qaror
//!    kutayotgani alohida ustunda turadi — u hali pul emas.
//! 2. Bu yerdagi «kelishildi» — ilova ichidagi qaror. Elektron raqamli imzo
//!    emas: kim va qachon qaror qilgani yoziladi, yuridik imzo qog'ozda qoladi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::checks;
use crate::domain::{
    AcceptState, ChangeKind, ChangeStatus, Contract, ContractChange, ContractKind, ContractStatus,
    PaymentStage, WorkAcceptance,
};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(16.0),
            );
        });
        return;
    };

    kpi_row(ui, app);
    ui.add_space(10.0);

    let tab_key = egui::Id::new("ct_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("ct_tab_contracts")),
            (1, t("ct_tab_changes")),
            (2, t("ct_tab_stages")),
            (3, t("ct_tab_accept")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => changes_tab(ui, app, pid),
        2 => stages_tab(ui, app, pid),
        3 => accept_tab(ui, app, pid),
        _ => contracts_tab(ui, app, pid),
    }
}

// ================================================================ Ko'rsatkichlar

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let today = app.today;
    let base: f64 = app.contracts.iter().map(|c| c.sum).sum();
    let approved: f64 = app
        .contract_changes
        .iter()
        .filter(|x| x.counts())
        .map(|x| x.amount)
        .sum();
    let pending = app.contract_changes.iter().filter(|x| x.pending()).count();
    let d = checks::payment_discipline(&app.payment_stages, today);
    let waiting = app.acceptances.iter().filter(|x| x.pending()).count();

    stat_row(
        ui,
        vec![
            stat(
                t("ct_kpi_sum"),
                money(base + approved),
                &if base > 0.0 {
                    format!("{:+.1}% {}", approved / base * 100.0, t("ct_from_base"))
                } else {
                    t("ct_no_contracts").to_string()
                },
                theme::text(),
            ),
            stat(
                t("ct_kpi_pending"),
                pending.to_string(),
                t("ct_kpi_pending_hint"),
                if pending == 0 {
                    theme::muted()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("ct_kpi_paid"),
                money(d.paid),
                &if d.planned > 0.0 {
                    format!("{:.0}% {}", d.paid / d.planned * 100.0, t("ct_of_schedule"))
                } else {
                    t("dash").to_string()
                },
                theme::ok(),
            ),
            stat(
                t("ct_kpi_debt"),
                money(d.debt),
                &if d.max_delay > 0 {
                    format!("{} {}", d.max_delay, t("ct_days_late"))
                } else {
                    t("ct_no_debt").to_string()
                },
                if d.debt == 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("ct_kpi_soon"),
                money(d.due_soon),
                t("ct_kpi_soon_hint"),
                theme::text(),
            ),
            stat(
                t("ct_kpi_accept"),
                waiting.to_string(),
                t("ct_kpi_accept_hint"),
                if waiting == 0 {
                    theme::muted()
                } else {
                    theme::accent()
                },
            ),
        ],
    );
}

// ================================================================ Shartnomalar

fn contracts_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Contracts);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("ct_add")).clicked() {
            app.db.insert_contract(&Contract {
                id: 0,
                project_id: pid,
                number: String::new(),
                name: String::new(),
                kind: ContractKind::General,
                party_id: None,
                signed: app.today,
                start: app.today,
                end: app.today + chrono::Duration::days(365),
                sum: 0.0,
                advance_pct: 0.0,
                retention_pct: 0.0,
                currency: app.settings.default_currency.clone(),
                status: ContractStatus::Draft,
                note: String::new(),
            });
            app.reload_modules();
        }
        ui.label(RichText::new(t("ct_hint")).size(11.0).color(theme::muted()));
    });
    ui.add_space(8.0);

    if app.contracts.is_empty() {
        empty(ui, t("ct_empty"));
        return;
    }

    let today = app.today;
    let changes = app.contract_changes.clone();
    let stages = app.payment_stages.clone();
    let mut edited: Option<Contract> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ct_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 100.0, t("col_number"));
                    head_l(ui, 230.0, t("col_name"));
                    head_l(ui, 120.0, t("col_kind"));
                    head_l(ui, 104.0, t("ct_signed"));
                    head_l(ui, 104.0, t("ct_end"));
                    head_r(ui, 150.0, t("ct_base"));
                    head_r(ui, 150.0, t("ct_current"));
                    head_r(ui, 90.0, t("ct_change"));
                    head_r(ui, 150.0, t("ct_paid"));
                    head_l(ui, 120.0, t("col_status"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.contracts {
                        let mut c = src.clone();
                        let mut changed = false;
                        let st = checks::contract_state(&c, &changes, &stages, today);

                        changed |= ui
                            .add_sized([100.0, 22.0], egui::TextEdit::singleline(&mut c.number))
                            .changed();
                        changed |= ui
                            .add_sized([230.0, 22.0], egui::TextEdit::singleline(&mut c.name))
                            .changed();
                        egui::ComboBox::from_id_salt(("ct_kind", c.id))
                            .selected_text(c.kind.label())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for k in ContractKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut c.kind, *k, k.label()).changed();
                                }
                            });
                        changed |=
                            super::passport::date_edit(ui, &format!("cts{}", c.id), &mut c.signed);
                        ui.horizontal(|ui| {
                            changed |=
                                super::passport::date_edit(ui, &format!("cte{}", c.id), &mut c.end);
                            if c.overdue(today) {
                                ui.label(RichText::new("!").color(theme::danger()).strong());
                            }
                        });

                        changed |= super::materials::num_edit(ui, 150.0, &mut c.sum, 1e6, 1e13);

                        // Amaldagi summa hisoblanadi — qo'lda kiritilmaydi.
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(st.current))
                                .size(12.0)
                                .strong()
                                .color(theme::text()),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(if st.approved_changes == 0.0 {
                                t("dash").to_string()
                            } else {
                                format!("{:+.1}%", st.change_pct())
                            })
                            .size(12.0)
                            .color(if st.approved_changes > 0.0 {
                                theme::warn()
                            } else if st.approved_changes < 0.0 {
                                theme::ok()
                            } else {
                                theme::muted()
                            }),
                        )
                        .on_hover_text(format!(
                            "{}: {}\n{}: {}\n{}: {}",
                            t("ct_approved"),
                            money(st.approved_changes),
                            t("ct_pending"),
                            money(st.pending_changes),
                            t("ct_days_shift"),
                            st.approved_days
                        ));
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(st.paid))
                                .size(12.0)
                                .color(if st.overdue > 0.0 {
                                    theme::danger()
                                } else {
                                    theme::ok()
                                }),
                        )
                        .on_hover_text(format!(
                            "{}: {}\n{}: {}\n{}: {}",
                            t("ct_scheduled"),
                            money(st.planned),
                            t("ct_debt"),
                            money(st.overdue),
                            // Jadval amaldagi summani qoplamasa — u eskirgan.
                            t("ct_gap"),
                            money(st.schedule_gap())
                        ));

                        egui::ComboBox::from_id_salt(("ct_st", c.id))
                            .selected_text(
                                RichText::new(c.status.label()).color(status_color(c.status)),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for v in ContractStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut c.status, *v, v.label()).changed();
                                }
                            });

                        if can
                            && ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                        {
                            removed = Some(c.id);
                        }
                        ui.end_row();

                        if changed && can {
                            edited = Some(c);
                        }
                    }
                });
        });

    if let Some(c) = edited {
        app.db.update_contract(&c);
        if let Some(slot) = app.contracts.iter_mut().find(|x| x.id == c.id) {
            *slot = c;
        }
    }
    if let Some(id) = removed {
        app.db.delete_contract(id);
        app.reload_modules();
    }
}

// ================================================================ O'zgarishlar

fn changes_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Contracts);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("ct_add_change")).clicked() {
            app.db.insert_contract_change(&ContractChange {
                id: 0,
                project_id: pid,
                contract_id: app.contracts.first().map(|c| c.id),
                number: String::new(),
                kind: ChangeKind::Extra,
                date: app.today,
                description: String::new(),
                amount: 0.0,
                days: 0,
                reason: String::new(),
                status: ChangeStatus::Draft,
                decided_at: None,
                decided_by: String::new(),
                note: String::new(),
            });
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("ct_changes_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.contract_changes.is_empty() {
        empty(ui, t("ct_changes_empty"));
        return;
    }

    let today = app.today;
    let contracts = app.contracts.clone();
    let mut edited: Option<ContractChange> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ct_ch_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 80.0, t("col_number"));
                    head_l(ui, 130.0, t("ct_contract"));
                    head_l(ui, 120.0, t("col_kind"));
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 280.0, t("col_description"));
                    head_r(ui, 150.0, t("ct_amount"));
                    head_r(ui, 70.0, t("ct_days"));
                    head_l(ui, 130.0, t("col_status"));
                    head_l(ui, 130.0, t("ct_decided"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.contract_changes {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut x.number))
                            .changed();
                        let label = x
                            .contract_id
                            .and_then(|id| contracts.iter().find(|c| c.id == id))
                            .map(|c| c.number.clone())
                            .unwrap_or_else(|| t("dash").to_string());
                        egui::ComboBox::from_id_salt(("ct_ch_c", x.id))
                            .selected_text(label)
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut x.contract_id, None, t("dash"))
                                    .changed();
                                for c in &contracts {
                                    changed |= ui
                                        .selectable_value(
                                            &mut x.contract_id,
                                            Some(c.id),
                                            format!("{} {}", c.number, c.name),
                                        )
                                        .changed();
                                }
                            });
                        egui::ComboBox::from_id_salt(("ct_ch_k", x.id))
                            .selected_text(x.kind.label())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for k in ChangeKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut x.kind, *k, k.label()).changed();
                                }
                            });
                        changed |=
                            super::passport::date_edit(ui, &format!("ctcd{}", x.id), &mut x.date);
                        changed |= ui
                            .add_sized(
                                [280.0, 22.0],
                                egui::TextEdit::singleline(&mut x.description),
                            )
                            .changed();
                        changed |= super::materials::num_edit(ui, 150.0, &mut x.amount, 1e6, 1e13);
                        let mut days = x.days as f64;
                        if super::materials::num_edit(ui, 70.0, &mut days, 1.0, 3650.0) {
                            x.days = days.round() as i64;
                            changed = true;
                        }

                        let before = x.status;
                        egui::ComboBox::from_id_salt(("ct_ch_s", x.id))
                            .selected_text(
                                RichText::new(x.status.label()).color(change_color(x.status)),
                            )
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for v in ChangeStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut x.status, *v, v.label()).changed();
                                }
                            });
                        // Qaror qabul qilingan payt o'z-o'zidan yoziladi.
                        if x.status != before {
                            match x.status {
                                ChangeStatus::Approved | ChangeStatus::Rejected => {
                                    x.decided_at = Some(today);
                                }
                                _ => {
                                    x.decided_at = None;
                                    x.decided_by.clear();
                                }
                            }
                        }
                        changed |= ui
                            .add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut x.decided_by))
                            .changed();

                        if can
                            && ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                        {
                            removed = Some(x.id);
                        }
                        ui.end_row();

                        if changed && can {
                            edited = Some(x);
                        }
                    }
                });
        });

    if let Some(x) = edited {
        app.db.update_contract_change(&x);
        if let Some(slot) = app.contract_changes.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_contract_change(id);
        app.reload_modules();
    }
}

// ================================================================ To'lov jadvali

fn stages_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Contracts);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("ct_add_stage")).clicked() {
            app.db.insert_payment_stage(&PaymentStage {
                id: 0,
                project_id: pid,
                contract_id: app.contracts.first().map(|c| c.id),
                number: String::new(),
                basis: String::new(),
                due: app.today,
                amount: 0.0,
                paid: 0.0,
                paid_at: None,
                note: String::new(),
            });
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("ct_stages_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.payment_stages.is_empty() {
        empty(ui, t("ct_stages_empty"));
        return;
    }

    let today = app.today;
    let contracts = app.contracts.clone();
    let mut edited: Option<PaymentStage> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ct_st_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 80.0, t("col_number"));
                    head_l(ui, 130.0, t("ct_contract"));
                    head_l(ui, 220.0, t("ct_basis"));
                    head_l(ui, 104.0, t("ct_due"));
                    head_r(ui, 150.0, t("ct_amount"));
                    head_r(ui, 150.0, t("ct_paid"));
                    head_r(ui, 140.0, t("ct_left"));
                    head_l(ui, 150.0, t("ct_paid_at"));
                    head_l(ui, 110.0, t("ct_state"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.payment_stages {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut x.number))
                            .changed();
                        let label = x
                            .contract_id
                            .and_then(|id| contracts.iter().find(|c| c.id == id))
                            .map(|c| c.number.clone())
                            .unwrap_or_else(|| t("dash").to_string());
                        egui::ComboBox::from_id_salt(("ct_st_c", x.id))
                            .selected_text(label)
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut x.contract_id, None, t("dash"))
                                    .changed();
                                for c in &contracts {
                                    changed |= ui
                                        .selectable_value(
                                            &mut x.contract_id,
                                            Some(c.id),
                                            format!("{} {}", c.number, c.name),
                                        )
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut x.basis))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("ctsd{}", x.id), &mut x.due);
                        changed |= super::materials::num_edit(ui, 150.0, &mut x.amount, 1e6, 1e13);
                        changed |= super::materials::num_edit(ui, 150.0, &mut x.paid, 1e6, 1e13);

                        // Qoldiq hisoblanadi.
                        cell_r(
                            ui,
                            140.0,
                            RichText::new(if x.left() == 0.0 {
                                t("dash").to_string()
                            } else {
                                money(x.left())
                            })
                            .size(12.0)
                            .color(if x.overdue(today) {
                                theme::danger()
                            } else {
                                theme::muted()
                            }),
                        );

                        ui.horizontal(|ui| {
                            let mut has = x.paid_at.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                x.paid_at = has.then_some(today);
                                changed = true;
                            }
                            if let Some(mut p) = x.paid_at {
                                if super::passport::date_edit(ui, &format!("ctp{}", x.id), &mut p) {
                                    x.paid_at = Some(p);
                                    changed = true;
                                }
                            }
                        });

                        // Holat hisobdan chiqadi: to'langan, kechikkan yoki kutilmoqda.
                        let delay = x.delay_days(today);
                        cell_l(
                            ui,
                            110.0,
                            if x.closed() {
                                RichText::new(t("ct_s_closed"))
                                    .size(11.5)
                                    .color(theme::ok())
                            } else if delay > 0 {
                                RichText::new(format!("{} {}", delay, t("ct_days_late")))
                                    .size(11.5)
                                    .color(theme::danger())
                            } else {
                                RichText::new(t("ct_s_waiting"))
                                    .size(11.5)
                                    .color(theme::muted())
                            },
                        );

                        if can
                            && ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                        {
                            removed = Some(x.id);
                        }
                        ui.end_row();

                        if changed && can {
                            edited = Some(x);
                        }
                    }
                });
        });

    if let Some(x) = edited {
        app.db.update_payment_stage(&x);
        if let Some(slot) = app.payment_stages.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_payment_stage(id);
        app.reload_modules();
    }
}

// ================================================================ Ishlarni qabul qilish

fn accept_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Contracts);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("ct_add_accept")).clicked() {
            app.db.insert_work_acceptance(&WorkAcceptance {
                id: 0,
                project_id: pid,
                task_id: None,
                number: String::new(),
                date: app.today,
                volume: 0.0,
                unit: String::new(),
                amount: 0.0,
                state: AcceptState::Submitted,
                decided_at: None,
                decided_by: String::new(),
                comment: String::new(),
            });
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("ct_accept_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.acceptances.is_empty() {
        empty(ui, t("ct_accept_empty"));
        return;
    }

    let today = app.today;
    let tasks = app.tasks.clone();
    let mut edited: Option<WorkAcceptance> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ct_ac_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 90.0, t("col_number"));
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 220.0, t("col_task"));
                    head_r(ui, 100.0, t("col_volume"));
                    head_l(ui, 70.0, t("col_unit"));
                    head_r(ui, 150.0, t("ct_amount"));
                    head_l(ui, 190.0, t("ct_state"));
                    head_l(ui, 130.0, t("ct_decided"));
                    head_l(ui, 240.0, t("ct_comment"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.acceptances {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut x.number))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("ctad{}", x.id), &mut x.date);

                        let label = x
                            .task_id
                            .and_then(|id| tasks.iter().find(|t| t.id == id))
                            .map(|t| format!("{} {}", t.wbs, t.name))
                            .unwrap_or_else(|| t("dash").to_string());
                        egui::ComboBox::from_id_salt(("ct_ac_t", x.id))
                            .selected_text(super::issues::truncate(&label, 26))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut x.task_id, None, t("dash"))
                                    .changed();
                                for tk in &tasks {
                                    changed |= ui
                                        .selectable_value(
                                            &mut x.task_id,
                                            Some(tk.id),
                                            format!("{} {}", tk.wbs, tk.name),
                                        )
                                        .changed();
                                }
                            });
                        changed |= super::materials::num_edit(ui, 100.0, &mut x.volume, 1.0, 1e7);
                        changed |= ui
                            .add_sized([70.0, 22.0], egui::TextEdit::singleline(&mut x.unit))
                            .changed();
                        changed |= super::materials::num_edit(ui, 150.0, &mut x.amount, 1e6, 1e13);

                        // Qaror: qabul qilish yoki rad etish. Ikkalasi ham
                        // sana va kim qaror qilganini yozadi.
                        ui.horizontal(|ui| {
                            if x.pending() && can {
                                if ui
                                    .small_button(RichText::new(t("ct_accept")).color(theme::ok()))
                                    .clicked()
                                {
                                    x.state = AcceptState::Accepted;
                                    x.decided_at = Some(today);
                                    changed = true;
                                }
                                if ui
                                    .small_button(
                                        RichText::new(t("ct_reject")).color(theme::danger()),
                                    )
                                    .clicked()
                                {
                                    x.state = AcceptState::Rejected;
                                    x.decided_at = Some(today);
                                    changed = true;
                                }
                            } else {
                                ui.label(
                                    RichText::new(x.state.label())
                                        .size(11.5)
                                        .color(accept_color(x.state)),
                                );
                                if let Some(dt) = x.decided_at {
                                    ui.label(
                                        RichText::new(dt.format("%d.%m.%Y").to_string())
                                            .size(11.0)
                                            .color(theme::muted()),
                                    );
                                }
                                if !x.pending() && can && ui.small_button(t("ct_reopen")).clicked()
                                {
                                    x.state = AcceptState::Submitted;
                                    x.decided_at = None;
                                    x.decided_by.clear();
                                    changed = true;
                                }
                            }
                        });

                        changed |= ui
                            .add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut x.decided_by))
                            .changed();
                        changed |= ui
                            .add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut x.comment))
                            .changed();

                        if can
                            && ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                        {
                            removed = Some(x.id);
                        }
                        ui.end_row();

                        if changed && can {
                            edited = Some(x);
                        }
                    }
                });
        });

    if let Some(x) = edited {
        app.db.update_work_acceptance(&x);
        if let Some(slot) = app.acceptances.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_work_acceptance(id);
        app.reload_modules();
    }
}

// ================================================================ Yordamchilar

fn status_color(s: ContractStatus) -> egui::Color32 {
    match s {
        ContractStatus::Active => theme::ok(),
        ContractStatus::Draft => theme::muted(),
        ContractStatus::Suspended => theme::warn(),
        ContractStatus::Closed => theme::text(),
    }
}

fn change_color(s: ChangeStatus) -> egui::Color32 {
    match s {
        ChangeStatus::Approved => theme::ok(),
        ChangeStatus::Sent => theme::warn(),
        ChangeStatus::Rejected => theme::danger(),
        ChangeStatus::Draft => theme::muted(),
    }
}

fn accept_color(s: AcceptState) -> egui::Color32 {
    match s {
        AcceptState::Accepted => theme::ok(),
        AcceptState::Rejected => theme::danger(),
        AcceptState::Submitted => theme::warn(),
    }
}

fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(msg).color(theme::muted()).size(15.0));
    });
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
