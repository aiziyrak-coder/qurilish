//! Kesishgan tahlil (TZ XVII).
//!
//! Bu modul yangi ma'lumot o'ylab topmaydi va hukm chiqarmaydi (TZ III.32) —
//! u boshqa modullardagi faktlarni bir joyga yig'ib, e'tibor talab qiladigan
//! holatlarni tartiblab beradi. Har bir topilma uchta qismga ajratilgan:
//! **fakt** (nima kuzatildi), **hisob** (qaysi sonlardan chiqdi) va **tavsiya**
//! (nima qilish mumkin). Xulosa «bo'lishi mumkin» ohangida yoziladi.
//!
//! Har bir qoidaning barqaror kodi bor — hisobot ishga tushirishlar orasida
//! solishtiriladigan bo'lsin.

use crate::app::Screen;
use crate::checks::{CostSummary, StockLine, SupplyLine};
use crate::cpm::{Progress, Schedule};
use crate::domain::{
    ExecDoc, ExecDocStatus, Issue, IssueStatus, Machine, MachineLog, Material, Payment,
    PurchaseStatus, QualityCheck, QualityResult, Request, SafetyEvent, SafetyKind, Severity,
    StockMove, TimesheetEntry, Unit, Worker,
};
use crate::domain::{Deal, MachineStatus, Purchase};
use crate::i18n::t;
use crate::model::Task;
use crate::sales::SalesSummary;
use chrono::NaiveDate;

/// Tahlil yo'nalishi — topilmalar shu bo'yicha guruhlanadi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Schedule,
    Cost,
    Docs,
    Supply,
    Quality,
    Safety,
    Resources,
    Sales,
}

impl Area {
    pub const ALL: &'static [Area] = &[
        Area::Schedule,
        Area::Cost,
        Area::Docs,
        Area::Supply,
        Area::Quality,
        Area::Safety,
        Area::Resources,
        Area::Sales,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Area::Schedule => t("area_schedule"),
            Area::Cost => t("area_cost"),
            Area::Docs => t("area_docs"),
            Area::Supply => t("area_supply"),
            Area::Quality => t("area_quality"),
            Area::Safety => t("area_safety"),
            Area::Resources => t("area_resources"),
            Area::Sales => t("area_sales"),
        }
    }

    /// Yo'nalish qaysi ekranda ochiladi.
    pub fn screen(self) -> Screen {
        match self {
            Area::Schedule => Screen::Gantt,
            Area::Cost => Screen::Estimate,
            Area::Docs => Screen::ExecDocs,
            Area::Supply => Screen::Requests,
            Area::Quality => Screen::Quality,
            Area::Safety => Screen::Safety,
            Area::Resources => Screen::Machines,
            Area::Sales => Screen::Deals,
        }
    }
}

/// Bitta topilma.
#[derive(Debug, Clone)]
pub struct Finding {
    pub area: Area,
    /// Barqaror kod — hisobotlarni solishtirish uchun.
    pub code: &'static str,
    pub severity: Severity,
    /// Fakt: nima kuzatildi.
    pub fact: String,
    /// Hisob: qaysi sonlardan chiqdi.
    pub evidence: String,
    /// Tavsiya: nima qilish mumkin.
    pub action: String,
    /// Qaysi ekranda ko'rish mumkin.
    pub screen: Screen,
}

/// Ko'rsatkich kartochkasi uchun bitta qiymat.
#[derive(Debug, Clone)]
pub struct Metric {
    pub area: Area,
    pub title: String,
    pub value: String,
    pub hint: String,
    pub severity: Severity,
}

/// Tahlil uchun kirish ma'lumoti — barcha modullardan bir marta yig'iladi.
pub struct Input<'a> {
    pub today: NaiveDate,
    pub tasks: &'a [Task],
    pub schedule: &'a Schedule,
    pub progress: &'a Progress,
    pub issues: &'a [Issue],
    pub exec_docs: &'a [ExecDoc],
    pub requests: &'a [Request],
    pub purchases: &'a [Purchase],
    pub supply: &'a [SupplyLine],
    pub stock: &'a [StockLine],
    pub stock_moves: &'a [StockMove],
    pub materials: &'a [Material],
    pub quality: &'a [QualityCheck],
    pub safety: &'a [SafetyEvent],
    pub machines: &'a [Machine],
    pub machine_logs: &'a [MachineLog],
    pub workers: &'a [Worker],
    pub timesheet: &'a [TimesheetEntry],
    pub units: &'a [Unit],
    pub deals: &'a [Deal],
    pub payments: &'a [Payment],
    pub cost: &'a CostSummary,
    pub sales: &'a SalesSummary,
    /// Shartnoma summasi (pasportdan) — moliyaviy solishtirish uchun.
    pub contract_sum: f64,
    pub paid_total: f64,
}

/// Sog'lomlik indeksi: 100 dan topilmalar og'irligi ayriladi.
///
/// Bu ball baho emas, e'tibor o'lchovi: qancha ko'p va qancha jiddiy topilma
/// bo'lsa, shuncha past. Hisob ochiq bo'lishi uchun har daraja narxi shu yerda.
pub fn health(findings: &[Finding]) -> i64 {
    let cost = |s: Severity| match s {
        Severity::Critical => 15,
        Severity::Major => 8,
        Severity::Warning => 4,
        _ => 0,
    };
    let total: i64 = findings.iter().map(|f| cost(f.severity)).sum();
    (100 - total).clamp(0, 100)
}

/// Muhimlik bo'yicha tartib — ro'yxatni saralashda ishlatiladi.
fn rank(s: Severity) -> u8 {
    match s {
        Severity::Critical => 0,
        Severity::Major => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
        Severity::Ok => 4,
    }
}

/// Obyekt bo'yicha barcha topilmalar, muhimligi bo'yicha tartiblangan.
pub fn findings(inp: &Input) -> Vec<Finding> {
    let mut out = Vec::new();
    schedule_rules(inp, &mut out);
    cost_rules(inp, &mut out);
    docs_rules(inp, &mut out);
    supply_rules(inp, &mut out);
    quality_rules(inp, &mut out);
    safety_rules(inp, &mut out);
    resource_rules(inp, &mut out);
    sales_rules(inp, &mut out);
    // Kod bo'yicha ikkilamchi saralash — tartib ishga tushirishlar orasida barqaror.
    out.sort_by(|a, b| rank(a.severity).cmp(&rank(b.severity)).then(a.code.cmp(b.code)));
    out
}

fn push(
    out: &mut Vec<Finding>,
    area: Area,
    code: &'static str,
    severity: Severity,
    fact: String,
    evidence: String,
    action: String,
) {
    out.push(Finding {
        area,
        code,
        severity,
        fact,
        evidence,
        action,
        screen: area.screen(),
    });
}

// ================================================================ Muddat

fn schedule_rules(inp: &Input, out: &mut Vec<Finding>) {
    let p = inp.progress;

    // Bog'lanishlar tsikli — hisob ishonchsiz bo'lib qoladi.
    if !inp.schedule.cycles.is_empty() {
        push(
            out,
            Area::Schedule,
            "AN-S1",
            Severity::Critical,
            t("an_s1_fact").to_string(),
            format!("{}: {}", t("an_tasks"), inp.schedule.cycles.len()),
            t("an_s1_action").to_string(),
        );
    }

    if p.delay_days > 0 {
        // Kechikish loyiha davomiyligiga nisbatan baholanadi.
        let share = if inp.schedule.project_days > 0 {
            p.delay_days as f64 / inp.schedule.project_days as f64
        } else {
            0.0
        };
        let severity = if share > 0.10 {
            Severity::Critical
        } else if share > 0.03 {
            Severity::Major
        } else {
            Severity::Warning
        };
        push(
            out,
            Area::Schedule,
            "AN-S2",
            severity,
            t("an_s2_fact").to_string(),
            format!(
                "{}: {:.1}% / {:.1}%, {} {} {}",
                t("an_plan_fact"),
                p.plan_pct,
                p.fact_pct,
                t("an_delay"),
                p.delay_days,
                t("days_short")
            ),
            t("an_s2_action").to_string(),
        );
    }

    if !p.overdue.is_empty() {
        push(
            out,
            Area::Schedule,
            "AN-S3",
            if p.overdue.len() > 5 {
                Severity::Major
            } else {
                Severity::Warning
            },
            t("an_s3_fact").to_string(),
            format!("{}: {}", t("an_tasks"), p.overdue.len()),
            t("an_s3_action").to_string(),
        );
    }

    // Kritik yo'ldagi kechikkan ishlar — umumiy muddatga to'g'ridan-to'g'ri ta'sir qiladi.
    let critical_overdue = p
        .overdue
        .iter()
        .filter(|id| inp.schedule.is_critical(**id))
        .count();
    if critical_overdue > 0 {
        push(
            out,
            Area::Schedule,
            "AN-S4",
            Severity::Critical,
            t("an_s4_fact").to_string(),
            format!("{}: {critical_overdue}", t("an_tasks")),
            t("an_s4_action").to_string(),
        );
    }
}

// ================================================================ Pul

fn cost_rules(inp: &Input, out: &mut Vec<Finding>) {
    let c = inp.cost;

    if c.volume_excess > 0.0 {
        push(
            out,
            Area::Cost,
            "AN-C1",
            Severity::Major,
            t("an_c1_fact").to_string(),
            format!("{}: {}", t("an_amount"), crate::ui::money(c.volume_excess)),
            t("an_c1_action").to_string(),
        );
    }
    if c.duplicate_count > 0 {
        push(
            out,
            Area::Cost,
            "AN-C2",
            Severity::Major,
            t("an_c2_fact").to_string(),
            format!(
                "{}: {} · {}",
                c.duplicate_count,
                t("an_positions"),
                crate::ui::money(c.duplicate_cost)
            ),
            t("an_c2_action").to_string(),
        );
    }
    if c.price_saving > 0.0 {
        push(
            out,
            Area::Cost,
            "AN-C3",
            Severity::Warning,
            t("an_c3_fact").to_string(),
            format!("{}: {}", t("an_amount"), crate::ui::money(c.price_saving)),
            t("an_c3_action").to_string(),
        );
    }

    // Smeta shartnoma summasidan oshgan bo'lsa — moliyaviy xavf.
    if inp.contract_sum > 0.0 && c.total > inp.contract_sum * 1.001 {
        push(
            out,
            Area::Cost,
            "AN-C4",
            Severity::Critical,
            t("an_c4_fact").to_string(),
            format!(
                "{} {} · {} {}",
                t("an_estimate"),
                crate::ui::money(c.total),
                t("an_contract"),
                crate::ui::money(inp.contract_sum)
            ),
            t("an_c4_action").to_string(),
        );
    }

    // Bajarilish moliyalashtirishdan oldinda ketgan — kassa uzilishi ehtimoli.
    if inp.contract_sum > 0.0 && inp.progress.fact_pct > 1.0 {
        let earned = inp.contract_sum * inp.progress.fact_pct / 100.0;
        if earned > inp.paid_total * 1.15 && earned - inp.paid_total > 0.0 {
            push(
                out,
                Area::Cost,
                "AN-C5",
                Severity::Warning,
                t("an_c5_fact").to_string(),
                format!(
                    "{} {} · {} {}",
                    t("an_earned"),
                    crate::ui::money(earned),
                    t("an_paid"),
                    crate::ui::money(inp.paid_total)
                ),
                t("an_c5_action").to_string(),
            );
        }
    }
}

// ================================================================ Hujjatlar

fn docs_rules(inp: &Input, out: &mut Vec<Finding>) {
    // Tugallangan, ammo imzolangan hujjati yo'q ishlar.
    let undocumented: Vec<&Task> = inp
        .tasks
        .iter()
        .filter(|t| t.progress >= 99.99)
        .filter(|t| {
            !inp.exec_docs
                .iter()
                .any(|d| d.task_id == Some(t.id) && d.status == ExecDocStatus::Signed)
        })
        .collect();
    if !undocumented.is_empty() {
        push(
            out,
            Area::Docs,
            "AN-D1",
            if undocumented.len() > 3 {
                Severity::Major
            } else {
                Severity::Warning
            },
            t("an_d1_fact").to_string(),
            format!("{}: {}", t("an_tasks"), undocumented.len()),
            t("an_d1_action").to_string(),
        );
    }

    // AI tekshiruvining yopilmagan kritik nomuvofiqliklari.
    let open_critical = inp
        .issues
        .iter()
        .filter(|i| {
            i.severity == Severity::Critical
                && matches!(i.status, IssueStatus::Open | IssueStatus::InWork)
        })
        .count();
    if open_critical > 0 {
        push(
            out,
            Area::Docs,
            "AN-D2",
            Severity::Critical,
            t("an_d2_fact").to_string(),
            format!("{}: {open_critical}", t("an_issues")),
            t("an_d2_action").to_string(),
        );
    }
}

// ================================================================ Ta'minot

fn supply_rules(inp: &Input, out: &mut Vec<Finding>) {
    let late = inp.supply.iter().filter(|l| l.late).count();
    if late > 0 {
        push(
            out,
            Area::Supply,
            "AN-P1",
            Severity::Major,
            t("an_p1_fact").to_string(),
            format!("{}: {late}", t("an_requests")),
            t("an_p1_action").to_string(),
        );
    }

    // Yetkazilgan, ammo omborga kirim qilinmagan xaridlar.
    let unposted = inp
        .purchases
        .iter()
        .filter(|p| {
            matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
                && p.qty > 0.0
                && !inp.stock_moves.iter().any(|m| m.document == p.number)
        })
        .count();
    if unposted > 0 {
        push(
            out,
            Area::Supply,
            "AN-P2",
            Severity::Warning,
            t("an_p2_fact").to_string(),
            format!("{}: {unposted}", t("an_purchases")),
            t("an_p2_action").to_string(),
        );
    }

    let negative = inp.stock.iter().filter(|l| l.negative).count();
    if negative > 0 {
        push(
            out,
            Area::Supply,
            "AN-P3",
            Severity::Critical,
            t("an_p3_fact").to_string(),
            format!("{}: {negative}", t("an_materials")),
            t("an_p3_action").to_string(),
        );
    }

    let below: Vec<&StockLine> = inp.stock.iter().filter(|l| l.below_min).collect();
    if !below.is_empty() {
        // Ochiq arizasi bor materiallar hisobga olinmaydi — ish allaqachon boshlangan.
        let uncovered = below
            .iter()
            .filter(|l| {
                !inp.requests.iter().any(|q| {
                    q.material_id == Some(l.material_id)
                        && !matches!(
                            q.status,
                            crate::domain::RequestStatus::Closed
                                | crate::domain::RequestStatus::Rejected
                        )
                })
            })
            .count();
        if uncovered > 0 {
            push(
                out,
                Area::Supply,
                "AN-P4",
                Severity::Warning,
                t("an_p4_fact").to_string(),
                format!("{}: {uncovered}", t("an_materials")),
                t("an_p4_action").to_string(),
            );
        }
    }

    // Sertifikat muddati o'tgan, ammo qoldig'i bor material.
    let expired: Vec<&Material> = inp
        .materials
        .iter()
        .filter(|m| m.cert_until.is_some_and(|d| d < inp.today))
        .filter(|m| {
            inp.stock
                .iter()
                .any(|l| l.material_id == m.id && l.balance > 0.0)
        })
        .collect();
    if !expired.is_empty() {
        push(
            out,
            Area::Supply,
            "AN-P5",
            Severity::Critical,
            t("an_p5_fact").to_string(),
            format!(
                "{}: {}",
                t("an_materials"),
                expired
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            t("an_p5_action").to_string(),
        );
    }
}

// ================================================================ Sifat

fn quality_rules(inp: &Input, out: &mut Vec<Finding>) {
    let fail = inp
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Fail)
        .count();
    let overdue = inp
        .quality
        .iter()
        .filter(|q| {
            q.result != QualityResult::Pass && q.deadline.is_some_and(|d| d < inp.today)
        })
        .count();

    if overdue > 0 {
        push(
            out,
            Area::Quality,
            "AN-Q1",
            Severity::Critical,
            t("an_q1_fact").to_string(),
            format!("{}: {overdue}", t("an_defects")),
            t("an_q1_action").to_string(),
        );
    }
    if fail > 0 && inp.quality.len() >= 3 {
        let share = fail as f64 / inp.quality.len() as f64 * 100.0;
        if share >= 15.0 {
            push(
                out,
                Area::Quality,
                "AN-Q2",
                Severity::Major,
                t("an_q2_fact").to_string(),
                format!("{fail} / {} ({share:.0}%)", inp.quality.len()),
                t("an_q2_action").to_string(),
            );
        }
    }

    // Tugallangan ish bo'yicha qabul nazorati yozilmagan.
    let done_without_acceptance = inp
        .tasks
        .iter()
        .filter(|t| t.progress >= 99.99)
        .filter(|t| {
            !inp.quality.iter().any(|q| {
                q.task_id == Some(t.id)
                    && q.kind == crate::domain::QualityKind::Acceptance
            })
        })
        .count();
    if done_without_acceptance > 2 {
        push(
            out,
            Area::Quality,
            "AN-Q3",
            Severity::Warning,
            t("an_q3_fact").to_string(),
            format!("{}: {done_without_acceptance}", t("an_tasks")),
            t("an_q3_action").to_string(),
        );
    }
}

// ================================================================ Xavfsizlik

fn safety_rules(inp: &Input, out: &mut Vec<Finding>) {
    let overdue = inp
        .safety
        .iter()
        .filter(|s| {
            matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                && s.deadline.is_some_and(|d| d < inp.today)
        })
        .count();
    if overdue > 0 {
        push(
            out,
            Area::Safety,
            "AN-X1",
            Severity::Critical,
            t("an_x1_fact").to_string(),
            format!("{}: {overdue}", t("an_events")),
            t("an_x1_action").to_string(),
        );
    }

    let incidents = inp
        .safety
        .iter()
        .filter(|s| s.kind == SafetyKind::Incident)
        .count();
    if incidents > 0 {
        push(
            out,
            Area::Safety,
            "AN-X2",
            Severity::Critical,
            t("an_x2_fact").to_string(),
            format!("{}: {incidents}", t("an_events")),
            t("an_x2_action").to_string(),
        );
    }

    // Ishchilar bor, instruktaj esa yozilmagan.
    let trainings = inp
        .safety
        .iter()
        .filter(|s| s.kind == SafetyKind::Training)
        .count();
    if trainings == 0 && !inp.workers.is_empty() {
        push(
            out,
            Area::Safety,
            "AN-X3",
            Severity::Major,
            t("an_x3_fact").to_string(),
            format!("{}: {}", t("an_workers"), inp.workers.len()),
            t("an_x3_action").to_string(),
        );
    }
}

// ================================================================ Resurslar

fn resource_rules(inp: &Input, out: &mut Vec<Finding>) {
    // Texnik ko'rigi tugagan texnikada smena yozilgan.
    let violating: Vec<&Machine> = inp
        .machines
        .iter()
        .filter(|m| {
            let Some(until) = m.inspection_until else {
                return false;
            };
            until < inp.today
                && inp
                    .machine_logs
                    .iter()
                    .any(|l| l.machine_id == m.id && l.date > until)
        })
        .collect();
    if !violating.is_empty() {
        push(
            out,
            Area::Resources,
            "AN-R1",
            Severity::Critical,
            t("an_r1_fact").to_string(),
            violating
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<_>>()
                .join(", "),
            t("an_r1_action").to_string(),
        );
    }

    // Muddati o'tgan texnik ko'rik (smena yozilmagan bo'lsa ham).
    let expired = inp
        .machines
        .iter()
        .filter(|m| m.inspection_until.is_some_and(|d| d < inp.today))
        .count();
    if expired > violating.len() {
        push(
            out,
            Area::Resources,
            "AN-R2",
            Severity::Warning,
            t("an_r2_fact").to_string(),
            format!("{}: {expired}", t("an_machines")),
            t("an_r2_action").to_string(),
        );
    }

    // «Ishlamoqda» deb belgilangan, ammo oxirgi 14 kunda smenasi yo'q texnika.
    let since = inp.today - chrono::Duration::days(14);
    let idle_but_working = inp
        .machines
        .iter()
        .filter(|m| m.status == MachineStatus::Working)
        .filter(|m| {
            !inp.machine_logs
                .iter()
                .any(|l| l.machine_id == m.id && l.date >= since)
        })
        .count();
    if idle_but_working > 0 {
        push(
            out,
            Area::Resources,
            "AN-R3",
            Severity::Warning,
            t("an_r3_fact").to_string(),
            format!("{}: {idle_but_working}", t("an_machines")),
            t("an_r3_action").to_string(),
        );
    }

    // Ishchilar ro'yxatda, tabel esa oxirgi haftada bo'sh.
    let week_ago = inp.today - chrono::Duration::days(7);
    let has_recent = inp.timesheet.iter().any(|e| e.date >= week_ago);
    if !inp.workers.is_empty() && !has_recent {
        push(
            out,
            Area::Resources,
            "AN-R4",
            Severity::Warning,
            t("an_r4_fact").to_string(),
            format!("{}: {}", t("an_workers"), inp.workers.len()),
            t("an_r4_action").to_string(),
        );
    }
}

// ================================================================ Sotuv

fn sales_rules(inp: &Input, out: &mut Vec<Finding>) {
    if inp.units.is_empty() {
        return;
    }
    let s = inp.sales;

    if s.overdue > 0.0 {
        push(
            out,
            Area::Sales,
            "AN-V1",
            Severity::Major,
            t("an_v1_fact").to_string(),
            format!("{}: {}", t("an_amount"), crate::ui::money(s.overdue)),
            t("an_v1_action").to_string(),
        );
    }

    // Sotuv qurilishdan orqada qolgan — moliyalashtirish xavfi.
    let sold_pct = if s.units > 0 {
        s.sold as f64 / s.units as f64 * 100.0
    } else {
        0.0
    };
    if inp.progress.fact_pct > 30.0 && sold_pct + 15.0 < inp.progress.fact_pct {
        push(
            out,
            Area::Sales,
            "AN-V2",
            Severity::Warning,
            t("an_v2_fact").to_string(),
            format!(
                "{} {sold_pct:.0}% · {} {:.0}%",
                t("an_sold"),
                t("an_built"),
                inp.progress.fact_pct
            ),
            t("an_v2_action").to_string(),
        );
    }

    // Band qilingan, ammo uzoq vaqt imzolanmagan shartnomalar.
    let stale = inp
        .deals
        .iter()
        .filter(|d| d.status == crate::domain::DealStatus::Reserved)
        .filter(|d| (inp.today - d.date).num_days() > 30)
        .count();
    if stale > 0 {
        push(
            out,
            Area::Sales,
            "AN-V3",
            Severity::Warning,
            t("an_v3_fact").to_string(),
            format!("{}: {stale}", t("an_deals")),
            t("an_v3_action").to_string(),
        );
    }

    // Shartnomasi bor, grafigi esa tuzilmagan.
    let no_schedule = inp
        .deals
        .iter()
        .filter(|d| d.status != crate::domain::DealStatus::Cancelled)
        .filter(|d| !inp.payments.iter().any(|p| p.deal_id == d.id))
        .count();
    if no_schedule > 0 {
        push(
            out,
            Area::Sales,
            "AN-V4",
            Severity::Warning,
            t("an_v4_fact").to_string(),
            format!("{}: {no_schedule}", t("an_deals")),
            t("an_v4_action").to_string(),
        );
    }
}

// ================================================================ Ko'rsatkichlar

/// Yo'nalishlar bo'yicha asosiy ko'rsatkichlar.
pub fn metrics(inp: &Input) -> Vec<Metric> {
    let mut out = Vec::new();
    let m = |area: Area, title: &str, value: String, hint: String, severity: Severity| Metric {
        area,
        title: title.to_string(),
        value,
        hint,
        severity,
    };

    let p = inp.progress;
    out.push(m(
        Area::Schedule,
        t("an_m_progress"),
        format!("{:.0}%", p.fact_pct),
        format!("{} {:.0}%", t("an_plan"), p.plan_pct),
        if p.delay_days > 0 {
            Severity::Warning
        } else {
            Severity::Ok
        },
    ));
    out.push(m(
        Area::Schedule,
        t("an_m_delay"),
        format!("{} {}", p.delay_days, t("days_short")),
        match p.forecast_end {
            Some(d) => format!("{} {}", t("an_forecast"), d.format("%d.%m.%Y")),
            None => String::new(),
        },
        if p.delay_days > 0 {
            Severity::Major
        } else {
            Severity::Ok
        },
    ));

    out.push(m(
        Area::Cost,
        t("an_m_estimate"),
        crate::ui::money(inp.cost.total),
        format!("{} {}", t("an_contract"), crate::ui::money(inp.contract_sum)),
        if inp.contract_sum > 0.0 && inp.cost.total > inp.contract_sum {
            Severity::Critical
        } else {
            Severity::Ok
        },
    ));

    let open_issues = inp
        .issues
        .iter()
        .filter(|i| matches!(i.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    out.push(m(
        Area::Docs,
        t("an_m_issues"),
        open_issues.to_string(),
        t("an_m_issues_hint").to_string(),
        if open_issues == 0 {
            Severity::Ok
        } else {
            Severity::Warning
        },
    ));

    let stock_value: f64 = inp.stock.iter().map(|l| l.value).sum();
    out.push(m(
        Area::Supply,
        t("an_m_stock"),
        crate::ui::money(stock_value),
        format!(
            "{} {}",
            inp.stock.iter().filter(|l| l.below_min).count(),
            t("an_below_min")
        ),
        if inp.stock.iter().any(|l| l.negative) {
            Severity::Critical
        } else if inp.stock.iter().any(|l| l.below_min) {
            Severity::Warning
        } else {
            Severity::Ok
        },
    ));

    let pass = inp
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Pass)
        .count();
    let qpct = if inp.quality.is_empty() {
        100.0
    } else {
        pass as f64 / inp.quality.len() as f64 * 100.0
    };
    out.push(m(
        Area::Quality,
        t("an_m_quality"),
        format!("{qpct:.0}%"),
        format!("{} {}", inp.quality.len(), t("an_checks")),
        if qpct >= 90.0 {
            Severity::Ok
        } else {
            Severity::Warning
        },
    ));

    let open_safety = inp
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    out.push(m(
        Area::Safety,
        t("an_m_safety"),
        open_safety.to_string(),
        t("an_m_safety_hint").to_string(),
        if open_safety == 0 {
            Severity::Ok
        } else {
            Severity::Warning
        },
    ));

    // Oxirgi 7 kundagi ish soati va texnika motosoati.
    let week = inp.today - chrono::Duration::days(7);
    let hours: f64 = inp
        .timesheet
        .iter()
        .filter(|e| e.date >= week)
        .map(|e| e.hours)
        .sum();
    let mhours: f64 = inp
        .machine_logs
        .iter()
        .filter(|l| l.date >= week)
        .map(|l| l.hours)
        .sum();
    out.push(m(
        Area::Resources,
        t("an_m_resources"),
        crate::ui::trim(hours),
        format!("{} {}", crate::ui::trim(mhours), t("an_machine_hours")),
        Severity::Ok,
    ));

    if !inp.units.is_empty() {
        let sold_pct = inp.sales.sold as f64 / inp.units.len() as f64 * 100.0;
        out.push(m(
            Area::Sales,
            t("an_m_sales"),
            format!("{sold_pct:.0}%"),
            format!("{} {}", t("an_received"), crate::ui::money(inp.sales.received)),
            if inp.sales.overdue > 0.0 {
                Severity::Warning
            } else {
                Severity::Ok
            },
        ));
    }

    out
}

/// Topilmalar va ko'rsatkichlardan matnli hisobot tuzadi.
///
/// Hisobot ekrandagi bilan bir xil manbadan chiqadi, shuning uchun faylda va
/// ekranda turli sonlar bo'lishi mumkin emas.
pub fn report(inp: &Input, project: &str) -> String {
    use std::fmt::Write;

    let f = findings(inp);
    let mut o = String::new();
    let _ = writeln!(o, "QURAi — {}", t("an_report_title"));
    let _ = writeln!(o, "{}: {project}", t("col_object"));
    let _ = writeln!(o, "{}: {}", t("col_date"), inp.today.format("%d.%m.%Y"));
    let _ = writeln!(o, "{}: {} / 100", t("an_health"), health(&f));
    let _ = writeln!(o);

    let _ = writeln!(o, "== {} ==", t("an_report_metrics"));
    for m in metrics(inp) {
        let _ = write!(o, "{:<28} {:>18}", m.title, m.value);
        if m.hint.is_empty() {
            let _ = writeln!(o);
        } else {
            let _ = writeln!(o, "   ({})", m.hint);
        }
    }

    let _ = writeln!(o);
    let _ = writeln!(o, "== {} ({}) ==", t("an_report_findings"), f.len());
    if f.is_empty() {
        let _ = writeln!(o, "{}", t("an_nothing"));
    }
    for x in &f {
        let _ = writeln!(o);
        let _ = writeln!(
            o,
            "[{}] {} · {}",
            x.code,
            x.severity.label(),
            x.area.label()
        );
        let _ = writeln!(o, "{}", x.fact);
        if !x.evidence.is_empty() {
            let _ = writeln!(o, "  {}: {}", t("an_evidence"), x.evidence);
        }
        if !x.action.is_empty() {
            let _ = writeln!(o, "  {}: {}", t("an_action"), x.action);
        }
    }
    let _ = writeln!(o);
    let _ = writeln!(o, "{}", t("an_report_note"));
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bo'sh obyektda tahlil ishlashi va yolg'on topilma bermasligi kerak.
    #[test]
    fn empty_project_has_no_findings() {
        let progress = Progress::default();
        let schedule = Schedule::default();
        let cost = CostSummary::default();
        let sales = SalesSummary::default();
        let inp = Input {
            today: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            tasks: &[],
            schedule: &schedule,
            progress: &progress,
            issues: &[],
            exec_docs: &[],
            requests: &[],
            purchases: &[],
            supply: &[],
            stock: &[],
            stock_moves: &[],
            materials: &[],
            quality: &[],
            safety: &[],
            machines: &[],
            machine_logs: &[],
            workers: &[],
            timesheet: &[],
            units: &[],
            deals: &[],
            payments: &[],
            cost: &cost,
            sales: &sales,
            contract_sum: 0.0,
            paid_total: 0.0,
        };
        let f = findings(&inp);
        assert!(f.is_empty(), "bo'sh obyektda topilma: {:?}", f);
        assert_eq!(health(&f), 100);
        // Ko'rsatkichlar baribir chiqadi — ekran bo'sh qolmasin.
        assert!(!metrics(&inp).is_empty());
    }

    #[test]
    fn health_drops_with_severity() {
        let f = |severity: Severity| Finding {
            area: Area::Schedule,
            code: "AN-T",
            severity,
            fact: String::new(),
            evidence: String::new(),
            action: String::new(),
            screen: Screen::Dashboard,
        };
        assert_eq!(health(&[f(Severity::Critical)]), 85);
        assert_eq!(health(&[f(Severity::Warning)]), 96);
        assert_eq!(health(&[f(Severity::Info)]), 100);
        // Ko'p topilmada ball nolga tushadi, manfiy bo'lmaydi.
        let many: Vec<Finding> = (0..20).map(|_| f(Severity::Critical)).collect();
        assert_eq!(health(&many), 0);
    }
}
