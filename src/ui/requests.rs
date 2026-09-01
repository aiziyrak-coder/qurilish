//! «Arizalar» ekrani (TZ IX).
//!
//! Ta'minot zanjirining birinchi bo'g'ini: ehtiyoj → ariza → tasdiqlash.
//! Har bir ariza uchun unga bog'langan xaridlar bo'yicha qoplanish
//! ko'rsatiladi, shunda «so'radim» va «keldi» orasidagi farq ko'rinib turadi.

use super::materials::{material_label, trim_num};
use super::*;
use crate::checks::SupplyLine;
use crate::domain::{Priority, Request, RequestKind, RequestStatus};

pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.label(
                RichText::new(t("no_object_selected"))
                    .color(theme::muted())
                    .size(16.0),
            );
        });
        return;
    };

    let mut add = false;
    let mut from_stock = false;
    ui.horizontal(|ui| {
        if ui.button(t("add_request")).clicked() {
            add = true;
        }
        // TZ IX: ehtiyoj arizaning manbai. Zaxiradan tushib ketgan materiallar
        // bo'yicha arizani qo'lda ko'chirib yozish shart emas.
        if ui
            .button(t("request_from_stock"))
            .on_hover_text(t("request_from_stock_hint"))
            .clicked()
        {
            from_stock = true;
        }
        ui.label(
            RichText::new(t("requests_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    let supply = app.supply();
    kpi_row(ui, app, &supply);
    ui.add_space(10.0);

    if app.requests.is_empty() {
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("requests_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        table(ui, app, &supply);
    }

    if add {
        let n = app.requests.len() + 1;
        app.db
            .insert_request(&new_request(pid, app.today, format!("Z-{n:03}")));
        app.reload_modules();
    }
    if from_stock {
        let created = create_from_stock(app, pid);
        app.reload_modules();
        app.notify(format!("{} {}", t("requests_created"), created));
    }
}

/// Bo'sh ariza namunasi — barcha joylarda bir xil bo'lishi uchun bitta joyda.
fn new_request(pid: i64, today: chrono::NaiveDate, number: String) -> Request {
    Request {
        id: 0,
        project_id: pid,
        number,
        date: today,
        kind: RequestKind::Material,
        title: String::new(),
        material_id: None,
        qty: 0.0,
        unit: String::new(),
        requester: String::new(),
        need_date: today + chrono::Duration::days(14),
        priority: Priority::Normal,
        status: RequestStatus::New,
        task_id: None,
        note: String::new(),
    }
}

/// Zaxiradan kam tushgan materiallar bo'yicha ariza yaratadi.
/// Shu material uchun yopilmagan ariza allaqachon bo'lsa — takrorlamaydi.
fn create_from_stock(app: &App, pid: i64) -> usize {
    let mut n = 0;
    let mut number = app.requests.len();
    for line in app.stock().iter().filter(|l| l.below_min) {
        let open = app.requests.iter().any(|q| {
            q.material_id == Some(line.material_id)
                && !matches!(
                    q.status,
                    RequestStatus::Closed | RequestStatus::Rejected | RequestStatus::Delivered
                )
        });
        if open {
            continue;
        }
        let Some(m) = app.materials.iter().find(|m| m.id == line.material_id) else {
            continue;
        };
        number += 1;
        let mut q = new_request(pid, app.today, format!("Z-{number:03}"));
        q.title = m.name.clone();
        q.material_id = Some(m.id);
        // Minimal zaxiraga yetkazish uchun yetishmaydigan miqdor.
        q.qty = (m.min_stock - line.balance).max(0.0);
        q.unit = m.unit.clone();
        q.priority = if line.negative {
            Priority::Urgent
        } else {
            Priority::High
        };
        app.db.insert_request(&q);
        n += 1;
    }
    n
}

fn kpi_row(ui: &mut egui::Ui, app: &App, supply: &[SupplyLine]) {
    let total = app.requests.len();
    let new = app
        .requests
        .iter()
        .filter(|q| q.status == RequestStatus::New)
        .count();
    let late = supply.iter().filter(|l| l.late).count();
    let uncovered = app
        .requests
        .iter()
        .filter(|q| {
            matches!(
                q.status,
                RequestStatus::Approved | RequestStatus::InPurchase
            ) && supply
                .iter()
                .find(|l| l.request_id == q.id)
                .is_some_and(|l| !l.covered)
        })
        .count();

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_requests"),
            total.to_string(),
            t("kpi_requests_hint"),
            theme::accent(),
        );
        stat_card(
            ui,
            t("kpi_req_new"),
            new.to_string(),
            t("kpi_req_new_hint"),
            if new == 0 { theme::ok() } else { theme::warn() },
        );
        stat_card(
            ui,
            t("kpi_req_late"),
            late.to_string(),
            t("kpi_req_late_hint"),
            if late == 0 {
                theme::ok()
            } else {
                theme::danger()
            },
        );
        stat_card(
            ui,
            t("kpi_req_uncovered"),
            uncovered.to_string(),
            t("kpi_req_uncovered_hint"),
            if uncovered == 0 {
                theme::ok()
            } else {
                theme::warn()
            },
        );
    });
}

/// Holat rangi: yakunlangan — yashil, rad etilgan — kulrang, qolgani — ish jarayonida.
fn status_color(s: RequestStatus) -> Color32 {
    match s {
        RequestStatus::New => theme::warn(),
        RequestStatus::Approved | RequestStatus::InPurchase => theme::accent(),
        RequestStatus::Delivered | RequestStatus::Closed => theme::ok(),
        RequestStatus::Rejected => theme::muted(),
    }
}

fn priority_color(p: Priority) -> Color32 {
    match p {
        Priority::Low => theme::muted(),
        Priority::Normal => theme::text(),
        Priority::High => theme::warn(),
        Priority::Urgent => theme::danger(),
    }
}

fn table(ui: &mut egui::Ui, app: &mut App, supply: &[SupplyLine]) {
    let mut edited: Option<Request> = None;
    let mut removed: Option<i64> = None;
    let today = app.today;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("requests_grid")
                .num_columns(13)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    // Tartib nazorat uchun: avval kim va qanday holatda,
                    // keyin tafsilotlar. Shunda muhim ustunlar surmasdan ko'rinadi.
                    head_l(ui, 76.0, t("col_number"));
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 124.0, t("col_status"));
                    head_l(ui, 96.0, t("col_priority"));
                    head_l(ui, 150.0, t("col_coverage"));
                    head_l(ui, 100.0, t("col_kind"));
                    head_l(ui, 180.0, t("col_item"));
                    head_l(ui, 180.0, t("col_material"));
                    head_r(ui, 80.0, t("col_qty"));
                    head_l(ui, 56.0, t("col_unit"));
                    head_l(ui, 110.0, t("col_need_date"));
                    head_l(ui, 120.0, t("col_requester"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.requests {
                        let mut q = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([76.0, 22.0], egui::TextEdit::singleline(&mut q.number))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("rq{}", q.id), &mut q.date);

                        egui::ComboBox::from_id_salt(("rq_st", q.id))
                            .selected_text(
                                RichText::new(q.status.label()).color(status_color(q.status)),
                            )
                            .width(124.0)
                            .show_ui(ui, |ui| {
                                for s in RequestStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut q.status, *s, s.label()).changed();
                                }
                            });
                        egui::ComboBox::from_id_salt(("rq_pri", q.id))
                            .selected_text(
                                RichText::new(q.priority.label()).color(priority_color(q.priority)),
                            )
                            .width(96.0)
                            .show_ui(ui, |ui| {
                                for p in Priority::ALL {
                                    changed |= ui
                                        .selectable_value(&mut q.priority, *p, p.label())
                                        .changed();
                                }
                            });
                        coverage_cell(ui, supply.iter().find(|l| l.request_id == q.id), q.qty);

                        egui::ComboBox::from_id_salt(("rq_kind", q.id))
                            .selected_text(q.kind.label())
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                for k in RequestKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut q.kind, *k, k.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut q.title))
                            .changed();

                        // Material ixtiyoriy: ish kuchi yoki hujjat arizasida bo'lmaydi,
                        // shuning uchun ro'yxatning birinchi qatori — «yo'q».
                        egui::ComboBox::from_id_salt(("rq_mat", q.id))
                            .selected_text(super::issues::truncate(
                                &match q.material_id {
                                    Some(id) => material_label(app, id),
                                    None => t("dash").to_string(),
                                },
                                24,
                            ))
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut q.material_id, None, t("dash"))
                                    .changed();
                                for m in &app.materials {
                                    changed |= ui
                                        .selectable_value(
                                            &mut q.material_id,
                                            Some(m.id),
                                            material_label(app, m.id),
                                        )
                                        .changed();
                                }
                            });

                        changed |= ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut q.qty).speed(1.0).range(0.0..=1e9),
                            )
                            .changed();
                        changed |= ui
                            .add_sized([56.0, 22.0], egui::TextEdit::singleline(&mut q.unit))
                            .changed();

                        // Kerak bo'lgan sana o'tgan bo'lsa — undov belgisi bilan.
                        ui.horizontal(|ui| {
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("rqn{}", q.id),
                                &mut q.need_date,
                            );
                            let done = matches!(
                                q.status,
                                RequestStatus::Delivered
                                    | RequestStatus::Closed
                                    | RequestStatus::Rejected
                            );
                            if !done && q.need_date < today {
                                ui.label(RichText::new("!").color(theme::danger()).strong());
                            }
                        });

                        changed |= ui
                            .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut q.requester))
                            .changed();

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(q.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(q);
                        }
                    }
                });
        });

    if let Some(q) = edited {
        app.db.update_request(&q);
        if let Some(slot) = app.requests.iter_mut().find(|x| x.id == q.id) {
            *slot = q;
        }
    }
    if let Some(id) = removed {
        app.db.del("request", id);
        app.reload_modules();
    }
}

/// Qoplanish: buyurtma qilingan miqdor arizadagi ehtiyojga nisbatan.
fn coverage_cell(ui: &mut egui::Ui, line: Option<&SupplyLine>, need: f64) {
    let Some(l) = line else {
        head_l(ui, 150.0, "");
        return;
    };
    ui.allocate_ui_with_layout(
        egui::vec2(150.0, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(150.0);
            if l.purchases == 0 {
                ui.label(
                    RichText::new(t("no_purchase"))
                        .size(11.0)
                        .color(theme::muted()),
                );
                return;
            }
            let color = if l.covered {
                theme::ok()
            } else {
                theme::warn()
            };
            ui.label(
                RichText::new(format!("{} / {}", trim_num(l.ordered), trim_num(need)))
                    .size(11.5)
                    .color(color),
            )
            .on_hover_text(format!("{} {}", t("kpi_purchase_total"), money(l.amount)));
            if l.delivered > 0.0 {
                ui.label(
                    RichText::new(format!(
                        "({} {})",
                        trim_num(l.delivered),
                        t("delivered_short")
                    ))
                    .size(10.5)
                    .color(theme::muted()),
                );
            }
            if l.planned_after_need {
                ui.label(
                    RichText::new(t("late_delivery_short"))
                        .size(10.5)
                        .color(theme::danger()),
                );
            }
        },
    );
}

/// Ariza nomini id bo'yicha — xaridlar ekrani ham ishlatadi.
pub fn request_label(app: &App, id: i64) -> String {
    app.requests
        .iter()
        .find(|q| q.id == id)
        .map(|q| {
            let title = if q.title.trim().is_empty() {
                match q.material_id {
                    Some(m) => material_label(app, m),
                    None => t("dash").to_string(),
                }
            } else {
                q.title.clone()
            };
            format!("{} · {}", q.number, title)
        })
        .unwrap_or_else(|| t("dash").to_string())
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
