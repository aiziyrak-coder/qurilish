//! «Xaridlar» ekrani (TZ X).
//!
//! Zanjirning ikkinchi bo'g'ini: ariza → xarid → yetkazish. Yetkazilgan xarid
//! ombor kirimiga aylanishi kerak; shuning uchun bu yerda kirim yozilmagan
//! yetkazishlar alohida ajratib ko'rsatiladi va bir bosishda kirim yaratiladi.

use super::materials::{material_label, trim_num};
use super::requests::request_label;
use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::NoteTarget;
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
    // Kunlik: buyurtma, taklif, ta'minotchi va zanjir nazorati.
    super::tab_row(
        ui,
        &mut tab,
        &[
            (0, t("pu_tab_orders")),
            (2, t("pu_tab_quotes")),
            (3, t("pu_tab_suppliers")),
            (6, t("pu_tab_chain")),
        ],
        &[
            (1, t("pu_tab_plan")),
            (4, t("pu_tab_budget")),
            (5, t("pu_tab_risks")),
        ],
    );
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => plan_tab(ui, app, pid),
        2 => quotes_tab(ui, app, pid),
        3 => suppliers_tab(ui, app, pid),
        4 => budget_tab(ui, app, pid),
        5 => risks_tab(ui, app),
        6 => chain_tab(ui, app),
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
        task_id: None,
        contract_id: None,
        urgent: false,
        buyer: String::new(),
        material_id: None,
        substitute_for: None,
        paid: 0.0,
        pay_due: None,
        tech_ok: false,
        tech_by: String::new(),
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
    let mut open_notes: Option<i64> = None;

    let today = app.today;
    let waiting: Vec<i64> = unposted(app).iter().map(|p| p.id).collect();
    // Tor ekranda mas'ul va ish ustunlari yashiriladi: kunlik ish miqdor,
    // narx va muddat ustunlarida.
    let wide = ui.available_width() > 1700.0;
    let tasks = app.tasks.clone();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("purchases_grid")
                .num_columns(if wide { 22 } else { 20 })
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 30.0, t("col_urgent"));
                    head_l(ui, 30.0, t("col_tech_ok"));
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
                    head_r(ui, 120.0, t("col_paid"));
                    head_l(ui, 110.0, t("col_pay_due"));
                    head_l(ui, 110.0, t("col_delivery"));
                    head_l(ui, 120.0, t("col_status"));
                    head_l(ui, 110.0, t("col_stock_post"));
                    if wide {
                        head_l(ui, 130.0, t("col_buyer"));
                        head_l(ui, 120.0, t("col_task"));
                    }
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for src in &app.purchases {
                        let mut p = src.clone();
                        let mut changed = false;

                        // Shoshilinch xarid odatdagi tartibdan chetga chiqish —
                        // u ro'yxatda ko'zga tashlanib turishi kerak.
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            changed |= ui
                                .checkbox(&mut p.urgent, "")
                                .on_hover_text(t("col_urgent"))
                                .changed();
                        });
                        // Texnik kelishuv (TZ X.18): xarid loyihaga mos
                        // ekanini muhandis tasdiqlaydi. Kim tasdiqlagani
                        // birga yoziladi — keyin so'rash uchun.
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            let mut ok = p.tech_ok;
                            if ui
                                .checkbox(&mut ok, "")
                                .on_hover_text(if p.tech_by.trim().is_empty() {
                                    t("col_tech_ok_hint").to_string()
                                } else {
                                    format!("{}: {}", t("col_tech_ok"), p.tech_by)
                                })
                                .changed()
                            {
                                p.tech_ok = ok;
                                p.tech_by = if ok {
                                    app.current_user_name()
                                } else {
                                    String::new()
                                };
                                changed = true;
                            }
                        });
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

                        // To'lov (TZ X.23): to'langan summa va muddati.
                        // Muddati o'tib qarz qolgan bo'lsa — qizil.
                        changed |= super::materials::num_edit(ui, 120.0, &mut p.paid, 1000.0, 1e12);
                        ui.horizontal(|ui| match p.pay_due {
                            Some(mut d) => {
                                if super::passport::date_edit(ui, &format!("pupay{}", p.id), &mut d)
                                {
                                    p.pay_due = Some(d);
                                    changed = true;
                                }
                                if p.payment_overdue(today) {
                                    ui.label(RichText::new("!").color(theme::danger()).strong())
                                        .on_hover_text(t("col_pay_overdue"));
                                }
                            }
                            None => {
                                if ui.small_button(t("col_pay_set")).clicked() {
                                    p.pay_due = Some(p.delivery_date + chrono::Duration::days(14));
                                    changed = true;
                                }
                            }
                        });

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

                        if wide {
                            changed |= ui
                                .add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut p.buyer))
                                .changed();
                            // Xarid qaysi ish uchun: kechikish qaysi ishni
                            // to'xtatishini shu bog'lanish ko'rsatadi.
                            let label = p
                                .task_id
                                .and_then(|id| tasks.iter().find(|t| t.id == id))
                                .map(|t| t.wbs.clone())
                                .unwrap_or_else(|| t("dash").to_string());
                            egui::ComboBox::from_id_salt(("pu_task", p.id))
                                .selected_text(label)
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut p.task_id, None, t("dash"))
                                        .changed();
                                    for tk in &tasks {
                                        changed |= ui
                                            .selectable_value(
                                                &mut p.task_id,
                                                Some(tk.id),
                                                format!("{} {}", tk.wbs, tk.name),
                                            )
                                            .changed();
                                    }
                                });
                        }

                        // Qabulda foto va izoh (TZ X.28).
                        if super::notes::badge(ui, app, NoteTarget::Purchase, p.id) {
                            open_notes = Some(p.id);
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
    super::notes::below_table(ui, app, NoteTarget::Purchase, open_notes);
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
/// Ta'minotchi e'tirozini gapga aylantiradi (TZ X.16).
fn supplier_issue_text(i: &crate::checks::SupplierIssue) -> String {
    use crate::checks::SupplierIssue as S;
    match i {
        S::NoInn => t("si_no_inn").to_string(),
        S::BadInn { value } => format!("{} «{value}»", t("si_bad_inn")),
        S::BlockedButUsed { purchases } => format!("{} ({purchases})", t("si_blocked")),
        S::NoContact => t("si_no_contact").to_string(),
        S::TooBigShare { pct } => format!("{} {pct:.0}%", t("si_share")),
        S::LateHistory { late, total } => {
            format!("{}: {late} / {total}", t("si_late"))
        }
    }
}

fn suppliers_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let lines = crate::checks::supplier_lines(&app.purchases, app.today);

    // Tekshiruv hujjatga qaraydi, obro'ga emas: STIR to'g'ri yozilganmi,
    // taqiq belgisi bormi, aloqa bormi, yetkazish tarixi qanday (TZ X.16).
    let cards = app.supplier_cards();
    let bad = cards.iter().filter(|c| !c.issues.is_empty()).count();
    if bad > 0 {
        ui.label(
            RichText::new(format!("{}: {bad}", t("pu_sup_issues")))
                .size(12.5)
                .strong()
                .color(theme::warn()),
        );
        for c in cards.iter().filter(|c| !c.issues.is_empty()).take(8) {
            ui.label(
                RichText::new(format!(
                    "· {} ({:.0}%) — {}",
                    super::issues::truncate(&c.name, 26),
                    c.share_pct,
                    c.issues
                        .iter()
                        .map(supplier_issue_text)
                        .collect::<Vec<_>>()
                        .join("; ")
                ))
                .size(12.0)
                .color(if c.issues.iter().any(|i| i.severe()) {
                    theme::danger()
                } else {
                    theme::muted()
                }),
            )
            // Xaridlar soni va summasi — e'tirozning og'irligi shundan
            // ko'rinadi: bitta xaridli ta'minotchi bilan o'ntalikning
            // e'tirozi bir xil emas.
            .on_hover_text(format!(
                "#{} · {} {} · {}",
                c.supplier_id,
                c.purchases,
                t("mat_maker_deals"),
                money(c.amount)
            ));
        }
        ui.add_space(10.0);
    }

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

// ================================================================ Xarid rejasi

/// Reja shuncha kun oldinga qaraydi.
const PLAN_HORIZON: i64 = 45;

/// Nima sotib olish kerakligi (TZ X.4-6).
///
/// Ro'yxat qo'lda tuzilmaydi: qoldiq, yo'ldagi buyurtma va yaqin ishlarning
/// normativ ehtiyoji solishtiriladi. Har qatordan bir bosishda ariza
/// ochiladi — reja va ariza orasida qo'lda ko'chirish bo'lmasin.
fn plan_tab(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let stock = app.stock();
    let lines = crate::checks::purchase_plan(
        &app.materials,
        &stock,
        &app.purchases,
        &app.requests,
        &app.material_norms,
        &app.tasks,
        app.today,
        PLAN_HORIZON,
    );

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(t("pu_plan_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if lines.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("pu_plan_empty"))
                    .color(theme::ok())
                    .size(15.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("pu_plan_empty_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    let total: f64 = lines.iter().map(|l| l.cost).sum();
    let tight = lines.iter().filter(|l| l.tight(app.today)).count();
    stat_row(
        ui,
        vec![
            stat(
                t("pu_plan_kpi_items"),
                lines.len().to_string(),
                t("pu_plan_kpi_items_hint"),
                theme::accent(),
            ),
            stat(
                t("pu_plan_kpi_sum"),
                money(total),
                t("pu_plan_kpi_sum_hint"),
                theme::text(),
            ),
            stat(
                t("pu_plan_kpi_tight"),
                tight.to_string(),
                t("pu_plan_kpi_tight_hint"),
                if tight == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(10.0);

    let can = app.can_edit(Screen::Requests);
    let today = app.today;
    let mut make: Option<(i64, f64)> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("pu_plan_grid")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 230.0, t("col_material"));
                    head_r(ui, 100.0, t("pu_plan_available"));
                    head_r(ui, 100.0, t("pu_plan_ordered"));
                    head_r(ui, 100.0, t("col_min_stock"));
                    head_r(ui, 110.0, t("pu_plan_needed"));
                    head_r(ui, 110.0, t("pu_plan_to_buy"));
                    head_r(ui, 140.0, t("pu_plan_cost"));
                    head_l(ui, 120.0, t("pu_plan_need_by"));
                    head_l(ui, 110.0, t("col_status"));
                    head_l(ui, 110.0, "");
                    ui.end_row();

                    for l in &lines {
                        let Some(m) = app.materials.iter().find(|m| m.id == l.material_id) else {
                            continue;
                        };
                        cell_l(
                            ui,
                            230.0,
                            RichText::new(super::issues::truncate(&m.name, 30)).size(12.5),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(format!("{} {}", trim_num(l.available), m.unit))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(if l.ordered > 0.0 {
                                trim_num(l.ordered)
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.0)
                            .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(trim_num(l.min_stock))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(if l.needed_for_tasks > 0.0 {
                                trim_num(l.needed_for_tasks)
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.0),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(trim_num(l.to_buy))
                                .size(12.5)
                                .strong()
                                .color(theme::accent()),
                        );
                        cell_r(ui, 140.0, RichText::new(money(l.cost)).size(12.0));
                        cell_l(
                            ui,
                            120.0,
                            match l.need_by {
                                Some(d) => RichText::new(d.format("%d.%m.%Y").to_string())
                                    .size(12.0)
                                    .color(if l.tight(today) {
                                        theme::danger()
                                    } else {
                                        theme::muted()
                                    }),
                                None => RichText::new(t("dash")).color(theme::muted()),
                            },
                        );
                        cell_l(
                            ui,
                            110.0,
                            if l.has_request {
                                RichText::new(t("pu_plan_has_request"))
                                    .size(11.0)
                                    .color(theme::ok())
                            } else {
                                RichText::new(t("pu_plan_no_request"))
                                    .size(11.0)
                                    .color(theme::warn())
                            },
                        );
                        // Rejadan arizaga: miqdor va material o'z-o'zidan
                        // ko'chadi, odam qaytadan yozib o'tirmaydi.
                        if ui
                            .add_enabled(
                                can && !l.has_request,
                                egui::Button::new(RichText::new(t("pu_plan_make")).size(11.5)),
                            )
                            .on_disabled_hover_text(t("pu_plan_make_off"))
                            .clicked()
                        {
                            make = Some((l.material_id, l.to_buy));
                        }
                        ui.end_row();
                    }
                });
        });

    if let Some((material_id, qty)) = make {
        let m = app.materials.iter().find(|m| m.id == material_id).cloned();
        if let Some(m) = m {
            let n = app.requests.len() + 1;
            app.db.insert_request(&crate::domain::Request {
                id: 0,
                project_id: pid,
                number: format!("A-{n:04}"),
                date: today,
                kind: crate::domain::RequestKind::Material,
                title: m.name.clone(),
                material_id: Some(m.id),
                qty,
                unit: m.unit.clone(),
                requester: app.current_user_name(),
                need_date: today + chrono::Duration::days(14),
                priority: crate::domain::Priority::Normal,
                status: crate::domain::RequestStatus::New,
                task_id: None,
                reject_reason: String::new(),
                note: t("pu_plan_from_plan").to_string(),
            });
            app.reload_modules();
            app.notify(format!("{} {}", t("pu_plan_made"), m.name));
        }
    }
}

// ================================================================ Risklar

/// Xarid jarayonidagi shubhali joylar (TZ X.41, 46).
/// Tartib e'tirozini odam o'qiydigan gapga aylantiradi (TZ X.18-19, 22).
fn control_text(i: &crate::checks::SupplyIssue) -> String {
    use crate::checks::SupplyIssue as S;
    match i {
        S::NoTechApproval { number, amount } => {
            format!("{} — {} ({})", t("si_no_tech"), number, money(*amount))
        }
        S::UnapprovedSubstitute { number, material } => {
            format!("{} — {} ({})", t("si_unapproved"), number, material)
        }
        S::SubstituteWithoutTech { number } => format!("{} — {}", t("si_sub_no_tech"), number),
        S::ContractOverrun {
            contract,
            over,
            pct,
        } => format!(
            "{} — {} · {} ({:.0}%)",
            t("si_overrun"),
            contract,
            money(*over),
            pct
        ),
        S::ContractExpired { contract, days } => {
            format!(
                "{} — {} ({} {})",
                t("si_expired"),
                contract,
                days,
                t("days")
            )
        }
        S::NoContract { number, amount } => {
            format!("{} — {} ({})", t("si_no_contract"), number, money(*amount))
        }
    }
}

// ================================================================ Zanjir

/// Ta'minot zanjiri: ariza → taklif → xarid → yetkazish → kirish
/// nazorati → ombor → ish → to'lov (TZ X.47, XI.47).
///
/// Har bosqich alohida modulda yozilgan; bu yerda ular bir qatorda
/// turadi, shuning uchun uzilish darhol ko'rinadi.
fn chain_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};
    let (rows, sum) = app.supply_chain();

    ui.label(
        RichText::new(t("pu_chain_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    stat_row(
        ui,
        vec![
            stat(
                t("pu_chain_lines"),
                sum.lines.to_string(),
                t("pu_chain_lines_hint"),
                theme::text(),
            ),
            stat(
                t("pu_chain_gaps"),
                sum.with_gaps.to_string(),
                t("pu_chain_gaps_hint"),
                if sum.with_gaps == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
            stat(
                t("pu_chain_paid"),
                format!("{} / {}", money(sum.paid), money(sum.amount)),
                t("pu_chain_paid_hint"),
                theme::text(),
            ),
            stat(
                t("pu_chain_overdue"),
                money(sum.overdue_pay),
                t("pu_chain_overdue_hint"),
                if sum.overdue_pay <= 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
    ui.add_space(12.0);

    if rows.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("pu_chain_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("pu_chain")
                .num_columns(9)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 220.0, t("col_item"));
                    head_r(ui, 90.0, t("pu_ch_request"));
                    head_r(ui, 80.0, t("pu_ch_quotes"));
                    head_r(ui, 90.0, t("pu_ch_order"));
                    head_r(ui, 90.0, t("col_delivered"));
                    head_r(ui, 80.0, t("pu_ch_checks"));
                    head_r(ui, 90.0, t("pu_ch_stock"));
                    head_r(ui, 90.0, t("pu_ch_issued"));
                    head_l(ui, 260.0, t("pu_ch_gaps"));
                    ui.end_row();

                    for (line, gaps) in &rows {
                        let name = cell_l(
                            ui,
                            220.0,
                            RichText::new(super::issues::truncate(&line.title, 28)).size(12.5),
                        );
                        // Zanjirning boshi: qaysi ariza va qaysi material —
                        // sichqoncha ostida, jadvalni kengaytirmasdan.
                        name.on_hover_ui(|ui| {
                            let request = line
                                .request_id
                                .and_then(|id| app.requests.iter().find(|r| r.id == id))
                                .map(|r| r.number.clone())
                                .unwrap_or_else(|| t("dash").to_string());
                            ui.label(
                                RichText::new(format!("{}: {}", t("col_request"), request))
                                    .size(11.5),
                            );
                            if let Some(id) = line.material_id {
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        t("col_material"),
                                        super::materials::material_label(app, id)
                                    ))
                                    .size(11.5),
                                );
                            }
                            ui.label(
                                RichText::new(format!("{}: {}", t("col_unit"), line.unit))
                                    .size(11.5)
                                    .color(theme::muted()),
                            );
                        });
                        let num = |ui: &mut egui::Ui, w: f32, v: f64| {
                            cell_r(
                                ui,
                                w,
                                if v == 0.0 {
                                    RichText::new(t("dash")).color(theme::muted())
                                } else {
                                    RichText::new(super::materials::trim_num(v)).size(12.0)
                                },
                            );
                        };
                        num(ui, 90.0, line.requested);
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(line.quotes.to_string()).size(12.0).color(
                                if line.quotes == 0 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                },
                            ),
                        );
                        num(ui, 90.0, line.ordered);
                        num(ui, 90.0, line.delivered);
                        cell_r(
                            ui,
                            80.0,
                            RichText::new(line.checks.to_string()).size(12.0).color(
                                if line.checks == 0 && line.delivered > 0.0 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                },
                            ),
                        );
                        num(ui, 90.0, line.stocked);
                        num(ui, 90.0, line.issued);
                        cell_l(
                            ui,
                            260.0,
                            RichText::new(if gaps.is_empty() {
                                t("pu_ch_ok").to_string()
                            } else {
                                gaps.iter().map(gap_text).collect::<Vec<_>>().join("; ")
                            })
                            .size(11.5)
                            .color(if gaps.is_empty() {
                                theme::ok()
                            } else if gaps.iter().any(|g| g.severe()) {
                                theme::danger()
                            } else {
                                theme::warn()
                            }),
                        );
                        ui.end_row();
                    }
                });
        });
}

/// Zanjirdagi uzilishni gapga aylantiradi.
fn gap_text(g: &crate::checks::ChainGap) -> String {
    use crate::checks::ChainGap as G;
    let n = super::materials::trim_num;
    match g {
        G::OrderOverRequest { over } => format!("{} +{}", t("cg_order_over"), n(*over)),
        G::DeliveryOverOrder { over } => format!("{} +{}", t("cg_delivery_over"), n(*over)),
        G::NotStocked { qty } => format!("{} {}", t("cg_not_stocked"), n(*qty)),
        G::IssuedOverStock { over } => format!("{} +{}", t("cg_issued_over"), n(*over)),
        G::NoInputCheck => t("cg_no_check").to_string(),
        G::NoQuotes => t("cg_no_quotes").to_string(),
        G::Overpaid { over } => format!("{} {}", t("cg_overpaid"), money(*over)),
        G::PaymentOverdue { unpaid } => format!("{} {}", t("cg_pay_overdue"), money(*unpaid)),
    }
}

fn risks_tab(ui: &mut egui::Ui, app: &mut App) {
    let risks = crate::checks::procurement_risks(&app.purchases, &app.quotes, &app.materials);
    let buyers = crate::checks::buyer_stats(&app.purchases, &app.quotes, app.today);
    let control = app.supply_control();

    ui.label(
        RichText::new(t("pu_risks_hint"))
            .size(11.0)
            .color(theme::muted()),
    );
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---------- Tartib nazorati (TZ X.18-19, 22) ----------
            ui.label(RichText::new(t("pu_control_title")).size(14.0).strong());
            ui.label(
                RichText::new(t("pu_control_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if control.is_empty() {
                ui.label(
                    RichText::new(t("pu_control_none"))
                        .size(12.0)
                        .color(theme::ok()),
                );
            }
            for i in &control {
                ui.label(
                    RichText::new(format!("· {}", control_text(i)))
                        .size(12.0)
                        .color(if i.severe() {
                            theme::danger()
                        } else {
                            theme::warn()
                        }),
                );
            }
            ui.add_space(16.0);

            // ---------- Belgilar ----------
            ui.label(RichText::new(t("pu_risks_title")).size(14.0).strong());
            ui.add_space(6.0);
            if risks.is_empty() {
                ui.label(
                    RichText::new(t("pu_risks_none"))
                        .size(12.0)
                        .color(theme::ok()),
                );
            }
            for r in &risks {
                let (text, detail, color) = risk_text(r);
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 1.5, color);
                    ui.add_space(6.0);
                    ui.label(RichText::new(text).size(12.5).color(color));
                    ui.label(RichText::new(detail).size(11.5).color(theme::muted()));
                });
                ui.add_space(3.0);
            }

            ui.add_space(16.0);

            // ---------- Xaridchilar ----------
            ui.label(RichText::new(t("pu_buyers_title")).size(14.0).strong());
            ui.label(
                RichText::new(t("pu_buyers_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);
            if buyers.is_empty() {
                ui.label(
                    RichText::new(t("pu_buyers_none"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                return;
            }
            egui::Grid::new("pu_buyers_grid")
                .num_columns(6)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 180.0, t("pu_buyer"));
                    head_r(ui, 90.0, t("pu_buyer_count"));
                    head_r(ui, 150.0, t("pu_buyer_amount"));
                    head_r(ui, 110.0, t("pu_buyer_on_time"));
                    head_r(ui, 110.0, t("pu_buyer_quotes"));
                    head_r(ui, 100.0, t("pu_buyer_urgent"));
                    ui.end_row();
                    for b in &buyers {
                        cell_l(ui, 180.0, RichText::new(&b.buyer).size(12.5));
                        cell_r(ui, 90.0, RichText::new(b.purchases.to_string()).size(12.0));
                        cell_r(ui, 150.0, RichText::new(money(b.amount)).size(12.0));
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(format!("{:.0}%", b.on_time_pct))
                                .size(12.0)
                                .color(if b.on_time_pct >= 80.0 {
                                    theme::ok()
                                } else {
                                    theme::warn()
                                }),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(format!("{:.0}%", b.with_quotes_pct))
                                .size(12.0)
                                .color(if b.with_quotes_pct >= 50.0 {
                                    theme::ok()
                                } else {
                                    theme::warn()
                                }),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(b.urgent.to_string()).size(12.0).color(
                                if b.urgent == 0 {
                                    theme::muted()
                                } else {
                                    theme::warn()
                                },
                            ),
                        );
                        ui.end_row();
                    }
                });
            ui.add_space(16.0);
        });
}

/// Risk belgisining matni, dalili va rangi.
fn risk_text(r: &crate::checks::ProcurementRisk) -> (String, String, egui::Color32) {
    use crate::checks::ProcurementRisk as R;
    match r {
        R::SupplierShare { supplier, pct } => (
            format!("{} — {:.0}%", supplier, pct),
            t("pu_risk_share").to_string(),
            theme::warn(),
        ),
        R::NoQuotes { number, amount } => (
            format!("{number} · {}", money(*amount)),
            t("pu_risk_no_quotes").to_string(),
            theme::accent(),
        ),
        R::HighPrice { number, over_pct } => (
            format!("{number} · +{:.0}%", over_pct),
            t("pu_risk_high_price").to_string(),
            theme::danger(),
        ),
        R::TooManyUrgent { count, pct } => (
            format!("{count} · {:.0}%", pct),
            t("pu_risk_urgent").to_string(),
            theme::warn(),
        ),
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
