//! «Yordamchi» ekrani (TZ XVIII).
//!
//! Savol beriladi — javob shu bazadagi ma'lumotdan hisoblanadi. Til modeli
//! ishlatilmaydi va bu ekranda ochiq aytiladi: har javob orqasida aniq son va
//! uni tekshirish mumkin bo'lgan ekran turadi.

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

    // Modeldan olingan javob shu ekranda saqlanadi.
    let reply_key = egui::Id::new("cp_reply");
    if ask_model {
        // Modelga savol va ilova hisoblab bergan sonlar birga boradi.
        let context = a
            .lines
            .iter()
            .map(|l| format!("{}: {}", l.label, l.value))
            .collect::<Vec<_>>()
            .join("\n");
        let question = if text.trim().is_empty() {
            a.title.clone()
        } else {
            text.clone()
        };
        let out = match crate::llm::ask(&app.llm, &question, &context) {
            Ok(v) => v,
            Err(e) => format!("{}: {e}", t("cp_llm_failed")),
        };
        ui.data_mut(|d| d.insert_temp(reply_key, out));
    }
    if let Some(reply) = ui.data(|d| d.get_temp::<String>(reply_key)) {
        ui.add_space(8.0);
        llm_card(ui, &reply);
    }

    if let Some(s) = go {
        app.screen = s;
    }
}

/// Modeldan kelgan javob. Alohida ramkada — bu ilova hisobi emas.
fn llm_card(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::accent()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(t("cp_llm_answer"))
                        .size(12.5)
                        .strong()
                        .color(theme::accent()),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(text).size(13.0));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("cp_llm_note"))
                        .size(10.5)
                        .color(theme::muted()),
                );
            });
        });
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
