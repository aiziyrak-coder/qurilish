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
    Unit(i64),
    Deal(i64),
    Screen(Screen),
    /// Jurnal yozuvi — sanasi bilan ochiladi.
    Journal(chrono::NaiveDate),
    /// Yordamchining tayyor javobi (TZ IV.28, V.30).
    Copilot(crate::copilot::Intent),
}

/// Qidiruv uchun matnni bir xil ko'rinishga keltiradi.
fn norm(s: &str) -> String {
    s.to_lowercase()
}

/// Oyning nomi — o'zbekcha va ruscha (TZ V.30: «aprelda beton»).
const MONTHS: [(&[&str], u32); 12] = [
    (&["yanvar", "январ"], 1),
    (&["fevral", "феврал"], 2),
    (&["mart", "март"], 3),
    (&["aprel", "апрел"], 4),
    (&["may", "мая", "май"], 5),
    (&["iyun", "июн"], 6),
    (&["iyul", "июл"], 7),
    (&["avgust", "август"], 8),
    (&["sentabr", "сентябр"], 9),
    (&["oktabr", "октябр"], 10),
    (&["noyabr", "ноябр"], 11),
    (&["dekabr", "декабр"], 12),
];

/// Tushunilgan so'rov: qidiriladigan so'zlar va (bo'lsa) davr.
///
/// Sana so'zlari matn qidiruvidan **chiqarib tashlanadi**: «aprelda beton»
/// so'rovida «aprel» yozuv matnida uchramaydi, u davrni bildiradi. Shu
/// sababli so'rov ikki qismga bo'linadi — nima qidirilyapti va qachondan.
struct Query {
    words: Vec<String>,
    month: Option<u32>,
    year: Option<i32>,
}

impl Query {
    /// Yozuv matni barcha so'zlarni o'z ichiga oladimi.
    fn hits(&self, text: &str) -> bool {
        if self.words.is_empty() {
            // Faqat davr ko'rsatilgan bo'lsa, matn cheklamaydi.
            return self.month.is_some() || self.year.is_some();
        }
        let t = norm(text);
        self.words.iter().all(|w| t.contains(w.as_str()))
    }

    /// Sanasi bor yozuv so'ralgan davrga tushadimi.
    fn in_period(&self, date: chrono::NaiveDate) -> bool {
        use chrono::Datelike;
        self.month.is_none_or(|m| date.month() == m) && self.year.is_none_or(|y| date.year() == y)
    }

    /// Davr ko'rsatilganmi.
    fn has_period(&self) -> bool {
        self.month.is_some() || self.year.is_some()
    }
}

/// So'rovni so'zlarga va davrga ajratadi.
fn parse(query: &str) -> Query {
    let mut words = Vec::new();
    let mut month = None;
    let mut year = None;

    for raw in norm(query).split_whitespace() {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if w.is_empty() {
            continue;
        }
        // Oy nomi: «aprel», «aprelda», «апреле» — o'zak bo'yicha.
        if let Some((_, m)) = MONTHS
            .iter()
            .find(|(names, _)| names.iter().any(|n| w.starts_with(n)))
        {
            month = Some(*m);
            continue;
        }
        // To'rt raqamli son — yil.
        if w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(y) = w.parse::<i32>() {
                if (2000..2100).contains(&y) {
                    year = Some(y);
                    continue;
                }
            }
        }
        words.push(w.to_string());
    }
    Query { words, month, year }
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
    // Ctrl+E — joriy ekran jadvalini Excel ga chiqarish.
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::E)) {
        super::export_current(app);
    }
    // Matn maydonida yozayotganda `/` oynani ochmasligi kerak.
    let typing = ctx.memory(|m| m.focused().is_some());
    if open_key && !(typing && !ctx.input(|i| i.modifiers.ctrl)) {
        app.search_open = true;
        app.search_query.clear();
        app.search_focus = true;
        // Oldingi qidiruvdan qolgan ajratish o'chadi.
        app.journal_focus = None;
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
    let p = parse(query);
    if p.words.is_empty() && !p.has_period() {
        return Vec::new();
    }
    let mut out: Vec<Hit> = Vec::new();
    const PER_KIND: usize = 6;

    // ---- Yordamchi javobi: savolga o'xshash so'rov birinchi qatorda ----
    // Bu qidiruvni «savol berish»ga ulaydi (TZ IV.28, V.30): son baribir
    // baza hisobidan chiqadi, qidiruv faqat yo'l ko'rsatadi.
    if query.split_whitespace().count() >= 2 {
        if let Some(intent) = crate::copilot::detect(query) {
            out.push(Hit {
                kind: t("search_kind_ai"),
                title: intent.question().to_string(),
                subtitle: intent.source().to_string(),
                color: theme::accent(),
                action: Action::Copilot(intent),
            });
        }
    }

    // ---- Ishlar ----
    for task in app
        .tasks
        .iter()
        .filter(|x| p.hits(&format!("{} {} {}", x.name, x.wbs, x.responsible)))
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
        .filter(|i| p.hits(&format!("{} {} {}", i.title, i.code, i.element)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_issue"),
            title: issue.title.clone(),
            subtitle: format!(
                "{} · {} · {}",
                issue.code,
                issue.element,
                issue.status.label()
            ),
            color: issue.severity.color(),
            action: Action::Issue(issue.id, issue.module),
        });
    }

    // ---- Loyiha elementlari ----
    for el in app
        .elements
        .iter()
        .filter(|e| p.hits(&format!("{} {} {}", e.mark, e.room, e.sheet)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_element"),
            title: if el.mark.trim().is_empty() {
                el.room.clone()
            } else {
                el.mark.clone()
            },
            subtitle: format!(
                "{} · {} · {}",
                el.kind.label(),
                el.section.label(),
                el.sheet
            ),
            color: theme::section_color(el.section.color()),
            action: Action::Element(el.id),
        });
    }

    // ---- Smeta pozitsiyalari ----
    for item in app
        .estimate_items
        .iter()
        .filter(|i| p.hits(&format!("{} {}", i.name, i.code)))
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
        .filter(|d| p.hits(&format!("{} {}", d.name, d.number)))
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

    // ---- Kvartiralar: raqami, planirovkasi yoki blok nomi bo'yicha ----
    for u in app
        .units
        .iter()
        .filter(|u| {
            let block = app
                .blocks
                .iter()
                .find(|b| b.id == u.block_id)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            p.hits(&format!(
                "{} {} {} {}",
                u.number,
                u.layout,
                block,
                u.status.label()
            ))
        })
        .take(PER_KIND)
    {
        let block = app
            .blocks
            .iter()
            .find(|b| b.id == u.block_id)
            .map(|b| b.name.clone())
            .unwrap_or_default();
        out.push(Hit {
            kind: t("screen_sales"),
            title: format!("{block} · {}", u.number),
            subtitle: format!(
                "{} {} · {} m² · {} · {}",
                u.floor,
                t("floor_short"),
                super::materials::trim_num(u.area),
                u.status.label(),
                super::money(u.price())
            ),
            color: theme::accent(),
            action: Action::Unit(u.id),
        });
    }

    // ---- Shartnomalar: raqami yoki mijoz bo'yicha ----
    for d in app
        .deals
        .iter()
        .filter(|d| p.hits(&format!("{} {}", d.number, d.client)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_deals"),
            title: format!("{} · {}", d.number, d.client),
            subtitle: format!(
                "{} · {} · {}",
                d.status.label(),
                d.pay_kind.label(),
                super::money(d.total())
            ),
            color: theme::ok(),
            action: Action::Deal(d.id),
        });
    }

    // ---- Ijro hujjatlari ----
    for d in app
        .exec_docs
        .iter()
        .filter(|d| {
            p.in_period(d.date) && p.hits(&format!("{} {} {}", d.number, d.name, d.responsible))
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_doc"),
            title: format!("{} {}", d.number, d.name),
            subtitle: format!(
                "{} · {} · {}",
                d.kind.label(),
                d.status.label(),
                d.date.format("%d.%m.%Y")
            ),
            color: theme::ok(),
            action: Action::Screen(Screen::ExecDocs),
        });
    }

    // ---- Kunlik jurnal: matn va davr bo'yicha ----
    for e in app
        .journal
        .iter()
        .filter(|e| {
            p.in_period(e.date)
                && p.hits(&format!(
                    "{} {} {}",
                    e.text,
                    e.remarks,
                    e.task_id
                        .and_then(|id| app.task(id))
                        .map(|x| x.name.clone())
                        .unwrap_or_default()
                ))
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("search_kind_journal"),
            title: if e.text.trim().is_empty() {
                e.remarks.clone()
            } else {
                e.text.clone()
            },
            subtitle: format!(
                "{} · {} · {} {}",
                e.date.format("%d.%m.%Y"),
                e.author,
                super::materials::trim_num(e.volume),
                e.unit
            ),
            color: theme::warn(),
            action: Action::Journal(e.date),
        });
    }

    // ---- Arizalar ----
    for r in app
        .requests
        .iter()
        .filter(|r| {
            p.in_period(r.date) && p.hits(&format!("{} {} {}", r.number, r.title, r.requester))
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_requests"),
            title: format!("{} {}", r.number, r.title),
            subtitle: format!(
                "{} · {} · {}",
                r.kind.label(),
                r.status.label(),
                r.date.format("%d.%m.%Y")
            ),
            color: theme::accent(),
            action: Action::Screen(Screen::Requests),
        });
    }

    // ---- Xaridlar ----
    for x in app
        .purchases
        .iter()
        .filter(|x| {
            p.in_period(x.date) && p.hits(&format!("{} {} {}", x.number, x.title, x.supplier))
        })
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_purchases"),
            title: format!("{} {}", x.number, x.title),
            subtitle: format!(
                "{} · {} · {}",
                x.supplier,
                x.status.label(),
                super::money(x.amount())
            ),
            color: theme::ok(),
            action: Action::Screen(Screen::Purchases),
        });
    }

    // ---- Materiallar ----
    for m in app
        .materials
        .iter()
        .filter(|m| !p.has_period() && p.hits(&format!("{} {}", m.code, m.name)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_materials"),
            title: format!("{} {}", m.code, m.name),
            subtitle: format!("{} · {}", m.section.label(), m.unit),
            color: theme::section_color(m.section.color()),
            action: Action::Screen(Screen::Materials),
        });
    }

    // ---- Ishchilar ----
    for w in app
        .workers
        .iter()
        .filter(|w| !p.has_period() && p.hits(&format!("{} {} {}", w.name, w.position, w.org)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_timesheet"),
            title: w.name.clone(),
            subtitle: format!("{} · {}", w.position, w.org),
            color: theme::muted(),
            action: Action::Screen(Screen::Timesheet),
        });
    }

    // ---- Texnika ----
    for m in app
        .machines
        .iter()
        .filter(|m| !p.has_period() && p.hits(&format!("{} {} {}", m.name, m.reg_no, m.operator)))
        .take(PER_KIND)
    {
        out.push(Hit {
            kind: t("screen_machines"),
            title: m.name.clone(),
            subtitle: format!("{} · {} · {}", m.kind.label(), m.reg_no, m.status.label()),
            color: theme::warn(),
            action: Action::Screen(Screen::Machines),
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
            app.check_tab = crate::app::CheckTab::Project;
            app.selected_element = Some(id);
        }
        Action::Issue(id, module) => {
            app.screen = match module {
                IssueModule::Estimate => Screen::Estimate,
                IssueModule::Ppr => Screen::Ppr,
                _ => Screen::AiCheck,
            };
            if app.screen == Screen::AiCheck {
                app.check_tab = crate::app::CheckTab::Check;
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
        Action::Unit(id) => {
            app.screen = Screen::Sales;
            app.selected_unit = Some(id);
            // Shaxmatka tanlangan kvartira turgan blokka o'tsin.
            app.sales_block = app.units.iter().find(|u| u.id == id).map(|u| u.block_id);
        }
        Action::Deal(id) => {
            app.screen = Screen::Deals;
            app.selected_deal = Some(id);
        }
        Action::Screen(s) => app.screen = s,
        Action::Journal(date) => {
            app.screen = Screen::Journal;
            app.journal_focus = Some(date);
        }
        Action::Copilot(intent) => {
            app.screen = Screen::Copilot;
            app.copilot_intent = Some(intent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TZ V.30: so'rovdagi oy nomi davrga aylanadi va matndan chiqadi.
    #[test]
    fn month_word_becomes_a_period() {
        let q = parse("aprelda beton quyish");
        assert_eq!(q.month, Some(4));
        assert_eq!(q.words, vec!["beton", "quyish"]);
        assert!(q.has_period());

        let ru = parse("бетон в апреле 2026");
        assert_eq!(ru.month, Some(4));
        assert_eq!(ru.year, Some(2026));
        assert_eq!(ru.words, vec!["бетон", "в"]);
    }

    /// Barcha so'z topilishi kerak: bitta so'z mos kelishi yetarli emas.
    #[test]
    fn all_words_must_be_found() {
        let q = parse("beton quyish");
        assert!(q.hits("Beton quyish ishlari"));
        assert!(!q.hits("Beton tayyorlash"));
        // Davr ko'rsatilgan, so'z yo'q — matn cheklamaydi.
        let only = parse("aprel");
        assert!(only.hits("istalgan matn"));
    }

    /// Davr so'ralganda boshqa oydagi yozuv chiqmaydi.
    #[test]
    fn period_filters_by_date() {
        use chrono::NaiveDate;
        let q = parse("aprel 2026");
        assert!(q.in_period(NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()));
        assert!(!q.in_period(NaiveDate::from_ymd_opt(2026, 5, 10).unwrap()));
        assert!(!q.in_period(NaiveDate::from_ymd_opt(2025, 4, 10).unwrap()));
        // Davrsiz so'rov hech qanday sanani rad etmaydi.
        let free = parse("beton");
        assert!(free.in_period(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()));
    }

    /// Bo'sh so'rov natija bermaydi — o'ylab topilgan javob yo'q.
    #[test]
    fn empty_query_finds_nothing() {
        let q = parse("   ");
        assert!(q.words.is_empty());
        assert!(!q.has_period());
    }
}
