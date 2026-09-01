//! «Umumiy ko'rinish» — obyekt bo'yicha boshqaruv paneli.
//!
//! Tuzilishi yuqoridan pastga qarab «muhimdan mayda-chuydaga»:
//! sarlavha va shartnoma muddati chizig'i → asosiy ko'rsatkichlar →
//! bajarilish egri chizig'i (reja/fakt) va diqqat talab qiladigan holatlar →
//! muddati o'tganlar va bo'limlar → bugungi va yaqinlashayotgan ishlar.
//! Har bir blok tegishli modulga bosish bilan olib boradi.

use super::*;
use crate::model::Section;
use chrono::{Datelike, Duration, NaiveDate};
use egui::{vec2, Sense};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let Some(p) = app.project().cloned() else {
        ui.vertical_centered(|ui| {
            ui.add_space(140.0);
            ui.label(
                RichText::new(t("no_objects_yet"))
                    .size(20.0)
                    .color(theme::muted()),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(t("create_first_object")).color(theme::muted()));
        });
        return;
    };

    let mut goto_task: Option<i64> = None;
    let mut goto_screen: Option<Screen> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        header(ui, app, &p);
        ui.add_space(12.0);
        kpi_row(ui, app, &p);
        ui.add_space(14.0);

        let avail = ui.available_width() - 24.0;
        let right_w = (avail * 0.32).clamp(300.0, 430.0);
        let left_w = (avail - right_w - 24.0).max(320.0);

        // ---- Egri chiziq + diqqat paneli ----
        ui.horizontal_top(|ui| {
            card_frame(ui, t("sc_title"), left_w, |ui| {
                if app.tasks.is_empty() {
                    ui.add_space(60.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(t("no_tasks")).color(theme::muted()));
                    });
                    ui.add_space(60.0);
                } else {
                    s_curve(ui, app, 236.0);
                }
            });
            if let Some(s) = attention_card(ui, app, &p, right_w) {
                goto_screen = Some(s);
            }
        });

        ui.add_space(12.0);

        // ---- Muddati o'tganlar + bo'limlar ----
        ui.horizontal_top(|ui| {
            card_frame(ui, t("block_overdue"), left_w, |ui| {
                if let Some(id) = overdue_table(ui, app, &p) {
                    goto_task = Some(id);
                }
            });
            card_frame(ui, t("block_sections"), right_w, |ui| {
                sections_card(ui, app);
            });
        });

        ui.add_space(12.0);

        // ---- Tahlil xulosasi + sotuv (agar obyekt sotuvda bo'lsa) ----
        ui.horizontal_top(|ui| {
            card_frame(ui, t("block_analytics"), left_w, |ui| {
                if let Some(sc) = analytics_card(ui, app) {
                    goto_screen = Some(sc);
                }
            });
            card_frame(ui, t("block_sales"), right_w, |ui| {
                if sales_card(ui, app) {
                    goto_screen = Some(Screen::Sales);
                }
            });
        });

        ui.add_space(12.0);

        // ---- Bugungi ishlar + yaqin 14 kun ----
        ui.horizontal_top(|ui| {
            card_frame(ui, t("block_today"), left_w, |ui| {
                if let Some(id) = today_card(ui, app, &p) {
                    goto_task = Some(id);
                }
            });
            card_frame(ui, t("upcoming_title"), right_w, |ui| {
                upcoming_card(ui, app);
            });
        });

        ui.add_space(24.0);
    });

    if let Some(id) = goto_task {
        app.selected_task = Some(id);
        app.screen = Screen::Gantt;
        if let Some(c) = app.schedule.get(id) {
            app.timeline_offset = (c.es as f32 - 20.0).max(0.0);
        }
        app.scroll_to_task(id);
    }
    if let Some(s) = goto_screen {
        app.screen = s;
    }
}

// ================================================================ Sarlavha

/// Obyekt nomi, manzili va shartnoma muddati chizig'i: boshlanish — bugun — tugash.
fn header(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&p.name)
                    .size(22.0)
                    .strong()
                    .color(theme::text()),
            );
            if !p.address.is_empty() {
                ui.label(RichText::new(&p.address).size(12.5).color(theme::muted()));
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            contract_timeline(ui, app, p);
        });
    });
}

fn contract_timeline(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) {
    let total = (p.planned_end - p.start_date).num_days().max(1);
    let elapsed = (app.today - p.start_date).num_days().clamp(0, total);
    let frac = elapsed as f32 / total as f32;

    let w = 300.0f32.min(ui.available_width() * 0.5);
    ui.vertical(|ui| {
        ui.set_width(w);
        // Sanalar chizig'i.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(p.start_date.format("%d.%m.%y").to_string())
                    .size(10.5)
                    .color(theme::muted())
                    .monospace(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(p.planned_end.format("%d.%m.%y").to_string())
                        .size(10.5)
                        .color(theme::muted())
                        .monospace(),
                );
            });
        });
        // Polosa: o'tgan qism to'ldiriladi, bugun belgisi qo'yiladi.
        let (rect, _) = ui.allocate_exact_size(vec2(w, 8.0), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, theme::track());
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(rect.width() * frac, rect.height())),
            4.0,
            theme::accent().gamma_multiply(0.85),
        );
        let tx = rect.min.x + rect.width() * frac;
        painter.line_segment(
            [pos2(tx, rect.min.y - 2.0), pos2(tx, rect.max.y + 2.0)],
            Stroke::new(2.0_f32, theme::warn()),
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{elapsed} {}", t("tl_elapsed")))
                    .size(10.5)
                    .color(theme::muted()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let left = (total - elapsed).max(0);
                ui.label(
                    RichText::new(format!("{left} {}", t("tl_left")))
                        .size(10.5)
                        .color(theme::muted()),
                );
            });
        });
    });
}

// ================================================================ KPI

fn kpi_row(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) {
    let pr = &app.progress;
    let sched_end = p.start_date + Duration::days((app.schedule.project_days - 1).max(0));
    let crit_count = app
        .tasks
        .iter()
        .filter(|x| app.schedule.is_critical(x.id))
        .count();
    let behind = pr.fact_pct + 0.5 < pr.plan_pct;

    // Eng katta kechikish — «kim aybdor» jadvalining yuqori qatori.
    let max_delay = pr
        .overdue
        .iter()
        .filter_map(|id| app.schedule.get(*id))
        .map(|c| (app.today - (p.start_date + Duration::days(c.ef))).num_days())
        .max()
        .unwrap_or(0);

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_progress"),
            format!("{:.1} %", pr.fact_pct),
            &format!("{} {:.1} %", t("kpi_plan_is"), pr.plan_pct),
            if behind { theme::danger() } else { theme::ok() },
        );
        stat_card(
            ui,
            t("kpi_overdue"),
            pr.overdue.len().to_string(),
            &if pr.overdue.is_empty() {
                t("no_overdue_short").to_string()
            } else {
                format!("{}: {max_delay} {}", t("kpi_max_delay"), t("days_short"))
            },
            if pr.overdue.is_empty() {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        stat_card(
            ui,
            t("kpi_today"),
            pr.in_progress.len().to_string(),
            t("kpi_in_progress"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_critical"),
            format!("{crit_count} {}", t("kpi_tasks_count")),
            &format!(
                "{} {} {}",
                t("kpi_cpm_length"),
                app.schedule.project_days,
                t("days_short")
            ),
            theme::warn(),
        );
        stat_card(
            ui,
            t("kpi_gpr_end"),
            sched_end.format("%d.%m.%Y").to_string(),
            &format!("{} {}", t("kpi_contract"), p.planned_end.format("%d.%m.%Y")),
            if sched_end > p.planned_end {
                theme::danger()
            } else {
                theme::ok()
            },
        );
        let delay = pr.delay_days;
        stat_card(
            ui,
            t("kpi_forecast"),
            pr.forecast_end
                .unwrap_or(sched_end)
                .format("%d.%m.%Y")
                .to_string(),
            &if delay > 0 {
                format!("{} {delay} {}", t("kpi_delay"), t("days_short"))
            } else {
                t("kpi_on_track").to_string()
            },
            if delay > 0 {
                theme::danger()
            } else {
                theme::ok()
            },
        );
    });
}

// ================================================================ S-egri

/// Rejaviy bajarilish egri chizig'i, bugungi reja/fakt nuqtalari va prognoz.
///
/// Reja `cpm::progress` bilan bir xil formulada hisoblanadi: har bir ishning
/// ulushi davomiylik bo'yicha vaznlanadi — shunda egri chiziqdagi «bugungi
/// reja» KPI dagi son bilan aynan mos tushadi.
fn s_curve(ui: &mut egui::Ui, app: &App, height: f32) {
    let origin = app.origin();
    let days_axis = (app.schedule.project_days - 1).max(1);
    let today_off = (app.today - origin).num_days();
    let forecast_off = app
        .progress
        .forecast_end
        .map(|d| (d - origin).num_days())
        .unwrap_or(days_axis);
    // O'q oxiri: grafik ham, bugun ham, prognoz ham sig'sin.
    let axis_end = days_axis.max(today_off).max(forecast_off).max(1);

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let p = ui.painter_at(rect);

    let plot = Rect::from_min_max(
        pos2(rect.min.x + 38.0, rect.min.y + 10.0),
        pos2(rect.max.x - 10.0, rect.max.y - 22.0),
    );
    let x = |d: f32| plot.min.x + (d / axis_end as f32) * plot.width();
    let y = |pct: f32| plot.max.y - (pct / 100.0) * plot.height();

    // ---- To'r va foiz belgilari ----
    for g in [0.0f32, 25.0, 50.0, 75.0, 100.0] {
        let gy = y(g);
        p.line_segment(
            [pos2(plot.min.x, gy), pos2(plot.max.x, gy)],
            Stroke::new(0.5_f32, theme::row_line()),
        );
        p.text(
            pos2(plot.min.x - 6.0, gy),
            Align2::RIGHT_CENTER,
            format!("{g:.0}%"),
            egui::FontId::proportional(9.5),
            theme::muted(),
        );
    }

    // ---- Oy belgilari ----
    let mut d = NaiveDate::from_ymd_opt(origin.year(), origin.month(), 1).unwrap_or(origin);
    let mut last_x = f32::MIN;
    let mut first = true;
    while (d - origin).num_days() <= axis_end {
        let off = (d - origin).num_days();
        if off >= 0 {
            let mx = x(off as f32);
            if mx >= plot.min.x && mx <= plot.max.x - 16.0 && mx - last_x >= 46.0 {
                p.line_segment(
                    [pos2(mx, plot.max.y), pos2(mx, plot.max.y + 4.0)],
                    Stroke::new(1.0_f32, theme::line()),
                );
                let label = if first || d.month() == 1 {
                    format!("{} {}", crate::i18n::month(d.month()), d.year() % 100)
                } else {
                    crate::i18n::month(d.month()).to_string()
                };
                p.text(
                    pos2(mx + 3.0, plot.max.y + 12.0),
                    Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(9.5),
                    theme::muted(),
                );
                last_x = mx;
                first = false;
            }
        }
        d = next_month(d);
    }

    // ---- Reja egri chizig'i ----
    let total_w: f64 = app.tasks.iter().map(|x| x.duration.max(1) as f64).sum();
    let plan_at = |day: i64| -> f32 {
        if total_w <= 0.0 {
            return 0.0;
        }
        let mut done = 0.0;
        for task in &app.tasks {
            let Some(c) = app.schedule.get(task.id) else {
                continue;
            };
            let dur = task.duration.max(1);
            let elapsed = (day - c.es + 1).clamp(0, dur) as f64;
            done += dur as f64 * (elapsed / dur as f64);
        }
        ((done / total_w) * 100.0) as f32
    };

    let step = (axis_end / 260).max(1);
    let mut pts: Vec<egui::Pos2> = Vec::new();
    let mut day = 0;
    while day <= axis_end {
        pts.push(pos2(x(day as f32), y(plan_at(day))));
        day += step;
    }
    if pts.last().map(|q| q.x < plot.max.x - 0.5).unwrap_or(false) {
        pts.push(pos2(x(axis_end as f32), y(plan_at(axis_end))));
    }

    // Egri chiziq ostini yumshoq rangga bo'yaymiz — har bir segment trapetsiya.
    let fill = theme::accent().gamma_multiply(0.07);
    for w in pts.windows(2) {
        p.add(egui::Shape::convex_polygon(
            vec![
                w[0],
                w[1],
                pos2(w[1].x, plot.max.y),
                pos2(w[0].x, plot.max.y),
            ],
            fill,
            Stroke::NONE,
        ));
    }
    p.add(egui::Shape::line(
        pts,
        Stroke::new(2.0_f32, theme::accent()),
    ));

    // ---- Prognoz chizig'i ----
    let late = app.progress.delay_days > 0;
    if late {
        let fx = x(forecast_off.clamp(0, axis_end) as f32);
        dashed_v(
            &p,
            fx,
            plot.min.y,
            plot.max.y,
            Stroke::new(1.2_f32, theme::danger()),
        );
        p.text(
            pos2(fx - 4.0, plot.min.y + 8.0),
            Align2::RIGHT_CENTER,
            t("sc_forecast"),
            egui::FontId::proportional(9.5),
            theme::danger(),
        );
    }

    // ---- Bugun: vertikal chiziq, reja va fakt nuqtalari ----
    let tx = x(today_off.clamp(0, axis_end) as f32);
    p.line_segment(
        [pos2(tx, plot.min.y), pos2(tx, plot.max.y)],
        Stroke::new(1.2_f32, theme::warn().gamma_multiply(0.8)),
    );
    let plan_now = app.progress.plan_pct as f32;
    let fact_now = app.progress.fact_pct as f32;
    let fact_color = if fact_now + 0.5 < plan_now {
        theme::danger()
    } else {
        theme::ok()
    };

    // Reja va fakt orasidagi farq — ingichka bog'lovchi chiziq.
    p.line_segment(
        [pos2(tx, y(plan_now)), pos2(tx, y(fact_now))],
        Stroke::new(1.0_f32, theme::muted().gamma_multiply(0.6)),
    );
    p.circle_filled(pos2(tx, y(plan_now)), 4.0, theme::accent());
    p.circle_stroke(
        pos2(tx, y(plan_now)),
        4.0,
        Stroke::new(1.5_f32, theme::card()),
    );
    p.circle_filled(pos2(tx, y(fact_now)), 4.0, fact_color);
    p.circle_stroke(
        pos2(tx, y(fact_now)),
        4.0,
        Stroke::new(1.5_f32, theme::card()),
    );

    // Yozuvlar chap yoki o'ngga — chetga tiralib qolmasin.
    let dx = if tx > plot.max.x - 78.0 { -8.0 } else { 8.0 };
    let align = if dx < 0.0 {
        Align2::RIGHT_CENTER
    } else {
        Align2::LEFT_CENTER
    };
    p.text(
        pos2(tx + dx, y(plan_now) - 9.0),
        align,
        format!("{} {plan_now:.0}%", t("sc_plan")),
        egui::FontId::proportional(10.5),
        theme::accent(),
    );
    p.text(
        pos2(tx + dx, y(fact_now) + 9.0),
        align,
        format!("{} {fact_now:.0}%", t("sc_fact")),
        egui::FontId::proportional(10.5),
        fact_color,
    );

    // ---- Belgilar izohi ----
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let item = |ui: &mut egui::Ui, c: Color32, label: &str| {
            let (r, _) = ui.allocate_exact_size(vec2(14.0, 12.0), Sense::hover());
            ui.painter().line_segment(
                [pos2(r.min.x, r.center().y), pos2(r.max.x, r.center().y)],
                Stroke::new(2.5_f32, c),
            );
            ui.label(RichText::new(label).size(10.5).color(theme::muted()));
            ui.add_space(10.0);
        };
        item(ui, theme::accent(), t("sc_plan"));
        item(ui, fact_color, t("sc_fact"));
        item(ui, theme::warn(), t("today_marker"));
        if late {
            item(ui, theme::danger(), t("sc_forecast"));
        }
    });
}

fn dashed_v(p: &egui::Painter, x: f32, y0: f32, y1: f32, stroke: Stroke) {
    let mut y = y0;
    while y < y1 {
        let y2 = (y + 5.0).min(y1);
        p.line_segment([pos2(x, y), pos2(x, y2)], stroke);
        y += 9.0;
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

// ================================================================ Diqqat paneli

/// Barcha modullardan yig'ilgan «diqqat talab qiladi» ro'yxati.
/// Qatorga bosilganda tegishli modul ochiladi.
fn attention_card(
    ui: &mut egui::Ui,
    app: &App,
    p: &crate::model::Project,
    w: f32,
) -> Option<Screen> {
    use crate::domain::{IssueModule, IssueStatus, Severity};

    let mut goto = None;

    // (rang, son, matn, ekran)
    let mut items: Vec<(Color32, i64, String, Screen)> = Vec::new();

    if !app.progress.overdue.is_empty() {
        items.push((
            theme::danger(),
            app.progress.overdue.len() as i64,
            t("att_overdue_tasks").to_string(),
            Screen::Gantt,
        ));
    }

    let sched_end = p.start_date + Duration::days((app.schedule.project_days - 1).max(0));
    if sched_end > p.planned_end {
        items.push((
            theme::danger(),
            (sched_end - p.planned_end).num_days(),
            format!("{} ({})", t("att_gpr_late"), t("days_short")),
            Screen::Gantt,
        ));
    }

    // Tasdiqlanmagan karta bo'yicha bajarilayotgan ishlar.
    let started: Vec<i64> = app
        .tasks
        .iter()
        .filter(|x| x.progress > 0.0 || x.fact_start.is_some())
        .map(|x| x.id)
        .collect();
    let unapproved = app
        .ppr_docs
        .iter()
        .filter(|d| !d.approved && d.task_id.map(|id| started.contains(&id)).unwrap_or(false))
        .count();
    if unapproved > 0 {
        items.push((
            theme::warn(),
            unapproved as i64,
            t("att_ppr_unapproved").to_string(),
            Screen::Ppr,
        ));
    }

    // Ochiq kritik/jiddiy nomuvofiqliklar — modul bo'yicha ajratiladi.
    let open_bad = |m: IssueModule| {
        app.issues
            .iter()
            .filter(|i| {
                i.module == m
                    && i.status == IssueStatus::Open
                    && matches!(i.severity, Severity::Critical | Severity::Major)
            })
            .count() as i64
    };
    let prj = open_bad(IssueModule::Project);
    if prj > 0 {
        items.push((
            theme::danger(),
            prj,
            t("att_project_issues").to_string(),
            Screen::AiCheck,
        ));
    }
    let est = open_bad(IssueModule::Estimate);
    if est > 0 {
        items.push((
            theme::danger(),
            est,
            t("att_estimate_issues").to_string(),
            Screen::Estimate,
        ));
    }
    let ppr_issues = open_bad(IssueModule::Ppr);
    if ppr_issues > 0 {
        items.push((
            theme::warn(),
            ppr_issues,
            t("att_ppr_issues").to_string(),
            Screen::Ppr,
        ));
    }

    // Tugallangan, lekin ijro hujjati rasmiylashtirilmagan ishlar.
    let covered: std::collections::HashSet<i64> =
        app.exec_docs.iter().filter_map(|d| d.task_id).collect();
    let undocumented = app
        .tasks
        .iter()
        .filter(|x| x.progress >= 99.999 || x.fact_end.is_some())
        .filter(|x| !covered.contains(&x.id))
        .count();
    if undocumented > 0 {
        items.push((
            theme::warn(),
            undocumented as i64,
            t("att_missing_docs").to_string(),
            Screen::ExecDocs,
        ));
    }

    if !app.schedule.cycles.is_empty() {
        items.push((
            theme::danger(),
            app.schedule.cycles.len() as i64,
            t("att_cycles").to_string(),
            Screen::Gantt,
        ));
    }

    card_frame(ui, t("attention_title"), w, |ui| {
        if items.is_empty() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let (icon, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                let painter = ui.painter();
                painter.circle_filled(icon.center(), 9.0, theme::ok().gamma_multiply(0.18));
                super::draw_check(painter, icon.center(), 5.0, theme::ok(), 2.0);
                ui.label(
                    RichText::new(t("attention_empty"))
                        .size(13.0)
                        .color(theme::ok()),
                );
            });
            ui.add_space(8.0);
            return;
        }
        for (color, count, text, screen) in &items {
            if att_row(ui, *color, *count, text)
                .on_hover_text(text)
                .clicked()
            {
                goto = Some(*screen);
            }
            ui.add_space(2.0);
        }
        ui.add_space(2.0);
        ui.label(
            RichText::new(t("attention_hint"))
                .size(10.5)
                .color(theme::muted()),
        );
    });

    goto
}

fn att_row(ui: &mut egui::Ui, color: Color32, count: i64, text: &str) -> egui::Response {
    let h = 38.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 6.0, theme::line().gamma_multiply(0.5));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let badge = Rect::from_min_size(
            pos2(rect.min.x + 2.0, rect.center().y - 12.0),
            vec2(38.0, 24.0),
        );
        p.rect_filled(badge, 6.0, color.gamma_multiply(0.16));
        p.text(
            badge.center(),
            Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::proportional(13.5),
            color,
        );
        p.text(
            pos2(badge.max.x + 10.0, rect.center().y),
            Align2::LEFT_CENTER,
            super::issues::truncate(text, 44),
            egui::FontId::proportional(12.5),
            theme::text(),
        );
        p.text(
            pos2(rect.max.x - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "›",
            egui::FontId::proportional(16.0),
            theme::muted(),
        );
    }
    resp
}

// ================================================================ Muddati o'tganlar

fn overdue_table(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) -> Option<i64> {
    let pr = &app.progress;
    if pr.overdue.is_empty() {
        ui.label(RichText::new(t("no_overdue")).color(theme::ok()));
        return None;
    }
    let mut goto = None;

    // Eng katta kechikish yuqorida — «kim aybdor» savoliga birinchi javob.
    let mut rows: Vec<(i64, i64)> = pr
        .overdue
        .iter()
        .filter_map(|id| {
            app.schedule.get(*id).map(|c| {
                let end = p.start_date + Duration::days(c.ef);
                (*id, (app.today - end).num_days())
            })
        })
        .collect();
    rows.sort_by_key(|(_, late)| -late);

    let shown = rows.len().min(10);
    egui::Grid::new("overdue")
        .num_columns(5)
        .spacing([12.0, 6.0])
        .striped(true)
        .show(ui, |ui| {
            let head = |ui: &mut egui::Ui, s: &str| {
                ui.label(RichText::new(s).color(theme::muted()).size(12.0));
            };
            head(ui, t("col_task"));
            head(ui, t("col_section_short"));
            head(ui, t("col_responsible"));
            head(ui, t("col_deadline"));
            head(ui, t("col_overdue_by"));
            ui.end_row();

            for (id, late) in rows.iter().take(shown) {
                let Some(task) = app.task(*id) else { continue };
                let Some(c) = app.schedule.get(*id) else {
                    continue;
                };
                let end = p.start_date + Duration::days(c.ef);
                if ui
                    .add(
                        egui::Label::new(
                            RichText::new(super::issues::truncate(&task.name, 34))
                                .color(theme::text()),
                        )
                        .sense(Sense::click()),
                    )
                    .on_hover_text(t("open_in_gantt"))
                    .clicked()
                {
                    goto = Some(*id);
                }
                section_chip(ui, task.section);
                ui.label(
                    RichText::new(if task.responsible.is_empty() {
                        t("dash")
                    } else {
                        &task.responsible
                    })
                    .color(theme::muted()),
                );
                ui.label(end.format("%d.%m.%Y").to_string());
                ui.label(
                    RichText::new(format!("{late} {}", t("days_short")))
                        .color(if *late > 14 {
                            theme::danger()
                        } else {
                            theme::warn()
                        })
                        .strong(),
                );
                ui.end_row();
            }
        });

    if rows.len() > shown {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("+ {} {}", t("more_prefix"), rows.len() - shown))
                .size(11.5)
                .color(theme::muted()),
        );
    }
    goto
}

fn section_chip(ui: &mut egui::Ui, s: Section) {
    if s == Section::None {
        ui.label(RichText::new(t("dash")).color(theme::muted()));
        return;
    }
    let color = theme::section_color(s.color());
    let (rect, _) = ui.allocate_exact_size(vec2(38.0, 17.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 4.0, color.gamma_multiply(0.30));
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        s.label(),
        egui::FontId::proportional(11.0),
        color,
    );
}

// ================================================================ Bo'limlar

/// Bo'limlar kesimidagi bajarilish: fakt polosa, bugungi reja esa chiziqcha.
fn sections_card(ui: &mut egui::Ui, app: &App) {
    let origin = app.origin();
    let today_off = (app.today - origin).num_days();
    let mut any = false;

    for s in Section::ALL.into_iter().skip(1) {
        let tasks: Vec<_> = app.tasks.iter().filter(|x| x.section == s).collect();
        if tasks.is_empty() {
            continue;
        }
        any = true;
        let total: f64 = tasks.iter().map(|x| x.duration.max(1) as f64).sum();
        let fact: f64 = tasks
            .iter()
            .map(|x| x.duration.max(1) as f64 * x.progress / 100.0)
            .sum();
        let plan: f64 = tasks
            .iter()
            .map(|x| {
                let dur = x.duration.max(1);
                let elapsed = app
                    .schedule
                    .get(x.id)
                    .map(|c| (today_off - c.es + 1).clamp(0, dur))
                    .unwrap_or(0);
                dur as f64 * (elapsed as f64 / dur as f64)
            })
            .sum();
        let fact_pct = (fact / total * 100.0) as f32;
        let plan_pct = (plan / total * 100.0) as f32;
        let col = theme::section_color(s.color());

        ui.horizontal(|ui| {
            ui.add_sized(
                [42.0, 18.0],
                egui::Label::new(RichText::new(s.label()).color(col).strong().size(12.5)),
            );
            ui.add_sized(
                [48.0, 18.0],
                egui::Label::new(
                    RichText::new(format!("{} {}", tasks.len(), t("tasks_short")))
                        .color(theme::muted())
                        .size(11.0),
                ),
            );
            let bar_w = (ui.available_width() - 46.0).clamp(60.0, 240.0);
            let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 12.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(rect, 3.0, theme::track());
            p.rect_filled(
                Rect::from_min_size(
                    rect.min,
                    vec2(rect.width() * fact_pct / 100.0, rect.height()),
                ),
                3.0,
                col,
            );
            // Reja chizig'i: fakt undan orqada bo'lsa, farq darrov ko'rinadi.
            let px = rect.min.x + rect.width() * plan_pct / 100.0;
            p.line_segment(
                [pos2(px, rect.min.y - 2.0), pos2(px, rect.max.y + 2.0)],
                Stroke::new(2.0_f32, theme::text().gamma_multiply(0.55)),
            );
            ui.label(RichText::new(format!("{fact_pct:.0}%")).size(11.5).color(
                if fact_pct + 1.0 < plan_pct {
                    theme::danger()
                } else {
                    theme::text()
                },
            ));
        });
        ui.add_space(2.0);
    }

    if any {
        ui.add_space(4.0);
        ui.label(
            RichText::new(t("sections_plan_hint"))
                .size(10.5)
                .color(theme::muted()),
        );
    } else {
        ui.label(RichText::new(t("sections_empty")).color(theme::muted()));
    }
}

// ================================================================ Bugungi ishlar

fn today_card(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) -> Option<i64> {
    let pr = &app.progress;
    if pr.in_progress.is_empty() {
        ui.label(RichText::new(t("no_active_today")).color(theme::muted()));
        return None;
    }
    let mut goto = None;

    for id in &pr.in_progress {
        let Some(task) = app.task(*id) else { continue };
        let Some(c) = app.schedule.get(*id) else {
            continue;
        };
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    [280.0, 18.0],
                    egui::Label::new(
                        RichText::new(super::issues::truncate(&task.name, 34)).color(theme::text()),
                    )
                    .sense(Sense::click()),
                )
                .on_hover_text(t("open_in_gantt"))
                .clicked()
            {
                goto = Some(*id);
            }
            section_chip(ui, task.section);
            ui.add_sized(
                [150.0, 18.0],
                egui::Label::new(
                    RichText::new(if task.responsible.is_empty() {
                        t("dash")
                    } else {
                        &task.responsible
                    })
                    .color(theme::muted())
                    .size(12.0),
                ),
            );
            ui.label(
                RichText::new(crate::i18n::until(
                    &(p.start_date + Duration::days(c.ef))
                        .format("%d.%m.%Y")
                        .to_string(),
                ))
                .color(theme::muted())
                .size(11.5),
            );
            if c.critical {
                ui.label(
                    RichText::new(t("critical_path_lower"))
                        .color(theme::danger())
                        .size(11.5),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.0}%", task.progress))
                        .size(12.0)
                        .color(theme::text()),
                );
                let (rect, _) = ui.allocate_exact_size(vec2(70.0, 9.0), Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(rect, 3.0, theme::track());
                painter.rect_filled(
                    Rect::from_min_size(
                        rect.min,
                        vec2(rect.width() * (task.progress / 100.0) as f32, rect.height()),
                    ),
                    3.0,
                    if c.critical {
                        theme::danger()
                    } else {
                        theme::accent()
                    },
                );
            });
        });
        ui.add_space(2.0);
    }
    goto
}

// ================================================================ Yaqin 14 kun

/// Yaqin ikki haftada boshlanadigan yoki topshirilishi kerak bo'lgan ishlar.
fn upcoming_card(ui: &mut egui::Ui, app: &App) {
    let origin = app.origin();
    let today_off = (app.today - origin).num_days();
    let horizon = today_off + 14;

    // (sana, boshlanishmi, ish nomi, bo'lim, kritikmi)
    let mut events: Vec<(NaiveDate, bool, String, Section, bool)> = Vec::new();
    for task in &app.tasks {
        let done = task.progress >= 99.999 || task.fact_end.is_some();
        if done {
            continue;
        }
        let Some(c) = app.schedule.get(task.id) else {
            continue;
        };
        if task.progress <= 0.0 && c.es > today_off && c.es <= horizon {
            events.push((
                origin + Duration::days(c.es),
                true,
                task.name.clone(),
                task.section,
                c.critical,
            ));
        }
        if c.ef >= today_off && c.ef <= horizon {
            events.push((
                origin + Duration::days(c.ef),
                false,
                task.name.clone(),
                task.section,
                c.critical,
            ));
        }
    }
    events.sort_by_key(|(d, is_start, ..)| (*d, !*is_start));

    if events.is_empty() {
        ui.label(RichText::new(t("upcoming_empty")).color(theme::muted()));
        return;
    }

    let shown = events.len().min(8);
    for (date, is_start, name, section, critical) in events.iter().take(shown) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [42.0, 17.0],
                egui::Label::new(
                    RichText::new(date.format("%d.%m").to_string())
                        .monospace()
                        .size(11.5)
                        .color(theme::muted()),
                ),
            );
            let (ev, col) = if *is_start {
                (t("ev_start"), theme::accent())
            } else if *critical {
                (t("ev_end"), theme::danger())
            } else {
                (t("ev_end"), theme::warn())
            };
            ui.add_sized(
                [86.0, 17.0],
                egui::Label::new(RichText::new(ev).size(11.0).color(col)),
            );
            section_chip(ui, *section);
            ui.label(
                RichText::new(super::issues::truncate(name, 24))
                    .size(12.0)
                    .color(theme::text()),
            );
        });
        ui.add_space(1.0);
    }
    if events.len() > shown {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("+ {} {}", t("more_prefix"), events.len() - shown))
                .size(11.0)
                .color(theme::muted()),
        );
    }
}

// ================================================================ Tahlil xulosasi

/// Eng muhim topilmalar — to'liq ro'yxat «AI analitika» ekranida.
fn analytics_card(ui: &mut egui::Ui, app: &App) -> Option<Screen> {
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, &sales);
    let found = crate::analytics::findings(&inp);
    let score = crate::analytics::health(&found);

    let color = if score >= 85 {
        theme::ok()
    } else if score >= 60 {
        theme::warn()
    } else {
        theme::danger()
    };

    let mut go = None;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{score}"))
                .size(26.0)
                .strong()
                .color(color),
        );
        ui.vertical(|ui| {
            ui.label(RichText::new(t("an_health")).size(12.0));
            ui.label(
                RichText::new(format!("{} {}", found.len(), t("an_findings_count")))
                    .size(11.0)
                    .color(theme::muted()),
            );
        });
    });
    ui.add_space(8.0);

    if found.is_empty() {
        ui.label(
            RichText::new(t("an_nothing"))
                .size(12.5)
                .color(theme::ok()),
        );
    } else {
        for f in found.iter().take(4) {
            let c = match f.severity {
                crate::domain::Severity::Critical => theme::danger(),
                crate::domain::Severity::Major => theme::warn(),
                _ => theme::accent(),
            };
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(4.0, 14.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, c);
                ui.add_space(4.0);
                ui.label(
                    RichText::new(f.area.label())
                        .size(10.5)
                        .color(theme::muted()),
                );
                ui.label(RichText::new(super::issues::truncate(&f.fact, 58)).size(12.0));
            });
        }
    }

    ui.add_space(8.0);
    if ui.button(t("block_analytics_open")).clicked() {
        go = Some(Screen::Analytics);
    }
    go
}

// ================================================================ Sotuv

/// Sotuv holati qisqacha. Obyekt sotuvda bo'lmasa — buni ochiq aytadi.
fn sales_card(ui: &mut egui::Ui, app: &App) -> bool {
    if app.units.is_empty() {
        ui.label(
            RichText::new(t("cp_no_sales"))
                .size(12.0)
                .color(theme::muted()),
        );
        return false;
    }
    let s = app.sales();
    let pct = if s.units > 0 {
        s.sold as f64 / s.units as f64 * 100.0
    } else {
        0.0
    };

    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(360.0), 10.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, theme::track());
    p.rect_filled(
        Rect::from_min_size(
            rect.min,
            vec2(rect.width() * (pct / 100.0).clamp(0.0, 1.0) as f32, rect.height()),
        ),
        3.0,
        theme::accent(),
    );
    ui.add_space(6.0);

    let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
        ui.horizontal(|ui| {
            ui.add_sized(
                [150.0, 18.0],
                egui::Label::new(RichText::new(label).size(11.5).color(theme::muted())),
            );
            ui.label(RichText::new(value).size(12.5).color(color));
        });
    };
    line(
        ui,
        t("cl_sold"),
        format!("{} / {} ({pct:.0}%)", s.sold, s.units),
        theme::text(),
    );
    line(ui, t("us_free"), s.free.to_string(), theme::ok());
    line(ui, t("kpi_received"), money(s.received), theme::ok());
    line(
        ui,
        t("kpi_debt"),
        money(s.debt),
        if s.debt > 0.0 { theme::warn() } else { theme::ok() },
    );
    if s.overdue > 0.0 {
        line(ui, t("kpi_overdue_pay"), money(s.overdue), theme::danger());
    }

    ui.add_space(8.0);
    ui.button(t("block_sales_open")).clicked()
}
