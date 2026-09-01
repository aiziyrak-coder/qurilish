//! «PPR — ishlar rejasi» ekrani (TZ I.3).
//!
//! TZ modulga ikkita vazifa qo'yadi: PPR, texnologik, sifat va xavfsizlik
//! kartalarini saqlash hamda ular bo'yicha beshta tekshiruvni bajarish —
//! PPR loyihaga mos keladimi, odam yetadimi, texnika yetadimi, ketma-ketlik
//! saqlanganmi va qanday risklar bor.
//!
//! Ekran shu vazifalarga qarab bo'lingan: kartalar ro'yxati, resurs
//! gistogrammasi, ishlar qoplanishi va tekshiruv natijasi.

use super::issues;
use super::*;
use crate::checks;
use crate::domain::{IssueModule, PprDoc, PprKind};
use crate::model::Section;
use chrono::{Datelike, Duration, NaiveDate};
use egui::{vec2, Sense};

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

    app.auto_check(IssueModule::Ppr);

    let tab_key = egui::Id::new("ppr_tab_v2");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    toolbar(ui, app);
    ui.add_space(8.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        let tabs = [
            (0u8, t("tab_ppr_cards")),
            (1, t("tab_ppr_resources")),
            (2, t("tab_ppr_coverage")),
            (3, t("tab_issues")),
        ];
        for (i, label) in tabs {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => resources_tab(ui, app),
        2 => coverage_tab(ui, app),
        3 => issues_tab(ui, app),
        _ => cards_tab(ui, app),
    }
}

// ================================================================ Toolbar

fn toolbar(ui: &mut egui::Ui, app: &mut App) {
    let mut add = false;
    let mut run = false;
    let mut settings_changed = false;

    ui.horizontal(|ui| {
        if ui.button(t("add_ppr")).clicked() {
            add = true;
        }
        if ui
            .button(RichText::new(t("run_ppr_check")).strong())
            .on_hover_text(t("run_check_hint"))
            .clicked()
        {
            run = true;
        }
        ui.separator();

        // Mavjud resurs: yetarlilik shu qiymatga nisbatan hisoblanadi.
        ui.label(
            RichText::new(t("avail_workers"))
                .color(theme::muted())
                .size(12.0),
        );
        settings_changed |= ui
            .add(
                egui::DragValue::new(&mut app.settings.avail_workers)
                    .range(0..=10_000)
                    .speed(1.0),
            )
            .changed();
        ui.label(
            RichText::new(t("avail_machines"))
                .color(theme::muted())
                .size(12.0),
        );
        settings_changed |= ui
            .add(
                egui::DragValue::new(&mut app.settings.avail_machines)
                    .range(0..=1000)
                    .speed(1.0),
            )
            .changed();
        ui.label(
            RichText::new(t("avail_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });

    if settings_changed {
        app.save_settings();
    }
    if add {
        new_card(app, None);
    }
    if run {
        app.run_ppr_check();
    }
}

/// Yangi karta yaratadi va uni tanlaydi. `task` berilsa, darhol ishga bog'lanadi.
fn new_card(app: &mut App, task: Option<i64>) {
    let Some(pid) = app.current else { return };
    let name = task
        .and_then(|id| app.task(id).map(|x| x.name.clone()))
        .unwrap_or_else(|| t("ppr_new_name").to_string());
    let section = task
        .and_then(|id| app.task(id).map(|x| x.section))
        .unwrap_or(Section::None);
    let id = app.db.insert_ppr(&PprDoc {
        id: 0,
        project_id: pid,
        kind: PprKind::TechCard,
        number: String::new(),
        name,
        section,
        task_id: task,
        workers: 0,
        machines: 0,
        path: String::new(),
        approved: false,
        author: String::new(),
        approved_at: None,
        note: String::new(),
    });
    app.reload_modules();
    if id > 0 {
        app.selected_ppr = Some(id);
    }
}

// ================================================================ KPI

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let total = app.ppr_docs.len();
    let approved = app.ppr_docs.iter().filter(|d| d.approved).count();

    // Qoplanish: nechta GPR ishi uchun PPR yoki texnologik karta bor.
    let covered: std::collections::HashSet<i64> = app
        .ppr_docs
        .iter()
        .filter(|d| matches!(d.kind, PprKind::TechCard | PprKind::Ppr))
        .filter_map(|d| d.task_id)
        .collect();
    let tasks_n = app.tasks.len();
    let cov_n = app.tasks.iter().filter(|x| covered.contains(&x.id)).count();
    let cov_pct = if tasks_n == 0 {
        0.0
    } else {
        cov_n as f32 / tasks_n as f32 * 100.0
    };

    // Kritik yo'l qoplanishi — eng muhim ko'rsatkich.
    let crit: Vec<i64> = app
        .tasks
        .iter()
        .filter(|x| app.schedule.is_critical(x.id))
        .map(|x| x.id)
        .collect();
    let crit_cov = crit.iter().filter(|id| covered.contains(id)).count();
    let crit_pct = if crit.is_empty() {
        100.0
    } else {
        crit_cov as f32 / crit.len() as f32 * 100.0
    };

    // Resurs cho'qqisi.
    let wd = checks::resource_demand(&app.tasks, &app.schedule, &app.ppr_docs, false);
    let md = checks::resource_demand(&app.tasks, &app.schedule, &app.ppr_docs, true);
    let wpeak = wd.iter().copied().max().unwrap_or(0);
    let mpeak = md.iter().copied().max().unwrap_or(0);
    let aw = app.settings.avail_workers;
    let am = app.settings.avail_machines;

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_ppr_cards"),
                total.to_string(),
                &format!("{approved} {}", t("kpi_ppr_approved")),
                if total > 0 && approved == total {
                    theme::ok()
                } else {
                    theme::accent()
                },
            ),
            stat(
                t("kpi_ppr_coverage"),
                format!("{cov_pct:.0} %"),
                &format!("{cov_n} / {tasks_n} {}", t("tasks_short")),
                if cov_pct >= 99.0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("kpi_ppr_crit_cov"),
                format!("{crit_pct:.0} %"),
                &format!("{crit_cov} / {} {}", crit.len(), t("kpi_ppr_crit_tasks")),
                if crit_pct >= 99.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_ppr_peak_workers"),
                wpeak.to_string(),
                &if aw > 0 {
                    format!("{} {aw}", t("chk_ppr_have"))
                } else {
                    t("avail_not_set").to_string()
                },
                if aw > 0 && wpeak > aw {
                    theme::danger()
                } else {
                    theme::text()
                },
            ),
            stat(
                t("kpi_ppr_peak_machines"),
                mpeak.to_string(),
                &if am > 0 {
                    format!("{} {am}", t("chk_ppr_have"))
                } else {
                    t("avail_not_set").to_string()
                },
                if am > 0 && mpeak > am {
                    theme::danger()
                } else {
                    theme::text()
                },
            ),
        ],
    );
}

// ================================================================ Kartalar

fn cards_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.ppr_docs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(
                RichText::new(t("ppr_empty"))
                    .color(theme::muted())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("ppr_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    let mut edited: Option<PprDoc> = None;
    let mut removed: Option<i64> = None;
    let mut pick_file: Option<i64> = None;
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ppr_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 150.0, t("col_kind"));
                    head(ui, 84.0, t("col_number"));
                    head(ui, 210.0, t("col_name"));
                    head(ui, 56.0, t("col_section_short"));
                    head(ui, 220.0, t("col_task"));
                    head(ui, 150.0, t("col_ppr_author"));
                    head(ui, 60.0, t("col_workers"));
                    head(ui, 60.0, t("col_machines"));
                    head(ui, 128.0, t("col_approved"));
                    head(ui, 64.0, t("col_file"));
                    head(ui, 24.0, "");
                    ui.end_row();

                    // Kartalarni turi bo'yicha guruhlab chiqaramiz.
                    for kind in PprKind::ALL {
                        for doc in app.ppr_docs.iter().filter(|d| d.kind == *kind) {
                            let mut d = doc.clone();
                            let mut changed = false;

                            egui::ComboBox::from_id_salt(("ppr_kind", d.id))
                                .selected_text(d.kind.label())
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for k in PprKind::ALL {
                                        changed |= ui
                                            .selectable_value(&mut d.kind, *k, k.label())
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add_sized([84.0, 22.0], egui::TextEdit::singleline(&mut d.number))
                                .changed();
                            changed |= ui
                                .add_sized([210.0, 22.0], egui::TextEdit::singleline(&mut d.name))
                                .changed();
                            egui::ComboBox::from_id_salt(("ppr_sec", d.id))
                                .selected_text(d.section.label())
                                .width(56.0)
                                .show_ui(ui, |ui| {
                                    for s in Section::ALL {
                                        changed |= ui
                                            .selectable_value(&mut d.section, s, s.label())
                                            .changed();
                                    }
                                });
                            changed |=
                                task_picker(ui, app, ("ppr_task", d.id), &mut d.task_id, 220.0);
                            changed |= ui
                                .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut d.author))
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [60.0, 22.0],
                                    egui::DragValue::new(&mut d.workers).range(0..=5000),
                                )
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [60.0, 22.0],
                                    egui::DragValue::new(&mut d.machines).range(0..=500),
                                )
                                .changed();

                            // Tasdiq: belgilanganda sana avtomatik qo'yiladi.
                            ui.horizontal(|ui| {
                                if ui
                                    .checkbox(&mut d.approved, "")
                                    .on_hover_text(t("approved_short"))
                                    .changed()
                                {
                                    d.approved_at = d.approved.then_some(today);
                                    changed = true;
                                }
                                match d.approved_at {
                                    Some(date) => {
                                        ui.label(
                                            RichText::new(date.format("%d.%m.%y").to_string())
                                                .size(11.0)
                                                .color(theme::ok())
                                                .monospace(),
                                        );
                                    }
                                    None => {
                                        ui.label(
                                            RichText::new(t("not_approved_short"))
                                                .size(11.0)
                                                .color(theme::warn()),
                                        );
                                    }
                                }
                            });

                            if d.path.is_empty() {
                                if ui.small_button(t("attach")).clicked() {
                                    pick_file = Some(d.id);
                                }
                            } else if ui.small_button(t("open")).on_hover_text(&d.path).clicked() {
                                open_path(&d.path);
                            }
                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(d.id);
                            }
                            ui.end_row();

                            if changed {
                                edited = Some(d);
                            }
                        }
                    }
                });
        });

    if let Some(d) = edited {
        app.db.update_ppr(&d);
        if let Some(slot) = app.ppr_docs.iter_mut().find(|x| x.id == d.id) {
            *slot = d;
        }
    }
    if let Some(id) = removed {
        app.db.del("ppr", id);
        app.reload_modules();
    }
    if let Some(id) = pick_file {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Some(d) = app.ppr_docs.iter_mut().find(|x| x.id == id) {
                d.path = path.to_string_lossy().to_string();
                let copy = d.clone();
                app.db.update_ppr(&copy);
            }
        }
    }
}

// ================================================================ Resurslar

/// TZ I.3: «hatchayapti odam / texnika» savoliga vizual javob.
/// Kunlik talab gistogrammasi va mavjud quvvat chizig'i.
fn resources_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.tasks.is_empty() {
        ui.label(RichText::new(t("no_tasks")).color(theme::muted()));
        return;
    }
    let has_data = app.ppr_docs.iter().any(|d| d.workers > 0 || d.machines > 0);
    if !has_data {
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("res_no_data"))
                    .color(theme::muted())
                    .size(15.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("res_no_data_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    let avail_w = app.settings.avail_workers;
    let avail_m = app.settings.avail_machines;
    let w = (ui.available_width() - 26.0).min(1200.0);

    card_frame(ui, t("res_workers_title"), w, |ui| {
        histogram(ui, app, false, avail_w, 168.0);
    });
    ui.add_space(10.0);
    card_frame(ui, t("res_machines_title"), w, |ui| {
        histogram(ui, app, true, avail_m, 138.0);
    });
}

/// Kunlik resurs talabi gistogrammasi. Mavjud quvvatdan oshgan kunlar qizil.
fn histogram(ui: &mut egui::Ui, app: &App, machines: bool, avail: i64, height: f32) {
    let demand = checks::resource_demand(&app.tasks, &app.schedule, &app.ppr_docs, machines);
    if demand.is_empty() {
        return;
    }
    let origin = app.origin();
    let peak = demand.iter().copied().max().unwrap_or(0).max(avail).max(1);
    let over_days = if avail > 0 {
        demand.iter().filter(|v| **v > avail).count()
    } else {
        0
    };

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);
    let plot = Rect::from_min_max(
        pos2(rect.min.x + 34.0, rect.min.y + 8.0),
        pos2(rect.max.x - 8.0, rect.max.y - 20.0),
    );

    // To'r va o'q belgilari.
    for k in 0..=2 {
        let v = peak * k / 2;
        let gy = plot.max.y - plot.height() * (v as f32 / peak as f32);
        p.line_segment(
            [pos2(plot.min.x, gy), pos2(plot.max.x, gy)],
            Stroke::new(0.5_f32, theme::row_line()),
        );
        p.text(
            pos2(plot.min.x - 6.0, gy),
            Align2::RIGHT_CENTER,
            v.to_string(),
            egui::FontId::proportional(9.5),
            theme::muted(),
        );
    }

    // Ustunlar: har bir kun uchun bitta tik chiziq.
    let n = demand.len() as f32;
    let bw = (plot.width() / n).max(1.0);
    for (i, v) in demand.iter().enumerate() {
        if *v <= 0 {
            continue;
        }
        let x = plot.min.x + plot.width() * (i as f32 / n);
        let h = plot.height() * (*v as f32 / peak as f32);
        let over = avail > 0 && *v > avail;
        p.rect_filled(
            Rect::from_min_size(pos2(x, plot.max.y - h), vec2(bw.max(1.2), h)),
            0.0,
            if over {
                theme::danger()
            } else if machines {
                theme::warn().gamma_multiply(0.75)
            } else {
                theme::accent().gamma_multiply(0.75)
            },
        );
    }

    // Mavjud quvvat chizig'i.
    if avail > 0 {
        let ay = plot.max.y - plot.height() * (avail as f32 / peak as f32);
        let mut x = plot.min.x;
        while x < plot.max.x {
            let x2 = (x + 6.0).min(plot.max.x);
            p.line_segment(
                [pos2(x, ay), pos2(x2, ay)],
                Stroke::new(1.6_f32, theme::ok()),
            );
            x += 11.0;
        }
        p.text(
            pos2(plot.max.x - 4.0, ay - 8.0),
            Align2::RIGHT_CENTER,
            format!("{} {avail}", t("chk_ppr_have")),
            egui::FontId::proportional(10.0),
            theme::ok(),
        );
    }

    // Bugun.
    let today_off = (app.today - origin).num_days();
    if today_off >= 0 && (today_off as usize) < demand.len() {
        let tx = plot.min.x + plot.width() * (today_off as f32 / n);
        p.line_segment(
            [pos2(tx, plot.min.y), pos2(tx, plot.max.y)],
            Stroke::new(1.2_f32, theme::warn().gamma_multiply(0.8)),
        );
    }

    // Oy belgilari.
    let mut d = NaiveDate::from_ymd_opt(origin.year(), origin.month(), 1).unwrap_or(origin);
    let mut last_x = f32::MIN;
    while (d - origin).num_days() < demand.len() as i64 {
        let off = (d - origin).num_days();
        if off >= 0 {
            let mx = plot.min.x + plot.width() * (off as f32 / n);
            if mx <= plot.max.x - 14.0 && mx - last_x >= 44.0 {
                p.text(
                    pos2(mx + 2.0, plot.max.y + 10.0),
                    Align2::LEFT_CENTER,
                    crate::i18n::month(d.month()).to_string(),
                    egui::FontId::proportional(9.5),
                    theme::muted(),
                );
                last_x = mx;
            }
        }
        d = next_month(d);
    }

    // Xulosa satri.
    ui.add_space(2.0);
    if avail <= 0 {
        ui.label(
            RichText::new(t("res_set_capacity"))
                .size(11.0)
                .color(theme::muted()),
        );
    } else if over_days > 0 {
        ui.label(
            RichText::new(format!(
                "{} {} — {} {}",
                over_days,
                t("chk_ppr_over_days"),
                t("res_peak"),
                demand.iter().copied().max().unwrap_or(0)
            ))
            .size(11.5)
            .color(theme::danger()),
        );
    } else {
        ui.label(RichText::new(t("res_enough")).size(11.5).color(theme::ok()));
    }
}

fn next_month(d: NaiveDate) -> NaiveDate {
    let (y, m) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

// ================================================================ Qoplanish

/// TZ I.3: «PPR loyihaga mos keladimi» — qaysi ishda karta bor, qaysida yo'q.
fn coverage_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.tasks.is_empty() {
        ui.label(RichText::new(t("no_tasks")).color(theme::muted()));
        return;
    }
    let mut create_for: Option<i64> = None;
    let origin = app.origin();

    ui.label(
        RichText::new(t("coverage_hint"))
            .size(11.5)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("cov_grid")
                .num_columns(7)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                        ui.add_sized(
                            [w, 16.0],
                            egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                        );
                    };
                    head(ui, 34.0, t("col_num"));
                    head(ui, 250.0, t("col_task"));
                    head(ui, 50.0, t("col_section_short"));
                    head(ui, 74.0, t("col_start_short"));
                    head(ui, 80.0, t("col_critical_short"));
                    head(ui, 260.0, t("col_ppr_card"));
                    head(ui, 90.0, "");
                    ui.end_row();

                    for task in &app.tasks {
                        let cards: Vec<&PprDoc> = app
                            .ppr_docs
                            .iter()
                            .filter(|d| d.task_id == Some(task.id))
                            .collect();
                        let main = cards
                            .iter()
                            .find(|d| matches!(d.kind, PprKind::TechCard | PprKind::Ppr));
                        let critical = app.schedule.is_critical(task.id);

                        ui.add_sized(
                            [34.0, 18.0],
                            egui::Label::new(
                                RichText::new(&task.wbs).size(11.5).color(theme::muted()),
                            ),
                        );
                        ui.add_sized(
                            [250.0, 18.0],
                            egui::Label::new(
                                RichText::new(issues::truncate(&task.name, 32)).size(12.5),
                            ),
                        );
                        ui.add_sized(
                            [50.0, 18.0],
                            egui::Label::new(
                                RichText::new(task.section.label())
                                    .size(11.5)
                                    .color(theme::section_color(task.section.color())),
                            ),
                        );
                        let start = app
                            .schedule
                            .get(task.id)
                            .map(|c| {
                                (origin + Duration::days(c.es))
                                    .format("%d.%m.%y")
                                    .to_string()
                            })
                            .unwrap_or_default();
                        ui.add_sized(
                            [74.0, 18.0],
                            egui::Label::new(
                                RichText::new(start)
                                    .size(11.5)
                                    .color(theme::muted())
                                    .monospace(),
                            ),
                        );
                        ui.add_sized(
                            [80.0, 18.0],
                            egui::Label::new(
                                RichText::new(if critical { t("crit_short") } else { "" })
                                    .size(11.5)
                                    .color(theme::danger()),
                            ),
                        );

                        // Karta holati: yo'q / tasdiqlanmagan / tasdiqlangan.
                        match main {
                            None => {
                                ui.add_sized(
                                    [260.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(t("cov_no_card")).size(12.0).color(
                                            if critical {
                                                theme::danger()
                                            } else {
                                                theme::muted()
                                            },
                                        ),
                                    ),
                                );
                                if ui
                                    .small_button(t("cov_create"))
                                    .on_hover_text(t("cov_create_hint"))
                                    .clicked()
                                {
                                    create_for = Some(task.id);
                                }
                            }
                            Some(card) => {
                                ui.horizontal(|ui| {
                                    let (icon, _) =
                                        ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                    let painter = ui.painter();
                                    if card.approved {
                                        super::draw_check(
                                            painter,
                                            icon.center(),
                                            4.5,
                                            theme::ok(),
                                            1.8,
                                        );
                                    } else {
                                        painter.circle_stroke(
                                            icon.center(),
                                            5.0,
                                            Stroke::new(1.3_f32, theme::warn()),
                                        );
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "{} {}",
                                            card.number,
                                            issues::truncate(&card.name, 22)
                                        ))
                                        .size(12.0)
                                        .color(
                                            if card.approved {
                                                theme::text()
                                            } else {
                                                theme::warn()
                                            },
                                        ),
                                    );
                                    // Qo'shimcha kartalar soni.
                                    if cards.len() > 1 {
                                        ui.label(
                                            RichText::new(format!("+{}", cards.len() - 1))
                                                .size(10.5)
                                                .color(theme::muted()),
                                        );
                                    }
                                });
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some(id) = create_for {
        new_card(app, Some(id));
    }
}

// ================================================================ Nomuvofiqliklar

fn issues_tab(ui: &mut egui::Ui, app: &mut App) {
    if issues::filter_bar(ui, app, t("run_ppr_check")) {
        app.run_ppr_check();
    }
    ui.add_space(6.0);

    let list = app.filtered_issues(IssueModule::Ppr);
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
            if let Some(id) = issues::issue_table(ui, app, IssueModule::Ppr, h) {
                app.selected_issue = Some(id);
            }
        });
        ui.vertical(|ui| {
            ui.set_width(detail_w);
            issues::issue_detail(ui, app, h - 24.0);
        });
    });
}
