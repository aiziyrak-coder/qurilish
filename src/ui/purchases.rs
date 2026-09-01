//! «Xaridlar» ekrani (TZ X).
//!
//! Zanjirning ikkinchi bo'g'ini: ariza → xarid → yetkazish. Yetkazilgan xarid
//! ombor kirimiga aylanishi kerak; shuning uchun bu yerda kirim yozilmagan
//! yetkazishlar alohida ajratib ko'rsatiladi va bir bosishda kirim yaratiladi.

use super::materials::{material_label, trim_num};
use super::requests::request_label;
use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{MoveKind, Purchase, PurchaseStatus, RequestStatus, StockMove};

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
    let mut from_requests = false;
    let mut post_all = false;
    ui.horizontal(|ui| {
        if ui.button(t("add_purchase")).clicked() {
            add = true;
        }
        if ui
            .button(t("purchase_from_requests"))
            .on_hover_text(t("purchase_from_requests_hint"))
            .clicked()
        {
            from_requests = true;
        }
        if !unposted(app).is_empty()
            && ui
                .button(RichText::new(t("post_to_stock")).color(theme::warn()))
                .on_hover_text(t("post_to_stock_hint"))
                .clicked()
        {
            post_all = true;
        }
        ui.label(
            RichText::new(t("purchases_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.purchases.is_empty() {
        ui.add_space(50.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("purchases_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        table(ui, app);
    }

    if add {
        let n = app.purchases.len() + 1;
        app.db
            .insert_purchase(&new_purchase(app, pid, format!("X-{n:03}"), None));
        app.reload_modules();
    }
    if from_requests {
        let n = create_from_requests(app, pid);
        app.reload_modules();
        app.notify(format!("{} {}", t("purchases_created"), n));
    }
    if post_all {
        let n = post_to_stock(app, pid);
        app.reload_modules();
        app.notify(format!("{} {}", t("posted_to_stock"), n));
    }
}

fn new_purchase(app: &App, pid: i64, number: String, request_id: Option<i64>) -> Purchase {
    Purchase {
        id: 0,
        project_id: pid,
        request_id,
        number,
        date: app.today,
        supplier: String::new(),
        title: String::new(),
        qty: 0.0,
        unit: String::new(),
        price: 0.0,
        currency: app.settings.default_currency.clone(),
        delivery_date: app.today + chrono::Duration::days(7),
        status: PurchaseStatus::Draft,
        note: String::new(),
    }
}

/// Tasdiqlangan, ammo hali xaridi yo'q arizalar bo'yicha xarid ochadi.
fn create_from_requests(app: &App, pid: i64) -> usize {
    let mut n = 0;
    let mut number = app.purchases.len();
    let supply = app.supply();
    for q in app.requests.iter().filter(|q| {
        matches!(
            q.status,
            RequestStatus::Approved | RequestStatus::InPurchase
        )
    }) {
        let has = supply
            .iter()
            .find(|l| l.request_id == q.id)
            .is_some_and(|l| l.purchases > 0);
        if has {
            continue;
        }
        number += 1;
        let mut p = new_purchase(app, pid, format!("X-{number:03}"), Some(q.id));
        p.title = if q.title.trim().is_empty() {
            match q.material_id {
                Some(m) => material_label(app, m),
                None => String::new(),
            }
        } else {
            q.title.clone()
        };
        p.qty = q.qty;
        p.unit = q.unit.clone();
        // Yetkazish sanasi ehtiyoj sanasidan kechikmasin.
        p.delivery_date = q.need_date;
        // Narx ma'lum bo'lsa — katalogdan olamiz, yo'q bo'lsa nol qoladi.
        if let Some(m) = q
            .material_id
            .and_then(|id| app.materials.iter().find(|m| m.id == id))
        {
            p.price = m.price;
        }
        app.db.insert_purchase(&p);
        n += 1;
    }
    n
}

/// Yetkazilgan, ammo omborga kirim qilinmagan xaridlar.
///
/// Bog'lash hujjat raqami orqali: kirim harakatining `document` maydonida
/// xarid raqami turadi. Shu sabab qayta bosilganda takror kirim bo'lmaydi.
fn unposted(app: &App) -> Vec<&Purchase> {
    app.purchases
        .iter()
        .filter(|p| {
            matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
                && p.qty > 0.0
                && material_of(app, p).is_some()
                && !app.stock_moves.iter().any(|m| m.document == p.number)
        })
        .collect()
}

/// Xarid qaysi materialga tegishli — arizadagi material orqali aniqlanadi.
fn material_of(app: &App, p: &Purchase) -> Option<i64> {
    let rid = p.request_id?;
    app.requests.iter().find(|q| q.id == rid)?.material_id
}

fn post_to_stock(app: &App, pid: i64) -> usize {
    let list: Vec<(i64, f64, f64, chrono::NaiveDate, String, String)> = unposted(app)
        .iter()
        .filter_map(|p| {
            Some((
                material_of(app, p)?,
                p.qty,
                p.price,
                p.delivery_date,
                p.number.clone(),
                p.supplier.clone(),
            ))
        })
        .collect();
    let n = list.len();
    for (material_id, qty, price, date, document, counterparty) in list {
        app.db.insert_stock_move(&StockMove {
            id: 0,
            project_id: pid,
            material_id,
            date,
            kind: MoveKind::In,
            qty,
            price,
            document,
            counterparty,
            task_id: None,
            note: String::new(),
        });
    }
    n
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let total: f64 = app.purchases.iter().map(|p| p.amount()).sum();
    let open: f64 = app
        .purchases
        .iter()
        .filter(|p| !matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed))
        .map(|p| p.amount())
        .sum();
    let late = app
        .purchases
        .iter()
        .filter(|p| {
            !matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
                && p.delivery_date < app.today
        })
        .count();
    let waiting = unposted(app).len();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_purchase_total"),
                money(total),
                t("kpi_purchase_total_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_purchase_open"),
                money(open),
                t("kpi_purchase_open_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_purchase_late"),
                late.to_string(),
                t("kpi_purchase_late_hint"),
                if late == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_purchase_unposted"),
                waiting.to_string(),
                t("kpi_purchase_unposted_hint"),
                if waiting == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
}

fn status_color(s: PurchaseStatus) -> Color32 {
    match s {
        PurchaseStatus::Draft => theme::muted(),
        PurchaseStatus::Ordered => theme::warn(),
        PurchaseStatus::Paid => theme::accent(),
        PurchaseStatus::Delivered | PurchaseStatus::Closed => theme::ok(),
    }
}

fn table(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<Purchase> = None;
    let mut removed: Option<i64> = None;
    let today = app.today;
    let waiting: Vec<i64> = unposted(app).iter().map(|p| p.id).collect();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("purchases_grid")
                .num_columns(13)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 80.0, t("col_number"));
                    head_l(ui, 110.0, t("col_date"));
                    head_l(ui, 180.0, t("col_supplier"));
                    head_l(ui, 200.0, t("col_item"));
                    head_l(ui, 190.0, t("col_request"));
                    head_r(ui, 80.0, t("col_qty"));
                    head_l(ui, 60.0, t("col_unit"));
                    head_r(ui, 110.0, t("col_price"));
                    head_r(ui, 130.0, t("col_sum"));
                    head_l(ui, 110.0, t("col_delivery"));
                    head_l(ui, 120.0, t("col_status"));
                    head_l(ui, 110.0, t("col_stock_post"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.purchases {
                        let mut p = src.clone();
                        let mut changed = false;

                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut p.number))
                            .changed();
                        changed |=
                            super::passport::date_edit(ui, &format!("pu{}", p.id), &mut p.date);
                        changed |= ui
                            .add_sized([180.0, 22.0], egui::TextEdit::singleline(&mut p.supplier))
                            .changed();
                        changed |= ui
                            .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut p.title))
                            .changed();

                        // Ariza bilan bog'lanish ixtiyoriy: to'g'ridan-to'g'ri xarid ham bo'ladi.
                        egui::ComboBox::from_id_salt(("pu_req", p.id))
                            .selected_text(super::issues::truncate(
                                &match p.request_id {
                                    Some(id) => request_label(app, id),
                                    None => t("dash").to_string(),
                                },
                                26,
                            ))
                            .width(190.0)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut p.request_id, None, t("dash"))
                                    .changed();
                                for q in &app.requests {
                                    changed |= ui
                                        .selectable_value(
                                            &mut p.request_id,
                                            Some(q.id),
                                            request_label(app, q.id),
                                        )
                                        .changed();
                                }
                            });

                        changed |= ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut p.qty).speed(1.0).range(0.0..=1e9),
                            )
                            .changed();
                        changed |= ui
                            .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut p.unit))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [110.0, 22.0],
                                egui::DragValue::new(&mut p.price)
                                    .speed(100.0)
                                    .range(0.0..=1e12),
                            )
                            .changed();
                        cell_r(ui, 130.0, RichText::new(money(p.amount())).size(12.5));

                        ui.horizontal(|ui| {
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("pud{}", p.id),
                                &mut p.delivery_date,
                            );
                            let done = matches!(
                                p.status,
                                PurchaseStatus::Delivered | PurchaseStatus::Closed
                            );
                            if !done && p.delivery_date < today {
                                ui.label(RichText::new("!").color(theme::danger()).strong());
                            }
                        });

                        egui::ComboBox::from_id_salt(("pu_st", p.id))
                            .selected_text(
                                RichText::new(p.status.label()).color(status_color(p.status)),
                            )
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for s in PurchaseStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut p.status, *s, s.label()).changed();
                                }
                            });

                        // Omborga kirim holati.
                        if waiting.contains(&p.id) {
                            cell_l(
                                ui,
                                110.0,
                                RichText::new(t("stock_pending"))
                                    .size(11.0)
                                    .color(theme::warn()),
                            );
                        } else if app.stock_moves.iter().any(|m| m.document == p.number) {
                            cell_l(
                                ui,
                                110.0,
                                RichText::new(t("stock_posted"))
                                    .size(11.0)
                                    .color(theme::ok()),
                            );
                        } else {
                            cell_l(ui, 110.0, RichText::new(t("dash")).color(theme::muted()));
                        }

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(p.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(p);
                        }
                    }
                });

            // Miqdor arizadagidan oshib ketgan xaridlar — fakt sifatida qayd etamiz.
            let over: Vec<String> = app
                .purchases
                .iter()
                .filter_map(|p| {
                    let q = app.requests.iter().find(|q| Some(q.id) == p.request_id)?;
                    (q.qty > 0.0 && p.qty > q.qty * 1.0001).then(|| {
                        format!("{} ({} > {})", p.number, trim_num(p.qty), trim_num(q.qty))
                    })
                })
                .collect();
            if !over.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        t("purchase_over_request"),
                        over.join(", ")
                    ))
                    .size(11.5)
                    .color(theme::warn()),
                );
            }
        });

    if let Some(p) = edited {
        app.db.update_purchase(&p);
        if let Some(slot) = app.purchases.iter_mut().find(|x| x.id == p.id) {
            *slot = p;
        }
    }
    if let Some(id) = removed {
        app.db.del("purchase", id);
        app.reload_modules();
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
