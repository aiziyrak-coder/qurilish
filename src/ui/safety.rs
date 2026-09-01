//! «Mehnat xavfsizligi» ekrani (TZ XV).
//!
//! Hodisalar jurnali: buzilish, xavfli holat, baxtsiz hodisa, tekshiruv va
//! instruktaj. Har yozuvda chora, mas'ul va muddat bor — yopilmagan va muddati
//! o'tgan yozuvlar alohida ajratiladi.
//!
//! TZ III.32: dastur hukm chiqarmaydi — bu yerda fakt qayd etiladi, aybdor
//! belgilanmaydi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{IssueStatus, SafetyEvent, SafetyKind, Severity};

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

    let mut add: Option<SafetyKind> = None;
    ui.horizontal_wrapped(|ui| {
        for k in SafetyKind::ALL {
            if ui.button(format!("+ {}", k.label())).clicked() {
                add = Some(*k);
            }
        }
        ui.label(
            RichText::new(t("safety_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.safety.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("safety_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        table(ui, app);
    }

    if let Some(kind) = add {
        app.db.insert_safety(&SafetyEvent {
            id: 0,
            project_id: pid,
            date: app.today,
            kind,
            // Instruktaj va tekshiruv — ma'lumot, buzilish esa ogohlantirish.
            severity: match kind {
                SafetyKind::Incident => Severity::Critical,
                SafetyKind::Violation | SafetyKind::NearMiss => Severity::Warning,
                _ => Severity::Info,
            },
            place: String::new(),
            description: String::new(),
            responsible: String::new(),
            measure: String::new(),
            deadline: None,
            status: IssueStatus::Open,
        });
        app.reload_modules();
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let is_event = |k: SafetyKind| {
        matches!(
            k,
            SafetyKind::Violation | SafetyKind::NearMiss | SafetyKind::Incident
        )
    };
    let events = app.safety.iter().filter(|s| is_event(s.kind)).count();
    let incidents = app
        .safety
        .iter()
        .filter(|s| s.kind == SafetyKind::Incident)
        .count();
    let open = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    let overdue = app
        .safety
        .iter()
        .filter(|s| {
            matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                && s.deadline.is_some_and(|d| d < app.today)
        })
        .count();
    let trainings = app
        .safety
        .iter()
        .filter(|s| s.kind == SafetyKind::Training)
        .count();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_safety_events"),
                events.to_string(),
                t("kpi_safety_events_hint"),
                if events == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("kpi_safety_incidents"),
                incidents.to_string(),
                t("kpi_safety_incidents_hint"),
                if incidents == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_safety_open"),
                open.to_string(),
                t("kpi_safety_open_hint"),
                if open == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("kpi_safety_overdue"),
                overdue.to_string(),
                t("kpi_safety_overdue_hint"),
                if overdue == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_safety_training"),
                trainings.to_string(),
                t("kpi_safety_training_hint"),
                theme::accent(),
            ),
        ],
    );
}

fn severity_color(s: Severity) -> Color32 {
    match s {
        Severity::Critical => theme::danger(),
        Severity::Major => theme::warn(),
        Severity::Warning => theme::warn(),
        Severity::Info => theme::muted(),
        Severity::Ok => theme::ok(),
    }
}

fn status_color(s: IssueStatus) -> Color32 {
    match s {
        IssueStatus::Open => theme::warn(),
        IssueStatus::InWork => theme::accent(),
        IssueStatus::Fixed => theme::ok(),
        IssueStatus::Rejected => theme::muted(),
    }
}

fn table(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<SafetyEvent> = None;
    let mut removed: Option<i64> = None;
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("safety_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 150.0, t("col_kind"));
                    head_l(ui, 120.0, t("col_severity"));
                    head_l(ui, 120.0, t("col_status"));
                    head_l(ui, 150.0, t("col_place"));
                    head_l(ui, 240.0, t("col_description"));
                    head_l(ui, 220.0, t("col_measure"));
                    head_l(ui, 140.0, t("col_responsible"));
                    head_l(ui, 130.0, t("col_fix_deadline"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.safety {
                        let mut s = src.clone();
                        let mut changed = false;

                        changed |=
                            super::passport::date_edit(ui, &format!("se{}", s.id), &mut s.date);
                        egui::ComboBox::from_id_salt(("se_kind", s.id))
                            .selected_text(s.kind.label())
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for k in SafetyKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut s.kind, *k, k.label()).changed();
                                }
                            });
                        egui::ComboBox::from_id_salt(("se_sev", s.id))
                            .selected_text(
                                RichText::new(s.severity.label()).color(severity_color(s.severity)),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for v in Severity::ALL {
                                    changed |= ui
                                        .selectable_value(&mut s.severity, *v, v.label())
                                        .changed();
                                }
                            });
                        egui::ComboBox::from_id_salt(("se_st", s.id))
                            .selected_text(
                                RichText::new(s.status.label()).color(status_color(s.status)),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for v in IssueStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut s.status, *v, v.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut s.place))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [240.0, 22.0],
                                egui::TextEdit::singleline(&mut s.description),
                            )
                            .changed();
                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut s.measure))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [140.0, 22.0],
                                egui::TextEdit::singleline(&mut s.responsible),
                            )
                            .changed();

                        ui.horizontal(|ui| {
                            let mut has = s.deadline.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                s.deadline = has.then(|| today + chrono::Duration::days(3));
                                changed = true;
                            }
                            if let Some(mut d) = s.deadline {
                                if super::passport::date_edit(ui, &format!("sed{}", s.id), &mut d) {
                                    s.deadline = Some(d);
                                    changed = true;
                                }
                                let open =
                                    matches!(s.status, IssueStatus::Open | IssueStatus::InWork);
                                if open && d < today {
                                    ui.label(RichText::new("!").color(theme::danger()).strong());
                                }
                            }
                        });

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(s.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(s);
                        }
                    }
                });
        });

    if let Some(s) = edited {
        app.db.update_safety(&s);
        if let Some(slot) = app.safety.iter_mut().find(|x| x.id == s.id) {
            *slot = s;
        }
    }
    if let Some(id) = removed {
        app.db.del("safety_event", id);
        app.reload_modules();
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

#[allow(dead_code)]
fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
