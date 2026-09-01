//! Hali ochilmagan modulning o'z sahifasi.
//!
//! Bu umumiy zaglushka emas: har bir modul o'z TZ raqami, o'z talablari
//! ro'yxati va o'z holati bilan chiqadi — nima tayyor, nimaga bog'liq va
//! ishni qayerdan boshlash kerakligi darrov ko'rinadi.

use super::*;
use crate::app::Readiness;

pub fn show(ui: &mut egui::Ui, screen: Screen) {
    let ready = screen.readiness();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let w = (ui.available_width() - 26.0).min(860.0);

            ui.add_space(18.0);
            header(ui, screen, ready);
            ui.add_space(18.0);

            let summary = screen.tz_summary();
            if !summary.is_empty() {
                ui.label(
                    RichText::new(summary)
                        .size(15.0)
                        .color(theme::text()),
                );
                ui.add_space(16.0);
            }

            let points = screen.tz_points();
            if !points.is_empty() {
                card_frame(ui, t("module_requirements"), w, |ui| {
                    for line in points.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        ui.horizontal_top(|ui| {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("—")
                                    .size(13.0)
                                    .color(theme::accent()),
                            );
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(RichText::new(line).size(13.5))
                                    .wrap(),
                            );
                        });
                        ui.add_space(3.0);
                    }
                });
                ui.add_space(12.0);
            }

            status_card(ui, screen, ready, w);
            ui.add_space(30.0);
        });
}

fn header(ui: &mut egui::Ui, screen: Screen, ready: Readiness) {
    ui.horizontal(|ui| {
        // TZ raqami — kvadrat nishon ichida.
        let numeral = screen.numeral();
        if !numeral.is_empty() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(74.0, 46.0), egui::Sense::hover());
            let p = ui.painter();
            p.rect_filled(rect, 8.0, theme::accent().gamma_multiply(0.14));
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                numeral,
                egui::FontId::monospace(21.0),
                theme::accent(),
            );
            ui.add_space(14.0);
        }
        ui.vertical(|ui| {
            ui.label(
                RichText::new(screen.label())
                    .size(26.0)
                    .strong()
                    .color(theme::text()),
            );
            ui.add_space(3.0);
            readiness_chip(ui, ready);
        });
    });
}

fn readiness_chip(ui: &mut egui::Ui, ready: Readiness) {
    let (text, color) = match ready {
        Readiness::Ready => (t("readiness_ready"), theme::ok()),
        Readiness::Storage => (t("readiness_storage"), theme::warn()),
        Readiness::Planned => (t("readiness_planned"), theme::muted()),
    };
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(12.0),
        color,
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 20.0, 22.0),
        egui::Sense::hover(),
    );
    let p = ui.painter();
    p.rect_filled(rect, 5.0, color.gamma_multiply(0.16));
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        color,
    );
}

fn status_card(ui: &mut egui::Ui, screen: Screen, ready: Readiness, w: f32) {
    let title = match ready {
        Readiness::Storage => t("module_state_storage_title"),
        _ => t("module_state_planned_title"),
    };
    card_frame(ui, title, w, |ui| {
        let body = match ready {
            Readiness::Storage => t("module_state_storage"),
            _ => t("module_state_planned"),
        };
        ui.add(egui::Label::new(RichText::new(body).size(13.5).color(theme::text())).wrap());

        let blocker = screen.blocker();
        if !blocker.is_empty() {
            ui.add_space(10.0);
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new(t("module_blocker"))
                        .size(11.0)
                        .color(theme::muted())
                        .strong(),
                );
            });
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(RichText::new(blocker).size(13.0).color(theme::warn())).wrap(),
            );
        }
    });
}
