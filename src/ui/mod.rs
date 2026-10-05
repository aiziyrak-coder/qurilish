//! Oyna bezagi, navigatsiya va umumiy vidjetlar.

mod aicheck;
mod analytics;
mod client;
mod contracts;
mod copilot;
mod dashboard;
mod deals;
mod director;
mod documents;
mod estimate;
mod execdocs;
pub mod export;
mod foreman;
mod gantt;
mod inspections;
mod issues;
pub mod journal;
mod machines;
pub mod materials;
mod notes;
mod notices;
mod passport;
mod plan;
mod portfolio;
mod ppr;
mod purchases;
mod quality;
mod reports;
mod requests;
mod safety;
pub(crate) mod sales;
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

/// Modul yordamchisi tugmasi (TZ VI.30, VII.33, VIII.32, IX.35, XI.45-46,
/// XII.39, XIII.40, XIV.38, XV.38).
///
/// Tugma yangi yordamchi ochmaydi — mavjud yordamchini shu modulning
/// savoli bilan ochadi. Shuning uchun javob har doim bitta joyda
/// hisoblanadi va ekranlar orasida farq qilmaydi.
pub fn assistant_button(ui: &mut egui::Ui, app: &mut App) {
    let Some(intent) = crate::copilot::Intent::for_screen(app.screen) else {
        return;
    };
    if ui
        .button(t("assist_open"))
        .on_hover_text(intent.question())
        .clicked()
    {
        app.copilot_intent = Some(intent);
        app.screen = Screen::Copilot;
    }
}

pub fn draw(ctx: &Context, app: &mut App) {
    // Til, mavzu yoki masshtab o'zgargan bo'lsa, uslubni qayta quramiz.
    if app.restyle {
        install_theme(ctx);
        ctx.set_pixels_per_point(app.settings.ui_scale);
        app.restyle = false;
    }

    // Fon oqimidagi sinxronizatsiya natijasi tayyor bo'lsa qo'llanadi.
    app.poll_sync();
    app.poll_hook();

    // Bazada yozuv o'zgargan bo'lsa bildirishnomalarni qayta yig'amiz.
    // Tekshiruv — bitta atomik son bilan, hisob esa faqat kerak bo'lganda.
    if app.db.revision() != app.notices_rev {
        app.notices_rev = app.db.revision();
        app.refresh_notices();
    }

    top_bar(ctx, app);
    side_bar(ctx, app);

    egui::CentralPanel::default().show(ctx, |ui| {
        // Har ekran o'zini tanishtiradi: nomi va nima uchun kerakligi.
        screen_header(ui, app);
        // Rol bu ekranni o'zgartira olmasa — buni yashirmaymiz, ochiq aytamiz.
        if !app.can_edit(app.screen) {
            readonly_banner(ui, app);
        }
        match app.screen {
            Screen::Dashboard => dashboard::show(ui, app),
            Screen::Portfolio => portfolio::show(ui, app),
            Screen::Notices => notices::show(ui, app),
            Screen::Director => director::show(ui, app),
            Screen::Inspections => inspections::show(ui, app),
            Screen::Contracts => contracts::show(ui, app),
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
            Screen::Reports => reports::show(ui, app),
            Screen::Foreman => foreman::show(ui, app),
            Screen::TechSupervision => supervision::show(ui, app),
            Screen::Client => client::show(ui, app),
            Screen::Copilot => copilot::show(ui, app),
            Screen::Settings => settings::show(ui, app),
        }
    });

    search::draw(ctx, app);
    dialogs(ctx, app);
    toast(ctx, app);
}

/// Joriy ekran jadvalini `.xlsx` ga saqlaydi (umumiy talab).
///
/// Fayl nomi ekran nomi va sanadan tuziladi — bir necha eksport bir-birining
/// ustiga yozilmasin.
pub fn export_current(app: &mut App) {
    let Some(table) = export::table_of(app, app.screen) else {
        app.notify(t("export_none").to_string());
        return;
    };
    let rows = table.rows.len();
    let name = export::file_name(app, app.screen);
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("export"))
        .set_file_name(&name)
        .add_filter("Excel", &["xlsx"])
        .add_filter("PDF", &["pdf"])
        .save_file()
    else {
        return;
    };
    // Formatni kengaytma belgilaydi: Excel — ish uchun, PDF — topshirish
    // uchun. Ikkalasi ham bitta jadvaldan chiqadi.
    let pdf = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    let result = if pdf {
        let subtitle = format!(
            "{} · {}",
            app.project().map(|p| p.name.clone()).unwrap_or_default(),
            app.today.format("%d.%m.%Y")
        );
        crate::pdf::write_table(&path, &table, subtitle.trim_start_matches(" · "))
    } else {
        crate::docgen::write_table(&path, &table).map_err(|e| format!("{e}"))
    };
    match result {
        Ok(()) => app.notify(format!("{} {rows} · {}", t("export_done"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("export_failed"))),
    }
}

/// Hujjat davri: joriy oyning boshidan bugungacha.
///
/// KS-2 va KS-3 odatda oylik topshiriladi, shuning uchun sukut bo'yicha shu
/// davr olinadi. Boshqa davr kerak bo'lsa hujjatda tahrirlanadi.
pub fn doc_period(app: &App) -> (chrono::NaiveDate, chrono::NaiveDate) {
    let start = chrono::NaiveDate::from_ymd_opt(
        chrono::Datelike::year(&app.today),
        chrono::Datelike::month(&app.today),
        1,
    )
    .unwrap_or(app.today);
    (start, app.today)
}

/// Ilova belgisi va nomi.
///
/// Nom va tagsarlavha ikki qatorda turadi: bitta qatorga cho'zilganda ular
/// bir-biriga yopishib, siqilgan ko'rinardi.
fn logo(ui: &mut egui::Ui, wide: bool) {
    let size = 30.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect_filled(rect, 8.0, theme::accent());
        p.text(
            rect.center() + egui::vec2(0.0, -0.5),
            Align2::CENTER_CENTER,
            "Q",
            egui::FontId::proportional(18.0),
            theme::on_accent(),
        );
    }
    ui.add_space(9.0);

    ui.vertical(|ui| {
        // Ikki qator markazga tekislanadi: panel balandligi 52 px, matn 30 px.
        ui.add_space(if wide { 3.0 } else { 7.0 });
        ui.label(
            RichText::new("QURAi")
                .size(16.5)
                .strong()
                .color(theme::text()),
        );
        if wide {
            ui.add_space(-3.0);
            ui.label(
                RichText::new(t("app_subtitle"))
                    .size(9.5)
                    .color(theme::muted()),
            );
        }
    });
}

/// Yuqori paneldagi vertikal ajratkich.
fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 24.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 0.0, theme::line());
    }
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
            // Oyna kengligiga qarab nima ko'rsatilishini hal qilamiz: tor oynada
            // elementlar bir-birining ustiga chiqib ketmasligi kerak.
            let w = ui.available_width();
            let wide = w > 1240.0;
            let medium = w > 1000.0;

            ui.horizontal_centered(|ui| {
                logo(ui, wide);
                ui.add_space(if wide { 16.0 } else { 8.0 });
                // Nozik ajratkich: logotip va ish maydoni bir-biriga
                // yopishib qolmasin.
                divider(ui);
                ui.add_space(if wide { 14.0 } else { 8.0 });

                if medium {
                    ui.label(RichText::new(t("object")).color(theme::muted()));
                }
                let current_name = app
                    .project()
                    .map(|p| format!("{} [{}]", p.name, p.code))
                    .unwrap_or_else(|| t("no_object").into());
                let mut pick: Option<i64> = None;
                egui::ComboBox::from_id_salt("project_picker")
                    .selected_text(current_name)
                    .width(if wide { 330.0 } else { 240.0 })
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

                ui.add_space(10.0);
                // Qidiruv har doim ko'rinib tursin — aks holda Ctrl+K ni
                // hech kim topmaydi. Tor oynada faqat belgisi qoladi.
                let label = if medium {
                    t("search_button").to_string()
                } else {
                    t("search_short").to_string()
                };
                if ui
                    .button(RichText::new(label).size(13.0))
                    .on_hover_text(t("search_keys"))
                    .clicked()
                {
                    app.search_open = true;
                    app.search_query.clear();
                    app.search_focus = true;
                }

                // Eksport yuqori panelda emas: u ko'pincha o'chiq turardi
                // (jadvali yo'q ekranda) va faqat joy egallardi. Joriy
                // ekran jadvali `Ctrl+E` bilan chiqariladi, to'liq
                // hisobotlar esa «Hisobotlar» bo'limida.
                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::E))
                    && export::table_of(app, app.screen).is_some()
                {
                    export_current(app);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if wide {
                        ui.label(
                            RichText::new(app.today.format("%d.%m.%Y").to_string())
                                .color(theme::muted())
                                .size(13.0),
                        );
                        ui.separator();
                    }
                    role_picker(ui, app, medium);
                    ui.separator();
                    ai_button(ui, app);
                    if medium {
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
                    }
                });
            });
        });
}

/// Tepadagi AI tugmasi: kalit shu yerda qo'yiladi va holat ko'rinib turadi.
///
/// Sababi: kalit sozlamalarning ichida turganda uni topish qiyin edi va
/// yordamchi faqat o'z ekranida ochilardi. Endi har ekrandan bir bosishda
/// ochiladi — kalit qo'yilgan bo'lsa yordamchi hamma bo'lim bo'yicha
/// javob beradi, chunki unga **barcha modullarning** sonlari beriladi.
///
/// Kalit ekranda ochiq ko'rsatilmaydi: maydon yashirin, ro'yxatda esa
/// faqat oxirgi to'rt belgisi turadi.
fn ai_button(ui: &mut egui::Ui, app: &mut App) {
    let ready = app.llm.is_ready();
    let (dot, tip) = if ready {
        (theme::ok(), t("ai_ready"))
    } else if app.llm.api_key.trim().is_empty() {
        (theme::muted(), t("ai_no_key"))
    } else {
        (theme::warn(), t("ai_off"))
    };

    let mut go_copilot = false;
    let mut go_settings = false;
    let mut save = false;
    let mut key = app.llm.api_key.clone();

    let resp = ui
        .button(RichText::new(format!("● {}", t("ai_short"))).color(dot))
        .on_hover_text(tip);
    if resp.clicked() {
        app.ai_panel = !app.ai_panel;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ai_panel = false;
    }
    if !app.ai_panel {
        return;
    }

    let area = egui::Area::new(egui::Id::new("ai_panel"))
        .order(egui::Order::Foreground)
        .fixed_pos(resp.rect.left_bottom())
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(330.0);
                ui.label(RichText::new(t("ai_title")).strong());
                ui.label(
                    RichText::new(t("ai_where"))
                        .size(10.5)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);

                ui.label(RichText::new(t("set_llm_key")).size(11.0));
                // `password` — kalit yelka ustidan ham, ekran rasmida ham
                // ko'rinmasin.
                let edited = ui
                    .add_sized(
                        [310.0, 22.0],
                        egui::TextEdit::singleline(&mut key)
                            .password(true)
                            .hint_text("sk-..."),
                    )
                    .changed();
                if edited {
                    save = true;
                }
                if !app.llm.api_key.trim().is_empty() {
                    ui.label(
                        RichText::new(app.llm.masked_key())
                            .size(10.5)
                            .color(theme::muted()),
                    );
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("set_llm_model")).size(11.0));
                    egui::ComboBox::from_id_salt("ai_model")
                        .selected_text(app.llm.model.clone())
                        .width(190.0)
                        .show_ui(ui, |ui| {
                            for m in crate::llm::MODELS {
                                if ui.selectable_label(app.llm.model == *m, *m).clicked() {
                                    app.llm.model = (*m).to_string();
                                    save = true;
                                }
                            }
                        });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(ready, egui::Button::new(t("ai_ask")))
                        .on_disabled_hover_text(t("ai_no_key"))
                        .clicked()
                    {
                        go_copilot = true;
                    }
                    if ui.button(t("screen_settings")).clicked() {
                        go_settings = true;
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t("ai_key_safety"))
                        .size(10.0)
                        .color(theme::muted()),
                );
            });
        });

    // Tashqariga bosilsa yopiladi — tugmaning o'ziga bosilgan holat
    // bundan mustasno, aks holda bosish darhol bekor bo'lardi.
    let outside = ui.input(|i| {
        i.pointer.any_click()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p) && !resp.rect.contains(p))
    });
    if outside {
        app.ai_panel = false;
    }

    if save {
        app.set_llm_key(&key);
    }
    if go_copilot {
        app.screen = Screen::Copilot;
        app.ai_panel = false;
    }
    if go_settings {
        app.screen = Screen::Settings;
        app.ai_panel = false;
    }
}

/// Joriy foydalanuvchi va rol. Bosilsa — almashtirish ro'yxati.
fn role_picker(ui: &mut egui::Ui, app: &mut App, wide: bool) {
    let role = app.role();
    let name = app
        .current_user
        .and_then(|id| app.users.iter().find(|u| u.id == id))
        .map(|u| u.name.clone())
        .unwrap_or_else(|| t("role_nobody").to_string());
    let color = if app.can_edit(app.screen) {
        theme::muted()
    } else {
        // Faqat o'qish holati yuqori panelda ham ko'rinib tursin.
        theme::warn()
    };

    let mut pick: Option<Option<i64>> = None;
    let mut pick_role: Option<Option<crate::roles::Role>> = None;
    // Tor oynada faqat rol nomi qoladi — ism kesilgandan ko'ra tushib qolgani yaxshi.
    let text = if wide {
        format!("{name} · {}", role.label())
    } else {
        role.label().to_string()
    };
    egui::ComboBox::from_id_salt("role_picker")
        .selected_text(RichText::new(text).color(color))
        .width(if wide { 220.0 } else { 150.0 })
        .show_ui(ui, |ui| {
            ui.label(
                RichText::new(t("role_switch_hint"))
                    .size(10.5)
                    .color(theme::muted()),
            );
            ui.separator();
            if ui
                .selectable_label(
                    app.current_user.is_none() && app.view_role.is_none(),
                    t("role_nobody"),
                )
                .clicked()
            {
                pick_role = Some(None);
                pick = Some(None);
            }

            // ---- Barcha rollar: xodim yaratmasdan ham har birining ish
            // o'rnini ochib ko'rish mumkin.
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("role_all_roles"))
                    .size(10.0)
                    .color(theme::muted())
                    .strong(),
            );
            for r in crate::roles::Role::ALL {
                let chosen = app.current_user.is_none() && app.view_role == Some(*r);
                let n = r.screens().len();
                let label = if n > 0 {
                    format!("{} · {n}", r.label())
                } else {
                    r.label().to_string()
                };
                if ui
                    .selectable_label(chosen, label)
                    .on_hover_text(r.hint())
                    .clicked()
                {
                    pick_role = Some(Some(*r));
                }
            }

            // ---- Bazaga kiritilgan xodimlar.
            if !app.users.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("role_people"))
                        .size(10.0)
                        .color(theme::muted())
                        .strong(),
                );
                for u in &app.users {
                    let label = format!("{} · {}", u.name, u.role.label());
                    if ui
                        .selectable_label(app.current_user == Some(u.id), label)
                        .clicked()
                    {
                        pick = Some(Some(u.id));
                    }
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.label(
                RichText::new(t("role_not_a_lock"))
                    .size(10.0)
                    .color(theme::muted()),
            );
        })
        .response
        .on_hover_text(role.hint());

    // Xodim tanlovi ustun: u rolni ham o'zi belgilaydi.
    if let Some(id) = pick {
        app.set_user(id);
    } else if let Some(r) = pick_role {
        app.set_view_role(r);
    }
}

/// Ekran faqat o'qish uchun ochilganda tepada chiqadigan tasma.
fn readonly_banner(ui: &mut egui::Ui, app: &App) {
    egui::Frame::new()
        .fill(theme::warn().gamma_multiply(0.14))
        .stroke(Stroke::new(1.0_f32, theme::warn()))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t("role_readonly"))
                        .size(12.0)
                        .color(theme::warn())
                        .strong(),
                );
                ui.label(
                    RichText::new(format!("{}: {}", t("role_current"), app.role().label()))
                        .size(11.5)
                        .color(theme::muted()),
                );
            });
        });
    ui.add_space(6.0);
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
fn nav_item(
    ui: &mut egui::Ui,
    screen: Screen,
    active: bool,
    badge: Option<(usize, egui::Color32)>,
) -> egui::Response {
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

        // Faol qatorni chap chekkadagi chiziq ko'rsatadi: TZ raqami bu
        // yerda emas — u qurilishchiga hech narsa aytmaydi va ro'yxatni
        // og'irlashtiradi. Raqam sichqoncha ustiga kelganda chiqadi.
        if active {
            p.rect_filled(
                Rect::from_min_size(
                    pos2(rect.min.x, rect.min.y + 4.0),
                    egui::vec2(3.0, rect.height() - 8.0),
                ),
                1.5,
                theme::accent(),
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
            pos2(rect.min.x + 14.0, cy),
            Align2::LEFT_CENTER,
            truncate_nav(screen.label(), 28),
            egui::FontId::proportional(13.5),
            name_color,
        );

        // Bildirishnoma soni — o'ng chekkada rangli belgi.
        if let Some((n, color)) = badge {
            let text = if n > 99 {
                "99+".to_string()
            } else {
                n.to_string()
            };
            let w = 14.0 + text.len() as f32 * 5.5;
            let br =
                Rect::from_center_size(pos2(rect.max.x - 8.0 - w / 2.0, cy), egui::vec2(w, 17.0));
            p.rect_filled(br, 8.5, color);
            p.text(
                br.center(),
                Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(10.5),
                theme::on_accent(),
            );
        }

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

/// Ekran sarlavhasi: nomi, TZ bo'limi va bir qatorlik izoh.
///
/// Izoh ataylab har doim ko'rinadi va yashirilmaydi: ekran ko'p va
/// ularning nomi hammaga bir xil tushunarli emas. Bir qator o'qishga bir
/// soniya ketadi, noto'g'ri ekranda ish qilish esa yarim kunni oladi.
fn screen_header(ui: &mut egui::Ui, app: &mut App) {
    let screen = app.screen;
    ui.horizontal(|ui| {
        ui.label(RichText::new(screen.label()).size(19.0).strong());
        let numeral = screen.numeral();
        if !numeral.is_empty() {
            ui.label(
                RichText::new(numeral)
                    .size(11.0)
                    .monospace()
                    .color(theme::muted()),
            )
            .on_hover_text(t("header_numeral_hint"));
        }
        // Modul yordamchisi shu yerda: ilgari u har ekranda alohida
        // qator egallab turardi va bir xil tugma ikki xil joyda edi.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            assistant_button(ui, app);
        });
    });
    ui.label(
        RichText::new(screen.purpose())
            .size(12.0)
            .color(theme::muted()),
    );
    ui.add_space(8.0);
}

/// Bo'limlar qatori: kunlik ishlaydiganlari oldinda.
///
/// Ekranlarda sakkiztagacha bo'lim bor va ular bir xil ko'rinardi — kerakli
/// bo'limni topish uchun hammasini o'qib chiqishga to'g'ri kelardi. Endi
/// har kuni ochiladiganlari oldinda, vaqti-vaqti bilan kerak bo'ladiganlari
/// ajratgichdan keyin turadi. Raqamlar o'zgarmaydi: faqat ko'rinish tartibi
/// boshqacha, shuning uchun eski holat ham to'g'ri ochiladi.
pub fn tab_row(ui: &mut egui::Ui, tab: &mut u8, daily: &[(u8, &str)], rare: &[(u8, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for (i, label) in daily {
            if ui.selectable_label(*tab == *i, *label).clicked() {
                *tab = *i;
            }
        }
        if !rare.is_empty() {
            ui.separator();
            for (i, label) in rare {
                if ui
                    .selectable_label(
                        *tab == *i,
                        RichText::new(*label).color(if *tab == *i {
                            theme::text()
                        } else {
                            theme::muted()
                        }),
                    )
                    .clicked()
                {
                    *tab = *i;
                }
            }
        }
    });
}

/// Yon paneldagi guruh sarlavhasi.
fn group_title(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(
            RichText::new(text)
                .size(10.0)
                .color(theme::muted())
                .strong(),
        );
    });
    ui.add_space(5.0);
}

/// Yon paneldagi bitta qator: belgi, maslahat va bosilishi.
fn nav_row(
    ui: &mut egui::Ui,
    app: &mut App,
    screen: Screen,
    notice_count: usize,
    notice_color: egui::Color32,
    jumped: Option<Screen>,
) {
    let active = app.screen == screen;
    // Bildirishnomalar ekrani yonida ochiq savollar soni.
    let badge = (screen == Screen::Notices)
        .then_some(())
        .and_then(|_| (notice_count > 0).then_some((notice_count, notice_color)));
    let resp = nav_item(ui, screen, active, badge);
    // Ochiq modul ro'yxatdan tashqarida qolib ketmasin — lekin bu **faqat
    // modul almashganda** qilinadi.
    //
    // Ilgari tekshiruv har kadrda ishlardi: foydalanuvchi ro'yxatni pastga
    // surganda faol qator ko'rinishdan chiqar, keyingi kadrda ro'yxat o'zi
    // tepaga qaytib ketardi va pastga tushib bo'lmasdi.
    if active && jumped != Some(screen) && !ui.is_rect_visible(resp.rect) {
        resp.scroll_to_me(Some(egui::Align::Center));
    }
    if resp.clicked() {
        app.screen = screen;
    }
    // Sichqoncha ustida: TZ bo'limi va (bo'lsa) tayyorlik holati.
    let numeral = screen.numeral();
    let hint = match screen.readiness() {
        Readiness::Ready => String::new(),
        Readiness::Storage => t("readiness_storage").to_string(),
        Readiness::Planned => t("readiness_planned").to_string(),
    };
    let tip = match (numeral.is_empty(), hint.is_empty()) {
        (true, true) => String::new(),
        (true, false) => hint,
        (false, true) => format!("{} {numeral}", t("search_module")),
        (false, false) => format!("{} {numeral} · {hint}", t("search_module")),
    };
    if !tip.is_empty() {
        resp.on_hover_text(tip);
    }
    ui.add_space(1.0);
}

/// Barcha bo'limlar, TZ bosqichlari bo'yicha guruhlab.
fn all_groups(
    ui: &mut egui::Ui,
    app: &mut App,
    notice_count: usize,
    notice_color: egui::Color32,
    jumped: Option<Screen>,
) {
    for (title_key, screens) in NAV_GROUPS {
        group_title(ui, t(title_key));
        for &screen in *screens {
            nav_row(ui, app, screen, notice_count, notice_color, jumped);
        }
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

            let list = egui::ScrollArea::vertical()
                .max_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Ro'yxat App da keshlangan: uni har kadrda qayta yig'ish
                    // o'nlab SQL so'rovni anglatardi.
                    // Oxirgi marta qaysi modulga surilgani: shu modul
                    // ochiq turganda ro'yxat boshqa surilmaydi.
                    let jump_key = egui::Id::new("nav_jumped_to");
                    let jumped = ui.data(|d| d.get_temp::<Screen>(jump_key));

                    let notice_count = app.notices.len();
                    let notice_color = match crate::notify::top_severity(&app.notices) {
                        Some(crate::domain::Severity::Critical) => theme::danger(),
                        Some(crate::domain::Severity::Major) => theme::warn(),
                        _ => theme::accent(),
                    };

                    // Rol tanlangan bo'lsa, uning o'z bo'limlari tepada
                    // alohida turadi. Qolganlari **yashirilmaydi** —
                    // yopiq sarlavha ostida qoladi va bir bosishda
                    // ochiladi: yashirilgan ma'lumot ishonchni yo'qotadi
                    // va odamlar baribir bir-biridan so'rab oladi.
                    let own = app.role().screens();
                    if own.is_empty() {
                        all_groups(ui, app, notice_count, notice_color, jumped);
                    } else {
                        group_title(ui, t("nav_my_work"));
                        for &screen in own {
                            nav_row(ui, app, screen, notice_count, notice_color, jumped);
                        }
                        ui.add_space(12.0);
                        egui::CollapsingHeader::new(
                            RichText::new(t("nav_other"))
                                .size(10.0)
                                .color(theme::muted())
                                .strong(),
                        )
                        .id_salt("nav_other")
                        .default_open(false)
                        .show(ui, |ui| {
                            all_groups(ui, app, notice_count, notice_color, jumped);
                        });
                    }

                    // Joriy modul belgilanadi: keyingi kadrlarda ro'yxat
                    // erkin suriladi.
                    ui.data_mut(|d| d.insert_temp(jump_key, app.screen));

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
            // Sinov ro'yxat haqiqatan erkin surilishini tekshirishi uchun
            // uning belgisi eslab qolinadi.
            ui.data_mut(|d| d.insert_temp(egui::Id::new("nav_scroll_id"), list.id));

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

/// Bitta ko'rsatkich kartochkasining tavsifi.
pub struct Stat {
    pub title: String,
    pub value: String,
    pub hint: String,
    pub color: Color32,
    /// Kartochka bosiladigan bo'lsa — kursor va javob o'zgaradi.
    pub link: bool,
}

/// Ko'rsatkich kartochkasi tavsifini yaratadi.
pub fn stat(title: &str, value: String, hint: &str, color: Color32) -> Stat {
    Stat {
        title: title.to_string(),
        value,
        hint: hint.to_string(),
        color,
        link: false,
    }
}

impl Stat {
    /// Kartochkani bosiladigan qiladi.
    pub fn link(mut self) -> Stat {
        self.link = true;
        self
    }
}

/// Ko'rsatkichlar qatori. Bosilgan kartochkaning tartib raqamini qaytaradi.
///
/// `horizontal_wrapped` kartochka kengligini oldindan bilmaydi va shu sababli
/// o'ramaydi — tor oynada oxirgi kartochkalar chetga chiqib ketardi. Shuning
/// uchun qatorga nechtasi sig'ishini o'zimiz hisoblaymiz.
pub fn stat_row(ui: &mut egui::Ui, cards: Vec<Stat>) -> Option<usize> {
    if cards.is_empty() {
        return None;
    }
    // Kartochka kengligi 168 + hoshiya 28 + oraliq 8.
    const SLOT: f32 = 204.0;
    let per_row = ((ui.available_width() / SLOT).floor() as usize).max(1);
    let mut clicked = None;
    for (chunk_i, chunk) in cards.chunks(per_row).enumerate() {
        ui.horizontal(|ui| {
            for (i, c) in chunk.iter().enumerate() {
                let hit = if c.link {
                    stat_card_link(ui, &c.title, c.value.clone(), &c.hint, c.color)
                } else {
                    stat_card(ui, &c.title, c.value.clone(), &c.hint, c.color);
                    false
                };
                if hit {
                    clicked = Some(chunk_i * per_row + i);
                }
            }
        });
        ui.add_space(6.0);
    }
    clicked
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
/// ISO 8601 vaqtni «08.09 07:41» ko'rinishiga keltiradi.
///
/// Serverdan kelgan vaqt UTC da va to'liq yozilgan; ekranda esa qisqa
/// ko'rinish o'qishga qulay.
pub fn short_stamp(iso: &str) -> String {
    let (date, time) = iso.split_once('T').unwrap_or((iso, ""));
    let parts: Vec<&str> = date.split('-').collect();
    let hm: String = time.chars().take(5).collect();
    if parts.len() == 3 {
        format!("{}.{} {hm}", parts[2], parts[1]).trim().to_string()
    } else {
        format!("{date} {hm}").trim().to_string()
    }
}

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

#[cfg(test)]
mod screen_tests {
    use super::*;
    use crate::db::Db;

    /// Vaqtinchalik baza — sinovlar bir-biriga xalaqit bermasligi uchun.
    fn temp_db() -> (std::path::PathBuf, Db) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("qurai_ui_{}_{n}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).expect("baza");
        (path, db)
    }

    /// Ekranlardagi tab tanlovi shu kalitlar ostida saqlanadi.
    ///
    /// Ro'yxat qo'lda yuritiladi: yangi tab qo'shilganda shu yerga ham
    /// qo'shiladi, aks holda u sinovdan o'tmay qoladi.
    const TAB_KEYS: &[&str] = &[
        "an_tab",
        "cp_tab",
        "ct_tab",
        "ed_tab",
        "estimate_tab_v2",
        "in_tab",
        "jr_tab",
        "mat_tab",
        "mch_tab",
        "ppr_tab_v2",
        "pu_tab",
        "ql_tab",
        "sales_tab",
        "sf_tab",
        "nt_tab",
        "ts_tab",
        "wh_tab",
    ];

    /// Eng ko'p tabli ekrandagi tab soni.
    const TABS: u8 = 9;

    /// Bitta kadrni oynasiz chizadi.
    ///
    /// egui immediate-mode: butun ekran har kadrda qaytadan quriladi,
    /// shuning uchun kadrni oynasiz o'tkazish haqiqiy chizishning o'zi —
    /// jadval ustunlari, identifikatorlar va hisoblar shu yerda ishlaydi.
    fn frame(ctx: &egui::Context, app: &mut App) {
        let _ = ctx.run(egui::RawInput::default(), |ctx| draw(ctx, app));
    }

    /// Har bir ekran namuna ma'lumotida chizilishi kerak.
    ///
    /// Bu sinov «ishlaydi shekilli» degan taxminni almashtiradi: ekran
    /// haqiqatda chiziladi va uning ichidagi hisoblar bajariladi.
    #[test]
    fn every_screen_draws_with_demo_data() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        app.auto_check(crate::domain::IssueModule::Project);

        let ctx = egui::Context::default();
        // Birinchi kadr uslubni o'rnatadi.
        frame(&ctx, &mut app);

        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                // Ekranning har bir tabi ham chiziladi: tab tanlovi
                // `ui.data` da saqlanadi, shuning uchun uni sinovda
                // to'g'ridan-to'g'ri qo'yamiz.
                for tab in 0u8..TABS {
                    for key in TAB_KEYS {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new(*key), tab));
                    }
                    // Ikki kadr: birinchisida holat yoziladi, ikkinchisida
                    // o'sha holat o'qiladi.
                    frame(&ctx, &mut app);
                    frame(&ctx, &mut app);
                }
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Bo'sh bazada ham hech qaysi ekran yiqilmaydi.
    ///
    /// Namuna ma'lumotisiz ilova birinchi marta shunday ochiladi: har bir
    /// ekran «ma'lumot yo'q» holatini o'zi ko'rsatishi kerak.
    #[test]
    fn every_screen_draws_on_an_empty_database() {
        let (path, db) = temp_db();
        drop(db);
        let mut app = App::new(Db::open(&path).expect("baza"));

        let ctx = egui::Context::default();
        frame(&ctx, &mut app);
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                for tab in 0u8..TABS {
                    for key in TAB_KEYS {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new(*key), tab));
                    }
                    frame(&ctx, &mut app);
                    frame(&ctx, &mut app);
                }
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Kichik oynada ham hech qaysi ekran yiqilmaydi.
    ///
    /// Tor oynada `available_width() - N` manfiy bo'lib qolishi mumkin —
    /// egui bunday o'lchamda darhol to'xtaydi. Shuning uchun tor oyna
    /// alohida sinaladi.
    #[test]
    fn every_screen_draws_in_a_small_window() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);

        for (w, h) in [(1024.0_f32, 700.0_f32), (760.0, 560.0), (420.0, 340.0)] {
            let ctx = egui::Context::default();
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(w, h),
                )),
                ..Default::default()
            };
            for (_, screens) in NAV_GROUPS {
                for s in *screens {
                    app.screen = *s;
                    // Har bo'lim ham chiziladi: tor oynada jadvallar
                    // ikkilamchi ustunlarni yashiradi va bu tarmoq ham
                    // sinovdan o'tishi kerak.
                    for tab in 0u8..TABS {
                        for key in TAB_KEYS {
                            ctx.data_mut(|d| d.insert_temp(egui::Id::new(*key), tab));
                        }
                        let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
                        let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
                    }
                }
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Diagnostika: ekranlar joyni qanchalik to'ldiradi.
    ///
    /// Sinov emas — o'lchov: `cargo test -- --ignored screen_fill
    /// --nocapture`. Har ekran uchun chizilgan tarkibning eni va bo'yi
    /// mavjud joyga nisbatan foizda beriladi. Bo'sh joy ko'p bo'lsa,
    /// ekranda joylashuv noto'g'ri; eni oshib ketsa — gorizontal surish
    /// kerak bo'ladi va bu ham noqulaylik.
    #[test]
    #[ignore = "diagnostika: ekran to'ldirilishini o'lchaydi"]
    fn screen_fill() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        app.auto_check(crate::domain::IssueModule::Project);

        // Odatdagi noutbuk ekrani.
        let (w, h) = (1366.0_f32, 768.0_f32);
        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(w, h),
            )),
            ..Default::default()
        };

        println!("{:<28} {:>6} {:>6}  izoh", "ekran", "en %", "bo'y %");
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
                let out = ctx.run(input(), |ctx| draw(ctx, &mut app));

                // Markaziy panel taxminan yon panel va yuqori paneldan keyin.
                let panel = egui::Rect::from_min_max(egui::pos2(280.0, 60.0), egui::pos2(w, h));
                // Faqat **matn** hisobga olinadi: fon to'rtburchagi doim
                // butun panelni qoplaydi va u joy to'lganini bildirmaydi.
                let mut used = egui::Rect::NOTHING;
                let mut overflow = 0.0_f32;
                let mut widest = String::new();
                for shape in &out.shapes {
                    let egui::Shape::Text(text) = &shape.shape else {
                        continue;
                    };
                    // Faqat **ko'rinadigan** qism hisobga olinadi: surish
                    // maydonidagi matn kesilib turadi va u ekrandan
                    // chiqib ketgan deb sanalmasligi kerak.
                    let r = text.visual_bounding_rect().intersect(shape.clip_rect);
                    if !r.is_positive() || r.max.x < panel.min.x || r.max.y < panel.min.y {
                        continue;
                    }
                    if r.max.x - panel.max.x > overflow {
                        overflow = r.max.x - panel.max.x;
                        widest = text.galley.text().chars().take(60).collect();
                    }
                    used = used.union(r.intersect(panel));
                }
                let (fw, fh) = if used.is_positive() {
                    (
                        used.width() / panel.width() * 100.0,
                        used.height() / panel.height() * 100.0,
                    )
                } else {
                    (0.0, 0.0)
                };
                let note = if overflow > 1.0 {
                    format!("eniga sig'maydi: +{overflow:.0} px · «{widest}»")
                } else if fw < 70.0 {
                    "o'ng tomon bo'sh".to_string()
                } else if fh < 60.0 {
                    "pasti bo'sh".to_string()
                } else {
                    String::new()
                };
                println!("{:<28} {fw:>6.0} {fh:>6.0}  {note}", s.label());
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Diagnostika: har bir ekranning kadr vaqti.
    ///
    /// Sinov emas — o'lchov: `cargo test -- --ignored screen_timing
    /// --nocapture`. Sekin ekranni topish uchun ishlatiladi; chegara
    /// qo'yilmaydi, chunki u mashinaga bog'liq.
    #[test]
    #[ignore = "diagnostika: kadr vaqtini o'lchaydi"]
    fn screen_timing() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        app.auto_check(crate::domain::IssueModule::Project);

        let ctx = egui::Context::default();
        frame(&ctx, &mut app);

        let mut rows: Vec<(String, f64)> = Vec::new();
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                // Birinchi kadr qizdiradi, keyingi beshtasi o'lchanadi.
                frame(&ctx, &mut app);
                let t0 = std::time::Instant::now();
                for _ in 0..5 {
                    frame(&ctx, &mut app);
                }
                rows.push((s.label().to_string(), t0.elapsed().as_secs_f64() * 200.0));
            }
        }
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (name, ms) in &rows {
            println!("{ms:8.2} ms · {name}");
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Har ekran o'zini tanishtiradi: nomi bor, izohi bor va ular
    /// bir-birini takrorlamaydi.
    #[test]
    fn every_screen_explains_itself() {
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                let label = s.label();
                let purpose = s.purpose();
                assert!(!label.is_empty(), "{s:?} nomsiz");
                assert_ne!(purpose, "?", "{s:?} izohi tarjimasiz");
                assert!(purpose.len() > 30, "{s:?} izohi juda qisqa: {purpose}");
                assert_ne!(purpose, label, "{s:?} izohi nomning takrori");
                // Izoh gap bo'lishi kerak, sarlavha emas.
                assert!(purpose.ends_with('.'), "{s:?} izohi gap emas: {purpose}");
            }
        }
    }

    /// AI tekshiruv ekranida hech qaysi bo'lim ko'rinishdan tushib
    /// qolmaydi.
    ///
    /// Bu ekran raqam emas, sanaladigan tur ishlatadi. Qo'lda yozilgan
    /// ikkinchi ro'yxat xavfli bo'lardi: yangi bo'lim qo'shilib, unga
    /// yo'l qolmay ketishi mumkin. Shuning uchun ro'yxat `CheckTab::ALL`
    /// dan hisoblab olinadi va sinov shu tuzilishni qo'riqlaydi.
    #[test]
    fn check_tabs_are_derived_from_the_full_list() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/aicheck.rs");
        let src = std::fs::read_to_string(&path).expect("aicheck.rs");
        assert!(
            src.contains("CheckTab::ALL"),
            "bo'limlar to'liq ro'yxatdan olinmayapti"
        );
        // Har bo'lim uchun chizish yo'li bor: `match` da hammasi
        // qamralgan bo'lishi kerak.
        for tab in crate::app::CheckTab::ALL {
            assert!(
                src.contains(&format!("CheckTab::{tab:?} =>")),
                "{tab:?} uchun chizish yo'li yo'q"
            );
        }
        // Ro'yxat bo'sh emas va takrorlanmaydi.
        let all = crate::app::CheckTab::ALL;
        for (i, a) in all.iter().enumerate() {
            assert!(!all[i + 1..].contains(a), "{a:?} ro'yxatda ikki marta");
        }
    }

    /// Kalkulyatsiya sahifasi hisob bilan ham, hisobsiz ham chiziladi.
    ///
    /// Uchala bo'lim haqiqatan chiziladi: jadvallar, konstruksiyalar va
    /// narxli yig'ma.
    #[test]
    fn the_takeoff_screen_draws_with_and_without_data() {
        use crate::takeoff::{SpecRow, SpecTable, Takeoff};
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        app.screen = Screen::AiCheck;
        let ctx = egui::Context::default();
        frame(&ctx, &mut app);

        // Hisobsiz: uchala bo'lim ham yiqilmaydi.
        for tab in crate::app::CheckTab::ALL {
            app.check_tab = tab;
            frame(&ctx, &mut app);
        }

        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
            ..Default::default()
        };
        let tables = vec![
            SpecTable {
                page: 10,
                rows: vec![
                    row("Фм3", "Фундамент монолитный Фм3", "4", "", "шт."),
                    row("К3", "2К138-6М3-c-а", "12", "", ""),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            SpecTable {
                page: 13,
                rows: vec![
                    row("", "Фундамент Фм3", "1", "", "шт."),
                    row("1", "∅14 A-III L=2550", "22", "3.09", "68.0"),
                    row("", "Бетон кл. В20(М250)W8", "", "", "3.66"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            // Takror: sanalmaydi, lekin ekranda aytiladi.
            SpecTable {
                page: 14,
                rows: vec![
                    row("", "Фундамент Фм3", "1", "", "шт."),
                    row("1", "∅14 A-III L=2550", "22", "3.09", "68.0"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
        ];
        let constructs = crate::takeoff::constructs(&tables);
        app.takeoff = Some(Takeoff {
            file: "loyiha.pdf".into(),
            pages: 77,
            tables,
            constructs,
            ..Default::default()
        });
        app.recompute_takeoff();
        app.set_takeoff_price("Beton B20", 650_000.0);
        assert_eq!(app.takeoff_repeats.len(), 1);
        assert!(!app.cost_rows.is_empty());

        // Orientir narx: beton qo'lda narxlangan, armaturaga narx
        // kiritilmagan — Ø14 uchun orientir chiqadi va belgilanadi.
        let rebar = app
            .cost_rows
            .iter()
            .find(|r| r.total.material.starts_with("Armatura"))
            .expect("armatura");
        assert!(rebar.orientir.is_some() && rebar.price == Some(8_200.0));
        let beton = app
            .cost_rows
            .iter()
            .find(|r| r.total.material == "Beton B20")
            .expect("beton");
        assert!(beton.manual && beton.orientir.is_none());
        // Smeta namunasi: olti bosqich ham ma'lumot bilan chiziladi.
        {
            use crate::smeta::*;
            let mut m = Smeta {
                summary: "Bir qavatli sex.".into(),
                questions: vec![Question {
                    topic: "Tom".into(),
                    text: "Tom turi?".into(),
                    options: vec!["Sendvich".into(), "Profnastil".into()],
                    answer: "Sendvich".into(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            m.digest.push(PageDigest {
                page: 1,
                sheet: "Reja".into(),
                kind: "АР".into(),
                facts: vec![Fact {
                    name: "Maydon".into(),
                    value: "2940".into(),
                    unit: "m2".into(),
                    page: Some(1),
                }],
                lists: Vec::new(),
            });
            m.facts = m.digest[0].facts.clone();
            m.stages.push(Stage {
                name: "Poydevor".into(),
                markup: 10.0,
                works: vec![Work {
                    name: "Beton quyish".into(),
                    qty: 14.64,
                    unit: "m3".into(),
                    source: Source::Project { page: Some(13) },
                    price: Some(150_000.0),
                    materials: vec![Resource {
                        name: "Beton B20".into(),
                        qty: 14.93,
                        unit: "m3".into(),
                        source: Source::Standard {
                            note: "1.02".into(),
                        },
                        price: None,
                    }],
                }],
            });
            app.smeta = Some(m);
            app.recompute_smeta();
            assert!(app.smeta_view.total > 0.0);
            assert_eq!(app.smeta_view.missing, 1);
            // Katalogga narx: keyingi hisobda o'sha narx chiqadi.
            app.set_price_scope(true);
            app.set_price(0, 0, Some(0), 650_000.0);
            assert_eq!(app.smeta_view.missing, 0);
            assert_eq!(app.catalog.materials.len(), 1);
            app.set_price_scope(false);
            // Katalogdagi o'xshash nom ham topiladi.
            app.edit_smeta(|m| m.stages[0].works[0].materials[0].name = "Beton klassi B20".into());
            assert_eq!(app.smeta_view.missing, 0);
            assert_eq!(
                app.smeta_view.stages[0].works[0].1[0].origin,
                crate::smeta::Origin::Catalog
            );
            // AI taxmini: jamiga kiradi, qabul qilinsa katalogga tushadi.
            app.edit_smeta(|m| m.stages[0].works[0].price = None);
            assert_eq!(app.smeta_view.missing, 1);
            app.price_hints.insert(
                crate::smeta::key("Beton quyish", "m3"),
                (120_000.0, "sinov".into()),
            );
            app.recompute_smeta();
            assert_eq!((app.smeta_view.missing, app.smeta_view.hinted), (0, 1));
            frame(&ctx, &mut app);
            // Topilmalar: miqdor tuzatish, ish qo'shish, takror olib tashlash.
            app.edit_smeta(|m| {
                m.review = vec![
                    crate::smeta::Finding {
                        kind: "qty".into(),
                        id: "s0.w0".into(),
                        text: "x".into(),
                        qty: Some(20.0),
                        unit: "m3".into(),
                        ..Default::default()
                    },
                    crate::smeta::Finding {
                        kind: "missing".into(),
                        id: "s0".into(),
                        text: "lestnitsa".into(),
                        name: "Zina".into(),
                        qty: Some(2.0),
                        unit: "dona".into(),
                        ..Default::default()
                    },
                    crate::smeta::Finding {
                        kind: "dup".into(),
                        id: "s0.w1".into(),
                        text: "dup".into(),
                        ..Default::default()
                    },
                    crate::smeta::Finding {
                        kind: "ask".into(),
                        text: "?".into(),
                        ..Default::default()
                    },
                ];
            });
            frame(&ctx, &mut app);
            app.apply_finding(0);
            assert_eq!(app.smeta.as_ref().unwrap().stages[0].works[0].qty, 20.0);
            app.apply_finding(1);
            assert_eq!(app.smeta.as_ref().unwrap().stages[0].works.len(), 2);
            app.apply_finding(2);
            assert_eq!(app.smeta.as_ref().unwrap().stages[0].works.len(), 1);
            app.dismiss_finding(3);
            assert!(app.smeta.as_ref().unwrap().review.iter().all(|f| f.done));
            assert_eq!(app.accept_hints(), 1);
            assert_eq!(app.smeta_view.hinted, 0);
            assert_eq!(app.catalog.works.len(), 1);
        }
        app.smeta_open = Some(0);
        app.check_tab = crate::app::CheckTab::Smeta;
        frame(&ctx, &mut app);
        // Prays va xolst ko'rinishlari.
        app.catalog_open = true;
        frame(&ctx, &mut app);
        app.catalog_open = false;
        app.sketch_open = true;
        app.sketch_draft.points = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)];
        frame(&ctx, &mut app);
        let sk = app.sketch_draft.clone();
        app.apply_sketch(sk);
        assert!(!app.sketch_open);
        assert!(app.smeta.as_ref().unwrap().sketch.is_some());
        assert!(app
            .smeta
            .as_ref()
            .unwrap()
            .facts
            .iter()
            .any(|f| f.name.starts_with("Площадь застройки")));
        app.check_tab = crate::app::CheckTab::Offer;
        frame(&ctx, &mut app);
        app.check_tab = crate::app::CheckTab::Smeta;

        // Jadval hisobdan chiqarilsa uning materiali yig'indidan ketadi,
        // qaytarilsa — qaytadi.
        let before = app.takeoff_lines.len();
        app.set_table_off(1, true);
        assert!(app.takeoff_lines.len() < before);
        app.check_tab = crate::app::CheckTab::Upload;
        frame(&ctx, &mut app);
        app.set_table_off(1, false);
        assert_eq!(app.takeoff_lines.len(), before);

        for tab in crate::app::CheckTab::ALL {
            app.check_tab = tab;
            frame(&ctx, &mut app);
            frame(&ctx, &mut app);
        }
    }

    /// Yon panel ro'yxati erkin suriladi.
    ///
    /// Xato shunday edi: faol modul ko'rinishdan chiqishi bilan ro'yxat
    /// o'zi tepaga qaytardi va pastdagi modullarga yetib bo'lmasdi.
    /// Sinov aynan shuni tekshiradi — ro'yxat surib qo'yiladi va bir
    /// necha kadrdan keyin o'sha joyda turganiga ishonch hosil qilinadi.
    #[test]
    fn sidebar_list_stays_where_it_was_scrolled() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        // Birinchi modul tanlangan bo'lsin: u ro'yxat boshida turadi.
        app.screen = Screen::Dashboard;

        let ctx = egui::Context::default();
        // Ro'yxat sig'masligi uchun oyna past bo'lishi kerak.
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(1100.0, 420.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
        let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));

        let id: egui::Id = ctx
            .data(|d| d.get_temp(egui::Id::new("nav_scroll_id")))
            .expect("ro'yxat belgisi");
        let mut state = egui::scroll_area::State::load(&ctx, id).expect("holat");
        // Foydalanuvchi ro'yxatni pastga suradi.
        state.offset.y = 300.0;
        state.store(&ctx, id);

        // Ko'p kadr: avtomatik surish silliq bajariladi, shuning uchun
        // bitta kadrda emas, bir necha kadrda sezilarli suriladi.
        for _ in 0..40 {
            let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
        }

        let after = egui::scroll_area::State::load(&ctx, id).expect("holat");
        assert!(
            after.offset.y > 250.0,
            "ro'yxat o'zi tepaga qaytib ketdi: 300 dan {} ga",
            after.offset.y
        );
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Boshqa ekranga o'tilganda ro'yxat o'sha modulga suriladi.
    ///
    /// Tuzatish keragidan ko'p narsani o'chirib qo'ymasligi kerak:
    /// «Ochish» tugmasi bilan pastdagi modulga o'tilganda u ro'yxatda
    /// ko'rinishi shart.
    #[test]
    fn sidebar_follows_when_the_screen_changes() {
        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        app.screen = Screen::Dashboard;

        let ctx = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(1100.0, 420.0),
            )),
            ..Default::default()
        };
        for _ in 0..3 {
            let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
        }
        let id: egui::Id = ctx
            .data(|d| d.get_temp(egui::Id::new("nav_scroll_id")))
            .expect("ro'yxat belgisi");
        let start = egui::scroll_area::State::load(&ctx, id)
            .expect("holat")
            .offset
            .y;

        // Ro'yxatning oxiridagi modulga o'tamiz (dasturdagi boshqa
        // tugma orqali o'tilgandek).
        app.screen = Screen::Settings;
        for _ in 0..40 {
            let _ = ctx.run(input(), |ctx| draw(ctx, &mut app));
        }
        let after = egui::scroll_area::State::load(&ctx, id)
            .expect("holat")
            .offset
            .y;
        assert!(
            after > start + 50.0,
            "ro'yxat yangi modulga surilmadi: {start} → {after}"
        );
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Ko'rsatilmagan bo'lim qolib ketmasin.
    ///
    /// Bo'lim raqamlari `match tab` da ishlanadi, ro'yxatda esa boshqa
    /// tartibda turadi. Ro'yxatga qo'shilmagan raqam ekranda ochilmaydi:
    /// kod ishlaydi, lekin bo'limga yo'l yo'q. Shuning uchun ikkalasi
    /// manbadan o'qib solishtiriladi.
    #[test]
    fn every_handled_tab_is_reachable() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut checked = 0usize;
        for entry in std::fs::read_dir(&dir).expect("src/ui") {
            let path = entry.expect("fayl").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // Bu fayl ekran emas: yordamchi shu yerda ta'riflangan va
            // sinovning o'zi ham shu yerda turadi.
            if path.file_name().is_some_and(|n| n == "mod.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("o'qish");
            let Some(call) = src.find("super::tab_row(") else {
                continue;
            };
            // Ro'yxatdagi raqamlar qavs ichida, vergulgacha turadi.
            let list_end = src[call..]
                .find(");")
                .map(|i| call + i)
                .unwrap_or(src.len());
            let mut shown: Vec<u8> = Vec::new();
            for part in src[call..list_end].split('(').skip(1) {
                if let Some(num) = part.split(',').next() {
                    if let Ok(n) = num.trim().trim_end_matches("u8").parse::<u8>() {
                        shown.push(n);
                    }
                }
            }
            assert!(!shown.is_empty(), "{path:?}: ro'yxat bo'sh");

            // `match tab {` ichidagi raqamlar.
            let Some(m) = src.find("match tab {") else {
                continue;
            };
            let block_end = src[m..].find("\n    }").map(|i| m + i).unwrap_or(src.len());
            let mut handled: Vec<u8> = Vec::new();
            for line in src[m..block_end].lines().skip(1) {
                let line = line.trim();
                let Some((head, _)) = line.split_once("=>") else {
                    continue;
                };
                if let Ok(n) = head.trim().parse::<u8>() {
                    handled.push(n);
                }
            }

            for n in &handled {
                assert!(
                    shown.contains(n),
                    "{path:?}: {n}-bo'lim ishlanadi, lekin ro'yxatda yo'q"
                );
            }
            checked += 1;
        }
        assert!(
            checked >= 5,
            "tab_row ishlatgan ekranlar topilmadi: {checked}"
        );
    }

    /// Tab kalitlari ro'yxati kod bilan mos turishi kerak.
    ///
    /// Ro'yxat qo'lda yuritiladi, shuning uchun u eskirib qolishi mumkin:
    /// yangi tab qo'shilib, sinovga kirmay qolsa, chizilmagan ekran
    /// sezilmay ketardi. Shu sababli kalitlar manbadan o'qib solishtiriladi.
    #[test]
    fn tab_key_list_matches_the_code() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut found: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("src/ui") {
            let path = entry.expect("fayl").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("o'qish");
            for part in src.split("Id::new(\"").skip(1) {
                let Some(key) = part.split('"').next() else {
                    continue;
                };
                // Faqat tab tanlovi kalitlari: ular `_tab` bilan tugaydi.
                if key.contains("_tab") && !found.iter().any(|k| k == key) {
                    found.push(key.to_string());
                }
            }
        }
        assert!(!found.is_empty(), "manbada tab kaliti topilmadi");
        for key in &found {
            assert!(
                TAB_KEYS.contains(&key.as_str()),
                "{key} sinov ro'yxatida yo'q — yangi tab qo'shilgan bo'lsa, TAB_KEYS ga qo'shing"
            );
        }
    }

    /// Chekka ma'lumotda ham ekranlar yiqilmaydi.
    ///
    /// Nolga bo'lish, bo'sh matn, nol hajm va juda katta sonlar — bular
    /// haqiqiy bazada uchraydi. «Ma'lumot yo'q — xulosa yo'q» qoidasi
    /// shu yerda sinaladi: dastur jim qolishi kerak, yiqilmasligi.
    #[test]
    fn screens_survive_edge_case_data() {
        use crate::model::{ObjectStatus, Project, Section, Task};

        let (path, db) = temp_db();
        let today = chrono::Local::now().date_naive();
        let pid = db
            .insert_project(&Project {
                id: 0,
                name: String::new(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: ObjectStatus::Design,
                start_date: today,
                // Tugash sanasi boshlanishidan oldin — noto'g'ri kiritish.
                planned_end: today - chrono::Duration::days(30),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: String::new(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .expect("obyekt");

        // Nol hajm, nol davomiylik, nomsiz ish.
        db.insert_task(&Task {
            id: 0,
            project_id: pid,
            wbs: String::new(),
            name: String::new(),
            section: Section::None,
            responsible: String::new(),
            duration: 0,
            plan_start: today,
            fact_start: None,
            fact_end: None,
            progress: 0.0,
            pinned: false,
            volume: 0.0,
            unit: String::new(),
        })
        .expect("ish");
        // Juda katta son va 100 % dan oshgan bajarilish.
        db.insert_task(&Task {
            id: 0,
            project_id: pid,
            wbs: "1".into(),
            name: "X".into(),
            section: Section::Kj,
            responsible: String::new(),
            duration: 100_000,
            plan_start: today - chrono::Duration::days(3650),
            fact_start: Some(today),
            fact_end: None,
            progress: 250.0,
            pinned: false,
            volume: 1.0e12,
            unit: "m3".into(),
        })
        .expect("ish");

        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);

        let ctx = egui::Context::default();
        frame(&ctx, &mut app);
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                for tab in 0u8..TABS {
                    for key in TAB_KEYS {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new(*key), tab));
                    }
                    frame(&ctx, &mut app);
                }
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Ekranni faqat-o'qish rolida ochish ham xavfsiz.
    #[test]
    fn every_screen_draws_for_a_read_only_role() {
        use crate::roles::{Role, User};

        let (path, db) = temp_db();
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        let uid = app.db.insert_user(&User {
            id: 0,
            name: "Buyurtmachi".into(),
            role: Role::Client,
            note: String::new(),
        });
        app.reload_users();
        app.set_user(Some(uid));

        let ctx = egui::Context::default();
        frame(&ctx, &mut app);
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                app.screen = *s;
                frame(&ctx, &mut app);
            }
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }
}
