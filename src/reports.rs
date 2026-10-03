//! Hisobotlar: har modul bo'yicha davr kesimidagi jadval.
//!
//! Hisobot **yangi hisob emas**. Har bir son o'z modulining funksiyasidan
//! olinadi: jurnal hajmi jurnaldan, ish haqi tabeldan, qarz sotuv
//! hisobidan. Shu sababli hisobotdagi son ekrandagi son bilan hech qachon
//! farq qilmaydi — ular bitta manbadan chiqadi.
//!
//! Ikkinchi qoida: **davr chegarasi ochiq**. Har hisobot sarlavhasida
//! qaysi kundan qaysi kungacha olingani yoziladi, chunki «qancha material
//! sarflandi» degan savolning javobi davrsiz ma'nosiz.
//!
//! Uchinchi qoida: **bo'sh hisobot ham hisobot**. Yozuv topilmasa jadval
//! bo'sh qaytadi va ekranda «bu davrda yozuv yo'q» deb ko'rsatiladi —
//! nol qatorlar bilan to'ldirilmaydi.

use crate::app::App;
use crate::docgen::{Cell, Table};
use crate::i18n::t;
use chrono::NaiveDate;

/// Hisobot davri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl Period {
    /// Sana davr ichidami.
    pub fn has(&self, d: NaiveDate) -> bool {
        d >= self.from && d <= self.to
    }

    /// Sarlavhada ko'rsatiladigan matn.
    pub fn label(&self) -> String {
        format!(
            "{} — {}",
            self.from.format("%d.%m.%Y"),
            self.to.format("%d.%m.%Y")
        )
    }
}

/// Tayyor davrlar — har safar sana terib o'tirmaslik uchun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Today,
    Week,
    Month,
    Quarter,
    Year,
    All,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Preset::Today,
        Preset::Week,
        Preset::Month,
        Preset::Quarter,
        Preset::Year,
        Preset::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Preset::Today => t("rp_today"),
            Preset::Week => t("rp_week"),
            Preset::Month => t("rp_month"),
            Preset::Quarter => t("rp_quarter"),
            Preset::Year => t("rp_year"),
            Preset::All => t("rp_all"),
        }
    }

    /// Davrni bugungi sanadan hisoblaydi.
    ///
    /// «Butun davr» obyektning boshlanish sanasidan olinadi: nol sanadan
    /// boshlash jadvalni ma'nosiz uzaytirardi.
    pub fn period(self, today: NaiveDate, start: NaiveDate) -> Period {
        let days = |n: i64| Period {
            from: today - chrono::Duration::days(n),
            to: today,
        };
        match self {
            Preset::Today => Period {
                from: today,
                to: today,
            },
            Preset::Week => days(6),
            Preset::Month => days(29),
            Preset::Quarter => days(89),
            Preset::Year => days(364),
            Preset::All => Period {
                from: start.min(today),
                to: today,
            },
        }
    }
}

/// Hisobot turi — TZ modullari bo'yicha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// I. Obyekt va grafik.
    Schedule,
    /// I.3. PPR qamrovi.
    Ppr,
    /// II. Loyiha tekshiruvi.
    Project,
    /// III. Smeta.
    Estimate,
    /// IV. Ijro hujjatlari.
    ExecDocs,
    /// V. Kunlik jurnal.
    Journal,
    /// VII. Texnik nazorat.
    Inspections,
    /// VIII. Shartnoma va to'lovlar.
    Contract,
    /// IX. Arizalar.
    Requests,
    /// X. Xaridlar.
    Purchases,
    /// XI. Ombor aylanmasi.
    Stock,
    /// XII. Material sarfi.
    Materials,
    /// XIII. Tabel va ish haqi.
    Timesheet,
    /// XIV. Sifat.
    Quality,
    /// XV. Xavfsizlik.
    Safety,
    /// XVI. Texnika.
    Machines,
    /// XVII. Kesishgan tahlil.
    Analytics,
    /// XIX. Sotuv: kvartiralar.
    Sales,
    /// XX. Sotuv shartnomalari va to'lovlari.
    Deals,
    /// XX. Qarzdorlik muddati bo'yicha.
    Debts,
    /// XIX. Qavatlar kesimi.
    Floors,
    /// XIX. Menejerlar kesimi.
    Managers,
    /// XIII.4. Kirish/chiqish belgilari.
    Attendance,
    /// IV.18. Imzo daftari va zanjir holati.
    Signatures,
    /// III.15. Narxlar bazasi bilan solishtirish.
    Prices,
    /// Obyekt yakuni: har modulning bosh soni bitta varaqda.
    Summary,
}

impl Kind {
    pub const ALL: [Kind; 26] = [
        Kind::Schedule,
        Kind::Ppr,
        Kind::Project,
        Kind::Estimate,
        Kind::ExecDocs,
        Kind::Journal,
        Kind::Inspections,
        Kind::Contract,
        Kind::Requests,
        Kind::Purchases,
        Kind::Stock,
        Kind::Materials,
        Kind::Timesheet,
        Kind::Quality,
        Kind::Safety,
        Kind::Machines,
        Kind::Analytics,
        Kind::Sales,
        Kind::Deals,
        Kind::Debts,
        Kind::Floors,
        Kind::Managers,
        Kind::Attendance,
        Kind::Signatures,
        Kind::Prices,
        Kind::Summary,
    ];

    /// Hisobot nomi.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Schedule => t("rp_schedule"),
            Kind::Ppr => t("rp_ppr"),
            Kind::Project => t("rp_project"),
            Kind::Estimate => t("rp_estimate"),
            Kind::ExecDocs => t("rp_execdocs"),
            Kind::Journal => t("rp_journal"),
            Kind::Inspections => t("rp_inspections"),
            Kind::Contract => t("rp_contract"),
            Kind::Requests => t("rp_requests"),
            Kind::Purchases => t("rp_purchases"),
            Kind::Stock => t("rp_stock"),
            Kind::Materials => t("rp_materials"),
            Kind::Timesheet => t("rp_timesheet"),
            Kind::Quality => t("rp_quality"),
            Kind::Safety => t("rp_safety"),
            Kind::Machines => t("rp_machines"),
            Kind::Analytics => t("rp_analytics"),
            Kind::Sales => t("rp_sales"),
            Kind::Deals => t("rp_deals"),
            Kind::Debts => t("rp_debts"),
            Kind::Floors => t("rp_floors"),
            Kind::Managers => t("rp_managers"),
            Kind::Attendance => t("rp_attendance"),
            Kind::Signatures => t("rp_signatures"),
            Kind::Prices => t("rp_prices"),
            Kind::Summary => t("rp_summary"),
        }
    }

    /// TZ moduli raqami — hisobot qaysi bo'limdan kelgani ko'rinsin.
    pub fn numeral(self) -> &'static str {
        match self {
            Kind::Schedule => "I.2",
            Kind::Ppr => "I.3",
            Kind::Project => "II",
            Kind::Estimate => "III",
            Kind::ExecDocs => "IV",
            Kind::Journal => "V",
            Kind::Inspections => "VII",
            Kind::Contract => "VIII",
            Kind::Requests => "IX",
            Kind::Purchases => "X",
            Kind::Stock => "XI",
            Kind::Materials => "XII",
            Kind::Timesheet => "XIII",
            Kind::Quality => "XIV",
            Kind::Safety => "XV",
            Kind::Machines => "XVI",
            Kind::Analytics => "XVII",
            Kind::Sales => "XIX",
            Kind::Deals => "XX",
            Kind::Debts => "XX",
            Kind::Floors => "XIX",
            Kind::Managers => "XIX",
            Kind::Attendance => "XIII.4",
            Kind::Signatures => "IV.18",
            Kind::Prices => "III.15",
            Kind::Summary => "—",
        }
    }

    /// Hisobot davrga bog'liqmi.
    ///
    /// Ba'zi hisobot holatni ko'rsatadi (ombor qoldig'i, kvartiralar
    /// ro'yxati) — ular uchun davr ma'nosiz va buni ochiq aytish kerak,
    /// aks holda foydalanuvchi davrni o'zgartirib, natija o'zgarmasligiga
    /// hayron bo'ladi.
    pub fn uses_period(self) -> bool {
        !matches!(
            self,
            Kind::Ppr
                | Kind::Project
                | Kind::Estimate
                | Kind::Sales
                | Kind::Debts
                | Kind::Floors
                | Kind::Prices
        )
    }
}

fn txt(s: impl Into<String>) -> Cell {
    Cell::Text(s.into())
}

fn table(name: String, headers: &[&str], rows: Vec<Vec<Cell>>) -> Table {
    Table {
        name,
        headers: headers.iter().map(|h| h.to_string()).collect(),
        rows,
    }
}

/// Jadval oxiriga «JAMI» qatorini qo'shadi.
///
/// Ustunlar **ataylab qo'lda ko'rsatiladi**: hamma sonni qo'shib bo'lmaydi.
/// Foizni, qavat raqamini yoki narxni qo'shish ma'nosiz son beradi va
/// hisobotga ishonchni yo'qotadi. Shuning uchun har hisobot o'zi qaysi
/// ustun yig'ilishini aytadi.
fn with_totals(mut table: Table, columns: &[usize]) -> Table {
    if table.rows.is_empty() || columns.is_empty() {
        return table;
    }
    let mut row: Vec<Cell> = Vec::with_capacity(table.headers.len());
    for i in 0..table.headers.len() {
        if !columns.contains(&i) {
            // Birinchi ustunda «JAMI» yozuvi turadi.
            row.push(if i == 0 {
                txt(t("rp_total"))
            } else {
                Cell::Empty
            });
            continue;
        }
        let mut sum = 0.0;
        let mut money = false;
        for r in &table.rows {
            match r.get(i) {
                Some(Cell::Num(v)) => sum += v,
                Some(Cell::Money(v)) => {
                    sum += v;
                    money = true;
                }
                _ => {}
            }
        }
        row.push(if money {
            Cell::Money(sum)
        } else {
            Cell::Num(sum)
        });
    }
    table.rows.push(row);
    table
}

/// Hisobotni tuzadi.
pub fn build(app: &App, kind: Kind, period: Period) -> Table {
    let name = format!("{} · {}", kind.label(), period.label());
    match kind {
        Kind::Schedule => schedule(app, name),
        Kind::Ppr => ppr(app, name),
        Kind::Project => project(app, name),
        Kind::Estimate => with_totals(estimate(app, name), &[4, 7]),
        Kind::ExecDocs => exec_docs(app, name, period),
        Kind::Journal => with_totals(journal(app, name, period), &[3]),
        Kind::Inspections => inspections(app, name, period),
        Kind::Contract => with_totals(contract(app, name, period), &[2, 4]),
        Kind::Requests => requests(app, name, period),
        Kind::Purchases => with_totals(purchases(app, name, period), &[6]),
        Kind::Stock => with_totals(stock(app, name, period), &[5]),
        Kind::Materials => with_totals(materials(app, name, period), &[4]),
        Kind::Timesheet => with_totals(timesheet(app, name, period), &[3, 4, 5, 7]),
        Kind::Quality => quality(app, name, period),
        Kind::Safety => safety(app, name, period),
        Kind::Machines => with_totals(machines(app, name, period), &[4, 5, 6, 7]),
        Kind::Analytics => analytics(app, name),
        Kind::Sales => with_totals(sales(app, name), &[5, 7]),
        Kind::Deals => with_totals(deals(app, name, period), &[5, 6, 7]),
        Kind::Debts => with_totals(debts(app, name), &[3, 4, 5]),
        Kind::Floors => with_totals(floors(app, name), &[1, 2, 4]),
        Kind::Managers => with_totals(managers(app, name), &[1, 2, 3, 4]),
        Kind::Attendance => with_totals(attendance(app, name, period), &[4, 5]),
        Kind::Signatures => signatures(app, name, period),
        Kind::Prices => prices(app, name),
        Kind::Summary => summary(app, name, period),
    }
}

/// Barcha hisobotlar — bitta kitobga yozish uchun.
pub fn build_all(app: &App, period: Period) -> Vec<Table> {
    Kind::ALL
        .iter()
        .map(|k| build(app, *k, period))
        .filter(|t| !t.rows.is_empty())
        .collect()
}

// ==================================================== XIII.4. Kirish/chiqish

/// Kirish-chiqish belgilari va ularning tabel bilan farqi.
///
/// Soat **shu yerda qayta hisoblanmaydi**: u `attend` modulidan olinadi —
/// ekranda ko'ringan son bilan hisobotdagi son bir xil bo'lishi shart.
fn attendance(app: &App, name: String, p: Period) -> Table {
    let days = app.attendance_days();
    let mismatch = app.attendance_mismatch();
    let rows = days
        .iter()
        .filter(|d| p.has(d.date))
        .map(|d| {
            // Shu kun va shu ishchi bo'yicha e'tirozlar bir qatorga
            // yig'iladi: hisobot qatori bitta bo'lishi kerak.
            let note = mismatch
                .iter()
                .filter(|m| m.date() == d.date && m.worker() == d.worker)
                .map(|m| m.text())
                .collect::<Vec<_>>()
                .join("; ");
            vec![
                Cell::Date(d.date),
                txt(&d.worker),
                txt(d
                    .first_in
                    .map(|x| x.format("%H:%M").to_string())
                    .unwrap_or_default()),
                txt(d
                    .last_out
                    .map(|x| x.format("%H:%M").to_string())
                    .unwrap_or_default()),
                Cell::Num(d.hours),
                Cell::Num(d.marks as f64),
                txt(if d.by_qr { t("at_by_qr") } else { "" }),
                txt(d.verdict.label()),
                txt(note),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_worker"),
            t("at_first_in"),
            t("at_last_out"),
            t("hours_short"),
            t("at_marks"),
            t("at_by_qr"),
            t("geo_distance"),
            t("col_note"),
        ],
        rows,
    )
}

// ======================================================= IV.18. Imzo daftari

/// Imzo daftari va zanjir holati.
///
/// Oxirgi ustun **hisobotning ma'nosi**: yozuv keyin tuzatilgan bo'lsa,
/// bu shu yerda ko'rinadi. Daftarni qog'ozga chiqarishning butun sababi
/// shu — tekshiruvchi zanjir butunligini o'zi ko'rsin.
fn signatures(app: &App, name: String, p: Period) -> Table {
    let breaks = app.sign_breaks();
    let rows = app
        .sign_log
        .iter()
        .enumerate()
        .filter(|(_, e)| p.has(e.at.date()))
        .map(|(i, e)| {
            let broken: Vec<String> = breaks
                .iter()
                .filter(|b| match b {
                    crate::signlog::Break::Chain { index, .. }
                    | crate::signlog::Break::Text { index, .. } => *index == i,
                })
                .map(|b| b.text())
                .collect();
            vec![
                Cell::Date(e.at.date()),
                txt(e.at.format("%H:%M").to_string()),
                txt(&e.document),
                txt(&e.subject),
                txt(&e.signer),
                txt(crate::roles::Role::parse(&e.role).label()),
                txt(if e.rejected.trim().is_empty() {
                    t("sl_approved").to_string()
                } else {
                    format!("{}: {}", t("sl_rejected"), e.rejected)
                }),
                txt(e.digest.chars().take(12).collect::<String>()),
                txt(if broken.is_empty() {
                    t("sl_intact").to_string()
                } else {
                    broken.join("; ")
                }),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("rp_time"),
            t("sl_col_doc"),
            t("col_name"),
            t("sl_col_signer"),
            t("col_role"),
            t("sl_col_state"),
            t("sl_col_digest"),
            t("col_check"),
        ],
        rows,
    )
}

// ====================================================== III.15. Narxlar bazasi

/// Narxlar bazasi va materiallarimizning unga nisbatan holati.
///
/// Ikkita ro'yxatni birlashtirish ataylab: bazaning o'zi hisobot emas —
/// u ma'lumotnoma. Hisobotning ma'nosi **bizning narximiz** bazadagi
/// diapazonga tushadimi degan savolda.
fn prices(app: &App, name: String) -> Table {
    let rows = app
        .materials
        .iter()
        .map(|m| {
            // Ekranda ko'ringan natija bilan bir xil bo'lishi uchun
            // hisobot ham tayyor natijadan o'qiydi.
            let check = app.price_check(m.id);
            let range = check.and_then(|c| c.range);
            let verdict = check
                .map(|c| c.verdict)
                .unwrap_or(crate::prices::Verdict::NoData);
            let trend = check.and_then(|c| c.trend);
            vec![
                txt(&m.name),
                txt(&m.unit),
                Cell::Money(m.price),
                match range {
                    Some(r) => Cell::Money(r.min),
                    None => Cell::Empty,
                },
                match range {
                    Some(r) => Cell::Money(r.max),
                    None => Cell::Empty,
                },
                match range {
                    Some(r) => Cell::Num(r.count as f64),
                    None => Cell::Empty,
                },
                txt(verdict.text()),
                match trend {
                    Some(tr) => Cell::Num((tr.per_month * 10.0).round() / 10.0),
                    None => Cell::Empty,
                },
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_material"),
            t("col_unit"),
            t("col_price"),
            t("rp_price_min"),
            t("rp_price_max"),
            t("pb_rows"),
            t("rp_price_verdict"),
            t("rp_price_trend"),
        ],
        rows,
    )
}

// ================================================================ I. Grafik

/// Ishlar grafigi: reja, fakt va kechikish.
///
/// Sanalar tarmoq hisobidan olinadi — grafik ekranidagi bilan bir xil.
fn schedule(app: &App, name: String) -> Table {
    let origin = app.origin();
    let rows = app
        .tasks
        .iter()
        .map(|task| {
            let calc = app.schedule.get(task.id);
            let start = calc
                .map(|c| origin + chrono::Duration::days(c.es.max(0)))
                .unwrap_or(task.plan_start);
            let end = calc
                .map(|c| origin + chrono::Duration::days(c.ef.max(0)))
                .unwrap_or(task.plan_start);
            vec![
                txt(&task.wbs),
                txt(&task.name),
                txt(task.section.label()),
                Cell::Date(start),
                Cell::Date(end),
                Cell::Num(task.progress),
                Cell::Num(task.volume),
                txt(&task.unit),
                txt(&task.responsible),
                txt(if app.schedule.is_critical(task.id) {
                    t("yes")
                } else {
                    t("dash")
                }),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_num"),
            t("col_task"),
            t("col_section"),
            t("col_start"),
            t("col_end"),
            "%",
            t("col_volume"),
            t("col_unit"),
            t("col_responsible"),
            t("col_critical"),
        ],
        rows,
    )
}

/// PPR qamrovi: qaysi ishda karta bor va tasdiqlanganmi.
fn ppr(app: &App, name: String) -> Table {
    let rows = app
        .tasks
        .iter()
        .map(|task| {
            let doc = app.ppr_docs.iter().find(|d| d.task_id == Some(task.id));
            vec![
                txt(&task.wbs),
                txt(&task.name),
                txt(doc.map(|d| d.number.clone()).unwrap_or_default()),
                txt(doc.map(|d| d.kind.label()).unwrap_or(t("dash"))),
                txt(match doc {
                    Some(d) if d.approved => t("yes"),
                    Some(_) => t("no"),
                    None => t("dash"),
                }),
                Cell::Num(task.progress),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_num"),
            t("col_task"),
            t("col_number"),
            t("col_kind"),
            t("col_approved"),
            "%",
        ],
        rows,
    )
}

// ================================================================ II-III

/// Loyiha tekshiruvi topilmalari.
fn project(app: &App, name: String) -> Table {
    let rows = app
        .issues
        .iter()
        .map(|i| {
            vec![
                txt(&i.code),
                txt(i.severity.label()),
                txt(i.section.label()),
                txt(&i.title),
                txt(&i.element),
                txt(i.status.label()),
                txt(&i.recommendation),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_code"),
            t("col_severity"),
            t("col_section"),
            t("col_title"),
            t("col_element"),
            t("col_status"),
            t("col_action"),
        ],
        rows,
    )
}

/// Smeta: pozitsiyalar va ularning qiymati.
fn estimate(app: &App, name: String) -> Table {
    let rows = app
        .estimate_items
        .iter()
        .map(|i| {
            vec![
                Cell::Num(i.pos as f64),
                txt(&i.code),
                txt(&i.name),
                txt(i.section.label()),
                Cell::Num(i.qty),
                txt(&i.unit),
                Cell::Money(i.price),
                Cell::Money(i.cost),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("chk_pos"),
            t("col_code"),
            t("col_name"),
            t("col_section"),
            t("col_qty"),
            t("col_unit"),
            t("col_price"),
            t("col_sum"),
        ],
        rows,
    )
}

// ================================================================ IV-V

/// Ijro hujjatlari reyestri.
fn exec_docs(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .exec_docs
        .iter()
        .filter(|d| p.has(d.date))
        .map(|d| {
            vec![
                Cell::Date(d.date),
                txt(&d.number),
                txt(d.kind.label()),
                txt(&d.name),
                txt(d.status.label()),
                txt(&d.responsible),
                txt(d.task_id.map(|id| app.task_name(id)).unwrap_or_default()),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_number"),
            t("col_kind"),
            t("col_name"),
            t("col_status"),
            t("col_responsible"),
            t("col_task"),
        ],
        rows,
    )
}

/// Kunlik jurnal: bajarilgan hajm, odam va texnika.
fn journal(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .journal
        .iter()
        .filter(|e| p.has(e.date))
        .map(|e| {
            vec![
                Cell::Date(e.date),
                txt(&e.author),
                txt(e.task_id.map(|id| app.task_name(id)).unwrap_or_default()),
                Cell::Num(e.volume),
                txt(&e.unit),
                Cell::Num(e.workers as f64),
                Cell::Num(e.machines as f64),
                txt(&e.weather),
                txt(&e.text),
                txt(&e.remarks),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_author"),
            t("col_task"),
            t("col_volume"),
            t("col_unit"),
            t("col_workers"),
            t("col_machines"),
            t("col_weather"),
            t("col_work_done"),
            t("col_remarks"),
        ],
        rows,
    )
}

// ================================================================ VII-VIII

/// Texnik nazorat tekshiruvlari.
fn inspections(app: &App, name: String, p: Period) -> Table {
    // Sana: o'tkazilgan bo'lsa o'tkazilgan kun, aks holda rejalashtirilgan.
    let rows = app
        .inspections
        .iter()
        .filter(|x| p.has(x.done.unwrap_or(x.planned)))
        .map(|x| {
            vec![
                Cell::Date(x.done.unwrap_or(x.planned)),
                txt(&x.number),
                txt(x.kind.label()),
                txt(x.result.label()),
                txt(&x.place),
                txt(&x.inspector),
                txt(&x.note),
                match x.deadline {
                    Some(d) => Cell::Date(d),
                    None => Cell::Empty,
                },
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_number"),
            t("col_kind"),
            t("col_result"),
            t("col_place"),
            t("col_inspector"),
            t("col_defect"),
            t("col_fix_deadline"),
        ],
        rows,
    )
}

/// Shartnoma bo'yicha to'lovlar va qabul.
fn contract(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .payment_stages
        .iter()
        .filter(|x| p.has(x.due))
        .map(|x| {
            vec![
                Cell::Date(x.due),
                txt(&x.basis),
                Cell::Money(x.amount),
                match x.paid_at {
                    Some(d) => Cell::Date(d),
                    None => Cell::Empty,
                },
                Cell::Money(x.paid),
                txt(if x.paid + 0.01 >= x.amount {
                    t("yes")
                } else {
                    t("no")
                }),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_due"),
            t("col_title"),
            t("col_amount"),
            t("col_paid_at"),
            t("col_paid"),
            t("col_closed"),
        ],
        rows,
    )
}

// ================================================================ IX-XII

/// Arizalar: ehtiyoj va uning holati.
fn requests(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .requests
        .iter()
        .filter(|q| p.has(q.date))
        .map(|q| {
            vec![
                Cell::Date(q.date),
                txt(&q.number),
                txt(q.kind.label()),
                txt(&q.title),
                Cell::Num(q.qty),
                txt(&q.unit),
                txt(q.status.label()),
                txt(&q.requester),
                Cell::Date(q.need_date),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_number"),
            t("col_kind"),
            t("col_title"),
            t("col_qty"),
            t("col_unit"),
            t("col_status"),
            t("col_requester"),
            t("col_need_date"),
        ],
        rows,
    )
}

/// Xaridlar: yetkazib beruvchi, summa va yetkazish.
fn purchases(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .purchases
        .iter()
        .filter(|x| p.has(x.date))
        .map(|x| {
            vec![
                Cell::Date(x.date),
                txt(&x.number),
                txt(&x.supplier),
                txt(&x.title),
                Cell::Num(x.qty),
                txt(&x.unit),
                Cell::Money(x.amount()),
                txt(x.status.label()),
                Cell::Date(x.delivery_date),
                Cell::Num(x.delivered_qty),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_number"),
            t("col_supplier"),
            t("col_title"),
            t("col_qty"),
            t("col_unit"),
            t("col_sum"),
            t("col_status"),
            t("col_delivery"),
            t("col_delivered"),
        ],
        rows,
    )
}

/// Ombor aylanmasi: davr ichidagi kirim va chiqim.
fn stock(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .stock_moves
        .iter()
        .filter(|m| p.has(m.date))
        .map(|m| {
            vec![
                Cell::Date(m.date),
                txt(m.kind.label()),
                txt(crate::ui::materials::material_label(app, m.material_id)),
                Cell::Num(m.qty),
                Cell::Money(m.price),
                Cell::Money(m.qty * m.price),
                txt(&m.document),
                txt(&m.counterparty),
                txt(m.task_id.map(|id| app.task_name(id)).unwrap_or_default()),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_kind"),
            t("col_material"),
            t("col_qty"),
            t("col_price"),
            t("col_sum"),
            t("col_document"),
            t("col_counterparty"),
            t("col_task"),
        ],
        rows,
    )
}

/// Material sarfi: davr ichida qancha ketdi va qoldiq qancha.
///
/// Qoldiq **davrga bog'liq emas** — u bugungi holat; sarf esa davr
/// ichidagi chiqim. Ikkalasi bir jadvalda turadi, chunki savol odatda
/// birga beriladi: «qancha ketdi va yana qancha bor».
fn materials(app: &App, name: String, p: Period) -> Table {
    let stock = app.stock();
    let rows = app
        .materials
        .iter()
        .map(|m| {
            let used: f64 = app
                .stock_moves
                .iter()
                .filter(|x| {
                    x.material_id == m.id
                        && p.has(x.date)
                        && matches!(x.kind, crate::domain::MoveKind::Out)
                })
                .map(|x| x.qty)
                .sum();
            let line = stock.iter().find(|l| l.material_id == m.id);
            vec![
                txt(&m.code),
                txt(&m.name),
                txt(m.section.label()),
                txt(&m.unit),
                Cell::Num(used),
                Cell::Num(line.map(|l| l.balance).unwrap_or(0.0)),
                Cell::Num(line.map(|l| l.available).unwrap_or(0.0)),
                Cell::Num(m.min_stock),
                Cell::Money(m.price),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_code"),
            t("col_name"),
            t("col_section"),
            t("col_unit"),
            t("col_used"),
            t("col_balance"),
            t("col_free"),
            t("col_min_stock"),
            t("col_price"),
        ],
        rows,
    )
}

// ================================================================ XIII-XVI

/// Tabel va ish haqi: davr ichidagi soat va hisoblangan haq.
fn timesheet(app: &App, name: String, p: Period) -> Table {
    let wages = crate::checks::wages(&app.workers, &app.timesheet, p.from, p.to);
    let rows = app
        .workers
        .iter()
        .filter(|w| w.active)
        .map(|w| {
            let line = wages.iter().find(|l| l.worker_id == w.id);
            vec![
                txt(&w.name),
                txt(&w.position),
                txt(&w.org),
                Cell::Num(line.map(|l| l.hours).unwrap_or(0.0)),
                Cell::Num(line.map(|l| l.overtime_hours).unwrap_or(0.0)),
                Cell::Num(line.map(|l| l.downtime_hours).unwrap_or(0.0)),
                Cell::Money(w.hourly_rate),
                Cell::Money(line.map(|l| l.wage).unwrap_or(0.0)),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_worker"),
            t("col_position"),
            t("col_org"),
            t("col_hours"),
            t("col_overtime"),
            t("col_downtime"),
            t("col_rate"),
            t("col_wage"),
        ],
        rows,
    )
}

/// Sifat: tekshiruvlar va nuqsonlar.
fn quality(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .quality
        .iter()
        .filter(|q| p.has(q.date))
        .map(|q| {
            vec![
                Cell::Date(q.date),
                txt(q.kind.label()),
                txt(q.result.label()),
                txt(&q.subject),
                txt(q.task_id.map(|id| app.task_name(id)).unwrap_or_default()),
                txt(&q.inspector),
                txt(&q.defect),
                match q.deadline {
                    Some(d) => Cell::Date(d),
                    None => Cell::Empty,
                },
                match q.fixed_at {
                    Some(d) => Cell::Date(d),
                    None => Cell::Empty,
                },
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_stage"),
            t("col_result"),
            t("col_subject"),
            t("col_task"),
            t("col_inspector"),
            t("col_defect"),
            t("col_fix_deadline"),
            t("col_fixed"),
        ],
        rows,
    )
}

/// Xavfsizlik: hodisalar va ularning holati.
fn safety(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .safety
        .iter()
        .filter(|s| p.has(s.date))
        .map(|s| {
            vec![
                Cell::Date(s.date),
                txt(s.kind.label()),
                txt(s.severity.label()),
                txt(s.status.label()),
                txt(&s.place),
                txt(&s.description),
                txt(&s.measure),
                txt(&s.responsible),
                match s.deadline {
                    Some(d) => Cell::Date(d),
                    None => Cell::Empty,
                },
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_kind"),
            t("col_severity"),
            t("col_status"),
            t("col_place"),
            t("col_description"),
            t("col_measure"),
            t("col_responsible"),
            t("col_fix_deadline"),
        ],
        rows,
    )
}

/// Texnika: davr ichidagi smenalar va yoqilg'i.
fn machines(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .machines
        .iter()
        .map(|m| {
            let logs: Vec<_> = app
                .machine_logs
                .iter()
                .filter(|l| l.machine_id == m.id && p.has(l.date))
                .collect();
            let hours: f64 = logs.iter().map(|l| l.hours).sum();
            let fuel: f64 = logs.iter().map(|l| l.fuel).sum();
            vec![
                txt(&m.name),
                txt(m.kind.label()),
                txt(&m.reg_no),
                txt(m.status.label()),
                Cell::Num(logs.len() as f64),
                Cell::Num(hours),
                Cell::Num(fuel),
                Cell::Money(hours * m.hour_rate),
                txt(&m.operator),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_machine"),
            t("col_kind"),
            t("col_reg_no"),
            t("col_status"),
            t("col_shifts"),
            t("col_hours"),
            t("col_fuel"),
            t("col_cost"),
            t("col_operator"),
        ],
        rows,
    )
}

/// Kesishgan tahlil topilmalari.
fn analytics(app: &App, name: String) -> Table {
    let supply = app.supply();
    let stock = app.stock();
    let cost = app.cost_summary();
    let sales = app.sales();
    let inp = app.analytics_input(&supply, &stock, &cost, sales);
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
        name,
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

// ================================================================ XIX-XX

/// Kvartiralar: holat, maydon va narx.
fn sales(app: &App, name: String) -> Table {
    let rows = app
        .units
        .iter()
        .map(|u| {
            let block = app
                .blocks
                .iter()
                .find(|b| b.id == u.block_id)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            let deal = crate::sales::active_deal(&app.deals, u.id);
            vec![
                txt(block),
                txt(&u.number),
                Cell::Num(u.floor as f64),
                txt(u.kind.label()),
                Cell::Num(u.rooms as f64),
                Cell::Num(u.area),
                Cell::Money(u.price_per_m2),
                Cell::Money(u.price()),
                txt(crate::sales::unit_status(u, &app.deals).label()),
                txt(deal.map(|d| d.client.clone()).unwrap_or_default()),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_block"),
            t("col_unit"),
            t("col_floor"),
            t("col_kind"),
            t("col_rooms"),
            t("col_area"),
            t("col_price_m2"),
            t("col_price"),
            t("col_status"),
            t("col_client"),
        ],
        rows,
    )
}

/// Sotuv shartnomalari: davr ichida tuzilganlari.
fn deals(app: &App, name: String, p: Period) -> Table {
    let rows = app
        .deals
        .iter()
        .filter(|d| p.has(d.date))
        .map(|d| {
            let state = crate::sales::deal_state(d, &app.payments, app.today);
            let unit = app
                .units
                .iter()
                .find(|u| u.id == d.unit_id)
                .map(|u| u.number.clone())
                .unwrap_or_default();
            vec![
                Cell::Date(d.date),
                txt(&d.number),
                txt(unit),
                txt(&d.client),
                txt(d.pay_kind.label()),
                Cell::Money(d.total()),
                Cell::Money(state.paid),
                Cell::Money(state.remaining),
                txt(d.status.label()),
                txt(&d.manager),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_date"),
            t("col_number"),
            t("col_unit"),
            t("col_client"),
            t("col_pay_kind"),
            t("col_total"),
            t("col_paid"),
            t("col_debt"),
            t("col_status"),
            t("col_manager"),
        ],
        rows,
    )
}

/// Qarzdorlik muddati bo'yicha (aging).
///
/// Qarz «bor yoki yo'q» degan savol emas: 90 kunlik qarz bilan bugun
/// muddati kelgan to'lov bir xil emas. Shuning uchun qarz kechikish
/// muddatiga qarab guruhlanadi.
fn debts(app: &App, name: String) -> Table {
    let rows = crate::sales::aging(&app.deals, &app.payments, app.today)
        .into_iter()
        .map(|a| {
            let unit = app
                .units
                .iter()
                .find(|u| u.id == a.unit_id)
                .map(|u| u.number.clone())
                .unwrap_or_default();
            vec![
                txt(a.number),
                txt(unit),
                txt(a.client),
                Cell::Money(a.total),
                Cell::Money(a.paid),
                Cell::Money(a.overdue),
                Cell::Num(a.days as f64),
                txt(a.bucket.label()),
                txt(a.manager),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_number"),
            t("col_unit"),
            t("col_client"),
            t("col_total"),
            t("col_paid"),
            t("col_overdue"),
            t("col_days_late"),
            t("col_bucket"),
            t("col_manager"),
        ],
        rows,
    )
}

/// Qavatlar kesimi: qoldiq va o'rtacha narx (TZ XIX).
fn floors(app: &App, name: String) -> Table {
    let rows = crate::sales::by_floor(&app.units, &app.deals)
        .into_iter()
        .map(|f| {
            vec![
                Cell::Num(f.floor as f64),
                Cell::Num(f.units as f64),
                Cell::Num(f.free as f64),
                Cell::Money(f.avg_price_m2),
                Cell::Num(f.area_free),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_floor"),
            t("col_units"),
            t("col_free"),
            t("col_price_m2"),
            t("col_area_free"),
        ],
        rows,
    )
}

/// Menejerlar kesimi (TZ XIX).
fn managers(app: &App, name: String) -> Table {
    let rows = crate::sales::by_manager(&app.deals, &app.payments, app.today)
        .into_iter()
        .map(|m| {
            vec![
                txt(if m.manager.is_empty() {
                    t("sl_no_manager").to_string()
                } else {
                    m.manager
                }),
                Cell::Num(m.deals as f64),
                Cell::Money(m.amount),
                Cell::Money(m.paid),
                Cell::Money(m.debt),
            ]
        })
        .collect();
    table(
        name,
        &[
            t("col_manager"),
            t("col_deals"),
            t("col_total"),
            t("col_paid"),
            t("col_debt"),
        ],
        rows,
    )
}

/// Obyekt yakuni: har modulning bosh soni bitta varaqda.
///
/// Bu **yig'ma**, yangi hisob emas: har qator o'z modulining
/// funksiyasidan olinadi. Shuning uchun yakundagi son modul ekranidagi
/// son bilan bir xil bo'ladi.
fn summary(app: &App, name: String, p: Period) -> Table {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut add = |module: &str, what: &str, value: Cell| {
        rows.push(vec![txt(module), txt(what), value]);
    };

    // ---- I. Grafik
    add("I.2", t("kpi_plan_today"), Cell::Num(app.progress.plan_pct));
    add("I.2", t("kpi_fact"), Cell::Num(app.progress.fact_pct));
    add(
        "I.2",
        t("kpi_delay"),
        Cell::Num(app.progress.delay_days as f64),
    );
    add(
        "I.2",
        t("att_overdue_tasks"),
        Cell::Num(app.progress.overdue.len() as f64),
    );

    // ---- II-III. Tekshiruvlar
    add("II", t("col_findings"), Cell::Num(app.issues.len() as f64));
    let cost = app.cost_summary();
    add("III", t("cl_estimate"), Cell::Money(cost.total));

    // ---- IV-V. Ijro
    add(
        "IV",
        t("rp_execdocs"),
        Cell::Num(app.exec_docs.len() as f64),
    );
    add(
        "V",
        t("rp_journal"),
        Cell::Num(app.journal.iter().filter(|e| p.has(e.date)).count() as f64),
    );

    // ---- IX-XII. Ta'minot
    add(
        "IX",
        t("rp_requests"),
        Cell::Num(app.requests.iter().filter(|q| p.has(q.date)).count() as f64),
    );
    add(
        "X",
        t("rp_purchases"),
        Cell::Money(
            app.purchases
                .iter()
                .filter(|x| p.has(x.date))
                .map(|x| x.amount())
                .sum(),
        ),
    );

    // ---- XIII. Tabel
    let wages = crate::checks::wages(&app.workers, &app.timesheet, p.from, p.to);
    add(
        "XIII",
        t("col_hours"),
        Cell::Num(wages.iter().map(|w| w.hours).sum()),
    );
    add(
        "XIII",
        t("col_wage"),
        Cell::Money(wages.iter().map(|w| w.wage).sum()),
    );

    // ---- XIV-XV. Sifat va xavfsizlik
    add(
        "XIV",
        t("kpi_defects"),
        Cell::Num(app.quality.iter().filter(|q| q.open_defect()).count() as f64),
    );
    add(
        "XV",
        t("kpi_safety_open"),
        Cell::Num(
            app.safety
                .iter()
                .filter(|s| {
                    matches!(
                        s.status,
                        crate::domain::IssueStatus::Open | crate::domain::IssueStatus::InWork
                    )
                })
                .count() as f64,
        ),
    );

    // ---- XIX-XX. Sotuv
    let funnel = crate::sales::funnel(&app.units, &app.deals);
    add("XIX", t("kpi_units"), Cell::Num(funnel.total as f64));
    add("XIX", t("sl_sold_pct"), Cell::Num(funnel.sold_pct()));
    let sales = app.sales();
    add("XX", t("kpi_received"), Cell::Money(sales.received));
    add("XX", t("kpi_debt"), Cell::Money(sales.debt));

    table(
        name,
        &[t("col_module"), t("col_indicator"), t("col_value")],
        rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    /// Sinov uchun namuna bazasi bilan ilova.
    fn app() -> (std::path::PathBuf, App) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("qurai_rp_{}_{n}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).expect("baza");
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        (path, app)
    }

    fn all_period(app: &App) -> Period {
        let start = app.project().map(|p| p.start_date).unwrap_or(app.today);
        Preset::All.period(app.today, start)
    }

    /// Har bir hisobot tuziladi, sarlavhalari bor va qatorlari
    /// sarlavhalar soniga mos keladi.
    #[test]
    fn every_report_builds_with_matching_columns() {
        let (path, app) = app();
        let period = all_period(&app);
        let mut with_rows = 0;

        for kind in Kind::ALL {
            let table = build(&app, kind, period);
            assert!(!table.name.is_empty(), "{kind:?}: nomsiz");
            assert!(!table.headers.is_empty(), "{kind:?}: sarlavhasiz");
            assert!(!kind.numeral().is_empty(), "{kind:?}: modul raqami yo'q");
            for (i, row) in table.rows.iter().enumerate() {
                assert_eq!(
                    row.len(),
                    table.headers.len(),
                    "{kind:?}: {i}-qatorda ustun soni mos emas"
                );
            }
            if !table.rows.is_empty() {
                with_rows += 1;
            }
        }
        // Namuna bazada hisobotlarning ko'pi to'ladi.
        assert!(with_rows >= 15, "faqat {with_rows} ta hisobot to'ldi");
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Davr chegarasi ishlaydi: bitta kunlik davrda faqat o'sha kunning
    /// yozuvlari qoladi.
    #[test]
    fn period_actually_filters_the_rows() {
        let (path, app) = app();
        let wide = all_period(&app);
        let narrow = Period {
            from: app.today,
            to: app.today,
        };

        let all = build(&app, Kind::Journal, wide).rows.len();
        let today = build(&app, Kind::Journal, narrow).rows.len();
        assert!(all > 0, "namunada jurnal yozuvi yo'q");
        assert!(today <= all, "tor davrda ko'proq qator chiqdi");

        // Kelajakdagi davrda hech narsa yo'q.
        let future = Period {
            from: app.today + chrono::Duration::days(3650),
            to: app.today + chrono::Duration::days(3700),
        };
        assert!(build(&app, Kind::Journal, future).rows.is_empty());
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Hisobotdagi son modul ekranidagi son bilan bir xil bo'lishi kerak:
    /// hisobot qaytadan hisoblamaydi.
    #[test]
    fn report_numbers_come_from_the_modules() {
        let (path, app) = app();
        let period = all_period(&app);

        // Grafik: qatorlar soni ishlar soniga teng.
        assert_eq!(
            build(&app, Kind::Schedule, period).rows.len(),
            app.tasks.len()
        );
        // Kvartiralar: qatorlar soni kvartiralar soni + «JAMI» qatori.
        let sales = build(&app, Kind::Sales, period);
        assert_eq!(sales.rows.len(), app.units.len() + 1);
        // Qarz hisoboti sotuv moduli bilan bir xil.
        let aging = crate::sales::aging(&app.deals, &app.payments, app.today);
        assert_eq!(build(&app, Kind::Debts, period).rows.len(), aging.len() + 1);

        // «JAMI» qatori haqiqatan yig'indi: maydonlar ustuni tekshiriladi.
        let area: f64 = app.units.iter().map(|u| u.area).sum();
        let last = sales.rows.last().expect("jami qatori");
        match last.get(5) {
            Some(crate::docgen::Cell::Num(v)) => {
                assert!((v - area).abs() < 0.01, "jami {v} ≠ {area}")
            }
            other => panic!("jami qatori noto'g'ri: {other:?}"),
        }
        // Birinchi katakda «JAMI» yozuvi turadi.
        assert!(matches!(last.first(), Some(crate::docgen::Cell::Text(s)) if !s.is_empty()));
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Yangi hisobotlar ham o'z modulidan o'qiydi (TZ XIII.4, IV.18, III.15).
    ///
    /// Hisobot ikkinchi hisob qilmasligi kerak: ekranda ko'ringan son
    /// bilan hisobotdagi son bir xil bo'lishi shart.
    #[test]
    fn the_new_reports_read_from_their_modules() {
        let (path, mut app) = app();
        let period = all_period(&app);

        // --- Kirish/chiqish: belgi qo'yamiz va hisobot uni oladi.
        let worker = app.workers.first().expect("ishchi").name.clone();
        let day = app.today;
        for (h, kind) in [
            (8u32, crate::domain::InOut::In),
            (17, crate::domain::InOut::Out),
        ] {
            app.db.insert_attendance(&crate::domain::Attendance {
                id: 0,
                project_id: app.current.expect("obyekt"),
                worker: worker.clone(),
                at: day.and_hms_opt(h, 0, 0).expect("vaqt"),
                kind,
                gps: String::new(),
                source: "list".into(),
            });
        }
        app.reload_modules();

        let days = app.attendance_days();
        let report = build(&app, Kind::Attendance, period);
        // Qatorlar soni kunlar soniga teng (+ «JAMI»).
        assert_eq!(report.rows.len(), days.len() + 1);
        // Soat modul hisobidan olinadi, qayta hisoblanmaydi.
        let hours = days.first().expect("kun").hours;
        match report.rows[0].get(4) {
            Some(crate::docgen::Cell::Num(v)) => {
                assert!((v - hours).abs() < 1e-9, "{v} ≠ {hours}")
            }
            other => panic!("soat ustuni noto'g'ri: {other:?}"),
        }

        // --- Imzo daftari: imzo qo'yamiz va hisobot uni ko'rsatadi.
        app.sign_day_report();
        let report = build(&app, Kind::Signatures, period);
        assert_eq!(report.rows.len(), app.sign_log.len());
        // Oxirgi ustunda zanjir holati: toza daftarda e'tiroz yo'q.
        match report.rows[0].last() {
            Some(crate::docgen::Cell::Text(s)) => {
                assert_eq!(s, crate::i18n::t("sl_intact"), "{s}")
            }
            other => panic!("tekshiruv ustuni noto'g'ri: {other:?}"),
        }

        // --- Narxlar bazasi: baza bo'sh bo'lsa ham hisobot chiqadi va
        // «bazada yo'q» deyiladi — jim qolmaydi.
        let report = build(&app, Kind::Prices, period);
        assert_eq!(report.rows.len(), app.materials.len());
        if let Some(row) = report.rows.first() {
            match row.get(6) {
                Some(crate::docgen::Cell::Text(s)) => {
                    assert_eq!(s, crate::i18n::t("pb_no_data"), "{s}")
                }
                other => panic!("holat ustuni noto'g'ri: {other:?}"),
            }
        }

        // Baza yuklangach hukm o'zgaradi.
        let m = app.materials.first().cloned().expect("material");
        app.db.add_prices(&[crate::prices::PriceRow {
            id: 0,
            code: String::new(),
            name: m.name.clone(),
            unit: m.unit.clone(),
            // Materialning o'z narxidan ancha past: chetlanish ko'rinishi kerak.
            price: (m.price / 3.0).max(1.0),
            source: "Sinov".into(),
            date: None,
            region: String::new(),
        }]);
        app.price_book = app.db.price_book();
        app.recompute_price_checks();
        let report = build(&app, Kind::Prices, period);
        let row = report
            .rows
            .iter()
            .find(|r| matches!(r.first(), Some(crate::docgen::Cell::Text(s)) if *s == m.name))
            .expect("material qatori");
        let verdict = app
            .price_check(m.id)
            .map(|c| c.verdict)
            .expect("solishtirish");
        assert!(
            verdict.outside(),
            "sinov ma'lumotida chetlanish yo'q: {verdict:?}"
        );
        match row.get(6) {
            Some(crate::docgen::Cell::Text(s)) => assert_eq!(*s, verdict.text()),
            other => panic!("holat ustuni noto'g'ri: {other:?}"),
        }

        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Bo'sh hisobot kitobga kirmaydi va barcha hisobot bitta faylga
    /// yoziladi.
    #[test]
    fn all_reports_go_into_one_book() {
        let (path, app) = app();
        let period = all_period(&app);
        let tables = build_all(&app, period);
        assert!(tables.len() >= 15, "kitobda {} ta varaq", tables.len());
        assert!(tables.iter().all(|t| !t.rows.is_empty()), "bo'sh varaq bor");

        let file = std::env::temp_dir().join(format!("qurai_book_{}.xlsx", std::process::id()));
        let _ = std::fs::remove_file(&file);
        crate::docgen::write_book(&file, &tables).expect("kitob yozilmadi");
        let size = std::fs::metadata(&file).expect("fayl").len();
        assert!(size > 5_000, "fayl juda kichik: {size}");
        let _ = std::fs::remove_file(&file);

        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Ctrl+E hisobotlar ekranida aynan tanlangan hisobotni beradi.
    #[test]
    fn export_follows_the_selected_report() {
        let (path, mut app) = app();
        app.screen = crate::app::Screen::Reports;
        // «Kvartiralar» hisoboti tanlanadi.
        let index = Kind::ALL.iter().position(|k| *k == Kind::Sales).unwrap();
        app.report_kind = index;

        let table =
            crate::ui::export::table_of(&app, crate::app::Screen::Reports).expect("hisobot");
        assert!(
            table.name.starts_with(Kind::Sales.label()),
            "{}",
            table.name
        );
        // Kvartiralar soni + «JAMI» qatori.
        assert_eq!(table.rows.len(), app.units.len() + 1);

        // Boshqa hisobot tanlansa, eksport ham o'zgaradi.
        app.report_kind = Kind::ALL.iter().position(|k| *k == Kind::Summary).unwrap();
        let other =
            crate::ui::export::table_of(&app, crate::app::Screen::Reports).expect("hisobot");
        assert!(
            other.name.starts_with(Kind::Summary.label()),
            "{}",
            other.name
        );
        assert_ne!(other.rows.len(), table.rows.len());
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Obyekt yakuni har modulning sonini modul funksiyasidan oladi.
    #[test]
    fn summary_matches_the_modules() {
        let (path, app) = app();
        let period = all_period(&app);
        let table = build(&app, Kind::Summary, period);
        assert!(table.rows.len() >= 15, "yakunda {} qator", table.rows.len());

        // Kvartiralar soni sotuv modulidagi bilan bir xil.
        let funnel = crate::sales::funnel(&app.units, &app.deals);
        let row = table
            .rows
            .iter()
            .find(|r| matches!(r.first(), Some(crate::docgen::Cell::Text(s)) if s == "XIX"))
            .expect("sotuv qatori");
        match row.get(2) {
            Some(crate::docgen::Cell::Num(v)) => {
                assert_eq!(*v as usize, funnel.total, "kvartira soni farq qildi")
            }
            other => panic!("qiymat noto'g'ri: {other:?}"),
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }

    /// Davrga bog'liq bo'lmagan hisobot davr o'zgarganda o'zgarmaydi.
    #[test]
    fn state_reports_ignore_the_period() {
        let (path, app) = app();
        let a = Period {
            from: app.today,
            to: app.today,
        };
        let b = all_period(&app);
        for kind in Kind::ALL.iter().filter(|k| !k.uses_period()) {
            assert_eq!(
                build(&app, *kind, a).rows.len(),
                build(&app, *kind, b).rows.len(),
                "{kind:?} davrga bog'lanib qolgan"
            );
        }
        drop(app);
        let _ = std::fs::remove_file(&path);
    }
}
