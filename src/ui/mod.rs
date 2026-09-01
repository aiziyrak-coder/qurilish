//! Oyna bezagi, navigatsiya va umumiy vidjetlar.

mod aicheck;
mod analytics;
mod client;
mod copilot;
mod dashboard;
mod deals;
mod documents;
mod estimate;
mod foreman;
mod execdocs;
mod gantt;
mod issues;
mod journal;
mod machines;
mod materials;
mod passport;
mod ppr;
mod purchases;
mod quality;
mod requests;
mod safety;
mod sales;
mod search;
mod settings;
mod supervision;
mod timesheet;
mod warehouse;

use crate::app::{App, Readiness, Screen, NAV_GROUPS};
use crate::i18n::t;
use crate::theme;
use egui::{pos2, Align2, Color32, Context, FontFamily, FontId, Rect, RichText, Stroke};

/// Interfeys ikki tilli: kirill va lotin harflari to'liq qamrab olinishi uchun
/// tizim shriftini yuklaymiz — egui ichidagi shriftda qamrov to'liq emas.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\tahoma.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "ui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "ui".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

/// Joriy mavzuga mos egui uslubi. Mavzu almashganda qayta chaqiriladi.
pub fn install_theme(ctx: &Context) {
    let light = theme::theme() == theme::Theme::Light;
    let mut style = (*ctx.style()).clone();
    style.visuals = if light {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    style.visuals.panel_fill = theme::bg();
    style.visuals.window_fill = theme::panel();
    style.visuals.extreme_bg_color = if light {
        Color32::from_rgb(252, 253, 254)
    } else {
        Color32::from_rgb(20, 22, 26)
    };
    style.visuals.faint_bg_color = theme::row_alt();
    style.visuals.override_text_color = Some(theme::text());
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, theme::line());
    style.visuals.widgets.inactive.bg_fill = if light {
        Color32::from_rgb(240, 243, 247)
    } else {
        Color32::from_rgb(38, 42, 51)
    };
    style.visuals.widgets.inactive.weak_bg_fill = style.visuals.widgets.inactive.bg_fill;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, theme::line());
    style.visuals.widgets.hovered.bg_fill = if light {
        Color32::from_rgb(228, 234, 242)
    } else {
        Color32::from_rgb(52, 58, 70)
    };
    style.visuals.widgets.active.bg_fill = theme::accent().gamma_multiply(0.75);
    style.visuals.selection.bg_fill =
        theme::accent().gamma_multiply(if light { 0.22 } else { 0.35 });
    style.visuals.selection.stroke = Stroke::new(1.0_f32, theme::accent());
    style.visuals.window_corner_radius = 8.into();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);

    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    ctx.set_style(style);
}

pub fn draw(ctx: &Context, app: &mut App) {
    // Til, mavzu yoki masshtab o'zgargan bo'lsa, uslubni qayta quramiz.
    if app.restyle {
        install_theme(ctx);
        ctx.set_pixels_per_point(app.settings.ui_scale);
        app.restyle = false;
    }

    top_bar(ctx, app);
    side_bar(ctx, app);

    egui::CentralPanel::default().show(ctx, |ui| match app.screen {
        Screen::Dashboard => dashboard::show(ui, app),
        Screen::Passport => passport::show(ui, app),
        Screen::Gantt => gantt::show(ui, app),
        Screen::Ppr => ppr::show(ui, app),
        Screen::AiCheck => aicheck::show(ui, app),
        Screen::Estimate => estimate::show(ui, app),
        Screen::ExecDocs => execdocs::show(ui, app),
        Screen::Journal => journal::show(ui, app),
        Screen::Warehouse => warehouse::show(ui, app),
        Screen::Materials => materials::show(ui, app),
        Screen::Requests => requests::show(ui, app),
        Screen::Purchases => purchases::show(ui, app),
        Screen::Sales => sales::show(ui, app),
        Screen::Deals => deals::show(ui, app),
        Screen::Timesheet => timesheet::show(ui, app),
        Screen::Quality => quality::show(ui, app),
        Screen::Safety => safety::show(ui, app),
        Screen::Machines => machines::show(ui, app),
        Screen::Analytics => analytics::show(ui, app),
        Screen::Foreman => foreman::show(ui, app),
        Screen::TechSupervision => supervision::show(ui, app),
        Screen::Client => client::show(ui, app),
        Screen::Copilot => copilot::show(ui, app),
        Screen::Settings => settings::show(ui, app),
    });

    search::draw(ctx, app);
    dialogs(ctx, app);
    toast(ctx, app);
}

fn top_bar(ctx: &Context, app: &mut App) {
    egui::TopBottomPanel::top("top")
        .exact_height(52.0)
        .frame(
            egui::Frame::new()
                .fill(theme::panel())
                .stroke(Stroke::new(1.0_f32, theme::line()))
                .inner_margin(egui::Margin::symmetric(14, 8)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new("QURAi")
                        .size(20.0)
                        .strong()
                        .color(theme::accent()),
                );
                ui.label(
                    RichText::new(t("app_subtitle"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.add_space(20.0);

                ui.label(RichText::new(t("object")).color(theme::muted()));
                let current_name = app
                    .project()
                    .map(|p| format!("{} [{}]", p.name, p.code))
                    .unwrap_or_else(|| t("no_object").into());
                let mut pick: Option<i64> = None;
                egui::ComboBox::from_id_salt("project_picker")
                    .selected_text(current_name)
                    .width(330.0)
                    .show_ui(ui, |ui| {
                        for p in &app.projects {
                            let sel = app.current == Some(p.id);
                            if ui
                                .selectable_label(sel, format!("{} [{}]", p.name, p.code))
                                .clicked()
                            {
                                pick = Some(p.id);
                            }
                        }
                    });
                if let Some(id) = pick {
                    app.select_project(id);
                }

                ui.add_space(12.0);
                // Qidiruv har doim ko'rinib tursin — aks holda Ctrl+K ni
                // hech kim topmaydi.
                if ui
                    .button(RichText::new(t("search_button")).size(13.0))
                    .on_hover_text(t("search_keys"))
                    .clicked()
                {
                    app.search_open = true;
                    app.search_query.clear();
                    app.search_focus = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(app.today.format("%d.%m.%Y").to_string())
                            .color(theme::muted())
                            .size(13.0),
                    );
                    ui.separator();
                    if let Some(p) = app.project() {
                        let c = match p.status {
                            crate::model::ObjectStatus::InProgress => theme::ok(),
                            crate::model::ObjectStatus::Suspended => theme::danger(),
                            crate::model::ObjectStatus::Completed => theme::accent(),
                            _ => theme::warn(),
                        };
                        ui.label(RichText::new(p.status.label()).color(c).strong());
                    }
                });
            });
        });
}

/// Modul holatining rangi. Tayyor modul belgisiz qoladi — ro'yxat shovqinsiz
/// bo'lishi uchun faqat tugallanmagani ajratiladi.
fn readiness_color(r: Readiness) -> Option<Color32> {
    match r {
        Readiness::Ready => None,
        Readiness::Storage => Some(theme::warn()),
        Readiness::Planned => Some(theme::muted()),
    }
}

/// Navigatsiyaning bitta qatori: TZ raqami, nomi va holat belgisi.
fn nav_item(ui: &mut egui::Ui, screen: Screen, active: bool) -> egui::Response {
    let h = 31.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        if active {
            p.rect_filled(rect, 7.0, theme::accent().gamma_multiply(0.16));
            p.rect_filled(
                Rect::from_min_size(rect.min + egui::vec2(0.0, 5.0), egui::vec2(3.0, h - 10.0)),
                1.5,
                theme::accent(),
            );
        } else if resp.hovered() {
            p.rect_filled(rect, 7.0, theme::line().gamma_multiply(0.55));
        }

        let cy = rect.center().y;
        let ready = screen.readiness();

        // TZ raqami — doim bir xil ustunda turadi, shunda ro'yxat tekis ko'rinadi.
        let numeral = screen.numeral();
        if !numeral.is_empty() {
            p.text(
                pos2(rect.min.x + 34.0, cy),
                Align2::RIGHT_CENTER,
                numeral,
                egui::FontId::monospace(10.5),
                if active {
                    theme::accent()
                } else {
                    theme::muted()
                },
            );
        }

        let name_color = if active {
            theme::accent()
        } else if ready == Readiness::Ready {
            theme::text()
        } else {
            theme::muted()
        };
        p.text(
            pos2(rect.min.x + 44.0, cy),
            Align2::LEFT_CENTER,
            truncate_nav(screen.label(), 24),
            egui::FontId::proportional(13.5),
            name_color,
        );

        // Tugallanmagan modul o'ng chekkada nuqta bilan belgilanadi.
        if let Some(c) = readiness_color(ready) {
            let center = pos2(rect.max.x - 11.0, cy);
            if ready == Readiness::Storage {
                p.circle_filled(center, 3.2, c);
            } else {
                p.circle_stroke(center, 3.2, Stroke::new(1.2_f32, c));
            }
        }
    }
    resp
}

fn truncate_nav(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn side_bar(ctx: &Context, app: &mut App) {
    egui::SidePanel::left("nav")
        .exact_width(266.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::panel())
                .stroke(Stroke::new(1.0_f32, theme::line()))
                .inner_margin(egui::Margin::symmetric(12, 10)),
        )
        .show(ctx, |ui| {
            // Pastki blok o'lchami oldindan ajratiladi: ro'yxat qolgan joyni oladi.
            let bottom_h = 92.0;
            let list_h = (ui.available_height() - bottom_h).max(120.0);

            egui::ScrollArea::vertical()
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (title_key, screens) in NAV_GROUPS {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(t(title_key))
                                    .size(10.0)
                                    .color(theme::muted())
                                    .strong(),
                            );
                        });
                        ui.add_space(5.0);

                        for &screen in *screens {
                            let active = app.screen == screen;
                            let resp = nav_item(ui, screen, active);
                            // Ochiq modul ro'yxatdan tashqarida qolmasin:
                            // ilova ochilganda yoki oyna kichrayganda uni
                            // ko'rinadigan joyga suramiz.
                            if active && !ui.is_rect_visible(resp.rect) {
                                resp.scroll_to_me(Some(egui::Align::Center));
                            }
                            if resp.clicked() {
                                app.screen = screen;
                            }
                            let hint = match screen.readiness() {
                                Readiness::Ready => None,
                                Readiness::Storage => Some(t("readiness_storage")),
                                Readiness::Planned => Some(t("readiness_planned")),
                            };
                            if let Some(h) = hint {
                                resp.on_hover_text(h);
                            }
                            ui.add_space(1.0);
                        }
                    }

                    // Belgilar izohi — nuqtalar nimani bildirishini aytadi.
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(6.0);
                    let legend = |ui: &mut egui::Ui, filled: bool, c: Color32, text: &str| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui
                                .allocate_exact_size(egui::vec2(14.0, 12.0), egui::Sense::hover());
                            if filled {
                                ui.painter().circle_filled(r.center(), 3.2, c);
                            } else {
                                ui.painter().circle_stroke(
                                    r.center(),
                                    3.2,
                                    Stroke::new(1.2_f32, c),
                                );
                            }
                            ui.label(RichText::new(text).size(10.5).color(theme::muted()));
                        });
                    };
                    // Izohda faqat haqiqatda uchraydigan belgilar ko'rsatiladi.
                    let has = |r: Readiness| {
                        NAV_GROUPS
                            .iter()
                            .flat_map(|(_, s)| s.iter())
                            .any(|s| s.readiness() == r)
                    };
                    if has(Readiness::Storage) {
                        legend(ui, true, theme::warn(), t("legend_storage"));
                    }
                    if has(Readiness::Planned) {
                        legend(ui, false, theme::muted(), t("legend_planned"));
                    }
                    ui.add_space(10.0);
                });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            if ui
                .add_sized(
                    [ui.available_width(), 30.0],
                    egui::Button::new(t("new_object")),
                )
                .clicked()
            {
                new_project(app);
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    t("local_db"),
                    crate::db::default_db_path().display()
                ))
                .size(9.5)
                .color(theme::muted()),
            );
        });
}

fn new_project(app: &mut App) {
    let today = chrono::Local::now().date_naive();
    let p = crate::model::Project {
        id: 0,
        name: t("new_object_name").into(),
        code: format!("OBJ-{}", app.projects.len() + 1),
        address: String::new(),
        object_type: String::new(),
        floors: 0,
        area_total: 0.0,
        paid_total: 0.0,
        status: crate::model::ObjectStatus::Design,
        start_date: today,
        planned_end: today + chrono::Duration::days(365),
        contract_sum: 0.0,
        currency: app.settings.default_currency.clone(),
        funding_source: String::new(),
        notes: String::new(),
    };
    match app.db.insert_project(&p) {
        Ok(id) => {
            app.reload_projects();
            app.select_project(id);
            app.screen = Screen::Passport;
        }
        Err(e) => app.notify(format!("{}: {e}", t("err_create_object"))),
    }
}

fn dialogs(ctx: &Context, app: &mut App) {
    // Ishni o'chirishni tasdiqlash.
    if let Some(id) = app.confirm_delete_task {
        let name = app.task_name(id);
        let mut open = true;
        let mut confirmed = false;
        egui::Window::new(t("delete_task_title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!("«{name}»"));
                ui.label(
                    RichText::new(t("delete_task_hint"))
                        .color(theme::muted())
                        .size(12.0),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t("delete")).color(theme::danger()).strong())
                        .clicked()
                    {
                        confirmed = true;
                    }
                    if ui.button(t("cancel")).clicked() {
                        app.confirm_delete_task = None;
                    }
                });
            });
        if confirmed {
            app.delete_task(id);
            app.confirm_delete_task = None;
        } else if !open {
            app.confirm_delete_task = None;
        }
    }

    // Obyektni o'chirishni tasdiqlash: bitta bosishda butun obyekt yo'qolmasin.
    if let Some(id) = app.confirm_delete_project {
        let name = app
            .projects
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let mut open = true;
        let mut confirmed = false;
        egui::Window::new(t("delete_object_title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("«{name}»")).strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t("delete_object_hint"))
                        .color(theme::danger())
                        .size(12.0),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t("delete")).color(theme::danger()).strong())
                        .clicked()
                    {
                        confirmed = true;
                    }
                    if ui.button(t("cancel")).clicked() {
                        app.confirm_delete_project = None;
                    }
                });
            });
        if confirmed {
            app.delete_project(id);
            app.confirm_delete_project = None;
        } else if !open {
            app.confirm_delete_project = None;
        }
    }

    // Yangi bog'lanish parametrlari.
    if let Some(mut l) = app.link_dialog.clone() {
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let pred = app.task_name(l.pred);
        let succ = app.task_name(l.succ);
        egui::Window::new(t("link_dialog_title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new(format!("{} {pred}", t("link_pred"))).color(theme::muted()));
                ui.label(RichText::new(format!("{} {succ}", t("link_succ"))).color(theme::muted()));
                ui.separator();
                egui::ComboBox::from_label(t("link_type"))
                    .selected_text(l.kind.label())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for k in crate::model::LinkType::ALL {
                            ui.selectable_value(&mut l.kind, k, k.label());
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label(t("link_lag"));
                    ui.add(egui::DragValue::new(&mut l.lag).range(-365..=365));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(t("create")).strong()).clicked() {
                        apply = true;
                    }
                    if ui.button(t("cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        if apply {
            app.add_link(l.pred, l.succ, l.kind, l.lag);
            app.link_dialog = None;
            app.linking_from = None;
        } else if !open || cancel {
            app.link_dialog = None;
            app.linking_from = None;
        } else {
            app.link_dialog = Some(l);
        }
    }
}

fn toast(ctx: &Context, app: &mut App) {
    let Some((msg, ttl)) = app.toast.clone() else {
        return;
    };
    let dt = ctx.input(|i| i.stable_dt).min(0.1);
    let left = ttl - dt;
    if left <= 0.0 {
        app.toast = None;
        return;
    }
    app.toast = Some((msg.clone(), left));
    ctx.request_repaint();

    egui::Area::new("toast".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -24.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme::card())
                .stroke(Stroke::new(1.0_f32, theme::line()))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    ui.label(RichText::new(msg).color(theme::text()));
                });
        });
}

// ---------- Umumiy vidjetlar ----------

/// Raqamni qisqa ko'rinishda: 12.0 -> «12», 4.5 -> «4.5».
pub fn trim(v: f64) -> String {
    materials::trim_num(v)
}

/// Bosiladigan ko'rsatkich kartochkasi — bosilganda `true` qaytaradi.
pub fn stat_card_link(
    ui: &mut egui::Ui,
    title: &str,
    value: String,
    hint: &str,
    color: Color32,
) -> bool {
    let r = egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(168.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(12.0).color(theme::muted()));
                ui.add_space(2.0);
                ui.label(RichText::new(value).size(20.0).strong().color(color));
                if !hint.is_empty() {
                    ui.label(RichText::new(hint).size(11.0).color(theme::muted()));
                }
            });
        })
        .response
        .interact(egui::Sense::click());
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    r.clicked()
}

/// Ko'rsatkich kartochkasi.
pub fn stat_card(ui: &mut egui::Ui, title: &str, value: String, hint: &str, color: Color32) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(168.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(12.0).color(theme::muted()));
                ui.add_space(2.0);
                ui.label(RichText::new(value).size(20.0).strong().color(color));
                if !hint.is_empty() {
                    ui.label(RichText::new(hint).size(11.0).color(theme::muted()));
                }
            });
        });
}

/// «Sarlavha — qiymat» ko'rinishidagi maydon.
pub fn field(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [210.0, 22.0],
            egui::Label::new(RichText::new(label).color(theme::muted())),
        );
        add(ui);
    });
}

/// Kartochka ramkasi. Frame ota-elementning yo'nalishini meros qiladi,
/// shuning uchun ichini aniq vertikalga o'raymiz.
pub fn card_frame(ui: &mut egui::Ui, title: &str, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.label(
                    RichText::new(title)
                        .size(15.0)
                        .strong()
                        .color(theme::accent()),
                );
                ui.add_space(8.0);
                add(ui);
            });
        });
}

/// GPR ishini tanlash uchun ro'yxat. O'zgargan bo'lsa `true` qaytadi.
pub fn task_picker(
    ui: &mut egui::Ui,
    app: &App,
    salt: impl std::hash::Hash,
    cur: &mut Option<i64>,
    width: f32,
) -> bool {
    let mut changed = false;
    let label = cur
        .and_then(|id| app.task(id).map(|x| format!("{} {}", x.wbs, x.name)))
        .unwrap_or_else(|| t("no_task").to_string());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(issues::truncate(&label, 34))
        .width(width)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(cur, None, t("no_task")).changed();
            for task in &app.tasks {
                changed |= ui
                    .selectable_value(cur, Some(task.id), format!("{} {}", task.wbs, task.name))
                    .changed();
            }
        });
    changed
}

/// Faylni tizimning standart dasturida ochadi.
pub fn open_path(path: &str) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// Vektorli «bajarildi» belgisi. Glif bilan chizilsa, tizim shriftida
/// bo'lmasligi mumkin — kvadratcha chiqib qolardi.
pub fn draw_check(p: &egui::Painter, c: egui::Pos2, r: f32, color: Color32, width: f32) {
    p.line_segment(
        [
            pos2(c.x - r, c.y + r * 0.1),
            pos2(c.x - r * 0.25, c.y + r * 0.75),
        ],
        Stroke::new(width, color),
    );
    p.line_segment(
        [
            pos2(c.x - r * 0.25, c.y + r * 0.75),
            pos2(c.x + r, c.y - r * 0.65),
        ],
        Stroke::new(width, color),
    );
}

/// Summani razryadlarga ajratib chiqaradi: 48 500 000 000.
///
/// Barcha pul qiymatlari shu funksiyadan o'tadi, shuning uchun hisobdagi
/// nosozlik («NaN», «inf») ekranga chiqib ketmasligi kerak — bunday qiymat
/// chiziqcha bilan ko'rsatiladi.
pub fn money(v: f64) -> String {
    if !v.is_finite() {
        return t("dash").to_string();
    }
    let s = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    if v < 0.0 {
        format!("-{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_digits() {
        assert_eq!(money(0.0), "0");
        assert_eq!(money(48_500_000_000.0), "48 500 000 000");
        assert_eq!(money(1234.0), "1 234");
        assert_eq!(money(-1_500_000.0), "-1 500 000");
    }

    /// Hisobdagi nosozlik ekranga «NaN» bo'lib chiqmasin.
    #[test]
    fn money_rejects_non_finite() {
        assert_eq!(money(f64::NAN), t("dash"));
        assert_eq!(money(f64::INFINITY), t("dash"));
        assert_eq!(money(f64::NEG_INFINITY), t("dash"));
    }
}
