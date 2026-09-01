//! «Texnika» ekrani (TZ XVI).
//!
//! Ikki ko'rinish: **parki** — texnika ro'yxati, holati, soatlik stavkasi va
//! texnik ko'rik muddati; **smenalar** — kunlik motosoat va yoqilg'i jurnali.
//! Ishlagan soat va xarajat smenalardan hisoblanadi, alohida kiritilmaydi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{Machine, MachineKind, MachineLog, MachineStatus};

/// Texnik ko'rik muddati shu kun ichida tugasa — ogohlantiramiz.
const INSPECTION_WARN_DAYS: i64 = 30;

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

    let tab_key = egui::Id::new("mch_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    let mut add_machine = false;
    let mut add_log = false;
    ui.horizontal(|ui| {
        if ui.button(t("add_machine")).clicked() {
            add_machine = true;
        }
        if !app.machines.is_empty() && ui.button(t("add_machine_log")).clicked() {
            add_log = true;
        }
        ui.label(
            RichText::new(t("machines_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.machines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("machines_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        ui.horizontal(|ui| {
            for (i, label) in [(0u8, t("mch_tab_park")), (1, t("mch_tab_logs"))] {
                if ui.selectable_label(tab == i, label).clicked() {
                    tab = i;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(tab_key, tab));
        ui.add_space(8.0);

        if tab == 1 {
            logs_tab(ui, app);
        } else {
            park_tab(ui, app);
        }
    }

    if add_machine {
        let n = app.machines.len() + 1;
        app.db.insert_machine(&Machine {
            id: 0,
            project_id: pid,
            name: format!("{} {n}", t("machine_new_name")),
            kind: MachineKind::Other,
            reg_no: String::new(),
            owner: String::new(),
            status: MachineStatus::Idle,
            hour_rate: 0.0,
            operator: String::new(),
            inspection_until: None,
        });
        app.reload_modules();
    }
    if add_log {
        if let Some(first) = app.machines.first().map(|m| m.id) {
            app.db.insert_machine_log(&MachineLog {
                id: 0,
                project_id: pid,
                machine_id: first,
                date: app.today,
                hours: 0.0,
                fuel: 0.0,
                task_id: None,
                note: String::new(),
            });
            app.reload_modules();
            ui.data_mut(|d| d.insert_temp(tab_key, 1u8));
        }
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let working = app
        .machines
        .iter()
        .filter(|m| m.status == MachineStatus::Working)
        .count();
    let repair = app
        .machines
        .iter()
        .filter(|m| m.status == MachineStatus::Repair)
        .count();

    // Oxirgi 30 kundagi motosoat, yoqilg'i va xarajat.
    let since = app.today - chrono::Duration::days(30);
    let recent: Vec<&MachineLog> = app.machine_logs.iter().filter(|l| l.date >= since).collect();
    let hours: f64 = recent.iter().map(|l| l.hours).sum();
    let fuel: f64 = recent.iter().map(|l| l.fuel).sum();
    let cost: f64 = recent
        .iter()
        .map(|l| {
            l.hours
                * app
                    .machines
                    .iter()
                    .find(|m| m.id == l.machine_id)
                    .map(|m| m.hour_rate)
                    .unwrap_or(0.0)
        })
        .sum();

    // Texnik ko'rik muddati o'tgan yoki yaqinda tugaydigan texnika.
    let mut expired = 0;
    let mut soon = 0;
    for m in &app.machines {
        let Some(until) = m.inspection_until else {
            continue;
        };
        let left = (until - app.today).num_days();
        if left < 0 {
            expired += 1;
        } else if left <= INSPECTION_WARN_DAYS {
            soon += 1;
        }
    }

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_machines"),
            app.machines.len().to_string(),
            &format!(
                "{working} {} · {repair} {}",
                t("ms_working"),
                t("ms_repair")
            ),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_machine_hours"),
            super::materials::trim_num(hours),
            t("kpi_machine_hours_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_machine_fuel"),
            super::materials::trim_num(fuel),
            t("kpi_machine_fuel_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_machine_cost"),
            money(cost),
            t("kpi_machine_cost_hint"),
            theme::warn(),
        );
        stat_card(
            ui,
            t("kpi_inspection"),
            expired.to_string(),
            &format!("{soon} {}", t("kpi_inspection_hint")),
            if expired == 0 && soon == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
    });
}

fn status_color(s: MachineStatus) -> Color32 {
    match s {
        MachineStatus::Working => theme::ok(),
        MachineStatus::Idle => theme::warn(),
        MachineStatus::Repair => theme::danger(),
        MachineStatus::Off => theme::muted(),
    }
}

// ================================================================ Park

fn park_tab(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<Machine> = None;
    let mut removed: Option<i64> = None;
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("machines_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 190.0, t("col_machine"));
                    head_l(ui, 140.0, t("col_kind"));
                    head_l(ui, 120.0, t("col_reg_no"));
                    head_l(ui, 120.0, t("col_status"));
                    head_l(ui, 160.0, t("col_owner"));
                    head_l(ui, 150.0, t("col_operator"));
                    head_r(ui, 120.0, t("col_hour_rate"));
                    head_l(ui, 150.0, t("col_inspection"));
                    head_r(ui, 100.0, t("col_hours_30"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.machines {
                        let mut m = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([190.0, 22.0], egui::TextEdit::singleline(&mut m.name))
                            .changed();
                        egui::ComboBox::from_id_salt(("mch_kind", m.id))
                            .selected_text(m.kind.label())
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for k in MachineKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut m.kind, *k, k.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut m.reg_no))
                            .changed();
                        egui::ComboBox::from_id_salt(("mch_st", m.id))
                            .selected_text(
                                RichText::new(m.status.label()).color(status_color(m.status)),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for s in MachineStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut m.status, *s, s.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([160.0, 22.0], egui::TextEdit::singleline(&mut m.owner))
                            .changed();
                        changed |= ui
                            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut m.operator))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut m.hour_rate)
                                    .speed(1000.0)
                                    .range(0.0..=1e9),
                            )
                            .changed();

                        // Texnik ko'rik muddati — belgilanmagan bo'lishi mumkin.
                        ui.horizontal(|ui| {
                            let mut has = m.inspection_until.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                m.inspection_until =
                                    has.then(|| today + chrono::Duration::days(365));
                                changed = true;
                            }
                            if let Some(mut d) = m.inspection_until {
                                if super::passport::date_edit(ui, &format!("mci{}", m.id), &mut d) {
                                    m.inspection_until = Some(d);
                                    changed = true;
                                }
                                let left = (d - today).num_days();
                                if left < 0 {
                                    ui.label(
                                        RichText::new(t("cert_expired"))
                                            .size(10.5)
                                            .color(theme::danger()),
                                    );
                                } else if left <= INSPECTION_WARN_DAYS {
                                    ui.label(
                                        RichText::new(format!("{left} {}", t("days_short")))
                                            .size(10.5)
                                            .color(theme::warn()),
                                    );
                                }
                            }
                        });

                        // Oxirgi 30 kundagi motosoat — bandlik ko'rsatkichi.
                        let since = today - chrono::Duration::days(30);
                        let hours: f64 = app
                            .machine_logs
                            .iter()
                            .filter(|l| l.machine_id == m.id && l.date >= since)
                            .map(|l| l.hours)
                            .sum();
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(super::materials::trim_num(hours)).size(12.5),
                        );

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(m.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(m);
                        }
                    }
                });
        });

    if let Some(m) = edited {
        app.db.update_machine(&m);
        if let Some(slot) = app.machines.iter_mut().find(|x| x.id == m.id) {
            *slot = m;
        }
    }
    if let Some(id) = removed {
        app.db.del("machine", id);
        app.reload_modules();
    }
}

// ================================================================ Smenalar

fn logs_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.machine_logs.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("machine_logs_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let mut edited: Option<MachineLog> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("machine_logs_grid")
                .num_columns(7)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 200.0, t("col_machine"));
                    head_r(ui, 90.0, t("col_hours"));
                    head_r(ui, 90.0, t("col_fuel"));
                    head_r(ui, 130.0, t("col_machine_cost"));
                    head_l(ui, 200.0, t("col_task"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.machine_logs {
                        let mut l = src.clone();
                        let mut changed = false;

                        changed |=
                            super::passport::date_edit(ui, &format!("ml{}", l.id), &mut l.date);
                        let cur = app
                            .machines
                            .iter()
                            .find(|m| m.id == l.machine_id)
                            .cloned();
                        egui::ComboBox::from_id_salt(("ml_mch", l.id))
                            .selected_text(
                                cur.as_ref()
                                    .map(|m| m.name.clone())
                                    .unwrap_or_else(|| t("dash").to_string()),
                            )
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for m in &app.machines {
                                    changed |= ui
                                        .selectable_value(&mut l.machine_id, m.id, &m.name)
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add_sized(
                                [90.0, 22.0],
                                egui::DragValue::new(&mut l.hours).speed(0.5).range(0.0..=24.0),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [90.0, 22.0],
                                egui::DragValue::new(&mut l.fuel).speed(1.0).range(0.0..=1e6),
                            )
                            .changed();
                        // Smena qiymati — soat va stavkadan.
                        let rate = cur.map(|m| m.hour_rate).unwrap_or(0.0);
                        cell_r(
                            ui,
                            130.0,
                            RichText::new(money(l.hours * rate))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        changed |= task_picker(ui, app, ("ml_task", l.id), &mut l.task_id, 200.0);

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(l.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(l);
                        }
                    }
                });
        });

    if let Some(l) = edited {
        app.db.update_machine_log(&l);
        if let Some(slot) = app.machine_logs.iter_mut().find(|x| x.id == l.id) {
            *slot = l;
        }
    }
    if let Some(id) = removed {
        app.db.del("machine_log", id);
        app.reload_modules();
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
