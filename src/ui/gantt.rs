//! «GPR» ekrani (TZ I.2): ishlar jadvali, drag & drop bilan Gantt diagrammasi,
//! bog'lanishlar, kritik yo'l, plan/fakt va kechikishlar.

use super::*;
use crate::app::GanttScale;
use crate::i18n;
use crate::model::{LinkType, Section, Task};
use chrono::{Datelike, Duration, NaiveDate};
use egui::{pos2, vec2, Align2, Pos2, Rect, Sense, Stroke};

const ROW_H: f32 = 28.0;
const TABLE_W: f32 = 566.0;
const HEADER_H: f32 = 44.0;

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

    toolbar(ui, app);
    ui.add_space(6.0);
    summary_strip(ui, app);
    ui.add_space(8.0);

    let bottom = if app.selected_task.is_some() {
        278.0
    } else {
        0.0
    };
    let avail = ui.available_size() - vec2(0.0, bottom);
    let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    chart(&mut child, app);

    if let Some(id) = app.selected_task {
        ui.add_space(6.0);
        inspector(ui, app, id);
    }

    keyboard(ui, app);

    if app.gantt_analysis {
        analysis_window(ui.ctx(), app);
    }
}

/// Klaviatura bilan boshqarish. Matn maydoni faolligida ishlamaydi —
/// aks holda yozayotganda ishlar surilib ketardi.
fn keyboard(ui: &egui::Ui, app: &mut App) {
    if ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    let (esc, del, up, down, left, right, mods) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Delete),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
            i.modifiers,
        )
    });

    if esc {
        app.linking_from = None;
        app.link_dialog = None;
        app.selected_task = None;
    }
    let Some(id) = app.selected_task else {
        return;
    };

    if del {
        app.confirm_delete_task = Some(id);
    }
    // Ctrl+chap/o'ng — ishni bir kunga surish (mahkamlanadi).
    if mods.ctrl && left {
        app.nudge_task(id, -1);
    }
    if mods.ctrl && right {
        app.nudge_task(id, 1);
    }
    // Alt+yuqori/past — ro'yxatdagi tartibni o'zgartirish.
    if mods.alt && up {
        app.reorder_task(id, true);
    } else if mods.alt && down {
        app.reorder_task(id, false);
    } else if up || down {
        // Oddiy strelka — tanlovni ko'chirish.
        let visible: Vec<i64> = app.visible_tasks().iter().map(|x| x.id).collect();
        if let Some(i) = visible.iter().position(|x| *x == id) {
            let j = if up {
                i.checked_sub(1)
            } else {
                (i + 1 < visible.len()).then_some(i + 1)
            };
            if let Some(j) = j {
                app.selected_task = Some(visible[j]);
                app.scroll_to_task(visible[j]);
            }
        }
    }
}

fn toolbar(ui: &mut egui::Ui, app: &mut App) {
    // Tor oynada hamma narsa bitta qatorga sig'maydi va ustma-ust chiqadi,
    // shuning uchun panel ikkiga bo'linadi.
    let wide = ui.available_width() > 1180.0;

    ui.horizontal(|ui| {
        if ui.button(t("add_task")).clicked() {
            app.add_task();
        }
        if let Some(id) = app.selected_task {
            if ui.button(t("delete")).clicked() {
                app.confirm_delete_task = Some(id);
            }
            let linking = app.linking_from == Some(id);
            let btn = ui.selectable_label(linking, t("link_tasks"));
            if btn.clicked() {
                app.linking_from = if linking { None } else { Some(id) };
            }
            btn.on_hover_text(t("link_hint"));
        }

        ui.separator();
        ui.label(RichText::new(t("scale")).color(theme::muted()));
        for sc in GanttScale::ALL {
            if ui.selectable_label(app.scale == sc, sc.label()).clicked() {
                app.scale = sc;
                app.px_per_day = sc.px_per_day();
                app.fit_timeline = false;
                app.scroll_to_today();
            }
        }

        // Mahkamlangan ishlar grafikni CPM dan uzib qo'yadi — ularni bir joyda
        // ko'rsatib, bir bosishda bo'shatish imkonini beramiz.
        let pinned = app.tasks.iter().filter(|x| x.pinned).count();
        if pinned > 0 {
            ui.separator();
            if ui
                .button(RichText::new(format!("{} {pinned}", t("unpin_all"))).color(theme::warn()))
                .on_hover_text(t("unpin_hint"))
                .clicked()
            {
                app.unpin_all();
            }
        }

        if wide {
            filters(ui, app);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // TZ I.2: «kim aybdor, nima qilish kerak» — tahlil oynasi.
            let an = ui.selectable_label(
                app.gantt_analysis,
                RichText::new(t("gantt_analysis")).strong(),
            );
            if an.on_hover_text(t("an_hint")).clicked() {
                app.gantt_analysis = !app.gantt_analysis;
            }
            ui.separator();
            if ui.button(t("today_btn")).clicked() {
                app.scroll_to_today();
            }
            if ui
                .button(t("fit_btn"))
                .on_hover_text(t("fit_hint"))
                .clicked()
            {
                app.fit_timeline = true;
            }
        });
    });

    if !wide {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            filters(ui, app);
        });
    }
}

/// Filtrlar va qidiruv — keng oynada asosiy qatorda, torida pastda.
fn filters(ui: &mut egui::Ui, app: &mut App) {
    ui.checkbox(&mut app.show_critical_only, t("only_critical"));
    ui.checkbox(&mut app.filter_overdue, t("filter_overdue"));

    ui.separator();
    {
        egui::ComboBox::from_id_salt("sec_filter")
            .selected_text(if app.filter_section == Section::None {
                t("all_sections").to_string()
            } else {
                app.filter_section.label().to_string()
            })
            .width(130.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.filter_section, Section::None, t("all_sections"));
                for s in Section::ALL.into_iter().skip(1) {
                    ui.selectable_value(&mut app.filter_section, s, s.label());
                }
            });
    }

    ui.add(
        egui::TextEdit::singleline(&mut app.search)
            .hint_text(t("search_tasks"))
            .desired_width(180.0),
    );
}

fn summary_strip(ui: &mut egui::Ui, app: &mut App) {
    let pr = &app.progress;
    let sched_end = app.origin() + Duration::days((app.schedule.project_days - 1).max(0));
    let crit_count = app
        .tasks
        .iter()
        .filter(|t| app.schedule.is_critical(t.id))
        .count();

    let fact_color = if pr.fact_pct + 0.5 < pr.plan_pct {
        theme::danger()
    } else {
        theme::ok()
    };
    let delay = pr.delay_days;
    stat_row(
        ui,
        vec![
            stat(
                t("kpi_plan_today"),
                format!("{:.1} %", pr.plan_pct),
                t("kpi_by_durations"),
                theme::accent(),
            ),
            stat(
                t("kpi_fact"),
                format!("{:.1} %", pr.fact_pct),
                &format!(
                    "{} {:+.1} {}",
                    t("kpi_deviation"),
                    pr.fact_pct - pr.plan_pct,
                    t("kpi_pp")
                ),
                fact_color,
            ),
            stat(
                t("kpi_overdue_tasks"),
                pr.overdue.len().to_string(),
                t("kpi_overdue_hint"),
                if pr.overdue.is_empty() {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_critical"),
                format!("{crit_count} {}", t("kpi_tasks_count")),
                &format!(
                    "{} {} {}",
                    t("kpi_cpm_length"),
                    app.schedule.project_days,
                    t("days_short")
                ),
                theme::warn(),
            ),
            stat(
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
            ),
        ],
    );

    if !app.schedule.cycles.is_empty() {
        ui.label(
            RichText::new(format!(
                "{} {}. {}",
                t("cycle_warning"),
                app.schedule.cycles.len(),
                t("cycle_warning_tail")
            ))
            .color(theme::danger()),
        );
    }
}

/// Таблица работ слева + таймлайн справа, рисуются в одной системе координат.
fn chart(ui: &mut egui::Ui, app: &mut App) {
    let full = ui.available_rect_before_wrap();
    let painter = ui.painter_at(full);
    painter.rect_filled(full, 6.0, theme::canvas());

    let origin = app.origin();
    let visible: Vec<i64> = app.visible_tasks().iter().map(|t| t.id).collect();

    let time_rect = Rect::from_min_max(pos2(full.min.x + TABLE_W, full.min.y), full.max);
    let rows_top = full.min.y + HEADER_H;

    // ---- Butun grafikni oynaga sig'dirish ----
    if app.fit_timeline {
        let days = app.schedule.project_days.max(1) as f32;
        let usable = (time_rect.width() - 24.0).max(60.0);
        app.px_per_day = (usable / days).clamp(0.35, 60.0);
        app.timeline_offset = 0.0;
        app.row_offset = 0.0;
        app.fit_timeline = false;
    }

    // ---- Aylantirish ----
    // G'ildirak — ishlar ro'yxati bo'ylab; ro'yxat oynaga to'liq sig'sa,
    // g'ildirak vaqt o'qini suradi (aks holda u umuman ishlamayotgandek tuyulardi).
    // Shift yoki gorizontal g'ildirak — doim vaqt o'qi, Ctrl — zum.
    // Sichqonchaning chap yoki o'rta tugmasi bilan bo'sh joyni sudrash — panorama.
    let rows_fit = ((full.max.y - rows_top) / ROW_H).floor().max(1.0);
    let max_row_off = (visible.len() as f32 - rows_fit).max(0.0);

    // Vaqt o'qi bo'yicha chegara: grafik oxiridan keyingi bo'shliqqa chiqmaslik uchun.
    let visible_days = time_rect.width() / app.px_per_day;
    let max_time_off = (app.schedule.project_days as f32 - visible_days).max(0.0);
    let pan = |app: &mut App, days: f32| {
        app.timeline_offset = (app.timeline_offset + days).clamp(0.0, max_time_off);
    };

    let time_resp = ui.interact(
        time_rect,
        egui::Id::new("timeline"),
        Sense::click_and_drag(),
    );
    if ui.rect_contains_pointer(full) {
        let (d, modifiers) = ui.input(|i| (i.raw_scroll_delta, i.modifiers));
        if modifiers.ctrl && d.y != 0.0 {
            let factor = if d.y > 0.0 { 1.15 } else { 1.0 / 1.15 };
            app.px_per_day = (app.px_per_day * factor).clamp(0.3, 60.0);
        } else if d.x != 0.0 || (modifiers.shift && d.y != 0.0) {
            let step = if d.x != 0.0 { -d.x } else { -d.y };
            pan(app, step / app.px_per_day * 3.0);
        } else if d.y != 0.0 {
            if max_row_off > 0.0 {
                app.row_offset = (app.row_offset - d.y / ROW_H).clamp(0.0, max_row_off);
            } else {
                pan(app, -d.y / app.px_per_day * 3.0);
            }
        }
    }
    // Bo'sh joyni sudrash: polosalar keyinroq ro'yxatdan o'tadi, shuning uchun
    // ular ustidagi sudrash bu yerga tushmaydi.
    if time_resp.dragged_by(egui::PointerButton::Primary)
        || time_resp.dragged_by(egui::PointerButton::Middle)
    {
        let dx = time_resp.drag_delta().x;
        pan(app, -dx / app.px_per_day);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if time_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    // Filtr yoki oyna o'lchami o'zgarganda ro'yxat oxiridan chiqib ketmasin.
    app.row_offset = app.row_offset.clamp(0.0, max_row_off);
    app.timeline_offset = app.timeline_offset.clamp(0.0, max_time_off);
    let first_row = app.row_offset.round() as usize;

    // Копируем параметры таймлайна в локальные значения: замыкания ниже не должны
    // удерживать заимствование `app`, который дальше меняется.
    let ppd = app.px_per_day;
    let t_off = app.timeline_offset;
    let day_x = |day: f32| time_rect.min.x + (day - t_off) * ppd;

    // ---- Шапка таймлайна: месяцы и дни/недели ----
    painter.rect_filled(
        Rect::from_min_max(full.min, pos2(full.max.x, rows_top)),
        0.0,
        theme::panel(),
    );
    let header_painter = ui.painter_at(Rect::from_min_max(
        pos2(time_rect.min.x, full.min.y),
        pos2(full.max.x, full.max.y),
    ));
    draw_time_header(&header_painter, time_rect, rows_top, origin, app, &day_x);

    // Заголовки таблицы
    let th = |x: f32, s: &str| {
        painter.text(
            pos2(x, rows_top - 12.0),
            Align2::LEFT_CENTER,
            s,
            egui::FontId::proportional(11.0),
            theme::muted(),
        );
    };
    th(full.min.x + 8.0, t("col_num"));
    th(full.min.x + 40.0, t("col_task_name"));
    th(full.min.x + 264.0, t("col_section_short"));
    th(full.min.x + 308.0, t("col_start_short"));
    th(full.min.x + 358.0, t("col_end_short"));
    th(full.min.x + 408.0, t("col_days"));
    th(full.min.x + 440.0, t("col_done_pct"));
    th(full.min.x + 508.0, t("col_slack"));

    // ---- Строки ----
    let mut row_y: std::collections::HashMap<i64, f32> = std::collections::HashMap::new();
    let mut clicked_task: Option<i64> = None;
    let mut drag_action: Option<(i64, DragKind, f32)> = None;

    // Два painter-а: `rp` для строк таблицы, `tp` — только для области таймлайна.
    // Иначе полосы и стрелки связей, уходящие влево за границу, рисуются поверх таблицы.
    let rp = ui.painter_at(Rect::from_min_max(pos2(full.min.x, rows_top), full.max));
    let tp = ui.painter_at(Rect::from_min_max(
        pos2(time_rect.min.x, rows_top),
        full.max,
    ));

    for (i, &tid) in visible.iter().enumerate().skip(first_row) {
        let y = rows_top + (i - first_row) as f32 * ROW_H;
        if y + ROW_H > full.max.y {
            break;
        }
        row_y.insert(tid, y);
        let Some(task) = app.task(tid).cloned() else {
            continue;
        };
        let calc = app.schedule.get(tid);
        let selected = app.selected_task == Some(tid);
        let critical = app.schedule.is_critical(tid);
        let overdue = app.progress.overdue.contains(&tid);

        let row_rect = Rect::from_min_size(pos2(full.min.x, y), vec2(full.width(), ROW_H));
        if selected {
            rp.rect_filled(row_rect, 0.0, theme::accent().gamma_multiply(0.18));
        } else if i % 2 == 1 {
            rp.rect_filled(row_rect, 0.0, theme::row_alt());
        }
        rp.line_segment(
            [pos2(full.min.x, y + ROW_H), pos2(full.max.x, y + ROW_H)],
            Stroke::new(0.5_f32, theme::row_line()),
        );

        // --- левая таблица ---
        let cy = y + ROW_H / 2.0;
        let text = |x: f32, s: String, col: Color32, size: f32| {
            rp.text(
                pos2(full.min.x + x, cy),
                Align2::LEFT_CENTER,
                s,
                egui::FontId::proportional(size),
                col,
            );
        };
        text(8.0, task.wbs.clone(), theme::muted(), 12.0);
        let name_col = if overdue {
            theme::danger()
        } else {
            theme::text()
        };
        text(40.0, truncate(&task.name, 29), name_col, 13.0);
        if task.section != Section::None {
            let chip = Rect::from_min_size(pos2(full.min.x + 262.0, cy - 8.0), vec2(36.0, 16.0));
            rp.rect_filled(
                chip,
                4.0,
                theme::section_color(task.section.color()).gamma_multiply(0.35),
            );
            rp.text(
                chip.center(),
                Align2::CENTER_CENTER,
                task.section.label(),
                egui::FontId::proportional(11.0),
                theme::section_color(task.section.color()),
            );
        }
        if let Some(c) = calc {
            // CPM bo'yicha boshlanish va tugash — jadvalda darrov ko'rinadi.
            let sd = origin + Duration::days(c.es);
            let ed = origin + Duration::days(c.ef);
            text(308.0, sd.format("%d.%m").to_string(), theme::muted(), 11.5);
            text(
                358.0,
                ed.format("%d.%m").to_string(),
                if overdue {
                    theme::danger()
                } else {
                    theme::muted()
                },
                11.5,
            );
        }
        text(408.0, task.duration.to_string(), theme::text(), 12.0);
        // Bajarilish: son + mitti polosa.
        text(
            440.0,
            format!("{:.0}", task.progress),
            if task.progress >= 99.99 {
                theme::ok()
            } else {
                theme::text()
            },
            12.0,
        );
        {
            let pbar = Rect::from_min_size(pos2(full.min.x + 464.0, cy + 4.0), vec2(34.0, 4.0));
            rp.rect_filled(pbar, 1.5, theme::track());
            rp.rect_filled(
                Rect::from_min_size(
                    pbar.min,
                    vec2(pbar.width() * (task.progress / 100.0) as f32, pbar.height()),
                ),
                1.5,
                if task.progress >= 99.99 {
                    theme::ok()
                } else {
                    theme::accent()
                },
            );
        }
        if let Some(c) = calc {
            text(
                508.0,
                if critical {
                    t("crit_short").to_string()
                } else {
                    format!("{}", c.slack)
                },
                if critical {
                    theme::danger()
                } else {
                    theme::muted()
                },
                11.0,
            );
        }

        // клик по строке таблицы
        let row_resp = ui.interact(
            Rect::from_min_size(pos2(full.min.x, y), vec2(TABLE_W, ROW_H)),
            egui::Id::new(("row", tid)),
            Sense::click(),
        );
        if row_resp.clicked() {
            clicked_task = Some(tid);
        }

        // --- полоса на таймлайне ---
        let Some(c) = calc else { continue };
        let x0 = day_x(c.es as f32);
        let x1 = day_x((c.ef + 1) as f32);
        if x1 < time_rect.min.x || x0 > time_rect.max.x {
            continue;
        }
        let bar = Rect::from_min_max(pos2(x0, y + 5.0), pos2(x1.max(x0 + 3.0), y + ROW_H - 6.0));

        let base = if critical {
            theme::danger()
        } else if task.section == Section::None {
            theme::arrow()
        } else {
            theme::section_color(task.section.color())
        };
        tp.rect_filled(bar, 3.0, base.gamma_multiply(theme::bar_track_alpha()));
        // фактическое выполнение — заливка внутри полосы
        if task.progress > 0.0 {
            let w = bar.width() * (task.progress / 100.0) as f32;
            tp.rect_filled(
                Rect::from_min_size(bar.min, vec2(w, bar.height())),
                3.0,
                base.gamma_multiply(theme::bar_fill_alpha()),
            );
        }
        tp.rect_stroke(
            bar,
            3.0,
            Stroke::new(
                if selected { 2.0_f32 } else { 1.0_f32 },
                if selected { theme::accent() } else { base },
            ),
            egui::StrokeKind::Inside,
        );
        if task.pinned {
            // Mahkamlangan ish: polosa oldida kichik sariq kvadrat.
            tp.rect_filled(
                Rect::from_center_size(pos2(bar.min.x - 7.0, cy), vec2(6.0, 6.0)),
                1.5,
                theme::warn(),
            );
        }
        // Nom faqat polosa ichiga sig'sa yoziladi. Tashqariga chiqarilsa,
        // diagramma matn bilan to'lib ketardi — nomlar jadvalda bor.
        if bar.width() > 60.0 {
            tp.text(
                pos2(bar.min.x + 6.0, cy),
                Align2::LEFT_CENTER,
                truncate(&task.name, (bar.width() / 7.0) as usize),
                egui::FontId::proportional(11.0),
                theme::bar_label(),
            );
        }

        // --- взаимодействие с полосой: перемещение и растягивание ---
        let bar_resp = ui
            .interact(bar, egui::Id::new(("bar", tid)), Sense::click_and_drag())
            .on_hover_ui(|ui| {
                // Matn faqat kursor polosa ustida turganda quriladi.
                ui.label(hover_text(&task, &c, app));
            });
        let edge = 6.0f32.min(bar.width() / 3.0);
        let pointer = ui.input(|i| i.pointer.hover_pos()).unwrap_or(bar.center());
        let hover_kind = if pointer.x >= bar.max.x - edge {
            DragKind::ResizeEnd
        } else if pointer.x <= bar.min.x + edge {
            DragKind::ResizeStart
        } else {
            DragKind::Move
        };
        if bar_resp.hovered() {
            ui.ctx().set_cursor_icon(match hover_kind {
                DragKind::Move => egui::CursorIcon::Grab,
                _ => egui::CursorIcon::ResizeHorizontal,
            });
        }
        if bar_resp.clicked() {
            clicked_task = Some(tid);
        }
        // Tur sudrash boshlangan paytda qulflanadi: aks holda polosa siljishi
        // bilan kursor chekkadan chiqib, cho'zish ko'chirishga aylanib ketardi.
        let kind_id = egui::Id::new(("drag_kind", tid));
        if bar_resp.drag_started() {
            app.drag_acc = 0.0; // oldingi sudrashdan qolgan qoldiq hisobga olinmasin
            ui.data_mut(|d| d.insert_temp(kind_id, hover_kind));
        }
        if bar_resp.dragged() {
            let kind = ui
                .data(|d| d.get_temp::<DragKind>(kind_id))
                .unwrap_or(hover_kind);
            let dx = bar_resp.drag_delta().x;
            if dx.abs() > 0.01 {
                drag_action = Some((tid, kind, dx));
            }
        }
        if bar_resp.drag_stopped() {
            ui.data_mut(|d| d.remove::<DragKind>(kind_id));
            app.drag_acc = 0.0;
            app.recompute();
        }
    }

    // ---- Стрелки связей ----
    let sel = app.selected_task;
    let any_selected = sel.is_some();
    for l in app.links.clone() {
        let (Some(&yp), Some(&ys)) = (row_y.get(&l.pred), row_y.get(&l.succ)) else {
            continue;
        };
        let (Some(cp), Some(cs)) = (app.schedule.get(l.pred), app.schedule.get(l.succ)) else {
            continue;
        };
        let critical = app.schedule.is_critical(l.pred) && app.schedule.is_critical(l.succ);
        // Ish tanlanganda uning bog'lanishlari ajraladi, qolganlari xiralashadi —
        // aks holda strelkalar bir-biriga qo'shilib o'qib bo'lmas edi.
        let touches = sel == Some(l.pred) || sel == Some(l.succ);
        let col = if critical {
            theme::danger().gamma_multiply(0.85)
        } else {
            theme::arrow()
        };
        let col = if any_selected && !touches {
            col.gamma_multiply(0.30)
        } else if touches {
            theme::accent()
        } else {
            col
        };
        let (from_day, to_day) = match l.kind {
            LinkType::Fs => ((cp.ef + 1) as f32, cs.es as f32),
            LinkType::Ss => (cp.es as f32, cs.es as f32),
            LinkType::Ff => ((cp.ef + 1) as f32, (cs.ef + 1) as f32),
            LinkType::Sf => (cp.es as f32, (cs.ef + 1) as f32),
        };
        arrow(
            &tp,
            pos2(day_x(from_day), yp + ROW_H / 2.0),
            pos2(day_x(to_day), ys + ROW_H / 2.0),
            col,
        );
    }

    // ---- Линия «сегодня» ----
    let tx = day_x((app.today - origin).num_days() as f32);
    if tx > time_rect.min.x && tx < time_rect.max.x {
        tp.line_segment(
            [pos2(tx, rows_top), pos2(tx, full.max.y)],
            Stroke::new(1.5_f32, theme::warn().gamma_multiply(0.8)),
        );
        // Yozuvni pastga qo'yamiz: yuqorida oy nomlari bilan ustma-ust tushardi.
        tp.text(
            pos2(tx, full.max.y - 6.0),
            Align2::CENTER_BOTTOM,
            t("today_marker"),
            egui::FontId::proportional(10.0),
            theme::warn(),
        );
    }

    // Разделитель таблицы и таймлайна
    painter.line_segment(
        [
            pos2(time_rect.min.x, full.min.y),
            pos2(time_rect.min.x, full.max.y),
        ],
        Stroke::new(1.0_f32, theme::line()),
    );

    // Vertikal aylantirish indikatori — ro'yxat oynaga sig'masa ko'rinadi.
    if max_row_off > 0.0 {
        let track = Rect::from_min_max(
            pos2(time_rect.min.x - 7.0, rows_top),
            pos2(time_rect.min.x - 2.0, full.max.y),
        );
        painter.rect_filled(track, 2.0, theme::track());
        let frac = rows_fit / visible.len() as f32;
        let h = (track.height() * frac).max(18.0);
        let top = track.min.y + (track.height() - h) * (app.row_offset / max_row_off);
        painter.rect_filled(
            Rect::from_min_size(pos2(track.min.x, top), vec2(track.width(), h)),
            2.0,
            theme::muted(),
        );
    }

    // ---- Bog'lash rejimi: polosadan kursorgacha chiziq ----
    if let Some(pred) = app.linking_from {
        if let (Some(&yp), Some(cp)) = (row_y.get(&pred), app.schedule.get(pred)) {
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let from = pos2(day_x((cp.ef + 1) as f32), yp + ROW_H / 2.0);
                tp.line_segment([from, pointer], Stroke::new(1.5_f32, theme::accent()));
                tp.circle_filled(from, 3.0, theme::accent());
                tp.text(
                    pointer + vec2(12.0, -10.0),
                    Align2::LEFT_CENTER,
                    t("linking_cursor"),
                    egui::FontId::proportional(11.0),
                    theme::accent(),
                );
                ui.ctx().request_repaint();
            }
        }
    }

    // ---- Gorizontal aylantirish indikatori ----
    let total_days = (app.schedule.project_days.max(1)) as f32;
    let visible_days = time_rect.width() / ppd;
    if total_days > visible_days {
        let track = Rect::from_min_max(
            pos2(time_rect.min.x + 4.0, full.max.y - 7.0),
            pos2(time_rect.max.x - 4.0, full.max.y - 2.0),
        );
        painter.rect_filled(track, 2.0, theme::track());
        let frac = (visible_days / total_days).clamp(0.02, 1.0);
        let thumb_w = (track.width() * frac).max(24.0);
        let max_off = (total_days - visible_days).max(1.0);
        let tx0 = track.min.x + (track.width() - thumb_w) * (t_off / max_off).clamp(0.0, 1.0);
        let thumb = Rect::from_min_size(pos2(tx0, track.min.y), vec2(thumb_w, track.height()));
        painter.rect_filled(thumb, 2.0, theme::muted());
        // Belgini sudrab aylantirish mumkin.
        let hresp = ui.interact(
            thumb.expand2(vec2(0.0, 4.0)),
            egui::Id::new("gantt_hscroll"),
            Sense::drag(),
        );
        if hresp.dragged() {
            let per_px = max_off / (track.width() - thumb_w).max(1.0);
            app.timeline_offset =
                (app.timeline_offset + hresp.drag_delta().x * per_px).clamp(0.0, max_off);
        }
    }

    // ---- Применяем перетаскивание ----
    if let Some((tid, kind, dx)) = drag_action {
        apply_drag(app, tid, kind, dx);
    }

    // ---- Bo'sh joyga bosish: tanlovni bekor qilish ----
    if clicked_task.is_none() && time_resp.clicked() {
        app.selected_task = None;
        app.linking_from = None;
    }

    // ---- Клик: выбор работы либо завершение связывания ----
    if let Some(tid) = clicked_task {
        match app.linking_from {
            Some(pred) if pred != tid => {
                app.link_dialog = Some(crate::model::Link {
                    id: 0,
                    pred,
                    succ: tid,
                    kind: LinkType::Fs,
                    lag: 0,
                });
            }
            _ => {
                app.selected_task = Some(tid);
                app.linking_from = None;
            }
        }
    }
}

// ================================================================ Tahlil

/// TZ I.2 savollariga javob: qaysi ishlar kechikkan, kim mas'ul va
/// obyektni vaqtida topshirish uchun nima qilish kerak.
/// Hisob GPR, CPM va plan/fakt ma'lumotidan avtomatik yig'iladi.
fn analysis_window(ctx: &egui::Context, app: &mut App) {
    let origin = app.origin();
    let today_off = (app.today - origin).num_days();

    // Mas'ullar kesimi: nechta ish, jami necha kun kechikkan.
    let mut blame: std::collections::HashMap<String, (usize, i64)> =
        std::collections::HashMap::new();
    let mut overdue_rows: Vec<(String, String, i64)> = Vec::new();
    for id in &app.progress.overdue {
        let Some(task) = app.task(*id) else { continue };
        let Some(c) = app.schedule.get(*id) else {
            continue;
        };
        let late = (today_off - c.ef).max(0);
        let resp = if task.responsible.trim().is_empty() {
            t("dash").to_string()
        } else {
            task.responsible.clone()
        };
        let e = blame.entry(resp.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += late;
        overdue_rows.push((task.name.clone(), resp, late));
    }
    overdue_rows.sort_by_key(|(_, _, l)| -l);
    let mut blame_rows: Vec<(String, usize, i64)> =
        blame.into_iter().map(|(k, (n, d))| (k, n, d)).collect();
    blame_rows.sort_by_key(|(_, _, d)| -d);

    // Sur'at: joriy tezlikda yetib olish uchun qancha tezlashish kerak.
    let pr = &app.progress;
    let behind = pr.fact_pct + 0.5 < pr.plan_pct;
    let elapsed = today_off.max(1) as f64;
    let remaining_days = ((app.schedule.project_days - 1) - today_off).max(1) as f64;
    let current_rate = pr.fact_pct / elapsed;
    let needed_rate = (100.0 - pr.fact_pct) / remaining_days;
    let boost = if current_rate > 0.01 {
        needed_rate / current_rate
    } else {
        0.0
    };

    // Kritik yo'ldagi navbatdagi ishlar.
    let mut crit_next: Vec<(chrono::NaiveDate, String, String)> = app
        .tasks
        .iter()
        .filter(|x| x.progress < 99.999 && x.fact_end.is_none())
        .filter(|x| app.schedule.is_critical(x.id))
        .filter_map(|x| {
            app.schedule
                .get(x.id)
                .filter(|c| c.ef >= today_off)
                .map(|c| {
                    (
                        origin + Duration::days(c.es.max(today_off)),
                        x.name.clone(),
                        x.responsible.clone(),
                    )
                })
        })
        .collect();
    crit_next.sort_by_key(|(d, ..)| *d);

    let mut open = app.gantt_analysis;
    egui::Window::new(t("an_title"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .anchor(egui::Align2::RIGHT_TOP, [-16.0, 64.0])
        .show(ctx, |ui| {
            let section = |ui: &mut egui::Ui, title: &str, color: Color32| {
                ui.add_space(6.0);
                ui.label(RichText::new(title).size(13.0).strong().color(color));
                ui.add_space(3.0);
            };

            // ---- 1. Muddati o'tganlar ----
            section(ui, t("an_overdue"), theme::danger());
            if overdue_rows.is_empty() {
                ui.label(
                    RichText::new(t("an_none_overdue"))
                        .color(theme::ok())
                        .size(12.5),
                );
            } else {
                for (name, resp, late) in overdue_rows.iter().take(6) {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{late} {}", t("days_short")))
                                .color(theme::danger())
                                .strong()
                                .size(12.0),
                        );
                        ui.label(RichText::new(truncate(name, 34)).size(12.5));
                        ui.label(
                            RichText::new(format!("— {resp}"))
                                .color(theme::muted())
                                .size(12.0),
                        );
                    });
                }
                if overdue_rows.len() > 6 {
                    ui.label(
                        RichText::new(format!("+ {} {}", t("more_prefix"), overdue_rows.len() - 6))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                }
            }

            // ---- 2. Mas'ullar kesimida ----
            if !blame_rows.is_empty() {
                section(ui, t("an_blame"), theme::warn());
                let max_d = blame_rows.first().map(|(_, _, d)| *d).unwrap_or(1).max(1);
                for (resp, n, total) in blame_rows.iter().take(5) {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [150.0, 16.0],
                            egui::Label::new(RichText::new(truncate(resp, 20)).size(12.5)),
                        );
                        let bw = 160.0 * (*total as f32 / max_d as f32);
                        let (r, _) =
                            ui.allocate_exact_size(egui::vec2(bw.max(4.0), 10.0), Sense::hover());
                        ui.painter().rect_filled(r, 3.0, theme::warn());
                        ui.label(
                            RichText::new(format!(
                                "{n} {} · {total} {}",
                                t("tasks_short"),
                                t("days_short")
                            ))
                            .size(11.5)
                            .color(theme::muted()),
                        );
                    });
                }
            }

            // ---- 3. Nima qilish kerak ----
            section(ui, t("an_todo"), theme::accent());
            if behind {
                ui.label(
                    RichText::new(format!(
                        "{} {} {}",
                        t("an_delay_fc"),
                        pr.delay_days.max(0),
                        t("days_short")
                    ))
                    .size(12.5)
                    .color(theme::danger()),
                );
                if boost > 1.05 {
                    ui.label(
                        RichText::new(format!("{} ×{boost:.2}", t("an_pace")))
                            .size(12.5)
                            .color(theme::warn()),
                    );
                }
            } else {
                ui.label(RichText::new(t("an_pace_ok")).size(12.5).color(theme::ok()));
            }
            if !crit_next.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t("an_crit_next"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                for (d, name, resp) in crit_next.iter().take(5) {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(d.format("%d.%m").to_string())
                                .monospace()
                                .size(11.5)
                                .color(theme::danger()),
                        );
                        ui.label(RichText::new(truncate(name, 32)).size(12.5));
                        if !resp.is_empty() {
                            ui.label(
                                RichText::new(format!("— {resp}"))
                                    .color(theme::muted())
                                    .size(11.5),
                            );
                        }
                    });
                }
            }

            ui.add_space(6.0);
            ui.label(RichText::new(t("an_hint")).size(10.5).color(theme::muted()));
        });
    app.gantt_analysis = open;
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragKind {
    Move,
    ResizeStart,
    ResizeEnd,
}

/// Перетаскивание полосы: сдвиг даты (с закреплением) или изменение длительности.
fn apply_drag(app: &mut App, tid: i64, kind: DragKind, dx: f32) {
    let ppd = app.px_per_day;
    let Some(mut task) = app.task(tid).cloned() else {
        return;
    };

    // Дробный остаток копится между кадрами: при мелком масштабе один кадр
    // даёт меньше пикселя на день, и без накопления полоса бы не двигалась.
    app.drag_acc += dx / ppd;
    let days = app.drag_acc.trunc() as i64;
    if days == 0 {
        return;
    }
    app.drag_acc -= days as f32;

    match kind {
        DragKind::Move => {
            task.plan_start += Duration::days(days);
            task.pinned = true; // явный сдвиг фиксирует работу, иначе CPM вернет ее назад
        }
        DragKind::ResizeEnd => {
            task.duration = (task.duration + days).max(1);
        }
        DragKind::ResizeStart => {
            let new_dur = (task.duration - days).max(1);
            let delta = task.duration - new_dur;
            task.duration = new_dur;
            task.plan_start += Duration::days(delta);
            task.pinned = true;
        }
    }
    app.save_task(task);
}

fn hover_text(task: &Task, c: &crate::cpm::Calc, app: &App) -> String {
    let origin = app.origin();
    let s = origin + Duration::days(c.es);
    let e = origin + Duration::days(c.ef);
    let mut txt = format!(
        "{}\n{}: {}\n{}: {}\n{}: {} — {} ({} {})\n{}: {:.0}%\n{}: {} {}",
        task.name,
        i18n::t("task_section"),
        task.section.label(),
        i18n::t("task_responsible"),
        if task.responsible.is_empty() {
            i18n::t("dash")
        } else {
            &task.responsible
        },
        i18n::t("hover_plan"),
        s.format("%d.%m.%Y"),
        e.format("%d.%m.%Y"),
        task.duration,
        i18n::t("days_short"),
        i18n::t("hover_done"),
        task.progress,
        i18n::t("hover_slack"),
        c.slack,
        i18n::t("days_short")
    );
    if c.critical {
        txt.push('\n');
        txt.push_str(i18n::t("hover_critical"));
    }
    if app.progress.overdue.contains(&task.id) {
        txt.push('\n');
        txt.push_str(i18n::t("hover_overdue"));
    }
    txt
}

/// Pastki qatordagi belgilar ko'rinishi.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tick {
    /// Faqat kun raqami: «29».
    Day,
    /// Kun va oy: «29.04».
    DayMonth,
    /// Oy va yil: «apr 26».
    MonthYear,
    /// Faqat yil.
    Year,
}

/// Qadamni masshtabga emas, haqiqiy piksel kengligiga qarab tanlaydi.
/// Aks holda zum kichrayganda yozuvlar bir-birining ustiga chiqib ketardi.
fn pick_tick(ppd: f32) -> (i64, Tick) {
    // (kun qadami, yozuv uchun kerakli minimal kenglik, ko'rinish)
    const TABLE: &[(i64, f32, Tick)] = &[
        (1, 22.0, Tick::Day),
        (2, 22.0, Tick::Day),
        (7, 30.0, Tick::Day),
        (7, 50.0, Tick::DayMonth),
        (14, 50.0, Tick::DayMonth),
        (30, 52.0, Tick::MonthYear),
        (91, 52.0, Tick::MonthYear),
        (182, 44.0, Tick::Year),
        (365, 44.0, Tick::Year),
    ];
    TABLE
        .iter()
        .find(|(step, min_px, _)| *step as f32 * ppd >= *min_px)
        .map(|(s, _, f)| (*s, *f))
        .unwrap_or((365, Tick::Year))
}

/// Шапка таймлайна: месяцы сверху, деления снизу.
/// Ikkala qatorda ham yozuvlar orasida yetarli joy bo'lmasa, ular tashlab
/// ketiladi — sarlavha hech qachon «tiqilib» qolmaydi.
fn draw_time_header(
    painter: &egui::Painter,
    time_rect: Rect,
    rows_top: f32,
    origin: NaiveDate,
    app: &App,
    day_x: &impl Fn(f32) -> f32,
) {
    let ppd = app.px_per_day;
    let first_day = app.timeline_offset.floor() as i64;
    let span = (time_rect.width() / ppd).ceil() as i64 + 2;

    let mut month_start = origin + Duration::days(first_day);
    month_start =
        NaiveDate::from_ymd_opt(month_start.year(), month_start.month(), 1).unwrap_or(month_start);
    let end = origin + Duration::days(first_day + span);
    let (step, fmt) = pick_tick(ppd);

    // ---- Yuqori qator ----
    // Ikki qator bir-birini to'ldiradi: pastda kunlar bo'lsa yuqorida oylar,
    // pastda oylar bo'lsa yuqorida yillar turadi. Aks holda ikkalasi bir xil
    // ma'lumotni takrorlardi.
    let mut last_label_x = f32::MIN;
    let mut top_cell = |x: f32, label: String, min_gap: f32| {
        if x >= time_rect.min.x {
            painter.line_segment(
                [pos2(x, time_rect.min.y + 2.0), pos2(x, rows_top)],
                Stroke::new(1.0_f32, theme::line()),
            );
        }
        let lx = x.max(time_rect.min.x) + 5.0;
        if lx - last_label_x >= min_gap && lx < time_rect.max.x - 26.0 {
            painter.text(
                pos2(lx, time_rect.min.y + 12.0),
                Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(12.0),
                theme::text(),
            );
            last_label_x = lx;
        }
    };

    match fmt {
        // Pastda kun — yuqorida oy.
        Tick::Day | Tick::DayMonth => {
            let month_px = 30.0 * ppd;
            let mut d = month_start;
            while d <= end {
                let x = day_x((d - origin).num_days() as f32);
                if x >= time_rect.max.x {
                    break;
                }
                if x >= time_rect.min.x - 300.0 {
                    let label = if month_px >= 62.0 {
                        format!("{} {}", i18n::month(d.month()), d.year())
                    } else {
                        i18n::month(d.month()).to_string()
                    };
                    top_cell(x, label, 34.0);
                }
                d = next_month(d);
            }
        }
        // Pastda oy — yuqorida yil.
        Tick::MonthYear | Tick::Year => {
            let mut y = origin.year();
            // Ko'rinadigan birinchi yildan boshlaymiz.
            if let Some(first) = origin.checked_add_signed(Duration::days(first_day)) {
                y = first.year();
            }
            while let Some(jan) = NaiveDate::from_ymd_opt(y, 1, 1) {
                if jan > end {
                    break;
                }
                let x = day_x((jan - origin).num_days() as f32);
                if x < time_rect.max.x && x >= time_rect.min.x - 400.0 {
                    top_cell(x, y.to_string(), 40.0);
                }
                y += 1;
            }
        }
    }

    // ---- Dam olish kunlari ----
    // Faqat kunlar ajralib turadigan masshtabda bo'yaymiz.
    if app.settings.show_weekends && ppd >= 4.0 {
        let mut day = first_day;
        while day <= first_day + span {
            let date = origin + Duration::days(day);
            if matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun) {
                let x = day_x(day as f32);
                if x >= time_rect.min.x && x < time_rect.max.x {
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x, rows_top), vec2(ppd, 4000.0)),
                        0.0,
                        theme::weekend(),
                    );
                }
            }
            day += 1;
        }
    }

    // ---- Pastki qator: sana belgilari ----
    if fmt == Tick::MonthYear || fmt == Tick::Year {
        // Oy va yil belgilarini haqiqiy oy boshiga qo'yamiz — «30 kun» qadami
        // sanalarni oy o'rtasiga surib yuborardi.
        let mut d = month_start;
        let mut last_x = f32::MIN;
        while d <= end {
            let x = day_x((d - origin).num_days() as f32);
            let show = fmt == Tick::MonthYear || d.month() == 1;
            if show && x >= time_rect.min.x && x < time_rect.max.x - 20.0 && x - last_x >= 46.0 {
                tick(painter, x, rows_top);
                // Yil yuqori qatorda turibdi — bu yerda faqat oy nomi.
                let label = if fmt == Tick::Year {
                    d.year().to_string()
                } else {
                    i18n::month(d.month()).to_string()
                };
                painter.text(
                    pos2(x + 3.0, rows_top - 7.0),
                    Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    theme::muted(),
                );
                last_x = x;
            }
            d = next_month(d);
        }
    } else {
        let mut day = first_day - first_day.rem_euclid(step);
        let mut last_x = f32::MIN;
        while day <= first_day + span {
            let x = day_x(day as f32);
            if x >= time_rect.min.x && x < time_rect.max.x - 16.0 && x - last_x >= 20.0 {
                let date = origin + Duration::days(day);
                tick(painter, x, rows_top);
                let label = if fmt == Tick::Day {
                    date.day().to_string()
                } else {
                    format!("{:02}.{:02}", date.day(), date.month())
                };
                painter.text(
                    pos2(x + 3.0, rows_top - 7.0),
                    Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    theme::muted(),
                );
                last_x = x;
            }
            day += step;
        }
    }
}

/// Sarlavha ostidagi qisqa vertikal chiziqcha.
fn tick(painter: &egui::Painter, x: f32, rows_top: f32) {
    painter.line_segment(
        [pos2(x, rows_top - 14.0), pos2(x, rows_top)],
        Stroke::new(0.5_f32, theme::row_line()),
    );
}

fn next_month(d: NaiveDate) -> NaiveDate {
    let (y, m) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

/// Ортогональная стрелка связи с наконечником.
fn arrow(p: &egui::Painter, from: Pos2, to: Pos2, col: Color32) {
    let stroke = Stroke::new(1.2_f32, col);
    let gap = 9.0;
    let mid_x = if to.x >= from.x + gap * 2.0 {
        to.x - gap
    } else {
        from.x + gap
    };
    let pts = [
        from,
        pos2(mid_x, from.y),
        pos2(mid_x, to.y),
        pos2(to.x - 5.0, to.y),
    ];
    for w in pts.windows(2) {
        p.line_segment([w[0], w[1]], stroke);
    }
    p.add(egui::Shape::convex_polygon(
        vec![
            pos2(to.x, to.y),
            pos2(to.x - 6.0, to.y - 3.5),
            pos2(to.x - 6.0, to.y + 3.5),
        ],
        col,
        Stroke::NONE,
    ));
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Панель свойств выбранной работы.
fn inspector(ui: &mut egui::Ui, app: &mut App, id: i64) {
    let Some(mut task) = app.task(id).cloned() else {
        app.selected_task = None;
        return;
    };
    let calc = app.schedule.get(id);
    let mut dirty = false;
    let mut del_link: Option<i64> = None;
    let mut new_pred: Option<i64> = None;
    let mut reorder: Option<bool> = None;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_height(252.0);
            ui.horizontal_top(|ui| {
                // Колонка 1 — основные поля
                ui.vertical(|ui| {
                    ui.set_width(430.0);
                    ui.label(
                        RichText::new(t("insp_task"))
                            .size(13.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("insp_wbs")).color(theme::muted()));
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut task.wbs).desired_width(50.0))
                            .changed();
                        dirty |= ui
                            .add(
                                egui::TextEdit::singleline(&mut task.name)
                                    .desired_width(320.0)
                                    .hint_text(t("field_name")),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("task_section")).color(theme::muted()));
                        egui::ComboBox::from_id_salt("t_sec")
                            .selected_text(task.section.label())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for s in Section::ALL {
                                    dirty |= ui
                                        .selectable_value(&mut task.section, s, s.label())
                                        .changed();
                                }
                            });
                        ui.label(RichText::new(t("task_responsible")).color(theme::muted()));
                        dirty |= ui
                            .add(
                                egui::TextEdit::singleline(&mut task.responsible)
                                    .desired_width(180.0),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("task_duration")).color(theme::muted()));
                        dirty |= ui
                            .add(egui::DragValue::new(&mut task.duration).range(1..=3650))
                            .changed();
                        ui.label(RichText::new(t("task_done")).color(theme::muted()));
                        dirty |= ui
                            .add(
                                egui::DragValue::new(&mut task.progress)
                                    .range(0.0..=100.0)
                                    .speed(1.0),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("task_volume")).color(theme::muted()));
                        dirty |= ui
                            .add(egui::DragValue::new(&mut task.volume).speed(1.0))
                            .changed();
                        dirty |= ui
                            .add(
                                egui::TextEdit::singleline(&mut task.unit)
                                    .desired_width(70.0)
                                    .hint_text(t("task_unit")),
                            )
                            .changed();
                        if ui
                            .checkbox(&mut task.pinned, t("task_pin"))
                            .on_hover_text(t("task_pin_hint"))
                            .changed()
                        {
                            dirty = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("task_plan_start")).color(theme::muted()));
                        dirty |= super::passport::date_edit(ui, "t_start", &mut task.plan_start);
                        // Tartib: ro'yxatda yuqoriga/pastga.
                        if ui
                            .small_button("↑")
                            .on_hover_text(t("insp_move_up"))
                            .clicked()
                        {
                            reorder = Some(true);
                        }
                        if ui
                            .small_button("↓")
                            .on_hover_text(t("insp_move_down"))
                            .clicked()
                        {
                            reorder = Some(false);
                        }
                    });
                    // Fakt sanalari (TZ I.2 plan/fakt): belgilanmagan bo'lishi mumkin.
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("task_fact_start")).color(theme::muted()));
                        let mut has = task.fact_start.is_some();
                        if ui.checkbox(&mut has, "").changed() {
                            task.fact_start = has.then_some(app.today);
                            dirty = true;
                        }
                        if let Some(mut d) = task.fact_start {
                            if super::passport::date_edit(ui, "t_fs", &mut d) {
                                task.fact_start = Some(d);
                                dirty = true;
                            }
                        }
                        ui.label(RichText::new(t("task_fact_end")).color(theme::muted()));
                        let mut has_e = task.fact_end.is_some();
                        if ui.checkbox(&mut has_e, "").changed() {
                            task.fact_end = has_e.then_some(app.today);
                            dirty = true;
                        }
                        if let Some(mut d) = task.fact_end {
                            if super::passport::date_edit(ui, "t_fe", &mut d) {
                                task.fact_end = Some(d);
                                dirty = true;
                            }
                        }
                    });
                });

                ui.separator();

                // Колонка 2 — расчет CPM
                ui.vertical(|ui| {
                    ui.set_width(280.0);
                    ui.label(
                        RichText::new(t("insp_calc"))
                            .size(13.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.add_space(4.0);
                    if let Some(c) = calc {
                        let o = app.origin();
                        let row = |ui: &mut egui::Ui, k: &str, v: String, col: Color32| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [150.0, 18.0],
                                    egui::Label::new(
                                        RichText::new(k).color(theme::muted()).size(12.0),
                                    ),
                                );
                                ui.label(RichText::new(v).color(col).size(12.0));
                            });
                        };
                        let d = |off: i64| (o + Duration::days(off)).format("%d.%m.%Y").to_string();
                        row(ui, t("insp_es"), d(c.es), theme::text());
                        row(ui, t("insp_ef"), d(c.ef), theme::text());
                        row(ui, t("insp_ls"), d(c.ls), theme::text());
                        row(ui, t("insp_lf"), d(c.lf), theme::text());
                        row(
                            ui,
                            t("insp_slack"),
                            format!("{} {}", c.slack, t("days_short")),
                            if c.slack <= 0 {
                                theme::danger()
                            } else {
                                theme::ok()
                            },
                        );
                        row(
                            ui,
                            t("kpi_critical"),
                            if c.critical {
                                t("yes").to_string()
                            } else {
                                t("no").to_string()
                            },
                            if c.critical {
                                theme::danger()
                            } else {
                                theme::muted()
                            },
                        );
                        if app.progress.overdue.contains(&id) {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(t("insp_overdue"))
                                    .color(theme::danger())
                                    .size(12.0)
                                    .strong(),
                            );
                        }
                    }
                });

                ui.separator();

                // Колонка 3 — связи
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t("insp_links"))
                            .size(13.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            let preds: Vec<_> =
                                app.links.iter().filter(|l| l.succ == id).cloned().collect();
                            let succs: Vec<_> =
                                app.links.iter().filter(|l| l.pred == id).cloned().collect();

                            if !preds.is_empty() {
                                ui.label(
                                    RichText::new(t("insp_preds"))
                                        .color(theme::muted())
                                        .size(11.0),
                                );
                                for l in preds {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {} {:+}",
                                                truncate(&app.task_name(l.pred), 26),
                                                l.kind.short(),
                                                l.lag
                                            ))
                                            .size(12.0),
                                        );
                                        if ui.small_button("x").clicked() {
                                            del_link = Some(l.id);
                                        }
                                    });
                                }
                            }
                            // Tez bog'lash: oldingi ishni tanlash bilan FS bog'lanish.
                            ui.add_space(4.0);
                            egui::ComboBox::from_id_salt("add_pred")
                                .selected_text(t("insp_add_pred"))
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    for cand in &app.tasks {
                                        if cand.id == id
                                            || app
                                                .links
                                                .iter()
                                                .any(|l| l.pred == cand.id && l.succ == id)
                                        {
                                            continue;
                                        }
                                        if ui
                                            .selectable_label(
                                                false,
                                                format!(
                                                    "{} {}",
                                                    cand.wbs,
                                                    truncate(&cand.name, 24)
                                                ),
                                            )
                                            .clicked()
                                        {
                                            new_pred = Some(cand.id);
                                        }
                                    }
                                });
                            if !succs.is_empty() {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(t("insp_succs"))
                                        .color(theme::muted())
                                        .size(11.0),
                                );
                                for l in succs {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {} {:+}",
                                                truncate(&app.task_name(l.succ), 26),
                                                l.kind.short(),
                                                l.lag
                                            ))
                                            .size(12.0),
                                        );
                                        if ui.small_button("x").clicked() {
                                            del_link = Some(l.id);
                                        }
                                    });
                                }
                            }
                        });
                });
                // To'rtinchi ustun — ish bo'yicha muhokama va fayllar.
                // Izoh ishning yonida turishi kerak: alohida ekranga
                // chiqarilsa, u yozilmay qoladi.
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(340.0);
                    ui.label(
                        RichText::new(t("nt_panel"))
                            .size(13.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("gt_notes")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            super::notes::panel(ui, app, crate::domain::NoteTarget::Task, id);
                        });
                });
            });
        });

    if dirty {
        app.save_task(task);
    }
    if let Some(lid) = del_link {
        app.delete_link(lid);
    }
    if let Some(pred) = new_pred {
        app.add_link(pred, id, LinkType::Fs, 0);
    }
    if let Some(up) = reorder {
        app.reorder_task(id, up);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zum kichrayganda qadam kattalashadi va yozuv turi almashadi —
    /// sarlavha hech qachon bir-birining ustiga chiqmaydi.
    #[test]
    fn tick_step_adapts_to_zoom() {
        // Kun masshtabi: har kuni raqam sig'adi.
        assert_eq!(pick_tick(22.0), (1, Tick::Day));
        // O'rta zum: ikki kunda bir.
        assert_eq!(pick_tick(12.0), (2, Tick::Day));
        // Hafta masshtabi: haftada bir kun raqami.
        assert_eq!(pick_tick(6.0), (7, Tick::Day));
        // Oy masshtabi: oyda bir.
        assert_eq!(pick_tick(2.2).1, Tick::MonthYear);
        // Juda kichik zum: yillar.
        assert_eq!(pick_tick(0.1).1, Tick::Year);
    }

    /// Tanlangan qadam yozuv uchun kerakli kenglikni ta'minlaydi.
    #[test]
    fn tick_step_always_leaves_room_for_label() {
        let mut ppd = 0.3_f32;
        while ppd < 60.0 {
            let (step, fmt) = pick_tick(ppd);
            let width = step as f32 * ppd;
            let need = match fmt {
                Tick::Day => 22.0,
                Tick::DayMonth => 50.0,
                Tick::MonthYear => 52.0,
                Tick::Year => 44.0,
            };
            // Eng katta qadamda ham joy yetmasligi mumkin — u oxirgi variant.
            if step < 365 {
                assert!(
                    width >= need - 0.01,
                    "ppd={ppd}: qadam {step} kun = {width} px, kerak {need}"
                );
            }
            ppd *= 1.15;
        }
    }
}
