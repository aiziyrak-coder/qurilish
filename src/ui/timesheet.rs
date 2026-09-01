//! «Tabel» ekrani (TZ XIII).
//!
//! Haftalik jadval: qatorlar — ishchilar, ustunlar — hafta kunlari, katakda
//! soat. Kun bo'yicha va ishchi bo'yicha yig'indilar chetda turadi, ish haqi
//! esa soat va stavkadan hisoblanadi — alohida kiritilmaydi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::Worker;
use chrono::{Datelike, Duration, NaiveDate};

/// Bir kunlik normal ish vaqti — undan oshgani ortiqcha ish deb belgilanadi.
const NORM_HOURS: f64 = 8.0;

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
    let mut move_week = 0i64;
    let mut to_today = false;

    ui.horizontal(|ui| {
        if ui.button(t("add_worker")).clicked() {
            add = true;
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
        ui.label(
            RichText::new(t("timesheet_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app, week);
    ui.add_space(10.0);

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
        grid(ui, app, week);
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
        });
        app.reload_modules();
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
    let in_week = |d: NaiveDate| d >= week && d <= end;

    let hours: f64 = app
        .timesheet
        .iter()
        .filter(|e| in_week(e.date))
        .map(|e| e.hours)
        .sum();
    // Ish haqi soat va stavkadan hisoblanadi — tabel bilan bir manbadan.
    let payroll: f64 = app
        .timesheet
        .iter()
        .filter(|e| in_week(e.date))
        .map(|e| {
            e.hours
                * app
                    .workers
                    .iter()
                    .find(|w| w.id == e.worker_id)
                    .map(|w| w.hourly_rate)
                    .unwrap_or(0.0)
        })
        .sum();
    let active = app.workers.iter().filter(|w| w.active).count();
    // Haftaning ish kunlari soni — o'rtacha bandlikni hisoblash uchun.
    let worked_days = (0..7)
        .map(|i| week + Duration::days(i))
        .filter(|d| app.timesheet.iter().any(|e| e.date == *d && e.hours > 0.0))
        .count()
        .max(1);
    let avg = hours / worked_days as f64;

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_workers"),
            active.to_string(),
            &format!("{} {}", app.workers.len(), t("kpi_workers_hint")),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_hours_week"),
            super::materials::trim_num(hours),
            t("kpi_hours_week_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_avg_day"),
            super::materials::trim_num(avg),
            t("kpi_avg_day_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_payroll"),
            money(payroll),
            t("kpi_payroll_hint"),
            theme::ok(),
        );
    });
}

fn grid(ui: &mut egui::Ui, app: &mut App, week: NaiveDate) {
    let Some(pid) = app.current else { return };
    let today = app.today;
    let days: Vec<NaiveDate> = (0..7).map(|i| week + Duration::days(i)).collect();

    let mut edited: Option<Worker> = None;
    let mut removed: Option<i64> = None;
    // (ishchi, kun, soat)
    let mut set: Option<(i64, NaiveDate, f64)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("timesheet_grid")
                .num_columns(13)
                .spacing([6.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 190.0, t("col_worker"));
                    head_l(ui, 140.0, t("col_position"));
                    head_r(ui, 110.0, t("col_hourly_rate"));
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
                            egui::vec2(56.0, 26.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(56.0);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(
                                    RichText::new(t(weekday_key(*d))).size(11.0).color(color),
                                );
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

                        // Faol bo'lmagan ishchi kulrang ko'rsatiladi.
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut w.active, "").changed();
                            changed |= ui
                                .add_sized(
                                    [160.0, 22.0],
                                    egui::TextEdit::singleline(&mut w.name),
                                )
                                .changed();
                        });
                        changed |= ui
                            .add_sized([140.0, 22.0], egui::TextEdit::singleline(&mut w.position))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [110.0, 22.0],
                                egui::DragValue::new(&mut w.hourly_rate)
                                    .speed(1000.0)
                                    .range(0.0..=1e9),
                            )
                            .changed();

                        let mut total = 0.0;
                        for d in &days {
                            let cur = app
                                .timesheet
                                .iter()
                                .find(|e| e.worker_id == w.id && e.date == *d)
                                .map(|e| e.hours)
                                .unwrap_or(0.0);
                            total += cur;
                            let mut v = cur;
                            let resp = ui.add_sized(
                                [56.0, 22.0],
                                egui::DragValue::new(&mut v).speed(0.5).range(0.0..=24.0),
                            );
                            if resp.changed() {
                                set = Some((w.id, *d, v));
                            }
                            // Norma ustidagi soat — ortiqcha ish.
                            if cur > NORM_HOURS {
                                resp.on_hover_text(t("overtime_hint"));
                            }
                        }

                        cell_r(
                            ui,
                            70.0,
                            RichText::new(super::materials::trim_num(total))
                                .size(12.5)
                                .strong(),
                        );
                        cell_r(
                            ui,
                            130.0,
                            RichText::new(money(total * w.hourly_rate)).size(12.0),
                        );
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
                        RichText::new(t("total_row")).size(11.5).color(theme::muted()),
                    );
                    cell_l(ui, 140.0, RichText::new(""));
                    cell_l(ui, 110.0, RichText::new(""));
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
                            56.0,
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
                    ui.end_row();
                });
        });

    if let Some((worker, day, hours)) = set {
        app.db.set_timesheet(pid, worker, day, hours);
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

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
