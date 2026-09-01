//! «Ombor» ekrani (TZ XI).
//!
//! Ikki ko'rinish: **qoldiqlar** — har bir material bo'yicha joriy holat,
//! minimal zaxira va qiymat; **harakatlar** — kirim, chiqim va hisobdan
//! chiqarish jurnali. Qoldiq harakatlardan hisoblanadi, alohida saqlanmaydi —
//! shunda hujjat va qoldiq hech qachon bir-biriga zid bo'lmaydi.

use super::materials::{material_label, material_picker, stock_bar, trim_num};
use super::*;
use crate::domain::{MoveKind, StockMove};

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
            ui.label(RichText::new(t("wh_no_materials")).color(theme::muted()).size(16.0));
            ui.add_space(6.0);
            ui.label(RichText::new(t("wh_no_materials_hint")).color(theme::muted()).size(12.0));
        });
        return;
    }

    let tab_key = egui::Id::new("wh_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);

    let mut add_move: Option<MoveKind> = None;
    ui.horizontal(|ui| {
        // Uch xil harakat — har biri o'z tugmasi bilan, tur adashmasin.
        if ui.button(t("wh_add_in")).clicked() {
            add_move = Some(MoveKind::In);
        }
        if ui.button(t("wh_add_out")).clicked() {
            add_move = Some(MoveKind::Out);
        }
        if ui
            .button(RichText::new(t("wh_add_writeoff")).color(theme::warn()))
            .clicked()
        {
            add_move = Some(MoveKind::WriteOff);
        }
        ui.label(
            RichText::new(t("wh_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    kpi_row(ui, app);
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        for (i, label) in [(0u8, t("wh_tab_balance")), (1, t("wh_tab_moves"))] {
            if ui.selectable_label(tab == i, label).clicked() {
                tab = i;
            }
        }
    });
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    if tab == 1 {
        moves_tab(ui, app);
    } else {
        balance_tab(ui, app);
    }

    if let Some(kind) = add_move {
        if let (Some(pid), Some(first)) = (app.current, app.materials.first().map(|m| m.id)) {
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
            });
            app.reload_modules();
            // Yangi yozuv harakatlar ro'yxatida — o'sha yerga o'tamiz.
            ui.data_mut(|d| d.insert_temp(tab_key, 1u8));
        }
    }
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let lines = app.stock();
    let value: f64 = lines.iter().map(|l| l.value).sum();
    let below = lines.iter().filter(|l| l.below_min).count();
    let negative = lines.iter().filter(|l| l.negative).count();

    // Oxirgi 30 kundagi harakatlar.
    let since = app.today - chrono::Duration::days(30);
    let recent = app.stock_moves.iter().filter(|m| m.date >= since).count();

    ui.horizontal_wrapped(|ui| {
        stat_card(
            ui,
            t("kpi_stock_value"),
            money(value),
            t("kpi_stock_value_hint"),
            theme::text(),
        );
        stat_card(
            ui,
            t("kpi_below_min"),
            below.to_string(),
            t("kpi_below_min_hint"),
            if below == 0 { theme::ok() } else { theme::danger() },
        );
        stat_card(
            ui,
            t("kpi_negative"),
            negative.to_string(),
            t("kpi_negative_hint"),
            if negative == 0 { theme::ok() } else { theme::danger() },
        );
        stat_card(
            ui,
            t("kpi_moves_30"),
            recent.to_string(),
            t("kpi_moves_30_hint"),
            theme::accent(),
        );
    });
}

// ================================================================ Qoldiqlar

fn balance_tab(ui: &mut egui::Ui, app: &mut App) {
    let lines = app.stock();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wh_balance")
                .num_columns(9)
                .spacing([10.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    // Matn ustunlari chapga, raqamlar o'ngga tekislanadi —
                    // shunda qiymatlarni ko'z bilan solishtirish oson.
                    head_l(ui, 260.0, t("col_material"));
                    head_l(ui, 56.0, t("col_unit"));
                    head_r(ui, 96.0, t("col_balance"));
                    head_l(ui, 130.0, "");
                    head_r(ui, 86.0, t("col_min_stock"));
                    head_r(ui, 110.0, t("col_unit_price"));
                    head_r(ui, 140.0, t("col_stock_value"));
                    head_r(ui, 150.0, t("col_in_out"));
                    head_r(ui, 100.0, t("col_last_move"));
                    ui.end_row();

                    for m in &app.materials {
                        let Some(l) = lines.iter().find(|x| x.material_id == m.id) else {
                            continue;
                        };
                        cell_l(
                            ui,
                            260.0,
                            RichText::new(super::issues::truncate(&material_label(app, m.id), 36))
                                .size(12.5),
                        );
                        cell_l(ui, 56.0, RichText::new(&m.unit).size(12.0).color(theme::muted()));
                        let color = if l.negative {
                            theme::danger()
                        } else if l.below_min {
                            theme::warn()
                        } else {
                            theme::text()
                        };
                        cell_r(
                            ui,
                            96.0,
                            RichText::new(trim_num(l.balance)).size(13.0).strong().color(color),
                        );
                        stock_bar(ui, l.balance, m.min_stock, 126.0);
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
                            RichText::new(money(l.unit_price)).size(12.0).color(theme::muted()),
                        );
                        cell_r(ui, 140.0, RichText::new(money(l.value)).size(12.5));
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(format!(
                                "+{} / -{}",
                                trim_num(l.incoming),
                                trim_num(l.outgoing + l.written_off)
                            ))
                            .size(11.5)
                            .color(theme::muted()),
                        );
                        cell_r(
                            ui,
                            100.0,
                            RichText::new(match l.last_move {
                                Some(d) => d.format("%d.%m.%y").to_string(),
                                None => t("dash").to_string(),
                            })
                            .size(11.5)
                            .monospace()
                            .color(theme::muted()),
                        );
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
    if app.stock_moves.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t("wh_moves_empty")).color(theme::muted()));
        });
        return;
    }

    let mut edited: Option<StockMove> = None;
    let mut removed: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("wh_moves")
                .num_columns(10)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_date"));
                    head_l(ui, 140.0, t("col_kind"));
                    head_l(ui, 230.0, t("col_material"));
                    head_r(ui, 86.0, t("col_qty"));
                    head_r(ui, 110.0, t("col_price"));
                    head_r(ui, 120.0, t("col_sum"));
                    head_l(ui, 120.0, t("col_document"));
                    head_l(ui, 150.0, t("col_counterparty"));
                    head_l(ui, 200.0, t("col_task"));
                    head_l(ui, 24.0, "");
                    ui.end_row();

                    for mv in &app.stock_moves {
                        let mut m = mv.clone();
                        let mut changed = false;

                        changed |= super::passport::date_edit(
                            ui,
                            &format!("wh{}", m.id),
                            &mut m.date,
                        );

                        // Harakat turi rangli: kirim yashil, chiqim ko'k, hisobdan
                        // chiqarish sariq — jurnal bir qarashda o'qiladi.
                        let kind_color = match m.kind {
                            MoveKind::In => theme::ok(),
                            MoveKind::Out => theme::accent(),
                            MoveKind::WriteOff => theme::warn(),
                        };
                        egui::ComboBox::from_id_salt(("wh_kind", m.id))
                            .selected_text(RichText::new(m.kind.label()).color(kind_color))
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for k in MoveKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut m.kind, *k, k.label()).changed();
                                }
                            });

                        changed |= material_picker(ui, app, ("wh_mat", m.id), &mut m.material_id, 230.0);
                        changed |= ui
                            .add_sized(
                                [86.0, 22.0],
                                egui::DragValue::new(&mut m.qty).speed(1.0).range(0.0..=1e9),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [110.0, 22.0],
                                egui::DragValue::new(&mut m.price).speed(100.0).range(0.0..=1e12),
                            )
                            .changed();
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(money(m.qty * m.price)).size(12.0).color(theme::muted()),
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

// ================================================================ Hujayralar

/// Chapga tekislangan matn hujayrasi.
pub fn cell_l(ui: &mut egui::Ui, w: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            // Grid ustuni to'liq kenglikni egallashi kerak — aks holda
            // keyingi ustun ustiga chiqib ketadi.
            ui.set_min_width(w);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

/// O'ngga tekislangan raqam hujayrasi.
pub fn cell_r(ui: &mut egui::Ui, w: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        egui::vec2(w, 18.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            // Grid ustuni to'liq kenglikni egallashi kerak — aks holda
            // keyingi ustun ustiga chiqib ketadi.
            ui.set_min_width(w);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
