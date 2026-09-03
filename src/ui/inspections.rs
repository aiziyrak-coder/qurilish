//! «Tekshiruvlar» ekrani (TZ VII.3-6, 12-14).
//!
//! Texnik nazoratning uch xil ishi bitta ekranda: qaysi tekshiruv qachonga
//! rejalashtirilgan (kalendar), beton namunalari nima ko'rsatdi va geodeziya
//! dopuskdan chiqdimi.
//!
//! Muhim qoida: **mustahkamlik ham, o'lchov ham hisoblanmaydi**. Laboratoriya
//! va geodezist bergan sonlar kiritiladi, ilova faqat solishtiradi. Shuning
//! uchun natijasi kelmagan namunada hukm ham yo'q — bo'sh katak turadi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::checks;
use crate::domain::{
    ConcreteTest, GeodesyPoint, Inspection, InspectionKind, InspectionResult, QualityResult,
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

    let tab_key = egui::Id::new("in_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("in_tab_list")),
            (1, t("in_tab_calendar")),
            (2, t("in_tab_concrete")),
            (3, t("in_tab_geodesy")),
            (4, t("in_tab_day")),
            (5, t("in_tab_final")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => calendar_tab(ui, app),
        2 => concrete_tab(ui, app, pid),
        3 => geodesy_tab(ui, app, pid),
        4 => day_tab(ui, app),
        5 => final_tab(ui, app),
        _ => list_tab(ui, app, pid),
    }
}

// ================================================================ Yakuniy qabul

/// Obyekt yakuniy qabulga tayyormi (TZ VII.35-36).
///
/// Tayyorlik yetti shart bo'yicha o'lchanadi va har biri o'z modulidagi
/// yozuvdan olinadi — bu alohida hisob-kitob emas, mavjud holatlarning
/// yig'indisi.
fn final_tab(ui: &mut egui::Ui, app: &mut App) {
    let r = app.final_readiness();
    let mut go: Option<Screen> = None;

    ui.label(
        RichText::new(t("in_final_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("in_final_ready"),
                format!("{:.0}%", r.ready_pct),
                t("in_final_ready_hint"),
                if r.ready() {
                    theme::ok()
                } else if r.ready_pct >= 70.0 {
                    theme::warn()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("in_final_blocks"),
                r.blocks.len().to_string(),
                &format!("{} {}", r.total_checks, t("in_final_of")),
                if r.blocks.is_empty() {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(14.0);

    if r.ready() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(
                RichText::new(t("in_final_all_clear"))
                    .color(theme::ok())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for b in &r.blocks {
                egui::Frame::new()
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(b.count().to_string())
                                    .size(15.0)
                                    .strong()
                                    .color(theme::danger()),
                            );
                            ui.label(RichText::new(final_text(b)).size(12.5));
                            if ui.small_button(t("an_open")).clicked() {
                                go = Some(final_screen(b));
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });

    if let Some(s) = go {
        app.screen = s;
    }
}

/// To'siqni odam o'qiydigan gapga aylantiradi.
fn final_text(b: &crate::checks::FinalBlock) -> &'static str {
    use crate::checks::FinalBlock as B;
    match b {
        B::TasksOpen { .. } => t("fb_tasks"),
        B::DocsUnsigned { .. } => t("fb_docs"),
        B::DefectsOpen { .. } => t("fb_defects"),
        B::LabFailed { .. } => t("fb_lab"),
        B::InspectionsOpen { .. } => t("fb_inspections"),
        B::SafetyOpen { .. } => t("fb_safety"),
        B::AcceptancePending { .. } => t("fb_acceptance"),
    }
}

/// To'siq qaysi ekranda yopiladi.
fn final_screen(b: &crate::checks::FinalBlock) -> Screen {
    use crate::checks::FinalBlock as B;
    match b {
        B::TasksOpen { .. } => Screen::Gantt,
        B::DocsUnsigned { .. } => Screen::ExecDocs,
        B::DefectsOpen { .. } | B::LabFailed { .. } => Screen::Quality,
        B::InspectionsOpen { .. } => Screen::Inspections,
        B::SafetyOpen { .. } => Screen::Safety,
        B::AcceptancePending { .. } => Screen::Contracts,
    }
}

/// Tekshiruvga chiqishdan oldingi chek-list (TZ VII.26).
///
/// Ro'yxat qisqa: uzun ro'yxat o'qilmaydi, o'qilmagan ro'yxat esa foydasiz.
fn checklist_popup(ui: &mut egui::Ui, app: &App, inspection: &Inspection) {
    let section = inspection
        .task_id
        .and_then(|id| app.task(id))
        .map(|t| t.section)
        .unwrap_or(crate::model::Section::None);
    let items = crate::checks::inspection_checklist(inspection.kind, section);

    ui.label(RichText::new(t("in_checklist")).size(12.5).strong());
    ui.add_space(4.0);
    for i in &items {
        ui.label(
            RichText::new(format!(
                "{} {}",
                if i.required { "•" } else { "◦" },
                t(i.key)
            ))
            .size(12.0)
            .color(if i.required {
                theme::text()
            } else {
                theme::muted()
            }),
        );
    }
}

// ================================================================ Ko'rsatkichlar

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let s = checks::inspection_summary(&app.inspections, app.today);
    let c = checks::concrete_summary(&app.concrete_tests, app.today);
    let out = checks::geodesy_issues(&app.geodesy_points).len();

    stat_row(
        ui,
        vec![
            stat(
                t("in_kpi_today"),
                s.today.to_string(),
                t("in_kpi_today_hint"),
                if s.today == 0 {
                    theme::muted()
                } else {
                    theme::accent()
                },
            ),
            stat(
                t("in_kpi_week"),
                s.week.to_string(),
                t("in_kpi_week_hint"),
                theme::text(),
            ),
            stat(
                t("in_kpi_overdue"),
                s.overdue.to_string(),
                t("in_kpi_overdue_hint"),
                if s.overdue == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("in_kpi_defects"),
                s.open_defects.to_string(),
                t("in_kpi_defects_hint"),
                if s.open_defects == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("in_kpi_concrete"),
                if c.tested == 0 {
                    t("dash").to_string()
                } else {
                    format!("{:.0} %", c.avg_pct)
                },
                &format!(
                    "{} {} · {} {}",
                    c.tested,
                    t("in_tested"),
                    c.failed,
                    t("in_failed")
                ),
                if c.failed == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("in_kpi_geodesy"),
                out.to_string(),
                t("in_kpi_geodesy_hint"),
                if out == 0 { theme::ok() } else { theme::warn() },
            ),
        ],
    );
}

// ================================================================ Tekshiruvlar ro'yxati

fn list_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Inspections);

    ui.horizontal_wrapped(|ui| {
        if can {
            for k in InspectionKind::ALL {
                if ui.button(format!("+ {}", k.label())).clicked() {
                    app.db.insert_inspection(&Inspection {
                        id: 0,
                        project_id: pid,
                        task_id: None,
                        kind: *k,
                        number: String::new(),
                        planned: app.today,
                        done: None,
                        requested_by: String::new(),
                        inspector: String::new(),
                        place: String::new(),
                        result: InspectionResult::Waiting,
                        deadline: None,
                        fixed_at: None,
                        note: String::new(),
                    });
                    app.reload_modules();
                }
            }
        }
        ui.label(RichText::new(t("in_hint")).size(11.0).color(theme::muted()));
    });
    ui.add_space(8.0);

    if app.inspections.is_empty() {
        empty(ui, t("in_empty"));
        return;
    }

    let today = app.today;
    let wide = ui.available_width() > 1400.0;
    let mut edited: Option<Inspection> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("in_grid")
                .num_columns(if wide { 11 } else { 9 })
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 90.0, t("col_number"));
                    head_l(ui, 130.0, t("col_kind"));
                    head_l(ui, 104.0, t("in_planned"));
                    head_l(ui, 120.0, t("in_done"));
                    head_l(ui, 230.0, t("col_place"));
                    head_l(ui, 130.0, t("col_result"));
                    if wide {
                        head_l(ui, 130.0, t("in_requested_by"));
                    }
                    head_l(ui, 130.0, t("in_inspector"));
                    if wide {
                        head_l(ui, 130.0, t("col_task"));
                    }
                    head_l(ui, 130.0, t("in_fix"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.inspections {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut x.number))
                            .changed();
                        egui::ComboBox::from_id_salt(("in_kind", x.id))
                            .selected_text(x.kind.label())
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for k in InspectionKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut x.kind, *k, k.label()).changed();
                                }
                            });
                        changed |=
                            super::passport::date_edit(ui, &format!("inp{}", x.id), &mut x.planned);

                        // O'tkazilgan sana: belgilangach natija ham talab qilinadi.
                        ui.horizontal(|ui| {
                            let mut has = x.done.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                x.done = has.then_some(today);
                                if !has {
                                    x.result = InspectionResult::Waiting;
                                }
                                changed = true;
                            }
                            if let Some(mut dt) = x.done {
                                if super::passport::date_edit(ui, &format!("ind{}", x.id), &mut dt)
                                {
                                    x.done = Some(dt);
                                    changed = true;
                                }
                            } else if x.planned < today {
                                ui.label(
                                    RichText::new(t("in_overdue"))
                                        .size(10.5)
                                        .color(theme::danger()),
                                );
                            }
                        });

                        changed |= ui
                            .add_sized([230.0, 22.0], egui::TextEdit::singleline(&mut x.place))
                            .changed();

                        // O'tkazilmagan tekshiruvda natija tanlanmaydi.
                        let done = x.done.is_some();
                        ui.add_enabled_ui(done, |ui| {
                            egui::ComboBox::from_id_salt(("in_res", x.id))
                                .selected_text(
                                    RichText::new(x.result.label()).color(result_color(x.result)),
                                )
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    for v in InspectionResult::ALL {
                                        changed |= ui
                                            .selectable_value(&mut x.result, *v, v.label())
                                            .changed();
                                    }
                                });
                        });

                        if wide {
                            changed |= ui
                                .add_sized(
                                    [130.0, 22.0],
                                    egui::TextEdit::singleline(&mut x.requested_by),
                                )
                                .changed();
                        }
                        changed |= ui
                            .add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut x.inspector))
                            .changed();

                        if wide {
                            let label = x
                                .task_id
                                .and_then(|id| app.task(id).map(|t| t.wbs.clone()))
                                .unwrap_or_else(|| t("dash").to_string());
                            egui::ComboBox::from_id_salt(("in_task", x.id))
                                .selected_text(label)
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut x.task_id, None, t("dash"))
                                        .changed();
                                    for tk in &app.tasks {
                                        changed |= ui
                                            .selectable_value(
                                                &mut x.task_id,
                                                Some(tk.id),
                                                format!("{} {}", tk.wbs, tk.name),
                                            )
                                            .changed();
                                    }
                                });
                        }

                        // Bartaraf etish: salbiy natijada muddat va yopilgan sana.
                        ui.horizontal(|ui| {
                            if x.open_defect() {
                                match x.deadline {
                                    Some(mut dl) => {
                                        if super::passport::date_edit(
                                            ui,
                                            &format!("inf{}", x.id),
                                            &mut dl,
                                        ) {
                                            x.deadline = Some(dl);
                                            changed = true;
                                        }
                                        if dl < today {
                                            ui.label(
                                                RichText::new("!").color(theme::danger()).strong(),
                                            );
                                        }
                                    }
                                    None => {
                                        if ui.small_button(t("in_set_deadline")).clicked() {
                                            x.deadline = Some(today + chrono::Duration::days(7));
                                            changed = true;
                                        }
                                    }
                                }
                                if ui.small_button(t("in_close")).clicked() {
                                    x.fixed_at = Some(today);
                                    changed = true;
                                }
                            } else if let Some(f) = x.fixed_at {
                                ui.label(
                                    RichText::new(f.format("%d.%m.%Y").to_string())
                                        .size(11.0)
                                        .color(theme::ok()),
                                );
                            } else {
                                ui.label(RichText::new(t("dash")).color(theme::muted()));
                            }
                        });

                        if can
                            && ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                        {
                            removed = Some(x.id);
                        }
                        // Chek-list tekshiruvga chiqishdan oldin kerak —
                        // shuning uchun u qatorning o'zida turadi (TZ VII.26).
                        ui.label(RichText::new(t("in_checklist_short")).size(11.0))
                            .on_hover_ui(|ui| checklist_popup(ui, app, &x));

                        ui.end_row();

                        if changed && can {
                            edited = Some(x);
                        }
                    }
                });
        });

    if let Some(x) = edited {
        app.db.update_inspection(&x);
        if let Some(slot) = app.inspections.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_inspection(id);
        app.reload_modules();
    }
}

// ================================================================ Kalendar

fn calendar_tab(ui: &mut egui::Ui, app: &mut App) {
    let days = checks::inspection_calendar(&app.inspections, app.today);
    if days.is_empty() {
        empty(ui, t("in_cal_empty"));
        return;
    }

    ui.label(
        RichText::new(t("in_cal_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for d in &days {
                let color = if d.overdue {
                    theme::danger()
                } else if d.date == app.today {
                    theme::accent()
                } else {
                    theme::muted()
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(d.date.format("%d.%m.%Y").to_string())
                            .size(12.5)
                            .strong()
                            .color(color),
                    );
                    let rel = (d.date - app.today).num_days();
                    ui.label(
                        RichText::new(match rel {
                            0 => t("in_cal_today").to_string(),
                            n if n < 0 => format!("{} {}", -n, t("in_cal_days_ago")),
                            n => format!("{} {}", n, t("in_cal_days_left")),
                        })
                        .size(11.0)
                        .color(color),
                    );
                });
                ui.add_space(2.0);
                for id in &d.ids {
                    let Some(x) = app.inspections.iter().find(|y| y.id == *id) else {
                        continue;
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(x.kind.label())
                                .size(11.0)
                                .color(theme::accent()),
                        );
                        ui.label(RichText::new(&x.number).size(11.5).strong());
                        ui.label(RichText::new(&x.place).size(11.5));
                        if !x.requested_by.is_empty() {
                            ui.label(
                                RichText::new(format!("· {}", x.requested_by))
                                    .size(11.0)
                                    .color(theme::muted()),
                            );
                        }
                    });
                }
                ui.add_space(8.0);
            }
        });
}

// ================================================================ Beton

fn concrete_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Inspections);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("in_add_sample")).clicked() {
            app.db.insert_concrete_test(&ConcreteTest {
                id: 0,
                project_id: pid,
                inspection_id: None,
                task_id: None,
                sample: String::new(),
                grade: String::new(),
                structure: String::new(),
                poured: app.today,
                age_days: 28,
                required: 0.0,
                actual: None,
                lab: String::new(),
                note: String::new(),
            });
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("in_concrete_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.concrete_tests.is_empty() {
        empty(ui, t("in_concrete_empty"));
        return;
    }

    let today = app.today;
    let mut edited: Option<ConcreteTest> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("in_conc_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 90.0, t("in_sample"));
                    head_l(ui, 70.0, t("in_grade"));
                    head_l(ui, 200.0, t("in_structure"));
                    head_l(ui, 104.0, t("in_poured"));
                    head_l(ui, 60.0, t("in_age"));
                    head_l(ui, 104.0, t("in_test_date"));
                    head_l(ui, 80.0, t("in_required"));
                    head_l(ui, 110.0, t("in_actual"));
                    head_l(ui, 90.0, t("in_of_required"));
                    head_l(ui, 160.0, t("in_lab"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.concrete_tests {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut x.sample))
                            .changed();
                        changed |= ui
                            .add_sized([70.0, 22.0], egui::TextEdit::singleline(&mut x.grade))
                            .changed();
                        changed |= ui
                            .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut x.structure))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("cp{}", x.id), &mut x.poured);
                        let mut age = x.age_days as f64;
                        if super::materials::num_edit(ui, 60.0, &mut age, 1.0, 365.0) {
                            x.age_days = age.round() as i64;
                            changed = true;
                        }
                        // Sinov sanasi hisoblanadi — qo'lda kiritilmaydi.
                        cell_l(
                            ui,
                            104.0,
                            RichText::new(x.test_date().format("%d.%m.%Y").to_string())
                                .size(11.5)
                                .color(if x.actual.is_none() && x.test_date() <= today {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                }),
                        );
                        changed |=
                            super::materials::num_edit(ui, 80.0, &mut x.required, 0.5, 200.0);

                        // Natija: hali yo'q bo'lsa bo'sh qoladi.
                        ui.horizontal(|ui| {
                            let mut has = x.actual.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                x.actual = has.then_some(x.required);
                                changed = true;
                            }
                            if let Some(mut v) = x.actual {
                                if super::materials::num_edit(ui, 70.0, &mut v, 0.1, 200.0) {
                                    x.actual = Some(v);
                                    changed = true;
                                }
                            }
                        });

                        cell_r(
                            ui,
                            90.0,
                            match (x.pct(), x.passed()) {
                                (Some(p), Some(ok)) => RichText::new(format!("{p:.0} %"))
                                    .size(12.0)
                                    .strong()
                                    .color(if ok { theme::ok() } else { theme::danger() }),
                                _ => RichText::new(t("in_waiting"))
                                    .size(11.0)
                                    .color(theme::muted()),
                            },
                        );
                        changed |= ui
                            .add_sized([160.0, 22.0], egui::TextEdit::singleline(&mut x.lab))
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
        app.db.update_concrete_test(&x);
        if let Some(slot) = app.concrete_tests.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_concrete_test(id);
        app.reload_modules();
    }
}

// ================================================================ Geodeziya

fn geodesy_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Inspections);
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("in_add_point")).clicked() {
            app.db.insert_geodesy_point(&GeodesyPoint {
                id: 0,
                project_id: pid,
                inspection_id: None,
                mark: String::new(),
                axis: String::new(),
                level: String::new(),
                design: 0.0,
                fact: 0.0,
                tolerance: 10.0,
                unit: "mm".into(),
                measured: app.today,
                surveyor: String::new(),
                note: String::new(),
            });
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("in_geodesy_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.geodesy_points.is_empty() {
        empty(ui, t("in_geodesy_empty"));
        return;
    }

    let mut edited: Option<GeodesyPoint> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("in_geo_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 80.0, t("in_mark"));
                    head_l(ui, 80.0, t("col_axis"));
                    head_l(ui, 70.0, t("in_level"));
                    head_l(ui, 100.0, t("in_design"));
                    head_l(ui, 100.0, t("in_fact"));
                    head_r(ui, 90.0, t("in_deviation"));
                    head_l(ui, 90.0, t("in_tolerance"));
                    head_l(ui, 60.0, t("col_unit"));
                    head_l(ui, 104.0, t("in_measured"));
                    head_l(ui, 140.0, t("in_surveyor"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.geodesy_points {
                        let mut x = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut x.mark))
                            .changed();
                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut x.axis))
                            .changed();
                        changed |= ui
                            .add_sized([70.0, 22.0], egui::TextEdit::singleline(&mut x.level))
                            .changed();
                        changed |= super::materials::num_edit(ui, 100.0, &mut x.design, 1.0, 1e7);
                        changed |= super::materials::num_edit(ui, 100.0, &mut x.fact, 1.0, 1e7);

                        // Chetlanish hisoblanadi: fakt minus loyiha.
                        let dev = x.deviation();
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(format!("{dev:+.1}"))
                                .size(12.0)
                                .strong()
                                .color(if x.within() {
                                    theme::ok()
                                } else {
                                    theme::danger()
                                }),
                        );

                        changed |=
                            super::materials::num_edit(ui, 90.0, &mut x.tolerance, 0.5, 1000.0);
                        changed |= ui
                            .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut x.unit))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("gm{}", x.id), &mut x.measured);
                        changed |= ui
                            .add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut x.surveyor))
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
        app.db.update_geodesy_point(&x);
        if let Some(slot) = app.geodesy_points.iter_mut().find(|y| y.id == x.id) {
            *slot = x;
        }
    }
    if let Some(id) = removed {
        app.db.delete_geodesy_point(id);
        app.reload_modules();
    }
}

// ================================================================ Kunlik hisobot

fn day_tab(ui: &mut egui::Ui, app: &mut App) {
    ui.label(
        RichText::new(t("in_day_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    // Oxirgi 14 kun: hisobot qo'lda yozilmaydi, bazadagi yozuvlardan yig'iladi.
    let mut days: Vec<checks::SupervisionDay> = (0..14)
        .map(|back| {
            let d = app.today - chrono::Duration::days(back);
            checks::supervision_day(
                d,
                &app.inspections,
                &app.issues,
                &app.quality,
                &app.exec_docs,
                &app.concrete_tests,
                &app.geodesy_points,
            )
        })
        .collect();
    days.retain(|d| !d.is_empty());

    if days.is_empty() {
        empty(ui, t("in_day_empty"));
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("in_day_grid")
                .num_columns(8)
                .spacing([10.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_date"));
                    head_r(ui, 110.0, t("in_d_checks"));
                    head_r(ui, 110.0, t("in_d_failed"));
                    head_r(ui, 120.0, t("in_d_issues_open"));
                    head_r(ui, 120.0, t("in_d_issues_closed"));
                    head_r(ui, 120.0, t("in_d_quality"));
                    head_r(ui, 110.0, t("in_d_docs"));
                    head_r(ui, 130.0, t("in_d_lab"));
                    ui.end_row();

                    for d in &days {
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(d.date.format("%d.%m.%Y").to_string())
                                .size(12.0)
                                .strong(),
                        );
                        num(ui, 110.0, d.inspections, theme::text());
                        num(
                            ui,
                            110.0,
                            d.inspections_failed,
                            if d.inspections_failed == 0 {
                                theme::muted()
                            } else {
                                theme::danger()
                            },
                        );
                        num(
                            ui,
                            120.0,
                            d.issues_opened,
                            if d.issues_opened == 0 {
                                theme::muted()
                            } else {
                                theme::warn()
                            },
                        );
                        num(ui, 120.0, d.issues_closed, theme::ok());
                        // Sifat: jami va salbiylari.
                        cell_r(
                            ui,
                            120.0,
                            if d.quality_checks == 0 {
                                RichText::new(t("dash")).color(theme::muted())
                            } else {
                                RichText::new(format!(
                                    "{} / {}",
                                    d.quality_checks, d.quality_failed
                                ))
                                .size(12.0)
                                .color(if d.quality_failed == 0 {
                                    theme::ok()
                                } else {
                                    theme::danger()
                                })
                            },
                        );
                        num(ui, 110.0, d.docs_signed, theme::ok());
                        cell_r(
                            ui,
                            130.0,
                            if d.concrete_results == 0 && d.geodesy_out == 0 {
                                RichText::new(t("dash")).color(theme::muted())
                            } else {
                                RichText::new(format!("{} · {}", d.concrete_results, d.geodesy_out))
                                    .size(12.0)
                                    .color(if d.geodesy_out == 0 {
                                        theme::text()
                                    } else {
                                        theme::warn()
                                    })
                            },
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Yordamchilar

fn num(ui: &mut egui::Ui, w: f32, v: usize, color: egui::Color32) {
    cell_r(
        ui,
        w,
        if v == 0 {
            RichText::new(t("dash")).color(theme::muted())
        } else {
            RichText::new(v.to_string()).size(12.0).color(color)
        },
    );
}

fn result_color(r: InspectionResult) -> egui::Color32 {
    match r {
        InspectionResult::Pass => theme::ok(),
        InspectionResult::Conditional => theme::warn(),
        InspectionResult::Fail => theme::danger(),
        InspectionResult::Waiting => theme::muted(),
    }
}

/// Sifat natijasi bilan bir xil rang jadvali — ikki ekranda bir xil ko'rinsin.
#[allow(dead_code)]
fn quality_color(r: QualityResult) -> egui::Color32 {
    match r {
        QualityResult::Pass => theme::ok(),
        QualityResult::Conditional => theme::warn(),
        QualityResult::Fail => theme::danger(),
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
