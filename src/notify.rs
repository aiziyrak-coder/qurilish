//! Bildirishnomalar markazi (umumiy talab, TZ VI.29, XVI.45).
//!
//! Har bir modul o'z ekranida ogohlantiradi, lekin kun boshida odam
//! o'n beshta ekranni ochib chiqmaydi. Bu modul barcha modullardan
//! **e'tibor talab qiladigan** yozuvlarni yig'adi va bitta ro'yxatga qo'yadi.
//!
//! Uch qoida:
//!
//! 1. **Yangi hisob yo'q.** Har bir bildirishnoma allaqachon mavjud
//!    funksiyalardan chiqadi — shuning uchun bu yerdagi son o'sha modul
//!    ekranidagi son bilan hech qachon zid bo'lmaydi.
//! 2. **Har biri manzilga ega.** Bildirishnoma qaysi ekranga olib borishini
//!    biladi; «nimadir yomon» degan xabar foydasiz.
//! 3. **Muddati o'tgani birinchi.** Tartib: og'irlik, keyin kechikish kuni.

use crate::app::{App, Screen};
use crate::checks;
use crate::domain::{
    AcceptState, ExecDocStatus, InspectionResult, IssueStatus, RequestStatus, Severity,
};
use crate::i18n::t;
use chrono::NaiveDate;

/// Bildirishnoma manbai — qaysi modul aytyapti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Schedule,
    Inspection,
    Quality,
    Documents,
    Supply,
    Stock,
    Safety,
    Machines,
    Money,
    Client,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Schedule => t("nt_src_schedule"),
            Source::Inspection => t("nt_src_inspection"),
            Source::Quality => t("nt_src_quality"),
            Source::Documents => t("nt_src_documents"),
            Source::Supply => t("nt_src_supply"),
            Source::Stock => t("nt_src_stock"),
            Source::Safety => t("nt_src_safety"),
            Source::Machines => t("nt_src_machines"),
            Source::Money => t("nt_src_money"),
            Source::Client => t("nt_src_client"),
        }
    }
}

/// Bitta bildirishnoma.
#[derive(Debug, Clone)]
pub struct Notice {
    /// Barqaror kod — sinovlar va o'qish uchun.
    pub code: &'static str,
    pub source: Source,
    pub severity: Severity,
    /// Nechta yozuv shu bildirishnomaga kirdi.
    pub count: usize,
    pub title: String,
    /// Nima uchun shunday deyilyapti — dalil.
    pub detail: String,
    /// Bosilganda ochiladigan ekran.
    pub screen: Screen,
    /// Eng katta kechikish, kunlarda. Kechikish bo'lmasa nol.
    pub days: i64,
}

impl Notice {
    // Sakkizta maydon — har biri bildirishnomaning ajralmas qismi;
    // ularni tuzilmaga guruhlash faqat qatlam qo'shadi.
    #[allow(clippy::too_many_arguments)]
    fn new(
        code: &'static str,
        source: Source,
        severity: Severity,
        count: usize,
        title: String,
        detail: String,
        screen: Screen,
        days: i64,
    ) -> Notice {
        Notice {
            code,
            source,
            severity,
            count,
            title,
            detail,
            screen,
            days,
        }
    }
}

/// Og'irlik tartibi: kritik birinchi.
fn rank(s: Severity) -> u8 {
    match s {
        Severity::Critical => 0,
        Severity::Major => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
        Severity::Ok => 4,
    }
}

/// Barcha modullardan bildirishnomalarni yig'adi.
///
/// Obyekt tanlanmagan bo'lsa ro'yxat bo'sh: bildirishnomalar doim aniq
/// obyektga tegishli.
pub fn collect(app: &App) -> Vec<Notice> {
    if app.current.is_none() {
        return Vec::new();
    }
    let today = app.today;
    let mut out: Vec<Notice> = Vec::new();

    schedule_notices(app, today, &mut out);
    inspection_notices(app, today, &mut out);
    quality_notices(app, today, &mut out);
    document_notices(app, &mut out);
    supply_notices(app, today, &mut out);
    stock_notices(app, &mut out);
    safety_notices(app, &mut out);
    machine_notices(app, &mut out);
    money_notices(app, today, &mut out);

    // Muhimi va eng ko'p kechikkani tepada.
    out.sort_by(|a, b| {
        rank(a.severity)
            .cmp(&rank(b.severity))
            .then(b.days.cmp(&a.days))
            .then(b.count.cmp(&a.count))
    });
    out
}

/// Ro'yxatdagi eng og'ir daraja — belgi rangi shundan olinadi.
pub fn top_severity(list: &[Notice]) -> Option<Severity> {
    list.iter().map(|n| n.severity).min_by_key(|s| rank(*s))
}

// ---------------------------------------------------------------- I. GPR

fn schedule_notices(app: &App, today: NaiveDate, out: &mut Vec<Notice>) {
    let overdue: Vec<&crate::model::Task> = app
        .tasks
        .iter()
        .filter(|x| {
            x.fact_end.is_none() && x.plan_start + chrono::Duration::days(x.duration) < today
        })
        .collect();
    if !overdue.is_empty() {
        let worst = overdue
            .iter()
            .map(|x| (today - (x.plan_start + chrono::Duration::days(x.duration))).num_days())
            .max()
            .unwrap_or(0);
        out.push(Notice::new(
            "NT-S1",
            Source::Schedule,
            if worst > 14 {
                Severity::Critical
            } else {
                Severity::Major
            },
            overdue.len(),
            format!("{} {}", overdue.len(), t("nt_tasks_overdue")),
            format!(
                "{}: {} · {} {}",
                t("nt_worst"),
                overdue
                    .iter()
                    .max_by_key(|x| {
                        (today - (x.plan_start + chrono::Duration::days(x.duration))).num_days()
                    })
                    .map(|x| x.name.clone())
                    .unwrap_or_default(),
                worst,
                t("nt_days")
            ),
            Screen::Gantt,
            worst,
        ));
    }

    // Boshlanishi kerak, lekin boshlanmagan ishlar.
    let not_started: Vec<&crate::model::Task> = app
        .tasks
        .iter()
        .filter(|x| x.fact_start.is_none() && x.progress == 0.0 && x.plan_start < today)
        .collect();
    if !not_started.is_empty() {
        let worst = not_started
            .iter()
            .map(|x| (today - x.plan_start).num_days())
            .max()
            .unwrap_or(0);
        out.push(Notice::new(
            "NT-S2",
            Source::Schedule,
            Severity::Warning,
            not_started.len(),
            format!("{} {}", not_started.len(), t("nt_tasks_not_started")),
            format!("{} {} {}", t("nt_worst"), worst, t("nt_days")),
            Screen::Gantt,
            worst,
        ));
    }
}

// ---------------------------------------------------------------- VII

fn inspection_notices(app: &App, today: NaiveDate, out: &mut Vec<Notice>) {
    let s = checks::inspection_summary(&app.inspections, today);
    if s.overdue > 0 {
        let worst = app
            .inspections
            .iter()
            .filter(|x| x.overdue(today))
            .map(|x| (today - x.planned).num_days())
            .max()
            .unwrap_or(0);
        out.push(Notice::new(
            "NT-I1",
            Source::Inspection,
            Severity::Major,
            s.overdue,
            format!("{} {}", s.overdue, t("nt_inspections_overdue")),
            format!("{} {} {}", t("nt_worst"), worst, t("nt_days")),
            Screen::Inspections,
            worst,
        ));
    }
    if s.today > 0 {
        out.push(Notice::new(
            "NT-I2",
            Source::Inspection,
            Severity::Info,
            s.today,
            format!("{} {}", s.today, t("nt_inspections_today")),
            t("nt_inspections_today_hint").to_string(),
            Screen::Inspections,
            0,
        ));
    }
    if s.open_defects > 0 {
        // Bartaraf etish muddati o'tganlari alohida og'irlikka ega.
        let late = app
            .inspections
            .iter()
            .filter(|x| x.open_defect() && x.deadline.is_some_and(|d| d < today))
            .count();
        out.push(Notice::new(
            "NT-I3",
            Source::Inspection,
            if late > 0 {
                Severity::Critical
            } else {
                Severity::Major
            },
            s.open_defects,
            format!("{} {}", s.open_defects, t("nt_inspection_defects")),
            format!("{} {}", late, t("nt_past_deadline")),
            Screen::Inspections,
            0,
        ));
    }

    // Beton: sinov sanasi kelgan, lekin natija kiritilmagan.
    let c = checks::concrete_summary(&app.concrete_tests, today);
    if c.due > 0 {
        out.push(Notice::new(
            "NT-I4",
            Source::Inspection,
            Severity::Warning,
            c.due,
            format!("{} {}", c.due, t("nt_concrete_due")),
            t("nt_concrete_due_hint").to_string(),
            Screen::Inspections,
            0,
        ));
    }
    if c.failed > 0 {
        out.push(Notice::new(
            "NT-I5",
            Source::Inspection,
            Severity::Critical,
            c.failed,
            format!("{} {}", c.failed, t("nt_concrete_failed")),
            format!("{}: {:.0} %", t("nt_worst_result"), c.worst_pct),
            Screen::Inspections,
            0,
        ));
    }

    let geo = checks::geodesy_issues(&app.geodesy_points).len();
    if geo > 0 {
        out.push(Notice::new(
            "NT-I6",
            Source::Inspection,
            Severity::Major,
            geo,
            format!("{} {}", geo, t("nt_geodesy_out")),
            t("nt_geodesy_hint").to_string(),
            Screen::Inspections,
            0,
        ));
    }
}

// ---------------------------------------------------------------- II, XIV

fn quality_notices(app: &App, today: NaiveDate, out: &mut Vec<Notice>) {
    let critical = app
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Critical)
        .filter(|i| !matches!(i.status, IssueStatus::Fixed | IssueStatus::Rejected))
        .count();
    if critical > 0 {
        out.push(Notice::new(
            "NT-Q1",
            Source::Quality,
            Severity::Critical,
            critical,
            format!("{} {}", critical, t("nt_critical_issues")),
            t("nt_critical_hint").to_string(),
            Screen::AiCheck,
            0,
        ));
    }

    let overdue_defects = app
        .quality
        .iter()
        .filter(|q| q.open_defect() && q.deadline.is_some_and(|d| d < today))
        .count();
    if overdue_defects > 0 {
        let worst = app
            .quality
            .iter()
            .filter(|q| q.open_defect())
            .filter_map(|q| q.deadline.map(|d| (today - d).num_days()))
            .max()
            .unwrap_or(0);
        out.push(Notice::new(
            "NT-Q2",
            Source::Quality,
            Severity::Major,
            overdue_defects,
            format!("{} {}", overdue_defects, t("nt_defects_overdue")),
            format!("{} {} {}", t("nt_worst"), worst, t("nt_days")),
            Screen::Quality,
            worst,
        ));
    }
}

// ---------------------------------------------------------------- IV

fn document_notices(app: &App, out: &mut Vec<Notice>) {
    let waiting = app
        .exec_docs
        .iter()
        .filter(|d| d.status == ExecDocStatus::OnReview)
        .count();
    if waiting > 0 {
        out.push(Notice::new(
            "NT-D1",
            Source::Documents,
            Severity::Warning,
            waiting,
            format!("{} {}", waiting, t("nt_docs_waiting")),
            t("nt_docs_waiting_hint").to_string(),
            Screen::ExecDocs,
            0,
        ));
    }
    let rejected = app
        .exec_docs
        .iter()
        .filter(|d| d.status == ExecDocStatus::Rejected)
        .count();
    if rejected > 0 {
        out.push(Notice::new(
            "NT-D2",
            Source::Documents,
            Severity::Major,
            rejected,
            format!("{} {}", rejected, t("nt_docs_rejected")),
            t("nt_docs_rejected_hint").to_string(),
            Screen::ExecDocs,
            0,
        ));
    }
}

// ---------------------------------------------------------------- IX, X

fn supply_notices(app: &App, today: NaiveDate, out: &mut Vec<Notice>) {
    let pending = app
        .requests
        .iter()
        .filter(|r| r.status == RequestStatus::New)
        .count();
    if pending > 0 {
        out.push(Notice::new(
            "NT-R1",
            Source::Supply,
            Severity::Warning,
            pending,
            format!("{} {}", pending, t("nt_requests_new")),
            t("nt_requests_new_hint").to_string(),
            Screen::Requests,
            0,
        ));
    }

    let late: Vec<&crate::domain::Purchase> = app
        .purchases
        .iter()
        .filter(|p| !p.fully_delivered() && p.delivery_date < today)
        .collect();
    if !late.is_empty() {
        let worst = late
            .iter()
            .map(|p| (today - p.delivery_date).num_days())
            .max()
            .unwrap_or(0);
        out.push(Notice::new(
            "NT-P1",
            Source::Supply,
            if worst > 7 {
                Severity::Major
            } else {
                Severity::Warning
            },
            late.len(),
            format!("{} {}", late.len(), t("nt_purchases_late")),
            format!("{} {} {}", t("nt_worst"), worst, t("nt_days")),
            Screen::Purchases,
            worst,
        ));
    }
}

// ---------------------------------------------------------------- XI

fn stock_notices(app: &App, out: &mut Vec<Notice>) {
    let stock = app.stock();
    let low: Vec<(&crate::domain::Material, f64)> = app
        .materials
        .iter()
        .filter(|m| m.min_stock > 0.0)
        .filter_map(|m| {
            let line = stock.iter().find(|l| l.material_id == m.id)?;
            (line.available < m.min_stock).then_some((m, line.available))
        })
        .collect();
    if !low.is_empty() {
        out.push(Notice::new(
            "NT-W1",
            Source::Stock,
            Severity::Warning,
            low.len(),
            format!("{} {}", low.len(), t("nt_stock_low")),
            low.iter()
                .take(3)
                .map(|(m, avail)| format!("{}: {:.0} {}", m.name, avail, m.unit))
                .collect::<Vec<_>>()
                .join(" · "),
            Screen::Warehouse,
            0,
        ));
    }
}

// ---------------------------------------------------------------- XV

fn safety_notices(app: &App, out: &mut Vec<Notice>) {
    let safety = app.worker_safety();
    let expired = safety.iter().filter(|s| !s.expired.is_empty()).count();
    if expired > 0 {
        out.push(Notice::new(
            "NT-X1",
            Source::Safety,
            Severity::Critical,
            expired,
            format!("{} {}", expired, t("nt_permits_expired")),
            t("nt_permits_expired_hint").to_string(),
            Screen::Safety,
            0,
        ));
    }
    let expiring = safety
        .iter()
        .filter(|s| s.expired.is_empty() && !s.expiring.is_empty())
        .count();
    if expiring > 0 {
        out.push(Notice::new(
            "NT-X2",
            Source::Safety,
            Severity::Warning,
            expiring,
            format!("{} {}", expiring, t("nt_permits_expiring")),
            t("nt_permits_expiring_hint").to_string(),
            Screen::Safety,
            0,
        ));
    }
    let ppe = safety.iter().filter(|s| !s.ppe_missing.is_empty()).count();
    if ppe > 0 {
        out.push(Notice::new(
            "NT-X3",
            Source::Safety,
            Severity::Major,
            ppe,
            format!("{} {}", ppe, t("nt_ppe_missing")),
            t("nt_ppe_missing_hint").to_string(),
            Screen::Safety,
            0,
        ));
    }
}

// ---------------------------------------------------------------- XVI

fn machine_notices(app: &App, out: &mut Vec<Notice>) {
    // Texnik xizmat muddati: soat normasi to'lgan mashinalar.
    let due: Vec<&crate::domain::Machine> = app
        .machines
        .iter()
        .filter(|m| m.service_hours > 0.0 && m.service_done >= m.service_hours)
        .collect();
    if !due.is_empty() {
        out.push(Notice::new(
            "NT-M1",
            Source::Machines,
            Severity::Warning,
            due.len(),
            format!("{} {}", due.len(), t("nt_service_due")),
            due.iter()
                .take(3)
                .map(|m| m.name.clone())
                .collect::<Vec<_>>()
                .join(" · "),
            Screen::Machines,
            0,
        ));
    }
}

// ---------------------------------------------------------------- VIII, XX

fn money_notices(app: &App, today: NaiveDate, out: &mut Vec<Notice>) {
    let d = checks::payment_discipline(&app.payment_stages, today);
    if d.debt > 0.0 {
        out.push(Notice::new(
            "NT-C1",
            Source::Money,
            if d.max_delay > 30 {
                Severity::Critical
            } else {
                Severity::Major
            },
            d.overdue,
            format!("{} {}", t("nt_payment_debt"), crate::ui::money(d.debt)),
            format!("{} {} {}", t("nt_worst"), d.max_delay, t("nt_days")),
            Screen::Contracts,
            d.max_delay,
        ));
    }
    if d.due_soon > 0.0 {
        out.push(Notice::new(
            "NT-C2",
            Source::Money,
            Severity::Info,
            0,
            format!("{} {}", t("nt_payment_soon"), crate::ui::money(d.due_soon)),
            t("nt_payment_soon_hint").to_string(),
            Screen::Contracts,
            0,
        ));
    }

    let changes = app.contract_changes.iter().filter(|x| x.pending()).count();
    if changes > 0 {
        out.push(Notice::new(
            "NT-C3",
            Source::Client,
            Severity::Warning,
            changes,
            format!("{} {}", changes, t("nt_changes_pending")),
            t("nt_changes_pending_hint").to_string(),
            Screen::Contracts,
            0,
        ));
    }
    let accepts = app
        .acceptances
        .iter()
        .filter(|x| x.state == AcceptState::Submitted)
        .count();
    if accepts > 0 {
        out.push(Notice::new(
            "NT-C4",
            Source::Client,
            Severity::Warning,
            accepts,
            format!("{} {}", accepts, t("nt_accept_pending")),
            t("nt_accept_pending_hint").to_string(),
            Screen::Contracts,
            0,
        ));
    }
    let rejected = app
        .acceptances
        .iter()
        .filter(|x| x.state == AcceptState::Rejected)
        .count();
    if rejected > 0 {
        out.push(Notice::new(
            "NT-C5",
            Source::Client,
            Severity::Major,
            rejected,
            format!("{} {}", rejected, t("nt_accept_rejected")),
            t("nt_accept_rejected_hint").to_string(),
            Screen::Contracts,
            0,
        ));
    }
}

/// Tekshiruv natijasi salbiy bo'lgan, lekin muddat qo'yilmagan yozuvlar.
///
/// Alohida funksiya: Copilot ham shu ro'yxatdan foydalanadi.
pub fn defects_without_deadline(app: &App) -> Vec<i64> {
    app.inspections
        .iter()
        .filter(|x| {
            x.result != InspectionResult::Pass && x.fixed_at.is_none() && x.deadline.is_none()
        })
        .filter(|x| x.done.is_some())
        .map(|x| x.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    /// Har bir sinov uchun alohida fayl: sinovlar parallel ishlaydi, umumiy
    /// nom bo'lsa biri ikkinchisining bazasini o'chirib yuboradi.
    fn demo_app(tag: &str) -> (std::path::PathBuf, App) {
        let path =
            std::env::temp_dir().join(format!("qurai_notify_{}_{tag}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).expect("baza");
        let pid = db.seed_demo().expect("namuna");
        let mut app = App::new(Db::open(&path).expect("baza"));
        app.select_project(pid);
        (path, app)
    }

    /// Namunaviy obyektda bildirishnomalar bo'sh bo'lmasligi kerak.
    #[test]
    fn demo_project_has_notices() {
        let (path, app) = demo_app("has");
        let list = collect(&app);
        assert!(!list.is_empty(), "bildirishnoma yo'q");

        // Har birida manzil, sarlavha va dalil bor.
        for n in &list {
            assert!(!n.title.is_empty(), "{}: sarlavha bo'sh", n.code);
            assert!(!n.detail.is_empty(), "{}: dalil bo'sh", n.code);
            assert!(!n.code.is_empty());
        }
        // Kodlar takrorlanmaydi.
        let mut codes: Vec<&str> = list.iter().map(|n| n.code).collect();
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), before, "kod takrorlandi");
        let _ = std::fs::remove_file(&path);
    }

    /// Kritik bildirishnoma ro'yxat boshida turadi.
    #[test]
    fn critical_notices_come_first() {
        let (path, app) = demo_app("order");
        let list = collect(&app);
        for w in list.windows(2) {
            assert!(
                rank(w[0].severity) <= rank(w[1].severity),
                "tartib buzilgan: {} keyin {}",
                w[0].code,
                w[1].code
            );
        }
        assert!(top_severity(&list).is_some());
        let _ = std::fs::remove_file(&path);
    }

    /// Sonlar modul ekranlaridagi bilan bir xil manbadan.
    #[test]
    fn notice_counts_match_the_modules() {
        let (path, app) = demo_app("counts");
        let list = collect(&app);
        let by = |code: &str| list.iter().find(|n| n.code == code);

        if let Some(n) = by("NT-I1") {
            let s = checks::inspection_summary(&app.inspections, app.today);
            assert_eq!(n.count, s.overdue);
        }
        if let Some(n) = by("NT-I5") {
            let c = checks::concrete_summary(&app.concrete_tests, app.today);
            assert_eq!(n.count, c.failed);
        }
        if let Some(n) = by("NT-C4") {
            let waiting = app
                .acceptances
                .iter()
                .filter(|x| x.state == AcceptState::Submitted)
                .count();
            assert_eq!(n.count, waiting);
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Obyekt tanlanmagan bo'lsa ro'yxat bo'sh.
    #[test]
    fn no_project_means_no_notices() {
        let path = std::env::temp_dir().join(format!("qurai_notify_e{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let app = App::new(Db::open(&path).expect("baza"));
        assert!(collect(&app).is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
