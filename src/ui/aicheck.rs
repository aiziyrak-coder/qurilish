//! «AI loyiha tekshiruvi» ekrani (TZ II).
//!
//! Loyiha bilimlar grafi (elementlar va ular orasidagi bog'lanishlar) ustida
//! qoidalar ishlaydi va bo'limlararo nomuvofiqliklarni topadi. Me'yoriy asos
//! qoidaga emas, alohida reyestrga bog'langan — TZ II.17 talabi.

use super::issues;
use super::*;
use crate::app::CheckTab;
use crate::checks::{self, Norm, RULES};
use crate::domain::{Element, ElementKind, ElementLink, IssueModule, Relation};
use crate::model::Section;
use egui::vec2;

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        empty(ui, t("no_object_selected"));
        return;
    }

    app.auto_check(IssueModule::Project);

    /// Har kuni ochiladigan bo'limlar.
    const DAILY: [CheckTab; 3] = [CheckTab::Issues, CheckTab::Clash, CheckTab::Action];

    // Kunlik: topilmalar, kolliziyalar va nima qilish kerakligi.
    // Qolganlari — ma'lumot kiritish va sozlash uchun, ajratgichdan keyin.
    ui.horizontal_wrapped(|ui| {
        let mut pick = |ui: &mut egui::Ui, tab: CheckTab, dim: bool| {
            let text = if dim && app.check_tab != tab {
                RichText::new(tab.label()).color(theme::muted())
            } else {
                RichText::new(tab.label())
            };
            if ui.selectable_label(app.check_tab == tab, text).clicked() {
                app.check_tab = tab;
            }
        };
        for tab in DAILY {
            pick(ui, tab, false);
        }
        ui.separator();
        // Qolganlari ro'yxatdan **hisoblab** olinadi: yangi bo'lim
        // qo'shilsa, u qo'lda yozilmagani uchun ko'rinmay qolmaydi.
        for tab in CheckTab::ALL {
            if !DAILY.contains(&tab) {
                pick(ui, tab, true);
            }
        }
    });
    ui.add_space(8.0);

    match app.check_tab {
        CheckTab::Issues => issues_tab(ui, app),
        CheckTab::Clash => clash_tab(ui, app),
        CheckTab::Action => action_tab(ui, app),
        CheckTab::Graph => graph_tab(ui, app),
        CheckTab::Elements => elements_tab(ui, app),
        CheckTab::Relations => relations_tab(ui, app),
        CheckTab::Norms => norms_tab(ui, app),
    }
}

// ---------------------------------------------------------------- ACTION

/// TZ II.19 dagi to'rtinchi ekran: nima tuzatilishi kerak, kim mas'ul va
/// qachongacha. Ochiq nomuvofiqliklar mas'ullar bo'yicha guruhlanadi.
fn action_tab(ui: &mut egui::Ui, app: &mut App) {
    use crate::domain::{IssueStatus, Severity};

    let mut edited: Option<crate::domain::Issue> = None;
    let today = app.today;

    // Ochiq va ishlanayotgan yozuvlar — bajarilishi kerak bo'lganlar.
    let mut open: Vec<&crate::domain::Issue> = app
        .issues
        .iter()
        .filter(|i| matches!(i.status, IssueStatus::Open | IssueStatus::InWork))
        .collect();
    open.sort_by(|a, b| {
        a.severity
            .rank()
            .cmp(&b.severity.rank())
            .then_with(|| a.code.cmp(&b.code))
    });

    // Ko'rsatkichlar: muddati belgilanganlar va o'tib ketganlar.
    let with_deadline = open.iter().filter(|i| i.deadline.is_some()).count();
    let overdue = open
        .iter()
        .filter(|i| i.deadline.map(|d| d < today).unwrap_or(false))
        .count();
    let crit = open
        .iter()
        .filter(|i| i.severity == Severity::Critical)
        .count();

    stat_row(
        ui,
        vec![
            stat(
                t("act_open"),
                open.len().to_string(),
                t("act_open_hint"),
                if open.is_empty() {
                    theme::ok()
                } else {
                    theme::accent()
                },
            ),
            stat(
                t("act_critical"),
                crit.to_string(),
                t("act_critical_hint"),
                if crit == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("act_with_deadline"),
                format!("{with_deadline} / {}", open.len()),
                t("act_with_deadline_hint"),
                if with_deadline == open.len() {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("act_overdue"),
                overdue.to_string(),
                t("act_overdue_hint"),
                if overdue == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(10.0);

    if open.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("act_empty")).color(theme::ok()).size(15.0));
        });
        return;
    }

    // Mas'ullar bo'yicha guruhlaymiz — kim nima qilishi kerakligi shu tartibda o'qiladi.
    let mut groups: Vec<(String, Vec<&crate::domain::Issue>)> = Vec::new();
    for issue in &open {
        let who = if issue.responsible.trim().is_empty() {
            t("act_no_owner").to_string()
        } else {
            issue.responsible.clone()
        };
        match groups.iter_mut().find(|(k, _)| *k == who) {
            Some((_, v)) => v.push(issue),
            None => groups.push((who, vec![issue])),
        }
    }
    // Kritik xatosi ko'p bo'lgan mas'ul yuqorida.
    groups.sort_by_key(|(_, v)| {
        -(v.iter()
            .filter(|i| i.severity == Severity::Critical)
            .count() as i64)
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let w = (ui.available_width() - 26.0).min(1020.0);
            for (who, items) in &groups {
                let crit_n = items
                    .iter()
                    .filter(|i| i.severity == Severity::Critical)
                    .count();
                let title = format!("{who} — {} {}", items.len(), t("act_items"));
                card_frame(ui, &title, w, |ui| {
                    if crit_n > 0 {
                        ui.label(
                            RichText::new(format!("{crit_n} {}", t("act_critical_of_them")))
                                .size(11.5)
                                .color(theme::danger()),
                        );
                        ui.add_space(4.0);
                    }
                    for issue in items {
                        let mut it = (*issue).clone();
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            issues::severity_chip(ui, it.severity, 92.0);
                            ui.add_sized(
                                [70.0, 18.0],
                                egui::Label::new(
                                    RichText::new(&it.code)
                                        .monospace()
                                        .size(11.5)
                                        .color(theme::muted()),
                                ),
                            );
                            ui.add_sized(
                                [270.0, 18.0],
                                egui::Label::new(
                                    RichText::new(issues::truncate(&it.title, 36)).size(12.5),
                                ),
                            );
                            ui.add_sized(
                                [140.0, 18.0],
                                egui::Label::new(
                                    RichText::new(issues::truncate(&it.element, 19))
                                        .size(12.0)
                                        .color(theme::muted()),
                                ),
                            );

                            // Muddat: belgilanmagan bo'lsa qo'shish mumkin.
                            ui.label(
                                RichText::new(t("act_deadline"))
                                    .size(11.5)
                                    .color(theme::muted()),
                            );
                            let mut has = it.deadline.is_some();
                            if ui.checkbox(&mut has, "").changed() {
                                it.deadline = has.then(|| today + chrono::Duration::days(14));
                                changed = true;
                            }
                            if let Some(mut d) = it.deadline {
                                if super::passport::date_edit(ui, &format!("dl{}", it.id), &mut d) {
                                    it.deadline = Some(d);
                                    changed = true;
                                }
                                let late = (today - d).num_days();
                                if late > 0 {
                                    ui.label(
                                        RichText::new(format!(
                                            "{late} {} {}",
                                            t("days_short"),
                                            t("act_late")
                                        ))
                                        .size(11.5)
                                        .color(theme::danger())
                                        .strong(),
                                    );
                                }
                            }

                            // Holatni shu yerdan o'zgartirish — muddat yonida.
                            ui.add_space(10.0);
                            for st in [IssueStatus::InWork, IssueStatus::Fixed] {
                                let sel = it.status == st;
                                if ui.selectable_label(sel, st.label()).clicked() && !sel {
                                    it.status = st;
                                    changed = true;
                                }
                            }
                        });
                        if changed {
                            edited = Some(it);
                        }
                    }
                });
                ui.add_space(8.0);
            }
            ui.add_space(20.0);
        });

    if let Some(it) = edited {
        app.save_issue(it);
    }
}

// ---------------------------------------------------------------- Bilimlar grafi

/// TZ II.18: loyiha bilimlar grafi. Elementlar bo'limlar bo'yicha ustunlarga
/// joylashtiriladi, bog'lanishlar chiziq bilan ko'rsatiladi — bo'limlararo
/// aloqalar shu ko'rinishda darrov ko'zga tashlanadi.
fn graph_tab(ui: &mut egui::Ui, app: &mut App) {
    use crate::domain::Element;

    if app.elements.is_empty() {
        empty(ui, t("no_elements"));
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(t("graph_hint"))
                .size(11.5)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    // Ustunlar: faqat elementi bor bo'limlar.
    let sections: Vec<Section> = Section::ALL
        .into_iter()
        .filter(|s| app.elements.iter().any(|e| e.section == *s))
        .collect();
    if sections.is_empty() {
        return;
    }

    const NODE_W: f32 = 108.0;
    const NODE_H: f32 = 26.0;
    const GAP_Y: f32 = 10.0;

    let rows_max = sections
        .iter()
        .map(|s| app.elements.iter().filter(|e| e.section == *s).count())
        .max()
        .unwrap_or(1);
    let need_h = 46.0 + rows_max as f32 * (NODE_H + GAP_Y) + 20.0;
    let col_w = (ui.available_width() - 20.0).max(80.0) / sections.len() as f32;
    let need_w = (col_w * sections.len() as f32).max(NODE_W * sections.len() as f32 + 40.0);

    let mut clicked: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                vec2(need_w, need_h.max(ui.available_height() - 4.0)),
                egui::Sense::hover(),
            );
            let p = ui.painter_at(rect);
            let col_w = rect.width() / sections.len() as f32;

            // Tugun markazini hisoblash.
            let mut pos: std::collections::HashMap<i64, egui::Pos2> =
                std::collections::HashMap::new();
            for (ci, sec) in sections.iter().enumerate() {
                let cx = rect.min.x + col_w * (ci as f32 + 0.5);
                // Ustun sarlavhasi.
                p.text(
                    pos2(cx, rect.min.y + 12.0),
                    Align2::CENTER_CENTER,
                    sec.label(),
                    egui::FontId::proportional(13.0),
                    theme::section_color(sec.color()),
                );
                let items: Vec<&Element> =
                    app.elements.iter().filter(|e| e.section == *sec).collect();
                for (ri, el) in items.iter().enumerate() {
                    let cy = rect.min.y + 46.0 + ri as f32 * (NODE_H + GAP_Y) + NODE_H / 2.0;
                    pos.insert(el.id, pos2(cx, cy));
                }
            }

            let sel = app.selected_element;

            // Avval bog'lanishlar — tugunlar ustiga chizilmasin.
            for link in &app.element_links {
                let (Some(a), Some(b)) = (pos.get(&link.from_el), pos.get(&link.to_el)) else {
                    continue;
                };
                let touches = sel == Some(link.from_el) || sel == Some(link.to_el);
                let color = if sel.is_some() && !touches {
                    theme::line().gamma_multiply(0.5)
                } else if touches {
                    theme::accent()
                } else {
                    theme::arrow().gamma_multiply(0.6)
                };
                let width = if touches { 1.8_f32 } else { 0.9_f32 };
                // To'g'ri chiziq: siniq chiziqlar ustunlar orasida bir-birining
                // ustiga tushib, grafni o'qib bo'lmas holga keltirardi.
                let side = if b.x > a.x { 1.0 } else { -1.0 };
                let from = pos2(a.x + NODE_W / 2.0 * side, a.y);
                let to = pos2(b.x - NODE_W / 2.0 * side, b.y);
                p.line_segment([from, to], Stroke::new(width, color));
                if touches {
                    // Yo'nalish: tanlangan tugunning aloqasida strelka uchi.
                    let dir = (to - from).normalized();
                    let n = egui::vec2(-dir.y, dir.x);
                    p.add(egui::Shape::convex_polygon(
                        vec![to, to - dir * 8.0 + n * 3.5, to - dir * 8.0 - n * 3.5],
                        color,
                        Stroke::NONE,
                    ));
                }
            }

            // Keyin tugunlar.
            for el in &app.elements {
                let Some(c) = pos.get(&el.id) else { continue };
                let node = Rect::from_center_size(*c, vec2(NODE_W, NODE_H));
                let active = sel == Some(el.id);
                let base = theme::section_color(el.section.color());
                p.rect_filled(
                    node,
                    5.0,
                    if active {
                        base.gamma_multiply(0.35)
                    } else {
                        base.gamma_multiply(0.16)
                    },
                );
                p.rect_stroke(
                    node,
                    5.0,
                    Stroke::new(if active { 2.0_f32 } else { 1.0_f32 }, base),
                    egui::StrokeKind::Inside,
                );
                let label = if el.mark.trim().is_empty() {
                    issues::truncate(&el.room, 12)
                } else {
                    issues::truncate(&el.mark, 12)
                };
                p.text(
                    pos2(node.min.x + 6.0, node.center().y - 4.0),
                    Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(11.5),
                    theme::text(),
                );
                p.text(
                    pos2(node.min.x + 6.0, node.center().y + 7.0),
                    Align2::LEFT_CENTER,
                    issues::truncate(el.kind.label(), 14),
                    egui::FontId::proportional(9.5),
                    theme::muted(),
                );

                let resp = ui.interact(node, egui::Id::new(("gnode", el.id)), egui::Sense::click());
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp
                    .on_hover_text(format!("{} · {}", el.kind.label(), el.sheet))
                    .clicked()
                {
                    clicked = Some(el.id);
                }
            }
        });

    if let Some(id) = clicked {
        // Ikkinchi marta bosilsa — tanlov bekor qilinadi.
        app.selected_element = if app.selected_element == Some(id) {
            None
        } else {
            Some(id)
        };
    }
}

fn empty(ui: &mut egui::Ui, msg: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(120.0);
        ui.label(RichText::new(msg).color(theme::muted()).size(18.0));
    });
}

// ---------------------------------------------------------------- Nomuvofiqliklar

// ================================================================ CLASH

/// Bo'limlar orasidagi to'qnashuvlar (TZ II.19).
///
/// CLASH — bu **bo'limlar orasidagi** nomuvofiqlik: tarmoq konstruksiyani
/// kesib o'tadi, xonaga xizmat yo'q, qurilma ta'minotsiz qolgan.
/// Ular alohida turadi, chunki yechim ham alohida: ikki bo'lim
/// loyihachisi birga o'tirib hal qiladi.
fn clash_tab(ui: &mut egui::Ui, app: &mut App) {
    use crate::domain::{IssueModule, IssueStatus, Severity};

    // Bo'limlararo qoidalar ro'yxati: sarlavhasi bo'yicha aniqlanadi.
    let clash_titles = [
        t("chk_cross_title"),
        t("chk_room_service_title"),
        t("chk_power_title"),
        t("chk_kmkj_title"),
        t("chk_arkj_title"),
        t("chk_build_size_title"),
        t("chk_pb_water_title"),
        t("chk_ss_cable_title"),
    ];
    let rows: Vec<&crate::domain::Issue> = app
        .issues
        .iter()
        .filter(|i| i.module == IssueModule::Project)
        .filter(|i| clash_titles.contains(&i.title.as_str()))
        .collect();
    let open = rows
        .iter()
        .filter(|i| matches!(i.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    let critical = rows
        .iter()
        .filter(|i| i.severity == Severity::Critical && i.status != IssueStatus::Fixed)
        .count();

    ui.label(
        RichText::new(t("clash_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("clash_total"),
                rows.len().to_string(),
                t("clash_total_hint"),
                theme::text(),
            ),
            stat(
                t("clash_open"),
                open.to_string(),
                t("clash_open_hint"),
                if open == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("clash_critical"),
                critical.to_string(),
                t("clash_critical_hint"),
                if critical == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new(t("clash_none")).color(theme::ok()).size(15.0));
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("clash_grid")
                .num_columns(5)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    for (i, h) in [
                        (120.0, t("col_code")),
                        (90.0, t("col_section_short")),
                        (300.0, t("col_title")),
                        (200.0, t("col_element")),
                        (120.0, t("col_status")),
                    ] {
                        super::warehouse::cell_l(
                            ui,
                            i,
                            RichText::new(h).size(11.0).color(theme::muted()),
                        );
                    }
                    ui.end_row();

                    for issue in &rows {
                        super::warehouse::cell_l(
                            ui,
                            120.0,
                            RichText::new(&issue.code).size(12.0).monospace(),
                        );
                        super::warehouse::cell_l(
                            ui,
                            90.0,
                            RichText::new(issue.section.code())
                                .size(11.5)
                                .color(theme::muted()),
                        );
                        super::warehouse::cell_l(
                            ui,
                            300.0,
                            RichText::new(super::issues::truncate(&issue.title, 40))
                                .size(12.5)
                                .color(if issue.severity == Severity::Critical {
                                    theme::danger()
                                } else {
                                    theme::text()
                                }),
                        );
                        super::warehouse::cell_l(
                            ui,
                            200.0,
                            RichText::new(super::issues::truncate(&issue.element, 24)).size(12.0),
                        );
                        super::warehouse::cell_l(
                            ui,
                            120.0,
                            RichText::new(issue.status.label()).size(11.5).color(
                                if issue.status == IssueStatus::Fixed {
                                    theme::ok()
                                } else {
                                    theme::warn()
                                },
                            ),
                        );
                        ui.end_row();
                    }
                });
        });
}

fn issues_tab(ui: &mut egui::Ui, app: &mut App) {
    if issues::filter_bar(ui, app, t("run_project_check")) {
        app.run_project_check();
    }
    ui.add_space(6.0);

    let list = app.filtered_issues(IssueModule::Project);
    issues::issue_kpis(
        ui,
        &list,
        Some((
            t("kpi_elements"),
            app.elements.len().to_string(),
            &format!("{} {}", app.element_links.len(), t("kpi_links_graph")),
        )),
    );
    ui.add_space(6.0);
    issues::section_report(ui, &list);
    drop(list);
    ui.add_space(8.0);

    let h = (ui.available_height() - 12.0).max(180.0);
    let detail_w = (ui.available_width() * 0.34).clamp(300.0, 440.0);

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - detail_w - 16.0).max(260.0));
            if let Some(id) = issues::issue_table(ui, app, IssueModule::Project, h) {
                app.selected_issue = Some(id);
            }
        });
        ui.vertical(|ui| {
            ui.set_width(detail_w);
            issues::issue_detail(ui, app, h - 24.0);
        });
    });
}

// ---------------------------------------------------------------- Elementlar

fn elements_tab(ui: &mut egui::Ui, app: &mut App) {
    let mut add = false;
    let mut import = false;
    let mut import_dxf = false;
    let mut removed: Option<i64> = None;

    ui.horizontal(|ui| {
        if ui.button(t("add_element")).clicked() {
            add = true;
        }
        // TZ II.1-2: chizmani qo'lda kiritish o'rniga IFC dan o'qish.
        if ui
            .button(t("import_ifc"))
            .on_hover_text(t("import_ifc_hint"))
            .clicked()
        {
            import = true;
        }
        // DWG va RVT yopiq; DXF — o'sha CAD dasturlarining ochiq formati.
        if ui
            .button(t("dxf_import"))
            .on_hover_text(t("dxf_hint"))
            .clicked()
        {
            import_dxf = true;
        }
        ui.label(
            RichText::new(t("elements_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    if import {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("import_ifc"))
            .add_filter("IFC", &["ifc", "IFC"])
            .pick_file()
        {
            app.import_ifc(&path);
        }
    }

    if import_dxf {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("dxf_import"))
            .add_filter("DXF", &["dxf", "DXF"])
            .pick_file()
        {
            app.import_dxf(&path);
        }
    }

    if app.elements.is_empty() {
        empty(ui, t("no_elements"));
    } else {
        let h = (ui.available_height() - 12.0).max(180.0);
        let editor_w = (ui.available_width() * 0.38).clamp(340.0, 480.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - editor_w - 16.0).max(260.0));
                egui::ScrollArea::vertical()
                    .max_height(h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("elements_grid")
                            .num_columns(6)
                            .spacing([10.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let head = |ui: &mut egui::Ui, w: f32, s: &str| {
                                    ui.add_sized(
                                        [w, 16.0],
                                        egui::Label::new(
                                            RichText::new(s).color(theme::muted()).size(11.0),
                                        ),
                                    );
                                };
                                head(ui, 100.0, t("col_mark"));
                                head(ui, 110.0, t("col_kind"));
                                head(ui, 46.0, t("col_section_short"));
                                head(ui, 140.0, t("col_room"));
                                head(ui, 90.0, t("col_sheet"));
                                head(ui, 90.0, t("col_size"));
                                ui.end_row();

                                for el in &app.elements {
                                    let sel = app.selected_element == Some(el.id);
                                    let mark = if el.mark.trim().is_empty() {
                                        t("dash").to_string()
                                    } else {
                                        el.mark.clone()
                                    };
                                    let lbl =
                                        egui::Label::new(RichText::new(mark).size(12.0).color(
                                            if sel { theme::accent() } else { theme::text() },
                                        ))
                                        .sense(egui::Sense::click());
                                    if ui.add_sized([100.0, 17.0], lbl).clicked() {
                                        app.selected_element = Some(el.id);
                                    }
                                    let cell =
                                        |ui: &mut egui::Ui, w: f32, s: String, c: Color32| {
                                            ui.add_sized(
                                                [w, 17.0],
                                                egui::Label::new(
                                                    RichText::new(s).size(12.0).color(c),
                                                ),
                                            );
                                        };
                                    cell(ui, 110.0, el.kind.label().to_string(), theme::muted());
                                    cell(
                                        ui,
                                        46.0,
                                        el.section.label().to_string(),
                                        theme::section_color(el.section.color()),
                                    );
                                    cell(ui, 140.0, issues::truncate(&el.room, 22), theme::muted());
                                    cell(ui, 90.0, el.sheet.clone(), theme::muted());
                                    cell(
                                        ui,
                                        90.0,
                                        if el.size > 0.0 {
                                            format!("{} {}", trim_num(el.size), el.unit)
                                        } else {
                                            t("dash").to_string()
                                        },
                                        theme::muted(),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
            });

            ui.vertical(|ui| {
                ui.set_width(editor_w);
                removed = element_editor(ui, app, h - 24.0);
            });
        });
    }

    if add {
        if let Some(pid) = app.current {
            let el = Element {
                id: 0,
                project_id: pid,
                section: Section::Ar,
                kind: ElementKind::Room,
                mark: String::new(),
                room: String::new(),
                axis: String::new(),
                level: String::new(),
                size: 0.0,
                unit: String::new(),
                value: 0.0,
                value_name: String::new(),
                sheet: String::new(),
                note: String::new(),
            };
            let id = app.db.insert_element(&el);
            app.reload_modules();
            if id > 0 {
                app.selected_element = Some(id);
            }
        }
    }
    if let Some(id) = removed {
        app.db.del("element", id);
        app.selected_element = None;
        app.reload_modules();
    }
}

/// Tanlangan element kartochkasi. O'chirish so'ralgan bo'lsa, id qaytaradi.
fn element_editor(ui: &mut egui::Ui, app: &mut App, height: f32) -> Option<i64> {
    let Some(id) = app.selected_element else {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("select_element")).color(theme::muted()));
        });
        return None;
    };
    let Some(mut el) = app.elements.iter().find(|e| e.id == id).cloned() else {
        app.selected_element = None;
        return None;
    };
    let mut dirty = false;
    let mut remove = None;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_height(height);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(t("element_card"))
                            .size(14.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.add_space(6.0);

                    let row =
                        |ui: &mut egui::Ui, label: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [110.0, 20.0],
                                    egui::Label::new(
                                        RichText::new(label).color(theme::muted()).size(12.0),
                                    ),
                                );
                                add(ui);
                            });
                        };

                    row(ui, t("col_section_short"), &mut |ui| {
                        egui::ComboBox::from_id_salt("el_sec")
                            .selected_text(el.section.label())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for s in Section::ALL {
                                    dirty |= ui
                                        .selectable_value(&mut el.section, s, s.label())
                                        .changed();
                                }
                            });
                    });
                    row(ui, t("col_kind"), &mut |ui| {
                        egui::ComboBox::from_id_salt("el_kind")
                            .selected_text(el.kind.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for k in ElementKind::ALL {
                                    dirty |=
                                        ui.selectable_value(&mut el.kind, *k, k.label()).changed();
                                }
                            });
                    });
                    row(ui, t("col_mark"), &mut |ui| {
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.mark).desired_width(150.0))
                            .changed();
                    });
                    row(ui, t("col_room"), &mut |ui| {
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.room).desired_width(200.0))
                            .changed();
                    });
                    row(ui, t("col_axis"), &mut |ui| {
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.axis).desired_width(90.0))
                            .changed();
                        ui.label(
                            RichText::new(t("col_level"))
                                .color(theme::muted())
                                .size(12.0),
                        );
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.level).desired_width(70.0))
                            .changed();
                    });
                    row(ui, t("col_size"), &mut |ui| {
                        dirty |= ui
                            .add(
                                egui::DragValue::new(&mut el.size)
                                    .speed(1.0)
                                    .range(0.0..=1e9),
                            )
                            .changed();
                        dirty |= ui
                            .add(
                                egui::TextEdit::singleline(&mut el.unit)
                                    .desired_width(70.0)
                                    .hint_text(t("col_unit")),
                            )
                            .changed();
                    });
                    row(ui, t("col_value"), &mut |ui| {
                        dirty |= ui
                            .add(egui::DragValue::new(&mut el.value).speed(0.001))
                            .changed();
                        dirty |= ui
                            .add(
                                egui::TextEdit::singleline(&mut el.value_name)
                                    .desired_width(120.0)
                                    .hint_text(t("value_name_hint")),
                            )
                            .changed();
                    });
                    row(ui, t("col_sheet"), &mut |ui| {
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.sheet).desired_width(150.0))
                            .changed();
                    });
                    row(ui, t("col_note"), &mut |ui| {
                        dirty |= ui
                            .add(egui::TextEdit::singleline(&mut el.note).desired_width(230.0))
                            .changed();
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    // TZ II.18: element o'zgarsa, qaysi bo'limlar ta'sirlanadi.
                    ui.label(
                        RichText::new(t("impact_title"))
                            .size(13.0)
                            .strong()
                            .color(theme::accent()),
                    );
                    ui.label(
                        RichText::new(t("impact_hint"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.add_space(4.0);
                    let impact = checks::impact(&app.elements, &app.element_links, id);
                    if impact.is_empty() {
                        ui.label(
                            RichText::new(t("impact_empty"))
                                .color(theme::muted())
                                .size(12.0),
                        );
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for (sec, n) in impact {
                                ui.label(
                                    RichText::new(format!("{} · {n}", sec.label()))
                                        .size(12.0)
                                        .color(theme::section_color(sec.color())),
                                );
                            }
                        });
                    }

                    ui.add_space(12.0);
                    if ui
                        .button(RichText::new(t("delete_element")).color(theme::danger()))
                        .clicked()
                    {
                        remove = Some(id);
                    }
                });
        });

    if dirty {
        app.db.update_element(&el);
        if let Some(slot) = app.elements.iter_mut().find(|x| x.id == id) {
            *slot = el;
        }
    }
    remove
}

// ---------------------------------------------------------------- Bog'lanishlar

fn relations_tab(ui: &mut egui::Ui, app: &mut App) {
    if app.elements.len() < 2 {
        empty(ui, t("relations_need_elements"));
        return;
    }

    let mut removed: Option<i64> = None;
    let mut add: Option<(i64, i64, Relation)> = None;

    // Yangi bog'lanish qatori holati oynalar orasida saqlanadi.
    let key = egui::Id::new("new_relation");
    let (mut from, mut rel_idx, mut to) = ui
        .data(|d| d.get_temp::<(i64, usize, i64)>(key))
        .unwrap_or((app.elements[0].id, 0, app.elements[1].id));

    let name = |app: &App, id: i64| -> String {
        app.elements
            .iter()
            .find(|e| e.id == id)
            .map(|e| {
                let mark = if e.mark.trim().is_empty() {
                    e.room.clone()
                } else {
                    e.mark.clone()
                };
                format!("{} · {} [{}]", mark, e.kind.label(), e.section.label())
            })
            .unwrap_or_else(|| t("dash").to_string())
    };

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t("add_relation"))
                    .size(13.0)
                    .strong()
                    .color(theme::accent()),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let picker = |ui: &mut egui::Ui, salt: &str, cur: &mut i64| {
                    egui::ComboBox::from_id_salt(salt)
                        .selected_text(name(app, *cur))
                        .width(280.0)
                        .show_ui(ui, |ui| {
                            for e in &app.elements {
                                ui.selectable_value(cur, e.id, name(app, e.id));
                            }
                        });
                };
                picker(ui, "rel_from", &mut from);
                egui::ComboBox::from_id_salt("rel_kind")
                    .selected_text(Relation::ALL[rel_idx.min(Relation::ALL.len() - 1)].label())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for (i, r) in Relation::ALL.iter().enumerate() {
                            ui.selectable_value(&mut rel_idx, i, r.label());
                        }
                    });
                picker(ui, "rel_to", &mut to);
                if ui.button(t("create")).clicked() {
                    add = Some((from, to, Relation::ALL[rel_idx]));
                }
            });
            ui.label(
                RichText::new(t("relations_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
        });
    ui.data_mut(|d| d.insert_temp(key, (from, rel_idx, to)));

    ui.add_space(10.0);

    if app.element_links.is_empty() {
        ui.label(RichText::new(t("relations_empty")).color(theme::muted()));
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("relations_grid")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for l in &app.element_links {
                            ui.add_sized(
                                [300.0, 18.0],
                                egui::Label::new(RichText::new(name(app, l.from_el)).size(12.0)),
                            );
                            ui.add_sized(
                                [150.0, 18.0],
                                egui::Label::new(
                                    RichText::new(l.relation.label())
                                        .size(12.0)
                                        .color(theme::accent()),
                                ),
                            );
                            ui.add_sized(
                                [300.0, 18.0],
                                egui::Label::new(RichText::new(name(app, l.to_el)).size(12.0)),
                            );
                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(l.id);
                            }
                            ui.end_row();
                        }
                    });
            });
    }

    if let Some((f, tt, r)) = add {
        if f == tt {
            app.notify(t("relation_self").to_string());
        } else {
            app.db.insert_element_link(&ElementLink {
                id: 0,
                from_el: f,
                to_el: tt,
                relation: r,
            });
            app.reload_modules();
        }
    }
    if let Some(id) = removed {
        app.db.del("element_link", id);
        app.reload_modules();
    }
}

// ---------------------------------------------------------------- Normativ reyestri

fn norms_tab(ui: &mut egui::Ui, app: &mut App) {
    let mut save: Option<(String, Norm)> = None;

    ui.label(
        RichText::new(t("norm_registry_hint"))
            .color(theme::muted())
            .size(12.0),
    );
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (key, name_key) in RULES {
                let mut n = app.norms.get(*key).cloned().unwrap_or_default();
                let filled = n.filled();
                let header = format!(
                    "{}  ·  {}",
                    t(name_key),
                    if filled {
                        t("norm_filled")
                    } else {
                        t("norm_empty")
                    }
                );
                egui::CollapsingHeader::new(RichText::new(header).size(13.0).color(if filled {
                    theme::ok()
                } else {
                    theme::warn()
                }))
                .id_salt(*key)
                .show(ui, |ui| {
                    let mut changed = false;
                    let row =
                        |ui: &mut egui::Ui,
                         label: &str,
                         add: &mut dyn FnMut(&mut egui::Ui) -> bool| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [130.0, 20.0],
                                    egui::Label::new(
                                        RichText::new(label).color(theme::muted()).size(12.0),
                                    ),
                                );
                                add(ui)
                            })
                            .inner
                        };
                    changed |= row(ui, t("norm_doc"), &mut |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut n.doc)
                                .desired_width(280.0)
                                .hint_text(t("norm_doc_hint")),
                        )
                        .changed()
                    });
                    changed |= row(ui, t("norm_edition"), &mut |ui| {
                        ui.add(egui::TextEdit::singleline(&mut n.edition).desired_width(140.0))
                            .changed()
                    });
                    changed |= row(ui, t("norm_clause"), &mut |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut n.clause)
                                .desired_width(140.0)
                                .hint_text(t("norm_clause_hint")),
                        )
                        .changed()
                    });
                    changed |= row(ui, t("norm_text"), &mut |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut n.text)
                                .desired_width(480.0)
                                .desired_rows(2),
                        )
                        .changed()
                    });
                    changed |= row(ui, t("norm_param"), &mut |ui| {
                        let mut c = ui
                            .add(egui::DragValue::new(&mut n.param).speed(0.001))
                            .changed();
                        c |= ui
                            .checkbox(&mut n.param_set, t("norm_param_set"))
                            .on_hover_text(t("norm_param_hint"))
                            .changed();
                        c
                    });
                    changed |= row(ui, t("norm_source"), &mut |ui| {
                        ui.add(egui::TextEdit::singleline(&mut n.source).desired_width(280.0))
                            .changed()
                    });

                    ui.add_space(4.0);
                    // Tahrir saqlanmagani ko'rinib tursin: tugma rangi o'zgaradi.
                    let label = if changed {
                        RichText::new(t("norm_save")).strong().color(theme::warn())
                    } else {
                        RichText::new(t("norm_save"))
                    };
                    if ui.button(label).clicked() {
                        save = Some((key.to_string(), n.clone()));
                    }
                });
            }
            ui.add_space(20.0);
        });

    if let Some((k, n)) = save {
        app.save_norm(&k, n);
    }
}

/// 12.0 -> «12», 0.008 -> «0.008».
fn trim_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{:.0}", v)
    } else {
        format!("{v}")
    }
}
