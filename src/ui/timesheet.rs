//! «Tabel» ekrani (TZ XIII).
//!
//! Uch ko'rinish: **tabel** (haftalik jadval — qatorlar ishchilar, ustunlar
//! kunlar), **brigadalar** (guruhlarni solishtirish) va **tannarx** (har bir
//! ishning ish haqi va material qiymati).
//!
//! Katakda nima tahrirlanishi yuqoridagi tugmalar bilan almashadi: soat,
//! kun turi (bo'sh turish, ta'til, kasallik), smena yoki qaysi ish. Shunday
//! qilinganining sababi: yetti kunlik ustunga to'rt xil maydonni birdaniga
//! sig'dirib bo'lmaydi, almashtirish esa jadvalni tor ekranda ham saqlaydi.
//!
//! Ish haqi soat, smena va kun turidan hisoblanadi — alohida kiritilmaydi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::checks::{NORM_HOURS, OVERTIME_RATE};
use crate::domain::{Brigade, DayKind, Shift, Worker};
use chrono::{Datelike, Duration, NaiveDate};

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

    let week = *app.timesheet_week.get_or_insert(monday_of(app.today));
    let mut add = false;
    let mut add_brigade = false;
    let mut move_week = 0i64;
    let mut to_today = false;

    let tab_key = egui::Id::new("ts_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_worker")).clicked() {
            add = true;
        }
        if ui
            .button(t("add_brigade"))
            .on_hover_text(t("add_brigade_hint"))
            .clicked()
        {
            add_brigade = true;
        }
        ui.add_space(10.0);
        if ui.button("<").on_hover_text(t("prev_week")).clicked() {
            move_week = -1;
        }
        ui.label(
            RichText::new(format!(
                "{} — {}",
                week.format("%d.%m.%Y"),
                (week + Duration::days(6)).format("%d.%m.%Y")
            ))
            .size(13.0)
            .strong(),
        );
        if ui.button(">").on_hover_text(t("next_week")).clicked() {
            move_week = 1;
        }
        if ui.button(t("this_week")).clicked() {
            to_today = true;
        }
    });
    ui.label(
        RichText::new(t("timesheet_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);

    kpi_row(ui, app, week);
    ui.add_space(10.0);

    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("ts_tab_sheet")),
            (1, t("ts_tab_brigades")),
            (2, t("ts_tab_cost")),
            (3, t("ts_tab_periods")),
            (4, t("ts_tab_staff")),
            (5, t("ts_tab_objects")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => brigades_tab(ui, app, week),
        2 => cost_tab(ui, app),
        3 => periods_tab(ui, app),
        4 => staff_tab(ui, app),
        5 => objects_tab(ui, app),
        _ => {
            if app.workers.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t("workers_empty"))
                            .color(theme::muted())
                            .size(15.0),
                    );
                });
            } else {
                sheet_tab(ui, app, week);
            }
        }
    }

    if add {
        let n = app.workers.len() + 1;
        app.db.insert_worker(&Worker {
            id: 0,
            project_id: pid,
            name: format!("{} {n}", t("worker_new_name")),
            position: String::new(),
            org: String::new(),
            hourly_rate: 0.0,
            active: true,
            brigade_id: None,
        });
        app.reload_modules();
    }
    if add_brigade {
        let n = app.brigades.len() + 1;
        app.db.insert_brigade(&Brigade {
            id: 0,
            project_id: pid,
            name: format!("{n}-{}", t("brigade_new_name")),
            foreman: String::new(),
            task_id: None,
            note: String::new(),
        });
        app.reload_modules();
        ui.data_mut(|d| d.insert_temp(tab_key, 1u8));
    }
    if move_week != 0 {
        app.timesheet_week = Some(week + Duration::days(7 * move_week));
    }
    if to_today {
        app.timesheet_week = Some(monday_of(app.today));
    }
}

/// Berilgan sanadagi haftaning dushanbasi.
fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

fn kpi_row(ui: &mut egui::Ui, app: &App, week: NaiveDate) {
    let end = week + Duration::days(6);
    let lines = crate::checks::wages(&app.workers, &app.timesheet, week, end);

    let hours: f64 = lines.iter().map(|l| l.hours).sum();
    let payroll: f64 = lines.iter().map(|l| l.wage).sum();
    let overtime: f64 = lines.iter().map(|l| l.overtime_hours).sum();
    let downtime: f64 = lines.iter().map(|l| l.downtime_hours).sum();
    let absences: i64 = lines.iter().map(|l| l.absence_days).sum();
    let active = app.workers.iter().filter(|w| w.active).count();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_workers"),
                active.to_string(),
                &format!("{} {}", app.workers.len(), t("kpi_workers_hint")),
                theme::accent(),
            ),
            stat(
                t("kpi_hours_week"),
                super::materials::trim_num(hours),
                t("kpi_hours_week_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_overtime"),
                super::materials::trim_num(overtime),
                t("kpi_overtime_hint"),
                if overtime > 0.0 {
                    theme::warn()
                } else {
                    theme::muted()
                },
            ),
            stat(
                t("kpi_downtime"),
                super::materials::trim_num(downtime),
                t("kpi_downtime_hint"),
                if downtime > 0.0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("kpi_absences"),
                absences.to_string(),
                t("kpi_absences_hint"),
                if absences > 0 {
                    theme::warn()
                } else {
                    theme::muted()
                },
            ),
            stat(
                t("kpi_payroll"),
                money(payroll),
                t("kpi_payroll_hint"),
                theme::ok(),
            ),
        ],
    );
}

// ================================================================ Tabel

/// Katakda nima tahrirlanadi.
#[derive(Clone, Copy, PartialEq)]
enum CellMode {
    Hours,
    Kind,
    Shift,
    Task,
}

fn sheet_tab(ui: &mut egui::Ui, app: &mut App, week: NaiveDate) {
    let Some(pid) = app.current else { return };
    let today = app.today;
    let days: Vec<NaiveDate> = (0..7).map(|i| week + Duration::days(i)).collect();

    let mode_key = egui::Id::new("ts_cell_mode");
    let mut mode = match ui.data(|d| d.get_temp::<u8>(mode_key)).unwrap_or(0) {
        1 => CellMode::Kind,
        2 => CellMode::Shift,
        3 => CellMode::Task,
        _ => CellMode::Hours,
    };

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(t("cell_shows"))
                .size(11.5)
                .color(theme::muted()),
        );
        for (m, label) in [
            (CellMode::Hours, t("cell_hours")),
            (CellMode::Kind, t("cell_kind")),
            (CellMode::Shift, t("cell_shift")),
            (CellMode::Task, t("cell_task")),
        ] {
            if ui.selectable_label(mode == m, label).clicked() {
                mode = m;
            }
        }
        // Koeffitsiyentlar yashirin qolmasin — hisob qanday chiqqani ko'rinsin.
        ui.label(
            RichText::new(format!(
                "{} ×{OVERTIME_RATE}, {} ×{}, {} ×{}",
                t("cell_overtime"),
                t("sh_evening"),
                Shift::Evening.rate(),
                t("sh_night"),
                Shift::Night.rate(),
            ))
            .size(11.0)
            .color(theme::muted()),
        );
    });
    ui.data_mut(|d| {
        d.insert_temp(
            mode_key,
            match mode {
                CellMode::Hours => 0u8,
                CellMode::Kind => 1,
                CellMode::Shift => 2,
                CellMode::Task => 3,
            },
        )
    });
    ui.add_space(6.0);

    // Tor ekranda brigada va lavozim ustunlari yashiriladi: kunlik ustunlar
    // muhimroq, ular gorizontal aylantirishning narigi chetiga tushib qolmasin.
    let wide = ui.available_width() > 1300.0;
    let wages = crate::checks::wages(&app.workers, &app.timesheet, week, days[6]);
    let mut edited: Option<Worker> = None;
    let mut removed: Option<i64> = None;
    // Katakka yozish: (ishchi, kun, nima)
    let mut set_hours: Option<(i64, NaiveDate, f64)> = None;
    let mut set_kind: Option<(i64, NaiveDate, DayKind)> = None;
    let mut set_shift: Option<(i64, NaiveDate, Shift)> = None;
    let mut set_task: Option<(i64, NaiveDate, Option<i64>)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("timesheet_grid")
                .num_columns(if wide { 14 } else { 12 })
                .spacing([6.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 190.0, t("col_worker"));
                    if wide {
                        head_l(ui, 150.0, t("col_brigade"));
                        head_l(ui, 130.0, t("col_position"));
                    }
                    head_r(ui, 105.0, t("col_hourly_rate"));
                    for d in &days {
                        let weekend = d.weekday().num_days_from_monday() >= 5;
                        let color = if *d == today {
                            theme::accent()
                        } else if weekend {
                            theme::warn()
                        } else {
                            theme::muted()
                        };
                        // Ikki qatorli sarlavha: kun nomi va sanasi. `cell_r`
                        // matnni qisqartirgani uchun o'zimiz chizamiz.
                        ui.allocate_ui_with_layout(
                            egui::vec2(58.0, 26.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(58.0);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(RichText::new(t(weekday_key(*d))).size(11.0).color(color));
                                ui.label(
                                    RichText::new(d.format("%d.%m").to_string())
                                        .size(9.5)
                                        .color(theme::muted()),
                                );
                            },
                        );
                    }
                    head_r(ui, 70.0, t("col_total_hours"));
                    head_r(ui, 130.0, t("col_wage"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.workers {
                        let mut w = src.clone();
                        let mut changed = false;

                        // Faol bo'lmagan ishchi belgisi olib tashlanadi.
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut w.active, "").changed();
                            changed |= ui
                                .add_sized([160.0, 22.0], egui::TextEdit::singleline(&mut w.name))
                                .changed();
                        });
                        if wide {
                            changed |= brigade_picker(ui, app, w.id, &mut w.brigade_id, 150.0);
                            changed |= ui
                                .add_sized(
                                    [130.0, 22.0],
                                    egui::TextEdit::singleline(&mut w.position),
                                )
                                .changed();
                        }
                        changed |= ui
                            .add_sized(
                                [105.0, 22.0],
                                egui::DragValue::new(&mut w.hourly_rate)
                                    .speed(1000.0)
                                    .range(0.0..=1e9),
                            )
                            .changed();

                        let mut total = 0.0;
                        for d in &days {
                            let e = app
                                .timesheet
                                .iter()
                                .find(|e| e.worker_id == w.id && e.date == *d);
                            let hours = e.map(|e| e.hours).unwrap_or(0.0);
                            let kind = e.map(|e| e.kind).unwrap_or(DayKind::Work);
                            let shift = e.map(|e| e.shift).unwrap_or(Shift::Day);
                            let task = e.and_then(|e| e.task_id);
                            total += hours;

                            match mode {
                                CellMode::Hours => {
                                    let mut v = hours;
                                    let resp = ui.add_sized(
                                        [58.0, 22.0],
                                        egui::DragValue::new(&mut v).speed(0.5).range(0.0..=24.0),
                                    );
                                    if resp.changed() {
                                        set_hours = Some((w.id, *d, v));
                                    }
                                    // Kun turi oddiy ish bo'lmasa — buni aytamiz,
                                    // aks holda soat sababsiz kam ko'rinardi.
                                    if kind != DayKind::Work {
                                        resp.on_hover_text(kind.label());
                                    } else if hours > NORM_HOURS {
                                        resp.on_hover_text(t("overtime_hint"));
                                    }
                                }
                                CellMode::Kind => {
                                    let mut v = kind;
                                    let mut hit = false;
                                    egui::ComboBox::from_id_salt(("ts_k", w.id, *d))
                                        .selected_text(
                                            RichText::new(short_kind(v))
                                                .size(11.5)
                                                .color(kind_color(v)),
                                        )
                                        .width(58.0)
                                        .show_ui(ui, |ui| {
                                            for k in DayKind::ALL {
                                                hit |= ui
                                                    .selectable_value(&mut v, *k, k.label())
                                                    .changed();
                                            }
                                        });
                                    if hit {
                                        set_kind = Some((w.id, *d, v));
                                    }
                                }
                                CellMode::Shift => {
                                    let mut v = shift;
                                    let mut hit = false;
                                    egui::ComboBox::from_id_salt(("ts_s", w.id, *d))
                                        .selected_text(
                                            RichText::new(short_shift(v)).size(11.5).color(
                                                if v == Shift::Day {
                                                    theme::muted()
                                                } else {
                                                    theme::accent()
                                                },
                                            ),
                                        )
                                        .width(58.0)
                                        .show_ui(ui, |ui| {
                                            for k in Shift::ALL {
                                                hit |= ui
                                                    .selectable_value(
                                                        &mut v,
                                                        *k,
                                                        format!("{} ×{}", k.label(), k.rate()),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    if hit {
                                        set_shift = Some((w.id, *d, v));
                                    }
                                }
                                CellMode::Task => {
                                    let mut v = task;
                                    let mut hit = false;
                                    let label = v
                                        .and_then(|id| app.task(id).map(|x| x.wbs.clone()))
                                        .unwrap_or_else(|| t("dash").to_string());
                                    egui::ComboBox::from_id_salt(("ts_t", w.id, *d))
                                        .selected_text(RichText::new(label).size(11.5))
                                        .width(58.0)
                                        .show_ui(ui, |ui| {
                                            hit |= ui
                                                .selectable_value(&mut v, None, t("no_task"))
                                                .changed();
                                            for task in &app.tasks {
                                                hit |= ui
                                                    .selectable_value(
                                                        &mut v,
                                                        Some(task.id),
                                                        format!("{} {}", task.wbs, task.name),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    if hit {
                                        set_task = Some((w.id, *d, v));
                                    }
                                }
                            }
                        }

                        cell_r(
                            ui,
                            70.0,
                            RichText::new(super::materials::trim_num(total))
                                .size(12.5)
                                .strong(),
                        );
                        let wage = wages
                            .iter()
                            .find(|l| l.worker_id == w.id)
                            .map(|l| l.wage)
                            .unwrap_or(0.0);
                        cell_r(ui, 130.0, RichText::new(money(wage)).size(12.0));
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(w.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(w);
                        }
                    }

                    // Kun bo'yicha yig'indi.
                    cell_l(
                        ui,
                        190.0,
                        RichText::new(t("total_row"))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                    if wide {
                        cell_l(ui, 150.0, RichText::new(""));
                        cell_l(ui, 130.0, RichText::new(""));
                    }
                    cell_l(ui, 105.0, RichText::new(""));
                    let mut week_total = 0.0;
                    for d in &days {
                        let sum: f64 = app
                            .timesheet
                            .iter()
                            .filter(|e| e.date == *d)
                            .map(|e| e.hours)
                            .sum();
                        week_total += sum;
                        cell_r(
                            ui,
                            58.0,
                            RichText::new(super::materials::trim_num(sum))
                                .size(11.5)
                                .color(theme::muted()),
                        );
                    }
                    cell_r(
                        ui,
                        70.0,
                        RichText::new(super::materials::trim_num(week_total))
                            .size(12.5)
                            .strong(),
                    );
                    cell_r(
                        ui,
                        130.0,
                        RichText::new(money(wages.iter().map(|l| l.wage).sum::<f64>()))
                            .size(12.0)
                            .strong(),
                    );
                    ui.end_row();
                });
        });

    if let Some((worker, day, hours)) = set_hours {
        app.db.set_timesheet(pid, worker, day, hours);
        app.timesheet = app.db.timesheet(pid);
    }
    if let Some((worker, day, kind)) = set_kind {
        app.db.set_timesheet_kind(pid, worker, day, kind);
        app.timesheet = app.db.timesheet(pid);
    }
    if let Some((worker, day, shift)) = set_shift {
        app.db.set_timesheet_shift(pid, worker, day, shift);
        app.timesheet = app.db.timesheet(pid);
    }
    if let Some((worker, day, task)) = set_task {
        app.db.set_timesheet_task(pid, worker, day, task);
        app.timesheet = app.db.timesheet(pid);
    }
    if let Some(w) = edited {
        app.db.update_worker(&w);
        if let Some(slot) = app.workers.iter_mut().find(|x| x.id == w.id) {
            *slot = w;
        }
    }
    if let Some(id) = removed {
        app.db.del("worker", id);
        app.reload_modules();
    }
}

// ================================================================ Brigadalar

fn brigades_tab(ui: &mut egui::Ui, app: &mut App, week: NaiveDate) {
    if app.brigades.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("brigades_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    ui.label(
        RichText::new(t("brigades_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    let lines = crate::checks::brigade_lines(
        &app.brigades,
        &app.workers,
        &app.timesheet,
        week,
        week + Duration::days(6),
    );
    let mut edited: Option<Brigade> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ts_brigades")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 220.0, t("col_brigade"));
                    head_l(ui, 170.0, t("col_foreman"));
                    head_l(ui, 230.0, t("col_task"));
                    head_r(ui, 80.0, t("col_people"));
                    head_r(ui, 90.0, t("col_total_hours"));
                    head_r(ui, 90.0, t("col_downtime"));
                    head_r(ui, 80.0, "%");
                    head_r(ui, 130.0, t("col_wage"));
                    head_r(ui, 120.0, t("col_cost_per_hour"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.brigades {
                        let mut b = src.clone();
                        let mut changed = false;
                        let l = lines.iter().find(|l| l.brigade_id == b.id);

                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut b.name))
                            .changed();
                        changed |= ui
                            .add_sized([170.0, 22.0], egui::TextEdit::singleline(&mut b.foreman))
                            .changed();
                        changed |= task_picker(ui, app, ("bg_task", b.id), &mut b.task_id, 230.0);

                        let num = |v: f64| super::materials::trim_num(v);
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(l.map(|l| l.workers).unwrap_or(0).to_string()).size(12.0),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(num(l.map(|l| l.hours).unwrap_or(0.0)))
                                .size(12.0)
                                .strong(),
                        );
                        let down = l.map(|l| l.downtime_hours).unwrap_or(0.0);
                        let pct = l.map(|l| l.downtime_pct).unwrap_or(0.0);
                        let color = if pct > 10.0 {
                            theme::danger()
                        } else if pct > 0.0 {
                            theme::warn()
                        } else {
                            theme::muted()
                        };
                        cell_r(ui, 90.0, RichText::new(num(down)).size(12.0).color(color));
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(format!("{pct:.0}%")).size(11.5).color(color),
                        );
                        cell_r(
                            ui,
                            130.0,
                            RichText::new(money(l.map(|l| l.wage).unwrap_or(0.0))).size(12.0),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(money(l.map(|l| l.cost_per_hour).unwrap_or(0.0)))
                                .size(11.5)
                                .color(theme::muted()),
                        );
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(b.id);
                        }
                        ui.end_row();
                        if changed {
                            edited = Some(b);
                        }
                    }
                });
        });

    if let Some(b) = edited {
        app.db.update_brigade(&b);
        app.reload_modules();
    }
    if let Some(id) = removed {
        // Brigada o'chsa ishchilar qolaveradi — ular brigadasiz bo'lib qoladi.
        app.db.del("brigade", id);
        app.reload_modules();
    }
}

// ================================================================ Tannarx

fn cost_tab(ui: &mut egui::Ui, app: &mut App) {
    let lines = crate::checks::task_costs(
        &app.tasks,
        &app.workers,
        &app.timesheet,
        &app.materials,
        &app.stock_moves,
        &app.machines,
        &app.machine_logs,
    );
    if lines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("cost_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let labour: f64 = lines.iter().map(|l| l.labour).sum();
    let material: f64 = lines.iter().map(|l| l.material).sum();
    let machine: f64 = lines.iter().map(|l| l.machine).sum();
    stat_row(
        ui,
        vec![
            stat(
                t("kpi_labour_cost"),
                money(labour),
                t("kpi_labour_cost_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_material_cost"),
                money(material),
                t("kpi_material_cost_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_task_machine"),
                money(machine),
                t("kpi_task_machine_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_total_cost"),
                money(labour + material + machine),
                t("kpi_total_cost_hint"),
                theme::ok(),
            ),
        ],
    );
    ui.add_space(10.0);
    ui.label(
        RichText::new(t("cost_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("ts_costs")
                .num_columns(8)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 300.0, t("col_task"));
                    head_r(ui, 90.0, t("col_total_hours"));
                    head_r(ui, 140.0, t("col_labour"));
                    head_r(ui, 150.0, t("col_material_cost"));
                    head_r(ui, 140.0, t("col_machine_cost"));
                    head_r(ui, 150.0, t("col_total"));
                    head_r(ui, 130.0, t("col_per_volume"));
                    head_l(ui, 60.0, t("col_unit"));
                    ui.end_row();

                    for l in &lines {
                        let task = app.task(l.task_id);
                        let name = task
                            .map(|x| format!("{} {}", x.wbs, x.name))
                            .unwrap_or_default();
                        cell_l(
                            ui,
                            300.0,
                            RichText::new(super::issues::truncate(&name, 44)).size(12.0),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(super::materials::trim_num(l.hours))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(ui, 140.0, RichText::new(money(l.labour)).size(12.0));
                        cell_r(ui, 150.0, RichText::new(money(l.material)).size(12.0));
                        cell_r(
                            ui,
                            140.0,
                            RichText::new(money(l.machine))
                                .size(12.0)
                                .color(if l.machine > 0.0 {
                                    theme::text()
                                } else {
                                    theme::muted()
                                }),
                        );
                        cell_r(ui, 150.0, RichText::new(money(l.total)).size(12.5).strong());
                        cell_r(
                            ui,
                            130.0,
                            RichText::new(if l.per_unit > 0.0 {
                                money(l.per_unit)
                            } else {
                                t("dash").to_string()
                            })
                            .size(11.5)
                            .color(theme::muted()),
                        );
                        cell_l(
                            ui,
                            60.0,
                            RichText::new(task.map(|x| x.unit.clone()).unwrap_or_default())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Yordamchilar

/// Brigada tanlash ro'yxati.
fn brigade_picker(
    ui: &mut egui::Ui,
    app: &App,
    salt: i64,
    cur: &mut Option<i64>,
    width: f32,
) -> bool {
    let mut changed = false;
    let label = cur
        .and_then(|id| app.brigades.iter().find(|b| b.id == id))
        .map(|b| b.name.clone())
        .unwrap_or_else(|| t("no_brigade").to_string());
    egui::ComboBox::from_id_salt(("ts_bg", salt))
        .selected_text(super::issues::truncate(&label, 20))
        .width(width)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(cur, None, t("no_brigade")).changed();
            for b in &app.brigades {
                changed |= ui.selectable_value(cur, Some(b.id), &b.name).changed();
            }
        });
    changed
}

/// Kun turining tor ustunga sig'adigan qisqa belgisi.
fn short_kind(k: DayKind) -> &'static str {
    match k {
        DayKind::Work => t("dks_work"),
        DayKind::Downtime => t("dks_downtime"),
        DayKind::Vacation => t("dks_vacation"),
        DayKind::Sick => t("dks_sick"),
        DayKind::Trip => t("dks_trip"),
        DayKind::Absent => t("dks_absent"),
    }
}

fn kind_color(k: DayKind) -> egui::Color32 {
    match k {
        DayKind::Work => theme::muted(),
        DayKind::Downtime | DayKind::Absent => theme::danger(),
        DayKind::Sick | DayKind::Vacation => theme::warn(),
        DayKind::Trip => theme::accent(),
    }
}

fn short_shift(s: Shift) -> &'static str {
    match s {
        Shift::Day => t("shs_day"),
        Shift::Evening => t("shs_evening"),
        Shift::Night => t("shs_night"),
    }
}

/// Hafta kunining tarjima kaliti.
fn weekday_key(d: NaiveDate) -> &'static str {
    match d.weekday().num_days_from_monday() {
        0 => "wd_mon",
        1 => "wd_tue",
        2 => "wd_wed",
        3 => "wd_thu",
        4 => "wd_fri",
        5 => "wd_sat",
        _ => "wd_sun",
    }
}

// ================================================================ Davrlar

/// Tabel davrlari: oyni yopish va tuzatish (TZ XIII.35-36).
///
/// Yopilgan oy tasodifan o'zgarmaydi. Tuzatish kerak bo'lsa — davr
/// qaytadan ochiladi va **sababi yoziladi**: ish haqi hisoblangandan
/// keyingi o'zgarish izsiz qolmasligi kerak.
fn periods_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else { return };
    let can = app.can_edit(Screen::Timesheet);
    let today = app.today;

    ui.label(
        RichText::new(t("ts_periods_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    // Tabelda uchraydigan oylar: davr yozuvi bo'lmasa ham ular ko'rinadi.
    let mut months: Vec<chrono::NaiveDate> = app
        .timesheet
        .iter()
        .map(|e| crate::analytics::month_of(e.date))
        .collect();
    months.sort_unstable();
    months.dedup();
    months.reverse();

    if months.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("ts_periods_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let periods = app.db.timesheet_periods(pid);
    let mut close: Option<chrono::NaiveDate> = None;
    let mut reopen: Option<i64> = None;

    egui::Grid::new("ts_periods_grid")
        .num_columns(6)
        .spacing([12.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            head_l(ui, 120.0, t("ts_period_month"));
            head_r(ui, 90.0, t("ts_period_days"));
            head_r(ui, 100.0, t("ts_period_hours"));
            head_r(ui, 150.0, t("ts_period_wage"));
            head_l(ui, 200.0, t("col_status"));
            head_l(ui, 140.0, "");
            ui.end_row();

            for m in &months {
                let next = crate::analytics::month_of(*m + chrono::Duration::days(32));
                let rows: Vec<&crate::domain::TimesheetEntry> = app
                    .timesheet
                    .iter()
                    .filter(|e| e.date >= *m && e.date < next)
                    .collect();
                let hours: f64 = rows.iter().map(|e| e.hours).sum();
                let wage: f64 = rows
                    .iter()
                    .map(|e| {
                        let rate = app
                            .workers
                            .iter()
                            .find(|w| w.id == e.worker_id)
                            .map(|w| w.hourly_rate)
                            .unwrap_or(0.0);
                        e.hours * rate * e.shift.rate()
                    })
                    .sum();
                let mut days: Vec<chrono::NaiveDate> = rows.iter().map(|e| e.date).collect();
                days.sort_unstable();
                days.dedup();

                let period = periods.iter().find(|p| p.month == *m);
                let closed = period.is_some_and(|p| p.closed);

                cell_l(
                    ui,
                    120.0,
                    RichText::new(m.format("%m.%Y").to_string())
                        .size(12.5)
                        .strong(),
                );
                cell_r(ui, 90.0, RichText::new(days.len().to_string()).size(12.0));
                cell_r(
                    ui,
                    100.0,
                    RichText::new(super::materials::trim_num(hours)).size(12.0),
                );
                cell_r(ui, 150.0, RichText::new(money(wage)).size(12.0));
                cell_l(
                    ui,
                    200.0,
                    match period {
                        Some(p) if p.closed => RichText::new(format!(
                            "{} · {}",
                            t("ts_period_closed"),
                            p.closed_at
                                .map(|d| d.format("%d.%m.%Y").to_string())
                                .unwrap_or_default()
                        ))
                        .size(11.5)
                        .color(theme::ok()),
                        _ => RichText::new(t("ts_period_open"))
                            .size(11.5)
                            .color(theme::warn()),
                    },
                );
                ui.horizontal(|ui| {
                    if !closed {
                        if can && ui.small_button(t("ts_period_close")).clicked() {
                            close = Some(*m);
                        }
                    } else if can && ui.small_button(t("ts_period_reopen")).clicked() {
                        reopen = period.map(|p| p.id);
                    }
                });
                ui.end_row();
            }
        });

    if let Some(month) = close {
        let existing = periods.iter().find(|p| p.month == month).cloned();
        match existing {
            Some(mut p) => {
                p.closed = true;
                p.closed_at = Some(today);
                p.closed_by = app.current_user_name();
                app.db.update_timesheet_period(&p);
            }
            None => {
                app.db
                    .insert_timesheet_period(&crate::domain::TimesheetPeriod {
                        id: 0,
                        project_id: pid,
                        month,
                        closed: true,
                        closed_at: Some(today),
                        closed_by: app.current_user_name(),
                        reopen_reason: String::new(),
                        note: String::new(),
                    });
            }
        }
        app.notify(t("ts_period_closed_msg").to_string());
    }
    if let Some(id) = reopen {
        if let Some(mut p) = periods.into_iter().find(|p| p.id == id) {
            p.closed = false;
            p.closed_at = None;
            // Sabab bo'sh qolmasin: uni odam yozadi, lekin belgisi qoladi.
            p.reopen_reason = format!("{} {}", t("ts_period_reopened_by"), app.current_user_name());
            app.db.update_timesheet_period(&p);
            app.notify(t("ts_period_reopened").to_string());
        }
    }
}

// ================================================================ Xodimlar

// ================================================================ Obyektlar

/// Obyektlar kesimida xodim, soat va ish grafigi (TZ XIII.3, 12, 28).
///
/// Ishchi obyektga biriktirilgan: bitta odam bir vaqtda ikki obyektda
/// bo'la olmaydi. Shuning uchun boshqa obyektga o'tkazish alohida amal —
/// tabel yozuvlari ko'chirilmaydi, ular o'sha obyektda ishlangan soatning
/// yozuvi bo'lib qoladi.
fn objects_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let to = app.today;
    let from = to - chrono::Duration::days(30);
    let rows = crate::portfolio::object_staff(&app.db, from, to);
    let issues = app.schedule_issues();

    ui.label(
        RichText::new(t("ts_obj_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    let mut moved: Option<(i64, i64)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Obyektlar bo'yicha (TZ XIII.3) ----------
            egui::Grid::new("ts_objects")
                .num_columns(6)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 240.0, t("col_object"));
                    head_r(ui, 100.0, t("ts_obj_workers"));
                    head_r(ui, 120.0, t("col_hours"));
                    head_r(ui, 120.0, t("ts_obj_idle"));
                    head_r(ui, 130.0, t("ts_obj_per_worker"));
                    head_r(ui, 150.0, t("ts_obj_payroll"));
                    ui.end_row();

                    for o in &rows {
                        let name = app
                            .projects
                            .iter()
                            .find(|p| p.id == o.project_id)
                            .map(|p| p.name.clone())
                            .unwrap_or_default();
                        cell_l(
                            ui,
                            240.0,
                            RichText::new(super::issues::truncate(&name, 30))
                                .size(12.5)
                                .color(if Some(o.project_id) == app.current {
                                    theme::accent()
                                } else {
                                    theme::text()
                                }),
                        );
                        cell_r(ui, 100.0, RichText::new(o.workers.to_string()).size(12.5));
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(format!("{:.0}", o.hours)).size(12.0),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(format!("{:.0}", o.idle_hours))
                                .size(12.0)
                                .color(if o.idle_hours > 0.0 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                }),
                        );
                        cell_r(
                            ui,
                            130.0,
                            RichText::new(format!("{:.0}", o.hours_per_worker))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(ui, 150.0, RichText::new(money(o.payroll)).size(12.0));
                        ui.end_row();
                    }
                });

            // ---------- Xodimni ko'chirish (TZ XIII.28) ----------
            if app.projects.len() > 1 {
                ui.add_space(16.0);
                ui.label(RichText::new(t("ts_obj_move")).size(13.5).strong());
                ui.label(
                    RichText::new(t("ts_obj_move_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                let workers: Vec<(i64, String)> = app
                    .workers
                    .iter()
                    .filter(|w| w.active)
                    .map(|w| (w.id, format!("{} — {}", w.name, w.position)))
                    .collect();
                for (id, label) in workers {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [260.0, 20.0],
                            egui::Label::new(
                                RichText::new(super::issues::truncate(&label, 34)).size(12.0),
                            ),
                        );
                        egui::ComboBox::from_id_salt(("ts_move", id))
                            .selected_text(t("ts_obj_move_to"))
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for p in &app.projects {
                                    if Some(p.id) == app.current {
                                        continue;
                                    }
                                    if ui.selectable_label(false, &p.name).clicked() {
                                        moved = Some((id, p.id));
                                    }
                                }
                            });
                    });
                }
            }

            // ---------- Ish grafigi (TZ XIII.12) ----------
            ui.add_space(16.0);
            ui.label(RichText::new(t("ts_obj_schedule")).size(13.5).strong());
            ui.label(
                RichText::new(t("ts_obj_schedule_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            // Grafik bo'yicha davrda nechta ish kuni borligi — e'tirozlarni
            // shu songa nisbatan o'qish kerak.
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    t("ts_obj_work_days"),
                    app.work_schedule.work_days_in(from, to)
                ))
                .size(11.5)
                .color(theme::muted()),
            );
            ui.add_space(6.0);
            if issues.is_empty() {
                ui.label(
                    RichText::new(t("ts_obj_schedule_ok"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for i in issues.iter().take(15) {
                ui.label(
                    RichText::new(format!("· {}", schedule_text(i)))
                        .size(12.0)
                        .color(theme::warn()),
                );
            }
            ui.add_space(16.0);
        });

    if let Some((worker, project)) = moved {
        if app.move_worker(worker, project) {
            app.notify(t("ts_obj_moved").to_string());
        }
    }
}

/// Grafik e'tirozini gapga aylantiradi.
fn schedule_text(i: &crate::checks::ScheduleIssue) -> String {
    use crate::checks::ScheduleIssue as S;
    match i {
        S::WorkOnRestDay { day, workers } => format!(
            "{} — {} ({} {})",
            t("ts_sch_rest"),
            day.format("%d.%m.%Y"),
            workers,
            t("ts_obj_workers")
        ),
        S::EmptyWorkDay { day } => {
            format!("{} — {}", t("ts_sch_empty"), day.format("%d.%m.%Y"))
        }
        S::OverShift { day, hours } => format!(
            "{} — {} ({:.0} {})",
            t("ts_sch_over"),
            day.format("%d.%m.%Y"),
            hours,
            t("col_hours")
        ),
    }
}

/// Anomaliyalar va xodim ehtiyoji (TZ XIII.20-21, 26-27).
fn staff_tab(ui: &mut egui::Ui, app: &mut App) {
    let anomalies = crate::checks::timesheet_anomalies(&app.timesheet);
    let forecast = app.staff_forecast();

    stat_row(
        ui,
        vec![
            stat(
                t("ts_staff_have"),
                forecast.have.to_string(),
                t("ts_staff_have_hint"),
                theme::text(),
            ),
            stat(
                t("ts_staff_need"),
                forecast.needed_workers.to_string(),
                &format!(
                    "{} {}",
                    super::materials::trim_num(forecast.needed_hours),
                    t("ts_staff_hours")
                ),
                theme::accent(),
            ),
            stat(
                t("ts_staff_gap"),
                format!("{:+}", forecast.gap),
                t("ts_staff_gap_hint"),
                if forecast.gap > 0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("ts_staff_anomalies"),
                anomalies.len().to_string(),
                t("ts_staff_anomalies_hint"),
                if anomalies.is_empty() {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    ui.label(
        RichText::new(t("ts_staff_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if anomalies.is_empty() {
        ui.label(
            RichText::new(t("ts_staff_clean"))
                .size(13.0)
                .color(theme::ok()),
        );
        return;
    }

    ui.label(RichText::new(t("ts_staff_list")).size(13.5).strong());
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for a in &anomalies {
                let (worker, text, color) = anomaly_text(app, a);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 1.5, color);
                    ui.add_space(6.0);
                    ui.add_sized(
                        [160.0, 18.0],
                        egui::Label::new(RichText::new(worker).size(12.5)),
                    );
                    ui.label(RichText::new(text).size(12.0).color(color));
                });
                ui.add_space(3.0);
            }
        });
}

/// Anomaliya matni: kim, nima va qanday rangda.
fn anomaly_text(app: &App, a: &crate::checks::TimesheetAnomaly) -> (String, String, egui::Color32) {
    use crate::checks::TimesheetAnomaly as A;
    let name = |id: i64| {
        app.workers
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.name.clone())
            .unwrap_or_default()
    };
    match a {
        A::TooManyHours {
            worker_id,
            day,
            hours,
        } => (
            name(*worker_id),
            format!(
                "{} — {} {} ({})",
                day.format("%d.%m.%Y"),
                super::materials::trim_num(*hours),
                t("ts_a_hours"),
                t("ts_a_too_many")
            ),
            theme::danger(),
        ),
        A::WeekendWork { worker_id, day } => (
            name(*worker_id),
            format!("{} — {}", day.format("%d.%m.%Y"), t("ts_a_weekend")),
            theme::warn(),
        ),
        A::NoRest { worker_id, days } => (
            name(*worker_id),
            format!("{days} {}", t("ts_a_no_rest")),
            theme::warn(),
        ),
        A::Identical {
            worker_id,
            hours,
            days,
        } => (
            name(*worker_id),
            format!(
                "{days} {} {} {}",
                t("ts_a_days"),
                super::materials::trim_num(*hours),
                t("ts_a_identical")
            ),
            theme::accent(),
        ),
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
