//! «Bildirishnomalar» ekrani (umumiy talab, TZ VI.29, XVI.45).
//!
//! Kun boshida ochiladigan ekran: nima e'tibor talab qiladi va qayerga
//! borish kerak. Har bir qator bosilsa tegishli modul ochiladi.
//!
//! Ro'yxat saqlanmaydi — u har safar bazadan hisoblanadi. Shuning uchun
//! «o'qildi» belgisi yo'q: muammo hal bo'lsa, bildirishnoma o'zi yo'qoladi.

use super::*;
use crate::domain::Severity;
use crate::notify::Notice;

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

    let tab_key = egui::Id::new("nt_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    super::tab_row(
        ui,
        &mut tab,
        &[(0, t("nt_tab_list")), (1, t("nt_tab_chat"))],
        &[],
    );
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);
    if tab == 1 {
        chat_tab(ui, app);
        return;
    }

    let list = app.notices.clone();

    ui.horizontal(|ui| {
        ui.label(RichText::new(t("nt_hint")).size(11.0).color(theme::muted()));
    });
    ui.add_space(8.0);

    if list.is_empty() {
        ui.add_space(60.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("nt_all_clear"))
                    .color(theme::ok())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("nt_all_clear_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    // Daraja bo'yicha qisqa yakun.
    let count = |s: Severity| list.iter().filter(|n| n.severity == s).count();
    stat_row(
        ui,
        vec![
            stat(
                t("nt_kpi_total"),
                list.len().to_string(),
                t("nt_kpi_total_hint"),
                theme::text(),
            ),
            stat(
                t("sev_critical"),
                count(Severity::Critical).to_string(),
                t("nt_kpi_critical_hint"),
                if count(Severity::Critical) == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("sev_major"),
                count(Severity::Major).to_string(),
                t("nt_kpi_major_hint"),
                theme::warn(),
            ),
            stat(
                t("sev_warning"),
                count(Severity::Warning).to_string(),
                t("nt_kpi_warning_hint"),
                theme::muted(),
            ),
        ],
    );
    ui.add_space(12.0);

    let mut go: Option<Screen> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for n in &list {
                if row(ui, n) {
                    go = Some(n.screen);
                }
                ui.add_space(6.0);
            }
            ui.add_space(12.0);
        });

    if let Some(screen) = go {
        app.screen = screen;
    }
}

/// Bitta bildirishnoma kartasi. Bosilsa `true` qaytaradi.
fn row(ui: &mut egui::Ui, n: &Notice) -> bool {
    let color = severity_color(n.severity);
    let mut clicked = false;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(egui::Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Chap chekkadagi rangli chiziq — darajani bir qarashda beradi.
                let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 34.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
                ui.add_space(6.0);

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&n.title).size(13.0).strong().color(color));
                        ui.label(
                            RichText::new(n.source.label())
                                .size(10.5)
                                .color(theme::muted()),
                        );
                        if n.days > 0 {
                            ui.label(
                                RichText::new(format!("· {} {}", n.days, t("nt_days")))
                                    .size(10.5)
                                    .color(theme::danger()),
                            );
                        }
                    });
                    ui.label(RichText::new(&n.detail).size(11.5).color(theme::muted()));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new(t("nt_open")).size(11.5))
                        .on_hover_text(n.screen.label())
                        .clicked()
                    {
                        clicked = true;
                    }
                    ui.label(
                        RichText::new(n.code)
                            .size(10.0)
                            .color(theme::muted())
                            .monospace(),
                    );
                });
            });
        });

    clicked
}

fn severity_color(s: Severity) -> egui::Color32 {
    match s {
        Severity::Critical => theme::danger(),
        Severity::Major => theme::warn(),
        Severity::Warning => theme::accent(),
        Severity::Info | Severity::Ok => theme::muted(),
    }
}

/// Ofis bilan yozishma (TZ VI.32).
///
/// Yozishma **serverda** yuritiladi va shu sababli aloqa kerak. Server
/// sozlanmagan bo'lsa ekran buni ochiq aytadi va bo'sh ro'yxat
/// ko'rsatmaydi: «xabar yo'q» bilan «ulanish yo'q» bir xil ko'rinmasligi
/// kerak.
fn chat_tab(ui: &mut egui::Ui, app: &mut App) {
    let ready = app.sync.ready();

    ui.label(
        RichText::new(t("chat_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    if !ready {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("·").color(theme::warn()));
            ui.label(
                RichText::new(t("chat_needs_server"))
                    .size(12.0)
                    .color(theme::warn()),
            );
        });
        ui.add_space(6.0);
    }

    // --- Yozish maydoni.
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 120.0).max(120.0);
        ui.add(
            egui::TextEdit::multiline(&mut app.chat_draft)
                .desired_width(width)
                .desired_rows(2)
                .hint_text(t("chat_placeholder")),
        );
        ui.vertical(|ui| {
            let can = ready && !app.chat_draft.trim().is_empty();
            if ui
                .add_enabled(can, egui::Button::new(t("chat_send")))
                .on_hover_text(t("chat_send_hint"))
                .clicked()
            {
                app.send_chat();
            }
            if ui
                .button(t("chat_refresh"))
                .on_hover_text(t("chat_refresh_hint"))
                .clicked()
                && ready
            {
                app.sync_now();
            }
        });
    });
    ui.add_space(10.0);

    if app.messages.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("chat_empty"))
                    .color(theme::muted())
                    .size(14.0),
            );
        });
        return;
    }

    // Yangi xabar pastda: suhbat shunday o'qiladi.
    egui::ScrollArea::vertical()
        .id_salt("chat_list")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for m in &app.messages {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&m.author).size(12.0).strong());
                    ui.label(
                        RichText::new(crate::ui::short_stamp(&m.at))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    if !m.role.trim().is_empty() {
                        ui.label(
                            RichText::new(crate::roles::Role::parse(&m.role).label())
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    }
                });
                ui.label(RichText::new(&m.text).size(13.0));
                ui.add_space(8.0);
            }
        });
}
