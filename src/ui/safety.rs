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

    // Modul yordamchisi (TZ: har modul uchun AI-yordamchi).
    ui.horizontal(|ui| {
        super::assistant_button(ui, app);
    });
    ui.add_space(6.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    let tab_key = egui::Id::new("sf_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("sf_tab_events")),
            (1, t("sf_tab_permits")),
            (2, t("sf_tab_ppe")),
            (3, t("sf_tab_work_permits")),
            (4, t("sf_tab_zones")),
            (5, t("sf_tab_analysis")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => permits_tab(ui, app, pid),
        2 => ppe_tab(ui, app, pid),
        3 => work_permits_tab(ui, app, pid),
        4 => zones_tab(ui, app, pid),
        5 => analysis_tab(ui, app),
        _ => {
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
        }
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
            root_cause: crate::domain::RootCause::Unknown,
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
    let open = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    let trainings = app
        .safety
        .iter()
        .filter(|s| s.kind == SafetyKind::Training)
        .count();

    let safety = app.worker_safety();
    let score = crate::checks::safety_score(&app.safety, &safety, &app.work_permits, app.today);
    // Hodisa va muddat ko'rsatkichlari ham shu hisobdan — ekranda va ballda
    // bir xil son turishi kerak.
    let incidents = score.incidents;
    let overdue = score.overdue;

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_safety_score"),
                format!("{:.0}", score.score),
                // Ball qanday chiqqani ko'rinsin — sirli son bo'lib qolmasin.
                &format!(
                    "{} {} · {} {}",
                    score.expired_permits,
                    t("kpi_score_permits"),
                    score.without_ppe,
                    t("kpi_score_ppe")
                ),
                if score.score >= 85.0 {
                    theme::ok()
                } else if score.score >= 60.0 {
                    theme::warn()
                } else {
                    theme::danger()
                },
            ),
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
                // Buzilish va near miss ham shu yerda ko'rinsin.
                &format!(
                    "{} {} · {} {}",
                    score.violations,
                    t("kpi_score_violations"),
                    score.near_misses,
                    t("kpi_score_near_miss")
                ),
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
                &format!(
                    "{} · {} {}",
                    t("kpi_safety_overdue_hint"),
                    score.bad_permits,
                    t("kpi_score_bad_permits")
                ),
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

// ================================================== Ruxsatlar matritsasi

/// Ishchi × ruxsat turi matritsasi (TZ XV.4–5).
fn permits_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    use crate::domain::{PermitKind, WorkerPermit};

    if app.workers.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("permits_no_workers"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    ui.label(
        RichText::new(t("permits_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    let today = app.today;
    let safety = app.worker_safety();
    // (ishchi, tur) — yangi ruxsat ochiladi yoki muddati uzaytiriladi.
    let mut issue: Option<(i64, PermitKind)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("sf_permits")
                .num_columns(PermitKind::ALL.len() + 2)
                .spacing([6.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 200.0, t("col_worker"));
                    for k in PermitKind::ALL {
                        // Sarlavha tor: qisqa belgisi va to'liq nomi ustida.
                        ui.allocate_ui_with_layout(
                            egui::vec2(78.0, 20.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(78.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(k.label()).size(10.5).color(theme::muted()),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(k.label());
                            },
                        );
                    }
                    head_l(ui, 150.0, t("col_status"));
                    ui.end_row();

                    for w in &app.workers {
                        let st = safety.iter().find(|s| s.worker_id == w.id);
                        cell_l(
                            ui,
                            200.0,
                            RichText::new(super::issues::truncate(&w.name, 28)).size(12.0),
                        );
                        for k in PermitKind::ALL {
                            let permit = app
                                .worker_permits
                                .iter()
                                .filter(|p| p.worker_id == w.id && p.kind == *k)
                                .max_by_key(|p| p.valid_until);
                            let (text, color) = match permit {
                                None => (t("dash").to_string(), theme::muted()),
                                Some(p) if p.expired(today) => (
                                    p.valid_until.format("%d.%m.%y").to_string(),
                                    theme::danger(),
                                ),
                                Some(p)
                                    if p.expires_soon(today, crate::checks::PERMIT_WARN_DAYS) =>
                                {
                                    (p.valid_until.format("%d.%m.%y").to_string(), theme::warn())
                                }
                                Some(p) => {
                                    (p.valid_until.format("%d.%m.%y").to_string(), theme::ok())
                                }
                            };
                            if ui
                                .add_sized(
                                    [78.0, 22.0],
                                    egui::Button::new(RichText::new(text).size(10.5).color(color))
                                        .frame(permit.is_some()),
                                )
                                .on_hover_text(match permit {
                                    Some(p) if !p.number.is_empty() => {
                                        format!("{} · {}", p.number, t("permit_extend"))
                                    }
                                    Some(_) => t("permit_extend").to_string(),
                                    None => t("permit_issue").to_string(),
                                })
                                .clicked()
                            {
                                issue = Some((w.id, *k));
                            }
                        }
                        // Umumiy xulosa: ishga qo'yish mumkinmi.
                        let (text, color) = match st {
                            Some(s) if s.blocked() => (t("worker_blocked"), theme::danger()),
                            Some(s) if !s.expired.is_empty() => {
                                (t("worker_expired"), theme::warn())
                            }
                            Some(s) if !s.expiring.is_empty() => {
                                (t("worker_expiring"), theme::warn())
                            }
                            _ => (t("worker_ok"), theme::ok()),
                        };
                        cell_l(ui, 150.0, RichText::new(text).size(11.0).color(color));
                        ui.end_row();
                    }
                });
        });

    if let Some((worker_id, kind)) = issue {
        // Bosilganda ruxsat bir yilga ochiladi yoki uzaytiriladi. Aniq sana
        // va raqam keyin tahrirlanadi — bu yerda tez kiritish muhim.
        let existing = app
            .worker_permits
            .iter()
            .filter(|p| p.worker_id == worker_id && p.kind == kind)
            .max_by_key(|p| p.valid_until)
            .cloned();
        match existing {
            Some(mut p) => {
                p.issued = today;
                p.valid_until = today + chrono::Duration::days(365);
                app.db.update_worker_permit(&p);
            }
            None => {
                app.db.insert_worker_permit(&WorkerPermit {
                    id: 0,
                    project_id: pid,
                    worker_id,
                    kind,
                    number: String::new(),
                    issued: today,
                    valid_until: today + chrono::Duration::days(365),
                    note: String::new(),
                });
            }
        }
        app.reload_modules();
    }
}

// ================================================================ SIZ

/// Shaxsiy himoya vositalari (TZ XV.8–9).
fn ppe_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    use crate::domain::{PpeIssue, PpeItem};

    if app.workers.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("permits_no_workers"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    ui.label(
        RichText::new(t("ppe_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    let today = app.today;
    let mut issue: Option<(i64, PpeItem)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("sf_ppe")
                .num_columns(PpeItem::ALL.len() + 2)
                .spacing([6.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 200.0, t("col_worker"));
                    for item in PpeItem::ALL {
                        let required = PpeItem::REQUIRED.contains(item);
                        ui.allocate_ui_with_layout(
                            egui::vec2(78.0, 20.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(78.0);
                                ui.add(
                                    egui::Label::new(RichText::new(item.label()).size(10.5).color(
                                        if required {
                                            theme::text()
                                        } else {
                                            theme::muted()
                                        },
                                    ))
                                    .truncate(),
                                )
                                .on_hover_text(if required {
                                    t("ppe_required")
                                } else {
                                    t("ppe_by_work")
                                });
                            },
                        );
                    }
                    head_l(ui, 130.0, t("col_status"));
                    ui.end_row();

                    for w in &app.workers {
                        cell_l(
                            ui,
                            200.0,
                            RichText::new(super::issues::truncate(&w.name, 28)).size(12.0),
                        );
                        let mut missing = 0usize;
                        for item in PpeItem::ALL {
                            let last = app
                                .ppe_issues
                                .iter()
                                .filter(|p| p.worker_id == w.id && p.item == *item)
                                .max_by_key(|p| p.issued);
                            let required = PpeItem::REQUIRED.contains(item);
                            let (text, color) = match last {
                                None if required => {
                                    missing += 1;
                                    (t("ppe_none").to_string(), theme::danger())
                                }
                                None => (t("dash").to_string(), theme::muted()),
                                Some(p) if p.expired(today) => {
                                    if required {
                                        missing += 1;
                                    }
                                    (
                                        p.until()
                                            .map(|d| d.format("%d.%m.%y").to_string())
                                            .unwrap_or_default(),
                                        theme::danger(),
                                    )
                                }
                                Some(p) => (
                                    p.until()
                                        .map(|d| d.format("%d.%m.%y").to_string())
                                        .unwrap_or_else(|| t("ppe_no_limit").to_string()),
                                    theme::ok(),
                                ),
                            };
                            if ui
                                .add_sized(
                                    [78.0, 22.0],
                                    egui::Button::new(RichText::new(text).size(10.5).color(color))
                                        .frame(last.is_some()),
                                )
                                .on_hover_text(t("ppe_issue_hint"))
                                .clicked()
                            {
                                issue = Some((w.id, *item));
                            }
                        }
                        cell_l(
                            ui,
                            130.0,
                            RichText::new(if missing == 0 {
                                t("worker_ok").to_string()
                            } else {
                                format!("{} {missing}", t("ppe_missing"))
                            })
                            .size(11.0)
                            .color(if missing == 0 {
                                theme::ok()
                            } else {
                                theme::danger()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });

    if let Some((worker_id, item)) = issue {
        app.db.insert_ppe_issue(&PpeIssue {
            id: 0,
            project_id: pid,
            worker_id,
            item,
            issued: today,
            months: item.months(),
            note: String::new(),
        });
        app.reload_modules();
    }
}

// ================================================== Naryad-dopusk

/// Yuqori xavfli ishga ruxsat (TZ XV.10–12).
fn work_permits_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    use crate::domain::{PermitKind, PermitStatus, WorkPermit};

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_work_permit")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("work_permits_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.work_permits.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("work_permits_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        let today = app.today;
        let safety = app.worker_safety();
        let mut edited: Option<WorkPermit> = None;
        let mut removed: Option<i64> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("sf_work_permits")
                    .num_columns(12)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 90.0, t("col_number"));
                        head_l(ui, 130.0, t("col_kind"));
                        head_l(ui, 190.0, t("col_task"));
                        head_l(ui, 150.0, t("col_place"));
                        head_l(ui, 110.0, t("col_from"));
                        head_l(ui, 110.0, t("col_to"));
                        head_l(ui, 140.0, t("col_issuer"));
                        head_l(ui, 140.0, t("col_supervisor"));
                        head_l(ui, 200.0, t("col_executors"));
                        head_l(ui, 120.0, t("col_status"));
                        head_l(ui, 240.0, t("col_permit_issues"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.work_permits {
                            let mut p = src.clone();
                            let mut changed = false;

                            changed |= ui
                                .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut p.number))
                                .changed();
                            egui::ComboBox::from_id_salt(("wp_kind", p.id))
                                .selected_text(p.kind.label())
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    for k in PermitKind::ALL {
                                        changed |= ui
                                            .selectable_value(&mut p.kind, *k, k.label())
                                            .changed();
                                    }
                                });
                            changed |=
                                task_picker(ui, app, ("wp_task", p.id), &mut p.task_id, 190.0);
                            changed |= ui
                                .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut p.place))
                                .changed();
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("wpf{}", p.id),
                                &mut p.date_from,
                            );
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("wpt{}", p.id),
                                &mut p.date_to,
                            );
                            changed |= ui
                                .add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut p.issuer))
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [140.0, 22.0],
                                    egui::TextEdit::singleline(&mut p.supervisor),
                                )
                                .changed();

                            // Bajaruvchilar — ro'yxatdan belgilanadi.
                            let mut ids = p.worker_ids();
                            let label = if ids.is_empty() {
                                t("workers_none").to_string()
                            } else {
                                format!("{} {}", ids.len(), t("workers_count"))
                            };
                            egui::ComboBox::from_id_salt(("wp_w", p.id))
                                .selected_text(label)
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    for w in &app.workers {
                                        let mut on = ids.contains(&w.id);
                                        if ui.checkbox(&mut on, &w.name).changed() {
                                            if on {
                                                ids.push(w.id);
                                            } else {
                                                ids.retain(|x| *x != w.id);
                                            }
                                            p.set_workers(&ids);
                                            changed = true;
                                        }
                                    }
                                });

                            egui::ComboBox::from_id_salt(("wp_st", p.id))
                                .selected_text(
                                    RichText::new(p.status.label())
                                        .color(permit_status_color(p.status)),
                                )
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for st in PermitStatus::ALL {
                                        changed |= ui
                                            .selectable_value(&mut p.status, *st, st.label())
                                            .changed();
                                    }
                                });

                            // Tekshiruv natijasi: nima yetishmaydi.
                            let issues = crate::checks::permit_issues(&p, &safety, today);
                            let text = if issues.is_empty() {
                                t("permit_ok").to_string()
                            } else {
                                issues
                                    .iter()
                                    .map(|i| issue_label(app, i))
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            };
                            cell_l(
                                ui,
                                240.0,
                                RichText::new(super::issues::truncate(&text, 40))
                                    .size(11.0)
                                    .color(if issues.is_empty() {
                                        theme::ok()
                                    } else {
                                        theme::danger()
                                    }),
                            )
                            .on_hover_text(text);

                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(p.id);
                            }
                            ui.end_row();
                            if changed {
                                edited = Some(p);
                            }
                        }
                    });
            });

        if let Some(p) = edited {
            app.db.update_work_permit(&p);
            app.reload_modules();
        }
        if let Some(id) = removed {
            app.db.del("work_permit", id);
            app.reload_modules();
        }
    }

    if add {
        let n = app.work_permits.len() + 1;
        app.db.insert_work_permit(&WorkPermit {
            id: 0,
            project_id: pid,
            number: format!("ND-{n:03}"),
            kind: PermitKind::Height,
            task_id: None,
            place: String::new(),
            date_from: app.today,
            date_to: app.today + chrono::Duration::days(5),
            issuer: String::new(),
            supervisor: String::new(),
            workers: String::new(),
            measures: String::new(),
            status: PermitStatus::Draft,
            note: String::new(),
        });
        app.reload_modules();
    }
}

fn permit_status_color(s: crate::domain::PermitStatus) -> Color32 {
    use crate::domain::PermitStatus;
    match s {
        PermitStatus::Draft => theme::muted(),
        PermitStatus::Open => theme::accent(),
        PermitStatus::Closed => theme::ok(),
        PermitStatus::Stopped => theme::danger(),
    }
}

/// Naryaddagi kamchilikning o'qiladigan matni.
fn issue_label(app: &App, i: &crate::checks::PermitIssue) -> String {
    use crate::checks::PermitIssue;
    let who = |id: i64| {
        app.workers
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.name.clone())
            .unwrap_or_default()
    };
    match i {
        PermitIssue::NoWorkers => t("pi_no_workers").to_string(),
        PermitIssue::NoMeasures => t("pi_no_measures").to_string(),
        PermitIssue::NoIssuer => t("pi_no_issuer").to_string(),
        PermitIssue::BadPeriod => t("pi_bad_period").to_string(),
        PermitIssue::Overdue => t("pi_overdue").to_string(),
        PermitIssue::WorkerNotAllowed(id) => format!("{}: {}", who(*id), t("pi_not_allowed")),
        PermitIssue::WorkerNoPpe(id) => format!("{}: {}", who(*id), t("pi_no_ppe")),
    }
}

// ================================================================ Zonalar va inventar

/// Xavfli zonalar va xavfsizlik inventari (TZ XV.15, 22-24).
///
/// Bitta ro'yxatda: xavfli zona ham, yong'in o'chirgich ham, evakuatsiya
/// belgisi ham. Ularning hammasida bir xil savol — **joyida turibdimi va
/// muddati o'tmaganmi**.
fn zones_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Safety);
    let today = app.today;

    let mut add: Option<crate::domain::ZoneKind> = None;
    ui.horizontal_wrapped(|ui| {
        if can {
            for k in crate::domain::ZoneKind::ALL {
                if ui.button(format!("+ {}", k.label())).clicked() {
                    add = Some(*k);
                }
            }
        }
        ui.label(
            RichText::new(t("sf_zones_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    let attention = app.zones.iter().filter(|z| z.needs_action(today)).count();
    let overdue = app.zones.iter().filter(|z| z.overdue(today)).count();
    stat_row(
        ui,
        vec![
            stat(
                t("sf_zones_total"),
                app.zones.len().to_string(),
                t("sf_zones_total_hint"),
                theme::accent(),
            ),
            stat(
                t("sf_zones_attention"),
                attention.to_string(),
                t("sf_zones_attention_hint"),
                if attention == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("sf_zones_overdue"),
                overdue.to_string(),
                t("sf_zones_overdue_hint"),
                if overdue == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(10.0);

    if app.zones.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("sf_zones_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        let mut edited: Option<crate::domain::SafetyZone> = None;
        let mut removed: Option<i64> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("sf_zones_grid")
                    .num_columns(9)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 150.0, t("col_kind"));
                        head_l(ui, 210.0, t("col_name"));
                        head_l(ui, 170.0, t("col_place"));
                        head_l(ui, 230.0, t("col_measure"));
                        head_l(ui, 140.0, t("col_responsible"));
                        head_l(ui, 140.0, t("sf_zone_check"));
                        head_l(ui, 90.0, t("sf_zone_ready"));
                        head_l(ui, 140.0, t("col_status"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.zones {
                            let mut z = src.clone();
                            let mut changed = false;

                            egui::ComboBox::from_id_salt(("sz_k", z.id))
                                .selected_text(z.kind.label())
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for k in crate::domain::ZoneKind::ALL {
                                        changed |= ui
                                            .selectable_value(&mut z.kind, *k, k.label())
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut z.name))
                                .changed();
                            changed |= ui
                                .add_sized([170.0, 22.0], egui::TextEdit::singleline(&mut z.place))
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [230.0, 22.0],
                                    egui::TextEdit::singleline(&mut z.measure),
                                )
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [140.0, 22.0],
                                    egui::TextEdit::singleline(&mut z.responsible),
                                )
                                .changed();

                            ui.horizontal(|ui| {
                                let mut has = z.check_due.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    z.check_due = has.then(|| today + chrono::Duration::days(30));
                                    changed = true;
                                }
                                if let Some(mut d) = z.check_due {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("szc{}", z.id),
                                        &mut d,
                                    ) {
                                        z.check_due = Some(d);
                                        changed = true;
                                    }
                                    if d < today {
                                        ui.label(
                                            RichText::new("!").color(theme::danger()).strong(),
                                        );
                                    }
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                if ui.checkbox(&mut z.ready, "").changed() {
                                    // Chora ko'rilgan deb belgilangan kun ham
                                    // yoziladi: keyin «qachon?» degan savol
                                    // javobsiz qolmasin.
                                    z.checked_at = z.ready.then_some(today);
                                    changed = true;
                                }
                            });

                            cell_l(
                                ui,
                                140.0,
                                if !z.ready {
                                    RichText::new(t("sf_zone_not_ready"))
                                        .size(11.5)
                                        .color(theme::danger())
                                } else if z.overdue(today) {
                                    RichText::new(t("sf_zone_check_due"))
                                        .size(11.5)
                                        .color(theme::warn())
                                } else {
                                    RichText::new(t("sf_zone_ok")).size(11.5).color(theme::ok())
                                },
                            );

                            if can
                                && ui
                                    .small_button(RichText::new("x").color(theme::danger()))
                                    .clicked()
                            {
                                removed = Some(z.id);
                            }
                            ui.end_row();

                            if changed && can {
                                edited = Some(z);
                            }
                        }
                    });
            });

        if let Some(z) = edited {
            app.db.update_safety_zone(&z);
            if let Some(slot) = app.zones.iter_mut().find(|y| y.id == z.id) {
                *slot = z;
            }
        }
        if let Some(id) = removed {
            app.db.delete_safety_zone(id);
            app.reload_modules();
        }
    }

    if let Some(kind) = add {
        app.db.insert_safety_zone(&crate::domain::SafetyZone {
            id: 0,
            project_id: pid,
            kind,
            name: String::new(),
            place: String::new(),
            measure: String::new(),
            responsible: String::new(),
            check_due: Some(today + chrono::Duration::days(30)),
            checked_at: None,
            ready: false,
            note: String::new(),
        });
        app.reload_modules();
    }
}

// ================================================================ Tahlil

/// Sabab tahlili, ogohlantirishlar va mas'ullar (TZ XV.28, 35-36).
fn analysis_tab(ui: &mut egui::Ui, app: &mut App) {
    let causes = crate::checks::root_causes(&app.safety);
    let risks = app.safety_risks();
    let rating = crate::checks::safety_rating(&app.safety, app.today);

    ui.label(
        RichText::new(t("sf_analysis_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Ogohlantirishlar ----------
            ui.label(RichText::new(t("sf_risks_title")).size(13.5).strong());
            ui.add_space(4.0);
            if risks.is_empty() {
                ui.label(
                    RichText::new(t("sf_risks_none"))
                        .size(12.0)
                        .color(theme::ok()),
                );
            }
            for r in &risks {
                let (text, color) = risk_line(app, r);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 1.5, color);
                    ui.add_space(6.0);
                    ui.label(RichText::new(text).size(12.0).color(color));
                });
                ui.add_space(3.0);
            }

            // ---------- Ildiz sabablar ----------
            if !causes.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(t("sf_causes_title")).size(13.5).strong());
                ui.label(
                    RichText::new(t("sf_causes_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                egui::Grid::new("sf_causes_grid")
                    .num_columns(4)
                    .spacing([12.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 240.0, t("sf_cause"));
                        head_r(ui, 90.0, t("sf_cause_count"));
                        head_r(ui, 110.0, t("sf_cause_serious"));
                        head_r(ui, 90.0, t("sf_cause_pct"));
                        ui.end_row();
                        for c in &causes {
                            let repeated = c.count >= crate::checks::CAUSE_REPEAT_LIMIT
                                && c.cause != crate::domain::RootCause::Unknown;
                            cell_l(
                                ui,
                                240.0,
                                RichText::new(c.cause.label())
                                    .size(12.5)
                                    .color(if repeated {
                                        theme::danger()
                                    } else {
                                        theme::text()
                                    }),
                            );
                            cell_r(ui, 90.0, RichText::new(c.count.to_string()).size(12.0));
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(c.serious.to_string()).size(12.0).color(
                                    if c.serious == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_r(ui, 90.0, RichText::new(format!("{:.0}%", c.pct)).size(12.0));
                            ui.end_row();
                        }
                    });
            }

            // ---------- Mas'ullar ----------
            if !rating.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(t("sf_rating_title")).size(13.5).strong());
                ui.add_space(6.0);
                egui::Grid::new("sf_rating_grid")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 200.0, t("col_responsible"));
                        head_r(ui, 90.0, t("sf_r_events"));
                        head_r(ui, 110.0, t("sf_r_violations"));
                        head_r(ui, 110.0, t("sf_r_incidents"));
                        head_r(ui, 90.0, t("sf_r_open"));
                        head_r(ui, 110.0, t("sf_r_overdue"));
                        ui.end_row();
                        for r in &rating {
                            cell_l(ui, 200.0, RichText::new(&r.name).size(12.5));
                            cell_r(ui, 90.0, RichText::new(r.events.to_string()).size(12.0));
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(r.violations.to_string()).size(12.0),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(r.incidents.to_string()).size(12.0).color(
                                    if r.incidents == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_r(ui, 90.0, RichText::new(r.open.to_string()).size(12.0));
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(r.overdue.to_string()).size(12.0).color(
                                    if r.overdue == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(14.0);
        });
}

/// Ogohlantirish matni va rangi.
fn risk_line(app: &App, r: &crate::checks::SafetyRisk) -> (String, egui::Color32) {
    use crate::checks::SafetyRisk as R;
    let zone = |id: i64| {
        app.zones
            .iter()
            .find(|z| z.id == id)
            .map(|z| {
                if z.name.trim().is_empty() {
                    z.kind.label().to_string()
                } else {
                    z.name.clone()
                }
            })
            .unwrap_or_default()
    };
    match r {
        R::ZoneNotReady { zone_id } => (
            format!("{} — {}", zone(*zone_id), t("sf_risk_not_ready")),
            theme::danger(),
        ),
        R::ZoneOverdue { zone_id, days } => (
            format!(
                "{} — {} {} {}",
                zone(*zone_id),
                t("sf_risk_overdue"),
                days,
                t("sf_risk_days")
            ),
            theme::warn(),
        ),
        R::RepeatedCause { cause, count } => (
            format!("{} — {count} {}", cause.label(), t("sf_risk_repeated")),
            theme::danger(),
        ),
        R::BlockedWorkers { count } => {
            (format!("{count} {}", t("sf_risk_blocked")), theme::danger())
        }
        R::BadPermits { count } => (format!("{count} {}", t("sf_risk_permits")), theme::warn()),
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

#[allow(dead_code)]
fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
