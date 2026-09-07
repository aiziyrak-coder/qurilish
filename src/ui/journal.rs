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
    // Rol jurnalga qaysi savol bilan kelishi (TZ V.2).
    let view = app.role().journal_role();
    let writes = view == crate::roles::JournalRole::Writes && app.can_edit(Screen::Journal);

    ui.horizontal(|ui| {
        if writes {
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
        }
        ui.label(
            RichText::new(format!("{} · {}", app.role().label(), t(view.hint())))
                .size(11.5)
                .color(theme::muted()),
        );
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
            ui.label(
                RichText::new(t("journal_empty"))
                    .color(theme::muted())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("journal_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
    } else {
        let tab_key = egui::Id::new("jr_tab");
        // Birinchi ochilishda rolga mos tab: tekshiruvchi rol uchun kun
        // tahlili, yozuvchi rol uchun yozuvlar.
        let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(view.tab());
        ui.horizontal_wrapped(|ui| {
            for (i, label) in [
                (0u8, t("jr_tab_entries")),
                (1, t("jr_tab_day")),
                (2, t("jr_tab_tomorrow")),
                (3, t("jr_tab_special")),
            ] {
                if ui.selectable_label(tab == i, label).clicked() {
                    tab = i;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(tab_key, tab));
        ui.add_space(8.0);
        match tab {
            1 => day_tab(ui, app),
            2 => tomorrow_tab(ui, app),
            3 => special_tab(ui, app),
            _ => entries(ui, app),
        }
    }

    if add {
        // Bitta joyda yaratiladi: tepadagi tugma ham, ro'yxatdagi tugma
        // ham bir xil yozuvni ochsin.
        add_entry(app);
    }
    if apply {
        app.apply_journal_to_tasks();
    }
}

// ================================================================ Maxsus jurnallar

/// Maxsus jurnallar (TZ IV.14).
///
/// Ular alohida jadval sifatida yuritilmaydi: bu mavjud yozuvlarning
/// ko'rinishi. Ikkinchi nusxa yuritish ularning bir-biriga zid bo'lishiga
/// olib kelardi.
fn special_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    use crate::checks::SpecialJournal;

    let key = egui::Id::new("jr_special");
    let mut kind = ui
        .data(|d| d.get_temp::<u8>(key))
        .and_then(|i| SpecialJournal::ALL.get(i as usize).copied())
        .unwrap_or(SpecialJournal::Concrete);

    ui.label(
        RichText::new(t("jr_special_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for (i, k) in SpecialJournal::ALL.iter().enumerate() {
            if ui.selectable_label(kind == *k, t(k.key())).clicked() {
                kind = *k;
                ui.data_mut(|d| d.insert_temp(key, i as u8));
            }
        }
    });
    ui.add_space(8.0);

    let rows = app.special_journal(kind);
    if rows.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("jr_special_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("jr_special_grid")
                .num_columns(4)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_date"));
                    head_l(ui, 160.0, t("col_number"));
                    head_l(ui, 320.0, t("col_item"));
                    head_r(ui, 180.0, t("col_result"));
                    ui.end_row();

                    for r in &rows {
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(r.date.format("%d.%m.%Y").to_string()).size(12.0),
                        );
                        cell_l(
                            ui,
                            160.0,
                            RichText::new(super::issues::truncate(&r.number, 20)).size(12.0),
                        );
                        cell_l(
                            ui,
                            320.0,
                            RichText::new(super::issues::truncate(&r.subject, 42)).size(12.5),
                        );
                        cell_r(
                            ui,
                            180.0,
                            RichText::new(&r.result).size(12.0).color(if r.bad {
                                theme::danger()
                            } else {
                                theme::ok()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Kun tahlili

/// Kunlik hajm bilan sarflangan material va yozuvlardagi ichki ziddiyatlar
/// (TZ V.10-11, 32).
fn day_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let day = app.today;
    let rows = app.day_material(day);
    let doubts = app.journal_doubts();

    ui.label(
        RichText::new(t("jr_day_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    // ---------- Direktor uchun kunlik xulosa (TZ V.24) ----------
    let d = app.day_report();
    stat_row(
        ui,
        vec![
            stat(
                t("jr_dr_running"),
                format!("{} / {}", d.logged, d.running),
                t("jr_dr_running_hint"),
                if d.logged >= d.running {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("jr_dr_crew"),
                d.workers.to_string(),
                &format!("{:.0} {}", d.hours, t("col_hours")),
                theme::text(),
            ),
            stat(
                t("jr_dr_machines"),
                d.machines.to_string(),
                t("jr_dr_machines_hint"),
                theme::text(),
            ),
            stat(
                t("jr_dr_material"),
                money(d.material_cost),
                t("jr_dr_material_hint"),
                theme::text(),
            ),
            stat(
                t("jr_dr_events"),
                format!("{} / {}", d.safety_new, d.quality_new),
                t("jr_dr_events_hint"),
                if d.safety_new == 0 {
                    theme::muted()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("jr_dr_blockers"),
                d.blockers.to_string(),
                t("jr_dr_blockers_hint"),
                if d.blockers == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("jr_dr_docs"),
                d.docs_signed.to_string(),
                t("jr_dr_docs_hint"),
                theme::text(),
            ),
        ],
    );
    ui.label(
        RichText::new(format!("{}: {}", t("col_date"), d.day.format("%d.%m.%Y")))
            .size(10.5)
            .color(theme::muted()),
    );
    ui.add_space(14.0);

    // ---------- Jurnaldan ariza (TZ V.17) ----------
    let suggestions = app.journal_requests();
    let mut make: Option<usize> = None;
    if !suggestions.is_empty() {
        ui.label(RichText::new(t("jr_req_title")).size(13.5).strong());
        ui.label(
            RichText::new(t("jr_req_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
        ui.add_space(4.0);
        for (i, r) in suggestions.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "· {} — {}: {} ({} {})",
                        app.task(r.task_id)
                            .map(|x| super::issues::truncate(&x.name, 28))
                            .unwrap_or_default(),
                        super::materials::material_label(app, r.material_id),
                        super::materials::trim_num(r.qty),
                        t("jr_req_stock"),
                        super::materials::trim_num(r.available)
                    ))
                    .size(12.0),
                );
                if ui.small_button(t("jr_req_make")).clicked() {
                    make = Some(i);
                }
            });
        }
        ui.add_space(12.0);
    }

    if let Some(i) = make {
        create_request_from(app, &suggestions[i]);
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(RichText::new(t("jr_day_material")).size(13.5).strong());
            ui.add_space(6.0);
            if rows.is_empty() {
                ui.label(
                    RichText::new(t("jr_day_no_material"))
                        .size(12.5)
                        .color(theme::muted()),
                );
            } else {
                egui::Grid::new("jr_day_mat")
                    .num_columns(6)
                    .spacing([10.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 240.0, t("col_task"));
                        head_l(ui, 220.0, t("col_material"));
                        head_r(ui, 110.0, t("jr_day_volume"));
                        head_r(ui, 120.0, t("jr_day_norm"));
                        head_r(ui, 120.0, t("jr_day_issued"));
                        head_r(ui, 120.0, t("jr_day_diff"));
                        ui.end_row();

                        for r in &rows {
                            let name = app
                                .task(r.task_id)
                                .map(|x| format!("{} {}", x.wbs, x.name))
                                .unwrap_or_default();
                            cell_l(
                                ui,
                                240.0,
                                RichText::new(super::issues::truncate(&name, 32)).size(12.5),
                            );
                            cell_l(
                                ui,
                                220.0,
                                RichText::new(super::issues::truncate(
                                    &super::materials::material_label(app, r.material_id),
                                    28,
                                ))
                                .size(12.0),
                            );
                            cell_r(
                                ui,
                                110.0,
                                RichText::new(super::materials::trim_num(r.volume)).size(12.0),
                            );
                            cell_r(
                                ui,
                                120.0,
                                RichText::new(super::materials::trim_num(r.by_norm)).size(12.0),
                            );
                            cell_r(
                                ui,
                                120.0,
                                RichText::new(super::materials::trim_num(r.issued)).size(12.0),
                            );
                            cell_r(
                                ui,
                                120.0,
                                RichText::new(super::materials::trim_num(r.diff))
                                    .size(12.5)
                                    .color(if r.over() {
                                        theme::danger()
                                    } else {
                                        theme::muted()
                                    }),
                            );
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(16.0);
            ui.label(RichText::new(t("jr_doubts")).size(13.5).strong());
            ui.label(
                RichText::new(t("jr_doubts_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if doubts.is_empty() {
                ui.label(
                    RichText::new(t("jr_doubts_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            }
            for c in &doubts {
                egui::Frame::new()
                    .fill(theme::card())
                    .inner_margin(10.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        // Sana yonida qaysi ish ekani turishi kerak:
                        // shubha «qaysi yozuvda» degan savolsiz o'qilsin.
                        let task = app
                            .journal
                            .iter()
                            .find(|j| j.id == c.entry_id)
                            .and_then(|j| j.task_id)
                            .and_then(|id| app.task(id))
                            .map(|x| format!(" · {} {}", x.wbs, x.name))
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(format!(
                                "{}{}",
                                c.date.format("%d.%m.%Y"),
                                super::issues::truncate(&task, 40)
                            ))
                            .size(12.5)
                            .strong(),
                        );
                        for d in &c.doubts {
                            ui.label(
                                RichText::new(format!("· {}", doubt_text(d)))
                                    .size(12.0)
                                    .color(if d.severe() {
                                        theme::danger()
                                    } else {
                                        theme::warn()
                                    }),
                            );
                        }
                    });
                ui.add_space(6.0);
            }
        });
}

/// Taklif bo'yicha ariza yaratadi (TZ V.17).
///
/// Ariza **qoralama** holatida ochiladi: dastur o'zi ariza yubormaydi,
/// prorab uni ko'rib, tasdiqqa qo'yadi.
pub fn create_request_from(app: &mut App, r: &crate::checks::JournalRequest) {
    let Some(pid) = app.current else { return };
    let material = app.materials.iter().find(|m| m.id == r.material_id);
    let title = material.map(|m| m.name.clone()).unwrap_or_default();
    let unit = material.map(|m| m.unit.clone()).unwrap_or_default();
    let n = app.requests.len() + 1;

    app.db.insert_request(&crate::domain::Request {
        id: 0,
        project_id: pid,
        number: format!("Z-{n:03}"),
        date: app.today,
        kind: crate::domain::RequestKind::Material,
        title,
        material_id: Some(r.material_id),
        qty: r.qty,
        unit,
        requester: app.current_user_name(),
        // Muddat: ish davom etyapti, shuning uchun material tez kerak.
        need_date: app.today + chrono::Duration::days(3),
        priority: crate::domain::Priority::High,
        status: crate::domain::RequestStatus::New,
        task_id: Some(r.task_id),
        reject_reason: String::new(),
        note: t("jr_req_note").to_string(),
    });
    app.reload_modules();
    app.notify(t("jr_req_done").to_string());
}

/// Shubhani odam o'qiydigan gapga aylantiradi.
fn doubt_text(d: &crate::checks::JournalDoubt) -> String {
    use crate::checks::JournalDoubt as D;
    match d {
        D::VolumeOverPlan { entered, left } => format!(
            "{}: {} > {}",
            t("jd_over_plan"),
            super::materials::trim_num(*entered),
            super::materials::trim_num(*left)
        ),
        D::CrewWithoutTimesheet { workers } => {
            format!("{} ({})", t("jd_no_timesheet"), workers)
        }
        D::RepeatedVolume { days, volume } => format!(
            "{}: {} × {}",
            t("jd_repeated"),
            days,
            super::materials::trim_num(*volume)
        ),
        D::FutureDate => t("jd_future").to_string(),
        D::ImplausibleRate {
            per_worker,
            average,
        } => format!(
            "{}: {} / {}",
            t("jd_rate"),
            super::materials::trim_num(*per_worker),
            super::materials::trim_num(*average)
        ),
    }
}

// ================================================================ Ertangi reja

/// Ertaga nima ketadi, material yetadimi, kim bor (TZ V.16).
fn tomorrow_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let plan = app.tomorrow_plan();
    let blocked = plan.iter().filter(|p| !p.ready()).count();

    ui.label(
        RichText::new(t("jr_tomorrow_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("jr_tm_tasks"),
                plan.len().to_string(),
                t("jr_tm_tasks_hint"),
                theme::text(),
            ),
            stat(
                t("jr_tm_starts"),
                plan.iter().filter(|p| p.starts).count().to_string(),
                t("jr_tm_starts_hint"),
                theme::text(),
            ),
            stat(
                t("jr_tm_blocked"),
                blocked.to_string(),
                t("jr_tm_blocked_hint"),
                if blocked == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    if plan.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("jr_tm_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("jr_tomorrow")
                .num_columns(6)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 280.0, t("col_task"));
                    head_l(ui, 110.0, t("jr_tm_state"));
                    head_r(ui, 120.0, t("jr_tm_left"));
                    head_r(ui, 110.0, t("mat_kit_ready"));
                    head_r(ui, 100.0, t("mat_kit_missing"));
                    head_r(ui, 100.0, t("jr_tm_crew"));
                    ui.end_row();

                    for p in &plan {
                        let name = app
                            .task(p.task_id)
                            .map(|x| format!("{} {}", x.wbs, x.name))
                            .unwrap_or_default();
                        cell_l(
                            ui,
                            280.0,
                            RichText::new(super::issues::truncate(&name, 38)).size(12.5),
                        );
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(if p.starts {
                                t("jr_tm_new")
                            } else {
                                t("jr_tm_going")
                            })
                            .size(11.5)
                            .color(if p.starts {
                                theme::accent()
                            } else {
                                theme::muted()
                            }),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(super::materials::trim_num(p.volume_left)).size(12.0),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(format!("{:.0}%", p.kit_ready))
                                .size(12.5)
                                .color(if p.missing == 0 {
                                    theme::ok()
                                } else {
                                    theme::warn()
                                }),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(p.missing.to_string()).size(12.0).color(
                                if p.missing == 0 {
                                    theme::muted()
                                } else {
                                    theme::danger()
                                },
                            ),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(p.crew_today.to_string()).size(12.0).color(
                                if p.crew_today == 0 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                },
                            ),
                        );
                        ui.end_row();
                    }
                });
        });
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).size(11.0).color(theme::muted()));
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
    let photos: usize = app
        .journal
        .iter()
        .map(|e| photo_list(&e.photos).len())
        .sum();

    stat_row(
        ui,
        vec![
            stat(
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
                if gap > 2 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("jr_covered"),
                format!("{days_covered} / 30"),
                t("jr_covered_hint"),
                if days_covered >= 20 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("jr_avg_workers"),
                avg_workers.to_string(),
                t("jr_avg_workers_hint"),
                theme::accent(),
            ),
            stat(
                t("jr_photos"),
                photos.to_string(),
                t("jr_photos_hint"),
                if photos == 0 {
                    theme::muted()
                } else {
                    theme::accent()
                },
            ),
        ],
    );
}

/// Bugungi yozuv holati va uni qo'shish tugmasi.
///
/// Jurnal kunlik hujjat: bugungi yozuv bo'lmasa, bu birinchi navbatdagi
/// ish. Kechagi yozuv ham yo'q bo'lsa — bu allaqachon uzilish va u
/// ko'rinib turishi kerak.
fn today_line(ui: &mut egui::Ui, app: &App, add: &mut bool) {
    let has_today = app.journal.iter().any(|e| e.date == app.today);
    let last = app.journal.iter().map(|e| e.date).max();
    let gap = last.map(|d| (app.today - d).num_days());
    let writes = app.role().journal_role() == crate::roles::JournalRole::Writes
        && app.can_edit(Screen::Journal);

    let (text, colour) = if has_today {
        (t("jr_today_done").to_string(), theme::ok())
    } else {
        match gap {
            // Kechagi yozuv bor: bugungisi hali yozilmagan.
            Some(1) => (t("jr_today_missing").to_string(), theme::warn()),
            // Bir necha kun yozilmagan — bu uzilish.
            Some(days) if days > 1 => (
                format!("{} {days} {}", t("jr_gap"), t("days_short")),
                theme::danger(),
            ),
            _ => (t("jr_today_missing").to_string(), theme::warn()),
        }
    };

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · {}",
                        app.today.format("%d.%m.%Y"),
                        crate::i18n::weekday_name(app.today)
                    ))
                    .size(13.0)
                    .strong(),
                );
                ui.label(RichText::new(text).size(12.5).color(colour));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !has_today && writes && ui.button(t("jr_add_today")).clicked() {
                        *add = true;
                    }
                });
            });
        });
    ui.add_space(10.0);
}

/// Yangi yozuv qo'shadi va uni bugungi sana bilan ochadi.
fn add_entry(app: &mut App) {
    let Some(pid) = app.current else { return };
    app.db.insert_journal(&JournalEntry {
        id: 0,
        project_id: pid,
        date: app.today,
        author: app.user_name(),
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
    // Yangi yozuv ro'yxat boshida turadi va ajratib ko'rsatiladi.
    app.journal_focus = Some(app.today);
}

fn entries(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<JournalEntry> = None;
    let mut removed: Option<i64> = None;
    let mut add_today = false;
    // Qaysi kunga surilgani eslab qolinadi — takror surilmasin.
    let jump_key = egui::Id::new("jr_jumped_to");
    let jumped = ui.data(|d| d.get_temp::<chrono::NaiveDate>(jump_key));
    let mut scroll_done: Option<chrono::NaiveDate> = None;

    // Bugungi holat — ro'yxatning tepasida. Prorab ekranni ochganda
    // birinchi savoli shu: «bugungi yozuv bormi?». Ilgari bunga javob
    // ro'yxatni ko'rib chiqib topilardi.
    today_line(ui, app, &mut add_today);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = (ui.available_width() - 24.0).min(1000.0);
            for entry in &app.journal {
                let mut e = entry.clone();
                let mut changed = false;

                // Qidiruvdan kelingan kun ajratiladi va ko'rinishga suriladi.
                let focused = app.journal_focus == Some(e.date);
                let frame = egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(
                        if focused { 2.0_f32 } else { 1.0_f32 },
                        if focused {
                            theme::accent()
                        } else {
                            theme::line()
                        },
                    ))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(width);

                        // 1-qator: sana, muallif, ob-havo, resurs
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(t("col_date"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |=
                                super::passport::date_edit(ui, &format!("j{}", e.id), &mut e.date);
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t("col_author"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |= ui
                                .add_sized([170.0, 22.0], egui::TextEdit::singleline(&mut e.author))
                                .changed();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t("col_weather"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |= ui
                                .add_sized(
                                    [120.0, 22.0],
                                    egui::TextEdit::singleline(&mut e.weather),
                                )
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut e.temperature)
                                        .range(-60.0..=60.0)
                                        .suffix(" °C"),
                                )
                                .changed();
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(RichText::new("x").color(theme::danger()))
                                        .clicked()
                                    {
                                        removed = Some(e.id);
                                    }
                                },
                            );
                        });

                        // 2-qator: ish, hajm, resurs
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(t("col_task"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |=
                                task_picker(ui, app, ("j_task", e.id), &mut e.task_id, 280.0);
                            ui.label(
                                RichText::new(t("col_volume"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |= ui
                                .add(egui::DragValue::new(&mut e.volume).speed(0.5))
                                .changed();
                            changed |= ui
                                .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut e.unit))
                                .changed();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t("col_workers"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
                            changed |= ui
                                .add(egui::DragValue::new(&mut e.workers).range(0..=5000))
                                .changed();
                            ui.label(
                                RichText::new(t("col_machines"))
                                    .color(theme::muted())
                                    .size(12.0),
                            );
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
                                    .add_filter(t("photos"), &["jpg", "jpeg", "png", "bmp", "webp"])
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
                                            let img = egui::Image::new(format!("file://{path}"))
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
                                            .small_button(RichText::new("x").color(theme::danger()))
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
                // Topilgan kun bir marta ko'rinishga suriladi. Har kadrda
                // surish ro'yxatni qulflab qo'yardi: qidiruvdan keyin
                // boshqa yozuvga o'tib bo'lmasdi.
                if focused && jumped != Some(e.date) {
                    frame.response.scroll_to_me(Some(egui::Align::Center));
                    scroll_done = Some(e.date);
                }
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
    if add_today {
        add_entry(app);
    }
    if let Some(day) = scroll_done {
        ui.data_mut(|d| d.insert_temp(jump_key, day));
    }
}
