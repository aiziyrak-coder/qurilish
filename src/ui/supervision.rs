//! «Texnik nazorat kabineti» ekrani (TZ VII).
//!
//! Nazoratchining ish navbati: imzo kutayotgan ijro hujjatlari, talabga mos
//! kelmagan sifat yozuvlari, yopilmagan kritik nomuvofiqliklar va muddati
//! o'tgan xavfsizlik choralari — hammasi bitta ro'yxatda, qaror tugmalari bilan.
//!
//! Ekran shu kompyuterdagi bazada ishlaydi. Rollar bo'yicha kirish va imzoni
//! masofadan tasdiqlash server qismini talab qiladi va bu yerda yo'q — hozircha
//! qaror shu ilovada qayd etiladi.

use super::warehouse::cell_l;
use super::*;
use crate::domain::{ExecDocStatus, IssueStatus, QualityResult, Severity};

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

    // Ekran nomi tepada, umumiy sarlavhada turadi; bu yerda faqat
    // chegara haqidagi izoh qoladi.
    ui.label(RichText::new(t("sv_hint")).size(11.0).color(theme::muted()));
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                docs_queue(ui, app);
                ui.add_space(12.0);
                quality_queue(ui, app);
                ui.add_space(12.0);
                issues_queue(ui, app);
                ui.add_space(12.0);
                safety_queue(ui, app);
                ui.add_space(20.0);
            });
        });
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let waiting = app
        .exec_docs
        .iter()
        .filter(|d| matches!(d.status, ExecDocStatus::Draft | ExecDocStatus::OnReview))
        .count();
    let rejected = app
        .exec_docs
        .iter()
        .filter(|d| d.status == ExecDocStatus::Rejected)
        .count();
    let quality = app
        .quality
        .iter()
        .filter(|q| q.result != QualityResult::Pass)
        .count();
    let issues = app
        .issues
        .iter()
        .filter(|i| {
            i.severity == Severity::Critical
                && matches!(i.status, IssueStatus::Open | IssueStatus::InWork)
        })
        .count();
    let safety = app
        .safety
        .iter()
        .filter(|s| {
            matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                && s.deadline.is_some_and(|d| d < app.today)
        })
        .count();

    stat_row(
        ui,
        vec![
            stat(
                t("sv_kpi_docs"),
                waiting.to_string(),
                t("sv_kpi_docs_hint"),
                if waiting == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("sv_kpi_rejected"),
                rejected.to_string(),
                t("sv_kpi_rejected_hint"),
                if rejected == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("sv_kpi_quality"),
                quality.to_string(),
                t("sv_kpi_quality_hint"),
                if quality == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("sv_kpi_issues"),
                issues.to_string(),
                t("sv_kpi_issues_hint"),
                if issues == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("sv_kpi_safety"),
                safety.to_string(),
                t("sv_kpi_safety_hint"),
                if safety == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
}

fn block(ui: &mut egui::Ui, title: &str, n: usize, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).size(14.0).strong());
                    ui.label(
                        RichText::new(format!("· {n}"))
                            .size(12.0)
                            .color(theme::muted()),
                    );
                });
                ui.add_space(6.0);
                add(ui);
            });
        });
}

// ================================================================ Ijro hujjatlari

fn docs_queue(ui: &mut egui::Ui, app: &mut App) {
    let waiting: Vec<i64> = app
        .exec_docs
        .iter()
        .filter(|d| matches!(d.status, ExecDocStatus::Draft | ExecDocStatus::OnReview))
        .map(|d| d.id)
        .collect();
    // (hujjat, yangi holat)
    let mut decision: Option<(i64, ExecDocStatus)> = None;
    let mut go = false;

    block(ui, t("sv_docs"), waiting.len(), |ui| {
        if waiting.is_empty() {
            ui.label(
                RichText::new(t("sv_docs_empty"))
                    .size(12.0)
                    .color(theme::ok()),
            );
            return;
        }
        egui::ScrollArea::horizontal()
            .id_salt("sv_docs_scroll")
            .show(ui, |ui| {
                egui::Grid::new("sv_docs")
                    .num_columns(6)
                    .spacing([10.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for id in &waiting {
                            let Some(d) = app.exec_docs.iter().find(|x| x.id == *id) else {
                                continue;
                            };
                            // Qaror tugmalari eng chapda: bu ekranning asosiy amali,
                            // tor oynada surib izlash kerak bo'lmasin.
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button(RichText::new(t("sv_sign")).color(theme::ok()))
                                    .clicked()
                                {
                                    decision = Some((d.id, ExecDocStatus::Signed));
                                }
                                if ui
                                    .small_button(
                                        RichText::new(t("sv_reject")).color(theme::danger()),
                                    )
                                    .clicked()
                                {
                                    decision = Some((d.id, ExecDocStatus::Rejected));
                                }
                            });
                            cell_l(ui, 110.0, RichText::new(&d.number).size(12.5).strong());
                            cell_l(
                                ui,
                                150.0,
                                RichText::new(d.kind.label())
                                    .size(11.5)
                                    .color(theme::muted()),
                            );
                            cell_l(
                                ui,
                                320.0,
                                RichText::new(super::issues::truncate(&d.name, 46)).size(12.5),
                            );
                            cell_l(
                                ui,
                                200.0,
                                RichText::new(super::issues::truncate(
                                    &d.task_id.map(|t| app.task_name(t)).unwrap_or_default(),
                                    26,
                                ))
                                .size(12.0)
                                .color(theme::muted()),
                            );
                            cell_l(
                                ui,
                                120.0,
                                RichText::new(d.status.label()).size(11.5).color(
                                    if d.status == ExecDocStatus::Draft {
                                        theme::muted()
                                    } else {
                                        theme::accent()
                                    },
                                ),
                            );
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(6.0);
        if ui.button(t("sv_open_docs")).clicked() {
            go = true;
        }
    });

    if let Some((id, status)) = decision {
        if let Some(d) = app.exec_docs.iter_mut().find(|x| x.id == id) {
            d.status = status;
            let copy = d.clone();
            app.db.update_exec_doc(&copy);
        }
    }
    if go {
        app.screen = Screen::ExecDocs;
    }
}

// ================================================================ Sifat

fn quality_queue(ui: &mut egui::Ui, app: &mut App) {
    let today = app.today;
    let list: Vec<i64> = app
        .quality
        .iter()
        .filter(|q| q.result != QualityResult::Pass)
        .map(|q| q.id)
        .collect();
    let mut go = false;

    block(ui, t("sv_quality"), list.len(), |ui| {
        if list.is_empty() {
            ui.label(
                RichText::new(t("sv_quality_empty"))
                    .size(12.0)
                    .color(theme::ok()),
            );
            return;
        }
        for id in &list {
            let Some(q) = app.quality.iter().find(|x| x.id == *id) else {
                continue;
            };
            let overdue = q.deadline.is_some_and(|d| d < today);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if q.result == QualityResult::Fail {
                        "!"
                    } else {
                        "·"
                    })
                    .color(if q.result == QualityResult::Fail {
                        theme::danger()
                    } else {
                        theme::warn()
                    })
                    .strong(),
                );
                ui.label(RichText::new(format!("{} — {}", q.kind.label(), q.subject)).size(12.5));
                if !q.defect.is_empty() {
                    ui.label(
                        RichText::new(super::issues::truncate(&q.defect, 60))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                }
                if let Some(d) = q.deadline {
                    ui.label(
                        RichText::new(d.format("%d.%m.%Y").to_string())
                            .size(11.0)
                            .monospace()
                            .color(if overdue {
                                theme::danger()
                            } else {
                                theme::muted()
                            }),
                    );
                }
            });
        }
        ui.add_space(6.0);
        if ui.button(t("sv_open_quality")).clicked() {
            go = true;
        }
    });

    if go {
        app.screen = Screen::Quality;
    }
}

// ================================================================ Nomuvofiqliklar

fn issues_queue(ui: &mut egui::Ui, app: &mut App) {
    let list: Vec<i64> = app
        .issues
        .iter()
        .filter(|i| {
            matches!(i.severity, Severity::Critical | Severity::Major)
                && matches!(i.status, IssueStatus::Open | IssueStatus::InWork)
        })
        .map(|i| i.id)
        .collect();
    let mut go: Option<Screen> = None;

    block(ui, t("sv_issues"), list.len(), |ui| {
        if list.is_empty() {
            ui.label(
                RichText::new(t("sv_issues_empty"))
                    .size(12.0)
                    .color(theme::ok()),
            );
            return;
        }
        // Uzun ro'yxatni to'liq chiqarmaymiz — nazoratchiga birinchi o'ntasi yetadi.
        for id in list.iter().take(10) {
            let Some(i) = app.issues.iter().find(|x| x.id == *id) else {
                continue;
            };
            let color = if i.severity == Severity::Critical {
                theme::danger()
            } else {
                theme::warn()
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(i.severity.label()).size(11.0).color(color));
                ui.label(
                    RichText::new(&i.code)
                        .size(10.5)
                        .monospace()
                        .color(theme::muted()),
                );
                ui.label(RichText::new(super::issues::truncate(&i.title, 70)).size(12.5));
            });
        }
        if list.len() > 10 {
            ui.label(
                RichText::new(format!("… {} {}", list.len() - 10, t("sv_more")))
                    .size(11.0)
                    .color(theme::muted()),
            );
        }
        ui.add_space(6.0);
        if ui.button(t("sv_open_issues")).clicked() {
            go = Some(Screen::AiCheck);
        }
    });

    if let Some(s) = go {
        app.screen = s;
    }
}

// ================================================================ Xavfsizlik

fn safety_queue(ui: &mut egui::Ui, app: &mut App) {
    let today = app.today;
    let list: Vec<i64> = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .map(|s| s.id)
        .collect();
    let mut go = false;

    block(ui, t("sv_safety"), list.len(), |ui| {
        if list.is_empty() {
            ui.label(
                RichText::new(t("sv_safety_empty"))
                    .size(12.0)
                    .color(theme::ok()),
            );
            return;
        }
        for id in &list {
            let Some(s) = app.safety.iter().find(|x| x.id == *id) else {
                continue;
            };
            let overdue = s.deadline.is_some_and(|d| d < today);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if overdue { "!" } else { "·" })
                        .color(if overdue {
                            theme::danger()
                        } else {
                            theme::warn()
                        })
                        .strong(),
                );
                ui.label(
                    RichText::new(s.kind.label())
                        .size(11.5)
                        .color(theme::muted()),
                );
                ui.label(RichText::new(super::issues::truncate(&s.description, 60)).size(12.5));
                if !s.measure.is_empty() {
                    ui.label(
                        RichText::new(super::issues::truncate(&s.measure, 40))
                            .size(11.5)
                            .color(theme::accent()),
                    );
                }
            });
        }
        ui.add_space(6.0);
        if ui.button(t("sv_open_safety")).clicked() {
            go = true;
        }
    });

    if go {
        app.screen = Screen::Safety;
    }
}
