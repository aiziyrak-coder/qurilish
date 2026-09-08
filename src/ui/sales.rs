//! «Sotuv — shaxmatka» ekrani (XIX).
//!
//! Shaxmatka — bino kesimi: vertikal o'q qavatlar (yuqoridan pastga), gorizontal
//! o'q qavatdagi kvartiralar. Har katakning rangi holatni bildiradi, shuning
//! uchun butun blokning sotuv holati bir qarashda ko'rinadi.
//!
//! Katak bosilganda o'ng panelda kvartira kartochkasi ochiladi — o'sha yerdan
//! shartnoma ochiladi va shartnomalar ekraniga o'tiladi.

use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{Block, Deal, DealStatus, PayKind, Unit, UnitKind, UnitStatus};
use crate::sales;
use egui::{vec2, Sense};

const CELL_W: f32 = 92.0;
const CELL_H: f32 = 56.0;
const FLOOR_W: f32 = 44.0;
/// Yon kartochkaning kengligi.
const CARD_W: f32 = 360.0;

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

    toolbar(ui, app, pid);
    ui.add_space(8.0);
    kpi_row(ui, app);
    ui.add_space(10.0);

    if app.blocks.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("sales_no_block"))
                    .color(theme::muted())
                    .size(15.0),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(t("sales_no_block_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    let tab_key = egui::Id::new("sales_tab");
    let mut tab = ui.data(|d| d.get_temp::<u8>(tab_key)).unwrap_or(0);
    super::tab_row(
        ui,
        &mut tab,
        &[(0, t("sales_tab_board")), (1, t("sales_tab_list"))],
        &[(2, t("sales_tab_review"))],
    );
    // QR yorliq: kvartira eshigiga osiladi, telefon bilan o'qilganda
    // o'sha kvartira sahifasi ochiladi.
    if tab == 1 && !app.units.is_empty() {
        ui.add_space(4.0);
        if ui
            .button(t("qr_labels"))
            .on_hover_text(t("qr_labels_hint"))
            .clicked()
        {
            let rows: Vec<(String, String, String)> = app
                .units
                .iter()
                .map(|u| {
                    (
                        u.number.clone(),
                        format!("{} {}", t("col_unit"), u.number),
                        format!(
                            "{} {} · {} m² · {}",
                            u.floor,
                            t("floor_short"),
                            super::materials::trim_num(u.area),
                            u.kind.label()
                        ),
                    )
                })
                .collect();
            app.save_labels(crate::qr::Kind::Unit, rows);
        }
    }
    ui.data_mut(|d| d.insert_temp(tab_key, tab));
    ui.add_space(8.0);

    match tab {
        1 => list_tab(ui, app),
        2 => review_tab(ui, app),
        _ => board_tab(ui, app),
    }
}

// ================================================================ Tahlil

/// Sotuv tahlili (TZ XIX): voronka, qavatlar, menejerlar va bronlar.
///
/// Barcha sonlar `sales` modulidan olinadi — shaxmatkadagi rang ham
/// o'sha yerdan chiqadi, shuning uchun ikki ko'rinish bir-biriga zid
/// bo'lishi mumkin emas.
fn review_tab(ui: &mut egui::Ui, app: &mut App) {
    use super::warehouse::{cell_l, cell_r};

    let funnel = crate::sales::funnel(&app.units, &app.deals);
    let floors = crate::sales::by_floor(&app.units, &app.deals);
    let managers = crate::sales::by_manager(&app.deals, &app.payments, app.today);
    let stale = crate::sales::stale_reserves(&app.units, &app.deals, app.today);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ---- Voronka
            stat_row(
                ui,
                vec![
                    stat(
                        t("sl_funnel_total"),
                        funnel.total.to_string(),
                        t("sl_funnel_total_hint"),
                        theme::accent(),
                    ),
                    stat(
                        t("us_free"),
                        funnel.free.to_string(),
                        t("sl_free_hint"),
                        if funnel.free == 0 {
                            theme::ok()
                        } else {
                            theme::text()
                        },
                    ),
                    stat(
                        t("us_reserved"),
                        funnel.reserved.to_string(),
                        t("sl_reserved_hint"),
                        theme::warn(),
                    ),
                    stat(
                        t("us_contract"),
                        funnel.contracted.to_string(),
                        t("sl_contract_hint"),
                        theme::accent(),
                    ),
                    stat(
                        t("us_sold"),
                        funnel.sold.to_string(),
                        t("sl_sold_hint"),
                        theme::ok(),
                    ),
                    stat(
                        t("sl_sold_pct"),
                        format!("{:.0} %", funnel.sold_pct()),
                        t("sl_sold_pct_hint"),
                        theme::ok(),
                    ),
                ],
            );
            ui.add_space(14.0);

            // ---- Muddati o'tgan bronlar
            ui.label(RichText::new(t("sl_stale")).size(13.5).strong());
            ui.label(
                RichText::new(t("sl_stale_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
            ui.add_space(4.0);
            if stale.is_empty() {
                ui.label(
                    RichText::new(t("sl_stale_none"))
                        .size(12.5)
                        .color(theme::ok()),
                );
            } else {
                for r in stale.iter().take(12) {
                    ui.label(
                        RichText::new(format!(
                            "· {} — {} · {} {}",
                            r.number,
                            r.client,
                            r.days,
                            t("days_short")
                        ))
                        .size(12.0)
                        .color(theme::warn()),
                    );
                }
            }
            ui.add_space(14.0);

            // ---- Qavatlar kesimi
            ui.label(RichText::new(t("sl_floors")).size(13.5).strong());
            ui.add_space(4.0);
            egui::Grid::new("sl_floors")
                .num_columns(5)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 90.0, t("col_floor"));
                    head_r(ui, 90.0, t("col_units"));
                    head_r(ui, 90.0, t("col_free"));
                    head_r(ui, 120.0, t("col_area_free"));
                    head_r(ui, 140.0, t("col_price_m2"));
                    ui.end_row();
                    for f in &floors {
                        cell_l(ui, 90.0, RichText::new(f.floor.to_string()).size(12.0));
                        cell_r(ui, 90.0, RichText::new(f.units.to_string()).size(12.0));
                        cell_r(
                            ui,
                            90.0,
                            RichText::new(f.free.to_string())
                                .size(12.0)
                                .color(if f.free == 0 {
                                    theme::ok()
                                } else {
                                    theme::text()
                                }),
                        );
                        cell_r(
                            ui,
                            120.0,
                            RichText::new(super::materials::trim_num(f.area_free)).size(12.0),
                        );
                        cell_r(ui, 140.0, RichText::new(money(f.avg_price_m2)).size(12.0));
                        ui.end_row();
                    }
                });
            ui.add_space(14.0);

            // ---- Menejerlar
            ui.label(RichText::new(t("sl_managers")).size(13.5).strong());
            ui.add_space(4.0);
            if managers.is_empty() {
                ui.label(
                    RichText::new(t("sl_no_deals"))
                        .size(12.5)
                        .color(theme::muted()),
                );
                return;
            }
            egui::Grid::new("sl_managers")
                .num_columns(5)
                .spacing([12.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 190.0, t("col_manager"));
                    head_r(ui, 90.0, t("col_deals"));
                    head_r(ui, 150.0, t("col_total"));
                    head_r(ui, 150.0, t("col_paid"));
                    head_r(ui, 150.0, t("col_debt"));
                    ui.end_row();
                    for m in &managers {
                        cell_l(
                            ui,
                            190.0,
                            RichText::new(if m.manager.is_empty() {
                                t("sl_no_manager").to_string()
                            } else {
                                m.manager.clone()
                            })
                            .size(12.5),
                        );
                        cell_r(ui, 90.0, RichText::new(m.deals.to_string()).size(12.0));
                        cell_r(ui, 150.0, RichText::new(money(m.amount)).size(12.0));
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(m.paid)).size(12.0).color(theme::ok()),
                        );
                        cell_r(
                            ui,
                            150.0,
                            RichText::new(money(m.debt))
                                .size(12.0)
                                .color(if m.debt > 0.0 {
                                    theme::warn()
                                } else {
                                    theme::muted()
                                }),
                        );
                        ui.end_row();
                    }
                });
        });
}

// ================================================================ Yuqori panel

fn toolbar(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let mut add_block = false;
    let mut generate = false;
    ui.horizontal(|ui| {
        if !app.blocks.is_empty() {
            let cur = app.sales_block;
            let text = cur
                .and_then(|id| app.blocks.iter().find(|b| b.id == id))
                .map(|b| b.name.clone())
                .unwrap_or_else(|| t("dash").to_string());
            egui::ComboBox::from_id_salt("sales_block")
                .selected_text(text)
                .width(170.0)
                .show_ui(ui, |ui| {
                    for b in &app.blocks {
                        ui.selectable_value(&mut app.sales_block, Some(b.id), &b.name);
                    }
                });
        }
        if ui.button(t("add_block")).clicked() {
            add_block = true;
        }
        if app.sales_block.is_some() && ui.button(t("generate_units")).clicked() {
            generate = true;
        }
        ui.label(
            RichText::new(t("sales_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });

    if add_block {
        let n = app.blocks.len() + 1;
        let id = app.db.insert_block(&Block {
            id: 0,
            project_id: pid,
            name: format!("{n}-{}", t("block_short")),
            floors: 9,
            first_floor: 1,
            note: String::new(),
        });
        app.reload_modules();
        if id > 0 {
            app.sales_block = Some(id);
        }
    }
    if generate {
        // Generator paneli ochiq/yopiq holatini almashtiramiz.
        let key = egui::Id::new("gen_open");
        let open = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
        ui.data_mut(|d| d.insert_temp(key, !open));
    }
    generator(ui, app, pid);
}

/// Qavatlar bo'yicha bir xil kvartiralarni ommaviy yaratish paneli.
fn generator(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let key = egui::Id::new("gen_open");
    if !ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false) {
        return;
    }
    let Some(bid) = app.sales_block else { return };
    let Some(block) = app.blocks.iter().find(|b| b.id == bid).cloned() else {
        return;
    };

    let state_key = egui::Id::new("gen_state");
    // (qavatdagi soni, boshlang'ich raqam, maydon, 1 m² narxi, xonalar)
    let mut st = ui
        .data(|d| d.get_temp::<(i64, i64, f64, f64, i64)>(state_key))
        .unwrap_or((4, 1, 48.0, 9_000_000.0, 2));

    let mut run = false;
    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                num(ui, t("gen_per_floor"), &mut st.0, 1.0, 1.0..=20.0);
                num(ui, t("gen_first_number"), &mut st.1, 1.0, 1.0..=9999.0);
                fnum(ui, t("gen_area"), &mut st.2, 1.0, 1.0..=1000.0);
                fnum(ui, t("gen_price_m2"), &mut st.3, 100_000.0, 0.0..=1e12);
                num(ui, t("gen_rooms"), &mut st.4, 1.0, 0.0..=10.0);
                if ui.button(t("gen_run")).clicked() {
                    run = true;
                }
            });
            ui.label(
                RichText::new(t("gen_hint"))
                    .size(11.0)
                    .color(theme::muted()),
            );
        });
    ui.data_mut(|d| d.insert_temp(state_key, st));

    if run {
        let created = generate_units(app, pid, &block, st);
        app.reload_modules();
        app.notify(format!("{} {}", t("units_created"), created));
        ui.data_mut(|d| d.insert_temp(key, false));
    }
    ui.add_space(8.0);
}

fn num(
    ui: &mut egui::Ui,
    label: &str,
    v: &mut i64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
) {
    ui.label(RichText::new(label).size(11.5).color(theme::muted()));
    ui.add(egui::DragValue::new(v).speed(speed).range(range));
}

fn fnum(
    ui: &mut egui::Ui,
    label: &str,
    v: &mut f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
) {
    ui.label(RichText::new(label).size(11.5).color(theme::muted()));
    ui.add(egui::DragValue::new(v).speed(speed).range(range));
}

/// Blokning har bir qavati uchun bir xil kvartiralar yaratadi.
/// Mavjud raqamlar takrorlanmaydi — generator ikki marta bosilsa dubl bo'lmaydi.
fn generate_units(app: &App, pid: i64, block: &Block, st: (i64, i64, f64, f64, i64)) -> usize {
    let (per_floor, first_number, area, price, rooms) = st;
    let mut n = 0;
    let mut number = first_number;
    for floor in block.first_floor..block.first_floor + block.floors.max(1) {
        for position in 1..=per_floor.max(1) {
            let label = number.to_string();
            number += 1;
            let taken = app
                .units
                .iter()
                .any(|u| u.block_id == block.id && u.number == label);
            if taken {
                continue;
            }
            app.db.insert_unit(&Unit {
                id: 0,
                project_id: pid,
                block_id: block.id,
                number: label,
                floor,
                position,
                kind: UnitKind::Flat,
                rooms,
                area,
                area_living: area * 0.6,
                price_per_m2: price,
                status: UnitStatus::Free,
                layout: String::new(),
                note: String::new(),
            });
            n += 1;
        }
    }
    n
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let s = app.sales();
    let deals = app
        .deals
        .iter()
        .filter(|d| d.status != DealStatus::Cancelled)
        .count();
    let pct = if s.contracted > 0.0 {
        s.received / s.contracted * 100.0
    } else {
        0.0
    };

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_units"),
                s.units.to_string(),
                &format!("{} {} · {} {}", s.free, t("us_free"), s.sold, t("us_sold")),
                theme::accent(),
            ),
            stat(
                t("kpi_sales_value"),
                money(s.value_total),
                &format!("{} m²", super::materials::trim_num(s.area_total)),
                theme::text(),
            ),
            stat(
                t("kpi_contracted"),
                money(s.contracted),
                &format!("{deals} {}", t("kpi_deals_hint")),
                theme::text(),
            ),
            stat(
                t("kpi_received"),
                money(s.received),
                &format!("{pct:.0}% {}", t("kpi_received_hint")),
                theme::ok(),
            ),
            stat(
                t("kpi_overdue_pay"),
                money(s.overdue),
                t("kpi_overdue_pay_hint"),
                if s.overdue <= 0.0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
        ],
    );
}

// ================================================================ Shaxmatka

/// Holat rangi. Fon uchun yumshoq, matn uchun to'q variant qaytariladi.
fn status_color(s: UnitStatus) -> Color32 {
    match s {
        UnitStatus::Free => theme::ok(),
        UnitStatus::Reserved => theme::warn(),
        UnitStatus::Contract => theme::accent(),
        UnitStatus::Sold => Color32::from_rgb(0x7c, 0x4d, 0xff),
        UnitStatus::Unavailable => theme::muted(),
    }
}

fn board_tab(ui: &mut egui::Ui, app: &mut App) {
    legend(ui, app);
    ui.add_space(8.0);

    let Some(bid) = app.sales_block else { return };
    let Some(block) = app.blocks.iter().find(|b| b.id == bid).cloned() else {
        return;
    };
    let units: Vec<Unit> = app
        .units
        .iter()
        .filter(|u| u.block_id == bid)
        .cloned()
        .collect();

    if units.is_empty() {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("sales_no_units"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    let cols = units.iter().map(|u| u.position).max().unwrap_or(1).max(1);
    let top = units
        .iter()
        .map(|u| u.floor)
        .max()
        .unwrap_or(block.first_floor);
    let bottom = units
        .iter()
        .map(|u| u.floor)
        .min()
        .unwrap_or(block.first_floor);

    let mut clicked: Option<i64> = None;
    // Tanlangan birlik kartochkasi — o'ng yon panel: kengligi aniq hurmat
    // qilinadi, shaxmatka esa qolgan joyni to'liq egallaydi.
    if app.selected_unit.is_some() {
        egui::SidePanel::right("unit_card")
            .resizable(false)
            .exact_width(CARD_W)
            .frame(
                egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show_inside(ui, |ui| {
                unit_card(ui, app);
            });
    }
    {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ScrollArea ota-ui ning yo'nalishini meros oladi; bu yerda ota
                // gorizontal, shuning uchun qavatlarni ataylab vertikal terамiz.
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                    for floor in (bottom..=top).rev() {
                        ui.horizontal(|ui| {
                            // Qavat raqami — chap ustunda.
                            let (r, _) =
                                ui.allocate_exact_size(vec2(FLOOR_W, CELL_H), Sense::hover());
                            ui.painter().text(
                                r.center(),
                                Align2::CENTER_CENTER,
                                format!("{floor}"),
                                FontId::proportional(13.0),
                                theme::muted(),
                            );
                            for position in 1..=cols {
                                match units
                                    .iter()
                                    .find(|u| u.floor == floor && u.position == position)
                                {
                                    Some(u) => {
                                        if cell(ui, app, u) {
                                            clicked = Some(u.id);
                                        }
                                    }
                                    None => {
                                        ui.allocate_exact_size(
                                            vec2(CELL_W, CELL_H),
                                            Sense::hover(),
                                        );
                                    }
                                }
                            }
                        });
                    }
                    ui.add_space(6.0);
                    // Pastdagi ustun raqamlari.
                    ui.horizontal(|ui| {
                        ui.allocate_exact_size(vec2(FLOOR_W, 16.0), Sense::hover());
                        for position in 1..=cols {
                            let (r, _) = ui.allocate_exact_size(vec2(CELL_W, 16.0), Sense::hover());
                            ui.painter().text(
                                r.center(),
                                Align2::CENTER_CENTER,
                                format!("{position}"),
                                FontId::proportional(11.0),
                                theme::muted(),
                            );
                        }
                    });
                });
            });
    }

    if let Some(id) = clicked {
        app.selected_unit = Some(id);
    }
}

/// Bitta katak. Bosilgan bo'lsa `true`.
fn cell(ui: &mut egui::Ui, app: &App, u: &Unit) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(CELL_W, CELL_H), Sense::click());
    let color = status_color(u.status);
    let selected = app.selected_unit == Some(u.id);
    let p = ui.painter();

    // Fon — holat rangining yengil varianti, chegara to'q.
    p.rect_filled(
        rect,
        6.0,
        color.gamma_multiply(if resp.hovered() { 0.34 } else { 0.20 }),
    );
    p.rect_stroke(
        rect,
        6.0,
        Stroke::new(if selected { 2.0_f32 } else { 1.0 }, color),
        egui::StrokeKind::Inside,
    );

    p.text(
        rect.left_top() + vec2(8.0, 8.0),
        Align2::LEFT_TOP,
        &u.number,
        FontId::proportional(14.0),
        theme::text(),
    );
    p.text(
        rect.right_top() + vec2(-8.0, 9.0),
        Align2::RIGHT_TOP,
        // Kvartirada xonalar soni, boshqa turlarda — turning qisqartmasi.
        if u.kind == UnitKind::Flat {
            format!("{}{}", u.rooms, t("rooms_short"))
        } else {
            t(u.kind.short_key()).to_string()
        },
        FontId::proportional(11.0),
        theme::muted(),
    );
    p.text(
        rect.left_bottom() + vec2(8.0, -8.0),
        Align2::LEFT_BOTTOM,
        format!("{} m²", super::materials::trim_num(u.area)),
        FontId::proportional(11.5),
        theme::muted(),
    );

    // Muddati o'tgan to'lov bo'lsa — o'ng pastda qizil nuqta.
    if let Some(d) = sales::active_deal(&app.deals, u.id) {
        let st = sales::deal_state(d, &app.payments, app.today);
        if st.overdue > 0.0 {
            p.circle_filled(
                rect.right_bottom() + vec2(-10.0, -10.0),
                4.0,
                theme::danger(),
            );
        }
    }

    let label = format!(
        "{} · {} · {} m² · {}",
        u.number,
        u.status.label(),
        super::materials::trim_num(u.area),
        money(u.price())
    );
    resp.on_hover_text(label).clicked()
}

fn legend(ui: &mut egui::Ui, app: &App) {
    let count = |s: UnitStatus| app.units.iter().filter(|u| u.status == s).count();
    ui.horizontal_wrapped(|ui| {
        for s in UnitStatus::ALL {
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            let c = status_color(*s);
            ui.painter().rect_filled(r, 3.0, c.gamma_multiply(0.25));
            ui.painter()
                .rect_stroke(r, 3.0, Stroke::new(1.0_f32, c), egui::StrokeKind::Inside);
            ui.label(
                RichText::new(format!("{} — {}", s.label(), count(*s)))
                    .size(11.5)
                    .color(theme::muted()),
            );
            ui.add_space(10.0);
        }
    });
}

// ================================================================ Kartochka

fn unit_card(ui: &mut egui::Ui, app: &mut App) {
    let Some(uid) = app.selected_unit else { return };
    let Some(src) = app.units.iter().find(|u| u.id == uid).cloned() else {
        app.selected_unit = None;
        return;
    };
    let mut u = src.clone();
    let mut changed = false;
    let mut make_deal = false;
    let mut go_deal: Option<i64> = None;
    let mut close = false;
    let mut remove = false;

    // Ramkani yon panelning o'zi chizadi.
    {
        {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{} {}", t("unit_card"), u.number))
                            .size(15.0)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(6.0);

                // Kartochka uzun — panel balandligidan oshsa ichida suriladi.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        fld(ui, t("col_number"), |ui| {
                            changed |= ui
                                .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut u.number))
                                .changed();
                        });
                        fld(ui, t("col_floor"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.floor)
                                        .speed(1.0)
                                        .range(-5.0..=200.0),
                                )
                                .changed();
                        });
                        fld(ui, t("col_position"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.position)
                                        .speed(1.0)
                                        .range(1.0..=50.0),
                                )
                                .changed();
                        });
                        fld(ui, t("col_kind"), |ui| {
                            egui::ComboBox::from_id_salt("uc_kind")
                                .selected_text(u.kind.label())
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for k in UnitKind::ALL {
                                        changed |= ui
                                            .selectable_value(&mut u.kind, *k, k.label())
                                            .changed();
                                    }
                                });
                        });
                        fld(ui, t("col_rooms"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.rooms)
                                        .speed(1.0)
                                        .range(0.0..=10.0),
                                )
                                .changed();
                        });
                        fld(ui, t("col_area"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.area)
                                        .speed(0.5)
                                        .range(0.0..=10000.0),
                                )
                                .changed();
                        });
                        fld(ui, t("col_area_living"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.area_living)
                                        .speed(0.5)
                                        .range(0.0..=10000.0),
                                )
                                .changed();
                        });
                        fld(ui, t("col_price_m2"), |ui| {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut u.price_per_m2)
                                        .speed(100_000.0)
                                        .range(0.0..=1e12),
                                )
                                .changed();
                        });
                        fld(ui, t("col_price_total"), |ui| {
                            ui.label(RichText::new(money(u.price())).size(14.0).strong());
                        });
                        fld(ui, t("col_status"), |ui| {
                            egui::ComboBox::from_id_salt("uc_status")
                                .selected_text(
                                    RichText::new(u.status.label()).color(status_color(u.status)),
                                )
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for s in UnitStatus::ALL {
                                        changed |= ui
                                            .selectable_value(&mut u.status, *s, s.label())
                                            .changed();
                                    }
                                });
                        });
                        fld(ui, t("col_layout"), |ui| {
                            changed |= ui
                                .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut u.layout))
                                .changed();
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        match sales::active_deal(&app.deals, u.id) {
                            Some(d) => {
                                let st = sales::deal_state(d, &app.payments, app.today);
                                ui.label(
                                    RichText::new(t("unit_deal"))
                                        .size(12.0)
                                        .color(theme::muted()),
                                );
                                ui.label(
                                    RichText::new(format!("{} · {}", d.number, d.client))
                                        .size(13.0),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {}",
                                        d.pay_kind.label(),
                                        money(d.total())
                                    ))
                                    .size(12.0)
                                    .color(theme::muted()),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        t("paid_short"),
                                        money(st.paid)
                                    ))
                                    .size(12.0)
                                    .color(theme::ok()),
                                );
                                if st.overdue > 0.0 {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: {}",
                                            t("overdue_short"),
                                            money(st.overdue)
                                        ))
                                        .size(12.0)
                                        .color(theme::danger()),
                                    );
                                }
                                ui.add_space(6.0);
                                if ui.button(t("open_deal")).clicked() {
                                    go_deal = Some(d.id);
                                }
                            }
                            None => {
                                ui.label(
                                    RichText::new(t("unit_no_deal"))
                                        .size(12.0)
                                        .color(theme::muted()),
                                );
                                ui.add_space(6.0);
                                if ui.button(t("create_deal")).clicked() {
                                    make_deal = true;
                                }
                            }
                        }

                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new(t("delete_unit")).color(theme::danger()))
                            .clicked()
                        {
                            remove = true;
                        }
                    });
            });
        }
    }

    if changed {
        app.db.update_unit(&u);
        if let Some(slot) = app.units.iter_mut().find(|x| x.id == u.id) {
            *slot = u.clone();
        }
    }
    if close {
        app.selected_unit = None;
    }
    if remove {
        app.db.del("unit", uid);
        app.selected_unit = None;
        app.reload_modules();
    }
    if let Some(id) = go_deal {
        app.selected_deal = Some(id);
        app.screen = Screen::Deals;
    }
    if make_deal {
        let id = create_deal(app, &u);
        if id > 0 {
            app.reload_modules();
            app.sync_unit_status(u.id);
            app.selected_deal = Some(id);
            app.screen = Screen::Deals;
        }
    }
}

/// Kvartira uchun band qilish yozuvini ochadi — narx katalogdan olinadi.
fn create_deal(app: &App, u: &Unit) -> i64 {
    let Some(pid) = app.current else { return 0 };
    let n = app.deals.len() + 1;
    app.db.insert_deal(&Deal {
        id: 0,
        project_id: pid,
        unit_id: u.id,
        number: format!("S-{n:03}"),
        date: app.today,
        client: String::new(),
        phone: String::new(),
        client_doc: String::new(),
        pay_kind: PayKind::Cash,
        price: u.price(),
        discount: 0.0,
        prepayment: 0.0,
        months: 0,
        status: DealStatus::Reserved,
        manager: String::new(),
        note: String::new(),
    })
}

// ================================================================ Ro'yxat

fn list_tab(ui: &mut egui::Ui, app: &mut App) {
    let mut edited: Option<Unit> = None;
    let mut selected: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("units_grid")
                .num_columns(11)
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    head_l(ui, 110.0, t("col_block"));
                    head_l(ui, 80.0, t("col_number"));
                    head_r(ui, 60.0, t("col_floor"));
                    head_l(ui, 120.0, t("col_kind"));
                    head_r(ui, 70.0, t("col_rooms"));
                    head_r(ui, 80.0, t("col_area"));
                    head_r(ui, 120.0, t("col_price_m2"));
                    head_r(ui, 140.0, t("col_price_total"));
                    head_l(ui, 130.0, t("col_status"));
                    head_l(ui, 170.0, t("col_client"));
                    head_l(ui, 60.0, "");
                    ui.end_row();

                    for src in &app.units {
                        let mut u = src.clone();
                        let mut changed = false;
                        let name = app
                            .blocks
                            .iter()
                            .find(|b| b.id == u.block_id)
                            .map(|b| b.name.clone())
                            .unwrap_or_else(|| t("dash").to_string());

                        cell_l(ui, 110.0, RichText::new(name).size(12.0));
                        changed |= ui
                            .add_sized([80.0, 22.0], egui::TextEdit::singleline(&mut u.number))
                            .changed();
                        cell_r(ui, 60.0, RichText::new(u.floor.to_string()).size(12.0));
                        egui::ComboBox::from_id_salt(("ul_kind", u.id))
                            .selected_text(u.kind.label())
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for k in UnitKind::ALL {
                                    changed |=
                                        ui.selectable_value(&mut u.kind, *k, k.label()).changed();
                                }
                            });
                        changed |= ui
                            .add_sized(
                                [70.0, 22.0],
                                egui::DragValue::new(&mut u.rooms)
                                    .speed(1.0)
                                    .range(0.0..=10.0),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut u.area)
                                    .speed(0.5)
                                    .range(0.0..=10000.0),
                            )
                            .changed();
                        changed |= ui
                            .add_sized(
                                [120.0, 22.0],
                                egui::DragValue::new(&mut u.price_per_m2)
                                    .speed(100_000.0)
                                    .range(0.0..=1e12),
                            )
                            .changed();
                        cell_r(ui, 140.0, RichText::new(money(u.price())).size(12.5));
                        egui::ComboBox::from_id_salt(("ul_st", u.id))
                            .selected_text(
                                RichText::new(u.status.label()).color(status_color(u.status)),
                            )
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for s in UnitStatus::ALL {
                                    changed |=
                                        ui.selectable_value(&mut u.status, *s, s.label()).changed();
                                }
                            });
                        let client = sales::active_deal(&app.deals, u.id)
                            .map(|d| d.client.clone())
                            .unwrap_or_else(|| t("dash").to_string());
                        cell_l(
                            ui,
                            170.0,
                            RichText::new(super::issues::truncate(&client, 22))
                                .size(12.0)
                                .color(theme::muted()),
                        );
                        if ui.small_button(t("open_short")).clicked() {
                            selected = Some(u.id);
                        }
                        ui.end_row();

                        if changed {
                            edited = Some(u);
                        }
                    }
                });
        });

    if let Some(u) = edited {
        app.db.update_unit(&u);
        if let Some(slot) = app.units.iter_mut().find(|x| x.id == u.id) {
            *slot = u;
        }
    }
    if let Some(id) = selected {
        app.selected_unit = Some(id);
        app.sales_block = app.units.iter().find(|u| u.id == id).map(|u| u.block_id);
        ui.data_mut(|d| d.insert_temp(egui::Id::new("sales_tab"), 0u8));
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

/// Yon paneldagi «yorliq — qiymat» qatori. Umumiy `field` dan tor yorliq bilan,
/// chunki kartochka kengligi 330 px.
fn fld(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [118.0, 22.0],
            egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
        );
        add(ui);
    });
}
