//! «Arizalar» ekrani (TZ IX).
//!
//! Ta'minot zanjirining birinchi bo'g'ini: ehtiyoj → ariza → tasdiqlash.
//! Har bir ariza uchun unga bog'langan xaridlar bo'yicha qoplanish
//! ko'rsatiladi, shunda «so'radim» va «keldi» orasidagi farq ko'rinib turadi.

use super::materials::{material_label, trim_num};
use super::warehouse::cell_l;
use super::*;
use crate::checks::SupplyLine;
use crate::domain::NoteTarget;
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
    let mut register = false;
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
        // Buxgalteriya uchun to'lov reyestri (TZ IX.39).
        if ui
            .button(t("rq_register"))
            .on_hover_text(t("rq_register_hint"))
            .clicked()
        {
            register = true;
        }
        ui.label(
            RichText::new(t("requests_hint"))
                .size(11.0)
                .color(theme::muted()),
        );
    });
    ui.add_space(8.0);

    if register {
        save_register(app);
    }

    let supply = app.supply();
    // Modul yordamchisi (TZ: har modul uchun AI-yordamchi).
    ui.horizontal(|ui| {
        super::assistant_button(ui, app);
    });
    ui.add_space(6.0);
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
        // Kelishuv paneli o'ng tomonda turadi — jadvalni bosib qolmasin.
        let open = app
            .request_open
            .filter(|id| app.requests.iter().any(|q| q.id == *id));
        if open.is_some() && ui.available_width() > 980.0 {
            egui::SidePanel::right("rq_route")
                .resizable(false)
                .exact_width(360.0)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.add_space(2.0);
                    route_panel(ui, app, pid);
                });
            table(ui, app, &supply);
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if open.is_some() {
                        route_panel(ui, app, pid);
                        ui.add_space(10.0);
                    }
                    table(ui, app, &supply);
                });
        }
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
        reject_reason: String::new(),
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

    stat_row(
        ui,
        vec![
            stat(
                t("kpi_requests"),
                total.to_string(),
                t("kpi_requests_hint"),
                theme::accent(),
            ),
            stat(
                t("kpi_req_new"),
                new.to_string(),
                t("kpi_req_new_hint"),
                if new == 0 { theme::ok() } else { theme::warn() },
            ),
            stat(
                t("kpi_req_late"),
                late.to_string(),
                t("kpi_req_late_hint"),
                if late == 0 {
                    theme::ok()
                } else {
                    theme::danger()
                },
            ),
            stat(
                t("kpi_req_uncovered"),
                uncovered.to_string(),
                t("kpi_req_uncovered_hint"),
                if uncovered == 0 {
                    theme::ok()
                } else {
                    theme::warn()
                },
            ),
        ],
    );
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
    let mut open_route: Option<i64> = None;
    let mut open_notes: Option<i64> = None;

    let today = app.today;
    // Tor ekranda ish ustuni yashiriladi: u bir marta to'ldiriladi,
    // kunlik ish esa holat va miqdor ustunlarida.
    let wide = ui.available_width() > 1650.0;
    let tasks = app.tasks.clone();
    // Tekshiruvlar bir marta hisoblanadi: har qator uchun qayta bajarish
    // o'nlab keraksiz taqqoslash bo'lardi.
    // Xodim ehtiyoji bir marta hisoblanadi: har ariza uchun qayta
    // hisoblash bir xil natijani beradi, lekin sekinroq.
    let staff = Some(app.staff_forecast());
    let checks: Vec<(i64, Vec<crate::checks::RequestIssue>)> = app
        .requests
        .iter()
        .map(|r| {
            (
                r.id,
                crate::checks::request_issues(
                    r,
                    &app.requests,
                    &app.materials,
                    &app.material_alts,
                    &app.estimate_items,
                    &app.purchase_budgets,
                    &app.purchases,
                    &app.material_norms,
                    &app.tasks,
                    staff.as_ref(),
                ),
            )
        })
        .collect();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("requests_grid")
                .num_columns(if wide { 16 } else { 15 })
                .spacing([8.0, 5.0])
                .striped(true)
                .show(ui, |ui| {
                    // Tartib nazorat uchun: avval kim va qanday holatda,
                    // keyin tafsilotlar. Shunda muhim ustunlar surmasdan ko'rinadi.
                    head_l(ui, 76.0, t("col_number"));
                    head_l(ui, 104.0, t("col_date"));
                    head_l(ui, 124.0, t("col_status"));
                    head_l(ui, 150.0, t("col_route"));
                    head_l(ui, 40.0, t("col_check"));
                    head_l(ui, 96.0, t("col_priority"));
                    head_l(ui, 150.0, t("col_coverage"));
                    head_l(ui, 100.0, t("col_kind"));
                    head_l(ui, 180.0, t("col_item"));
                    head_l(ui, 180.0, t("col_material"));
                    head_r(ui, 80.0, t("col_qty"));
                    head_l(ui, 56.0, t("col_unit"));
                    head_l(ui, 110.0, t("col_need_date"));
                    head_l(ui, 120.0, t("col_requester"));
                    if wide {
                        head_l(ui, 120.0, t("col_task"));
                    }
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
                        // Kelishuv holati — bosilsa o'ng panel ochiladi.
                        let (text, color) = route_label(app, q.id);
                        if ui
                            .add_sized(
                                [150.0, 22.0],
                                egui::Button::new(RichText::new(text).size(11.5).color(color))
                                    .frame(app.request_open == Some(q.id)),
                            )
                            .on_hover_text(t("route_open_hint"))
                            .clicked()
                        {
                            open_route = Some(q.id);
                        }

                        // Tekshiruv belgisi: savollar bo'lsa ularning
                        // hammasi hover matnida ko'rinadi.
                        let issues = checks
                            .iter()
                            .find(|(id, _)| *id == q.id)
                            .map(|(_, v)| v.as_slice())
                            .unwrap_or(&[]);
                        let resp = cell_l(
                            ui,
                            40.0,
                            if issues.is_empty() {
                                RichText::new("✓").size(13.0).color(theme::ok())
                            } else {
                                RichText::new(issues.len().to_string())
                                    .size(12.5)
                                    .strong()
                                    .color(theme::warn())
                            },
                        );
                        if issues.is_empty() {
                            resp.on_hover_text(t("rq_check_ok"));
                        } else {
                            resp.on_hover_text(
                                issues.iter().map(issue_text).collect::<Vec<_>>().join(
                                    "
",
                                ),
                            );
                        }

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

                        if wide {
                            // Ariza qaysi ish uchun: kechikish kimga ta'sir
                            // qilishini shu bog'lanish ko'rsatadi.
                            let label = q
                                .task_id
                                .and_then(|id| tasks.iter().find(|t| t.id == id))
                                .map(|t| t.wbs.clone())
                                .unwrap_or_else(|| t("dash").to_string());
                            egui::ComboBox::from_id_salt(("rq_task", q.id))
                                .selected_text(label)
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    changed |= ui
                                        .selectable_value(&mut q.task_id, None, t("dash"))
                                        .changed();
                                    for tk in &tasks {
                                        changed |= ui
                                            .selectable_value(
                                                &mut q.task_id,
                                                Some(tk.id),
                                                format!("{} {}", tk.wbs, tk.name),
                                            )
                                            .changed();
                                    }
                                });
                        }

                        // Arizaga foto va hujjat biriktirish (TZ IX.28).
                        if super::notes::badge(ui, app, NoteTarget::Request, q.id) {
                            open_notes = Some(q.id);
                        }
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
    if let Some(id) = open_route {
        // Qayta bosilsa panel yopiladi.
        app.request_open = (app.request_open != Some(id)).then_some(id);
    }
    super::notes::below_table(ui, app, NoteTarget::Request, open_notes);
}

// ================================================================ Kelishuv

/// Jadvaldagi qisqa holat: kim kutilmoqda yoki natija.
fn route_label(app: &App, request_id: i64) -> (String, Color32) {
    match crate::checks::route_state(request_id, &app.approvals) {
        crate::checks::RouteState::None => (t("route_none").to_string(), theme::muted()),
        crate::checks::RouteState::Waiting { step, role } => (
            format!("{step}. {} {}", role.label(), t("route_waiting")),
            theme::warn(),
        ),
        crate::checks::RouteState::Approved => (t("route_approved").to_string(), theme::ok()),
        crate::checks::RouteState::Rejected { role, .. } => (
            format!("{} {}", t("route_rejected"), role.label()),
            theme::danger(),
        ),
    }
}

/// Kelishuv marshruti paneli: bosqichlar, qaror tugmalari va tarix (TZ IX.8, 31–32).
fn route_panel(ui: &mut egui::Ui, app: &mut App, pid: i64) {
    let Some(rid) = app.request_open else { return };
    let Some(q) = app.requests.iter().find(|q| q.id == rid).cloned() else {
        return;
    };
    let amount = crate::checks::request_amount(&q, &app.materials);
    let route = crate::checks::approval_route(amount);

    let mut build = false;
    let mut clear = false;
    let mut close = false;
    // (bosqich id, qaror)
    let mut decide: Option<(i64, crate::domain::ApprovalDecision)> = None;
    let mut edited: Option<Request> = None;

    egui::Frame::group(ui.style())
        .fill(theme::card())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_min_width(320.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("route_title")).size(14.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("x").clicked() {
                        close = true;
                    }
                });
            });
            ui.label(
                RichText::new(format!("{} · {}", q.number, q.title))
                    .size(12.0)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);

            // Summa va shu summaga mos marshrut — limitlar ochiq yozilgan.
            ui.label(RichText::new(format!("{}: {}", t("route_amount"), money(amount))).size(12.5));
            ui.label(
                RichText::new(format!(
                    "{} {}",
                    t("route_by_limit"),
                    route
                        .iter()
                        .map(|r| r.label())
                        .collect::<Vec<_>>()
                        .join(" → ")
                ))
                .size(11.0)
                .color(theme::muted()),
            );
            ui.label(
                RichText::new(t("route_limits_hint"))
                    .size(10.5)
                    .color(theme::muted()),
            );
            ui.add_space(6.0);

            // Byudjet tekshiruvi (TZ IX.10).
            match crate::checks::request_budget_left(
                &q,
                &app.materials,
                &app.purchase_budgets,
                &app.purchases,
            ) {
                Some(left) if left < 0.0 => {
                    ui.label(
                        RichText::new(format!("{} {}", t("route_over_budget"), money(-left)))
                            .size(11.5)
                            .color(theme::danger()),
                    );
                }
                Some(left) => {
                    ui.label(
                        RichText::new(format!("{} {}", t("route_budget_left"), money(left)))
                            .size(11.5)
                            .color(theme::ok()),
                    );
                }
                None => {
                    ui.label(
                        RichText::new(t("route_no_budget"))
                            .size(11.0)
                            .color(theme::muted()),
                    );
                }
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let mine: Vec<crate::domain::Approval> = app
                .approvals
                .iter()
                .filter(|a| a.request_id == rid)
                .cloned()
                .collect();

            if mine.is_empty() {
                ui.label(
                    RichText::new(t("route_not_built"))
                        .size(12.0)
                        .color(theme::muted()),
                );
                ui.add_space(6.0);
                if ui
                    .button(t("route_build"))
                    .on_hover_text(t("route_build_hint"))
                    .clicked()
                {
                    build = true;
                }
            } else {
                let state = crate::checks::route_state(rid, &app.approvals);
                for a in &mine {
                    let role = crate::roles::Role::parse(&a.role);
                    let waiting = matches!(
                        state,
                        crate::checks::RouteState::Waiting { step, .. } if step == a.step
                    );
                    ui.horizontal(|ui| {
                        let (mark, color) = match a.decision {
                            crate::domain::ApprovalDecision::Approved => ("+", theme::ok()),
                            crate::domain::ApprovalDecision::Rejected => ("x", theme::danger()),
                            crate::domain::ApprovalDecision::Pending if waiting => {
                                ("*", theme::warn())
                            }
                            _ => ("·", theme::muted()),
                        };
                        ui.label(RichText::new(mark).color(color).strong().monospace());
                        ui.label(RichText::new(format!("{}. {}", a.step, role.label())).size(12.0));
                        if let Some(d) = a.decided_at {
                            ui.label(
                                RichText::new(d.format("%d.%m.%y").to_string())
                                    .size(10.5)
                                    .monospace()
                                    .color(theme::muted()),
                            );
                        }
                    });
                    if !a.approver.is_empty() {
                        ui.label(
                            RichText::new(format!("    {}", a.approver))
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    }
                    if !a.comment.is_empty() {
                        ui.label(
                            RichText::new(format!("    {}", a.comment))
                                .size(11.0)
                                .color(theme::muted()),
                        );
                    }

                    // Qaror faqat navbatdagi bosqichda va faqat o'sha rolda.
                    if waiting {
                        let can = app.role() == role || app.role() == crate::roles::Role::Admin;
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            if ui
                                .add_enabled(can, egui::Button::new(t("route_approve")))
                                .on_disabled_hover_text(format!(
                                    "{} {}",
                                    t("route_wrong_role"),
                                    role.label()
                                ))
                                .clicked()
                            {
                                decide = Some((a.id, crate::domain::ApprovalDecision::Approved));
                            }
                            if ui
                                .add_enabled(
                                    can,
                                    egui::Button::new(
                                        RichText::new(t("route_reject")).color(theme::danger()),
                                    ),
                                )
                                .clicked()
                            {
                                decide = Some((a.id, crate::domain::ApprovalDecision::Rejected));
                            }
                        });
                    }
                    ui.add_space(4.0);
                }

                // Ariza summasi o'zgargan bo'lsa marshrut eskirib qoladi —
                // buni jim o'tkazib yubormaymiz.
                let built: Vec<crate::roles::Role> = mine
                    .iter()
                    .map(|a| crate::roles::Role::parse(&a.role))
                    .collect();
                if built != route {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(t("route_stale"))
                            .size(11.0)
                            .color(theme::warn()),
                    );
                }

                ui.add_space(4.0);
                if ui
                    .small_button(t("route_rebuild"))
                    .on_hover_text(t("route_rebuild_hint"))
                    .clicked()
                {
                    clear = true;
                }
            }

            // Rad etish sababi (TZ IX.32).
            if q.status == RequestStatus::Rejected
                || matches!(
                    crate::checks::route_state(rid, &app.approvals),
                    crate::checks::RouteState::Rejected { .. }
                )
            {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new(t("reject_reason")).size(12.0).strong());
                let mut r = q.clone();
                if ui
                    .add_sized(
                        [ui.available_width().min(320.0), 46.0],
                        egui::TextEdit::multiline(&mut r.reject_reason)
                            .hint_text(t("reject_reason_hint")),
                    )
                    .changed()
                {
                    edited = Some(r);
                }
                if q.reject_reason.trim().is_empty() {
                    ui.label(
                        RichText::new(t("reject_reason_missing"))
                            .size(11.0)
                            .color(theme::warn()),
                    );
                }
            }
        });

    if build {
        for (i, role) in route.iter().enumerate() {
            app.db.insert_approval(&crate::domain::Approval {
                id: 0,
                project_id: pid,
                request_id: rid,
                step: i as i64 + 1,
                role: role.code().to_string(),
                approver: String::new(),
                decision: crate::domain::ApprovalDecision::Pending,
                decided_at: None,
                comment: String::new(),
            });
        }
        app.reload_modules();
    }
    if clear {
        app.db.clear_approvals(rid);
        app.reload_modules();
    }
    if let Some((id, decision)) = decide {
        if let Some(mut a) = app.approvals.iter().find(|a| a.id == id).cloned() {
            a.decision = decision;
            a.decided_at = Some(app.today);
            a.approver = app
                .current_user
                .and_then(|u| app.users.iter().find(|x| x.id == u))
                .map(|u| u.name.clone())
                .unwrap_or_else(|| t("route_unknown_user").to_string());
            app.db.update_approval(&a);
            app.reload_modules();

            // Marshrut natijasi ariza holatiga o'tadi — ikkalasi bir joyda
            // turmasin: holat qo'lda ham o'zgartiriladi, lekin kelishuv
            // tugagach o'zi yangilanadi.
            if let Some(mut q) = app.requests.iter().find(|q| q.id == rid).cloned() {
                match crate::checks::route_state(rid, &app.approvals) {
                    crate::checks::RouteState::Approved if q.status == RequestStatus::New => {
                        q.status = RequestStatus::Approved;
                        app.db.update_request(&q);
                        app.reload_modules();
                    }
                    crate::checks::RouteState::Rejected { .. } => {
                        q.status = RequestStatus::Rejected;
                        app.db.update_request(&q);
                        app.reload_modules();
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(q) = edited {
        app.db.update_request(&q);
        app.reload_modules();
    }
    if close {
        app.request_open = None;
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

/// To'lov reyestrini faylga yozadi (TZ IX.39).
fn save_register(app: &mut App) {
    let Some(project) = app.project().cloned() else {
        return;
    };
    let (from, to) = super::doc_period(app);
    let file = format!("REG-{}.xlsx", to.format("%Y-%m"));
    let Some(path) = rfd::FileDialog::new()
        .set_title(t("doc_save"))
        .set_file_name(&file)
        .add_filter("Excel", &["xlsx"])
        .save_file()
    else {
        return;
    };
    let inp = crate::docgen::DocInput {
        project: &project,
        parties: &app.parties,
        tasks: &app.tasks,
        today: app.today,
        from,
        to,
    };
    match crate::docgen::write_pay_register(&path, &inp, &app.requests, &app.purchases) {
        Ok(()) => app.notify(format!("{} {}", t("doc_saved"), path.display())),
        Err(e) => app.notify(format!("{}: {e}", t("doc_failed"))),
    }
}

/// Ariza tekshiruvidagi savolning matni.
fn issue_text(i: &crate::checks::RequestIssue) -> String {
    use crate::checks::RequestIssue as I;
    match i {
        I::Duplicate { number } => format!("{} {number}", t("rq_i_duplicate")),
        I::NotInEstimate => t("rq_i_not_in_estimate").to_string(),
        I::OverBudget { over } => format!("{} {}", t("rq_i_over_budget"), money(*over)),
        I::Cheaper { name, saving } => {
            format!("{} {name} — {}", t("rq_i_cheaper"), money(*saving))
        }
        I::NoTask => t("rq_i_no_task").to_string(),
        I::Banned { reason } => {
            if reason.trim().is_empty() {
                t("rq_i_banned").to_string()
            } else {
                format!("{}: {reason}", t("rq_i_banned"))
            }
        }
        I::NoSpecRef => t("rq_i_no_spec").to_string(),
        I::OverNorm { need, by_norm } => format!(
            "{}: {} > {}",
            t("rq_i_over_norm"),
            super::materials::trim_num(*need),
            super::materials::trim_num(*by_norm)
        ),
        I::NoProfession => t("rq_i_no_profession").to_string(),
        I::StaffEnough { have, need } => {
            format!("{}: {have} / {need}", t("rq_i_staff_enough"))
        }
    }
}

fn head_l(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_l(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}

fn head_r(ui: &mut egui::Ui, w: f32, s: &str) {
    super::warehouse::cell_r(ui, w, RichText::new(s).color(theme::muted()).size(11.0));
}
