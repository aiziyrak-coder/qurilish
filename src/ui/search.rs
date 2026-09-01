//! Umumiy qidiruv: `Ctrl+K` bilan ochiladi va barcha modullar bo'ylab qidiradi.
//!
//! 22 ta ekranda kerakli yozuvni qo'lda qidirish noqulay — bu oyna ish, element,
//! smeta pozitsiyasi, nomuvofiqlik, hujjat va kartani bir joyda topib beradi va
//! tanlanганда tegишli ekranni ochib, o'sha yozuvni ajratadi.

use super::*;
use crate::domain::IssueModule;
use egui::{vec2, Sense};

/// Qidiruv natijasi: qayerga olib borishi va nima ko'rsatishi.
struct Hit {
    /// Qaysi modulda topildi.
    kind: &'static str,
    title: String,
    subtitle: String,
    color: Color32,
    action: Action,
}

#[derive(Clone, Copy)]
enum Action {
    Task(i64),
    Element(i64),
    Issue(i64, IssueModule),
    EstimateItem(i64),
    Ppr(i64),
    Screen(Screen),
}

/// Qidiruv uchun matnni bir xil ko'rinishga keltiradi.
fn norm(s: &str) -> String {
    s.to_lowercase()
}

/// Oynani chizadi. `Ctrl+K` yoki `/` bilan ochiladi, `Esc` bilan yopiladi.
pub fn draw(ctx: &Context, app: &mut App) {
    // ---- Ochish va yopish ----
    let (open_key, esc) = ctx.input(|i| {
        (
            (i.modifiers.ctrl && i.key_pressed(egui::Key::K))
                || (i.key_pressed(egui::Key::Slash) && !i.modifiers.ctrl),
            i.key_pressed(egui::Key::Escape),
        )
    });
    // Matn maydonida yozayotganda `/` oynani ochmasligi kerak.
    let typing = ctx.memory(|m| m.focused().is_some());
    if open_key && !(typing && !ctx.input(|i| i.modifiers.ctrl)) {
        app.search_open = true;
        app.search_query.clear();
        app.search_focus = true;
    }
    if !app.search_open {
        return;
    }
    if esc {
        app.search_open = false;
        return;
    }

    let mut close = false;
    let mut go: Option<Action> = None;

    egui::Window::new(t("search_title"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
        .default_width(620.0)
        .show(ctx, |ui| {
            ui.set_width(600.0);

            let edit = ui.add(
                egui::TextEdit::singleline(&mut app.search_query)
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(16.0))
                    .hint_text(t("search_hint")),
            );
            // Oyna ochilganda kursor darhol maydonda bo'lsin.
            if app.search_focus {
                edit.request_focus();
                app.search_focus = false;
            }
            ui.label(
                RichText::new(t("search_keys"))
                    .size(10.5)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);

            let hits = collect(app, &app.search_query);
            if app.search_query.trim().is_empty() {
                ui.label(
                    RichText::new(t("search_empty_query"))
                        .size(12.5)
                        .color(theme::muted()),
                );
                return;
            }
            if hits.is_empty() {
                ui.label(
                    RichText::new(t("search_nothing"))
                        .size(12.5)
                        .color(theme::muted()),
                );
                return;
            }

            egui::ScrollArea::vertical()
                .max_height(420.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for hit in &hits {
                        if row(ui, hit) {
                            go = Some(hit.action);
                        }
                    }
                });
        });

    if let Some(action) = go {
        apply(app, action);
        close = true;
    }
    if close {
        app.search_open = false;
    }
}

/// Bitta natija qatori.
fn row(ui: &mut egui::Ui, hit: &Hit) -> bool {
    let h = 42.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 6.0, theme::line().gamma_multiply(0.55));
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        // Modul yorlig'i.
        let badge = Rect::from_min_size(
            pos2(rect.min.x + 4.0, rect.center().y - 9.0),
            vec2(74.0, 18.0),
        );
        p.rect_filled(badge, 4.0, hit.color.gamma_multiply(0.16));
        p.text(
            badge.center(),
            Align2::CENTER_CENTER,
            hit.kind,
            egui::FontId::proportional(10.5),
            hit.color,
        );
        p.text(
            pos2(badge.max.x + 10.0, rect.center().y - 7.0),
            Align2::LEFT_CENTER,
            super::issues::truncate(&hit.title, 52),
            egui::FontId::proportional(13.0),
            theme::text(),
        );
        p.text(
            pos2(badge.max.x + 10.0, rect.center().y + 9.0),
            Align2::LEFT_CENTER,
            super::issues::truncate(&hit.subtitle, 60),
            egui::FontId::proportional(11.0),
            theme::muted(),
        );
    }
    resp.clicked()
}

/// Barcha modullar bo'ylab qidiradi. Natija soni cheklanadi — ro'yxat
/// o'qilishi kerak, hamma narsani ko'rsatish emas.
fn collect(app: &App, query: &str) -> Vec<Hit> {
    let q = norm(query.trim());
    if q.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Hit> = Vec::new();
    const PER_KIND: usize = 6;

    // ---- Ishlar ----
    for task in app
        .tasks
        .iter()
        .filter(|x| norm(&x.name).contains(&q) || norm(&x.wbs) == q || norm(&x.responsible).contains(&q))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_gantt_short"),
            title: task.name.clone(),
            subtitle: format!(
                "{} {} · {} · {:.0} %",
                t("col_num"),
                task.wbs,
                task.section.label(),
                task.progress
            ),
            color: theme::accent(),
            action: Action::Task(task.id),
        });
    }

    // ---- Nomuvofiqliklar ----
    for issue in app
        .issues
        .iter()
        .filter(|i| {
            norm(&i.title).contains(&q)
                || norm(&i.code).contains(&q)
                || norm(&i.element).contains(&q)
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_issue"),
            title: issue.title.clone(),
            subtitle: format!("{} · {} · {}", issue.code, issue.element, issue.status.label()),
            color: issue.severity.color(),
            action: Action::Issue(issue.id, issue.module),
        });
    }

    // ---- Loyiha elementlari ----
    for el in app
        .elements
        .iter()
        .filter(|e| {
            norm(&e.mark).contains(&q) || norm(&e.room).contains(&q) || norm(&e.sheet).contains(&q)
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_element"),
            title: if el.mark.trim().is_empty() {
                el.room.clone()
            } else {
                el.mark.clone()
            },
            subtitle: format!("{} · {} · {}", el.kind.label(), el.section.label(), el.sheet),
            color: theme::section_color(el.section.color()),
            action: Action::Element(el.id),
        });
    }

    // ---- Smeta pozitsiyalari ----
    for item in app
        .estimate_items
        .iter()
        .filter(|i| norm(&i.name).contains(&q) || norm(&i.code).contains(&q))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_estimate"),
            title: item.name.clone(),
            subtitle: format!(
                "{} {} · {} {} · {}",
                t("chk_pos"),
                item.pos,
                super::money(item.qty),
                item.unit,
                super::money(item.cost)
            ),
            color: theme::warn(),
            action: Action::EstimateItem(item.id),
        });
    }

    // ---- PPR kartalari ----
    for card in app
        .ppr_docs
        .iter()
        .filter(|d| norm(&d.name).contains(&q) || norm(&d.number).contains(&q))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_ppr"),
            title: format!("{} {}", card.number, card.name),
            subtitle: card.kind.label().to_string(),
            color: theme::ok(),
            action: Action::Ppr(card.id),
        });
    }

    // ---- Modul nomlari: «qayerda edi?» degan savolga javob ----
    for (_, screens) in crate::app::NAV_GROUPS {
        for &screen in *screens {
            if norm(screen.label()).contains(&q) || norm(screen.numeral()) == q {
                out.push(Hit {
                    kind: t("search_kind_screen"),
                    title: screen.label().to_string(),
                    subtitle: if screen.numeral().is_empty() {
                        String::new()
                    } else {
                        format!("{} {}", t("search_module"), screen.numeral())
                    },
                    color: theme::muted(),
                    action: Action::Screen(screen),
                });
            }
        }
    }

    out
}

/// Tanlangan natijaga o'tadi: ekranni ochadi va yozuvni ajratadi.
fn apply(app: &mut App, action: Action) {
    match action {
        Action::Task(id) => {
            app.screen = Screen::Gantt;
            app.selected_task = Some(id);
            if let Some(c) = app.schedule.get(id) {
                app.timeline_offset = (c.es as f32 - 20.0).max(0.0);
            }
            app.scroll_to_task(id);
        }
        Action::Element(id) => {
            app.screen = Screen::AiCheck;
            app.check_tab = crate::app::CheckTab::Elements;
            app.selected_element = Some(id);
        }
        Action::Issue(id, module) => {
            app.screen = match module {
                IssueModule::Estimate => Screen::Estimate,
                IssueModule::Ppr => Screen::Ppr,
                _ => Screen::AiCheck,
            };
            if app.screen == Screen::AiCheck {
                app.check_tab = crate::app::CheckTab::Issues;
            }
            app.selected_issue = Some(id);
            // Natija filtrlar ostida yashirinib qolmasin.
            app.issue_sev = None;
            app.issue_status = None;
            app.issue_search.clear();
        }
        Action::EstimateItem(id) => {
            app.screen = Screen::Estimate;
            // Pozitsiya boshqa smetada bo'lsa, o'shanga o'tamiz.
            if !app.estimate_items.iter().any(|i| i.id == id) {
                for est in app.estimates.clone() {
                    if app.db.estimate_items(est.id).iter().any(|i| i.id == id) {
                        app.current_estimate = Some(est.id);
                        app.reload_estimate_items();
                        break;
                    }
                }
            }
        }
        Action::Ppr(id) => {
            app.screen = Screen::Ppr;
            app.selected_ppr = Some(id);
        }
        Action::Screen(s) => app.screen = s,
    }
}
