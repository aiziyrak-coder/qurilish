//! «AI analitika» ekrani (TZ XVII).
//!
//! Barcha modullar bo'yicha bitta ko'rinish: yo'nalishlar kesimidagi
//! ko'rsatkichlar va e'tibor talab qiladigan topilmalar ro'yxati. Topilmalar
//! `analytics` modulida hisoblanadi — ekran faqat ko'rsatadi, o'zi xulosa
//! chiqarmaydi.

use super::*;
use crate::analytics::{self, Area, Finding};
use crate::domain::Severity;
use egui::{vec2, Sense};

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

    // Hisoblarni bir marta bajaramiz — ekran bo'ylab bir xil sonlar ishlatiladi.
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, &sales);
    let metrics = analytics::metrics(&inp);
    let found = analytics::findings(&inp);
    let score = analytics::health(&found);

    // Yo'nalish filtri.
    let filter_key = egui::Id::new("an_filter");
    let mut filter = ui.data(|d| d.get_temp::<Option<Area>>(filter_key)).flatten();

    let mut go: Option<Screen> = None;

    let mut export = false;
    ui.horizontal(|ui| {
        if ui
            .button(t("an_export"))
            .on_hover_text(t("an_export_hint"))
            .clicked()
        {
            export = true;
        }
        ui.label(
            RichText::new(t("analytics_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    header(ui, score, &found);
    ui.add_space(10.0);

    // Ko'rsatkichlar. `horizontal_wrapped` kartochka kengligini oldindan
    // bilmaydi, shuning uchun qatorga nechtasi sig'ishini o'zimiz hisoblaymiz.
    const CARD: f32 = 212.0;
    let per_row = ((ui.available_width() / CARD).floor() as usize).max(1);
    for chunk in metrics.chunks(per_row) {
        ui.horizontal(|ui| {
            for m in chunk {
                let color = severity_color(m.severity);
                // Kartochka bosilsa — o'sha yo'nalish ekraniga o'tamiz.
                if stat_card_link(ui, &m.title, m.value.clone(), &m.hint, color) {
                    go = Some(m.area.screen());
                }
            }
        });
        ui.add_space(6.0);
    }
    ui.add_space(12.0);

    // Yo'nalish bo'yicha filtr tugmalari.
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(filter.is_none(), t("an_all")).clicked() {
            filter = None;
        }
        for area in Area::ALL {
            let n = found.iter().filter(|f| f.area == *area).count();
            let text = if n > 0 {
                format!("{} · {n}", area.label())
            } else {
                area.label().to_string()
            };
            let color = if n == 0 {
                theme::muted()
            } else {
                theme::text()
            };
            if ui
                .selectable_label(filter == Some(*area), RichText::new(text).color(color))
                .clicked()
            {
                filter = if filter == Some(*area) { None } else { Some(*area) };
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(filter_key, filter));
    ui.add_space(8.0);

    let list: Vec<&Finding> = found
        .iter()
        .filter(|f| filter.is_none_or(|a| f.area == a))
        .collect();

    if list.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("an_nothing"))
                    .color(theme::ok())
                    .size(15.0),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("an_nothing_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for f in &list {
                    if card(ui, f) {
                        go = Some(f.screen);
                    }
                    ui.add_space(6.0);
                }
            });
    }

    if export {
        // Hisobot ekrandagi bilan bir xil hisobdan chiqadi.
        let name = app
            .project()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| t("dash").to_string());
        let text = analytics::report(&inp, &name);
        let file = format!("qurai-{}.txt", app.today.format("%Y-%m-%d"));
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t("an_export"))
            .set_file_name(&file)
            .add_filter("Text", &["txt"])
            .save_file()
        {
            match std::fs::write(&path, text) {
                Ok(()) => app.notify(format!("{} {}", t("an_export_done"), path.display())),
                Err(e) => app.notify(format!("{}: {e}", t("an_export_failed"))),
            }
        }
    }

    if let Some(screen) = go {
        app.screen = screen;
    }
}

/// Sog'lomlik indeksi va daraja bo'yicha hisob.
fn header(ui: &mut egui::Ui, score: i64, found: &[Finding]) {
    let count = |s: Severity| found.iter().filter(|f| f.severity == s).count();
    let critical = count(Severity::Critical);
    let major = count(Severity::Major);
    let warning = count(Severity::Warning);

    let color = if score >= 85 {
        theme::ok()
    } else if score >= 60 {
        theme::warn()
    } else {
        theme::danger()
    };

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Halqa ko'rinishidagi indeks.
                let (rect, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                let p = ui.painter();
                let c = rect.center();
                p.circle_stroke(c, 30.0, Stroke::new(6.0_f32, theme::track()));
                // To'liq aylana 100 ballga to'g'ri keladi.
                let frac = score as f32 / 100.0;
                arc(p, c, 30.0, frac, color);
                p.text(
                    c,
                    Align2::CENTER_CENTER,
                    score.to_string(),
                    FontId::proportional(20.0),
                    color,
                );

                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(t("an_health")).size(14.0).strong());
                    ui.label(
                        RichText::new(t("an_health_hint"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        chip(ui, theme::danger(), critical, t("sev_critical"));
                        chip(ui, theme::warn(), major, t("sev_major"));
                        chip(ui, theme::accent(), warning, t("sev_warning"));
                    });
                });
            });
        });
}

/// Halqaning to'ldirilgan qismi — 12 soatdan boshlab soat yo'nalishida.
fn arc(p: &egui::Painter, c: egui::Pos2, r: f32, frac: f32, color: Color32) {
    let steps = 64;
    let n = (steps as f32 * frac.clamp(0.0, 1.0)).round() as usize;
    if n == 0 {
        return;
    }
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::TAU * (i as f32 / steps as f32);
        points.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
    }
    p.add(egui::Shape::line(points, Stroke::new(6.0_f32, color)));
}

fn chip(ui: &mut egui::Ui, color: Color32, n: usize, label: &str) {
    let text = format!("{n} {label}");
    let color = if n == 0 { theme::muted() } else { color };
    ui.label(RichText::new(text).size(11.5).color(color));
    ui.add_space(6.0);
}

fn severity_color(s: Severity) -> Color32 {
    match s {
        Severity::Critical => theme::danger(),
        Severity::Major => theme::warn(),
        Severity::Warning => theme::accent(),
        Severity::Info => theme::muted(),
        Severity::Ok => theme::ok(),
    }
}

/// Bitta topilma kartochkasi. «O'tish» bosilsa `true`.
fn card(ui: &mut egui::Ui, f: &Finding) -> bool {
    let color = severity_color(f.severity);
    let mut open = false;

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    // Chapdagi rangli chiziq — muhimlik darajasi.
                    let (r, _) = ui.allocate_exact_size(vec2(4.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(r, 2.0, color);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(f.severity.label())
                            .size(11.0)
                            .color(color)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(f.area.label())
                            .size(11.0)
                            .color(theme::muted()),
                    );
                    ui.label(RichText::new(f.code).size(10.5).monospace().color(theme::muted()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(t("an_open")).clicked() {
                            open = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(RichText::new(&f.fact).size(13.5));
                if !f.evidence.is_empty() {
                    ui.label(
                        RichText::new(format!("{}: {}", t("an_evidence"), f.evidence))
                            .size(11.5)
                            .color(theme::muted()),
                    );
                }
                if !f.action.is_empty() {
                    ui.label(
                        RichText::new(format!("{}: {}", t("an_action"), f.action))
                            .size(11.5)
                            .color(theme::accent()),
                    );
                }
            });
        });

    open
}
