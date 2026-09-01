//! «Kundalik ishlar jurnali» ekrani (TZ V).
//!
//! Prorab har kuni haqiqiy holatni yozadi: bajarilgan hajm, ishchilar,
//! texnika, ob-havo, muammolar. Jurnaldagi hajm yig'indisi GPR dagi
//! bajarilish foizini yangilaydi — plan/fakt shundan keyin haqiqiy bo'ladi.

use super::*;
use crate::domain::JournalEntry;
use egui::{vec2, Sense};

/// Yozuvga biriktirilgan fotolar ro'yxati. Yo'llar nuqtali vergul bilan
/// ajratiladi — alohida jadval ochmaslik uchun.
fn photo_list(s: &str) -> Vec<String> {
    s.split(';')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect()
}

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

    let mut add = false;
    let mut apply = false;

    ui.horizontal(|ui| {
        if ui.button(t("add_journal_entry")).clicked() {
            add = true;
        }
        if ui
            .button(RichText::new(t("apply_to_gantt")).strong())
            .on_hover_text(t("apply_to_gantt_hint"))
            .clicked()
        {
            apply = true;
        }
        ui.separator();
        ui.label(
            RichText::new(format!("{} {}", app.journal.len(), t("journal_entries")))
                .size(12.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.journal.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new(t("journal_empty")).color(theme::muted()).size(16.0));
            ui.add_space(6.0);
            ui.label(RichText::new(t("journal_hint")).color(theme::muted()).size(12.0));
        });
    } else {
        entries(ui, app);
    }

    if add {
        if let Some(pid) = app.current {
            let id = app.db.insert_journal(&JournalEntry {
                id: 0,
                project_id: pid,
                date: app.today,
                author: String::new(),
                weather: String::new(),
                temperature: 0.0,
                workers: 0,
                machines: 0,
                task_id: None,
                volume: 0.0,
                unit: String::new(),
                text: String::new(),
                remarks: String::new(),
                photos: String::new(),
            });
            app.reload_modules();
            let _ = id;
        }
    }
    if apply {
        app.apply_journal_to_tasks();
    }
}

/// Jurnal bo'yicha xulosa: oxirgi yozuv, oy davomidagi hajm va resurs.
fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let last = app.journal.iter().map(|e| e.date).max();
    let gap = last.map(|d| (app.today - d).num_days()).unwrap_or(-1);

    // Oxirgi 30 kun kesimi.
    let since = app.today - chrono::Duration::days(30);
    let recent: Vec<&JournalEntry> = app.journal.iter().filter(|e| e.date >= since).collect();
    let days_covered = {
        let mut ds: Vec<chrono::NaiveDate> = recent.iter().map(|e| e.date).collect();
        ds.sort_unstable();
        ds.dedup();
        ds.len()
    };
    let avg_workers = if recent.is_empty() {
        0
    } else {
        recent.iter().map(|e| e.workers).sum::<i64>() / recent.len() as i64
    };
    let photos: usize = app.journal.iter().map(|e| photo_list(&e.photos).len()).sum();

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("jr_last"),
            match last {
                Some(d) => d.format("%d.%m.%Y").to_string(),
                None => t("dash").to_string(),
            },
            &if gap < 0 {
                t("jr_never").to_string()
            } else if gap == 0 {
                t("jr_today").to_string()
            } else {
                format!("{gap} {} {}", t("days_short"), t("jr_ago"))
            },
            if gap > 2 { theme::danger() } else { theme::ok() },
        );
        stat_card(
            ui,
            t("jr_covered"),
            format!("{days_covered} / 30"),
            t("jr_covered_hint"),
            if days_covered >= 20 { theme::ok() } else { theme::warn() },
        );
        stat_card(
            ui,
            t("jr_avg_workers"),
            avg_workers.to_string(),
            t("jr_avg_workers_hint"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("jr_photos"),
            photos.to_string(),
            t("jr_photos_hint"),
            if photos == 0 { theme::muted() } else { theme::accent() },
        );
    });
}

fn entries(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<JournalEntry> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = (ui.available_width() - 24.0).min(1000.0);
            for entry in &app.journal {
                let mut e = entry.clone();
                let mut changed = false;

                egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(width);

                        // 1-qator: sana, muallif, ob-havo, resurs
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t("col_date")).color(theme::muted()).size(12.0));
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("j{}", e.id),
                                &mut e.date,
                            );
                            ui.add_space(8.0);
                            ui.label(RichText::new(t("col_author")).color(theme::muted()).size(12.0));
                            changed |= ui
                                .add_sized([170.0, 22.0], egui::TextEdit::singleline(&mut e.author))
                                .changed();
                            ui.add_space(8.0);
                            ui.label(RichText::new(t("col_weather")).color(theme::muted()).size(12.0));
                            changed |= ui
                                .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut e.weather))
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut e.temperature)
                                        .range(-60.0..=60.0)
                                        .suffix(" °C"),
                                )
                                .changed();
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .small_button(RichText::new("x").color(theme::danger()))
                                    .clicked()
                                {
                                    removed = Some(e.id);
                                }
                            });
                        });

                        // 2-qator: ish, hajm, resurs
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t("col_task")).color(theme::muted()).size(12.0));
                            changed |=
                                task_picker(ui, app, ("j_task", e.id), &mut e.task_id, 280.0);
                            ui.label(RichText::new(t("col_volume")).color(theme::muted()).size(12.0));
                            changed |= ui
                                .add(egui::DragValue::new(&mut e.volume).speed(0.5))
                                .changed();
                            changed |= ui
                                .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut e.unit))
                                .changed();
                            ui.add_space(8.0);
                            ui.label(RichText::new(t("col_workers")).color(theme::muted()).size(12.0));
                            changed |= ui
                                .add(egui::DragValue::new(&mut e.workers).range(0..=5000))
                                .changed();
                            ui.label(RichText::new(t("col_machines")).color(theme::muted()).size(12.0));
                            changed |= ui
                                .add(egui::DragValue::new(&mut e.machines).range(0..=500))
                                .changed();
                        });

                        // 3-qator: bajarilgan ishlar va e'tirozlar
                        changed |= ui
                            .add(
                                egui::TextEdit::multiline(&mut e.text)
                                    .desired_width(width - 20.0)
                                    .desired_rows(2)
                                    .hint_text(t("journal_text_hint")),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut e.remarks)
                                    .desired_width(width - 20.0)
                                    .hint_text(t("journal_remarks_hint")),
                            )
                            .changed();

                        // 4-qator: fotofiksatsiya (TZ V).
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.small_button(t("jr_add_photo")).clicked() {
                                if let Some(files) = rfd::FileDialog::new()
                                    .add_filter(
                                        t("photos"),
                                        &["jpg", "jpeg", "png", "bmp", "webp"],
                                    )
                                    .pick_files()
                                {
                                    let mut all = photo_list(&e.photos);
                                    for f in files {
                                        all.push(f.to_string_lossy().to_string());
                                    }
                                    e.photos = all.join(";");
                                    changed = true;
                                }
                            }
                            let list = photo_list(&e.photos);
                            if list.is_empty() {
                                ui.label(
                                    RichText::new(t("jr_no_photos"))
                                        .size(11.0)
                                        .color(theme::muted()),
                                );
                            } else {
                                ui.label(
                                    RichText::new(format!("{} {}", list.len(), t("photos")))
                                        .size(11.0)
                                        .color(theme::muted()),
                                );
                            }
                        });

                        let list = photo_list(&e.photos);
                        if !list.is_empty() {
                            ui.add_space(4.0);
                            let mut drop_idx: Option<usize> = None;
                            ui.horizontal_wrapped(|ui| {
                                for (i, path) in list.iter().enumerate() {
                                    ui.vertical(|ui| {
                                        ui.set_width(104.0);
                                        if std::path::Path::new(path).exists() {
                                            let img =
                                                egui::Image::new(format!("file://{path}"))
                                                    .fit_to_exact_size(vec2(104.0, 72.0))
                                                    .maintain_aspect_ratio(true)
                                                    .corner_radius(4);
                                            if ui
                                                .add(egui::ImageButton::new(img).frame(false))
                                                .on_hover_text(t("open_file"))
                                                .clicked()
                                            {
                                                open_path(path);
                                            }
                                        } else {
                                            let (r, _) = ui.allocate_exact_size(
                                                vec2(104.0, 72.0),
                                                Sense::hover(),
                                            );
                                            ui.painter().rect_filled(r, 4.0, theme::track());
                                            ui.painter().text(
                                                r.center(),
                                                Align2::CENTER_CENTER,
                                                t("file_missing"),
                                                egui::FontId::proportional(10.0),
                                                theme::danger(),
                                            );
                                        }
                                        if ui
                                            .small_button(
                                                RichText::new("x").color(theme::danger()),
                                            )
                                            .on_hover_text(t("remove_from_list"))
                                            .clicked()
                                        {
                                            drop_idx = Some(i);
                                        }
                                    });
                                }
                            });
                            if let Some(i) = drop_idx {
                                let mut all = list.clone();
                                all.remove(i);
                                e.photos = all.join(";");
                                changed = true;
                            }
                        }
                    });
                ui.add_space(8.0);

                if changed {
                    edited = Some(e);
                }
            }
        });

    if let Some(e) = edited {
        app.db.update_journal(&e);
        if let Some(slot) = app.journal.iter_mut().find(|x| x.id == e.id) {
            *slot = e;
        }
    }
    if let Some(id) = removed {
        app.db.del("journal", id);
        app.reload_modules();
    }
}
