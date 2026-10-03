//! «Shartnomalar va to'lovlar» ekrani (XX).
//!
//! Chapda shartnomalar ro'yxati, o'ngda tanlangan shartnomaning shartlari va
//! to'lov grafigi. Grafik shartnoma shartlaridan quriladi (`sales::build_schedule`),
//! shuning uchun reja summasi shartnoma summasiga har doim to'g'ri keladi;
//! mos kelmasa ekran buni ochiq aytadi.

use super::materials::trim_num;
use super::warehouse::{cell_l, cell_r};
use super::*;
use crate::domain::{Deal, DealStatus, PayKind, Payment};
use crate::sales;

/// Shartnoma kartochkasining kengligi.
const CARD_W: f32 = 460.0;

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
    if app.units.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.label(
                RichText::new(t("deals_no_units"))
                    .color(theme::muted())
                    .size(16.0),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t("deals_no_units_hint"))
                    .color(theme::muted())
                    .size(12.0),
            );
        });
        return;
    }

    kpi_row(ui, app);
    ui.add_space(8.0);
    debts_row(ui, app);
    ui.add_space(10.0);

    if app.deals.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("deals_empty"))
                    .color(theme::muted())
                    .size(15.0),
            );
        });
        return;
    }

    // Kartochka yon panel sifatida — kengligi aniq hurmat qilinadi va
    // ro'yxat qolgan joyni to'liq egallaydi.
    if app.selected_deal.is_some() {
        egui::SidePanel::right("deal_card")
            .resizable(false)
            .exact_width(CARD_W)
            .frame(
                egui::Frame::new()
                    .fill(theme::card())
                    .stroke(Stroke::new(1.0_f32, theme::line()))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show_inside(ui, |ui| {
                detail(ui, app);
            });
    }
    list(ui, app);
}

/// Qarzdorlik kechikish muddati bo'yicha (TZ XX).
///
/// Qarzning umumiy summasi kam narsa aytadi: bugun muddati kelgan
/// to'lov bilan uch oylik qarz bir xil emas. Shuning uchun qarz
/// guruhlarga bo'lib ko'rsatiladi va eng eskisi ajratiladi.
fn debts_row(ui: &mut egui::Ui, app: &App) {
    use crate::sales::Bucket;

    // Qarz yoshi bir marta hisoblangan: bu yerda butun to'lov jadvali
    // har kadrda ikki marta ko'rilardi.
    let rows = &app.aging;
    if rows.is_empty() {
        return;
    }
    let totals = crate::sales::aging_totals(rows);

    egui::Frame::new()
        .fill(theme::card())
        .stroke(Stroke::new(1.0_f32, theme::line()))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(t("dl_aging")).size(12.5).strong());
                ui.label(
                    RichText::new(t("dl_aging_hint"))
                        .size(11.0)
                        .color(theme::muted()),
                );
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (bucket, count, sum) in &totals {
                    if *count == 0 {
                        continue;
                    }
                    // Kechikish qancha uzoq bo'lsa, rang shuncha kuchli.
                    let colour = match bucket {
                        Bucket::Future => theme::muted(),
                        Bucket::Days30 => theme::text(),
                        Bucket::Days60 => theme::warn(),
                        _ => theme::danger(),
                    };
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(bucket.label())
                                .size(10.5)
                                .color(theme::muted()),
                        );
                        ui.label(RichText::new(money(*sum)).size(13.0).strong().color(colour));
                        ui.label(
                            RichText::new(format!("{count} {}", t("dl_deals")))
                                .size(10.5)
                                .color(theme::muted()),
                        );
                    });
                    ui.add_space(18.0);
                }
            });
        });
}

fn kpi_row(ui: &mut egui::Ui, app: &App) {
    let s = app.sales();
    let by_kind = sales::by_pay_kind(&app.deals);
    // Eng ko'p ishlatilgan to'lov turi — sotuv siyosati bir qarashda ko'rinsin.
    let top = by_kind
        .iter()
        .filter(|(_, _, n)| *n > 0)
        .max_by(|a, b| a.1.total_cmp(&b.1));

    let top_hint = match top {
        Some((k, _, n)) => format!("{} · {n}", k.label()),
        None => t("dash").to_string(),
    };
    stat_row(
        ui,
        vec![
            stat(
                t("kpi_contracted"),
                money(s.contracted),
                &format!(
                    "{} {}",
                    app.deals
                        .iter()
                        .filter(|d| d.status != DealStatus::Cancelled)
                        .count(),
                    t("kpi_deals_hint")
                ),
                theme::text(),
            ),
            stat(
                t("kpi_received"),
                money(s.received),
                t("kpi_received_hint2"),
                theme::ok(),
            ),
            stat(
                t("kpi_debt"),
                money(s.debt),
                t("kpi_debt_hint"),
                if s.debt <= 0.0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
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
            stat(
                t("kpi_avg_m2"),
                money(s.avg_price_m2),
                &top_hint,
                theme::accent(),
            ),
        ],
    );
}

fn status_color(s: DealStatus) -> Color32 {
    match s {
        DealStatus::Reserved => theme::warn(),
        DealStatus::Signed => theme::accent(),
        DealStatus::Completed => theme::ok(),
        DealStatus::Cancelled => theme::muted(),
    }
}

/// Birlik nomi: blok va raqam.
fn unit_label(app: &App, unit_id: i64) -> String {
    match app.units.iter().find(|u| u.id == unit_id) {
        Some(u) => {
            let block = app
                .blocks
                .iter()
                .find(|b| b.id == u.block_id)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            format!("{block} · {} ({} m²)", u.number, trim_num(u.area))
        }
        None => t("dash").to_string(),
    }
}

// ================================================================ Ro'yxat

fn list(ui: &mut egui::Ui, app: &mut App) {
    let mut selected: Option<i64> = None;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Ota-ui gorizontal — jadvalni vertikal oqimga qaytaramiz.
            ui.vertical(|ui| {
                egui::Grid::new("deals_grid")
                    .num_columns(9)
                    .spacing([8.0, 5.0])
                    .striped(true)
                    .show(ui, |ui| {
                        head_l(ui, 80.0, t("col_number"));
                        head_l(ui, 104.0, t("col_date"));
                        head_l(ui, 120.0, t("col_status"));
                        head_l(ui, 170.0, t("col_flat"));
                        head_l(ui, 160.0, t("col_client"));
                        head_l(ui, 120.0, t("col_pay_kind"));
                        head_r(ui, 140.0, t("col_deal_total"));
                        head_r(ui, 140.0, t("col_paid"));
                        head_l(ui, 60.0, "");
                        ui.end_row();

                        for d in &app.deals {
                            let st = app.deal_state(d.id);
                            cell_l(ui, 80.0, RichText::new(&d.number).size(12.5).strong());
                            cell_l(
                                ui,
                                104.0,
                                RichText::new(d.date.format("%d.%m.%Y").to_string()).size(12.0),
                            );
                            cell_l(
                                ui,
                                120.0,
                                RichText::new(d.status.label())
                                    .size(12.0)
                                    .color(status_color(d.status)),
                            );
                            cell_l(
                                ui,
                                170.0,
                                RichText::new(super::issues::truncate(
                                    &unit_label(app, d.unit_id),
                                    24,
                                ))
                                .size(12.0),
                            );
                            cell_l(
                                ui,
                                160.0,
                                RichText::new(super::issues::truncate(&d.client, 22)).size(12.0),
                            );
                            cell_l(
                                ui,
                                120.0,
                                RichText::new(d.pay_kind.label())
                                    .size(12.0)
                                    .color(theme::muted()),
                            );
                            cell_r(ui, 140.0, RichText::new(money(d.total())).size(12.5));
                            // To'langan summa: muddati o'tgan qarz bo'lsa qizil.
                            cell_r(
                                ui,
                                140.0,
                                RichText::new(money(st.paid)).size(12.5).color(
                                    if st.overdue > 0.0 {
                                        theme::danger()
                                    } else {
                                        theme::ok()
                                    },
                                ),
                            );
                            if ui.small_button(t("open_short")).clicked() {
                                selected = Some(d.id);
                            }
                            ui.end_row();
                        }
                    });
            });
        });

    if let Some(id) = selected {
        app.selected_deal = Some(id);
    }
}

// ================================================================ Kartochka

fn detail(ui: &mut egui::Ui, app: &mut App) {
    let Some(did) = app.selected_deal else { return };
    let Some(src) = app.deals.iter().find(|d| d.id == did).cloned() else {
        app.selected_deal = None;
        return;
    };
    let mut d = src.clone();
    let mut changed = false;
    let mut close = false;
    let mut remove = false;
    let mut rebuild = false;
    let mut add_row = false;
    let mut go_unit = false;
    let st = sales::deal_state(&d, &app.payments, app.today);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} {}", t("deal_card"), d.number))
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

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                terms(ui, app, &mut d, &mut changed, &mut go_unit);
                ui.add_space(8.0);
                summary(ui, &d, &st);
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("payment_plan")).size(13.0).strong());
                    if ui
                        .button(t("rebuild_schedule"))
                        .on_hover_text(t("rebuild_schedule_hint"))
                        .clicked()
                    {
                        rebuild = true;
                    }
                    if ui.button(t("add_payment")).clicked() {
                        add_row = true;
                    }
                });
                if st.schedule_mismatch {
                    ui.label(
                        RichText::new(t("schedule_mismatch"))
                            .size(11.0)
                            .color(theme::warn()),
                    );
                }
                ui.add_space(4.0);
                schedule(ui, app, d.id);

                ui.add_space(10.0);
                if ui
                    .button(RichText::new(t("delete_deal")).color(theme::danger()))
                    .clicked()
                {
                    remove = true;
                }
            });
    });

    // ---- Amallar
    if changed {
        app.db.update_deal(&d);
        if let Some(slot) = app.deals.iter_mut().find(|x| x.id == d.id) {
            *slot = d.clone();
        }
        app.sync_unit_status(d.unit_id);
    }
    if close {
        app.selected_deal = None;
    }
    if go_unit {
        app.selected_unit = Some(d.unit_id);
        app.sales_block = app
            .units
            .iter()
            .find(|u| u.id == d.unit_id)
            .map(|u| u.block_id);
        app.screen = Screen::Sales;
    }
    if rebuild {
        // Tushgan pul saqlanadi: qayta qurish rejani yangilaydi, faktni
        // emas.
        let plan = sales::rebuild_schedule(&d, &app.payments);
        for id in &plan.remove {
            app.db.del("payment", *id);
        }
        for p in &plan.add {
            app.db.insert_payment(p);
        }
        app.reload_modules();
        let note = if plan.kept_rows > 0 {
            format!(
                "{} · {} {} ({})",
                t("schedule_rebuilt"),
                plan.kept_rows,
                t("schedule_kept"),
                money(plan.kept)
            )
        } else {
            t("schedule_rebuilt").to_string()
        };
        app.notify(note);
    }
    if add_row {
        app.db.insert_payment(&Payment {
            id: 0,
            project_id: d.project_id,
            deal_id: d.id,
            due: app.today,
            planned: 0.0,
            paid: 0.0,
            paid_date: None,
            kind: d.pay_kind,
            document: String::new(),
            note: String::new(),
        });
        app.reload_modules();
    }
    if remove {
        let unit = d.unit_id;
        app.db.del("deal", did);
        app.selected_deal = None;
        app.reload_modules();
        // Holat umumiy qoidadan qayta hisoblanadi. Avval bu yerda «bo'sh»
        // deb qo'yilardi — birlikda boshqa amaldagi shartnoma qolgan
        // bo'lsa ham, va kvartira ikkinchi marta sotilishi mumkin edi.
        app.sync_unit_status(unit);
    }
}

fn terms(ui: &mut egui::Ui, app: &App, d: &mut Deal, changed: &mut bool, go_unit: &mut bool) {
    fld(ui, t("col_flat"), |ui| {
        if ui
            .link(super::issues::truncate(&unit_label(app, d.unit_id), 28))
            .clicked()
        {
            *go_unit = true;
        }
    });
    fld(ui, t("col_number"), |ui| {
        *changed |= ui
            .add_sized([120.0, 22.0], egui::TextEdit::singleline(&mut d.number))
            .changed();
    });
    fld(ui, t("col_date"), |ui| {
        *changed |= super::passport::date_edit(ui, "deal_date", &mut d.date);
    });
    fld(ui, t("col_status"), |ui| {
        egui::ComboBox::from_id_salt("deal_status")
            .selected_text(RichText::new(d.status.label()).color(status_color(d.status)))
            .width(150.0)
            .show_ui(ui, |ui| {
                for s in DealStatus::ALL {
                    *changed |= ui.selectable_value(&mut d.status, *s, s.label()).changed();
                }
            });
    });
    fld(ui, t("col_client"), |ui| {
        *changed |= ui
            .add_sized([200.0, 22.0], egui::TextEdit::singleline(&mut d.client))
            .changed();
    });
    fld(ui, t("col_phone"), |ui| {
        *changed |= ui
            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut d.phone))
            .changed();
    });
    fld(ui, t("col_client_doc"), |ui| {
        *changed |= ui
            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut d.client_doc))
            .changed();
    });
    fld(ui, t("col_pay_kind"), |ui| {
        egui::ComboBox::from_id_salt("deal_pay")
            .selected_text(d.pay_kind.label())
            .width(150.0)
            .show_ui(ui, |ui| {
                for k in PayKind::ALL {
                    *changed |= ui
                        .selectable_value(&mut d.pay_kind, *k, k.label())
                        .changed();
                }
            });
    });
    fld(ui, t("col_price"), |ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut d.price)
                    .speed(100_000.0)
                    .range(0.0..=1e13),
            )
            .changed();
    });
    fld(ui, t("col_discount"), |ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut d.discount)
                    .speed(100_000.0)
                    .range(0.0..=1e13),
            )
            .changed();
    });
    fld(ui, t("col_prepayment"), |ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut d.prepayment)
                    .speed(100_000.0)
                    .range(0.0..=1e13),
            )
            .changed();
    });
    fld(ui, t("col_months"), |ui| {
        *changed |= ui
            .add(
                egui::DragValue::new(&mut d.months)
                    .speed(1.0)
                    .range(0.0..=360.0),
            )
            .changed();
    });
    fld(ui, t("col_manager"), |ui| {
        *changed |= ui
            .add_sized([150.0, 22.0], egui::TextEdit::singleline(&mut d.manager))
            .changed();
    });
}

fn summary(ui: &mut egui::Ui, d: &Deal, st: &sales::DealState) {
    let line = |ui: &mut egui::Ui, label: &str, value: String, color: Color32| {
        ui.horizontal(|ui| {
            ui.add_sized(
                [150.0, 18.0],
                egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
            );
            ui.label(RichText::new(value).size(13.0).color(color).strong());
        });
    };
    line(ui, t("deal_total"), money(d.total()), theme::text());
    line(ui, t("deal_paid"), money(st.paid), theme::ok());
    line(
        ui,
        t("deal_remaining"),
        money(st.remaining),
        if st.remaining > 0.0 {
            theme::warn()
        } else {
            theme::ok()
        },
    );
    if st.overdue > 0.0 {
        line(ui, t("deal_overdue"), money(st.overdue), theme::danger());
    }
    // Grafik qatorlari va ularning reja summasi — shartnoma summasi bilan
    // solishtirish uchun.
    if st.rows > 0 {
        line(
            ui,
            t("deal_schedule"),
            format!("{} · {}", st.rows, money(st.planned)),
            if st.schedule_mismatch {
                theme::warn()
            } else {
                theme::muted()
            },
        );
    }
    if let Some(due) = st.next_due {
        line(
            ui,
            t("deal_next_due"),
            due.format("%d.%m.%Y").to_string(),
            theme::text(),
        );
    }
}

// ================================================================ To'lov grafigi

fn schedule(ui: &mut egui::Ui, app: &mut App, deal_id: i64) {
    let rows: Vec<Payment> = app
        .payments
        .iter()
        .filter(|p| p.deal_id == deal_id)
        .cloned()
        .collect();
    if rows.is_empty() {
        ui.label(
            RichText::new(t("schedule_empty"))
                .size(12.0)
                .color(theme::muted()),
        );
        return;
    }

    let today = app.today;
    let mut edited: Option<Payment> = None;
    let mut removed: Option<i64> = None;
    let mut pay_full: Option<i64> = None;

    egui::Grid::new(("pay_grid", deal_id))
        .num_columns(6)
        .spacing([6.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            head_l(ui, 96.0, t("col_due"));
            head_r(ui, 96.0, t("col_planned"));
            head_r(ui, 96.0, t("col_fact"));
            head_l(ui, 44.0, t("col_document"));
            head_l(ui, 24.0, "");
            head_l(ui, 24.0, "");
            ui.end_row();

            for src in &rows {
                let mut p = src.clone();
                let mut changed = false;
                let unpaid = p.paid + 0.01 < p.planned;

                // Muddati o'tgan va to'lanmagan qator qizil sana bilan ajraladi.
                ui.horizontal(|ui| {
                    changed |= super::passport::date_edit(ui, &format!("pd{}", p.id), &mut p.due);
                    if unpaid && p.due <= today {
                        ui.label(RichText::new("!").color(theme::danger()).strong());
                    }
                });
                changed |= ui
                    .add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut p.planned)
                            .speed(100_000.0)
                            .range(0.0..=1e13),
                    )
                    .changed();
                let before = p.paid;
                changed |= ui
                    .add_sized(
                        [96.0, 22.0],
                        egui::DragValue::new(&mut p.paid)
                            .speed(100_000.0)
                            .range(0.0..=1e13),
                    )
                    .changed();
                // Pul kiritilganda to'lov sanasi o'zi qo'yiladi.
                if p.paid > before && p.paid_date.is_none() {
                    p.paid_date = Some(today);
                }
                changed |= ui
                    .add_sized([44.0, 22.0], egui::TextEdit::singleline(&mut p.document))
                    .changed();

                if unpaid {
                    if ui
                        .small_button(RichText::new("v").color(theme::ok()))
                        .on_hover_text(t("mark_paid"))
                        .clicked()
                    {
                        pay_full = Some(p.id);
                    }
                } else {
                    ui.label(RichText::new("v").size(11.0).color(theme::ok()));
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

            // Jami qatori.
            cell_l(
                ui,
                104.0,
                RichText::new(t("total_row"))
                    .size(11.5)
                    .color(theme::muted()),
            );
            cell_r(
                ui,
                110.0,
                RichText::new(money(rows.iter().map(|p| p.planned).sum()))
                    .size(12.0)
                    .strong(),
            );
            cell_r(
                ui,
                96.0,
                RichText::new(money(rows.iter().map(|p| p.paid).sum()))
                    .size(12.0)
                    .strong()
                    .color(theme::ok()),
            );
            ui.end_row();
        });

    if let Some(id) = pay_full {
        if let Some(p) = app.payments.iter_mut().find(|p| p.id == id) {
            p.paid = p.planned;
            p.paid_date = Some(today);
            let copy = p.clone();
            app.db.update_payment(&copy);
        }
    }
    if let Some(p) = edited {
        app.db.update_payment(&p);
        if let Some(slot) = app.payments.iter_mut().find(|x| x.id == p.id) {
            *slot = p;
        }
    }
    if let Some(id) = removed {
        app.db.del("payment", id);
        app.reload_modules();
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

/// Shartnoma kartochkasidagi «yorliq — qiymat» qatori.
fn fld(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [150.0, 22.0],
            egui::Label::new(RichText::new(label).size(12.0).color(theme::muted())),
        );
        add(ui);
    });
}
