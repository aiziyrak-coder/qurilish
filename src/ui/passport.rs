//! «Obyekt pasporti» (TZ I.1) — obyekt haqidagi hamma narsa bitta ekranda.
//!
//! Yuqorida bosqichlar chizig'i (loyihalash → tender → qurilish → topshirilgan),
//! chapda obyekt kartasi, muddatlar va moliyalashtirish, o'ngda obyekt holati
//! va pasport to'ldirilishi nazorati. Pastda ishtirokchilar (TZ dagi olti rol
//! bo'yicha guruhlangan), hujjatlar, foto va izohlar.

use super::*;
use crate::domain::IssueStatus;
use crate::model::{ObjectStatus, Party, PartyRole};
use egui::{vec2, Sense};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let Some(mut p) = app.project().cloned() else {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(18.0),
            );
        });
        return;
    };

    let mut dirty = false;
    let mut ask_delete = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ---- Sarlavha ----
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&p.name)
                    .size(22.0)
                    .strong()
                    .color(theme::text()),
            );
            code_chip(ui, &p.code);
        });
        if !p.address.is_empty() {
            ui.label(RichText::new(&p.address).size(12.5).color(theme::muted()));
        }
        ui.add_space(10.0);

        // ---- Bosqichlar chizig'i ----
        dirty |= status_pipeline(ui, &mut p);
        ui.add_space(14.0);

        let avail = (ui.available_width() - 24.0).max(120.0);
        // Tor oynada ikki ustun siqilib, o'ng ustun kesilib qolardi —
        // shunda bloklar ustma-ust joylashadi.
        let two_col = avail > 900.0;
        let right_w = if two_col {
            (avail * 0.30).clamp(280.0, 380.0)
        } else {
            avail
        };
        let left_w = if two_col {
            (avail - right_w - 24.0).max(360.0)
        } else {
            avail
        };

        let columns = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if two_col {
                ui.horizontal_top(|ui| add(ui));
            } else {
                add(ui);
            }
        };

        columns(ui, &mut |ui| {
            // ---- Chap ustun: karta, muddatlar, moliya ----
            ui.vertical(|ui| {
                ui.set_width(left_w);
                card_frame(ui, t("card_object"), left_w - 36.0, |ui| {
                    object_fields(ui, &mut p, &mut dirty);
                });
                ui.add_space(10.0);
                card_frame(ui, t("card_dates"), left_w - 36.0, |ui| {
                    dates_fields(ui, app, &mut p, &mut dirty);
                });
                ui.add_space(10.0);
                card_frame(ui, t("card_finance"), left_w - 36.0, |ui| {
                    finance_fields(ui, &mut p, &mut dirty);
                });
                ui.add_space(10.0);
                card_frame(ui, t("geo_fence"), left_w - 36.0, |ui| {
                    geofence_card(ui, app);
                });
            });

            // ---- O'ng ustun: holat va to'ldirilish ----
            ui.vertical(|ui| {
                ui.set_width(right_w);
                card_frame(ui, t("passport_summary"), right_w - 36.0, |ui| {
                    summary_card(ui, app);
                });
                ui.add_space(10.0);
                card_frame(ui, t("passport_completeness"), right_w - 36.0, |ui| {
                    completeness_card(ui, app, &p);
                });
            });
        });

        ui.add_space(10.0);
        parties_card(ui, app, avail);

        ui.add_space(10.0);
        super::documents::card(ui, app, avail);

        ui.add_space(10.0);
        card_frame(ui, t("card_notes"), avail, |ui| {
            dirty |= ui
                .add(
                    egui::TextEdit::multiline(&mut p.notes)
                        .desired_width((ui.available_width() - 20.0).max(80.0))
                        .desired_rows(3),
                )
                .changed();
        });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(t("delete_object")).color(theme::danger()))
                .clicked()
            {
                ask_delete = true;
            }
            ui.label(
                RichText::new(t("delete_object_note"))
                    .size(11.0)
                    .color(theme::muted()),
            );
        });
        ui.add_space(24.0);
    });

    if dirty {
        if let Err(e) = app.db.update_project(&p) {
            app.notify(format!("{}: {e}", t("save_failed")));
        } else if let Some(slot) = app.projects.iter_mut().find(|x| x.id == p.id) {
            *slot = p.clone();
            app.recompute();
        }
    }

    if ask_delete {
        app.confirm_delete_project = Some(p.id);
    }
}

// ================================================================ Sarlavha

fn code_chip(ui: &mut egui::Ui, code: &str) {
    if code.trim().is_empty() {
        return;
    }
    let galley = ui.painter().layout_no_wrap(
        code.to_string(),
        egui::FontId::monospace(12.0),
        theme::accent(),
    );
    let (rect, _) = ui.allocate_exact_size(vec2(galley.size().x + 16.0, 22.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 5.0, theme::accent().gamma_multiply(0.14));
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        code,
        egui::FontId::monospace(12.0),
        theme::accent(),
    );
}

// ================================================================ Bosqichlar

/// Obyekt bosqichlari: loyihalash → tender → qurilish → topshirilgan.
/// Bosqichga bosilsa, holat o'zgaradi. «To'xtatilgan» alohida tugma —
/// u bosqich emas, favqulodda holat.
fn status_pipeline(ui: &mut egui::Ui, p: &mut crate::model::Project) -> bool {
    let mut changed = false;
    let steps = [
        ObjectStatus::Design,
        ObjectStatus::Tender,
        ObjectStatus::InProgress,
        ObjectStatus::Completed,
    ];
    let suspended = p.status == ObjectStatus::Suspended;
    // To'xtatilganda oxirgi haqiqiy bosqich sifatida «qurilish» ko'rsatiladi.
    let cur = steps.iter().position(|s| *s == p.status).unwrap_or(2);

    let w = (ui.available_width() - 190.0).max(80.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(w.max(420.0), 52.0), Sense::hover());
        let painter = ui.painter();
        let n = steps.len();
        let pad = 70.0;
        let cy = rect.min.y + 16.0;
        let xs: Vec<f32> = (0..n)
            .map(|i| rect.min.x + pad + (rect.width() - 2.0 * pad) * i as f32 / (n - 1) as f32)
            .collect();

        // Bog'lovchi chiziqlar.
        for i in 0..n - 1 {
            let done = i < cur && !suspended;
            painter.line_segment(
                [pos2(xs[i] + 13.0, cy), pos2(xs[i + 1] - 13.0, cy)],
                Stroke::new(2.0_f32, if done { theme::accent() } else { theme::line() }),
            );
        }

        for (i, st) in steps.iter().enumerate() {
            let passed = i < cur && !suspended;
            let active = i == cur && !suspended;
            let (fill, ring, txt) = if passed || active {
                (theme::accent(), theme::accent(), theme::panel())
            } else {
                (theme::panel(), theme::line(), theme::muted())
            };
            painter.circle_filled(pos2(xs[i], cy), 11.0, fill);
            painter.circle_stroke(pos2(xs[i], cy), 11.0, Stroke::new(1.5_f32, ring));
            if passed {
                super::draw_check(painter, pos2(xs[i], cy), 5.0, txt, 2.0);
            } else {
                painter.text(
                    pos2(xs[i], cy),
                    Align2::CENTER_CENTER,
                    (i + 1).to_string(),
                    egui::FontId::proportional(11.5),
                    txt,
                );
            }
            painter.text(
                pos2(xs[i], cy + 22.0),
                Align2::CENTER_CENTER,
                st.label(),
                egui::FontId::proportional(11.5),
                if active {
                    theme::accent()
                } else if passed {
                    theme::text()
                } else {
                    theme::muted()
                },
            );

            // Bosish maydoni — doira va yozuvni qamrab oladi.
            let hit = Rect::from_center_size(pos2(xs[i], cy + 10.0), vec2(96.0, 52.0));
            let resp = ui.interact(hit, egui::Id::new(("stage", i)), Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.on_hover_text(t("stage_hint")).clicked() && p.status != *st {
                p.status = *st;
                changed = true;
            }
        }

        // «To'xtatilgan» — alohida holat tugmasi.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = RichText::new(ObjectStatus::Suspended.label())
                .size(12.5)
                .color(if suspended {
                    theme::panel()
                } else {
                    theme::danger()
                });
            let btn = egui::Button::new(label)
                .fill(if suspended {
                    theme::danger()
                } else {
                    theme::danger().gamma_multiply(0.12)
                })
                .corner_radius(6);
            if ui.add(btn).on_hover_text(t("suspend_hint")).clicked() {
                p.status = if suspended {
                    ObjectStatus::InProgress
                } else {
                    ObjectStatus::Suspended
                };
                changed = true;
            }
        });
    });
    changed
}

// ================================================================ Kartalar

/// Obyekt geozonasi (TZ XIII.5, XV.5).
///
/// Bu obyektning **joyi**: markaz koordinatasi va radius. Telefondan
/// kelgan yozuv va foto shu doiraga tushdimi — tekshiruv shundan
/// boshlanadi. Kiritilmagan bo'lsa joy bo'yicha hech qanday hukm
/// chiqarilmaydi va bu ochiq yoziladi.
fn geofence_card(ui: &mut egui::Ui, app: &mut App) {
    let current = app.fence();
    let mut lat = current.map(|f| f.center.lat).unwrap_or(0.0);
    let mut lon = current.map(|f| f.center.lon).unwrap_or(0.0);
    let mut radius = current.map(|f| f.radius).unwrap_or(crate::geo::MIN_RADIUS);
    let mut changed = false;

    ui.label(
        RichText::new(t("geo_fence_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(6.0);

    field(ui, t("geo_center"), |ui| {
        changed |= ui
            .add(
                egui::DragValue::new(&mut lat)
                    .speed(0.0001)
                    .range(-90.0..=90.0)
                    .max_decimals(6),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut lon)
                    .speed(0.0001)
                    .range(-180.0..=180.0)
                    .max_decimals(6),
            )
            .changed();
    });
    field(ui, t("geo_radius"), |ui| {
        changed |= ui
            .add(
                egui::DragValue::new(&mut radius)
                    .speed(5.0)
                    .range(crate::geo::MIN_RADIUS..=5_000.0),
            )
            .changed();
    });

    if changed {
        let point = crate::geo::Point::new(lat, lon);
        app.set_fence(point.valid().then_some(crate::geo::Fence {
            center: point,
            radius,
        }));
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if current.is_none() {
            ui.label(
                RichText::new(t("geo_fence_empty"))
                    .size(11.0)
                    .color(theme::muted()),
            );
        }
        // Koordinatani fotolardan taklif qilish: obyektda olingan
        // rasmlarning o'rtachasi. Bu **taklif**, o'zi qabul qilinmaydi —
        // tugma bosilgandagina yoziladi.
        if ui
            .button(t("geo_suggest"))
            .on_hover_text(t("geo_suggest_hint"))
            .clicked()
        {
            let points: Vec<crate::geo::Point> = crate::photocheck::from_journal(&app.journal)
                .iter()
                .filter_map(|s| crate::exif::read(std::path::Path::new(&s.file)))
                .filter_map(|m| m.point)
                .chain(app.journal.iter().filter_map(|e| crate::geo::parse(&e.gps)))
                .collect();
            match crate::photocheck::suggest_center(&points) {
                Some(c) => {
                    app.set_fence(Some(crate::geo::Fence {
                        center: c,
                        radius: radius.max(crate::geo::MIN_RADIUS),
                    }));
                    app.notify(format!("{} · {}", t("geo_center"), c.label()));
                }
                None => app.notify(t("geo_no_points").to_string()),
            }
        }
    });
}

fn object_fields(ui: &mut egui::Ui, p: &mut crate::model::Project, dirty: &mut bool) {
    field(ui, t("field_name"), |ui| {
        *dirty |= ui
            .add(egui::TextEdit::singleline(&mut p.name).desired_width(420.0))
            .changed();
    });
    field(ui, t("field_code"), |ui| {
        *dirty |= ui
            .add(egui::TextEdit::singleline(&mut p.code).desired_width(160.0))
            .changed();
    });
    field(ui, t("field_address"), |ui| {
        *dirty |= ui
            .add(egui::TextEdit::singleline(&mut p.address).desired_width(420.0))
            .changed();
    });
    field(ui, t("field_object_type"), |ui| {
        *dirty |= ui
            .add(
                egui::TextEdit::singleline(&mut p.object_type)
                    .desired_width(280.0)
                    .hint_text(t("object_type_hint")),
            )
            .changed();
    });
    field(ui, t("field_floors"), |ui| {
        *dirty |= ui
            .add(egui::DragValue::new(&mut p.floors).range(0..=200))
            .changed();
        ui.add_space(16.0);
        ui.label(RichText::new(t("field_area")).color(theme::muted()));
        *dirty |= ui
            .add(
                egui::DragValue::new(&mut p.area_total)
                    .range(0.0..=10_000_000.0)
                    .speed(10.0),
            )
            .changed();
        if p.area_total > 0.0 {
            ui.label(RichText::new(format!("{} m²", money(p.area_total))).color(theme::muted()));
        }
    });
}

fn dates_fields(ui: &mut egui::Ui, app: &App, p: &mut crate::model::Project, dirty: &mut bool) {
    field(ui, t("field_start"), |ui| {
        *dirty |= date_edit(ui, "start", &mut p.start_date);
    });
    field(ui, t("field_planned_end"), |ui| {
        *dirty |= date_edit(ui, "end", &mut p.planned_end);
    });
    let days = (p.planned_end - p.start_date).num_days();
    field(ui, t("field_duration"), |ui| {
        ui.label(RichText::new(format!("{days} {}", t("days"))).color(theme::text()));
        let elapsed = (app.today - p.start_date).num_days().clamp(0, days.max(1));
        ui.label(
            RichText::new(format!(
                "· {elapsed} {} · {} {}",
                t("tl_elapsed"),
                (days - elapsed).max(0),
                t("tl_left")
            ))
            .size(11.5)
            .color(theme::muted()),
        );
    });

    // Muddat chizig'i: boshlanish — bugun — tugash.
    ui.horizontal(|ui| {
        ui.add_space(210.0);
        let bar_w = (ui.available_width() - 30.0).clamp(120.0, 460.0);
        let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 8.0), Sense::hover());
        let painter = ui.painter();
        let frac = ((app.today - p.start_date).num_days() as f32
            / (p.planned_end - p.start_date).num_days().max(1) as f32)
            .clamp(0.0, 1.0);
        painter.rect_filled(rect, 4.0, theme::track());
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(rect.width() * frac, rect.height())),
            4.0,
            theme::accent().gamma_multiply(0.85),
        );
        let tx = rect.min.x + rect.width() * frac;
        painter.line_segment(
            [pos2(tx, rect.min.y - 2.0), pos2(tx, rect.max.y + 2.0)],
            Stroke::new(2.0_f32, theme::warn()),
        );
    });
    ui.add_space(4.0);

    if !app.tasks.is_empty() {
        let sched_end =
            p.start_date + chrono::Duration::days((app.schedule.project_days - 1).max(0));
        let over = sched_end > p.planned_end;
        field(ui, t("field_cpm_end"), |ui| {
            ui.label(
                RichText::new(sched_end.format("%d.%m.%Y").to_string())
                    .color(if over { theme::danger() } else { theme::ok() })
                    .strong(),
            );
            if over {
                ui.label(
                    RichText::new(format!(
                        "{} {} {}",
                        t("later_than_contract"),
                        (sched_end - p.planned_end).num_days(),
                        t("days_short")
                    ))
                    .color(theme::danger())
                    .size(12.0),
                );
            }
        });
    }
}

fn finance_fields(ui: &mut egui::Ui, p: &mut crate::model::Project, dirty: &mut bool) {
    field(ui, t("field_contract_sum"), |ui| {
        *dirty |= ui
            .add(
                egui::DragValue::new(&mut p.contract_sum)
                    .speed(1_000_000.0)
                    .range(0.0..=f64::MAX),
            )
            .changed();
        ui.label(
            RichText::new(format!("{} {}", money(p.contract_sum), p.currency))
                .color(theme::muted()),
        );
    });
    field(ui, t("field_paid"), |ui| {
        *dirty |= ui
            .add(
                egui::DragValue::new(&mut p.paid_total)
                    .speed(1_000_000.0)
                    .range(0.0..=f64::MAX),
            )
            .changed();
        ui.label(
            RichText::new(format!("{} {}", money(p.paid_total), p.currency)).color(theme::muted()),
        );
    });

    // To'lov nazorati: shartnomaning qancha qismi yopilgan.
    if p.contract_sum > 0.0 {
        let frac = (p.paid_total / p.contract_sum).clamp(0.0, 1.0) as f32;
        ui.horizontal(|ui| {
            ui.add_space(210.0);
            let bar_w = (ui.available_width() - 30.0).clamp(120.0, 460.0);
            let (rect, _) = ui.allocate_exact_size(vec2(bar_w, 10.0), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, 4.0, theme::track());
            painter.rect_filled(
                Rect::from_min_size(rect.min, vec2(rect.width() * frac, rect.height())),
                4.0,
                theme::ok(),
            );
        });
        ui.horizontal(|ui| {
            ui.add_space(210.0);
            ui.label(
                RichText::new(format!(
                    "{:.0} % · {}: {} {}",
                    frac * 100.0,
                    t("finance_remaining"),
                    money((p.contract_sum - p.paid_total).max(0.0)),
                    p.currency
                ))
                .size(11.5)
                .color(theme::muted()),
            );
        });
        ui.add_space(4.0);
    }

    field(ui, t("field_currency"), |ui| {
        *dirty |= ui
            .add(egui::TextEdit::singleline(&mut p.currency).desired_width(90.0))
            .changed();
    });
    field(ui, t("field_funding"), |ui| {
        *dirty |= ui
            .add(egui::TextEdit::singleline(&mut p.funding_source).desired_width(420.0))
            .changed();
    });
}

// ================================================================ O'ng ustun

/// Obyektning tezkor holati — boshqa modullardan yig'ilgan sonlar.
fn summary_card(ui: &mut egui::Ui, app: &App) {
    let row = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0).color(theme::muted()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(value).size(13.0).strong().color(color));
            });
        });
        ui.add_space(2.0);
    };

    let pr = &app.progress;
    let behind = pr.fact_pct + 0.5 < pr.plan_pct;
    row(
        ui,
        t("kpi_progress"),
        format!("{:.1} % / {:.1} %", pr.fact_pct, pr.plan_pct),
        if behind { theme::danger() } else { theme::ok() },
    );
    row(
        ui,
        t("sum_tasks"),
        app.tasks.len().to_string(),
        theme::text(),
    );
    row(
        ui,
        t("kpi_overdue"),
        pr.overdue.len().to_string(),
        if pr.overdue.is_empty() {
            theme::ok()
        } else {
            theme::danger()
        },
    );
    let open_issues = app
        .issues
        .iter()
        .filter(|i| i.status == IssueStatus::Open)
        .count();
    row(
        ui,
        t("sum_issues"),
        open_issues.to_string(),
        if open_issues == 0 {
            theme::ok()
        } else {
            theme::warn()
        },
    );
    row(
        ui,
        t("sum_parties"),
        app.parties.len().to_string(),
        theme::text(),
    );
    row(
        ui,
        t("sum_docs"),
        app.documents.len().to_string(),
        theme::text(),
    );
}

/// Pasport to'ldirilishi: TZ I.1 dagi majburiy bo'laklar bo'yicha nazorat.
fn completeness_card(ui: &mut egui::Ui, app: &App, p: &crate::model::Project) {
    let roles_full = PartyRole::ALL
        .iter()
        .all(|r| app.parties.iter().any(|x| x.role == *r));

    let checks: [(bool, &str); 6] = [
        (!p.address.trim().is_empty(), t("pc_address")),
        (!p.object_type.trim().is_empty(), t("pc_type")),
        (p.planned_end > p.start_date, t("pc_dates")),
        (p.contract_sum > 0.0, t("pc_sum")),
        (roles_full, t("pc_parties")),
        (!app.documents.is_empty(), t("pc_docs")),
    ];
    let done = checks.iter().filter(|(ok, _)| *ok).count();
    let pct = done as f32 / checks.len() as f32;

    // Umumiy polosa.
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            vec2((ui.available_width() - 52.0).max(60.0), 10.0),
            Sense::hover(),
        );
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, theme::track());
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(rect.width() * pct, rect.height())),
            4.0,
            if pct >= 1.0 {
                theme::ok()
            } else {
                theme::accent()
            },
        );
        ui.label(
            RichText::new(format!("{:.0} %", pct * 100.0))
                .size(12.0)
                .strong()
                .color(if pct >= 1.0 {
                    theme::ok()
                } else {
                    theme::text()
                }),
        );
    });
    ui.add_space(6.0);

    for (ok, label) in checks {
        ui.horizontal(|ui| {
            let (icon, _) = ui.allocate_exact_size(vec2(16.0, 17.0), Sense::hover());
            let painter = ui.painter();
            if ok {
                painter.circle_filled(icon.center(), 7.0, theme::ok().gamma_multiply(0.18));
                super::draw_check(painter, icon.center(), 4.0, theme::ok(), 1.8);
            } else {
                painter.circle_stroke(icon.center(), 6.0, Stroke::new(1.3_f32, theme::muted()));
            }
            ui.label(RichText::new(label).size(12.0).color(if ok {
                theme::text()
            } else {
                theme::muted()
            }));
        });
    }
    ui.add_space(4.0);
    ui.label(RichText::new(t("pc_hint")).size(10.5).color(theme::muted()));
}

// ================================================================ Ishtirokchilar

/// TZ I.1 dagi olti rol uchun rang: ro'yxat rol bo'yicha guruhlangan holda o'qiladi.
fn role_color(r: PartyRole) -> Color32 {
    let base = match r {
        PartyRole::Client => Color32::from_rgb(37, 99, 235),
        PartyRole::Contractor => Color32::from_rgb(21, 128, 71),
        PartyRole::Subcontractor => Color32::from_rgb(96, 170, 160),
        PartyRole::Designer => Color32::from_rgb(120, 110, 200),
        PartyRole::TechSupervision => Color32::from_rgb(176, 104, 0),
        PartyRole::AuthorSupervision => Color32::from_rgb(160, 140, 190),
    };
    theme::section_color(base)
}

fn parties_card(ui: &mut egui::Ui, app: &mut App, w: f32) {
    let mut edited: Option<Party> = None;
    let mut removed: Option<i64> = None;
    let mut add_role: Option<PartyRole> = None;

    card_frame(ui, t("card_parties"), w, |ui| {
        if app.parties.is_empty() {
            ui.label(RichText::new(t("parties_empty")).color(theme::muted()));
        } else {
            let head = |ui: &mut egui::Ui, cw: f32, s: &str| {
                ui.add_sized(
                    [cw, 16.0],
                    egui::Label::new(RichText::new(s).color(theme::muted()).size(11.0)),
                );
            };
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                head(ui, 176.0, t("col_role"));
                head(ui, 240.0, t("col_org"));
                head(ui, 180.0, t("col_person"));
                head(ui, 150.0, t("col_phone"));
                head(ui, 190.0, t("col_email"));
            });

            // Rol tartibida guruhlab chiqamiz: buyurtmachi doim yuqorida.
            for role in PartyRole::ALL {
                for party in app.parties.iter().filter(|x| x.role == role) {
                    let mut pt = party.clone();
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        // Rol rangli nuqtasi.
                        let (dot, _) = ui.allocate_exact_size(vec2(10.0, 20.0), Sense::hover());
                        ui.painter().circle_filled(
                            pos2(dot.center().x, dot.center().y),
                            4.0,
                            role_color(role),
                        );
                        egui::ComboBox::from_id_salt(("role", pt.id))
                            .selected_text(pt.role.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for r in PartyRole::ALL {
                                    changed |=
                                        ui.selectable_value(&mut pt.role, r, r.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut pt.name))
                            .changed();
                        changed |= ui
                            .add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut pt.person))
                            .changed();
                        changed |= ui
                            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut pt.phone))
                            .changed();
                        changed |= ui
                            .add_sized([190.0, 22.0], egui::TextEdit::singleline(&mut pt.email))
                            .changed();
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .on_hover_text(t("delete_party"))
                            .clicked()
                        {
                            removed = Some(pt.id);
                        }
                    });
                    if changed {
                        edited = Some(pt);
                    }
                }
            }
        }

        // TZ I.1: olti rol ham to'ldirilishi kerak — yetishmayotgani taklif qilinadi.
        let missing: Vec<PartyRole> = PartyRole::ALL
            .into_iter()
            .filter(|r| !app.parties.iter().any(|x| x.role == *r))
            .collect();
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if !missing.is_empty() {
                ui.label(
                    RichText::new(t("missing_roles"))
                        .size(11.5)
                        .color(theme::warn()),
                );
                for r in &missing {
                    if ui
                        .button(
                            RichText::new(format!("+ {}", r.label()))
                                .size(11.5)
                                .color(role_color(*r)),
                        )
                        .clicked()
                    {
                        add_role = Some(*r);
                    }
                }
                ui.add_space(10.0);
            }
            if ui.button(t("add_party")).clicked() {
                add_role = Some(PartyRole::Contractor);
            }
        });
    });

    if let Some(pt) = edited {
        let _ = app.db.update_party(&pt);
        if let Some(slot) = app.parties.iter_mut().find(|x| x.id == pt.id) {
            *slot = pt;
        }
    }
    if let Some(id) = removed {
        let _ = app.db.delete_party(id);
        app.parties.retain(|x| x.id != id);
    }
    if let Some(role) = add_role {
        if let Some(pid) = app.current {
            let _ = app.db.insert_party(&Party {
                id: 0,
                project_id: pid,
                role,
                name: String::new(),
                person: String::new(),
                phone: String::new(),
                email: String::new(),
            });
            app.parties = app.db.parties(pid).unwrap_or_default();
        }
    }
}

// ================================================================ Sana maydoni

/// KK.OO.YYYY ko'rinishidagi oddiy sana maydoni — tashqi kalendarsiz ishlaydi
/// va klaviaturadan kiritishni qabul qiladi.
pub fn date_edit(ui: &mut egui::Ui, id: &str, date: &mut chrono::NaiveDate) -> bool {
    let key = egui::Id::new(("date_edit", id));
    let mut buf = ui
        .memory(|m| m.data.get_temp::<String>(key))
        .unwrap_or_else(|| date.format("%d.%m.%Y").to_string());

    let resp = ui.add(egui::TextEdit::singleline(&mut buf).desired_width(110.0));
    let mut changed = false;

    if resp.changed() {
        ui.memory_mut(|m| m.data.insert_temp(key, buf.clone()));
    }
    if resp.lost_focus() || resp.ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(buf.trim(), "%d.%m.%Y") {
            if d != *date {
                *date = d;
                changed = true;
            }
        }
        ui.memory_mut(|m| m.data.remove::<String>(key));
    }
    if !resp.has_focus() {
        ui.memory_mut(|m| m.data.remove::<String>(key));
    }
    changed
}
