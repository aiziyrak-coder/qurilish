//! Savol-javob yordamchisi (TZ XVIII).
//!
//! **Bu yerda til modeli yo'q.** Yordamchi faqat shu bazadagi ma'lumotdan
//! javob beradi: savol kalit so'zlar bo'yicha ma'lum bir hisobga bog'lanadi va
//! javob o'sha hisobning natijasi bo'ladi. Shuning uchun javob har doim
//! tekshirilishi mumkin — orqasida aniq son turadi va u qaysi ekrandan
//! kelganini ko'rsatib beriladi.
//!
//! Til modeli qo'shilganda shu funksiyalar unga «asboblar» bo'lib beriladi:
//! model savolni tushunadi, sonlarni esa baribir shu yerdan oladi.

use crate::analytics::{self, Input};
use crate::app::Screen;
use crate::domain::{IssueStatus, PurchaseStatus, QualityResult, RequestStatus, Severity};
use crate::i18n::t;
use crate::ui::{money, trim};

/// Yordamchi javob bera oladigan savol turi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Overview,
    Delays,
    Critical,
    Money,
    Supply,
    Stock,
    Crew,
    Machines,
    Quality,
    Safety,
    Sales,
    Docs,
    Cash,
    Attention,
    /// Loyiha hujjatlari va ulardagi ziddiyatlar (TZ II, III).
    Project,
}

impl Intent {
    pub const ALL: &'static [Intent] = &[
        Intent::Overview,
        Intent::Attention,
        Intent::Delays,
        Intent::Critical,
        // Pul oqimi umumiy «pul» savolidan oldin tekshiriladi: aks holda
        // «pul oqimi» so'rovi umumiy moliya javobiga tushib ketardi.
        Intent::Cash,
        Intent::Money,
        Intent::Docs,
        Intent::Supply,
        Intent::Stock,
        Intent::Crew,
        Intent::Machines,
        Intent::Quality,
        Intent::Safety,
        Intent::Sales,
        Intent::Project,
    ];

    /// Savolning tayyor ko'rinishi — tugma sifatida chiqadi.
    pub fn question(self) -> &'static str {
        match self {
            Intent::Overview => t("cp_q_overview"),
            Intent::Attention => t("cp_q_attention"),
            Intent::Delays => t("cp_q_delays"),
            Intent::Critical => t("cp_q_critical"),
            Intent::Money => t("cp_q_money"),
            Intent::Cash => t("cp_q_cash"),
            Intent::Docs => t("cp_q_docs"),
            Intent::Supply => t("cp_q_supply"),
            Intent::Stock => t("cp_q_stock"),
            Intent::Crew => t("cp_q_crew"),
            Intent::Machines => t("cp_q_machines"),
            Intent::Quality => t("cp_q_quality"),
            Intent::Safety => t("cp_q_safety"),
            Intent::Sales => t("cp_q_sales"),
            Intent::Project => t("cp_q_project"),
        }
    }

    /// Javob qaysi ekranda tekshiriladi.
    pub fn screen(self) -> Screen {
        match self {
            Intent::Overview | Intent::Attention => Screen::Analytics,
            Intent::Delays | Intent::Critical => Screen::Gantt,
            Intent::Money => Screen::Estimate,
            Intent::Cash => Screen::Analytics,
            Intent::Docs => Screen::ExecDocs,
            Intent::Supply => Screen::Requests,
            Intent::Stock => Screen::Warehouse,
            Intent::Crew => Screen::Timesheet,
            Intent::Machines => Screen::Machines,
            Intent::Quality => Screen::Quality,
            Intent::Safety => Screen::Safety,
            Intent::Sales => Screen::Deals,
            Intent::Project => Screen::AiCheck,
        }
    }

    /// Javobdagi sonlar qaysi yozuvlardan chiqqani (TZ XVIII.41).
    ///
    /// Ekran nomi «qayerda ko'rish mumkin»ni aytadi, bu esa «nimadan
    /// hisoblangan»ni. Ikkalasi birga turgandagina javobni tekshirib
    /// bo'ladi: foydalanuvchi o'sha yozuvlarni ochib, sonni o'zi qayta
    /// sanay oladi.
    pub fn source(self) -> &'static str {
        match self {
            Intent::Overview => t("cp_src_overview"),
            Intent::Attention => t("cp_src_attention"),
            Intent::Delays | Intent::Critical => t("cp_src_tasks"),
            Intent::Money => t("cp_src_money"),
            Intent::Cash => t("cp_src_cash"),
            Intent::Docs => t("cp_src_docs"),
            Intent::Supply => t("cp_src_supply"),
            Intent::Stock => t("cp_src_stock"),
            Intent::Crew => t("cp_src_crew"),
            Intent::Machines => t("cp_src_machines"),
            Intent::Quality => t("cp_src_quality"),
            Intent::Safety => t("cp_src_safety"),
            Intent::Sales => t("cp_src_sales"),
            Intent::Project => t("cp_src_project"),
        }
    }

    /// Ekranga mos niyat: modul yordamchisi shu orqali ochiladi.
    ///
    /// TZ har modul uchun «AI-yordamchi» talab qiladi (VI.30, VII.33,
    /// VIII.32, IX.35, XI.45-46, XII.39, XIII.40, XIV.38, XV.38). Har
    /// modulga alohida yordamchi yozish o'rniga bittasi ishlatiladi:
    /// modul ekranidan kelgan savol o'sha modulning javobiga tushadi.
    pub fn for_screen(screen: Screen) -> Option<Intent> {
        Some(match screen {
            Screen::Gantt | Screen::Ppr => Intent::Delays,
            Screen::Estimate => Intent::Money,
            Screen::ExecDocs | Screen::Inspections => Intent::Docs,
            Screen::Requests | Screen::Purchases => Intent::Supply,
            Screen::Warehouse | Screen::Materials => Intent::Stock,
            Screen::Timesheet => Intent::Crew,
            Screen::Machines => Intent::Machines,
            Screen::Quality => Intent::Quality,
            Screen::Safety => Intent::Safety,
            Screen::Sales | Screen::Deals => Intent::Sales,
            Screen::Client | Screen::Contracts => Intent::Cash,
            Screen::Foreman | Screen::Journal => Intent::Attention,
            Screen::Dashboard | Screen::Director | Screen::Portfolio => Intent::Overview,
            Screen::Notices | Screen::Analytics => Intent::Attention,
            Screen::TechSupervision => Intent::Docs,
            Screen::Passport | Screen::AiCheck => Intent::Project,
            _ => return None,
        })
    }

    /// Savolni tanish uchun kalit so'zlar (o'zbekcha va ruscha).
    fn keywords(self) -> &'static [&'static str] {
        match self {
            Intent::Overview => &[
                "umumiy",
                "holat",
                "qanday ket",
                "ahvol",
                "общ",
                "как дела",
                "состоян",
            ],
            Intent::Attention => &[
                "diqqat",
                "e'tibor",
                "muammo",
                "risk",
                "вниман",
                "проблем",
                "риск",
            ],
            Intent::Delays => &["kechik", "muddat", "kech qol", "просроч", "отстава", "срок"],
            Intent::Critical => &["kritik", "kritik yo'l", "критич", "критическ"],
            Intent::Money => &[
                "pul",
                "smeta",
                "summa",
                "moliya",
                "to'lov",
                "деньг",
                "смет",
                "сумм",
                "финанс",
                "оплат",
            ],
            Intent::Docs => &[
                "hujjat",
                "dalolatnoma",
                "akt",
                "imzo",
                "документ",
                "акт",
                "подпис",
            ],
            Intent::Supply => &[
                "ariza",
                "xarid",
                "ta'minot",
                "yetkaz",
                "заявк",
                "закупк",
                "снабж",
                "постав",
            ],
            Intent::Stock => &[
                "ombor",
                "qoldiq",
                "material",
                "zaxira",
                "склад",
                "остат",
                "материал",
                "запас",
            ],
            Intent::Cash => &[
                "pul oqim",
                "kassa",
                "uzilish",
                "denejn",
                "денежн",
                "поток",
                "кассов",
                "разрыв",
            ],
            Intent::Crew => &[
                "ishchi",
                "brigada",
                "tabel",
                "soat",
                "рабоч",
                "бригад",
                "табел",
                "час",
            ],
            Intent::Machines => &[
                "texnika",
                "mashina",
                "kran",
                "motosoat",
                "техник",
                "машин",
                "кран",
                "моточас",
            ],
            Intent::Quality => &["sifat", "nuqson", "brak", "качеств", "дефект", "брак"],
            Intent::Safety => &[
                "xavfsizlik",
                "hodisa",
                "instruktaj",
                "безопасн",
                "происшеств",
                "инструктаж",
                "тб",
            ],
            Intent::Sales => &[
                "sotuv",
                "kvartira",
                "shartnoma",
                "mijoz",
                "продаж",
                "квартир",
                "договор",
                "клиент",
            ],
            Intent::Project => &[
                "loyiha hujjat",
                "chizma",
                "ziddiyat",
                "kolliz",
                "bo'lim",
                "проектн",
                "чертеж",
                "коллиз",
                "раздел",
            ],
        }
    }
}

/// Javobning bir qatori: nomi va qiymati.
#[derive(Debug, Clone)]
pub struct Line {
    pub label: String,
    pub value: String,
    /// Diqqatni tortadigan qator (muammoli son).
    pub alert: bool,
}

fn line(label: &str, value: String) -> Line {
    Line {
        label: label.to_string(),
        value,
        alert: false,
    }
}

fn alert(label: &str, value: String) -> Line {
    Line {
        label: label.to_string(),
        value,
        alert: true,
    }
}

/// Tayyor javob.
#[derive(Debug, Clone)]
pub struct Answer {
    pub intent: Intent,
    pub title: String,
    pub lines: Vec<Line>,
    /// Qo'shimcha izoh — javobning chegarasi haqida.
    pub note: String,
    /// Sonlar qaysi yozuvlardan chiqqani (TZ XVIII.41).
    pub source: String,
    pub screen: Screen,
}

/// Umumiy niyatlar: «qanday ketyapti», «nimaga e'tibor berish kerak».
///
/// Ular oxirida tekshiriladi, chunki «holat», «qanday» kabi so'zlar deyarli
/// har bir savolda uchraydi va aniqroq savolni bosib qo'yishi mumkin
/// («ombor holati qanday?» — bu ombor haqidagi savol, umumiy holat haqida emas).
const GENERIC: &[Intent] = &[Intent::Overview, Intent::Attention];

/// Erkin yozilgan savoldan niyatni topadi.
///
/// Avval aniq mavzuli niyatlar tekshiriladi, keyin umumiylari; ikkalasida ham
/// eng ko'p kalit so'z mos kelgani tanlanadi. Hech biri mos kelmasa — `None`,
/// va ekran buni ochiq aytadi (o'ylab topilgan javob berilmaydi).
pub fn detect(question: &str) -> Option<Intent> {
    let q = question.to_lowercase();
    if q.trim().is_empty() {
        return None;
    }
    let pick = |generic: bool| -> Option<Intent> {
        let mut best: Option<(Intent, usize)> = None;
        for i in Intent::ALL {
            if GENERIC.contains(i) != generic {
                continue;
            }
            let hits = i.keywords().iter().filter(|k| q.contains(**k)).count();
            if hits == 0 {
                continue;
            }
            if best.is_none_or(|(_, b)| hits > b) {
                best = Some((*i, hits));
            }
        }
        best.map(|(i, _)| i)
    };
    pick(false).or_else(|| pick(true))
}

/// Niyat bo'yicha javobni hisoblaydi.
pub fn answer(intent: Intent, inp: &Input) -> Answer {
    let lines = match intent {
        Intent::Overview => overview(inp),
        Intent::Attention => attention(inp),
        Intent::Delays => delays(inp),
        Intent::Critical => critical(inp),
        Intent::Money => money_lines(inp),
        Intent::Docs => docs(inp),
        Intent::Supply => supply(inp),
        Intent::Stock => stock(inp),
        Intent::Cash => cash(inp),
        Intent::Crew => crew(inp),
        Intent::Machines => machines(inp),
        Intent::Quality => quality(inp),
        Intent::Safety => safety(inp),
        Intent::Sales => sales(inp),
        Intent::Project => project(inp),
    };
    Answer {
        intent,
        title: intent.question().to_string(),
        lines,
        note: t("cp_note").to_string(),
        source: intent.source().to_string(),
        screen: intent.screen(),
    }
}

// ================================================================ Javoblar

fn overview(inp: &Input) -> Vec<Line> {
    let p = inp.progress;
    let mut out = vec![
        line(t("cp_l_fact"), format!("{:.1}%", p.fact_pct)),
        line(t("cp_l_plan"), format!("{:.1}%", p.plan_pct)),
    ];
    if p.delay_days > 0 {
        out.push(alert(
            t("cp_l_delay"),
            format!("{} {}", p.delay_days, t("days_short")),
        ));
    } else {
        out.push(line(t("cp_l_delay"), t("cl_on_time").to_string()));
    }
    if let Some(d) = p.forecast_end {
        out.push(line(t("cp_l_forecast"), d.format("%d.%m.%Y").to_string()));
    }
    let f = analytics::findings(inp);
    out.push(line(
        t("cp_l_health"),
        format!("{} / 100", analytics::health(&f)),
    ));
    out
}

fn attention(inp: &Input) -> Vec<Line> {
    let f = analytics::findings(inp);
    if f.is_empty() {
        return vec![line(t("cp_l_none"), t("an_nothing").to_string())];
    }
    // Eng muhim beshtasi — qolganini analitika ekranida ko'rish mumkin.
    f.iter()
        .take(5)
        .map(|x| Line {
            label: format!("{} · {}", x.severity.label(), x.area.label()),
            value: x.fact.clone(),
            alert: matches!(x.severity, Severity::Critical | Severity::Major),
        })
        .collect()
}

fn delays(inp: &Input) -> Vec<Line> {
    let p = inp.progress;
    if p.overdue.is_empty() {
        return vec![line(t("cp_l_none"), t("cp_no_overdue").to_string())];
    }
    let mut out: Vec<Line> = p
        .overdue
        .iter()
        .take(8)
        .filter_map(|id| {
            let task = inp.tasks.iter().find(|t| t.id == *id)?;
            let critical = inp.schedule.is_critical(*id);
            Some(Line {
                label: format!("{} {}", task.wbs, task.name),
                value: format!("{:.0}%", task.progress),
                alert: critical,
            })
        })
        .collect();
    if p.overdue.len() > 8 {
        out.push(line(t("sv_more"), format!("{}", p.overdue.len() - 8)));
    }
    out
}

fn critical(inp: &Input) -> Vec<Line> {
    let list: Vec<&crate::model::Task> = inp
        .tasks
        .iter()
        .filter(|t| inp.schedule.is_critical(t.id))
        .collect();
    if list.is_empty() {
        return vec![line(t("cp_l_none"), t("cp_no_critical").to_string())];
    }
    let mut out: Vec<Line> = list
        .iter()
        .take(10)
        .map(|task| Line {
            label: format!("{} {}", task.wbs, task.name),
            value: format!("{:.0}%", task.progress),
            alert: task.progress < 99.99,
        })
        .collect();
    out.insert(0, line(t("cp_l_count"), format!("{}", list.len())));
    out
}

fn money_lines(inp: &Input) -> Vec<Line> {
    let c = inp.cost;
    let earned = inp.contract_sum * inp.progress.fact_pct / 100.0;
    let mut out = vec![
        line(t("cl_contract"), money(inp.contract_sum)),
        line(t("cl_estimate"), money(c.total)),
        line(t("cl_earned"), money(earned)),
        line(t("cl_paid"), money(inp.paid_total)),
    ];
    let unpaid = (earned - inp.paid_total).max(0.0);
    if unpaid > 0.0 {
        out.push(alert(t("cl_unpaid"), money(unpaid)));
    }
    if c.volume_excess > 0.0 {
        out.push(alert(t("an_c1_fact"), money(c.volume_excess)));
    }
    if c.price_saving > 0.0 {
        out.push(line(t("an_c3_fact"), money(c.price_saving)));
    }
    out
}

fn docs(inp: &Input) -> Vec<Line> {
    use crate::domain::ExecDocStatus;
    let count = |s: ExecDocStatus| inp.exec_docs.iter().filter(|d| d.status == s).count();
    let waiting = count(ExecDocStatus::Draft) + count(ExecDocStatus::OnReview);
    let undocumented = inp
        .tasks
        .iter()
        .filter(|t| t.progress >= 99.99)
        .filter(|t| {
            !inp.exec_docs
                .iter()
                .any(|d| d.task_id == Some(t.id) && d.status == ExecDocStatus::Signed)
        })
        .count();

    let mut out = vec![
        line(t("cp_l_signed"), count(ExecDocStatus::Signed).to_string()),
        line(t("cp_l_waiting"), waiting.to_string()),
    ];
    if count(ExecDocStatus::Rejected) > 0 {
        out.push(alert(
            t("eds_rejected"),
            count(ExecDocStatus::Rejected).to_string(),
        ));
    }
    if undocumented > 0 {
        out.push(alert(t("an_d1_fact"), undocumented.to_string()));
    }
    out
}

fn supply(inp: &Input) -> Vec<Line> {
    let open = inp
        .requests
        .iter()
        .filter(|q| !matches!(q.status, RequestStatus::Closed | RequestStatus::Rejected))
        .count();
    let late = inp.supply.iter().filter(|l| l.late).count();
    let unposted = inp
        .purchases
        .iter()
        .filter(|p| {
            matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed)
                && !inp.stock_moves.iter().any(|m| m.document == p.number)
        })
        .count();
    let in_transit: f64 = inp
        .purchases
        .iter()
        .filter(|p| !matches!(p.status, PurchaseStatus::Delivered | PurchaseStatus::Closed))
        .map(|p| p.amount())
        .sum();

    let mut out = vec![
        line(t("cp_l_open_requests"), open.to_string()),
        line(t("kpi_purchase_open"), money(in_transit)),
    ];
    if late > 0 {
        out.push(alert(t("kpi_req_late"), late.to_string()));
    }
    if unposted > 0 {
        out.push(alert(t("kpi_purchase_unposted"), unposted.to_string()));
    }
    out
}

fn stock(inp: &Input) -> Vec<Line> {
    let value: f64 = inp.stock.iter().map(|l| l.value).sum();
    let mut out = vec![
        line(t("kpi_stock_value"), money(value)),
        line(t("kpi_materials"), inp.materials.len().to_string()),
    ];
    // Zaxiradan kam materiallarni nomi bilan aytamiz — javob amaliy bo'lsin.
    let low: Vec<String> = inp
        .stock
        .iter()
        .filter(|l| l.below_min)
        .filter_map(|l| {
            inp.materials
                .iter()
                .find(|m| m.id == l.material_id)
                .map(|m| format!("{} ({} {})", m.name, trim(l.balance), m.unit))
        })
        .collect();
    if low.is_empty() {
        out.push(line(t("kpi_below_min"), "0".into()));
    } else {
        out.push(alert(t("kpi_below_min"), low.join(", ")));
    }
    let negative = inp.stock.iter().filter(|l| l.negative).count();
    if negative > 0 {
        out.push(alert(t("kpi_negative"), negative.to_string()));
    }
    out
}

fn crew(inp: &Input) -> Vec<Line> {
    let today = inp.today;
    let today_hours: f64 = inp
        .timesheet
        .iter()
        .filter(|e| e.date == today)
        .map(|e| e.hours)
        .sum();
    let today_people = inp.timesheet.iter().filter(|e| e.date == today).count();
    let week = today - chrono::Duration::days(7);
    let week_hours: f64 = inp
        .timesheet
        .iter()
        .filter(|e| e.date >= week)
        .map(|e| e.hours)
        .sum();
    // Ish haqi tabel ekranidagi bilan bitta hisobdan olinadi: smena va
    // ortiqcha ish koeffitsiyentlari bu yerda ham qo'llanadi.
    let wages = crate::checks::wages(inp.workers, inp.timesheet, week, today);
    let payroll: f64 = wages.iter().map(|w| w.wage).sum();
    let downtime: f64 = wages.iter().map(|w| w.downtime_hours).sum();
    let absences: i64 = wages.iter().map(|w| w.absence_days).sum();

    let mut out = vec![
        line(
            t("kpi_workers"),
            inp.workers.iter().filter(|w| w.active).count().to_string(),
        ),
        line(t("cp_l_today_people"), today_people.to_string()),
        line(t("cp_l_today_hours"), trim(today_hours)),
        line(t("kpi_hours_week"), trim(week_hours)),
        line(t("kpi_payroll"), money(payroll)),
    ];
    // Bo'sh turish va yo'qliklar — soat yo'qolishining sababi.
    if downtime > 0.0 {
        out.push(alert(t("kpi_downtime"), trim(downtime)));
    }
    if absences > 0 {
        out.push(line(t("kpi_absences"), absences.to_string()));
    }
    if today_people == 0 && !inp.workers.is_empty() {
        out.push(alert(t("cp_l_no_timesheet"), t("an_r4_fact").to_string()));
    }
    out
}

/// Pul oqimi va kassa uzilishi (TZ XVII.30–31).
fn cash(inp: &Input) -> Vec<Line> {
    let flow = crate::analytics::cash_flow(inp, 2, 4);
    let now = crate::analytics::month_of(inp.today);
    let mut out = Vec::new();

    if let Some(m) = flow.iter().find(|m| m.month == now) {
        out.push(line(t("cp_l_this_month_in"), money(m.income)));
        out.push(line(t("cp_l_this_month_out"), money(m.expense)));
        out.push(Line {
            label: t("col_net").to_string(),
            value: money(m.net),
            alert: m.net < 0.0,
        });
    }
    // Kelasi oylar rejasi.
    for m in flow.iter().filter(|m| m.month > now).take(3) {
        out.push(Line {
            label: m.month.format("%m.%Y").to_string(),
            value: format!("{} / {}", money(m.income), money(m.expense)),
            alert: m.net < 0.0,
        });
    }
    match crate::analytics::cash_gap(&flow, inp.today) {
        Some(g) => out.push(alert(
            t("cash_gap_title"),
            format!("{} · {}", g.month.format("%m.%Y"), money(g.amount)),
        )),
        None => out.push(line(t("cp_l_none"), t("cash_gap_none").to_string())),
    }
    out
}

fn machines(inp: &Input) -> Vec<Line> {
    use crate::domain::MachineStatus;
    let count = |s: MachineStatus| inp.machines.iter().filter(|m| m.status == s).count();
    let since = inp.today - chrono::Duration::days(30);
    let hours: f64 = inp
        .machine_logs
        .iter()
        .filter(|l| l.date >= since)
        .map(|l| l.hours)
        .sum();
    let fuel: f64 = inp
        .machine_logs
        .iter()
        .filter(|l| l.date >= since)
        .map(|l| l.fuel)
        .sum();

    let mut out = vec![
        line(t("kpi_machines"), inp.machines.len().to_string()),
        line(t("ms_working"), count(MachineStatus::Working).to_string()),
        line(t("ms_repair"), count(MachineStatus::Repair).to_string()),
        line(t("kpi_machine_hours"), trim(hours)),
        line(t("kpi_machine_fuel"), trim(fuel)),
    ];
    let expired: Vec<String> = inp
        .machines
        .iter()
        .filter(|m| m.inspection_until.is_some_and(|d| d < inp.today))
        .map(|m| m.name.clone())
        .collect();
    if !expired.is_empty() {
        out.push(alert(t("kpi_inspection"), expired.join(", ")));
    }
    out
}

fn quality(inp: &Input) -> Vec<Line> {
    let total = inp.quality.len();
    let pass = inp
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Pass)
        .count();
    let fail = inp
        .quality
        .iter()
        .filter(|q| q.result == QualityResult::Fail)
        .count();
    let overdue = inp
        .quality
        .iter()
        .filter(|q| q.result != QualityResult::Pass && q.deadline.is_some_and(|d| d < inp.today))
        .count();

    let mut out = vec![
        line(t("kpi_quality_total"), total.to_string()),
        line(
            t("kpi_quality_pass"),
            if total > 0 {
                format!("{:.0}%", pass as f64 / total as f64 * 100.0)
            } else {
                "—".into()
            },
        ),
    ];
    if fail > 0 {
        out.push(alert(t("kpi_quality_fail"), fail.to_string()));
    }
    if overdue > 0 {
        out.push(alert(t("kpi_quality_overdue"), overdue.to_string()));
    }
    out
}

fn safety(inp: &Input) -> Vec<Line> {
    use crate::domain::SafetyKind;
    let count = |k: SafetyKind| inp.safety.iter().filter(|s| s.kind == k).count();
    let open = inp
        .safety
        .iter()
        .filter(|s| matches!(s.status, IssueStatus::Open | IssueStatus::InWork))
        .count();
    let overdue = inp
        .safety
        .iter()
        .filter(|s| {
            matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                && s.deadline.is_some_and(|d| d < inp.today)
        })
        .count();

    let mut out = vec![
        line(
            t("kpi_safety_events"),
            count(SafetyKind::Violation).to_string(),
        ),
        line(
            t("kpi_safety_training"),
            count(SafetyKind::Training).to_string(),
        ),
        line(t("kpi_safety_open"), open.to_string()),
    ];
    if count(SafetyKind::Incident) > 0 {
        out.push(alert(
            t("kpi_safety_incidents"),
            count(SafetyKind::Incident).to_string(),
        ));
    }
    if overdue > 0 {
        out.push(alert(t("kpi_safety_overdue"), overdue.to_string()));
    }
    out
}

fn sales(inp: &Input) -> Vec<Line> {
    if inp.units.is_empty() {
        return vec![line(t("cp_l_none"), t("cp_no_sales").to_string())];
    }
    let s = inp.sales;
    let mut out = vec![
        line(t("kpi_units"), s.units.to_string()),
        line(t("us_free"), s.free.to_string()),
        line(t("us_sold"), s.sold.to_string()),
        line(t("kpi_contracted"), money(s.contracted)),
        line(t("kpi_received"), money(s.received)),
        line(t("kpi_debt"), money(s.debt)),
    ];
    if s.overdue > 0.0 {
        out.push(alert(t("kpi_overdue_pay"), money(s.overdue)));
    }
    out
}

/// Loyiha hujjatlari va ulardagi ziddiyatlar (TZ II, III).
///
/// Sonlar `issues` ro'yxatidan olinadi — bu AI tekshiruvi topgan
/// ziddiyatlarning **o'sha ro'yxati**, ikkinchi marta hisoblanmaydi.
fn project(inp: &Input) -> Vec<Line> {
    if inp.issues.is_empty() {
        return vec![line(t("cp_l_none"), t("cp_no_issues").to_string())];
    }
    let by = |s: Severity| inp.issues.iter().filter(|i| i.severity == s).count();
    let open = inp
        .issues
        .iter()
        .filter(|i| matches!(i.status, IssueStatus::Open | IssueStatus::InWork))
        .count();

    let mut out = vec![line(t("cp_l_count"), inp.issues.len().to_string())];
    if by(Severity::Critical) > 0 {
        out.push(alert(
            Severity::Critical.label(),
            by(Severity::Critical).to_string(),
        ));
    }
    if by(Severity::Major) > 0 {
        out.push(alert(
            Severity::Major.label(),
            by(Severity::Major).to_string(),
        ));
    }
    if by(Severity::Warning) > 0 {
        out.push(line(
            Severity::Warning.label(),
            by(Severity::Warning).to_string(),
        ));
    }
    out.push(if open > 0 {
        alert(t("cp_l_open"), open.to_string())
    } else {
        line(t("cp_l_open"), open.to_string())
    });

    // Eng jiddiy uchtasi nomma-nom — javob umumiy son bo'lib qolmasin.
    let mut top: Vec<&crate::domain::Issue> = inp.issues.iter().collect();
    top.sort_by_key(|i| rank(i.severity));
    for i in top.iter().take(3) {
        out.push(Line {
            label: i.code.clone(),
            value: if i.title.chars().count() > 46 {
                format!("{}…", i.title.chars().take(45).collect::<String>())
            } else {
                i.title.clone()
            },
            alert: matches!(i.severity, Severity::Critical | Severity::Major),
        });
    }
    out
}

/// Muhimlik tartibi — eng jiddiysi oldinda.
fn rank(s: Severity) -> u8 {
    match s {
        Severity::Critical => 0,
        Severity::Major => 1,
        Severity::Warning => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_map_to_intent() {
        assert_eq!(detect("Ombor holati qanday?"), Some(Intent::Stock));
        assert_eq!(detect("Что с деньгами?"), Some(Intent::Money));
        assert_eq!(detect("nima kechikkan"), Some(Intent::Delays));
        assert_eq!(detect("сколько рабочих"), Some(Intent::Crew));
        // Pul oqimi umumiy «pul» savolidan ajratiladi.
        assert_eq!(detect("pul oqimi qanday"), Some(Intent::Cash));
        assert_eq!(detect("кассовый разрыв"), Some(Intent::Cash));
        assert_eq!(detect("kvartira sotuvi"), Some(Intent::Sales));
    }

    /// Aniq mavzu umumiy so'zlardan ustun: «ombor holati qanday?» — ombor haqida.
    #[test]
    fn specific_topic_beats_generic_words() {
        assert_eq!(detect("ombor holati qanday"), Some(Intent::Stock));
        assert_eq!(detect("sifat qanday ketyapti"), Some(Intent::Quality));
        assert_eq!(detect("состояние склада"), Some(Intent::Stock));
        // Mavzusiz umumiy savol — umumiy javob.
        assert_eq!(detect("obyekt qanday ketyapti"), Some(Intent::Overview));
        assert_eq!(detect("nimaga e'tibor bering"), Some(Intent::Attention));
    }

    /// TZ XVIII.41: har javob sonlar qaysi yozuvlardan chiqqanini aytadi.
    #[test]
    fn every_answer_names_its_source() {
        for i in Intent::ALL {
            assert!(!i.source().is_empty(), "{i:?} manbasiz");
            // Manba ekran nomining takrori bo'lmasin — u boshqa savolga javob beradi.
            assert_ne!(i.source(), i.screen().label(), "{i:?}");
        }
    }

    /// TZ XVIII.43: ma'lumot ko'rsatadigan har bir ekrandan yordamchiga
    /// kirish bor — sozlama va yordamchining o'zi bundan mustasno.
    #[test]
    fn every_data_screen_reaches_the_copilot() {
        use crate::app::{Screen, NAV_GROUPS};
        for (_, screens) in NAV_GROUPS {
            for s in *screens {
                if matches!(s, Screen::Settings | Screen::Copilot) {
                    continue;
                }
                assert!(
                    Intent::for_screen(*s).is_some(),
                    "{s:?} ekranidan yordamchiga kirish yo'q"
                );
            }
        }
    }

    /// Tanilmagan savolga javob o'ylab topilmaydi.
    #[test]
    fn unknown_question_has_no_intent() {
        assert_eq!(detect("bugun ob-havo qanday"), None);
        assert_eq!(detect(""), None);
        assert_eq!(detect("   "), None);
    }

    /// Har bir niyatning kaliti bor va o'z savoli bilan tanilishi kerak.
    #[test]
    fn every_intent_is_reachable() {
        for i in Intent::ALL {
            assert!(!i.keywords().is_empty(), "{i:?} kalit so'zsiz");
            let first = i.keywords()[0];
            assert!(
                detect(first).is_some(),
                "{i:?} birinchi kaliti bo'yicha topilmadi"
            );
        }
    }
}
