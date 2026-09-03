//! «Sifat nazorati» ekrani (TZ XIV).
//!
//! Uch bosqichli nazorat: kirish (material qabuli), operatsion (ish jarayonida)
//! va qabul (bosqich yakuni). Har yozuv natijasi va nuqson bartaraf etish
//! muddati bilan; muddati o'tgan nuqson alohida ajratiladi.

use super::materials::material_label;
use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::NoteTarget;
use crate::domain::{QualityCheck, QualityKind, QualityResult};

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

    let mut add: Option<QualityKind> = None;
    ui.horizontal(|ui| {
        // Uch bosqich — uch tugma, bosqich adashmasin.
        for k in QualityKind::ALL {
            if ui.button(format!("+ {}", k.label())).clicked() {
                add = Some(*k);
            }
        }
        ui.label(
            RichText::new(t("quality_hint"))
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

    let tab_key = egui::Id::new("ql_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("ql_tab_checks")),
            (1, t("ql_tab_checklists")),
            (2, t("ql_tab_defects")),
            (3, t("ql_tab_blocks")),
            (4, t("ql_tab_tests")),
            (5, t("ql_tab_risks")),
            (6, t("ql_tab_week")),
            (7, t("ql_tab_sections")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => checklists_tab(ui, app, pid),
        2 => defects_tab(ui, app),
        3 => blocks_tab(ui, app),
        4 => tests_tab(ui, app, pid),
        5 => risks_tab(ui, app),
        6 => week_tab(ui, app),
        7 => sections_tab(ui, app),
        _ => {
            if app.quality.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t("quality_empty"))
                            .color(theme::muted())
                            .size(15.0),
                    );
                });
            } else {
                // Nazorat nuqtalari paneli o'ng tomonda turadi.
                let open = app
                    .quality_open
                    .filter(|id| app.quality.iter().any(|q| q.id == *id));
                if open.is_some() && ui.available_width() > 980.0 {
                    egui::SidePanel::right("ql_points")
                        .resizable(false)
                        .exact_width(360.0)
                        .frame(egui::Frame::NONE)
                        .show_inside(ui, |ui| {
                            ui.add_space(2.0);
                            points_panel(ui, app);
                        });
                    table(ui, app);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if open.is_some() {
                                points_panel(ui, app);
                                ui.add_space(10.0);
                            }
                            table(ui, app);
                        });
                }
            }
        }
    }

    if let Some(kind) = add {
        let n = app.quality.len() + 1;
        app.db.insert_quality(&QualityCheck {
            id: 0,
            project_id: pid,
            kind,
            date: app.today,
            task_id: None,
            material_id: None,
            subject: format!("{} {n}", t("quality_new_subject")),
            inspector: String::new(),
            result: QualityResult::Pass,
            defect: String::new(),
            deadline: None,
            checklist_id: None,
            fixed_at: None,
            note: String::new(),
        });
        app.reload_modules();
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    // Barcha ko'rsatkich bitta hisobdan olinadi — ekranda va ballda bir xil son.
    let score = crate::checks::quality_score(&app.quality, app.today);
    let total = score.checks;
    let fail = score.failed;
    let conditional = score.conditional;
    // Nuqsoni bor va muddati o'tgan yozuvlar — birinchi navbatdagi ish.
    let overdue = score.overdue;
    // Yopilishga yaqin, lekin sifat bo'yicha yopilmagan ishlar (TZ XIV.35).
    let blocked = app.task_blocks().len();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_quality_total"),
                total.to_string(),
                t("kpi_quality_total_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_quality_score"),
                if score.checks == 0 {
                    t("dash").to_string()
                } else {
                    format!("{:.0}", score.score)
                },
                &if score.checks == 0 {
                    t("kpi_quality_score_hint").to_string()
                } else {
                    // Ball qanday chiqqani ko'rinib tursin.
                    format!(
                        "{}/{} · {} {}",
                        score.passed + score.conditional,
                        score.checks,
                        score.open,
                        t("kpi_score_open")
                    )
                },
                if score.checks == 0 {
                    theme::muted()
                } else if score.score >= 85.0 {
                    theme::ok()
                } else if score.score >= 60.0 {
                    theme::warn()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_quality_fail"),
                fail.to_string(),
                &format!("{conditional} {}", t("kpi_quality_cond_hint")),
                if fail == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_quality_overdue"),
                overdue.to_string(),
                t("kpi_quality_overdue_hint"),
                if overdue == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_quality_blocked"),
                blocked.to_string(),
                t("kpi_quality_blocked_hint"),
                if blocked == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
}

fn result_color(r: QualityResult) -> Color32 {
    match r {
        QualityResult::Pass => theme::ok(),
        QualityResult::Conditional => theme::warn(),
        QualityResult::Fail => theme::danger(),
    }
}

fn table(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<QualityCheck> = None;
    let mut removed: Option<i64> = None;
    let mut open_points: Option<i64> = None;
    let mut open_notes: Option<i64> = None;
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("quality_grid")
                .num_columns(13)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 130.0, t("col_stage"));
                    head_l(ui, 130.0, t("col_result"));
                    head_l(ui, 210.0, t("col_subject"));
                    head_l(ui, 170.0, t("col_task"));
                    head_l(ui, 170.0, t("col_material"));
                    head_l(ui, 140.0, t("col_inspector"));
                    head_l(ui, 200.0, t("col_defect"));
                    head_l(ui, 130.0, t("col_fix_deadline"));
                    head_l(ui, 110.0, t("col_fixed"));
                    head_l(ui, 150.0, t("col_points"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.quality {
                        let mut q = src.clone();
                        let mut changed = false;

                        changed |=
                            super::passport::date_edit(ui, &format!("qc{}", q.id), &mut q.date);
                        egui::ComboBox::from_id_salt(("qc_kind", q.id))
                            .selected_text(q.kind.label())
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for k in QualityKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut q.kind, *k, k.label()).changed();
                                }
                            });
                        egui::ComboBox::from_id_salt(("qc_res", q.id))
                            .selected_text(
                                RichText::new(q.result.label()).color(result_color(q.result)),
                            )
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for r in QualityResult::ALL {
                                    changed |=
                                        ui.selectable_value(&mut q.result, *r, r.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut q.subject))
                            .changed();
                        changed |= task_picker(ui, app, ("qc_task", q.id), &mut q.task_id, 170.0);

                        // Material faqat kirish nazoratida ma'noga ega, lekin
                        // boshqa bosqichda ham bog'lash mumkin.
                        egui::ComboBox::from_id_salt(("qc_mat", q.id))
                            .selected_text(super::issues::truncate(
                                &match q.material_id {
                                    Some(id) => material_label(app, id),
                                    None => t("dash").to_string(),
                                },
                                22,
                            ))
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut q.material_id, None, t("dash"))
                                    .changed();
                                for m in &app.materials {
                                    changed |= ui
                                        .selectable_value(
                                            &mut q.material_id,
                                            Some(m.id),
                                            material_label(app, m.id),
                                        )
                                        .changed();
                                }
                            });

                        changed |= ui
                            .add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut q.inspector))
                            .changed();
                        changed |= ui
                            .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut q.defect))
                            .changed();

                        // Bartaraf etish muddati — faqat nuqson bo'lganda kerak.
                        ui.horizontal(|ui| {
                            let mut has = q.deadline.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                q.deadline = has.then(|| today + chrono::Duration::days(7));
                                changed = true;
                            }
                            if let Some(mut d) = q.deadline {
                                if super::passport::date_edit(ui, &format!("qcd{}", q.id), &mut d) {
                                    q.deadline = Some(d);
                                    changed = true;
                                }
                                if q.result != QualityResult::Pass && d < today {
                                    ui.label(RichText::new("!").color(theme::danger()).strong());
                                }
                            }
                        });

                        // Nuqson bartaraf etilgan sana (TZ XIV.20).
                        ui.horizontal(|ui| {
                            let mut fixed = q.fixed_at.is_some();
                            if ui
                                .checkbox(&mut fixed, "")
                                .on_hover_text(t("fixed_hint"))
                                .changed()
                            {
                                q.fixed_at = fixed.then_some(today);
                                changed = true;
                            }
                            if let Some(mut d) = q.fixed_at {
                                if super::passport::date_edit(ui, &format!("qcf{}", q.id), &mut d) {
                                    q.fixed_at = Some(d);
                                    changed = true;
                                }
                            }
                        });

                        // Nazorat nuqtalari: to'ldirilgani / jami.
                        let pts: Vec<&crate::domain::CheckPoint> = app
                            .check_points
                            .iter()
                            .filter(|p| p.check_id == q.id)
                            .collect();
                        let done = pts
                            .iter()
                            .filter(|p| p.result != crate::domain::PointResult::Pending)
                            .count();
                        let label = if pts.is_empty() {
                            t("points_none").to_string()
                        } else {
                            format!("{done} / {}", pts.len())
                        };
                        let color = if pts.is_empty() {
                            theme::muted()
                        } else if done == pts.len() {
                            theme::ok()
                        } else {
                            theme::warn()
                        };
                        if ui
                            .add_sized(
                                [150.0, 22.0],
                                egui::Button::new(RichText::new(label).size(11.5).color(color))
                                    .frame(app.quality_open == Some(q.id)),
                            )
                            .on_hover_text(t("points_open_hint"))
                            .clicked()
                        {
                            open_points = Some(q.id);
                        }

                        // Foto va izoh (TZ XIV.11, XIV.21): «oldin» va «keyin»
                        // suratlari shu yerda belgilanadi.
                        if super::notes::badge(ui, app, NoteTarget::Quality, q.id) {
                            open_notes = Some(q.id);
                        }

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(q.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(q);
                        }
                    }
                });
        });

    if let Some(q) = edited {
        app.db.update_quality(&q);
        if let Some(slot) = app.quality.iter_mut().find(|x| x.id == q.id) {
            *slot = q;
        }
    }
    if let Some(id) = removed {
        app.db.del("quality_check", id);
        app.reload_modules();
    }
    if let Some(id) = open_points {
        app.quality_open = (app.quality_open != Some(id)).then_some(id);
    }
    super::notes::below_table(ui, app, NoteTarget::Quality, open_notes);
}

// ================================================================ Bo'limlar

/// Bo'limlar kesimi, nuqsonlar ustuvorligi va texnologik ketma-ketlik
/// (TZ XIV.15, 19, 27, VII.28).
fn sections_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let sections = app.section_quality();
    let priority = app.defect_priority();
    let breaks = app.sequence_breaks();

    ui.label(
        RichText::new(t("ql_sec_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Bo'limlar (TZ XIV.15) ----------
            if sections.is_empty() {
                ui.label(
                    RichText::new(t("ql_sec_empty"))
                        .size(12.5)
                        .color(theme::muted()),
                );
            } else {
                egui::Grid::new("ql_sections")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 200.0, t("col_section"));
                        head_r(ui, 100.0, t("ql_w_checks"));
                        head_r(ui, 100.0, t("ql_sec_failed"));
                        head_r(ui, 110.0, t("ql_w_open"));
                        head_r(ui, 120.0, t("dr_defects_overdue"));
                        head_r(ui, 90.0, t("dr_score"));
                        ui.end_row();

                        for s in &sections {
                            cell_l(ui, 200.0, RichText::new(s.section.label()).size(12.5));
                            cell_r(ui, 100.0, RichText::new(s.checks.to_string()).size(12.0));
                            cell_r(
                                ui,
                                100.0,
                                RichText::new(s.failed.to_string()).size(12.0).color(
                                    if s.failed == 0 {
                                        theme::muted()
                                    } else {
                                        theme::warn()
                                    },
                                ),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(s.defects_open.to_string()).size(12.0),
                            );
                            cell_r(
                                ui,
                                120.0,
                                RichText::new(s.overdue.to_string()).size(12.0).color(
                                    if s.overdue == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(format!("{:.0}", s.score)).size(12.5).color(
                                    if s.score >= 80.0 {
                                        theme::ok()
                                    } else if s.score >= 60.0 {
                                        theme::warn()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            ui.end_row();
                        }
                    });
            }

            // ---------- Ustuvorlik (TZ XIV.19) ----------
            ui.add_space(16.0);
            ui.label(RichText::new(t("ql_priority")).size(13.5).strong());
            ui.label(
                RichText::new(t("ql_priority_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if priority.is_empty() {
                ui.label(
                    RichText::new(t("ql_priority_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for p in priority.iter().take(12) {
                ui.horizontal(|ui| {
                    // Ball chizig'i: tartib ko'z bilan ko'rinadi.
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(52.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, theme::line());
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(rect.width() * (p.weight / 100.0) as f32, rect.height()),
                        ),
                        2.0,
                        if p.urgent() {
                            theme::danger()
                        } else {
                            theme::warn()
                        },
                    );
                    // Nuqson qaysi ishga tegishli — sichqoncha ostida.
                    let task = app
                        .quality
                        .iter()
                        .find(|q| q.id == p.check_id)
                        .and_then(|q| q.task_id)
                        .and_then(|id| app.task(id))
                        .map(|x| format!("{} {}", x.wbs, x.name))
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(super::issues::truncate(&p.defect, 50))
                            .size(12.0)
                            .color(if p.urgent() {
                                theme::text()
                            } else {
                                theme::muted()
                            }),
                    )
                    .on_hover_text(if task.is_empty() {
                        t("dash").to_string()
                    } else {
                        task
                    });
                    // Nima uchun aynan shu nuqson birinchi ekani.
                    ui.label(
                        RichText::new(
                            p.reasons
                                .iter()
                                .map(|k| t(k))
                                .collect::<Vec<_>>()
                                .join(" · "),
                        )
                        .size(10.5)
                        .color(theme::muted()),
                    );
                });
            }

            // ---------- Ketma-ketlik (TZ XIV.27, VII.28) ----------
            ui.add_space(16.0);
            ui.label(RichText::new(t("ql_sequence")).size(13.5).strong());
            ui.label(
                RichText::new(t("ql_sequence_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if breaks.is_empty() {
                ui.label(
                    RichText::new(t("ql_sequence_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for b in &breaks {
                let name = |id: i64| {
                    app.task(id)
                        .map(|x| format!("{} {}", x.wbs, x.name))
                        .unwrap_or_default()
                };
                ui.label(
                    RichText::new(format!(
                        "· {} ← {} ({:.0}%) — {}",
                        super::issues::truncate(&name(b.task_id), 30),
                        super::issues::truncate(&name(b.pred_id), 30),
                        b.pred_progress,
                        if b.pred_checked {
                            t("ql_seq_checked")
                        } else {
                            t("ql_seq_unchecked")
                        }
                    ))
                    .size(12.0)
                    .color(if b.pred_checked {
                        theme::muted()
                    } else {
                        theme::warn()
                    }),
                );
            }
            ui.add_space(16.0);
        });
}

// ================================================================ Haftalik hisobot

/// Hafta bo'yicha sifat xulosasi (TZ XIV.37).
///
/// Ball sifat modulidagi umumiy ball bilan bir xil qoidada hisoblanadi —
/// hisobotdagi son ekrandagi bilan hech qachon farq qilmaydi.
fn week_tab(ui: &mut egui::Ui, app: &mut App) {
    let w = app.quality_week();

    ui.label(
        RichText::new(format!(
            "{} — {}",
            w.from.format("%d.%m.%Y"),
            w.to.format("%d.%m.%Y")
        ))
        .size(11.5)
        .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("ql_w_score"),
                format!("{:.0}", w.score),
                t("ql_w_score_hint"),
                if w.score >= 80.0 {
                    theme::ok()
                } else if w.score >= 60.0 {
                    theme::warn()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("ql_w_checks"),
                w.checks.to_string(),
                &format!("{} / {}", w.passed, w.failed),
                theme::text(),
            ),
            stat(
                t("ql_w_opened"),
                w.defects_opened.to_string(),
                t("ql_w_opened_hint"),
                theme::text(),
            ),
            stat(
                t("ql_w_closed"),
                w.defects_closed.to_string(),
                t("ql_w_closed_hint"),
                theme::ok(),
            ),
            stat(
                t("ql_w_open"),
                w.defects_open.to_string(),
                &format!("{} {}", w.overdue, t("ql_w_overdue")),
                if w.defects_open == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(14.0);

    if w.top_defects.is_empty() {
        ui.label(
            RichText::new(t("ql_w_no_defects"))
                .size(12.5)
                .color(theme::ok()),
        );
        return;
    }

    ui.label(RichText::new(t("ql_w_top")).size(13.5).strong());
    ui.label(
        RichText::new(t("ql_w_top_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);
    for (name, count) in &w.top_defects {
        ui.label(
            RichText::new(format!(
                "· {} — {} {}",
                super::issues::truncate(name, 60),
                count,
                t("ql_w_times")
            ))
            .size(12.0),
        );
    }
}

// ================================================== Nazorat nuqtalari

/// Tanlangan tekshiruvning nazorat nuqtalari (TZ XIV.8).
fn points_panel(ui: &mut egui::Ui, app: &mut App) {
    use crate::domain::{CheckPoint, PointResult};
    let Some(cid) = app.quality_open else { return };
    let Some(q) = app.quality.iter().find(|q| q.id == cid).cloned() else {
        return;
    };

    let mut close = false;
    let mut apply: Option<i64> = None;
    let mut edited: Option<CheckPoint> = None;
    let mut removed: Option<i64> = None;
    let mut add = false;

    egui::Frame::group(ui.style())
        .fill(theme::card())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(320.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("points_title")).size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("x").clicked() {
                        close = true;
                    }
                });
            });
            ui.label(RichText::new(&q.subject).size(12.0).color(theme::muted()));
            ui.add_space(6.0);

            // Chek-list namunasini biriktirish.
            let fit: Vec<&crate::domain::Checklist> =
                app.checklists.iter().filter(|c| c.kind == q.kind).collect();
            if fit.is_empty() {
                ui.label(
                    RichText::new(t("points_no_checklist"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(t("points_apply"))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                    for c in &fit {
                        if ui
                            .small_button(super::issues::truncate(&c.name, 24))
                            .on_hover_text(t("points_apply_hint"))
                            .clicked()
                        {
                            apply = Some(c.id);
                        }
                    }
                });
            }
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            let points: Vec<CheckPoint> = app
                .check_points
                .iter()
                .filter(|p| p.check_id == cid)
                .cloned()
                .collect();
            if points.is_empty() {
                ui.label(
                    RichText::new(t("points_empty"))
                        .size(12.0)
                        .color(theme::muted()),
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for src in &points {
                            let mut p = src.clone();
                            let mut changed = false;
                            ui.horizontal_top(|ui| {
                                // Natija — uch holat, tor tugmalar bilan.
                                egui::ComboBox::from_id_salt(("cp_res", p.id))
                                    .selected_text(
                                        RichText::new(short_point(p.result))
                                            .size(11.5)
                                            .color(point_color(p.result)),
                                    )
                                    .width(52.0)
                                    .show_ui(ui, |ui| {
                                        for r in PointResult::ALL {
                                            changed |= ui
                                                .selectable_value(&mut p.result, *r, r.label())
                                                .changed();
                                        }
                                    });
                                ui.vertical(|ui| {
                                    changed |= ui
                                        .add_sized(
                                            [ui.available_width().min(240.0), 22.0],
                                            egui::TextEdit::singleline(&mut p.text),
                                        )
                                        .changed();
                                    if !p.norm_doc.is_empty() {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {}",
                                                p.norm_doc, p.norm_clause
                                            ))
                                            .size(10.5)
                                            .color(theme::muted()),
                                        );
                                    }
                                });
                                if ui
                                    .small_button(RichText::new("x").color(theme::danger()))
                                    .clicked()
                                {
                                    removed = Some(p.id);
                                }
                            });
                            ui.add_space(3.0);
                            if changed {
                                edited = Some(p);
                            }
                        }
                    });
            }
            ui.add_space(6.0);
            if ui.small_button(t("points_add")).clicked() {
                add = true;
            }
        });

    if let Some(list) = apply {
        let n = app.db.apply_checklist(cid, list);
        if let Some(mut x) = app.quality.iter().find(|x| x.id == cid).cloned() {
            x.checklist_id = Some(list);
            app.db.update_quality(&x);
        }
        app.reload_modules();
        app.notify(format!("{} {n}", t("points_applied")));
    }
    if add {
        let pos = app
            .check_points
            .iter()
            .filter(|p| p.check_id == cid)
            .map(|p| p.pos)
            .max()
            .unwrap_or(0)
            + 1;
        app.db.insert_check_point(&crate::domain::CheckPoint {
            id: 0,
            check_id: cid,
            pos,
            text: String::new(),
            norm_doc: String::new(),
            norm_clause: String::new(),
            result: crate::domain::PointResult::Pending,
            note: String::new(),
        });
        app.reload_modules();
    }
    if let Some(p) = edited {
        app.db.update_check_point(&p);
        app.reload_modules();
    }
    if let Some(id) = removed {
        app.db.del("check_point", id);
        app.reload_modules();
    }
    if close {
        app.quality_open = None;
    }
}

fn short_point(r: crate::domain::PointResult) -> &'static str {
    use crate::domain::PointResult;
    match r {
        PointResult::Pending => t("pts_pending"),
        PointResult::Pass => t("pts_pass"),
        PointResult::Fail => t("pts_fail"),
        PointResult::Na => t("pts_na"),
    }
}

fn point_color(r: crate::domain::PointResult) -> Color32 {
    use crate::domain::PointResult;
    match r {
        PointResult::Pending => theme::muted(),
        PointResult::Pass => theme::ok(),
        PointResult::Fail => theme::danger(),
        PointResult::Na => theme::muted(),
    }
}

// ================================================================ Chek-listlar

/// Chek-list namunalari (TZ XIV.8).
fn checklists_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    use crate::domain::{Checklist, ChecklistItem};

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_checklist")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("checklists_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.checklists.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("checklists_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        let mut edited: Option<Checklist> = None;
        let mut removed: Option<i64> = None;
        let mut item_edited: Option<ChecklistItem> = None;
        let mut item_removed: Option<i64> = None;
        let mut item_add: Option<i64> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for src in &app.checklists {
                    let mut c = src.clone();
                    let mut changed = false;
                    egui::Frame::group(ui.style())
                        .fill(theme::card())
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                changed |= ui
                                    .add_sized(
                                        [260.0, 22.0],
                                        egui::TextEdit::singleline(&mut c.name),
                                    )
                                    .changed();
                                egui::ComboBox::from_id_salt(("cl_sec", c.id))
                                    .selected_text(c.section.code())
                                    .width(90.0)
                                    .show_ui(ui, |ui| {
                                        for sec in crate::model::Section::ALL {
                                            changed |= ui
                                                .selectable_value(&mut c.section, sec, sec.label())
                                                .changed();
                                        }
                                    });
                                egui::ComboBox::from_id_salt(("cl_kind", c.id))
                                    .selected_text(c.kind.label())
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for k in QualityKind::ALL {
                                            changed |= ui
                                                .selectable_value(&mut c.kind, *k, k.label())
                                                .changed();
                                        }
                                    });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button(RichText::new("x").color(theme::danger()))
                                            .clicked()
                                        {
                                            removed = Some(c.id);
                                        }
                                        if ui.small_button(t("checklist_add_item")).clicked() {
                                            item_add = Some(c.id);
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);

                            let items: Vec<ChecklistItem> = app
                                .checklist_items
                                .iter()
                                .filter(|i| i.checklist_id == c.id)
                                .cloned()
                                .collect();
                            if items.is_empty() {
                                ui.label(
                                    RichText::new(t("checklist_no_items"))
                                        .size(11.0)
                                        .color(theme::muted()),
                                );
                            }
                            for isrc in &items {
                                let mut i = isrc.clone();
                                let mut ic = false;
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{}.", i.pos))
                                            .size(11.5)
                                            .color(theme::muted())
                                            .monospace(),
                                    );
                                    ic |= ui
                                        .add_sized(
                                            [380.0, 22.0],
                                            egui::TextEdit::singleline(&mut i.text),
                                        )
                                        .changed();
                                    ic |= ui
                                        .add_sized(
                                            [130.0, 22.0],
                                            egui::TextEdit::singleline(&mut i.norm_doc)
                                                .hint_text(t("col_norm_doc")),
                                        )
                                        .changed();
                                    ic |= ui
                                        .add_sized(
                                            [90.0, 22.0],
                                            egui::TextEdit::singleline(&mut i.norm_clause)
                                                .hint_text(t("col_norm_clause")),
                                        )
                                        .changed();
                                    if ui
                                        .small_button(RichText::new("x").color(theme::danger()))
                                        .clicked()
                                    {
                                        item_removed = Some(i.id);
                                    }
                                });
                                if ic {
                                    item_edited = Some(i);
                                }
                            }
                        });
                    ui.add_space(8.0);
                    if changed {
                        edited = Some(c);
                    }
                }
            });

        if let Some(c) = edited {
            app.db.update_checklist(&c);
            app.reload_modules();
        }
        if let Some(id) = removed {
            app.db.del("checklist", id);
            app.reload_modules();
        }
        if let Some(i) = item_edited {
            app.db.update_checklist_item(&i);
            app.reload_modules();
        }
        if let Some(id) = item_removed {
            app.db.del("checklist_item", id);
            app.reload_modules();
        }
        if let Some(list) = item_add {
            let pos = app
                .checklist_items
                .iter()
                .filter(|i| i.checklist_id == list)
                .map(|i| i.pos)
                .max()
                .unwrap_or(0)
                + 1;
            app.db.insert_checklist_item(&ChecklistItem {
                id: 0,
                checklist_id: list,
                pos,
                text: String::new(),
                norm_doc: String::new(),
                norm_clause: String::new(),
            });
            app.reload_modules();
        }
    }

    if add {
        let n = app.checklists.len() + 1;
        app.db.insert_checklist(&Checklist {
            id: 0,
            project_id: pid,
            name: format!("{} {n}", t("checklist_new_name")),
            section: crate::model::Section::None,
            kind: QualityKind::Operational,
            note: String::new(),
        });
        app.reload_modules();
    }
}

// ================================================================ Brak tahlili

/// Takrorlanuvchi nuqsonlar (TZ XIV.30–31).
fn defects_tab(ui: &mut egui::Ui, app: &mut App) {
    let groups = crate::checks::defect_groups(&app.quality);
    if groups.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("defects_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    ui.label(
        RichText::new(t("defects_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ql_defects")
                .num_columns(5)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 360.0, t("col_defect"));
                    head_r(ui, 90.0, t("col_times"));
                    head_r(ui, 90.0, t("col_open"));
                    head_l(ui, 110.0, t("col_last"));
                    head_l(ui, 300.0, t("col_tasks"));
                    ui.end_row();

                    for g in &groups {
                        // Ikki martadan ko'p takrorlangani — tizimli muammo.
                        let repeat = g.count > 2;
                        cell_l(
                            ui,
                            360.0,
                            RichText::new(super::issues::truncate(&g.defect, 52))
                                .size(12.0)
                                .color(if repeat {
                                    theme::danger()
                                } else {
                                    theme::text()
                                }),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(g.count.to_string()).size(12.5).strong(),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(g.open.to_string())
                                .size(12.0)
                                .color(if g.open > 0 {
                                    theme::danger()
                                } else {
                                    theme::ok()
                                }),
                        );
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(match g.last {
                                Some(d) => d.format("%d.%m.%y").to_string(),
                                None => t("dash").to_string(),
                            })
                            .size(11.5)
                            .monospace()
                            .color(theme::muted()),
                        );
                        let names: Vec<String> = g
                            .tasks
                            .iter()
                            .filter_map(|id| app.task(*id).map(|x| x.wbs.clone()))
                            .collect();
                        cell_l(
                            ui,
                            300.0,
                            RichText::new(if names.is_empty() {
                                t("dash").to_string()
                            } else {
                                names.join(", ")
                            })
                            .size(11.5)
                            .color(theme::muted()),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Bloklash

/// Sifat bo'yicha yopib bo'lmaydigan ishlar (TZ XIV.10, 35).
fn blocks_tab(ui: &mut egui::Ui, app: &mut App) {
    let blocks = app.task_blocks();

    ui.label(
        RichText::new(t("blocks_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    if blocks.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("blocks_empty"))
                    .color(theme::ok())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ql_blocks")
                .num_columns(6)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 320.0, t("col_task"));
                    head_r(ui, 90.0, t("col_progress"));
                    head_r(ui, 110.0, t("col_open_defects"));
                    head_r(ui, 110.0, t("col_overdue"));
                    head_r(ui, 130.0, t("col_pending_points"));
                    head_l(ui, 220.0, t("col_reason"));
                    ui.end_row();

                    for b in &blocks {
                        let task = app.task(b.task_id);
                        cell_l(
                            ui,
                            320.0,
                            RichText::new(super::issues::truncate(
                                &task
                                    .map(|x| format!("{} {}", x.wbs, x.name))
                                    .unwrap_or_default(),
                                46,
                            ))
                            .size(12.0),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(format!(
                                "{:.0}%",
                                task.map(|x| x.progress).unwrap_or(0.0)
                            ))
                            .size(12.0)
                            .color(theme::muted()),
                        );
                        let num = |v: usize, danger: bool| {
                            RichText::new(v.to_string()).size(12.0).color(if v == 0 {
                                theme::muted()
                            } else if danger {
                                theme::danger()
                            } else {
                                theme::warn()
                            })
                        };
                        cell_r(ui, 110.0, num(b.open_defects, true));
                        cell_r(ui, 110.0, num(b.overdue, true));
                        cell_r(ui, 130.0, num(b.pending_points, false));
                        cell_l(
                            ui,
                            220.0,
                            RichText::new(if b.no_acceptance {
                                t("block_no_acceptance")
                            } else if b.open_defects > 0 {
                                t("block_open_defects")
                            } else {
                                t("block_pending_points")
                            })
                            .size(11.5)
                            .color(if b.severe() {
                                theme::danger()
                            } else {
                                theme::warn()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Sinovlar

/// Laboratoriya va maydon sinovlari (TZ XIV.22, 24-25).
///
/// Betondan farqi: bu yerda «o'tdi / o'tmadi» muhim. Raqamli qiymat
/// bo'lsa u ham saqlanadi, lekin hukm laboratoriyaniki — ilova faqat
/// solishtiradi.
fn tests_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let can = app.can_edit(Screen::Quality);
    let today = app.today;

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if can && ui.button(t("ql_add_test")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("ql_tests_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.lab_tests.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("ql_tests_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        let pending = app.lab_tests.iter().filter(|x| x.pending()).count();
        let failed = app
            .lab_tests
            .iter()
            .filter(|x| x.result == crate::domain::LabTestResult::Fail)
            .count();
        stat_row(
            ui,
            vec![
                stat(
                    t("ql_tests_total"),
                    app.lab_tests.len().to_string(),
                    t("ql_tests_total_hint"),
                    theme::accent(),
                ),
                stat(
                    t("ql_tests_pending"),
                    pending.to_string(),
                    t("ql_tests_pending_hint"),
                    if pending == 0 {
                        theme::ok()
                    } else {
                        theme::warn()
                    },
                ),
                stat(
                    t("ql_tests_failed"),
                    failed.to_string(),
                    t("ql_tests_failed_hint"),
                    if failed == 0 {
                        theme::ok()
                    } else {
                        theme::danger()
                    },
                ),
            ],
        );
        ui.add_space(10.0);

        let tasks = app.tasks.clone();
        let mut edited: Option<crate::domain::LabTest> = None;
        let mut removed: Option<i64> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("ql_tests_grid")
                    .num_columns(11)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 90.0, t("col_number"));
                        head_l(ui, 140.0, t("col_kind"));
                        head_l(ui, 230.0, t("ql_test_subject"));
                        head_l(ui, 104.0, t("col_date"));
                        head_l(ui, 130.0, t("col_task"));
                        head_r(ui, 90.0, t("ql_test_value"));
                        head_r(ui, 90.0, t("ql_test_required"));
                        head_l(ui, 60.0, t("col_unit"));
                        head_l(ui, 120.0, t("col_result"));
                        head_l(ui, 150.0, t("ql_test_lab"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.lab_tests {
                            let mut x = src.clone();
                            let mut changed = false;

                            changed |= ui
                                .add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut x.number))
                                .changed();
                            egui::ComboBox::from_id_salt(("ql_lk", x.id))
                                .selected_text(x.kind.label())
                                .width(140.0)
                                .show_ui(ui, |ui| {
                                    for k in crate::domain::LabTestKind::ALL {
                                        changed |= ui
                                            .selectable_value(&mut x.kind, *k, k.label())
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add_sized(
                                    [230.0, 22.0],
                                    egui::TextEdit::singleline(&mut x.subject),
                                )
                                .changed();
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("qlt{}", x.id),
                                &mut x.date,
                            );

                            let label = x
                                .task_id
                                .and_then(|id| tasks.iter().find(|t| t.id == id))
                                .map(|t| t.wbs.clone())
                                .unwrap_or_else(|| t("dash").to_string());
                            egui::ComboBox::from_id_salt(("ql_lt", x.id))
                                .selected_text(label)
                                .width(130.0)
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

                            // Qiymat majburiy emas: ba'zi sinovlarda faqat
                            // «o'tdi / o'tmadi» bo'ladi.
                            ui.horizontal(|ui| {
                                let mut has = x.value.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    x.value = has.then_some(0.0);
                                    changed = true;
                                }
                                if let Some(mut v) = x.value {
                                    if super::materials::num_edit(ui, 60.0, &mut v, 0.1, 1e9) {
                                        x.value = Some(v);
                                        changed = true;
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                let mut has = x.required.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    x.required = has.then_some(0.0);
                                    changed = true;
                                }
                                if let Some(mut v) = x.required {
                                    if super::materials::num_edit(ui, 60.0, &mut v, 0.1, 1e9) {
                                        x.required = Some(v);
                                        changed = true;
                                    }
                                }
                            });
                            changed |= ui
                                .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut x.unit))
                                .changed();

                            egui::ComboBox::from_id_salt(("ql_lr", x.id))
                                .selected_text(
                                    RichText::new(x.result.label())
                                        .color(lab_result_color(x.result)),
                                )
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for r in crate::domain::LabTestResult::ALL {
                                        changed |= ui
                                            .selectable_value(&mut x.result, *r, r.label())
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut x.lab))
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
            app.db.update_lab_test(&x);
            if let Some(slot) = app.lab_tests.iter_mut().find(|y| y.id == x.id) {
                *slot = x;
            }
        }
        if let Some(id) = removed {
            app.db.delete_lab_test(id);
            app.reload_modules();
        }
    }

    if add {
        app.db.insert_lab_test(&crate::domain::LabTest {
            id: 0,
            project_id: pid,
            task_id: None,
            kind: crate::domain::LabTestKind::Other,
            number: String::new(),
            subject: String::new(),
            date: today,
            value: None,
            required: None,
            unit: String::new(),
            result: crate::domain::LabTestResult::Waiting,
            lab: String::new(),
            retest: None,
            note: String::new(),
        });
        app.reload_modules();
    }
}

fn lab_result_color(r: crate::domain::LabTestResult) -> egui::Color32 {
    use crate::domain::LabTestResult as R;
    match r {
        R::Pass => theme::ok(),
        R::Fail => theme::danger(),
        R::Waiting => theme::muted(),
    }
}

// ================================================================ Xavflar

/// Nuqson ehtimoli yuqori ishlar va mas'ullar reytingi (TZ XIV.28-29, 32).
fn risks_tab(ui: &mut egui::Ui, app: &mut App) {
    let risks = app.quality_risks();
    let rating = crate::checks::contractor_quality(&app.quality, app.today);
    let banned = crate::checks::banned_usage(&app.materials, &app.stock_moves);

    ui.label(
        RichText::new(t("ql_risks_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Taqiqlangan material ----------
            if !banned.is_empty() {
                ui.label(
                    RichText::new(t("ql_banned_title"))
                        .size(13.5)
                        .strong()
                        .color(theme::danger()),
                );
                ui.label(
                    RichText::new(t("ql_banned_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.add_space(4.0);
                for b in &banned {
                    let name = app
                        .materials
                        .iter()
                        .find(|m| m.id == b.material_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!(
                                "{name} — {} ({} {})",
                                super::materials::trim_num(b.qty),
                                b.moves,
                                t("ql_banned_moves")
                            ))
                            .size(12.0)
                            .color(theme::danger()),
                        );
                        if !b.reason.trim().is_empty() {
                            ui.label(RichText::new(&b.reason).size(11.0).color(theme::muted()));
                        }
                    });
                }
                ui.add_space(14.0);
            }

            // ---------- Xavfli ishlar ----------
            ui.label(RichText::new(t("ql_risks_title")).size(13.5).strong());
            ui.add_space(4.0);
            if risks.is_empty() {
                ui.label(
                    RichText::new(t("ql_risks_none"))
                        .size(12.0)
                        .color(theme::ok()),
                );
            }
            for r in &risks {
                let Some(task) = app.task(r.task_id) else {
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let color = if r.level >= 3 {
                        theme::danger()
                    } else {
                        theme::warn()
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 1.5, color);
                    ui.add_space(6.0);
                    ui.add_sized(
                        [260.0, 18.0],
                        egui::Label::new(
                            RichText::new(super::issues::truncate(
                                &format!("{} {}", task.wbs, task.name),
                                34,
                            ))
                            .size(12.5),
                        ),
                    );
                    ui.label(
                        RichText::new(
                            r.reasons
                                .iter()
                                .map(risk_reason_text)
                                .collect::<Vec<_>>()
                                .join(" · "),
                        )
                        .size(11.5)
                        .color(color),
                    );
                });
                ui.add_space(3.0);
            }

            // ---------- Mas'ullar reytingi ----------
            if !rating.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(t("ql_rating_title")).size(13.5).strong());
                ui.label(
                    RichText::new(t("ql_rating_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                egui::Grid::new("ql_rating_grid")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 200.0, t("ql_rating_name"));
                        head_r(ui, 90.0, t("ql_rating_checks"));
                        head_r(ui, 90.0, t("ql_rating_failed"));
                        head_r(ui, 110.0, t("ql_rating_open"));
                        head_r(ui, 110.0, t("ql_rating_overdue"));
                        head_r(ui, 90.0, t("ql_rating_score"));
                        ui.end_row();
                        for c in &rating {
                            cell_l(ui, 200.0, RichText::new(&c.name).size(12.5));
                            cell_r(ui, 90.0, RichText::new(c.checks.to_string()).size(12.0));
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(c.failed.to_string()).size(12.0).color(
                                    if c.failed == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(c.open_defects.to_string()).size(12.0),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(c.overdue.to_string()).size(12.0).color(
                                    if c.overdue == 0 {
                                        theme::muted()
                                    } else {
                                        theme::danger()
                                    },
                                ),
                            );
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(format!("{:.0}", c.score))
                                    .size(12.5)
                                    .strong()
                                    .color(if c.score >= 85.0 {
                                        theme::ok()
                                    } else if c.score >= 60.0 {
                                        theme::warn()
                                    } else {
                                        theme::danger()
                                    }),
                            );
                            ui.end_row();
                        }
                    });
            }
            ui.add_space(14.0);
        });
}

/// Sifat xavfi sababining matni.
fn risk_reason_text(r: &crate::checks::QualityRiskReason) -> String {
    use crate::checks::QualityRiskReason as R;
    match r {
        R::PastDefects { count } => format!("{count} {}", t("ql_r_past")),
        R::Delayed { days } => format!("{} {} {}", t("ql_r_delayed"), days, t("ql_r_days")),
        R::OverUsage => t("ql_r_over").to_string(),
        R::NoPpr => t("ql_r_no_ppr").to_string(),
        R::NoInspection => t("ql_r_no_inspection").to_string(),
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
