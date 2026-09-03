//! «Yordamchi» ekrani (TZ XVIII).
//!
//! Ekranda ikki xil javob bor va ular ataylab **ajratilgan**:
//!
//! 1. «Savol-javob» — javob shu bazadagi ma'lumotdan hisoblanadi. Har javob
//!    orqasida aniq son va uni tekshirish mumkin bo'lgan ekran turadi.
//! 2. «AI suhbat» — OpenAI modeli. U son hisoblamaydi: so'rovga ilova
//!    hisoblab bergan sonlar biriktiriladi va model faqat shularni
//!    tushuntiradi. Modeldan kelgan javob boshqa ramkada turadi, chunki u
//!    ilovaning hisobi emas.

use super::*;
use crate::copilot::{self, Answer, Intent};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    if app.current.is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(16.0),
            );
        });
        return;
    }

    let text_key = egui::Id::new("cp_text");
    let intent_key = egui::Id::new("cp_intent");
    let mut text = ui
        .data(|d| d.get_temp::<String>(text_key))
        .unwrap_or_default();
    let mut intent = ui
        .data(|d| d.get_temp::<Option<Intent>>(intent_key))
        .flatten();
    // Savol yozilmagan bo'lsa — birinchi savol namuna sifatida ochiladi.
    let mut unknown = false;

    ui.horizontal(|ui| {
        ui.label(RichText::new(t("cp_title")).size(15.0).strong());
    });
    ui.label(
        RichText::new(t("cp_disclaimer"))
            .size(11.5)
            .color(theme::warn()),
    );
    ui.add_space(8.0);

    // Ikki ko'rinish: savol-javob va bajarish uchun takliflar.
    let tab_key = egui::Id::new("cp_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    let pending = crate::actions::suggest(app, &app.stock()).len();
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("cp_tab_ask").to_string()),
            (
                1,
                if pending > 0 {
                    format!("{} · {pending}", t("cp_tab_actions"))
                } else {
                    t("cp_tab_actions").to_string()
                },
            ),
            (2, t("cp_tab_chat").to_string()),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(10.0);

    if tab == 1 {
        actions_tab(ui, app);
        return;
    }
    if tab == 2 {
        chat_tab(ui, app);
        return;
    }

    // Erkin savol.
    ui.horizontal(|ui| {
        let edit = ui.add_sized(
            [ui.available_width().min(560.0), 26.0],
            egui::TextEdit::singleline(&mut text).hint_text(t("cp_placeholder")),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button(t("cp_ask")).clicked() || enter {
            match copilot::detect(&text) {
                Some(i) => intent = Some(i),
                None => {
                    intent = None;
                    unknown = !text.trim().is_empty();
                }
            }
        }
        if !text.is_empty() && ui.button(t("cp_clear")).clicked() {
            text.clear();
            intent = None;
        }
    });
    ui.data_mut(|d| d.insert_temp(text_key, text.clone()));
    ui.add_space(8.0);

    // Tayyor savollar.
    ui.label(
        RichText::new(t("cp_suggestions"))
            .size(11.5)
            .color(theme::muted()),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for i in Intent::ALL {
            if ui
                .selectable_label(intent == Some(*i), i.question())
                .clicked()
            {
                intent = Some(*i);
                text.clear();
                ui.data_mut(|d| d.insert_temp(text_key, String::new()));
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(intent_key, intent));
    ui.add_space(12.0);

    if unknown {
        egui::Frame::new()
            .fill(theme::card())
            .stroke(Stroke::new(1.0_f32, theme::warn()))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t("cp_unknown"))
                            .size(13.0)
                            .color(theme::warn()),
                    );
                    ui.label(
                        RichText::new(t("cp_unknown_hint"))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                });
            });
        return;
    }

    let Some(i) = intent else {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("cp_start"))
                    .size(14.0)
                    .color(theme::muted()),
            );
        });
        return;
    };

    // Javobni hisoblaymiz.
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, &sales);
    let a = copilot::answer(i, &inp);

    let mut go: Option<Screen> = None;
    let mut ask_model = false;
    answer_card(ui, &a, &mut go, app.llm.is_ready(), &mut ask_model);

    // «Modeldan so'rash» savolni suhbatga o'tkazadi: javob shu yerda emas,
    // alohida tabda paydo bo'ladi — ilova hisobi bilan model javobi
    // aralashib ketmasin.
    if ask_model {
        let question = if text.trim().is_empty() {
            a.title.clone()
        } else {
            text.clone()
        };
        app.ask_llm(question);
        ui.data_mut(|d| d.insert_temp(tab_key, 2u8));
    }

    if let Some(s) = go {
        app.screen = s;
    }
}

// ================================================================ AI suhbat

/// OpenAI modeli bilan suhbat (TZ XVIII).
///
/// Model son hisoblamaydi: har so'rovga ilova hisoblab bergan sonlar
/// biriktiriladi va modeldan faqat shularga tayanish so'raladi. Shuning
/// uchun javobni tegishli ekranda tekshirib ko'rish mumkin.
fn chat_tab(ui: &mut egui::Ui, app: &mut App) {
    // Fon so'rovi tugagan bo'lsa javobni olamiz.
    if app.poll_llm() {
        ui.ctx().request_repaint();
    }

    if !app.llm.is_ready() {
        let mut go_settings = false;
        egui::Frame::new()
            .fill(theme::card())
            .stroke(Stroke::new(1.0_f32, theme::warn()))
            .corner_radius(10)
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t("cp_chat_off"))
                            .size(13.5)
                            .strong()
                            .color(theme::warn()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(t("cp_chat_off_hint"))
                            .size(12.0)
                            .color(theme::muted()),
                    );
                    ui.add_space(8.0);
                    if ui.button(t("cp_chat_open_settings")).clicked() {
                        go_settings = true;
                    }
                });
            });
        if go_settings {
            app.screen = Screen::Settings;
        }
        return;
    }

    // Suhbat oynasi.
    let busy = app.llm_pending.is_some();
    let mut retry = false;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(ui.available_height() - 96.0)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if app.llm_chat.is_empty() {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(t("cp_chat_empty"))
                            .size(13.5)
                            .color(theme::muted()),
                    );
                });
            }
            for turn in &app.llm_chat {
                bubble(ui, turn);
                ui.add_space(6.0);
            }
            if busy {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(14.0));
                    ui.label(
                        RichText::new(t("cp_chat_waiting"))
                            .size(12.0)
                            .color(theme::muted()),
                    );
                });
                // Javob kelganini sezish uchun ekran yangilanib tursin.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
            }
            if let Some((key, detail, can_retry)) = app.llm_error.clone() {
                ui.add_space(6.0);
                if error_card(ui, key, &detail, can_retry) {
                    retry = true;
                }
            }
        });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    // Savol yozish qatori.
    let key = egui::Id::new("cp_chat_text");
    let mut text = ui.data(|d| d.get_temp::<String>(key)).unwrap_or_default();
    let mut send = false;
    let mut clear = false;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 210.0).max(200.0);
        let edit = ui.add_sized(
            [width, 28.0],
            egui::TextEdit::singleline(&mut text).hint_text(t("cp_chat_placeholder")),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui
            .add_enabled(
                !busy && !text.trim().is_empty(),
                egui::Button::new(t("cp_ask")),
            )
            .clicked()
            || (enter && !busy)
        {
            send = true;
        }
        if !app.llm_chat.is_empty() && ui.button(t("cp_chat_clear")).clicked() {
            clear = true;
        }
    });

    // Sarflangan tokenlar — xarajat ko'rinib tursin.
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} · {}", app.llm.model, t("cp_chat_note")))
                .size(10.5)
                .color(theme::muted()),
        );
        if app.llm_tokens > 0 {
            ui.label(
                RichText::new(format!("{} {}", app.llm_tokens, t("cp_chat_tokens")))
                    .size(10.5)
                    .color(theme::muted()),
            );
        }
    });

    if send {
        app.ask_llm(text.clone());
        text.clear();
    }
    if clear {
        app.clear_llm_chat();
    }
    // Qayta urinish: oxirgi savol tarixda turibdi, uni qaytadan yozish shart emas.
    if retry {
        if let Some(last) = app
            .llm_chat
            .iter()
            .rev()
            .find(|x| x.from_user)
            .map(|x| x.text.clone())
        {
            // Javobsiz qolgan savol ikki marta tarixga tushmasin.
            if app.llm_chat.last().map(|x| x.from_user).unwrap_or(false) {
                app.llm_chat.pop();
            }
            app.ask_llm(last);
        }
    }
    ui.data_mut(|d| d.insert_temp(key, text));
}

/// Suhbatdagi bitta gap.
fn bubble(ui: &mut egui::Ui, turn: &crate::llm::Turn) {
    let (title, colour) = if turn.from_user {
        (t("cp_chat_you"), theme::muted())
    } else {
        (t("cp_llm_answer"), theme::accent())
    };
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(
            1.0_f32,
            if turn.from_user {
                theme::line()
            } else {
                theme::accent()
            },
        ))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(11.5).strong().color(colour));
                ui.add_space(3.0);
                ui.label(RichText::new(&turn.text).size(13.0));
            });
        });
}

/// Xatoni sababi bilan ko'rsatadi — nima bo'lgani yashirilmaydi.
///
/// Qaytadan urinish ma'noli bo'lgan xatolarda (limit, tarmoq, xizmat) tugma
/// chiqadi; kalit noto'g'ri bo'lsa qaytadan urinishdan foyda yo'q.
fn error_card(ui: &mut egui::Ui, key: &str, detail: &str, can_retry: bool) -> bool {
    let mut retry = false;
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::danger()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(t(key))
                        .size(12.5)
                        .strong()
                        .color(theme::danger()),
                );
                if !detail.trim().is_empty() {
                    ui.label(RichText::new(detail).size(11.0).color(theme::muted()));
                }
                if can_retry {
                    ui.add_space(4.0);
                    if ui.small_button(t("cp_chat_retry")).clicked() {
                        retry = true;
                    }
                }
            });
        });
    retry
}

// ================================================================ Takliflar

/// Bajarish uchun takliflar (TZ XVIII.12–30, 44).
///
/// Har bir taklif — bajarilmagan qoralama: nima qilinishi va qaysi sondan
/// chiqqani yozilgan. Bajarish uchun ikki bosqich: «Bajarish» bosiladi,
/// keyin tasdiqlanadi. Bir bosishda yozuv yaratilib qolmasin.
fn actions_tab(ui: &mut egui::Ui, app: &mut App) {
    let list = crate::actions::suggest(app, &app.stock());

    ui.label(
        RichText::new(t("cp_actions_hint"))
            .size(11.5)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    if list.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("cp_actions_empty"))
                    .size(15.0)
                    .color(theme::ok()),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("cp_actions_empty_hint"))
                    .size(12.0)
                    .color(theme::muted()),
            );
        });
        return;
    }

    // Tasdiq kutayotgan taklif — kod bo'yicha eslab qolinadi.
    let confirm_key = egui::Id::new("cp_confirm");
    let mut confirming = ui.data(|d| d.get_temp::<String>(confirm_key));
    let mut run: Option<usize> = None;
    let mut goto: Option<Screen> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, a) in list.iter().enumerate() {
                let id = format!("{}-{i}", a.code);
                let can = app.can_edit(a.screen);
                egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width() - 4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(a.code)
                                    .size(10.5)
                                    .monospace()
                                    .color(theme::muted()),
                            );
                            ui.label(RichText::new(&a.title).size(13.5).strong());
                        });
                        ui.label(RichText::new(&a.evidence).size(11.5).color(theme::muted()));
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if confirming.as_deref() == Some(id.as_str()) {
                                // Tasdiq bosqichi: nima bo'lishi yana bir bor aytiladi.
                                ui.label(
                                    RichText::new(t("cp_confirm_q"))
                                        .size(12.0)
                                        .color(theme::warn()),
                                );
                                if ui
                                    .button(RichText::new(t("cp_confirm_yes")).strong())
                                    .clicked()
                                {
                                    run = Some(i);
                                    confirming = None;
                                }
                                if ui.button(t("cancel")).clicked() {
                                    confirming = None;
                                }
                            } else {
                                if ui
                                    .add_enabled(can, egui::Button::new(t("cp_run")))
                                    .on_disabled_hover_text(format!(
                                        "{} — {}",
                                        t("role_readonly"),
                                        app.role().label()
                                    ))
                                    .clicked()
                                {
                                    confirming = Some(id.clone());
                                }
                                if ui.small_button(t("cp_open")).clicked() {
                                    goto = Some(a.screen);
                                }
                            }
                            ui.label(
                                RichText::new(a.screen.label())
                                    .size(11.0)
                                    .color(theme::muted()),
                            );
                        });
                    });
                ui.add_space(8.0);
            }
        });

    ui.data_mut(|d| match &confirming {
        Some(v) => d.insert_temp(confirm_key, v.clone()),
        None => d.remove::<String>(confirm_key),
    });

    if let Some(i) = run {
        match crate::actions::perform(app, &list[i]) {
            Ok(msg) => app.notify(msg),
            Err(e) => app.notify(e),
        }
    }
    if let Some(screen) = goto {
        app.screen = screen;
    }
}

fn answer_card(
    ui: &mut egui::Ui,
    a: &Answer,
    go: &mut Option<Screen>,
    llm_ready: bool,
    ask_model: &mut bool,
) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&a.title).size(14.5).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(t("cp_check")).clicked() {
                            *go = Some(a.screen);
                        }
                        // Til modeli yoqilgan bo'lsagina ko'rinadi.
                        if llm_ready
                            && ui
                                .small_button(t("cp_ask_model"))
                                .on_hover_text(t("cp_ask_model_hint"))
                                .clicked()
                        {
                            *ask_model = true;
                        }
                        // Javob qaysi bo'limdan olingani — tekshirish uchun.
                        ui.label(
                            RichText::new(a.intent.screen().label())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    });
                });
                ui.add_space(8.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(460.0)
                    .show(ui, |ui| {
                        for l in &a.lines {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [280.0, 20.0],
                                    egui::Label::new(
                                        RichText::new(&l.label).size(12.5).color(theme::muted()),
                                    ),
                                );
                                ui.label(RichText::new(&l.value).size(13.5).color(if l.alert {
                                    theme::danger()
                                } else {
                                    theme::text()
                                }));
                            });
                        }
                    });

                ui.add_space(8.0);
                ui.label(RichText::new(&a.note).size(10.5).color(theme::muted()));
            });
        });
}
