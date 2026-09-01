//! «Xaridlar» ekrani (TZ X).
//!
//! Zanjirning ikkinchi bo'g'ini: ariza → xarid → yetkazish. Yetkazilgan xarid
//! ombor kirimiga aylanishi kerak; shuning uchun bu yerda kirim yozilmagan
//! yetkazishlar alohida ajratib ko'rsatiladi va bir bosishda kirim yaratiladi.

use super::materials::{material_label, trim_num};
use super::requests::request_label;
use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{
    MoveKind, Purchase, PurchaseBudget, PurchaseStatus, Quote, RequestStatus, StockMove, Supplier,
};
use crate::model::Section;

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

    let tab_key = egui::Id::new("pu_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("pu_tab_orders")),
            (1, t("pu_tab_quotes")),
            (2, t("pu_tab_suppliers")),
            (3, t("pu_tab_budget")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => quotes_tab(ui, app, pid),
        2 => suppliers_tab(ui, app, pid),
        3 => budget_tab(ui, app, pid),
        _ => {
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
        }
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
        delivered_qty: 0.0,
        // Bo'lim arizadagi materialdan olinadi — qo'lda tanlash ham mumkin.
        section: request_id
            .and_then(|id| app.requests.iter().find(|q| q.id == id))
            .and_then(|q| q.material_id)
            .and_then(|m| app.materials.iter().find(|x| x.id == m))
            .map(|m| m.section)
            .unwrap_or(Section::None),
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
            // Qisman yetkazilgan buyurtma ham kirim qilinadi (TZ X.30):
            // kelgan qismi omborda turishi kerak, qolgani yo'lda qoladi.
            let arrived = matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
                || p.delivered_qty > 0.0;
            arrived
                && posted_qty(p) > 0.0
                && material_of(app, p).is_some()
                && !app.stock_moves.iter().any(|m| m.document == p.number)
        })
        .collect()
}

/// Omborga qancha kirim qilinadi: kelgani ko'rsatilgan bo'lsa — o'sha,
/// aks holda buyurtma miqdori to'liq kelgan deb olinadi.
fn posted_qty(p: &Purchase) -> f64 {
    if p.delivered_qty > 0.0 {
        p.delivered_qty
    } else {
        p.qty
    }
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
                posted_qty(p),
                p.price,
                p.delivery_date,
                p.number.clone(),
                p.supplier.clone(),
            ))
        })
        .collect();
    let n = list.len();
    // Kelgan material omborga tushadi (TZ XI.3): ombor tanlangan bo'lsa o'shanga,
    // aks holda birinchi omborga. Ombor umuman yo'q bo'lsa — bog'lanmagan qoladi.
    let warehouse = app
        .warehouse_filter
        .or_else(|| app.warehouses.first().map(|w| w.id));
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
            warehouse_id: warehouse,
            batch_id: None,
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
                .num_columns(16)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 80.0, t("col_number"));
                    head_l(ui, 110.0, t("col_date"));
                    head_l(ui, 180.0, t("col_supplier"));
                    head_l(ui, 200.0, t("col_item"));
                    head_l(ui, 190.0, t("col_request"));
                    head_l(ui, 90.0, t("col_section"));
                    head_r(ui, 80.0, t("col_qty"));
                    head_r(ui, 90.0, t("col_delivered"));
                    head_l(ui, 60.0, t("col_unit"));
                    head_r(ui, 110.0, t("col_price"));
                    head_l(ui, 14.0, "");
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

                        // Bo'lim — byudjet nazorati shu kesimda yuritiladi.
                        egui::ComboBox::from_id_salt(("pu_sec", p.id))
                            .selected_text(RichText::new(p.section.code()).size(11.5))
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for sec in Section::ALL {
                                    changed |= ui
                                        .selectable_value(&mut p.section, sec, sec.label())
                                        .changed();
                                }
                            });
                        changed |= ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut p.qty).speed(1.0).range(0.0..=1e9),
                            )
                            .changed();
                        // Kelgan miqdor (TZ X.30): qisman yetkazish oddiy holat.
                        let resp = ui.add_sized(
                            [90.0, 22.0],
                            egui::DragValue::new(&mut p.delivered_qty)
                                .speed(1.0)
                                .range(0.0..=1e9),
                        );
                        changed |= resp.changed();
                        if p.partial() {
                            resp.on_hover_text(format!(
                                "{} {}",
                                t("purchase_partial"),
                                trim_num(p.remaining())
                            ));
                        }
                        changed |= ui
                            .add_sized([60.0, 22.0], egui::TextEdit::singleline(&mut p.unit))
                            .changed();
                        let resp = ui.add_sized(
                            [110.0, 22.0],
                            egui::DragValue::new(&mut p.price)
                                .speed(100.0)
                                .range(0.0..=1e12),
                        );
                        changed |= resp.changed();
                        // Narx katalogdagidan keskin farq qilsa — e'tibor tortamiz
                        // (TZ X.13). Bu taqiq emas: bozor narxi o'zgargan bo'lishi
                        // mumkin, lekin xato ham shu yerda ko'rinadi.
                        if let Some(pct) = catalog_price(app, &p)
                            .and_then(|c| crate::checks::price_anomaly(p.price, c))
                        {
                            resp.on_hover_text(format!("{} {pct:+.0}%", t("price_anomaly")));
                            ui.label(RichText::new("!").color(theme::warn()).strong());
                        } else {
                            ui.label("");
                        }
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

/// Xaridning katalogdagi narxi — narx anomaliyasini shunga solishtiramiz.
fn catalog_price(app: &App, p: &Purchase) -> Option<f64> {
    let m = material_of(app, p)?;
    app.materials
        .iter()
        .find(|x| x.id == m)
        .map(|x| x.price)
        .filter(|v| *v > 0.0)
}

// ================================================================ Takliflar

/// Tijorat takliflari — KP (TZ X.9–12, 15).
fn quotes_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_quote")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("quotes_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.quotes.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("quotes_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
    } else {
        let lines = crate::checks::quote_lines(&app.quotes, app.today);
        let mut edited: Option<Quote> = None;
        let mut removed: Option<i64> = None;
        let mut chosen: Option<i64> = None;
        let mut to_purchase: Option<i64> = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("pu_quotes")
                    .num_columns(12)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 170.0, t("col_request"));
                        head_l(ui, 180.0, t("col_supplier"));
                        head_l(ui, 190.0, t("col_item"));
                        head_r(ui, 80.0, t("col_qty"));
                        head_r(ui, 120.0, t("col_price"));
                        head_r(ui, 130.0, t("col_sum"));
                        head_r(ui, 80.0, t("col_over_best"));
                        head_r(ui, 90.0, t("col_delivery_days"));
                        head_l(ui, 110.0, t("col_valid_until"));
                        head_l(ui, 120.0, t("col_verdict"));
                        head_l(ui, 150.0, "");
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.quotes {
                            let mut q = src.clone();
                            let mut changed = false;
                            let l = lines.iter().find(|l| l.quote_id == q.id);

                            egui::ComboBox::from_id_salt(("q_req", q.id))
                                .selected_text(super::issues::truncate(
                                    &match q.request_id {
                                        Some(id) => request_label(app, id),
                                        None => t("dash").to_string(),
                                    },
                                    22,
                                ))
                                .width(170.0)
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut q.request_id, None, t("dash"))
                                        .changed();
                                    for r in &app.requests {
                                        changed |= ui
                                            .selectable_value(
                                                &mut q.request_id,
                                                Some(r.id),
                                                request_label(app, r.id),
                                            )
                                            .changed();
                                    }
                                });
                            changed |= ui
                                .add_sized(
                                    [180.0, 22.0],
                                    egui::TextEdit::singleline(&mut q.supplier),
                                )
                                .changed();
                            changed |= ui
                                .add_sized([190.0, 22.0], egui::TextEdit::singleline(&mut q.title))
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [80.0, 22.0],
                                    egui::DragValue::new(&mut q.qty).speed(1.0).range(0.0..=1e9),
                                )
                                .changed();
                            changed |= ui
                                .add_sized(
                                    [120.0, 22.0],
                                    egui::DragValue::new(&mut q.price)
                                        .speed(100.0)
                                        .range(0.0..=1e12),
                                )
                                .changed();
                            cell_r(ui, 130.0, RichText::new(money(q.amount())).size(12.5));

                            // Eng arzondan farq: nol bo'lsa — shu eng arzoni.
                            let over = l.map(|l| l.over_best_pct).unwrap_or(0.0);
                            cell_r(
                                ui,
                                80.0,
                                RichText::new(if over > 0.01 {
                                    format!("+{over:.0}%")
                                } else {
                                    t("dash").to_string()
                                })
                                .size(11.5)
                                .color(if over > 10.0 {
                                    theme::danger()
                                } else if over > 0.01 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                }),
                            );
                            changed |= ui
                                .add_sized(
                                    [90.0, 22.0],
                                    egui::DragValue::new(&mut q.delivery_days)
                                        .speed(1.0)
                                        .range(0..=365),
                                )
                                .changed();

                            ui.horizontal(|ui| {
                                let mut has = q.valid_until.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    q.valid_until =
                                        has.then(|| app.today + chrono::Duration::days(14));
                                    changed = true;
                                }
                                if let Some(mut d) = q.valid_until {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("qv{}", q.id),
                                        &mut d,
                                    ) {
                                        q.valid_until = Some(d);
                                        changed = true;
                                    }
                                }
                            });

                            // Xulosa: eng arzoni, eng tezi yoki muddati o'tgani.
                            let (verdict, color) = match l {
                                Some(l) if l.expired => (t("quote_expired"), theme::muted()),
                                Some(l) if l.cheapest && l.fastest => {
                                    (t("quote_best"), theme::ok())
                                }
                                Some(l) if l.cheapest => (t("quote_cheapest"), theme::ok()),
                                Some(l) if l.fastest => (t("quote_fastest"), theme::accent()),
                                _ => ("", theme::muted()),
                            };
                            cell_l(ui, 120.0, RichText::new(verdict).size(11.0).color(color));

                            ui.horizontal(|ui| {
                                if q.chosen {
                                    ui.label(
                                        RichText::new(t("quote_chosen"))
                                            .size(11.0)
                                            .color(theme::ok())
                                            .strong(),
                                    );
                                    if ui.small_button(t("quote_to_purchase")).clicked() {
                                        to_purchase = Some(q.id);
                                    }
                                } else if ui
                                    .small_button(t("quote_choose"))
                                    .on_hover_text(t("quote_choose_hint"))
                                    .clicked()
                                {
                                    chosen = Some(q.id);
                                }
                            });

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
            app.db.update_quote(&q);
            app.reload_modules();
        }
        if let Some(id) = chosen {
            app.db.choose_quote(id);
            app.reload_modules();
        }
        if let Some(id) = removed {
            app.db.del("quote", id);
            app.reload_modules();
        }
        if let Some(id) = to_purchase {
            if let Some(q) = app.quotes.iter().find(|q| q.id == id).cloned() {
                let n = app.purchases.len() + 1;
                let mut p = new_purchase(app, pid, format!("X-{n:03}"), q.request_id);
                p.supplier = q.supplier.clone();
                p.title = q.title.clone();
                p.qty = q.qty;
                p.unit = q.unit.clone();
                p.price = q.price;
                p.currency = q.currency.clone();
                p.delivery_date = app.today + chrono::Duration::days(q.delivery_days.max(0));
                p.status = PurchaseStatus::Ordered;
                app.db.insert_purchase(&p);
                app.reload_modules();
                app.notify(t("quote_purchase_created").to_string());
            }
        }
    }

    if add {
        app.db.insert_quote(&Quote {
            id: 0,
            project_id: pid,
            request_id: None,
            supplier: String::new(),
            title: String::new(),
            qty: 0.0,
            unit: String::new(),
            price: 0.0,
            currency: app.settings.default_currency.clone(),
            delivery_days: 7,
            valid_until: Some(app.today + chrono::Duration::days(14)),
            chosen: false,
            date: app.today,
            note: String::new(),
        });
        app.reload_modules();
    }
}

// ================================================== Yetkazib beruvchilar

/// Yetkazib beruvchilar va ularning xaridlardan hisoblangan tarixi (TZ X.7–8, 40).
fn suppliers_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let lines = crate::checks::supplier_lines(&app.purchases, app.today);

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_supplier")).clicked() {
            add = true;
        }
        // Xaridlarda uchraydigan, lekin kartochkasi yo'q yetkazib beruvchilar.
        let missing = lines
            .iter()
            .filter(|l| !app.suppliers.iter().any(|s| s.name.trim() == l.supplier))
            .count();
        if missing > 0 && ui.button(t("suppliers_from_purchases")).clicked() {
            for l in &lines {
                if app.suppliers.iter().any(|s| s.name.trim() == l.supplier) {
                    continue;
                }
                app.db.insert_supplier(&Supplier {
                    id: 0,
                    project_id: pid,
                    name: l.supplier.clone(),
                    inn: String::new(),
                    contact: String::new(),
                    phone: String::new(),
                    blocked: false,
                    note: String::new(),
                });
            }
            app.reload_modules();
        }
        ui.label(
            RichText::new(t("suppliers_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if app.suppliers.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("suppliers_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let mut edited: Option<Supplier> = None;
    let mut removed: Option<i64> = None;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("pu_suppliers")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 220.0, t("col_supplier"));
                    head_l(ui, 110.0, t("col_inn"));
                    head_l(ui, 150.0, t("col_contact"));
                    head_l(ui, 150.0, t("col_phone"));
                    head_r(ui, 80.0, t("col_orders"));
                    head_r(ui, 150.0, t("col_sum"));
                    head_r(ui, 90.0, t("col_on_time"));
                    head_r(ui, 100.0, t("col_avg_delay"));
                    head_l(ui, 110.0, t("col_last_order"));
                    head_l(ui, 90.0, t("col_blocked"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.suppliers {
                        let mut x = src.clone();
                        let mut changed = false;
                        let l = lines.iter().find(|l| l.supplier == x.name.trim());

                        changed |= ui
                            .add_sized([220.0, 22.0], egui::TextEdit::singleline(&mut x.name))
                            .changed();
                        changed |= ui
                            .add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut x.inn))
                            .changed();
                        changed |= ui
                            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut x.contact))
                            .changed();
                        changed |= ui
                            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut x.phone))
                            .changed();

                        // Jami buyurtma va shundan yopilmagani.
                        let open = l.map(|l| l.open_orders).unwrap_or(0);
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(match l.map(|l| l.orders).unwrap_or(0) {
                                0 => t("dash").to_string(),
                                n if open > 0 => format!("{n} · {open}"),
                                n => n.to_string(),
                            })
                            .size(12.0),
                        )
                        .on_hover_text(t("orders_open_hint"));
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(l.map(|l| l.amount).unwrap_or(0.0))).size(12.0),
                        );
                        // Muddatida yetkazish — asosiy ko'rsatkich.
                        let on_time = l.map(|l| l.on_time_pct).unwrap_or(0.0);
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(if l.is_some_and(|l| l.orders > 0) {
                                format!("{on_time:.0}%")
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.0)
                            .color(
                                if l.is_none_or(|l| l.orders == 0) {
                                    theme::muted()
                                } else if on_time >= 90.0 {
                                    theme::ok()
                                } else if on_time >= 60.0 {
                                    theme::warn()
                                } else {
                                    theme::danger()
                                },
                            ),
                        );
                        let delay = l.map(|l| l.avg_delay).unwrap_or(0.0);
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(if delay > 0.0 {
                                format!("{delay:.0} {}", t("days_short"))
                            } else {
                                t("dash").to_string()
                            })
                            .size(11.5)
                            .color(if delay > 0.0 {
                                theme::danger()
                            } else {
                                theme::muted()
                            }),
                        );
                        cell_l(
                            ui,
                            110.0,
                            RichText::new(match l.and_then(|l| l.last_order) {
                                Some(d) => d.format("%d.%m.%y").to_string(),
                                None => t("dash").to_string(),
                            })
                            .size(11.5)
                            .monospace()
                            .color(theme::muted()),
                        );
                        ui.horizontal(|ui| {
                            changed |= ui.checkbox(&mut x.blocked, "").changed();
                            if x.blocked {
                                ui.label(
                                    RichText::new(t("supplier_blocked"))
                                        .size(11.0)
                                        .color(theme::danger()),
                                );
                            }
                        });
                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(x.id);
                        }
                        ui.end_row();
                        if changed {
                            edited = Some(x);
                        }
                    }
                });
        });

    if let Some(x) = edited {
        app.db.update_supplier(&x);
        app.reload_modules();
    }
    if let Some(id) = removed {
        app.db.del("supplier", id);
        app.reload_modules();
    }
    if add {
        app.db.insert_supplier(&Supplier {
            id: 0,
            project_id: pid,
            name: t("supplier_new_name").to_string(),
            inn: String::new(),
            contact: String::new(),
            phone: String::new(),
            blocked: false,
            note: String::new(),
        });
        app.reload_modules();
    }
}

// ================================================================ Byudjet

/// Bo'limlar bo'yicha xarid byudjeti (TZ X.34–35).
fn budget_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let lines = crate::checks::budget_lines(&app.purchase_budgets, &app.purchases);

    let mut add = false;
    ui.horizontal_wrapped(|ui| {
        if ui.button(t("add_budget")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("budget_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if lines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("budget_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let planned: f64 = lines.iter().map(|l| l.planned).sum();
    let ordered: f64 = lines.iter().map(|l| l.ordered).sum();
    let over = lines.iter().filter(|l| l.over).count();
    stat_row(
        ui,
        vec![
            stat(
                t("kpi_budget_planned"),
                money(planned),
                t("kpi_budget_planned_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_budget_ordered"),
                money(ordered),
                t("kpi_budget_ordered_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_budget_left"),
                money(planned - ordered),
                t("kpi_budget_left_hint"),
                if ordered > planned {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
            stat(
                t("kpi_budget_over"),
                over.to_string(),
                t("kpi_budget_over_hint"),
                if over > 0 {
                    theme::danger()
                } else {
                    theme::ok()
                },
            ),
        ],
    );
    ui.add_space(10.0);

    let mut edited: Option<PurchaseBudget> = None;
    let mut removed: Option<i64> = None;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("pu_budget")
                .num_columns(8)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 200.0, t("col_section"));
                    head_r(ui, 150.0, t("col_planned"));
                    head_r(ui, 150.0, t("col_ordered"));
                    head_r(ui, 150.0, t("col_delivered"));
                    head_r(ui, 150.0, t("col_left"));
                    head_l(ui, 160.0, "");
                    head_r(ui, 70.0, "%");
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for l in &lines {
                        let b = app
                            .purchase_budgets
                            .iter()
                            .find(|b| b.section == l.section)
                            .cloned();

                        cell_l(ui, 200.0, RichText::new(l.section.label()).size(12.0));

                        // Reja tahrirlanadi; byudjeti yo'q bo'limda faqat ko'rinadi.
                        let mut budget_id = None;
                        match b {
                            Some(mut b) => {
                                budget_id = Some(b.id);
                                if ui
                                    .add_sized(
                                        [150.0, 22.0],
                                        egui::DragValue::new(&mut b.planned)
                                            .speed(100_000.0)
                                            .range(0.0..=1e15),
                                    )
                                    .changed()
                                {
                                    edited = Some(b.clone());
                                }
                            }
                            None => {
                                cell_r(
                                    ui,
                                    150.0,
                                    RichText::new(t("dash")).size(12.0).color(theme::muted()),
                                );
                            }
                        }
                        cell_r(ui, 150.0, RichText::new(money(l.ordered)).size(12.0));
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(l.delivered))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        let color = if l.over {
                            theme::danger()
                        } else if l.used_pct > 90.0 {
                            theme::warn()
                        } else {
                            theme::ok()
                        };
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(if l.planned > 0.0 {
                                money(l.left)
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.5)
                            .color(color),
                        );
                        // Foizli chiziq: byudjet qanchalik ishlatilgani.
                        bar(ui, 160.0, l.used_pct / 100.0, color);
                        cell_r(
                            ui,
                            70.0,
                            RichText::new(if l.planned > 0.0 {
                                format!("{:.0}%", l.used_pct)
                            } else {
                                t("dash").to_string()
                            })
                            .size(11.5)
                            .color(color),
                        );
                        match budget_id {
                            Some(id) => {
                                if ui
                                    .small_button(RichText::new("x").color(theme::danger()))
                                    .clicked()
                                {
                                    removed = Some(id);
                                }
                            }
                            None => {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some(b) = edited {
        app.db.update_purchase_budget(&b);
        app.reload_modules();
    }
    if let Some(id) = removed {
        app.db.del("purchase_budget", id);
        app.reload_modules();
    }
    if add {
        // Byudjeti yo'q birinchi bo'lim uchun qator ochamiz.
        let free = Section::ALL
            .into_iter()
            .find(|s| !app.purchase_budgets.iter().any(|b| b.section == *s));
        match free {
            Some(section) => {
                app.db.insert_purchase_budget(&PurchaseBudget {
                    id: 0,
                    project_id: pid,
                    section,
                    planned: 0.0,
                    note: String::new(),
                });
                app.reload_modules();
            }
            None => app.notify(t("budget_all_sections").to_string()),
        }
    }
}

/// Foiz chizig'i: to'ldirilgan ulush `v` (0..1 dan oshsa to'liq bo'ladi).
fn bar(ui: &mut egui::Ui, width: f32, v: f64, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, theme::track());
    let w = (v.clamp(0.0, 1.0) as f32) * rect.width();
    if w > 0.5 {
        let mut fill = rect;
        fill.set_width(w);
        p.rect_filled(fill, 3.0, color);
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
