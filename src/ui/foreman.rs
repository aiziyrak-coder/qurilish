//! «Prorab ish o'rni» ekrani (TZ VI).
//!
//! Bir kunlik ish o'rni: bugun qaysi ishlar ketyapti, ularning bajarilishi,
//! kunlik jurnal yozuvi, tabel va texnika smenalari — hammasi bitta ekranda,
//! kunni yopish uchun boshqa bo'limlarga o'tish shart emas.
//!
//! Bu ekran shu kompyuterdagi bazada ishlaydi. TZ dagi mobil klient va bir
//! nechta foydalanuvchi o'rtasida sinxronizatsiya alohida bosqich — u server
//! qismini talab qiladi va bu yerda ko'zda tutilmagan.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{IssueStatus, JournalEntry, MachineLog, MachineStatus, SafetyEvent};
use chrono::{Duration, NaiveDate};

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
    let today = app.today;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} — {}", t("foreman_today"), today.format("%d.%m.%Y")))
                .size(14.0)
                .strong(),
        );
        ui.label(
            RichText::new(t("foreman_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                today_tasks(ui, app);
                ui.add_space(12.0);
                journal_block(ui, app, pid);
                ui.add_space(12.0);
                crew_block(ui, app, pid);
                ui.add_space(12.0);
                machines_block(ui, app, pid);
                ui.add_space(12.0);
                attention_block(ui, app);
                ui.add_space(20.0);
            });
        });
}

/// Bugun ketayotgan ishlar: boshlangan, tugallanmagan va rejasi bugunni qamragan.
fn running_today(app: &App) -> Vec<i64> {
    let origin = app.origin();
    app.tasks
        .iter()
        .filter(|t| t.progress < 99.99)
        .filter(|t| {
            let Some(c) = app.schedule.get(t.id) else {
                return false;
            };
            let start = origin + Duration::days(c.es);
            let end = origin + Duration::days(c.ef);
            start <= app.today && app.today <= end
        })
        .map(|t| t.id)
        .collect()
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let running = running_today(app).len();
    let workers_today = app
        .timesheet
        .iter()
        .filter(|e| e.date == app.today)
        .count();
    let machines_today = app
        .machine_logs
        .iter()
        .filter(|l| l.date == app.today)
        .count();
    let journal_done = app.journal.iter().any(|j| j.date == app.today);
    let open_safety = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_running_today"),
            running.to_string(),
            t("kpi_running_today_hint"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_crew_today"),
            workers_today.to_string(),
            t("kpi_crew_today_hint"),
            if workers_today == 0 { theme::warn() } else { theme::ok() },
        );
        stat_card(
            ui,
            t("kpi_machines_today"),
            machines_today.to_string(),
            t("kpi_machines_today_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_journal_today"),
            if journal_done { t("yes").into() } else { t("no").into() },
            t("kpi_journal_today_hint"),
            if journal_done { theme::ok() } else { theme::warn() },
        );
        stat_card(
            ui,
            t("kpi_open_safety"),
            open_safety.to_string(),
            t("kpi_open_safety_hint"),
            if open_safety == 0 { theme::ok() } else { theme::danger() },
        );
    });
}

/// Sarlavhali blok — barcha bo'limlar bir xil ko'rinishda.
fn block(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(14.0).strong());
                ui.add_space(6.0);
                add(ui);
            });
        });
}

// ================================================================ Bugungi ishlar

fn today_tasks(ui: &mut egui::Ui, app: &mut App) {
    let ids = running_today(app);
    let mut changed: Option<crate::model::Task> = None;
    let mut open_gantt: Option<i64> = None;

    block(ui, t("foreman_tasks"), |ui| {
        if ids.is_empty() {
            ui.label(
                RichText::new(t("foreman_no_tasks"))
                    .size(12.0)
                    .color(theme::muted()),
            );
            return;
        }
        egui::Grid::new("fm_tasks")
            .num_columns(6)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                cell_l(ui, 60.0, RichText::new(t("col_wbs")).size(11.0).color(theme::muted()));
                cell_l(ui, 300.0, RichText::new(t("col_task")).size(11.0).color(theme::muted()));
                cell_l(ui, 70.0, RichText::new(t("col_section_short")).size(11.0).color(theme::muted()));
                cell_r(ui, 140.0, RichText::new(t("col_progress")).size(11.0).color(theme::muted()));
                cell_l(ui, 170.0, RichText::new(t("col_responsible")).size(11.0).color(theme::muted()));
                cell_l(ui, 70.0, RichText::new("").size(11.0));
                ui.end_row();

                for id in &ids {
                    let Some(src) = app.tasks.iter().find(|t| t.id == *id) else {
                        continue;
                    };
                    let mut task = src.clone();
                    cell_l(ui, 60.0, RichText::new(&task.wbs).size(12.0).color(theme::muted()));
                    cell_l(
                        ui,
                        300.0,
                        RichText::new(super::issues::truncate(&task.name, 42)).size(12.5),
                    );
                    cell_l(
                        ui,
                        70.0,
                        RichText::new(task.section.code())
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    // Bajarilishni shu yerda o'zgartirish mumkin — prorabning asosiy amali.
                    let resp = ui.add_sized(
                        [140.0, 22.0],
                        egui::Slider::new(&mut task.progress, 0.0..=100.0)
                            .suffix("%")
                            .show_value(true),
                    );
                    cell_l(
                        ui,
                        170.0,
                        RichText::new(super::issues::truncate(&task.responsible, 22))
                            .size(12.0)
                            .color(theme::muted()),
                    );
                    if ui.small_button(t("an_open")).clicked() {
                        open_gantt = Some(task.id);
                    }
                    ui.end_row();

                    if resp.changed() {
                        changed = Some(task);
                    }
                }
            });
    });

    if let Some(task) = changed {
        app.save_task(task);
    }
    if let Some(id) = open_gantt {
        app.screen = Screen::Gantt;
        app.selected_task = Some(id);
        app.scroll_to_task(id);
    }
}

// ================================================================ Kunlik jurnal

fn journal_block(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let today = app.today;
    let existing = app.journal.iter().find(|j| j.date == today).cloned();
    let mut create = false;
    let mut edited: Option<JournalEntry> = None;
    let mut open_journal = false;

    block(ui, t("foreman_journal"), |ui| {
        match existing.clone() {
            Some(mut j) => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("col_task")).size(11.5).color(theme::muted()));
                    changed |= task_picker(ui, app, "fm_j_task", &mut j.task_id, 260.0);
                    ui.label(RichText::new(t("col_volume")).size(11.5).color(theme::muted()));
                    changed |= ui
                        .add_sized(
                            [90.0, 22.0],
                            egui::DragValue::new(&mut j.volume).speed(1.0).range(0.0..=1e9),
                        )
                        .changed();
                    changed |= ui
                        .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut j.unit))
                        .changed();
                    ui.label(RichText::new(t("col_workers")).size(11.5).color(theme::muted()));
                    changed |= ui
                        .add_sized(
                            [60.0, 22.0],
                            egui::DragValue::new(&mut j.workers).speed(1.0).range(0.0..=999.0),
                        )
                        .changed();
                    ui.label(RichText::new(t("col_machines")).size(11.5).color(theme::muted()));
                    changed |= ui
                        .add_sized(
                            [60.0, 22.0],
                            egui::DragValue::new(&mut j.machines).speed(1.0).range(0.0..=999.0),
                        )
                        .changed();
                });
                ui.add_space(4.0);
                changed |= ui
                    .add_sized(
                        [ui.available_width().min(760.0), 46.0],
                        egui::TextEdit::multiline(&mut j.text).hint_text(t("journal_text_hint")),
                    )
                    .changed();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button(t("open_journal")).clicked() {
                        open_journal = true;
                    }
                    ui.label(
                        RichText::new(t("foreman_journal_saved"))
                            .size(11.0)
                            .color(theme::ok()),
                    );
                });
                if changed {
                    edited = Some(j);
                }
            }
            None => {
                ui.label(
                    RichText::new(t("foreman_no_journal"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                if ui.button(t("foreman_start_journal")).clicked() {
                    create = true;
                }
            }
        }
    });

    if create {
        // Bugungi tabel va smenalardan boshlang'ich sonlar olinadi.
        let workers = app.timesheet.iter().filter(|e| e.date == today).count() as i64;
        let machines = app.machine_logs.iter().filter(|l| l.date == today).count() as i64;
        let running = running_today(app);
        app.db.insert_journal(&JournalEntry {
            id: 0,
            project_id: pid,
            date: today,
            author: String::new(),
            weather: String::new(),
            temperature: 0.0,
            workers,
            machines,
            task_id: running.first().copied(),
            volume: 0.0,
            unit: String::new(),
            text: String::new(),
            remarks: String::new(),
            photos: String::new(),
        });
        app.reload_modules();
    }
    if let Some(j) = edited {
        app.db.update_journal(&j);
        if let Some(slot) = app.journal.iter_mut().find(|x| x.id == j.id) {
            *slot = j;
        }
    }
    if open_journal {
        app.screen = Screen::Journal;
    }
}

// ================================================================ Brigada

fn crew_block(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let today = app.today;
    let mut set: Option<(i64, f64)> = None;
    let mut fill_all = false;

    block(ui, t("foreman_crew"), |ui| {
        if app.workers.is_empty() {
            ui.label(
                RichText::new(t("foreman_no_workers"))
                    .size(12.0)
                    .color(theme::muted()),
            );
            return;
        }
        ui.horizontal(|ui| {
            if ui
                .button(t("foreman_fill_shift"))
                .on_hover_text(t("foreman_fill_shift_hint"))
                .clicked()
            {
                fill_all = true;
            }
            let total: f64 = app
                .timesheet
                .iter()
                .filter(|e| e.date == today)
                .map(|e| e.hours)
                .sum();
            ui.label(
                RichText::new(format!("{}: {}", t("col_total_hours"), trim(total)))
                    .size(12.0)
                    .color(theme::muted()),
            );
        });
        ui.add_space(6.0);
        egui::Grid::new("fm_crew")
            .num_columns(3)
            .spacing([10.0, 5.0])
            .striped(true)
            .show(ui, |ui| {
                for w in app.workers.iter().filter(|w| w.active) {
                    cell_l(ui, 220.0, RichText::new(&w.name).size(12.5));
                    cell_l(
                        ui,
                        160.0,
                        RichText::new(&w.position).size(12.0).color(theme::muted()),
                    );
                    let cur = app
                        .timesheet
                        .iter()
                        .find(|e| e.worker_id == w.id && e.date == today)
                        .map(|e| e.hours)
                        .unwrap_or(0.0);
                    let mut v = cur;
                    if ui
                        .add_sized(
                            [70.0, 22.0],
                            egui::DragValue::new(&mut v).speed(0.5).range(0.0..=24.0),
                        )
                        .changed()
                    {
                        set = Some((w.id, v));
                    }
                    ui.end_row();
                }
            });
    });

    if fill_all {
        // Faol brigadaga standart 8 soat — keyin alohida tuzatish mumkin.
        for w in app.workers.iter().filter(|w| w.active) {
            app.db.set_timesheet(pid, w.id, today, 8.0);
        }
        app.timesheet = app.db.timesheet(pid);
    }
    if let Some((worker, hours)) = set {
        app.db.set_timesheet(pid, worker, today, hours);
        app.timesheet = app.db.timesheet(pid);
    }
}

// ================================================================ Texnika

fn machines_block(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let today = app.today;
    let mut add: Option<i64> = None;
    let mut edited: Option<MachineLog> = None;

    block(ui, t("foreman_machines"), |ui| {
        if app.machines.is_empty() {
            ui.label(
                RichText::new(t("foreman_no_machines"))
                    .size(12.0)
                    .color(theme::muted()),
            );
            return;
        }
        egui::Grid::new("fm_mch")
            .num_columns(4)
            .spacing([10.0, 5.0])
            .striped(true)
            .show(ui, |ui| {
                for m in &app.machines {
                    cell_l(ui, 220.0, RichText::new(&m.name).size(12.5));
                    let color = match m.status {
                        MachineStatus::Working => theme::ok(),
                        MachineStatus::Repair => theme::danger(),
                        _ => theme::muted(),
                    };
                    cell_l(
                        ui,
                        120.0,
                        RichText::new(m.status.label()).size(11.5).color(color),
                    );
                    match app
                        .machine_logs
                        .iter()
                        .find(|l| l.machine_id == m.id && l.date == today)
                    {
                        Some(src) => {
                            let mut l = src.clone();
                            let mut changed = false;
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(t("col_hours")).size(11.0).color(theme::muted()),
                                );
                                changed |= ui
                                    .add_sized(
                                        [64.0, 22.0],
                                        egui::DragValue::new(&mut l.hours)
                                            .speed(0.5)
                                            .range(0.0..=24.0),
                                    )
                                    .changed();
                                ui.label(
                                    RichText::new(t("col_fuel")).size(11.0).color(theme::muted()),
                                );
                                changed |= ui
                                    .add_sized(
                                        [64.0, 22.0],
                                        egui::DragValue::new(&mut l.fuel)
                                            .speed(1.0)
                                            .range(0.0..=1e6),
                                    )
                                    .changed();
                            });
                            if changed {
                                edited = Some(l);
                            }
                        }
                        None => {
                            if ui.small_button(t("foreman_add_shift")).clicked() {
                                add = Some(m.id);
                            }
                        }
                    }
                    // Texnik ko'rik muddati o'tgan bo'lsa — prorab buni ko'rishi shart.
                    if m.inspection_until.is_some_and(|d| d < today) {
                        cell_l(
                            ui,
                            160.0,
                            RichText::new(t("inspection_expired"))
                                .size(11.0)
                                .color(theme::danger()),
                        );
                    } else {
                        cell_l(ui, 160.0, RichText::new(""));
                    }
                    ui.end_row();
                }
            });
    });

    if let Some(machine_id) = add {
        app.db.insert_machine_log(&MachineLog {
            id: 0,
            project_id: pid,
            machine_id,
            date: today,
            hours: 8.0,
            fuel: 0.0,
            task_id: running_today(app).first().copied(),
            note: String::new(),
        });
        app.reload_modules();
    }
    if let Some(l) = edited {
        app.db.update_machine_log(&l);
        if let Some(slot) = app.machine_logs.iter_mut().find(|x| x.id == l.id) {
            *slot = l;
        }
    }
}

// ================================================================ Diqqat

fn attention_block(ui: &mut egui::Ui, app: &mut App) {
    let today = app.today;
    let open: Vec<&SafetyEvent> = app
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .collect();
    let low: Vec<String> = app
        .stock()
        .iter()
        .filter(|l| l.below_min)
        .filter_map(|l| {
            app.materials
                .iter()
                .find(|m| m.id == l.material_id)
                .map(|m| format!("{} ({} {})", m.name, trim(l.balance), m.unit))
        })
        .collect();

    let mut go: Option<Screen> = None;
    block(ui, t("foreman_attention"), |ui| {
        if open.is_empty() && low.is_empty() {
            ui.label(
                RichText::new(t("foreman_all_clear"))
                    .size(12.0)
                    .color(theme::ok()),
            );
            return;
        }
        for s in &open {
            ui.horizontal(|ui| {
                let overdue = s.deadline.is_some_and(|d| d < today);
                ui.label(
                    RichText::new(if overdue { "!" } else { "·" })
                        .color(if overdue { theme::danger() } else { theme::warn() })
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("{} — {}", s.kind.label(), s.description))
                        .size(12.0),
                );
                if ui.small_button(t("an_open")).clicked() {
                    go = Some(Screen::Safety);
                }
            });
        }
        if !low.is_empty() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("·").color(theme::warn()).strong());
                ui.label(
                    RichText::new(format!("{}: {}", t("an_below_min"), low.join(", ")))
                        .size(12.0),
                );
                if ui.small_button(t("an_open")).clicked() {
                    go = Some(Screen::Warehouse);
                }
            });
        }
    });

    if let Some(s) = go {
        app.screen = s;
    }
}

/// Sana chegaralarini tekshirish uchun kichik yordamchi (testlarda ishlatiladi).
#[allow(dead_code)]
fn covers(start: NaiveDate, end: NaiveDate, day: NaiveDate) -> bool {
    start <= day && day <= end
}
