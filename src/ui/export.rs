//! Joriy ekran jadvalini Excel ga chiqarish (umumiy talab).
//!
//! Har bir ekran uchun eksport kodini alohida yozish o'rniga, jadval shu yerda
//! `App` holatidan yig'iladi. Shuning uchun eksport tugmasi bitta joyda turadi
//! va yangi ekran qo'shilganda faqat shu faylga bitta tarmoq qo'shiladi.
//!
//! Sonlar son bo'lib chiqadi, matn — matn: Excel da darrov yig'indi olish yoki
//! saralash mumkin. Ekranda ko'rinmaydigan hisob qiymatlar (erkin qoldiq,
//! normativ sarf, foydalanish koeffitsiyenti) ham qo'shiladi — ular baribir
//! shu bazadan hisoblanadi.

use crate::app::{App, Screen};
use crate::docgen::{Cell, Table};
use crate::i18n::t;

/// Joriy ekran uchun eksport jadvali. Ekranda jadval bo'lmasa — `None`.
pub fn table_of(app: &App, screen: Screen) -> Option<Table> {
    let mut table = match screen {
        Screen::Portfolio => portfolio(app),
        Screen::Inspections => inspections(app),
        Screen::Contracts => contracts(app),
        Screen::Gantt => gantt(app),
        Screen::Ppr => ppr(app),
        Screen::AiCheck => issues(app),
        Screen::Estimate => estimate(app),
        Screen::ExecDocs => exec_docs(app),
        Screen::Journal => journal(app),
        Screen::Materials => materials(app),
        Screen::Warehouse => warehouse(app),
        Screen::Requests => requests(app),
        Screen::Purchases => purchases(app),
        Screen::Timesheet => timesheet(app),
        Screen::Quality => quality(app),
        Screen::Safety => safety(app),
        Screen::Machines => machines(app),
        Screen::Deals => deals(app),
        Screen::Sales => units(app),
        Screen::Analytics => analytics(app),
        // Hisobotlar ekranida Ctrl+E tanlangan hisobotni saqlaydi:
        // ekranda ko'rinib turgan narsa chiqadi, boshqasi emas.
        Screen::Reports => return Some(current_report(app)),
        Screen::Notices => notices(app),
        Screen::Director => director(app),
        _ => return None,
    };
    // Nom yon paneldagi ekran nomi bilan bir xil bo'ladi.
    table.name = screen.label().to_string();
    (!table.rows.is_empty()).then_some(table)
}

/// Ekran uchun taklif qilinadigan fayl nomi.
pub fn file_name(app: &App, screen: Screen) -> String {
    let name = table_of(app, screen)
        .map(|t| t.name)
        .unwrap_or_else(|| t("export").to_string());
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let safe = safe.trim_matches('-').to_lowercase();
    format!("qurai-{safe}-{}.xlsx", app.today.format("%Y-%m-%d"))
}

/// «Hisobotlar» ekranida tanlangan hisobot.
///
/// Tanlov ilova holatida turadi, shuning uchun Ctrl+E aynan ko'rinib
/// turgan hisobotni saqlaydi — boshqasini emas.
fn current_report(app: &App) -> Table {
    use crate::reports::{Kind, Preset};

    let kind = Kind::ALL
        .get(app.report_kind)
        .copied()
        .unwrap_or(Kind::Journal);
    let preset = Preset::ALL
        .get(app.report_preset)
        .copied()
        .unwrap_or(Preset::Month);
    let start = app.project().map(|p| p.start_date).unwrap_or(app.today);
    crate::reports::build(app, kind, preset.period(app.today, start))
}

fn txt(s: impl Into<String>) -> Cell {
    Cell::Text(s.into())
}

/// Ixtiyoriy sanani katakka aylantiradi.
fn odate(d: Option<chrono::NaiveDate>) -> Cell {
    match d {
        Some(d) => Cell::Date(d),
        None => Cell::Empty,
    }
}

fn table(name: &str, headers: &[&str], rows: Vec<Vec<Cell>>) -> Table {
    Table {
        name: name.to_string(),
        headers: headers.iter().map(|h| h.to_string()).collect(),
        rows,
    }
}

// ================================================================ E'tibor va rahbar

/// Barcha modullardan e'tibor talab qiladigan yozuvlar.
///
/// Ro'yxat `notify::collect` dan olinadi — ekranda ko'ringan bilan aynan
/// bir xil bo'lishi uchun; eksport uchun alohida hisob yozilmaydi.
fn notices(app: &App) -> Table {
    let rows = crate::notify::collect(app)
        .iter()
        .map(|n| {
            vec![
                txt(n.severity.label()),
                txt(n.source.label()),
                txt(n.code),
                txt(&n.title),
                Cell::Num(n.count as f64),
                if n.days > 0 {
                    Cell::Num(n.days as f64)
                } else {
                    Cell::Empty
                },
                txt(&n.detail),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_severity"),
            t("col_module"),
            t("col_code"),
            t("col_title"),
            t("col_count"),
            t("col_days"),
            t("col_evidence"),
        ],
        rows,
    )
}

/// Rahbar ekrani: modul kartalari bitta jadvalga tushadi.
///
/// Har qator — modulning o'z hisobidan chiqqan son; bu yerda qayta
/// hisoblanmaydi, faqat bir joyga yig'iladi.
fn director(app: &App) -> Table {
    let e = app.executive_score();
    let mut rows: Vec<Vec<Cell>> = [
        ("an_exec_schedule", e.schedule),
        ("an_exec_money", e.money),
        ("an_exec_quality", e.quality),
        ("an_exec_safety", e.safety),
        ("an_exec_supply", e.supply),
        ("an_exec_docs", e.docs),
    ]
    .iter()
    .map(|(key, score)| vec![txt(t(key)), Cell::Num(*score)])
    .collect();
    rows.push(vec![txt(t("an_exec_total")), Cell::Num(e.total)]);
    table("", &[t("col_area"), t("col_score")], rows)
}

// ================================================================ VII Tekshiruvlar

/// Uch jadval bitta varaqqa sig'maydi, shuning uchun eksportga tekshiruvlar
/// ro'yxati chiqadi — beton va geodeziya unga ustun sifatida qo'shiladi.
fn inspections(app: &App) -> Table {
    let rows = app
        .inspections
        .iter()
        .map(|x| {
            vec![
                txt(&x.number),
                txt(x.kind.label()),
                Cell::Date(x.planned),
                odate(x.done),
                txt(&x.place),
                txt(x.result.label()),
                txt(&x.requested_by),
                txt(&x.inspector),
                txt(x
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                odate(x.deadline),
                odate(x.fixed_at),
                txt(if x.open_defect() { t("yes") } else { t("no") }),
                txt(&x.note),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_kind"),
            t("in_planned"),
            t("in_done"),
            t("col_place"),
            t("col_result"),
            t("in_requested_by"),
            t("in_inspector"),
            t("col_task"),
            t("col_fix_deadline"),
            t("in_fixed_at"),
            t("in_open_defect"),
            t("col_note"),
        ],
        rows,
    )
}

// ================================================================ VIII Shartnomalar

/// Shartnomalar ro'yxati: hisoblangan ustunlar (amaldagi summa, to'langan,
/// qarz) ham chiqadi — ular ekranda ko'rinadi, lekin bazada saqlanmaydi.
fn contracts(app: &App) -> Table {
    let rows = app
        .contracts
        .iter()
        .map(|c| {
            let st = crate::checks::contract_state(
                c,
                &app.contract_changes,
                &app.payment_stages,
                app.today,
            );
            vec![
                txt(&c.number),
                txt(&c.name),
                txt(c.kind.label()),
                Cell::Date(c.signed),
                Cell::Date(c.start),
                Cell::Date(c.end),
                Cell::Money(c.base_sum()),
                Cell::Money(st.approved_changes),
                Cell::Money(st.pending_changes),
                Cell::Money(st.current),
                Cell::Num(st.approved_days as f64),
                Cell::Money(st.planned),
                Cell::Money(st.paid),
                Cell::Money(st.overdue),
                Cell::Money(c.advance()),
                Cell::Money(c.retention()),
                txt(c.status.label()),
                txt(&c.note),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_name"),
            t("col_kind"),
            t("ct_signed"),
            t("col_start"),
            t("ct_end"),
            t("ct_base"),
            t("ct_approved"),
            t("ct_pending"),
            t("ct_current"),
            t("ct_days_shift"),
            t("ct_scheduled"),
            t("ct_paid"),
            t("ct_debt"),
            t("ct_advance"),
            t("ct_retention"),
            t("col_status"),
            t("col_note"),
        ],
        rows,
    )
}

// ================================================================ XVII.4 Obyektlar

fn portfolio(app: &App) -> Table {
    let rows = crate::portfolio::summaries(&app.db, app.today)
        .into_iter()
        .map(|p| {
            vec![
                txt(&p.code),
                txt(&p.name),
                txt(p.status.label()),
                Cell::Num(p.fact_pct),
                Cell::Num(p.plan_pct),
                Cell::Num(p.deviation()),
                Cell::Num(p.delay_days as f64),
                Cell::Num(p.overdue_tasks as f64),
                Cell::Money(p.contract_sum),
                Cell::Money(p.paid_total),
                Cell::Money(p.estimate_total),
                Cell::Money(p.stock_value),
                Cell::Num(p.critical_issues as f64),
                match p.quality_score {
                    Some(v) => Cell::Num(v),
                    None => Cell::Empty,
                },
                Cell::Num(p.safety_score),
                Cell::Num(p.late_purchases as f64),
                Cell::Num(p.units_sold as f64),
                Cell::Num(p.units_total as f64),
                Cell::Money(p.sales_paid),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_code"),
            t("pf_object"),
            t("col_status"),
            t("pf_fact"),
            t("pf_plan"),
            t("pf_deviation"),
            t("col_delay_days"),
            t("col_overdue"),
            t("pf_contract_short"),
            t("pf_paid_short"),
            t("col_estimate"),
            t("pf_stock"),
            t("col_critical_count"),
            t("pf_quality"),
            t("pf_safety"),
            t("col_late_delivery"),
            t("pf_sales"),
            t("col_units_total"),
            t("pf_sales_paid"),
        ],
        rows,
    )
}

// ================================================================ I.2 GPR

fn gantt(app: &App) -> Table {
    let rows = app
        .tasks
        .iter()
        .map(|x| {
            let calc = app.schedule.get(x.id);
            vec![
                txt(&x.wbs),
                txt(&x.name),
                txt(x.section.code()),
                txt(&x.responsible),
                Cell::Date(x.plan_start),
                Cell::Num(x.duration as f64),
                odate(x.fact_start),
                odate(x.fact_end),
                Cell::Num(x.progress),
                Cell::Num(x.volume),
                txt(&x.unit),
                Cell::Num(calc.map(|c| c.slack as f64).unwrap_or(0.0)),
                txt(if app.schedule.is_critical(x.id) {
                    t("yes")
                } else {
                    t("no")
                }),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_wbs"),
            t("col_task"),
            t("col_section"),
            t("col_responsible"),
            t("col_start"),
            t("col_days"),
            t("col_fact_start"),
            t("col_fact_end"),
            t("col_progress"),
            t("col_volume"),
            t("col_unit"),
            t("col_slack"),
            t("col_critical"),
        ],
        rows,
    )
}

// ================================================================ I.3 PPR

fn ppr(app: &App) -> Table {
    let rows = app
        .ppr_docs
        .iter()
        .map(|d| {
            vec![
                txt(d.kind.label()),
                txt(&d.number),
                txt(&d.name),
                txt(d.section.code()),
                txt(d
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                txt(&d.author),
                odate(d.approved_at),
                txt(if d.approved { t("yes") } else { t("no") }),
                Cell::Num(d.workers as f64),
                Cell::Num(d.machines as f64),
                txt(&d.note),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_kind"),
            t("col_number"),
            t("col_name"),
            t("col_section"),
            t("col_task"),
            t("col_author"),
            t("col_approved_at"),
            t("col_approved"),
            t("col_workers"),
            t("col_machines"),
            t("col_note"),
        ],
        rows,
    )
}

// ================================================================ II Nomuvofiqliklar

fn issues(app: &App) -> Table {
    let rows = app
        .issues
        .iter()
        .map(|i| {
            vec![
                txt(&i.code),
                txt(i.severity.label()),
                txt(i.module.label()),
                txt(i.section.code()),
                txt(&i.title),
                txt(&i.description),
                txt(&i.element),
                txt(&i.location),
                txt(&i.sheet),
                txt(format!("{} {}", i.norm_doc, i.norm_clause).trim()),
                txt(&i.recommendation),
                txt(&i.responsible),
                odate(i.deadline),
                txt(i.status.label()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_code"),
            t("col_severity"),
            t("col_module"),
            t("col_section"),
            t("col_title"),
            t("col_description"),
            t("col_element"),
            t("col_location"),
            t("col_sheet"),
            t("col_norm"),
            t("col_recommendation"),
            t("col_responsible"),
            t("col_deadline"),
            t("col_status"),
        ],
        rows,
    )
}

// ================================================================ III Smeta

fn estimate(app: &App) -> Table {
    let rows = app
        .estimate_items
        .iter()
        .map(|x| {
            vec![
                Cell::Num(x.pos as f64),
                txt(&x.code),
                txt(&x.name),
                txt(x.section.code()),
                txt(&x.unit),
                Cell::Num(x.qty),
                Cell::Money(x.price),
                Cell::Money(x.computed()),
                Cell::Money(x.cost),
                // Hujjatdagi summa hisoblangandan farq qilsa — shu yerda ko'rinadi.
                Cell::Money(x.cost - x.computed()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_pos"),
            t("col_code"),
            t("col_name"),
            t("col_section"),
            t("col_unit"),
            t("col_qty"),
            t("col_price"),
            t("col_computed"),
            t("col_declared"),
            t("col_diff"),
        ],
        rows,
    )
}

// ================================================================ IV Ijro hujjatlari

fn exec_docs(app: &App) -> Table {
    let rows = app
        .exec_docs
        .iter()
        .map(|x| {
            vec![
                txt(&x.number),
                Cell::Date(x.date),
                txt(x.kind.label()),
                txt(&x.name),
                txt(x
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                txt(x.status.label()),
                txt(&x.responsible),
                txt(&x.note),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_date"),
            t("col_kind"),
            t("col_name"),
            t("col_task"),
            t("col_status"),
            t("col_responsible"),
            t("col_note"),
        ],
        rows,
    )
}

// ================================================================ V Jurnal

fn journal(app: &App) -> Table {
    let rows = app
        .journal
        .iter()
        .map(|x| {
            vec![
                Cell::Date(x.date),
                txt(&x.author),
                txt(x
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                Cell::Num(x.volume),
                txt(&x.unit),
                Cell::Num(x.workers as f64),
                Cell::Num(x.machines as f64),
                txt(&x.weather),
                txt(&x.text),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_date"),
            t("col_author"),
            t("col_task"),
            t("col_volume"),
            t("col_unit"),
            t("col_workers"),
            t("col_machines"),
            t("col_weather"),
            t("col_text"),
        ],
        rows,
    )
}

// ================================================================ XII Materiallar

fn materials(app: &App) -> Table {
    let stock = app.stock();
    let rows = app
        .materials
        .iter()
        .map(|m| {
            let l = stock.iter().find(|l| l.material_id == m.id);
            vec![
                txt(&m.code),
                txt(&m.name),
                txt(&m.unit),
                txt(m.section.code()),
                txt(&m.spec),
                txt(&m.cert_no),
                odate(m.cert_until),
                Cell::Num(m.min_stock),
                Cell::Money(m.price),
                Cell::Num(l.map(|l| l.balance).unwrap_or(0.0)),
                Cell::Money(l.map(|l| l.value).unwrap_or(0.0)),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_code"),
            t("col_name"),
            t("col_unit"),
            t("col_section"),
            t("col_spec"),
            t("col_cert"),
            t("col_cert_until"),
            t("col_min_stock"),
            t("col_price"),
            t("col_balance"),
            t("col_value"),
        ],
        rows,
    )
}

// ================================================================ XI Ombor

fn warehouse(app: &App) -> Table {
    let rows = app
        .stock_moves
        .iter()
        .map(|m| {
            let mat = app.materials.iter().find(|x| x.id == m.material_id);
            vec![
                Cell::Date(m.date),
                txt(m.kind.label()),
                txt(mat
                    .map(|x| format!("{} {}", x.code, x.name))
                    .unwrap_or_default()),
                txt(mat.map(|x| x.unit.clone()).unwrap_or_default()),
                Cell::Num(m.qty),
                Cell::Money(m.price),
                Cell::Money(m.qty * m.price),
                txt(&m.document),
                txt(&m.counterparty),
                txt(m
                    .warehouse_id
                    .and_then(|id| app.warehouses.iter().find(|w| w.id == id))
                    .map(|w| w.name.clone())
                    .unwrap_or_default()),
                txt(m
                    .batch_id
                    .and_then(|id| app.batches.iter().find(|b| b.id == id))
                    .map(|b| b.number.clone())
                    .unwrap_or_default()),
                txt(m
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                txt(&m.note),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_date"),
            t("col_kind"),
            t("col_material"),
            t("col_unit"),
            t("col_qty"),
            t("col_price"),
            t("col_sum"),
            t("col_document"),
            t("col_counterparty"),
            t("col_warehouse"),
            t("col_batch"),
            t("col_task"),
            t("col_note"),
        ],
        rows,
    )
}

// ================================================================ IX Arizalar

fn requests(app: &App) -> Table {
    let supply = app.supply();
    let rows = app
        .requests
        .iter()
        .map(|q| {
            let l = supply.iter().find(|l| l.request_id == q.id);
            vec![
                txt(&q.number),
                Cell::Date(q.date),
                txt(q.kind.label()),
                txt(&q.title),
                txt(q
                    .material_id
                    .and_then(|id| app.materials.iter().find(|m| m.id == id))
                    .map(|m| format!("{} {}", m.code, m.name))
                    .unwrap_or_default()),
                Cell::Num(q.qty),
                txt(&q.unit),
                Cell::Date(q.need_date),
                txt(q.priority.label()),
                txt(q.status.label()),
                txt(&q.requester),
                Cell::Num(l.map(|l| l.ordered).unwrap_or(0.0)),
                txt(&q.reject_reason),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_date"),
            t("col_kind"),
            t("col_item"),
            t("col_material"),
            t("col_qty"),
            t("col_unit"),
            t("col_need_date"),
            t("col_priority"),
            t("col_status"),
            t("col_requester"),
            t("col_ordered"),
            t("reject_reason"),
        ],
        rows,
    )
}

// ================================================================ X Xaridlar

fn purchases(app: &App) -> Table {
    let rows = app
        .purchases
        .iter()
        .map(|p| {
            vec![
                txt(&p.number),
                Cell::Date(p.date),
                txt(&p.supplier),
                txt(&p.title),
                txt(p.section.code()),
                Cell::Num(p.qty),
                Cell::Num(p.delivered_qty),
                Cell::Num(p.remaining()),
                txt(&p.unit),
                Cell::Money(p.price),
                Cell::Money(p.amount()),
                Cell::Date(p.delivery_date),
                txt(p.status.label()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_date"),
            t("col_supplier"),
            t("col_item"),
            t("col_section"),
            t("col_qty"),
            t("col_delivered"),
            t("col_left"),
            t("col_unit"),
            t("col_price"),
            t("col_sum"),
            t("col_delivery"),
            t("col_status"),
        ],
        rows,
    )
}

// ================================================================ XIII Tabel

fn timesheet(app: &App) -> Table {
    // Tabel — kunlik yozuvlar ro'yxati: Excel da o'zi jadvalga aylantiriladi.
    let rows = app
        .timesheet
        .iter()
        .map(|e| {
            let w = app.workers.iter().find(|w| w.id == e.worker_id);
            vec![
                Cell::Date(e.date),
                txt(w.map(|w| w.name.clone()).unwrap_or_default()),
                txt(w
                    .and_then(|w| w.brigade_id)
                    .and_then(|id| app.brigades.iter().find(|b| b.id == id))
                    .map(|b| b.name.clone())
                    .unwrap_or_default()),
                txt(w.map(|w| w.position.clone()).unwrap_or_default()),
                Cell::Num(e.hours),
                txt(e.kind.label()),
                txt(e.shift.label()),
                txt(e
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                Cell::Money(w.map(|w| w.hourly_rate).unwrap_or(0.0)),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_date"),
            t("col_worker"),
            t("col_brigade"),
            t("col_position"),
            t("col_hours"),
            t("cell_kind"),
            t("cell_shift"),
            t("col_task"),
            t("col_hourly_rate"),
        ],
        rows,
    )
}

// ================================================================ XIV Sifat

fn quality(app: &App) -> Table {
    let rows = app
        .quality
        .iter()
        .map(|q| {
            vec![
                Cell::Date(q.date),
                txt(q.kind.label()),
                txt(&q.subject),
                txt(q
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
                txt(q
                    .material_id
                    .and_then(|id| app.materials.iter().find(|m| m.id == id))
                    .map(|m| format!("{} {}", m.code, m.name))
                    .unwrap_or_default()),
                txt(&q.inspector),
                txt(q.result.label()),
                txt(&q.defect),
                odate(q.deadline),
                odate(q.fixed_at),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_date"),
            t("col_stage"),
            t("col_subject"),
            t("col_task"),
            t("col_material"),
            t("col_inspector"),
            t("col_result"),
            t("col_defect"),
            t("col_fix_deadline"),
            t("col_fixed"),
        ],
        rows,
    )
}

// ================================================================ XV Xavfsizlik

fn safety(app: &App) -> Table {
    let rows = app
        .safety
        .iter()
        .map(|e| {
            vec![
                Cell::Date(e.date),
                txt(e.kind.label()),
                txt(e.severity.label()),
                txt(&e.place),
                txt(&e.description),
                txt(&e.measure),
                txt(&e.responsible),
                odate(e.deadline),
                txt(e.status.label()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_date"),
            t("col_kind"),
            t("col_severity"),
            t("col_place"),
            t("col_description"),
            t("col_measure"),
            t("col_responsible"),
            t("col_deadline"),
            t("col_status"),
        ],
        rows,
    )
}

// ================================================================ XVI Mashinalar

fn machines(app: &App) -> Table {
    // Yo'l varaqalari — kunlik yozuv, park esa o'zgarmas ma'lumot.
    let rows = app
        .machine_logs
        .iter()
        .map(|l| {
            let m = app.machines.iter().find(|m| m.id == l.machine_id);
            vec![
                txt(&l.number),
                Cell::Date(l.date),
                txt(m.map(|m| m.name.clone()).unwrap_or_default()),
                txt(m.map(|m| m.reg_no.clone()).unwrap_or_default()),
                txt(&l.driver),
                txt(&l.route),
                Cell::Num(l.hours),
                Cell::Num(l.odo_start),
                Cell::Num(l.odo_end),
                Cell::Num(l.distance()),
                Cell::Num(l.fuel),
                Cell::Num(l.trips as f64),
                Cell::Num(l.cargo),
                Cell::Money(l.hours * m.map(|m| m.hour_rate).unwrap_or(0.0)),
                txt(l
                    .task_id
                    .and_then(|id| app.task(id).map(|t| format!("{} {}", t.wbs, t.name)))
                    .unwrap_or_default()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_date"),
            t("col_machine"),
            t("col_reg_no"),
            t("col_driver"),
            t("col_route_way"),
            t("col_hours"),
            t("col_odo_start"),
            t("col_odo_end"),
            t("col_distance"),
            t("col_fuel"),
            t("col_trips"),
            t("col_cargo"),
            t("col_machine_cost"),
            t("col_task"),
        ],
        rows,
    )
}

// ================================================================ XX Shartnomalar

fn deals(app: &App) -> Table {
    let rows = app
        .deals
        .iter()
        .map(|d| {
            let st = crate::sales::deal_state(d, &app.payments, app.today);
            let unit = app.units.iter().find(|u| u.id == d.unit_id);
            vec![
                txt(&d.number),
                Cell::Date(d.date),
                txt(&d.client),
                txt(&d.phone),
                txt(unit.map(|u| u.number.clone()).unwrap_or_default()),
                Cell::Num(unit.map(|u| u.area).unwrap_or(0.0)),
                txt(d.pay_kind.label()),
                Cell::Money(d.price),
                Cell::Money(d.discount),
                Cell::Money(d.total()),
                Cell::Money(st.paid),
                Cell::Money(st.remaining),
                Cell::Money(st.overdue),
                txt(d.status.label()),
                txt(&d.manager),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_number"),
            t("col_date"),
            t("col_client"),
            t("col_phone"),
            t("col_flat"),
            t("col_area"),
            t("col_pay_kind"),
            t("col_price"),
            t("col_discount"),
            t("col_total"),
            t("col_paid"),
            t("col_left"),
            t("col_overdue"),
            t("col_status"),
            t("col_manager"),
        ],
        rows,
    )
}

// ================================================================ XIX Shaxmatka

fn units(app: &App) -> Table {
    let rows = app
        .units
        .iter()
        .map(|u| {
            let block = app.blocks.iter().find(|b| b.id == u.block_id);
            // Amaldagi shartnoma: bekor qilingani raqam ham, mijoz ham
            // bo'lib ko'rinmasligi kerak edi.
            let deal = crate::sales::active_deal(&app.deals, u.id);
            vec![
                txt(block.map(|b| b.name.clone()).unwrap_or_default()),
                Cell::Num(u.floor as f64),
                txt(&u.number),
                Cell::Num(u.rooms as f64),
                Cell::Num(u.area),
                Cell::Money(u.price_per_m2),
                Cell::Money(u.price()),
                // Holat butun dastur bilan bitta qoidadan.
                txt(crate::sales::unit_status(u, &app.deals).label()),
                txt(deal.map(|d| d.number.clone()).unwrap_or_default()),
                txt(deal.map(|d| d.client.clone()).unwrap_or_default()),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_block"),
            t("col_floor"),
            t("col_flat"),
            t("col_rooms"),
            t("col_area"),
            t("col_price_m2"),
            t("col_price"),
            t("col_status"),
            t("col_contract"),
            t("col_client"),
        ],
        rows,
    )
}

// ================================================================ XVII Analitika

fn analytics(app: &App) -> Table {
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, &sales);
    let rows = crate::analytics::findings(&inp)
        .iter()
        .map(|f| {
            vec![
                txt(f.severity.label()),
                txt(f.area.label()),
                txt(f.code),
                txt(&f.fact),
                txt(&f.evidence),
                txt(&f.action),
            ]
        })
        .collect();
    table(
        "",
        &[
            t("col_severity"),
            t("col_area"),
            t("col_code"),
            t("col_fact_short"),
            t("col_evidence"),
            t("col_action"),
        ],
        rows,
    )
}
