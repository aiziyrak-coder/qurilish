//! «Buyurtmachi kabineti» ekrani (TZ VIII).
//!
//! Buyurtmachiga ko'rinadigan holat: muddat, pul, bo'limlar bo'yicha bajarilish,
//! oxirgi ish kunlari va sotuv (agar obyekt sotuvda bo'lsa). Ekran **faqat
//! o'qish uchun** — bu yerdan hech narsa o'zgartirilmaydi, chunki buyurtmachi
//! ijro ma'lumotini kiritmaydi.
//!
//! Masofadan kirish va alohida buyurtmachi hisobi server qismini talab qiladi —
//! bu yerda ko'zda tutilmagan; hozircha bu ichki ko'rinish.

use super::*;
use crate::model::Section;
use egui::{vec2, Sense};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let Some(project) = app.project().cloned() else {
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

    ui.horizontal(|ui| {
        ui.label(RichText::new(&project.name).size(16.0).strong());
        if !project.address.is_empty() {
            ui.label(
                RichText::new(&project.address)
                    .size(12.0)
                    .color(theme::muted()),
            );
        }
    });
    ui.label(RichText::new(t("cl_hint")).size(11.0).color(theme::muted()));
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                kpi_row(ui, app, &project);
                ui.add_space(12.0);
                sections_block(ui, app);
                ui.add_space(12.0);
                finance_block(ui, app, &project);
                ui.add_space(12.0);
                week_block(ui, app);
                if !app.contracts.is_empty() {
                    ui.add_space(12.0);
                    contracts_block(ui, app);
                }
                ui.add_space(12.0);
                payments_block(ui, app);
                acceptance_block(ui, app);
                ui.add_space(12.0);
                recent_block(ui, app);
                if !app.units.is_empty() {
                    ui.add_space(12.0);
                    sales_block(ui, app);
                }
                ui.add_space(20.0);
            });
        });
}

fn kpi_row(ui: &mut egui::Ui, app: &App, project: &crate::model::Project) {
    let p = &app.progress;
    let left = (project.planned_end - app.today).num_days();

    stat_row(
        ui,
        vec![
            stat(
                t("cl_progress"),
                format!("{:.0}%", p.fact_pct),
                &format!("{} {:.0}%", t("an_plan"), p.plan_pct),
                if p.fact_pct + 2.0 >= p.plan_pct {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("cl_deadline"),
                project.planned_end.format("%d.%m.%Y").to_string(),
                &if left >= 0 {
                    format!("{left} {}", t("cl_days_left"))
                } else {
                    format!("{} {}", -left, t("cl_days_over"))
                },
                if left >= 0 {
                    theme::text()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("cl_forecast"),
                match p.forecast_end {
                    Some(d) => d.format("%d.%m.%Y").to_string(),
                    None => t("dash").to_string(),
                },
                &if p.delay_days > 0 {
                    format!("{} {} {}", t("an_delay"), p.delay_days, t("days_short"))
                } else {
                    t("cl_on_time").to_string()
                },
                if p.delay_days > 0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("cl_contract"),
                money(project.contract_sum),
                t("cl_contract_hint"),
                theme::text(),
            ),
            stat(
                t("cl_paid"),
                money(project.paid_total),
                &if project.contract_sum > 0.0 {
                    format!(
                        "{:.0}% {}",
                        project.paid_total / project.contract_sum * 100.0,
                        t("cl_of_contract")
                    )
                } else {
                    String::new()
                },
                theme::ok(),
            ),
        ],
    );
}

fn block(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(14.0).strong());
                ui.add_space(8.0);
                add(ui);
            });
        });
}

/// Bo'limlar kesimidagi bajarilish — vaznlangan o'rtacha, ish davomiyligi bo'yicha.
fn sections_block(ui: &mut egui::Ui, app: &App) {
    block(ui, t("cl_sections"), |ui| {
        let mut any = false;
        for s in Section::ALL {
            let tasks: Vec<&crate::model::Task> =
                app.tasks.iter().filter(|t| t.section == s).collect();
            if tasks.is_empty() {
                continue;
            }
            any = true;
            let total: f64 = tasks.iter().map(|t| t.duration.max(1) as f64).sum();
            let done: f64 = tasks
                .iter()
                .map(|t| t.duration.max(1) as f64 * t.progress / 100.0)
                .sum();
            let pct = if total > 0.0 {
                done / total * 100.0
            } else {
                0.0
            };

            ui.horizontal(|ui| {
                ui.add_sized(
                    [70.0, 18.0],
                    egui::Label::new(RichText::new(s.label()).size(12.0)),
                );
                bar(ui, pct, 320.0);
                ui.label(
                    RichText::new(format!("{pct:.0}%"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.label(
                    RichText::new(format!("{} {}", tasks.len(), t("an_tasks").to_lowercase()))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });
        }
        if !any {
            ui.label(
                RichText::new(t("cl_no_sections"))
                    .size(12.0)
                    .color(theme::muted()),
            );
        }
    });
}

/// Bajarilish polosasi.
fn bar(ui: &mut egui::Ui, pct: f64, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 10.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, theme::track());
    let frac = (pct / 100.0).clamp(0.0, 1.0) as f32;
    let color = if pct >= 99.0 {
        theme::ok()
    } else if pct > 0.0 {
        theme::accent()
    } else {
        theme::muted()
    };
    p.rect_filled(
        Rect::from_min_size(rect.min, vec2(rect.width() * frac, rect.height())),
        3.0,
        color,
    );
}

fn finance_block(ui: &mut egui::Ui, app: &App, project: &crate::model::Project) {
    let cost = app.cost_summary();
    let earned = project.contract_sum * app.progress.fact_pct / 100.0;

    block(ui, t("cl_finance"), |ui| {
        let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [230.0, 18.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                ui.label(RichText::new(value).size(13.5).color(color));
            });
        };
        line(
            ui,
            t("cl_contract"),
            money(project.contract_sum),
            theme::text(),
        );
        line(ui, t("cl_estimate"), money(cost.total), theme::text());
        line(ui, t("cl_earned"), money(earned), theme::accent());
        line(ui, t("cl_paid"), money(project.paid_total), theme::ok());
        // Bajarilgan, ammo hali to'lanmagan qism.
        let unpaid = (earned - project.paid_total).max(0.0);
        line(
            ui,
            t("cl_unpaid"),
            money(unpaid),
            if unpaid > 0.0 {
                theme::warn()
            } else {
                theme::ok()
            },
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(t("cl_finance_note"))
                .size(10.5)
                .color(theme::muted()),
        );
    });
}

/// Oxirgi jurnal yozuvlari — buyurtmachi obyektda nima bo'layotganini ko'radi.
fn recent_block(ui: &mut egui::Ui, app: &App) {
    block(ui, t("cl_recent"), |ui| {
        let mut list: Vec<&crate::domain::JournalEntry> = app.journal.iter().collect();
        // Yangi yozuv yuqorida.
        list.sort_by_key(|j| std::cmp::Reverse(j.date));
        if list.is_empty() {
            ui.label(
                RichText::new(t("cl_no_recent"))
                    .size(12.0)
                    .color(theme::muted()),
            );
            return;
        }
        for j in list.iter().take(7) {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [90.0, 18.0],
                    egui::Label::new(
                        RichText::new(j.date.format("%d.%m.%Y").to_string())
                            .size(11.5)
                            .monospace()
                            .color(theme::muted()),
                    ),
                );
                ui.label(RichText::new(super::issues::truncate(&j.text, 70)).size(12.5));
                if j.volume > 0.0 {
                    ui.label(
                        RichText::new(format!("{} {}", trim(j.volume), j.unit))
                            .size(11.5)
                            .color(theme::accent()),
                    );
                }
            });
        }
    });
}

fn sales_block(ui: &mut egui::Ui, app: &App) {
    let s = app.sales();
    block(ui, t("cl_sales"), |ui| {
        let sold_pct = if s.units > 0 {
            s.sold as f64 / s.units as f64 * 100.0
        } else {
            0.0
        };
        ui.horizontal(|ui| {
            ui.add_sized(
                [230.0, 18.0],
                egui::Label::new(RichText::new(t("cl_sold")).size(12.0).color(theme::muted())),
            );
            bar(ui, sold_pct, 300.0);
            ui.label(
                RichText::new(format!("{} / {}", s.sold, s.units))
                    .size(12.0)
                    .color(theme::muted()),
            );
        });
        ui.add_space(4.0);
        let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [230.0, 18.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                ui.label(RichText::new(value).size(13.5).color(color));
            });
        };
        line(ui, t("kpi_contracted"), money(s.contracted), theme::text());
        line(ui, t("kpi_received"), money(s.received), theme::ok());
        line(
            ui,
            t("kpi_debt"),
            money(s.debt),
            if s.debt > 0.0 {
                theme::warn()
            } else {
                theme::ok()
            },
        );
    });
}

/// Shartnoma qiymati qanday o'zgargani (TZ VIII.11-12, 27).
fn contracts_block(ui: &mut egui::Ui, app: &App) {
    block(ui, t("cl_contracts"), |ui| {
        for c in &app.contracts {
            let st = crate::checks::contract_state(
                c,
                &app.contract_changes,
                &app.payment_stages,
                app.today,
            );
            ui.horizontal(|ui| {
                ui.add_sized(
                    [230.0, 18.0],
                    egui::Label::new(
                        RichText::new(format!("{} · {}", c.number, c.kind.label()))
                            .size(12.0)
                            .color(theme::muted()),
                    ),
                );
                ui.add_sized(
                    [150.0, 18.0],
                    egui::Label::new(RichText::new(money(st.current)).size(13.0).strong()),
                );
                // Dastlabki summadan chetlanish — buyurtmachi uchun asosiy savol.
                ui.add_sized(
                    [90.0, 18.0],
                    egui::Label::new(
                        RichText::new(if st.approved_changes == 0.0 {
                            t("cl_no_change").to_string()
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
                    ),
                );
                if st.pending_changes != 0.0 {
                    ui.label(
                        RichText::new(format!("{} {}", t("cl_pending"), money(st.pending_changes)))
                            .size(11.5)
                            .color(theme::accent()),
                    );
                }
                if st.approved_days > 0 {
                    ui.label(
                        RichText::new(format!("+{} {}", st.approved_days, t("cl_days")))
                            .size(11.5)
                            .color(theme::warn()),
                    );
                }
            });
        }
        if app.contracts.is_empty() {
            ui.label(
                RichText::new(t("cl_no_contracts"))
                    .size(12.0)
                    .color(theme::muted()),
            );
        }
    });
}

/// To'lov jadvali bo'yicha intizom (TZ VIII.29-30).
fn payments_block(ui: &mut egui::Ui, app: &App) {
    let d = crate::checks::payment_discipline(&app.payment_stages, app.today);
    if d.stages == 0 {
        return;
    }
    block(ui, t("cl_payments"), |ui| {
        let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [230.0, 18.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                ui.label(RichText::new(value).size(13.5).color(color));
            });
        };
        ui.horizontal(|ui| {
            ui.add_sized(
                [230.0, 18.0],
                egui::Label::new(
                    RichText::new(t("cl_pay_progress"))
                        .size(12.0)
                        .color(theme::muted()),
                ),
            );
            let pct = if d.planned > 0.0 {
                d.paid / d.planned * 100.0
            } else {
                0.0
            };
            bar(ui, pct, 300.0);
            ui.label(
                RichText::new(format!("{pct:.0}%"))
                    .size(12.0)
                    .color(theme::muted()),
            );
        });
        ui.add_space(4.0);
        line(ui, t("cl_pay_planned"), money(d.planned), theme::text());
        line(ui, t("cl_pay_paid"), money(d.paid), theme::ok());
        line(
            ui,
            t("cl_pay_debt"),
            if d.debt == 0.0 {
                t("cl_no_debt").to_string()
            } else {
                format!("{} · {} {}", money(d.debt), d.max_delay, t("cl_days_late"))
            },
            if d.debt == 0.0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        line(ui, t("cl_pay_soon"), money(d.due_soon), theme::text());
    });
}

/// Buyurtmachining qaroriga qolgan hujjatlar (TZ VIII.21).
fn acceptance_block(ui: &mut egui::Ui, app: &App) {
    let waiting: Vec<&crate::domain::WorkAcceptance> =
        app.acceptances.iter().filter(|x| x.pending()).collect();
    if waiting.is_empty() {
        return;
    }
    block(ui, t("cl_acceptance"), |ui| {
        for x in &waiting {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [110.0, 18.0],
                    egui::Label::new(RichText::new(&x.number).size(12.5).strong()),
                );
                ui.add_sized(
                    [104.0, 18.0],
                    egui::Label::new(
                        RichText::new(x.date.format("%d.%m.%Y").to_string())
                            .size(12.0)
                            .color(theme::muted()),
                    ),
                );
                let task = x
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default();
                ui.add_sized(
                    [280.0, 18.0],
                    egui::Label::new(RichText::new(super::issues::truncate(&task, 34)).size(12.0)),
                );
                ui.add_sized(
                    [150.0, 18.0],
                    egui::Label::new(RichText::new(money(x.amount)).size(12.5)),
                );
            });
        }
        ui.add_space(4.0);
        ui.label(
            RichText::new(t("cl_acceptance_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
}

/// Haftalik hisobot (TZ VIII.33): oxirgi yetti kunda nima bo'ldi.
fn week_block(ui: &mut egui::Ui, app: &App) {
    let r = crate::checks::week_report(
        app.today,
        &app.tasks,
        &app.journal,
        &app.inspections,
        &app.issues,
        &app.exec_docs,
        &app.payment_stages,
        &app.contract_changes,
        &app.acceptances,
    );
    block(ui, t("cl_week"), |ui| {
        ui.label(
            RichText::new(format!(
                "{} — {}",
                r.from.format("%d.%m.%Y"),
                r.to.format("%d.%m.%Y")
            ))
            .size(11.5)
            .color(theme::muted()),
        );
        ui.add_space(6.0);

        let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [230.0, 18.0],
                    egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
                );
                ui.label(RichText::new(value).size(13.0).color(color));
            });
        };
        line(
            ui,
            t("cl_w_progress"),
            format!(
                "{:.1}% → {:.1}% ({:+.1})",
                r.progress_start,
                r.progress_end,
                r.progress_end - r.progress_start
            ),
            if r.progress_end > r.progress_start {
                theme::ok()
            } else {
                theme::warn()
            },
        );
        line(
            ui,
            t("cl_w_tasks"),
            format!("{} / {}", r.tasks_done, r.tasks_started),
            theme::text(),
        );
        line(
            ui,
            t("cl_w_journal"),
            format!("{} / 7", r.journal_days),
            if r.journal_days >= 5 {
                theme::ok()
            } else {
                theme::warn()
            },
        );
        line(
            ui,
            t("cl_w_inspections"),
            format!("{} / {}", r.inspections, r.inspections_failed),
            if r.inspections_failed == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        line(
            ui,
            t("cl_w_issues"),
            r.issues_opened.to_string(),
            if r.issues_opened == 0 {
                theme::ok()
            } else {
                theme::warn()
            },
        );
        line(ui, t("cl_w_docs"), r.docs_signed.to_string(), theme::text());
        line(ui, t("cl_w_paid"), money(r.paid), theme::ok());
        if r.changes_pending > 0 || r.acceptances_pending > 0 {
            line(
                ui,
                t("cl_w_waiting"),
                format!("{} · {}", r.changes_pending, r.acceptances_pending),
                theme::accent(),
            );
        }
    });
}
