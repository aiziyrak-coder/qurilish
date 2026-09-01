//! «Sifat nazorati» ekrani (TZ XIV).
//!
//! Uch bosqichli nazorat: kirish (material qabuli), operatsion (ish jarayonida)
//! va qabul (bosqich yakuni). Har yozuv natijasi va nuqson bartaraf etish
//! muddati bilan; muddati o'tgan nuqson alohida ajratiladi.

use super::materials::material_label;
use super::warehouse::{cell_l, cell_r};
use super::*;
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

    kpi_row(ui, app);
    ui.add_space(10.0);

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
        table(ui, app);
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
            note: String::new(),
        });
        app.reload_modules();
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let total = app.quality.len();
    let fail = app
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Fail)
        .count();
    let conditional = app
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Conditional)
        .count();
    // Nuqsoni bor va muddati o'tgan yozuvlar — birinchi navbatdagi ish.
    let overdue = app
        .quality
        .iter()
        .filter(|q| {
            q.result != QualityResult::Pass
                && q.deadline.is_some_and(|d| d < app.today)
        })
        .count();
    let pct = if total > 0 {
        (total - fail - conditional) as f64 / total as f64 * 100.0
    } else {
        100.0
    };

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_quality_total"),
            total.to_string(),
            t("kpi_quality_total_hint"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_quality_pass"),
            format!("{pct:.0}%"),
            t("kpi_quality_pass_hint"),
            if pct >= 90.0 { theme::ok() } else { theme::warn() },
        );
        stat_card(
            ui,
            t("kpi_quality_fail"),
            fail.to_string(),
            &format!("{conditional} {}", t("kpi_quality_cond_hint")),
            if fail == 0 { theme::ok() } else { theme::danger() },
        );
        stat_card(
            ui,
            t("kpi_quality_overdue"),
            overdue.to_string(),
            t("kpi_quality_overdue_hint"),
            if overdue == 0 { theme::ok() } else { theme::danger() },
        );
    });
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
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("quality_grid")
                .num_columns(11)
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
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

#[allow(dead_code)]
fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
