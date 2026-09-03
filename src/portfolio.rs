//! Bir necha obyekt bo'yicha konsolidatsiya (TZ XVII.4, X.33, XV.34, XVI.33).
//!
//! Qolgan modullar bitta obyekt kesimida ishlaydi — bu to'g'ri, chunki kunlik
//! ish bitta maydonchada boradi. Rahbarga esa boshqa savol kerak: **qaysi
//! obyekt orqada qolyapti va nega**. Shuning uchun bu modul har bir obyekt
//! uchun bir xil ko'rsatkichlar to'plamini bazadan yig'adi va yonma-yon
//! qo'yadi.
//!
//! Ko'rsatkichlar boshqa modullardagi funksiyalardan olinadi — bu yerda
//! qaytadan hisoblanmaydi. Shuning uchun portfeldagi son obyekt ekranidagi
//! son bilan hech qachon zid bo'lmaydi.

use crate::checks;
use crate::db::Db;
use crate::model::{ObjectStatus, Project};
use chrono::NaiveDate;

/// Bitta obyekt bo'yicha yakun.
#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub project_id: i64,
    pub name: String,
    pub code: String,
    pub status: ObjectStatus,
    /// Haqiqiy bajarilish, foizda.
    pub fact_pct: f64,
    /// Reja bo'yicha bugungi kunga bajarilishi kerak bo'lgan foiz.
    pub plan_pct: f64,
    /// Kechikish, kunlarda.
    pub delay_days: i64,
    /// Muddati o'tgan, lekin yopilmagan ishlar.
    pub overdue_tasks: usize,
    pub contract_sum: f64,
    pub paid_total: f64,
    /// Smeta yakuniy summasi (koeffitsiyentlar bilan).
    pub estimate_total: f64,
    /// Ochiq kritik nomuvofiqliklar.
    pub critical_issues: usize,
    /// Sifat balli. Tekshiruv bo'lmasa `None`.
    pub quality_score: Option<f64>,
    /// Xavfsizlik balli.
    pub safety_score: f64,
    /// Ombor qoldig'ining qiymati.
    pub stock_value: f64,
    /// Muddati o'tgan, yetkazilmagan xaridlar.
    pub late_purchases: usize,
    /// Sotilgan va jami birliklar.
    pub units_sold: usize,
    pub units_total: usize,
    /// Shartnomalar bo'yicha tushgan pul.
    pub sales_paid: f64,
}

impl ProjectSummary {
    /// Rejadan chetlanish: fakt minus reja, foiz punktida.
    pub fn deviation(&self) -> f64 {
        self.fact_pct - self.plan_pct
    }

    /// Moliyalashtirish ulushi: to'langan / shartnoma.
    pub fn paid_pct(&self) -> f64 {
        if self.contract_sum > 0.0 {
            self.paid_total / self.contract_sum * 100.0
        } else {
            0.0
        }
    }

    /// Sotuv ulushi.
    pub fn sold_pct(&self) -> f64 {
        if self.units_total > 0 {
            self.units_sold as f64 / self.units_total as f64 * 100.0
        } else {
            0.0
        }
    }

    /// E'tibor talab qiladigan obyekt.
    ///
    /// Uch belgidan biri yetarli: sezilarli kechikish, ochiq kritik
    /// nomuvofiqlik yoki past xavfsizlik balli. Bu «yomon» degani emas —
    /// «birinchi shu yerga qarash kerak» degani.
    pub fn needs_attention(&self) -> bool {
        self.deviation() < -5.0 || self.critical_issues > 0 || self.safety_score < 60.0
    }
}

/// Barcha obyektlar bo'yicha yakun, e'tibor talab qiladiganlari yuqorida.
///
/// Har bir obyekt uchun baza alohida o'qiladi. Obyektlar soni o'nlab bo'lgani
/// uchun bu yetarli tez; minglab obyekt paydo bo'lsa bu joy qayta ko'riladi.
pub fn summaries(db: &Db, today: NaiveDate) -> Vec<ProjectSummary> {
    let projects = db.projects().unwrap_or_default();
    let mut out: Vec<ProjectSummary> = projects.iter().map(|p| one(db, p, today)).collect();
    // Diqqat talab qiladiganlar oldinda, keyin kechikish bo'yicha.
    out.sort_by(|a, b| {
        b.needs_attention()
            .cmp(&a.needs_attention())
            .then(a.deviation().total_cmp(&b.deviation()))
    });
    out
}

/// Bitta obyekt bo'yicha ko'rsatkichlarni yig'adi.
fn one(db: &Db, p: &Project, today: NaiveDate) -> ProjectSummary {
    let pid = p.id;
    let tasks = db.tasks(pid).unwrap_or_default();
    let links = db.links(pid).unwrap_or_default();
    let schedule = crate::cpm::compute(&tasks, &links, p.start_date);
    let progress = crate::cpm::progress(&tasks, &schedule, p.start_date, today);

    let materials = db.materials(pid);
    let moves = db.stock_moves(pid);
    let stock = checks::stock_balances(&materials, &moves, &db.reservations(pid), today);

    let quality = db.quality_checks(pid);
    let score = checks::quality_score(&quality, today);

    let workers = db.workers(pid);
    let safety = checks::worker_safety(
        &workers,
        &db.worker_permits(pid),
        &db.ppe_issues(pid),
        today,
    );
    let safety_score = checks::safety_score(
        &db.safety_events(pid),
        &safety,
        &db.work_permits(pid),
        today,
    );

    // Smeta: birinchi smeta bo'yicha, koeffitsiyentlar bilan.
    let estimate_total = db
        .estimates(pid)
        .first()
        .map(|e| {
            let direct: f64 = db.estimate_items(e.id).iter().map(|i| i.computed()).sum();
            e.totals(direct).total
        })
        .unwrap_or(0.0);

    let purchases = db.purchases(pid);
    let units = db.units(pid);
    let deals = db.deals(pid);
    let payments = db.payments(pid);

    ProjectSummary {
        project_id: pid,
        name: p.name.clone(),
        code: p.code.clone(),
        status: p.status,
        fact_pct: progress.fact_pct,
        plan_pct: progress.plan_pct,
        delay_days: progress.delay_days,
        overdue_tasks: progress.overdue.len(),
        contract_sum: p.contract_sum,
        paid_total: p.paid_total,
        estimate_total,
        critical_issues: db
            .issues(pid)
            .iter()
            .filter(|i| i.severity == crate::domain::Severity::Critical)
            .filter(|i| {
                !matches!(
                    i.status,
                    crate::domain::IssueStatus::Fixed | crate::domain::IssueStatus::Rejected
                )
            })
            .count(),
        quality_score: (score.checks > 0).then_some(score.score),
        safety_score: safety_score.score,
        stock_value: stock.iter().map(|l| l.value).sum(),
        late_purchases: purchases
            .iter()
            .filter(|x| !x.fully_delivered() && x.delivery_date < today)
            .count(),
        units_sold: units
            .iter()
            .filter(|u| {
                crate::sales::status_for(deals.iter().find(|d| d.unit_id == u.id))
                    .unwrap_or(u.status)
                    == crate::domain::UnitStatus::Sold
            })
            .count(),
        units_total: units.len(),
        sales_paid: payments.iter().map(|x| x.paid).sum(),
    }
}

/// Obyektlar orasida material ko'chirish takliflari (TZ XI.42).
///
/// Bir obyektda ortiqcha turgan material boshqasida yetishmayotgan
/// bo'lishi mumkin. Ko'chirish sotib olishdan arzon — lekin qaror
/// odamniki, shuning uchun bu faqat taklif.
pub fn redistribution(db: &Db, today: NaiveDate) -> Vec<checks::Redistribution> {
    let projects = db.projects().unwrap_or_default();
    let mut surplus = Vec::new();
    let mut need = Vec::new();

    for p in &projects {
        let materials = db.materials(p.id);
        if materials.is_empty() {
            continue;
        }
        let moves = db.stock_moves(p.id);
        let stock = checks::stock_balances(&materials, &moves, &db.reservations(p.id), today);
        let tasks = db.tasks(p.id).unwrap_or_default();
        let plan = checks::purchase_plan(
            &materials,
            &stock,
            &db.purchases(p.id),
            &db.requests(p.id),
            &db.material_norms(p.id),
            &tasks,
            today,
            PLAN_HORIZON,
        );
        surplus.push((p.id, stock, materials.clone()));
        need.push((p.id, plan, materials));
    }
    checks::redistribution(&surplus, &need)
}

/// Obyektlar bo'yicha xaridlarni bir jadvalda solishtiradi (TZ X.33, 37-38).
pub fn central_purchases(db: &Db) -> Vec<checks::CentralLine> {
    let rows: Vec<(i64, Vec<crate::domain::Purchase>)> = db
        .projects()
        .unwrap_or_default()
        .iter()
        .map(|p| (p.id, db.purchases(p.id)))
        .collect();
    checks::central_purchases(&rows)
}

/// Obyektlar kesimida xodim va soat (TZ XIII.3).
pub fn object_staff(db: &Db, from: NaiveDate, to: NaiveDate) -> Vec<checks::ObjectStaff> {
    let rows: Vec<(
        i64,
        Vec<crate::domain::Worker>,
        Vec<crate::domain::TimesheetEntry>,
    )> = db
        .projects()
        .unwrap_or_default()
        .iter()
        .map(|p| (p.id, db.workers(p.id), db.timesheet(p.id)))
        .collect();
    checks::object_staff(&rows, from, to)
}

/// Obyektlarni bir xil o'lchovda solishtiradi (TZ XVII.27-28).
pub fn benchmark(db: &Db, today: NaiveDate) -> Vec<checks::Benchmark> {
    let rows: Vec<checks::BenchmarkInput> = db
        .projects()
        .unwrap_or_default()
        .iter()
        .map(|p| {
            let tasks = db.tasks(p.id).unwrap_or_default();
            let links = db.links(p.id).unwrap_or_default();
            let schedule = crate::cpm::compute(&tasks, &links, p.start_date);
            let progress = crate::cpm::progress(&tasks, &schedule, p.start_date, today);
            let quality = db.quality_checks(p.id);
            let safety = db.safety_events(p.id);
            checks::BenchmarkInput {
                project_id: p.id,
                fact_pct: progress.fact_pct,
                plan_pct: progress.plan_pct,
                volume: tasks.iter().map(|t| t.volume).sum(),
                cost: db
                    .purchases(p.id)
                    .iter()
                    .filter(|x| x.status != crate::domain::PurchaseStatus::Draft)
                    .map(|x| x.amount())
                    .sum(),
                hours: db.timesheet(p.id).iter().map(|e| e.hours).sum(),
                quality: checks::quality_score(&quality, today).score,
                safety: checks::safety_score(
                    &safety,
                    &checks::worker_safety(
                        &db.workers(p.id),
                        &db.worker_permits(p.id),
                        &db.ppe_issues(p.id),
                        today,
                    ),
                    &db.work_permits(p.id),
                    today,
                )
                .score,
            }
        })
        .collect();
    checks::benchmark(&rows)
}

/// Qayta taqsimlash rejasi shuncha kun oldinga qaraydi.
const PLAN_HORIZON: i64 = 45;

/// Portfel bo'yicha umumiy yakun.
#[derive(Debug, Clone, Default)]
pub struct PortfolioTotals {
    pub projects: usize,
    pub attention: usize,
    pub contract_sum: f64,
    pub paid_total: f64,
    pub estimate_total: f64,
    pub stock_value: f64,
    /// Ish hajmi bo'yicha vaznlangan o'rtacha bajarilish.
    ///
    /// Oddiy o'rtacha noto'g'ri javob berardi: kichik obyektning 100% i katta
    /// obyektning 20% i bilan bir xil vazn olardi.
    pub weighted_pct: f64,
    pub critical_issues: usize,
}

pub fn totals(list: &[ProjectSummary]) -> PortfolioTotals {
    let mut t = PortfolioTotals {
        projects: list.len(),
        ..Default::default()
    };
    let mut weight = 0.0;
    for p in list {
        if p.needs_attention() {
            t.attention += 1;
        }
        t.contract_sum += p.contract_sum;
        t.paid_total += p.paid_total;
        t.estimate_total += p.estimate_total;
        t.stock_value += p.stock_value;
        t.critical_issues += p.critical_issues;
        // Vazn — shartnoma summasi; u yo'q bo'lsa obyekt vaznsiz qoladi.
        weight += p.contract_sum;
        t.weighted_pct += p.fact_pct * p.contract_sum;
    }
    t.weighted_pct = if weight > 0.0 {
        t.weighted_pct / weight
    } else {
        0.0
    };
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(id: i64, fact: f64, plan: f64, contract: f64) -> ProjectSummary {
        ProjectSummary {
            project_id: id,
            name: format!("Obyekt {id}"),
            code: String::new(),
            status: ObjectStatus::InProgress,
            fact_pct: fact,
            plan_pct: plan,
            delay_days: 0,
            overdue_tasks: 0,
            contract_sum: contract,
            paid_total: 0.0,
            estimate_total: 0.0,
            critical_issues: 0,
            quality_score: None,
            safety_score: 100.0,
            stock_value: 0.0,
            late_purchases: 0,
            units_sold: 0,
            units_total: 0,
            sales_paid: 0.0,
        }
    }

    /// Vaznlangan o'rtacha kichik obyektni katta obyekt bilan tenglashtirmaydi.
    #[test]
    fn weighted_average_respects_contract_size() {
        // Kichik obyekt 100%, katta obyekt 20%.
        let list = vec![
            summary(1, 100.0, 100.0, 1_000.0),
            summary(2, 20.0, 20.0, 9_000.0),
        ];
        let t = totals(&list);
        assert_eq!(t.projects, 2);
        // Oddiy o'rtacha 60% bo'lardi; vaznlangan — 28%.
        assert_eq!(t.weighted_pct, 28.0);
        assert_eq!(t.contract_sum, 10_000.0);
    }

    /// Shartnoma summasi yo'q bo'lsa vaznlangan o'rtacha nol qoladi —
    /// o'ylab topilgan son chiqmaydi.
    #[test]
    fn no_contract_sum_gives_no_weighted_average() {
        let list = vec![summary(1, 50.0, 50.0, 0.0)];
        assert_eq!(totals(&list).weighted_pct, 0.0);
    }

    /// E'tibor uch sababdan biri bo'yicha belgilanadi.
    #[test]
    fn attention_has_three_reasons() {
        // Hammasi joyida.
        let ok = summary(1, 50.0, 50.0, 100.0);
        assert!(!ok.needs_attention());

        // 1. Rejadan sezilarli orqada.
        let late = summary(2, 40.0, 50.0, 100.0);
        assert!(late.needs_attention());
        assert_eq!(late.deviation(), -10.0);

        // Kichik chetlanish e'tibor talab qilmaydi.
        let small = summary(3, 48.0, 50.0, 100.0);
        assert!(!small.needs_attention());

        // 2. Ochiq kritik nomuvofiqlik.
        let mut critical = ok.clone();
        critical.critical_issues = 1;
        assert!(critical.needs_attention());

        // 3. Past xavfsizlik balli.
        let mut unsafe_one = ok.clone();
        unsafe_one.safety_score = 55.0;
        assert!(unsafe_one.needs_attention());
    }

    /// Ro'yxatda e'tibor talab qiladiganlar oldinda turadi.
    #[test]
    fn attention_projects_come_first() {
        // Vaqtinchalik baza — sinovlar bir-biriga xalaqit bermasin.
        let path = std::env::temp_dir().join(format!("qurai_portfolio_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).expect("baza");
        let pid = db.seed_demo().unwrap();
        // Ikkinchi obyekt — bo'sh, muammosiz.
        let calm = db
            .insert_project(&Project {
                id: 0,
                name: "Tinch obyekt".into(),
                code: "T-1".into(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: ObjectStatus::InProgress,
                start_date: chrono::Local::now().date_naive(),
                planned_end: chrono::Local::now().date_naive(),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: "UZS".into(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();

        // Namuna obyekti biroz orqada, lekin chegaradan oshmagan — shuning
        // uchun ataylab kritik nomuvofiqlik qo'shamiz.
        db.insert_issue(&crate::domain::Issue {
            id: 0,
            project_id: pid,
            module: crate::domain::IssueModule::Project,
            section: crate::model::Section::Kj,
            code: "PF-1".into(),
            sheet: String::new(),
            location: String::new(),
            element: String::new(),
            title: "Sinov".into(),
            description: String::new(),
            severity: crate::domain::Severity::Critical,
            norm_doc: String::new(),
            norm_clause: String::new(),
            norm_text: String::new(),
            recommendation: String::new(),
            responsible: String::new(),
            deadline: None,
            status: crate::domain::IssueStatus::Open,
            auto: false,
            created_at: String::new(),
        });

        let list = summaries(&db, chrono::Local::now().date_naive());
        // Namunaning ikki obyekti va shu yerda qo'shilgan tinch obyekt.
        assert_eq!(list.len(), 3);
        assert!(list[0].needs_attention(), "muammoli obyekt oldinda emas");

        // Tinch obyektda ish ham, chetlanish ham yo'q — u eng oxirida turadi.
        let last = list.last().expect("ro'yxat bo'sh");
        assert_eq!(last.project_id, calm);
        assert!(!last.needs_attention());

        let demo = list
            .iter()
            .find(|p| p.project_id == pid)
            .expect("namuna obyekti");
        assert!(demo.needs_attention());
        // Namunada ochiq kritik nomuvofiqliklar bor; shu yerda qo'shilgani
        // ham ular qatoriga tushadi.
        assert!(demo.critical_issues >= 1);
        let _ = std::fs::remove_file(&path);

        // Ko'rsatkichlar obyekt ekranidagi bilan bir xil manbadan.
        assert!(demo.stock_value > 0.0);
        assert!(demo.units_total > 0);
        assert!(demo.safety_score < 100.0);
    }
}
