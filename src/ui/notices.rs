//! «Bildirishnomalar» ekrani (umumiy talab, TZ VI.29, XVI.45).
//!
//! Kun boshida ochiladigan ekran: nima e'tibor talab qiladi va qayerga
//! borish kerak. Har bir qator bosilsa tegishli modul ochiladi.
//!
//! Ro'yxat saqlanmaydi — u har safar bazadan hisoblanadi. Shuning uchun
//! «o'qildi» belgisi yo'q: muammo hal bo'lsa, bildirishnoma o'zi yo'qoladi.

use super::*;
use crate::domain::Severity;
use crate::notify::{self, Notice};

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

    let list = notify::collect(app);

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
