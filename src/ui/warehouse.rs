//! «Ombor» ekrani (TZ XI).
//!
//! Besh ko'rinish: **qoldiqlar** (ombor kesimida, rezerv va erkin qoldiq bilan),
//! **harakatlar** (kirim, chiqim, qaytarish, hisobdan chiqarish), **partiyalar**
//! (sertifikat, yaroqlilik muddati, FEFO navbati), **rezerv** (material aniq
//! ishga band qilinadi) va **inventarizatsiya** (hisob va fakt farqi).
//!
//! Qoldiq harakatlardan hisoblanadi, alohida saqlanmaydi — shunda hujjat va
//! qoldiq hech qachon bir-biriga zid bo'lmaydi.

use super::materials::{material_label, material_picker, stock_bar, trim_num};
use super::*;
use crate::domain::{
    Batch, Inventory, InventoryLine, MoveKind, Reservation, StockMove, Warehouse, WarehouseKind,
};

/// Shuncha kundan beri qimirlamagan qoldiq «uzoq turgan» hisoblanadi (TZ XI.41).
const STALE_DAYS: i64 = 90;

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
    if app.materials.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.label(
                RichText::new(t("wh_no_materials"))
                    .color(theme::muted())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("wh_no_materials_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    let tab_key = egui::Id::new("wh_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    toolbar(ui, app, &mut tab);
    ui.add_space(8.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    ui.horizontal_wrapped(|ui| {
        for (i, label) in [
            (0u8, t("wh_tab_balance")),
            (1, t("wh_tab_moves")),
            (2, t("wh_tab_batches")),
            (3, t("wh_tab_reserve")),
            (4, t("wh_tab_inventory")),
        ] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => moves_tab(ui, app),
        2 => batches_tab(ui, app),
        3 => reserve_tab(ui, app),
        4 => inventory_tab(ui, app),
        _ => balance_tab(ui, app),
    }
}

// ================================================================ Yuqori panel

fn toolbar(ui: &mut egui::Ui, app: &mut App, tab: &mut u8) {
    let Some(pid) = app.current else { return };
    let mut add_move: Option<MoveKind> = None;
    let mut add_warehouse = false;
    let mut transfer = false;

    ui.horizontal_wrapped(|ui| {
        // Ombor tanlash (TZ XI.3). Bir obyektda bir nechta ombor bo'lishi mumkin.
        let text = app
            .warehouse_filter
            .and_then(|id| app.warehouses.iter().find(|w| w.id == id))
            .map(|w| w.name.clone())
            .unwrap_or_else(|| t("wh_all").to_string());
        egui::ComboBox::from_id_salt("wh_filter")
            .selected_text(text)
            .width(180.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.warehouse_filter, None, t("wh_all"));
                for w in &app.warehouses {
                    ui.selectable_value(
                        &mut app.warehouse_filter,
                        Some(w.id),
                        format!("{} · {}", w.name, w.kind.label()),
                    );
                }
            });
        if ui
            .button(t("add_warehouse"))
            .on_hover_text(t("add_warehouse_hint"))
            .clicked()
        {
            add_warehouse = true;
        }

        ui.separator();
        for (kind, label) in [
            (MoveKind::In, t("wh_add_in")),
            (MoveKind::Out, t("wh_add_out")),
            (MoveKind::Return, t("wh_add_return")),
        ] {
            if ui.button(label).clicked() {
                add_move = Some(kind);
            }
        }
        // Omborlar orasida ko'chirish (TZ XI.20) — kamida ikkita ombor kerak.
        let many = app.warehouses.len() >= 2;
        if ui
            .add_enabled(many, egui::Button::new(t("wh_add_transfer")))
            .on_hover_text(t("wh_transfer_hint"))
            .on_disabled_hover_text(t("wh_transfer_need_two"))
            .clicked()
        {
            transfer = true;
        }
        if ui
            .button(RichText::new(t("wh_add_writeoff")).color(theme::warn()))
            .clicked()
        {
            add_move = Some(MoveKind::WriteOff);
        }
    });
    ui.label(RichText::new(t("wh_hint")).size(11.0).color(theme::muted()));

    if add_warehouse {
        let n = app.warehouses.len() + 1;
        let id = app.db.insert_warehouse(&Warehouse {
            id: 0,
            project_id: pid,
            name: format!("{} {n}", t("warehouse_new_name")),
            // Birinchi ombor odatda obyektdagi ombor bo'ladi.
            kind: if n == 1 {
                WarehouseKind::Object
            } else {
                WarehouseKind::Temporary
            },
            responsible: String::new(),
            note: String::new(),
        });
        app.reload_modules();
        if id > 0 {
            app.warehouse_filter = Some(id);
        }
    }
    if transfer {
        // Ko'chirish — ikkita bog'langan yozuv: birinchi ombordan chiqim,
        // ikkinchisiga kirim. Shunda ikkala ombor qoldig'i ham to'g'ri qoladi
        // va hujjat raqami bo'yicha juftni topish mumkin.
        let from = app
            .warehouse_filter
            .or_else(|| app.warehouses.first().map(|w| w.id));
        let to = app
            .warehouses
            .iter()
            .map(|w| w.id)
            .find(|id| Some(*id) != from);
        if let (Some(from), Some(to), Some(first)) = (from, to, app.materials.first().map(|m| m.id))
        {
            let n = app
                .stock_moves
                .iter()
                .filter(|m| m.document.starts_with("PER-"))
                .count()
                / 2
                + 1;
            let doc = format!("PER-{n}");
            let name = |id: i64| {
                app.warehouses
                    .iter()
                    .find(|w| w.id == id)
                    .map(|w| w.name.clone())
                    .unwrap_or_default()
            };
            let note = format!("{} → {}", name(from), name(to));
            for (kind, wh) in [(MoveKind::Out, from), (MoveKind::In, to)] {
                app.db.insert_stock_move(&StockMove {
                    id: 0,
                    project_id: pid,
                    material_id: first,
                    date: app.today,
                    kind,
                    qty: 0.0,
                    price: 0.0,
                    document: doc.clone(),
                    counterparty: String::new(),
                    task_id: None,
                    note: note.clone(),
                    warehouse_id: Some(wh),
                    batch_id: None,
                });
            }
            app.reload_modules();
            *tab = 1;
            ui.data_mut(|d| d.insert_temp(egui::Id::new("wh_tab"), 1u8));
        }
    }
    if let Some(kind) = add_move {
        if let Some(first) = app.materials.first().map(|m| m.id) {
            app.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: first,
                date: app.today,
                kind,
                qty: 0.0,
                price: 0.0,
                document: String::new(),
                counterparty: String::new(),
                task_id: None,
                note: String::new(),
                // Yangi yozuv tanlangan omborga tushadi.
                warehouse_id: app.warehouse_filter,
                batch_id: None,
            });
            app.reload_modules();
            *tab = 1;
            ui.data_mut(|d| d.insert_temp(egui::Id::new("wh_tab"), 1u8));
        }
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let lines = app.stock_in(app.warehouse_filter);
    let value: f64 = lines.iter().map(|l| l.value).sum();
    let below = lines.iter().filter(|l| l.below_min).count();
    let negative = lines.iter().filter(|l| l.negative).count();
    let reserved = lines.iter().filter(|l| l.reserved > 0.0).count();

    // Oxirgi 30 kundagi harakatlar.
    let since = app.today - chrono::Duration::days(30);
    let recent = app
        .stock_moves
        .iter()
        .filter(|m| m.date >= since)
        .filter(|m| {
            app.warehouse_filter
                .is_none_or(|w| m.warehouse_id == Some(w))
        })
        .count();

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_stock_value"),
                money(value),
                t("kpi_stock_value_hint"),
                theme::text(),
            ),
            stat(
                t("kpi_below_min"),
                below.to_string(),
                t("kpi_below_min_hint"),
                if below == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_reserved"),
                reserved.to_string(),
                t("kpi_reserved_hint"),
                if reserved == 0 {
                    theme::muted()
                } else {
                    theme::accent()
                },
            ),
            stat(
                t("kpi_negative"),
                negative.to_string(),
                t("kpi_negative_hint"),
                if negative == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_moves_30"),
                recent.to_string(),
                t("kpi_moves_30_hint"),
                theme::accent(),
            ),
        ],
    );
}

// ================================================================ Qoldiqlar

fn balance_tab(ui: &mut egui::Ui, app: &mut App) {
    let lines = app.stock_in(app.warehouse_filter);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wh_balance")
                .num_columns(11)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    // Matn ustunlari chapga, raqamlar o'ngga tekislanadi —
                    // shunda qiymatlarni ko'z bilan solishtirish oson.
                    head_l(ui, 240.0, t("col_material"));
                    head_l(ui, 56.0, t("col_unit"));
                    head_r(ui, 90.0, t("col_balance"));
                    head_r(ui, 90.0, t("col_reserved"));
                    head_r(ui, 90.0, t("col_available"));
                    head_l(ui, 130.0, "");
                    head_r(ui, 86.0, t("col_min_stock"));
                    head_r(ui, 110.0, t("col_unit_price"));
                    head_r(ui, 140.0, t("col_stock_value"));
                    head_r(ui, 160.0, t("col_in_out"));
                    head_r(ui, 100.0, t("col_last_move"));
                    ui.end_row();

                    for m in &app.materials {
                        let Some(l) = lines.iter().find(|x| x.material_id == m.id) else {
                            continue;
                        };
                        cell_l(
                            ui,
                            240.0,
                            RichText::new(super::issues::truncate(&material_label(app, m.id), 32))
                                .size(12.5),
                        );
                        cell_l(
                            ui,
                            56.0,
                            RichText::new(&m.unit).size(12.0).color(theme::muted()),
                        );
                        let color = if l.negative {
                            theme::danger()
                        } else if l.below_min {
                            theme::warn()
                        } else {
                            theme::text()
                        };
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(trim_num(l.balance))
                                .size(13.0)
                                .strong()
                                .color(color),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(if l.reserved > 0.0 {
                                trim_num(l.reserved)
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.0)
                            .color(if l.reserved > 0.0 {
                                theme::accent()
                            } else {
                                theme::muted()
                            }),
                        );
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(trim_num(l.available)).size(12.5).color(color),
                        );
                        stock_bar(ui, l.available, m.min_stock, 126.0);
                        cell_r(
                            ui,
                            86.0,
                            RichText::new(if m.min_stock > 0.0 {
                                trim_num(m.min_stock)
                            } else {
                                t("dash").to_string()
                            })
                            .size(12.0)
                            .color(theme::muted()),
                        );
                        // Birlik narxi kirimlarning vaznlangan o'rtachasi — qoldiq
                        // qiymati qayerdan chiqqani ko'rinib tursin.
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(money(l.unit_price))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        cell_r(ui, 140.0, RichText::new(money(l.value)).size(12.5));
                        // Kirim (qaytarish bilan) va chiqim — qoldiq qayerdan
                        // chiqqani ko'rinib tursin.
                        cell_r(
                            ui,
                            160.0,
                            RichText::new(format!(
                                "+{} / -{}",
                                trim_num(l.incoming + l.returned),
                                trim_num(l.outgoing + l.written_off)
                            ))
                            .size(11.5)
                            .color(theme::muted()),
                        );
                        // Uzoq turgan material (TZ XI.41, XI.43): qoldiq bor,
                        // lekin 90 kundan beri qimirlamagan — pul shu yerda
                        // qotib qolgan.
                        let idle = l.balance > 0.0
                            && l.last_move
                                .is_none_or(|d| (app.today - d).num_days() > STALE_DAYS);
                        let r = cell_r(
                            ui,
                            100.0,
                            RichText::new(match l.last_move {
                                Some(d) => d.format("%d.%m.%y").to_string(),
                                None => t("dash").to_string(),
                            })
                            .size(11.5)
                            .monospace()
                            .color(if idle {
                                theme::warn()
                            } else {
                                theme::muted()
                            }),
                        );
                        if idle {
                            r.on_hover_text(t("stock_idle_hint"));
                        }
                        ui.end_row();
                    }
                });

            // Manfiy qoldiq — hujjatlarda xato borligini bildiradi.
            if lines.iter().any(|l| l.negative) {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t("wh_negative_warning"))
                        .size(11.5)
                        .color(theme::danger()),
                );
            }
        });
}

// ================================================================ Harakatlar

fn moves_tab(ui: &mut egui::Ui, app: &mut App) {
    let visible: Vec<i64> = app
        .stock_moves
        .iter()
        .filter(|m| {
            app.warehouse_filter
                .is_none_or(|w| m.warehouse_id == Some(w))
        })
        .map(|m| m.id)
        .collect();

    if visible.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("wh_moves_empty")).color(theme::muted()));
        });
        return;
    }

    // Nazoratsiz hisobdan chiqarishni ushlaymiz (TZ XI.23): sababi yozilmagan
    // hisobdan chiqarish — bu material qayerga ketgani noma'lum degani.
    let blind = app
        .stock_moves
        .iter()
        .filter(|m| visible.contains(&m.id))
        .filter(|m| m.kind == MoveKind::WriteOff && m.note.trim().is_empty())
        .count();
    if blind > 0 {
        ui.label(
            RichText::new(format!("{}: {blind}", t("writeoff_blind")))
                .size(11.5)
                .color(theme::warn()),
        );
        ui.add_space(6.0);
    }

    let mut edited: Option<StockMove> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wh_moves")
                .num_columns(12)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_date"));
                    head_l(ui, 150.0, t("col_kind"));
                    head_l(ui, 200.0, t("col_material"));
                    head_l(ui, 150.0, t("col_warehouse"));
                    head_l(ui, 130.0, t("col_batch"));
                    head_r(ui, 86.0, t("col_qty"));
                    head_r(ui, 110.0, t("col_price"));
                    head_r(ui, 120.0, t("col_sum"));
                    head_l(ui, 120.0, t("col_document"));
                    head_l(ui, 150.0, t("col_counterparty"));
                    head_l(ui, 200.0, t("col_task"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for id in &visible {
                        let Some(src) = app.stock_moves.iter().find(|x| x.id == *id) else {
                            continue;
                        };
                        let mut m = src.clone();
                        let mut changed = false;

                        changed |=
                            super::passport::date_edit(ui, &format!("wh{}", m.id), &mut m.date);

                        // Harakat turi rangli: kirim va qaytarish yashil, chiqim
                        // ko'k, hisobdan chiqarish sariq — jurnal bir qarashda o'qiladi.
                        let kind_color = match m.kind {
                            MoveKind::In | MoveKind::Return => theme::ok(),
                            MoveKind::Out => theme::accent(),
                            MoveKind::WriteOff => theme::warn(),
                            MoveKind::ToSupplier => theme::danger(),
                        };
                        egui::ComboBox::from_id_salt(("wh_kind", m.id))
                            .selected_text(RichText::new(m.kind.label()).color(kind_color))
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for k in MoveKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut m.kind, *k, k.label()).changed();
                                }
                            });

                        changed |=
                            material_picker(ui, app, ("wh_mat", m.id), &mut m.material_id, 200.0);
                        changed |= warehouse_picker(ui, app, ("wh_wh", m.id), &mut m.warehouse_id);
                        changed |= batch_picker(ui, app, m.id, m.material_id, &mut m.batch_id);

                        changed |= ui
                            .add_sized(
                                [86.0, 22.0],
                                egui::DragValue::new(&mut m.qty).speed(1.0).range(0.0..=1e9),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [110.0, 22.0],
                                egui::DragValue::new(&mut m.price)
                                    .speed(100.0)
                                    .range(0.0..=1e12),
                            )
                            .changed();
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(money(m.qty * m.price))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        changed |= ui
                            .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut m.document))
                            .changed();
                        changed |= ui
                            .add_sized(
                                [150.0, 22.0],
                                egui::TextEdit::singleline(&mut m.counterparty),
                            )
                            .changed();
                        changed |= task_picker(ui, app, ("wh_task", m.id), &mut m.task_id, 200.0);

                        if ui
                            .small_button(RichText::new("x").color(theme::danger()))
                            .clicked()
                        {
                            removed = Some(m.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(m);
                        }
                    }
                });
        });

    if let Some(m) = edited {
        app.db.update_stock_move(&m);
        if let Some(slot) = app.stock_moves.iter_mut().find(|x| x.id == m.id) {
            *slot = m;
        }
    }
    if let Some(id) = removed {
        app.db.del("stock_move", id);
        app.reload_modules();
    }
}

/// Ombor tanlash. O'zgargan bo'lsa `true`.
fn warehouse_picker(
    ui: &mut egui::Ui,
    app: &App,
    salt: impl std::hash::Hash,
    cur: &mut Option<i64>,
) -> bool {
    let mut changed = false;
    let text = cur
        .and_then(|id| app.warehouses.iter().find(|w| w.id == id))
        .map(|w| w.name.clone())
        .unwrap_or_else(|| t("dash").to_string());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(super::issues::truncate(&text, 18))
        .width(150.0)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(cur, None, t("dash")).changed();
            for w in &app.warehouses {
                changed |= ui.selectable_value(cur, Some(w.id), &w.name).changed();
            }
        });
    changed
}

/// Partiya tanlash — faqat shu materialning partiyalari ko'rsatiladi.
fn batch_picker(
    ui: &mut egui::Ui,
    app: &App,
    move_id: i64,
    material_id: i64,
    cur: &mut Option<i64>,
) -> bool {
    let mut changed = false;
    let text = cur
        .and_then(|id| app.batches.iter().find(|b| b.id == id))
        .map(|b| b.number.clone())
        .unwrap_or_else(|| t("dash").to_string());
    egui::ComboBox::from_id_salt(("wh_batch", move_id))
        .selected_text(super::issues::truncate(&text, 16))
        .width(130.0)
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(cur, None, t("dash")).changed();
            for b in app.batches.iter().filter(|b| b.material_id == material_id) {
                changed |= ui.selectable_value(cur, Some(b.id), &b.number).changed();
            }
        });
    changed
}

// ================================================================ Partiyalar

fn batches_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else { return };
    let today = app.today;
    let lines = app.batch_lines();
    let mut add = false;
    let mut edited: Option<Batch> = None;
    let mut removed: Option<i64> = None;

    ui.horizontal(|ui| {
        if ui.button(t("add_batch")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("batches_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    if app.batches.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("batches_empty")).color(theme::muted()));
        });
    } else {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("wh_batches")
                    .num_columns(10)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 110.0, t("col_batch"));
                        head_l(ui, 200.0, t("col_material"));
                        head_l(ui, 110.0, t("col_received"));
                        head_r(ui, 90.0, t("col_balance"));
                        head_r(ui, 130.0, t("col_in_out"));
                        head_l(ui, 150.0, t("col_supplier"));
                        head_l(ui, 210.0, t("col_cert"));
                        head_l(ui, 130.0, t("col_expires"));
                        head_l(ui, 140.0, t("col_fefo"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.batches {
                            let mut b = src.clone();
                            let mut changed = false;
                            let line = lines.iter().find(|l| l.batch_id == b.id);

                            changed |= ui
                                .add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut b.number))
                                .changed();
                            changed |= material_picker(
                                ui,
                                app,
                                ("bt_mat", b.id),
                                &mut b.material_id,
                                200.0,
                            );
                            changed |= super::passport::date_edit(
                                ui,
                                &format!("btr{}", b.id),
                                &mut b.received,
                            );
                            cell_r(
                                ui,
                                90.0,
                                RichText::new(trim_num(line.map(|l| l.balance).unwrap_or(0.0)))
                                    .size(12.5)
                                    .strong(),
                            );
                            // Kirim va chiqim — qoldiq qayerdan chiqqani ko'rinsin.
                            cell_r(
                                ui,
                                130.0,
                                RichText::new(match line {
                                    Some(l) => format!(
                                        "+{} / -{}",
                                        trim_num(l.incoming),
                                        trim_num(l.outgoing)
                                    ),
                                    None => String::new(),
                                })
                                .size(11.5)
                                .color(theme::muted()),
                            );
                            changed |= ui
                                .add_sized(
                                    [150.0, 22.0],
                                    egui::TextEdit::singleline(&mut b.supplier),
                                )
                                .changed();
                            // Sertifikat: raqami va amal qilish muddati (TZ XI.10).
                            // Muddati o'tgan sertifikat qizil bo'ladi — bunday
                            // partiyani ishga bermaslik kerak.
                            ui.horizontal(|ui| {
                                let stale = b.cert_until.is_some_and(|d| d < today);
                                let mut edit = egui::TextEdit::singleline(&mut b.cert_no);
                                if stale {
                                    edit = edit.text_color(theme::danger());
                                }
                                let r = ui.add_sized([100.0, 22.0], edit);
                                changed |= r.changed();
                                if stale {
                                    r.on_hover_text(t("cert_stale_hint"));
                                }
                                let mut has = b.cert_until.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    b.cert_until = has.then(|| today + chrono::Duration::days(365));
                                    changed = true;
                                }
                                if let Some(mut d) = b.cert_until {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("btc{}", b.id),
                                        &mut d,
                                    ) {
                                        b.cert_until = Some(d);
                                        changed = true;
                                    }
                                }
                            });

                            // Yaroqlilik muddati — belgilanmagan bo'lishi mumkin.
                            ui.horizontal(|ui| {
                                let mut has = b.expires.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    b.expires = has.then(|| today + chrono::Duration::days(180));
                                    changed = true;
                                }
                                if let Some(mut d) = b.expires {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("bte{}", b.id),
                                        &mut d,
                                    ) {
                                        b.expires = Some(d);
                                        changed = true;
                                    }
                                }
                            });

                            // FEFO: muddati birinchi tugaydigan partiya birinchi ishlatiladi.
                            let (text, color) = match line {
                                Some(l) if l.expired => (t("batch_expired"), theme::danger()),
                                _ if b.cert_until.is_some_and(|d| d < today) => {
                                    (t("cert_stale"), theme::danger())
                                }
                                Some(l) if l.next_to_use => (t("batch_next"), theme::accent()),
                                _ => ("", theme::muted()),
                            };
                            cell_l(ui, 140.0, RichText::new(text).size(11.0).color(color));

                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(b.id);
                            }
                            ui.end_row();

                            if changed {
                                edited = Some(b);
                            }
                        }
                    });
            });
    }

    if add {
        if let Some(first) = app.materials.first().map(|m| m.id) {
            let n = app.batches.len() + 1;
            app.db.insert_batch(&Batch {
                id: 0,
                project_id: pid,
                material_id: first,
                number: format!("P-{n:03}"),
                received: today,
                supplier: String::new(),
                cert_no: String::new(),
                cert_until: None,
                expires: None,
                note: String::new(),
            });
            app.reload_modules();
        }
    }
    if let Some(b) = edited {
        app.db.update_batch(&b);
        if let Some(slot) = app.batches.iter_mut().find(|x| x.id == b.id) {
            *slot = b;
        }
    }
    if let Some(id) = removed {
        app.db.del("batch", id);
        app.reload_modules();
    }
}

// ================================================================ Rezerv

fn reserve_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else { return };
    let today = app.today;
    let stock = app.stock();
    let mut add = false;
    let mut edited: Option<Reservation> = None;
    let mut removed: Option<i64> = None;

    ui.horizontal(|ui| {
        if ui.button(t("add_reservation")).clicked() {
            add = true;
        }
        ui.label(
            RichText::new(t("reserve_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(6.0);

    if app.reservations.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("reserve_empty")).color(theme::muted()));
        });
    } else {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("wh_reserve")
                    .num_columns(7)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 220.0, t("col_material"));
                        head_r(ui, 90.0, t("col_qty"));
                        head_l(ui, 220.0, t("col_task"));
                        head_l(ui, 110.0, t("col_date"));
                        head_l(ui, 140.0, t("col_until"));
                        head_l(ui, 160.0, t("col_available"));
                        head_l(ui, 24.0, "");
                        ui.end_row();

                        for src in &app.reservations {
                            let mut r = src.clone();
                            let mut changed = false;

                            changed |= material_picker(
                                ui,
                                app,
                                ("rs_mat", r.id),
                                &mut r.material_id,
                                220.0,
                            );
                            changed |= ui
                                .add_sized(
                                    [90.0, 22.0],
                                    egui::DragValue::new(&mut r.qty).speed(1.0).range(0.0..=1e9),
                                )
                                .changed();
                            changed |=
                                task_picker(ui, app, ("rs_task", r.id), &mut r.task_id, 220.0);
                            changed |=
                                super::passport::date_edit(ui, &format!("rs{}", r.id), &mut r.date);

                            // Muddat: undan keyin rezerv o'z-o'zidan kuchini yo'qotadi.
                            ui.horizontal(|ui| {
                                let mut has = r.until.is_some();
                                if ui.checkbox(&mut has, "").changed() {
                                    r.until = has.then(|| today + chrono::Duration::days(30));
                                    changed = true;
                                }
                                if let Some(mut d) = r.until {
                                    if super::passport::date_edit(
                                        ui,
                                        &format!("rsu{}", r.id),
                                        &mut d,
                                    ) {
                                        r.until = Some(d);
                                        changed = true;
                                    }
                                    if d < today {
                                        ui.label(
                                            RichText::new(t("reserve_expired"))
                                                .size(10.5)
                                                .color(theme::muted()),
                                        );
                                    }
                                }
                            });

                            // Rezerv qoldiqdan oshib ketgan bo'lsa — buni ko'rsatamiz.
                            let line = stock.iter().find(|l| l.material_id == r.material_id);
                            let (text, color) = match line {
                                Some(l) if l.available < 0.0 => (
                                    format!("{} {}", t("reserve_over"), trim_num(-l.available)),
                                    theme::danger(),
                                ),
                                Some(l) => (trim_num(l.available), theme::muted()),
                                None => (t("dash").to_string(), theme::muted()),
                            };
                            cell_l(ui, 160.0, RichText::new(text).size(11.5).color(color));

                            if ui
                                .small_button(RichText::new("x").color(theme::danger()))
                                .clicked()
                            {
                                removed = Some(r.id);
                            }
                            ui.end_row();

                            if changed {
                                edited = Some(r);
                            }
                        }
                    });
            });
    }

    if add {
        if let Some(first) = app.materials.first().map(|m| m.id) {
            app.db.insert_reservation(&Reservation {
                id: 0,
                project_id: pid,
                material_id: first,
                task_id: None,
                qty: 0.0,
                date: today,
                until: None,
                note: String::new(),
            });
            app.reload_modules();
        }
    }
    if let Some(r) = edited {
        app.db.update_reservation(&r);
        if let Some(slot) = app.reservations.iter_mut().find(|x| x.id == r.id) {
            *slot = r;
        }
    }
    if let Some(id) = removed {
        app.db.del("reservation", id);
        app.reload_modules();
    }
}

// ================================================================ Inventarizatsiya

fn inventory_tab(ui: &mut egui::Ui, app: &mut App) {
    let Some(pid) = app.current else { return };
    let sel_key = egui::Id::new("wh_inv_sel");
    let mut selected = ui.data(|d| d.get_temp::<Option<i64>>(sel_key)).flatten();
    if selected.is_none_or(|id| !app.inventories.iter().any(|v| v.id == id)) {
        selected = app.inventories.first().map(|v| v.id);
    }

    let mut start = false;
    let mut close: Option<i64> = None;
    let label = |v: &Inventory| {
        format!(
            "{} · {}",
            v.date.format("%d.%m.%Y"),
            if v.closed {
                t("inv_closed")
            } else {
                t("inv_open")
            }
        )
    };

    ui.horizontal_wrapped(|ui| {
        if ui
            .button(t("start_inventory"))
            .on_hover_text(t("start_inventory_hint"))
            .clicked()
        {
            start = true;
        }
        if !app.inventories.is_empty() {
            let text = selected
                .and_then(|id| app.inventories.iter().find(|v| v.id == id))
                .map(label)
                .unwrap_or_default();
            egui::ComboBox::from_id_salt("inv_pick")
                .selected_text(text)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for v in &app.inventories {
                        ui.selectable_value(&mut selected, Some(v.id), label(v));
                    }
                });
        }
        ui.label(
            RichText::new(t("inventory_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.data_mut(|d| d.insert_temp(sel_key, selected));
    ui.add_space(6.0);

    let Some(inv_id) = selected else {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("inventory_empty")).color(theme::muted()));
        });
        if start {
            new_inventory(app, pid);
        }
        return;
    };
    let closed = app
        .inventories
        .iter()
        .find(|v| v.id == inv_id)
        .map(|v| v.closed)
        .unwrap_or(false);

    let diffs = crate::checks::inventory_diffs(&app.inventory_lines, inv_id);
    let short: f64 = diffs.iter().filter(|d| d.diff < 0.0).map(|d| -d.diff).sum();
    let over: f64 = diffs.iter().filter(|d| d.diff > 0.0).map(|d| d.diff).sum();

    ui.horizontal_wrapped(|ui| {
        // Har bir farqning hisob va fakt qiymati — sichqoncha ostida.
        let detail = diffs
            .iter()
            .map(|d| {
                format!(
                    "{}: {} → {}",
                    material_label(app, d.material_id),
                    trim_num(d.book),
                    trim_num(d.fact)
                )
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        ui.label(
            RichText::new(format!("{}: {}", t("inv_diff_count"), diffs.len()))
                .size(12.0)
                .color(if diffs.is_empty() {
                    theme::ok()
                } else {
                    theme::warn()
                }),
        )
        .on_hover_text(if detail.is_empty() {
            t("inv_no_diff").to_string()
        } else {
            detail
        });
        if short > 0.0 {
            ui.label(
                RichText::new(format!("{}: {}", t("inv_short"), trim_num(short)))
                    .size(12.0)
                    .color(theme::danger()),
            );
        }
        if over > 0.0 {
            ui.label(
                RichText::new(format!("{}: {}", t("inv_over"), trim_num(over)))
                    .size(12.0)
                    .color(theme::warn()),
            );
        }
        if !closed
            && ui
                .button(RichText::new(t("close_inventory")).color(theme::warn()))
                .on_hover_text(t("close_inventory_hint"))
                .clicked()
        {
            close = Some(inv_id);
        }
        if closed {
            ui.label(
                RichText::new(t("inv_closed_note"))
                    .size(11.5)
                    .color(theme::muted()),
            );
        }
    });
    ui.add_space(6.0);

    let mut edited: Option<InventoryLine> = None;
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wh_inv")
                .num_columns(5)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 260.0, t("col_material"));
                    head_r(ui, 110.0, t("col_book"));
                    head_r(ui, 110.0, t("col_fact"));
                    head_r(ui, 110.0, t("col_diff"));
                    head_l(ui, 200.0, t("col_note"));
                    ui.end_row();

                    for src in app
                        .inventory_lines
                        .iter()
                        .filter(|l| l.inventory_id == inv_id)
                    {
                        let mut l = src.clone();
                        let mut changed = false;

                        cell_l(
                            ui,
                            260.0,
                            RichText::new(super::issues::truncate(
                                &material_label(app, l.material_id),
                                34,
                            ))
                            .size(12.5),
                        );
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(trim_num(l.book))
                                .size(12.5)
                                .color(theme::muted()),
                        );
                        ui.add_enabled_ui(!closed, |ui| {
                            changed |= ui
                                .add_sized(
                                    [110.0, 22.0],
                                    egui::DragValue::new(&mut l.fact)
                                        .speed(1.0)
                                        .range(0.0..=1e9),
                                )
                                .changed();
                        });
                        let d = l.diff();
                        cell_r(
                            ui,
                            110.0,
                            RichText::new(if d.abs() < 0.0001 {
                                t("dash").to_string()
                            } else {
                                format!("{}{}", if d > 0.0 { "+" } else { "" }, trim_num(d))
                            })
                            .size(12.5)
                            .strong()
                            .color(if d < 0.0 {
                                theme::danger()
                            } else if d > 0.0 {
                                theme::warn()
                            } else {
                                theme::ok()
                            }),
                        );
                        ui.add_enabled_ui(!closed, |ui| {
                            changed |= ui
                                .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut l.note))
                                .changed();
                        });
                        ui.end_row();

                        if changed {
                            edited = Some(l);
                        }
                    }
                });
        });

    if start {
        new_inventory(app, pid);
    }
    if let Some(l) = edited {
        app.db.update_inventory_line(&l);
        if let Some(slot) = app.inventory_lines.iter_mut().find(|x| x.id == l.id) {
            *slot = l;
        }
    }
    if let Some(id) = close {
        close_inventory(app, pid, id);
    }
}

/// Yangi inventarizatsiya: hisobdagi qoldiq shu paytda yozib qo'yiladi.
fn new_inventory(app: &mut App, pid: i64) {
    let id = app.db.insert_inventory(&Inventory {
        id: 0,
        project_id: pid,
        warehouse_id: app.warehouse_filter,
        date: app.today,
        responsible: String::new(),
        closed: false,
        note: String::new(),
    });
    if id == 0 {
        return;
    }
    // Har bir material uchun qator: hisobdagi qoldiq «book» ga tushadi.
    for l in app.stock_in(app.warehouse_filter) {
        app.db.insert_inventory_line(&InventoryLine {
            id: 0,
            inventory_id: id,
            material_id: l.material_id,
            book: l.balance,
            fact: l.balance,
            note: String::new(),
        });
    }
    app.reload_modules();
    app.notify(t("inventory_started").to_string());
}

/// Inventarizatsiyani yopish: farqlar tuzatuvchi harakatga aylanadi.
///
/// Kamomad hisobdan chiqarish, ortiqcha esa kirim sifatida yoziladi — shunda
/// qoldiq harakatlardan hisoblanishi buzilmaydi va farq hujjatda qoladi.
fn close_inventory(app: &mut App, pid: i64, inv_id: i64) {
    let diffs = crate::checks::inventory_diffs(&app.inventory_lines, inv_id);
    let today = app.today;
    let warehouse = app
        .inventories
        .iter()
        .find(|v| v.id == inv_id)
        .and_then(|v| v.warehouse_id);
    let document = format!("{} #{inv_id}", t("inv_document"));

    for d in &diffs {
        app.db.insert_stock_move(&StockMove {
            id: 0,
            project_id: pid,
            material_id: d.material_id,
            date: today,
            kind: if d.diff < 0.0 {
                MoveKind::WriteOff
            } else {
                MoveKind::In
            },
            qty: d.diff.abs(),
            price: 0.0,
            document: document.clone(),
            counterparty: String::new(),
            task_id: None,
            note: t("inv_adjust_note").to_string(),
            warehouse_id: warehouse,
            batch_id: None,
        });
    }
    if let Some(v) = app.inventories.iter_mut().find(|v| v.id == inv_id) {
        v.closed = true;
        let copy = v.clone();
        app.db.update_inventory(&copy);
    }
    app.reload_modules();
    app.notify(format!("{} {}", t("inventory_closed"), diffs.len()));
}

// ================================================================ Hujayralar

/// Chapga tekislangan matn hujayrasi.
pub fn cell_l(ui: &mut egui::Ui, w: f32, text: RichText) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            // Grid ustuni to'liq kenglikni egallashi kerak — aks holda
            // keyingi ustun ustiga chiqib ketadi.
            ui.set_min_width(w);
            ui.add(egui::Label::new(text).truncate())
        },
    )
    .inner
}

/// O'ngga tekislangan raqam hujayrasi.
pub fn cell_r(ui: &mut egui::Ui, w: f32, text: RichText) -> egui::Response {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.set_min_width(w);
            ui.add(egui::Label::new(text).truncate())
        },
    )
    .inner
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
