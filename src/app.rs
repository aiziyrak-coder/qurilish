//! Состояние приложения и связка модулей с хранилищем.

use crate::checks::{self, Norm};
use crate::cpm::{self, Progress, Schedule};
use crate::db::Db;
use crate::domain::{
    Batch, Block, ConcreteTest, Contract, ContractChange, DayKind, Deal, Document, Element,
    ElementLink, Estimate, EstimateItem, ExecDoc, GeodesyPoint, Inspection, Inventory,
    InventoryLine, Issue, IssueModule, IssueStatus, JournalEntry, LabTest, Machine, MachineBooking,
    MachineLog, MachineRepair, Material, Payment, PaymentStage, PprDoc, Purchase, QualityCheck,
    Request, Reservation, SafetyEvent, SafetyZone, Severity, Shift, StockMove, TimesheetEntry,
    Tool, ToolIssue, Unit, Warehouse, WorkAcceptance, Worker,
};
use crate::i18n::{self, t, Lang};
use crate::model::*;
use crate::theme::{self, Theme};
use chrono::NaiveDate;
use std::collections::HashMap;

/// Platformaning ochiq bo'limi. Navigatsiya TZ ning butun tuzilishini
/// ko'rsatadi: har bir modul o'z raqami bilan turadi, hali ochilmagani esa
/// holati bilan belgilanadi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    /// Barcha obyektlar bo'yicha konsolidatsiya (TZ XVII.4).
    Portfolio,
    /// Barcha modullardan e'tibor talab qiladigan yozuvlar (umumiy talab).
    Notices,
    /// Texnik nazorat tekshiruvlari (TZ VII.3-6, 12-14).
    Inspections,
    /// Shartnomalar, o'zgarishlar, to'lov jadvali va qabul (TZ VIII.11-12, 21, 27-30).
    Contracts,
    // I. Qurilish loyihasini boshqarish
    Passport,
    Gantt,
    Ppr,
    // II-III. AI tekshiruv
    AiCheck,
    Estimate,
    // IV, V, XIV, XV. Ijro va nazorat
    ExecDocs,
    Journal,
    Quality,
    Safety,
    // VI-VIII. Kabinetlar
    Foreman,
    TechSupervision,
    Client,
    // IX-XII. Ta'minot
    Requests,
    Purchases,
    Warehouse,
    Materials,
    // XIII, XVI. Resurslar
    Timesheet,
    Machines,
    // XVII-XVIII. Analitika
    Analytics,
    /// Har modul bo'yicha hisobot (umumiy talab).
    Reports,
    Copilot,
    Director,
    // XIX-XX. Sotuv
    Sales,
    Deals,
    Settings,
}

/// Modulning tayyorlik darajasi — navigatsiyada va zaglushkada ko'rsatiladi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// Ekran ishlaydi.
    Ready,
    /// Domen turlari va baza sxemasi yozilgan, ekran qolgan.
    Storage,
    /// Hali boshlanmagan: server, mobil klient yoki tashqi xizmat kerak.
    Planned,
}

impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Screen::Dashboard => t("screen_dashboard"),
            Screen::Portfolio => t("screen_portfolio"),
            Screen::Notices => t("screen_notices"),
            Screen::Director => t("screen_director"),
            Screen::Inspections => t("screen_inspections"),
            Screen::Contracts => t("screen_contracts"),
            Screen::Passport => t("screen_passport"),
            Screen::Gantt => t("screen_gantt"),
            Screen::Ppr => t("screen_ppr"),
            Screen::AiCheck => t("screen_ai_check"),
            Screen::Estimate => t("screen_estimate"),
            Screen::ExecDocs => t("screen_exec_docs"),
            Screen::Journal => t("screen_journal"),
            Screen::Quality => t("screen_quality"),
            Screen::Safety => t("screen_safety"),
            Screen::Foreman => t("screen_foreman"),
            Screen::TechSupervision => t("screen_tech_supervision"),
            Screen::Client => t("screen_client"),
            Screen::Requests => t("screen_requests"),
            Screen::Purchases => t("screen_purchases"),
            Screen::Warehouse => t("screen_warehouse"),
            Screen::Materials => t("screen_materials"),
            Screen::Timesheet => t("screen_timesheet"),
            Screen::Machines => t("screen_machines"),
            Screen::Analytics => t("screen_analytics"),
            Screen::Reports => t("screen_reports"),
            Screen::Copilot => t("screen_copilot"),
            Screen::Sales => t("screen_sales"),
            Screen::Deals => t("screen_deals"),
            Screen::Settings => t("screen_settings"),
        }
    }

    /// Ekran nima uchun kerakligi — bir qatorda.
    ///
    /// Ro'yxatda nom bor, lekin nom ekranni tushuntirmaydi: «Arizalar» —
    /// bu nima, kim to'ldiradi, keyin nima bo'ladi? Shu qator har ekran
    /// tepasida turadi va shu savolga javob beradi.
    pub fn purpose(self) -> &'static str {
        match self {
            Screen::Dashboard => t("purpose_dashboard"),
            Screen::Portfolio => t("purpose_portfolio"),
            Screen::Notices => t("purpose_notices"),
            Screen::Director => t("purpose_director"),
            Screen::Inspections => t("purpose_inspections"),
            Screen::Contracts => t("purpose_contracts"),
            Screen::Passport => t("purpose_passport"),
            Screen::Gantt => t("purpose_gantt"),
            Screen::Ppr => t("purpose_ppr"),
            Screen::AiCheck => t("purpose_ai_check"),
            Screen::Estimate => t("purpose_estimate"),
            Screen::ExecDocs => t("purpose_exec_docs"),
            Screen::Journal => t("purpose_journal"),
            Screen::Quality => t("purpose_quality"),
            Screen::Safety => t("purpose_safety"),
            Screen::Foreman => t("purpose_foreman"),
            Screen::TechSupervision => t("purpose_tech_supervision"),
            Screen::Client => t("purpose_client"),
            Screen::Requests => t("purpose_requests"),
            Screen::Purchases => t("purpose_purchases"),
            Screen::Warehouse => t("purpose_warehouse"),
            Screen::Materials => t("purpose_materials"),
            Screen::Timesheet => t("purpose_timesheet"),
            Screen::Machines => t("purpose_machines"),
            Screen::Analytics => t("purpose_analytics"),
            Screen::Reports => t("purpose_reports"),
            Screen::Copilot => t("purpose_copilot"),
            Screen::Sales => t("purpose_sales"),
            Screen::Deals => t("purpose_deals"),
            Screen::Settings => t("purpose_settings"),
        }
    }

    /// TZ dagi bo'lim raqami. Umumiy ko'rinish va sozlamalar TZ moduli emas.
    pub fn numeral(self) -> &'static str {
        match self {
            Screen::Dashboard
            | Screen::Portfolio
            | Screen::Notices
            | Screen::Director
            // Hisobotlar TZ da alohida modul emas — u barcha modullardan
            // yig'iladi, shuning uchun raqami ham yo'q.
            | Screen::Reports
            | Screen::Settings => "",
            Screen::Passport => "I.1",
            Screen::Gantt => "I.2",
            Screen::Ppr => "I.3",
            Screen::AiCheck => "II",
            Screen::Estimate => "III",
            Screen::ExecDocs => "IV",
            Screen::Journal => "V",
            Screen::Foreman => "VI",
            Screen::TechSupervision | Screen::Inspections => "VII",
            Screen::Client | Screen::Contracts => "VIII",
            Screen::Requests => "IX",
            Screen::Purchases => "X",
            Screen::Warehouse => "XI",
            Screen::Materials => "XII",
            Screen::Timesheet => "XIII",
            Screen::Quality => "XIV",
            Screen::Safety => "XV",
            Screen::Machines => "XVI",
            Screen::Analytics => "XVII",
            Screen::Copilot => "XVIII",
            Screen::Sales => "XIX",
            Screen::Deals => "XX",
        }
    }

    pub fn readiness(self) -> Readiness {
        match self {
            Screen::Dashboard
            | Screen::Portfolio
            | Screen::Notices
            | Screen::Director
            | Screen::Inspections
            | Screen::Contracts
            | Screen::Passport
            | Screen::Gantt
            | Screen::Ppr
            | Screen::AiCheck
            | Screen::Estimate
            | Screen::ExecDocs
            | Screen::Journal
            | Screen::Warehouse
            | Screen::Materials
            | Screen::Requests
            | Screen::Purchases
            | Screen::Sales
            | Screen::Deals
            | Screen::Timesheet
            | Screen::Quality
            | Screen::Safety
            | Screen::Machines
            | Screen::Analytics
            | Screen::Foreman
            | Screen::TechSupervision
            | Screen::Client
            | Screen::Copilot
            | Screen::Reports
            | Screen::Settings => Readiness::Ready,
        }
    }
}

/// Navigatsiya guruhlari — TZ ning mantiqiy bloklari.
/// Yon paneldagi bo'limlar — **qurilish jarayoni ketma-ketligi bo'yicha**.
///
/// Tartib TZ modullarining raqami bo'yicha emas, ishning haqiqiy borishi
/// bo'yicha: loyiha → reja → ta'minot → qurilish → nazorat → hujjat →
/// sotuv → tahlil. Odam ekranni qidirmaydi: u qaysi bosqichda turgan
/// bo'lsa, o'sha guruhga qaraydi.
///
/// Yuqoridagi «bugun» guruhi bosqich emas: bu har kuni ochiladigan ikki
/// ekran, shuning uchun u raqamsiz turadi.
pub const NAV_GROUPS: &[(&str, &[Screen])] = &[
    ("nav_today", &[Screen::Dashboard, Screen::Notices]),
    (
        "nav_stage_design",
        &[Screen::Passport, Screen::AiCheck, Screen::Estimate],
    ),
    ("nav_stage_plan", &[Screen::Gantt, Screen::Ppr]),
    (
        "nav_stage_supply",
        &[
            Screen::Requests,
            Screen::Purchases,
            Screen::Warehouse,
            Screen::Materials,
        ],
    ),
    (
        "nav_stage_build",
        &[
            Screen::Foreman,
            Screen::Journal,
            Screen::Timesheet,
            Screen::Machines,
        ],
    ),
    (
        "nav_stage_control",
        &[
            Screen::Quality,
            Screen::Safety,
            Screen::TechSupervision,
            Screen::Inspections,
        ],
    ),
    (
        "nav_stage_docs",
        &[Screen::ExecDocs, Screen::Contracts, Screen::Client],
    ),
    ("nav_stage_sales", &[Screen::Sales, Screen::Deals]),
    (
        "nav_stage_review",
        &[
            Screen::Reports,
            Screen::Analytics,
            Screen::Director,
            Screen::Portfolio,
            Screen::Copilot,
        ],
    ),
    ("nav_system", &[Screen::Settings]),
];

/// AI tekshiruv ekranidagi ochiq bo'lim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckTab {
    Issues,
    Clash,
    Action,
    /// Elementlarning joylashuv rejasi (TZ VI.13, VIII.6).
    Plan,
    Graph,
    Elements,
    Relations,
    Norms,
}

impl CheckTab {
    pub const ALL: [CheckTab; 8] = [
        CheckTab::Issues,
        CheckTab::Clash,
        CheckTab::Action,
        CheckTab::Plan,
        CheckTab::Graph,
        CheckTab::Elements,
        CheckTab::Relations,
        CheckTab::Norms,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CheckTab::Issues => t("tab_issues"),
            CheckTab::Clash => t("tab_clash"),
            CheckTab::Action => t("tab_action"),
            CheckTab::Plan => t("plan_title"),
            CheckTab::Graph => t("tab_graph"),
            CheckTab::Elements => t("tab_elements"),
            CheckTab::Relations => t("tab_relations"),
            CheckTab::Norms => t("tab_norms"),
        }
    }
}

/// Что показывать полосой на диаграмме Ганта.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GanttScale {
    Day,
    Week,
    Month,
}

impl GanttScale {
    pub const ALL: [GanttScale; 3] = [GanttScale::Day, GanttScale::Week, GanttScale::Month];

    pub fn label(self) -> &'static str {
        match self {
            GanttScale::Day => t("scale_day"),
            GanttScale::Week => t("scale_week"),
            GanttScale::Month => t("scale_month"),
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            GanttScale::Day => "day",
            GanttScale::Week => "week",
            GanttScale::Month => "month",
        }
    }

    pub fn parse(s: &str) -> GanttScale {
        match s {
            "day" => GanttScale::Day,
            "month" => GanttScale::Month,
            _ => GanttScale::Week,
        }
    }

    /// Bir kunga to'g'ri keladigan piksellar soni.
    pub fn px_per_day(self) -> f32 {
        match self {
            GanttScale::Day => 22.0,
            GanttScale::Week => 6.0,
            GanttScale::Month => 2.2,
        }
    }
}

/// Foydalanuvchi sozlamalari — `settings` jadvalida saqlanadi.
#[derive(Debug, Clone)]
pub struct Settings {
    pub lang: Lang,
    pub theme: Theme,
    pub ui_scale: f32,
    pub default_scale: GanttScale,
    pub show_weekends: bool,
    pub default_currency: String,
    pub default_task_days: i64,
    /// Obyektda mavjud ishchi va texnika soni — PPR yetarlilik tekshiruvi uchun.
    pub avail_workers: i64,
    pub avail_machines: i64,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            lang: Lang::Uz,
            theme: Theme::Light,
            ui_scale: 1.0,
            default_scale: GanttScale::Week,
            show_weekends: true,
            default_currency: "UZS".into(),
            default_task_days: 5,
            avail_workers: 0,
            avail_machines: 0,
        }
    }
}

impl Settings {
    /// Bazadan o'qish; qiymat bo'lmasa standart qiymat ishlatiladi.
    pub fn load(db: &Db) -> Settings {
        let d = Settings::default();
        let get = |k: &str| db.get_setting(k);
        Settings {
            lang: get("lang").map(|v| Lang::parse(&v)).unwrap_or(d.lang),
            theme: get("theme").map(|v| Theme::parse(&v)).unwrap_or(d.theme),
            ui_scale: get("ui_scale")
                .and_then(|v| v.parse::<f32>().ok())
                .map(|v| v.clamp(0.8, 1.6))
                .unwrap_or(d.ui_scale),
            default_scale: get("gantt_scale")
                .map(|v| GanttScale::parse(&v))
                .unwrap_or(d.default_scale),
            show_weekends: get("show_weekends")
                .map(|v| v == "1")
                .unwrap_or(d.show_weekends),
            default_currency: get("currency").unwrap_or(d.default_currency),
            default_task_days: get("task_days")
                .and_then(|v| v.parse::<i64>().ok())
                .map(|v| v.clamp(1, 365))
                .unwrap_or(d.default_task_days),
            avail_workers: get("avail_workers")
                .and_then(|v| v.parse::<i64>().ok())
                .map(|v| v.clamp(0, 10_000))
                .unwrap_or(d.avail_workers),
            avail_machines: get("avail_machines")
                .and_then(|v| v.parse::<i64>().ok())
                .map(|v| v.clamp(0, 1_000))
                .unwrap_or(d.avail_machines),
        }
    }

    /// Bazaga yozish.
    pub fn save(&self, db: &Db) {
        let _ = db.set_setting("lang", self.lang.code());
        let _ = db.set_setting("theme", self.theme.code());
        let _ = db.set_setting("ui_scale", &format!("{:.2}", self.ui_scale));
        let _ = db.set_setting("gantt_scale", self.default_scale.code());
        let _ = db.set_setting("show_weekends", if self.show_weekends { "1" } else { "0" });
        let _ = db.set_setting("currency", &self.default_currency);
        let _ = db.set_setting("task_days", &self.default_task_days.to_string());
        let _ = db.set_setting("avail_workers", &self.avail_workers.to_string());
        let _ = db.set_setting("avail_machines", &self.avail_machines.to_string());
    }

    /// Global holatga qo'llash: til va mavzu.
    pub fn apply_globals(&self) {
        i18n::set_lang(self.lang);
        theme::set_theme(self.theme);
    }
}

/// Hisobot katagini matnga aylantiradi — serverga yuborish uchun.
///
/// Formatlash shu yerda bir marta qilinadi: kabinetda ham, ekranda ham
/// son bir xil ko'rinishda bo'lsin.
fn cell_text(c: Option<&crate::docgen::Cell>) -> String {
    use crate::docgen::Cell;
    match c {
        Some(Cell::Text(s)) => s.clone(),
        Some(Cell::Num(v)) => crate::ui::materials::trim_num(*v),
        Some(Cell::Money(v)) => crate::ui::money(*v),
        Some(Cell::Date(d)) => d.format("%d.%m.%Y").to_string(),
        _ => String::new(),
    }
}

/// Obyektning server tomonidagi kaliti.
///
/// Kod bo'sh bo'lishi mumkin, shuning uchun nom zaxira sifatida olinadi:
/// ikkala nusxada bir xil kalit chiqishi kerak, aks holda paketlar
/// boshqa obyektga tushib qolardi.
pub fn project_key(p: &Project) -> String {
    let code = p.code.trim();
    if !code.is_empty() {
        return code.to_string();
    }
    p.name.trim().to_string()
}

/// Amallar tarixida shuncha yozuv saqlanadi.
///
/// Chegara bor: jurnal bazani cheksiz shishirmasligi kerak. 50 000 yozuv
/// bir necha oylik faol ishga yetadi.
const AUDIT_KEEP: i64 = 50_000;

/// Material yetarliligi shuncha kun oldin tekshiriladi (TZ XII.28).
///
/// Ikki hafta — buyurtma berib, materialni olib kelishga yetadigan eng qisqa
/// muddat; undan kechroq bilish foydasiz bo'lib qoladi.
const READINESS_DAYS: i64 = 14;

/// Park bo'yicha foydalanish shuncha kun ortga qarab hisoblanadi (TZ XVI.39).
///
/// Bir hafta juda qisqa: bitta bo'sh kun koeffitsiyentni buzib ko'rsatardi.
/// Bir chorak esa juda uzun: mavsum o'zgarishi yo'qolib ketardi.
const PARK_DAYS: i64 = 30;

/// Xodim ehtiyoji shuncha kun oldinga qarab hisoblanadi (TZ XIII.26-27).
const STAFF_HORIZON: i64 = 30;

/// Ish grafigi shuncha kun ortga qarab tekshiriladi (TZ XIII.12).
const SCHEDULE_DAYS: i64 = 30;

/// Xarid rejasi shuncha kun oldinga qaraydi (TZ X.6, XVII.10).
const PLAN_HORIZON: i64 = 45;

pub struct App {
    pub db: Db,
    pub projects: Vec<Project>,
    pub current: Option<i64>,

    pub parties: Vec<Party>,
    pub tasks: Vec<Task>,
    pub links: Vec<Link>,
    pub schedule: Schedule,
    pub progress: Progress,

    pub screen: Screen,
    pub today: NaiveDate,

    // Состояние экрана ГПР
    pub selected_task: Option<i64>,
    pub scale: GanttScale,
    /// Смещение прокрутки таймлайна в днях от начала проекта.
    pub timeline_offset: f32,
    /// Вертикальная прокрутка списка работ, в строках.
    pub row_offset: f32,
    /// Keyingi kadrda butun grafik oynaga sig'dirilsin.
    /// Masshtab oynaning kengligiga bog'liq, shuning uchun uni chizish paytida
    /// hisoblaymiz — App oyna o'lchamini bilmaydi.
    pub fit_timeline: bool,
    pub px_per_day: f32,
    pub show_critical_only: bool,
    /// Faqat muddati o'tgan ishlarni ko'rsatish.
    pub filter_overdue: bool,
    /// «Tahlil» oynasi: kim aybdor va nima qilish kerak (TZ I.2).
    pub gantt_analysis: bool,
    pub filter_section: Section,
    pub search: String,
    /// Связывание работ мышью: работа-предшественник ждёт выбора преемника.
    pub linking_from: Option<i64>,
    pub link_dialog: Option<Link>,
    /// Дробный остаток перетаскивания полосы Ганта, в днях.
    pub drag_acc: f32,

    // ---- II-III modullar: loyiha va smeta tekshiruvi ----
    pub issues: Vec<Issue>,
    pub documents: Vec<Document>,
    pub ppr_docs: Vec<PprDoc>,
    pub exec_docs: Vec<ExecDoc>,
    pub journal: Vec<JournalEntry>,
    /// Maydonchaga kirish-chiqish belgilari (TZ XIII.4). Faqat telefondan
    /// keladi — ilovada qo'lda kiritilmaydi.
    pub attendance: Vec<crate::domain::Attendance>,
    /// Ofis bilan yozishma (TZ VI.32). Serverda yuritiladi, bu yerda nusxa.
    pub messages: Vec<crate::domain::ChatMessage>,
    /// Yozishma maydonidagi matn.
    pub chat_draft: String,
    /// Keyingi sinxronizatsiyada yuboriladigan xabar.
    pub chat_outgoing: String,
    /// Imzo daftari (TZ IV.18, V.28).
    pub sign_log: Vec<crate::domain::SignEntry>,
    /// Serverda qayd etilgan zanjir belgilari.
    pub chain_marks: Vec<crate::sync::ChainMark>,
    /// Geometriya bo'yicha kolliziyalar (TZ II, VII.31).
    ///
    /// Elementlar o'qilganda bir marta hisoblanadi: ekran har kadrda
    /// qayta chiziladi va u yerda qayta hisoblash ortiqcha bo'lardi.
    pub clashes: Vec<crate::clash::Clash>,
    /// Narxlar bazasi (TZ III.15). Obyektga bog'liq emas.
    pub price_book: Vec<crate::prices::PriceRow>,
    /// Narxlar bazasini tozalash tasdig'i kutilyaptimi.
    pub confirm_clear_prices: bool,
    /// Oxirgi foto-nazorat natijasi (TZ V.9). Tugma bosilganda hisoblanadi:
    /// fayllar diskdan o'qiladi va bu har kadrda qilinadigan ish emas.
    pub photo_report: Option<Vec<crate::photocheck::PhotoIssue>>,
    pub materials: Vec<Material>,
    pub stock_moves: Vec<StockMove>,
    pub warehouses: Vec<Warehouse>,
    pub batches: Vec<Batch>,
    pub reservations: Vec<Reservation>,
    pub inventories: Vec<Inventory>,
    pub inventory_lines: Vec<InventoryLine>,
    /// Ishga material sarf normalari (TZ XI.15).
    pub material_norms: Vec<crate::domain::MaterialNorm>,
    /// Material analoglari (TZ XII.9).
    pub material_alts: Vec<crate::domain::MaterialAlt>,
    /// Brigadalar (TZ XIII.8).
    pub brigades: Vec<crate::domain::Brigade>,
    /// Yetkazib beruvchilar va tijorat takliflari (TZ X.7–12).
    pub suppliers: Vec<crate::domain::Supplier>,
    pub quotes: Vec<crate::domain::Quote>,
    /// Arizalarning kelishuv bosqichlari (TZ IX.8).
    pub approvals: Vec<crate::domain::Approval>,
    /// Sifat chek-listlari va nazorat nuqtalari (TZ XIV.8).
    pub checklists: Vec<crate::domain::Checklist>,
    pub checklist_items: Vec<crate::domain::ChecklistItem>,
    pub check_points: Vec<crate::domain::CheckPoint>,
    /// Ruxsatlar, SIZ va naryad-dopusklar (TZ XV.4–12).
    pub worker_permits: Vec<crate::domain::WorkerPermit>,
    pub ppe_issues: Vec<crate::domain::PpeIssue>,
    pub work_permits: Vec<crate::domain::WorkPermit>,
    /// Nazorat nuqtalari paneli ochilgan tekshiruv.
    pub quality_open: Option<i64>,
    /// Kelishuv paneli ochilgan ariza.
    pub request_open: Option<i64>,
    pub purchase_budgets: Vec<crate::domain::PurchaseBudget>,
    /// Omborda tanlangan bo'lim (barcha omborlar — `None`).
    pub warehouse_filter: Option<i64>,
    pub requests: Vec<Request>,
    pub purchases: Vec<Purchase>,
    pub blocks: Vec<Block>,
    pub units: Vec<Unit>,
    pub deals: Vec<Deal>,
    pub payments: Vec<Payment>,
    /// Shaxmatkada tanlangan blok va birlik.
    pub workers: Vec<Worker>,
    pub timesheet: Vec<TimesheetEntry>,
    pub quality: Vec<QualityCheck>,
    pub safety: Vec<SafetyEvent>,
    pub machines: Vec<Machine>,
    pub machine_logs: Vec<MachineLog>,
    /// VII. Texnik nazorat: tekshiruvlar, beton sinovlari, geodeziya.
    pub inspections: Vec<Inspection>,
    /// XI.32-34. Asboblar va ularni berish.
    pub tools: Vec<Tool>,
    /// XIV.22-25. Laboratoriya va maydon sinovlari.
    pub lab_tests: Vec<LabTest>,
    /// XVI.10-12, 23-24. Texnika bandligi va ta'miri.
    /// XV.15, 22-24. Xavfli zonalar va xavfsizlik inventari.
    pub zones: Vec<SafetyZone>,
    pub bookings: Vec<MachineBooking>,
    pub repairs: Vec<MachineRepair>,
    pub tool_issues: Vec<ToolIssue>,
    /// Bildirishnomalar: ma'lumot o'zgarganda bir marta hisoblanadi.
    ///
    /// Uni har kadrda qayta yig'ish bo'lmaydi — hisob o'nlab SQL so'rovni
    /// o'z ichiga oladi va yon panel har kadrda chiziladi.
    pub notices: Vec<crate::notify::Notice>,
    /// Ro'yxat qaysi baza revizyasida yig'ilgani.
    pub notices_rev: u64,
    /// VIII. Buyurtmachi: shartnomalar, o'zgarishlar, to'lovlar, qabul.
    pub contracts: Vec<Contract>,
    pub contract_changes: Vec<ContractChange>,
    pub payment_stages: Vec<PaymentStage>,
    pub acceptances: Vec<WorkAcceptance>,
    pub concrete_tests: Vec<ConcreteTest>,
    pub geodesy_points: Vec<GeodesyPoint>,
    /// Tabelda ko'rsatilayotgan hafta boshi (dushanba).
    pub timesheet_week: Option<chrono::NaiveDate>,
    /// Ilova foydalanuvchilari va joriy tanlangani (TZ VI–VIII).
    /// Til modeli sozlamasi. Sukut bo'yicha o'chiq — ilova lokal qoladi.
    pub llm: crate::llm::Config,
    /// Server bilan sinxronizatsiya sozlamasi (TZ VI.36).
    pub sync: crate::sync::Config,
    /// Tashqi xabar nuqtasi (TZ VI.29, XVI.45). Sukut bo'yicha o'chiq.
    pub hook: crate::hook::Config,
    /// Kutilayotgan yuborish natijasi.
    #[allow(clippy::type_complexity)]
    pub hook_pending: Option<std::sync::mpsc::Receiver<(crate::hook::Outcome, Vec<String>)>>,
    /// Oxirgi yuborish natijasi: xabar va u xatomi.
    pub hook_status: Option<(String, bool)>,
    /// Kutilayotgan sinxronizatsiya natijasi.
    pub sync_pending:
        Option<std::sync::mpsc::Receiver<Result<crate::sync::Outcome, crate::sync::Error>>>,
    /// Oxirgi sinxronizatsiya natijasi: xabar va u xatomi.
    pub sync_status: Option<(String, bool)>,
    /// Modul ekranidan yordamchiga uzatilgan savol (TZ VI.30 va h.k.).
    pub copilot_intent: Option<crate::copilot::Intent>,
    /// Hisobotlar ekranida tanlangan hisobot va davr.
    ///
    /// Tanlov ekran xotirasida emas, ilova holatida turadi: Ctrl+E ham
    /// aynan ko'rinib turgan hisobotni saqlashi kerak.
    pub report_kind: usize,
    pub report_preset: usize,
    /// Qidiruvdan kelingan jurnal yozuvining sanasi (TZ V.30).
    ///
    /// Jurnal yozuvi alohida tanlanmaydi — u kunlik yozuv, shuning uchun
    /// ajratish ham kun bo'yicha bo'ladi.
    pub journal_focus: Option<NaiveDate>,
    /// Ish grafigi: qaysi kunlar ish kuni (TZ XIII.12).
    pub work_schedule: checks::WorkSchedule,
    /// Texnikaning kunlik ko'rigi (TZ XVI.28, XV.18).
    pub machine_checks: Vec<crate::domain::MachineCheck>,
    /// Yozuvlarga qoldirilgan izohlar (umumiy mexanizm).
    pub notes: Vec<crate::domain::Note>,
    /// Yozuvlarga biriktirilgan fayllar (umumiy mexanizm).
    pub attachments: Vec<crate::domain::Attachment>,
    /// Model bilan suhbat: savol va javoblar ketma-ketligi.
    ///
    /// Suhbat **bazaga yozilmaydi**: unda obyekt ma'lumoti bo'lishi mumkin va
    /// u tashqi xizmatga jo'natilgan — nusxasini saqlab qo'yish keraksiz xavf.
    pub llm_chat: Vec<crate::llm::Turn>,
    /// Fonda ketayotgan so'rov. Bo'lsa — ekranda kutish ko'rsatiladi.
    pub llm_pending:
        Option<std::sync::mpsc::Receiver<Result<crate::llm::Answer, crate::llm::Error>>>,
    /// Oxirgi xato: tarjima kaliti, tafsiloti va qaytadan urinish
    /// ma'nolimi. Uchinchi qiymat `llm::Error` ning o'zidan olinadi —
    /// interfeys xatolar ro'yxatini takrorlamasin.
    pub llm_error: Option<(&'static str, String, bool)>,
    /// Suhbatda sarflangan tokenlar — xarajat ko'rinib tursin.
    pub llm_tokens: u32,
    pub users: Vec<crate::roles::User>,
    pub current_user: Option<i64>,
    pub sales_block: Option<i64>,
    pub selected_unit: Option<i64>,
    pub selected_deal: Option<i64>,
    pub selected_ppr: Option<i64>,
    pub elements: Vec<Element>,
    pub element_links: Vec<ElementLink>,
    pub estimates: Vec<Estimate>,
    pub estimate_items: Vec<EstimateItem>,
    pub current_estimate: Option<i64>,
    /// Normativ reyestri: qoida kaliti -> me'yoriy asos (TZ II.17).
    pub norms: HashMap<String, Norm>,
    pub selected_issue: Option<i64>,
    pub selected_element: Option<i64>,
    /// Tekshiruv ekrani birinchi ochilganda natija avtomatik hisoblanadi —
    /// bo'sh ro'yxat o'rniga darhol ko'rinadi. Har biri sessiyada bir marta,
    /// obyekt almashganda qaytadan.
    /// Umumiy qidiruv oynasi (Ctrl+K).
    pub search_open: bool,
    pub search_query: String,
    /// Oyna ochilgan kadrda kursorni maydonga qo'yish kerak.
    pub search_focus: bool,
    pub auto_check_project: bool,
    pub auto_check_estimate: bool,
    pub auto_check_ppr: bool,
    pub check_tab: CheckTab,
    pub issue_sev: Option<Severity>,
    pub issue_status: Option<IssueStatus>,
    pub issue_search: String,

    pub toast: Option<(String, f32)>,
    pub confirm_delete_task: Option<i64>,
    pub confirm_delete_project: Option<i64>,

    pub settings: Settings,
    /// Mavzu yoki masshtab o'zgarganda egui uslubini qayta qurish kerak.
    pub restyle: bool,
}

impl App {
    pub fn new(db: Db) -> App {
        let settings = Settings::load(&db);
        settings.apply_globals();
        let mut app = App {
            db,
            projects: Vec::new(),
            current: None,
            parties: Vec::new(),
            tasks: Vec::new(),
            links: Vec::new(),
            schedule: Schedule::default(),
            progress: Progress::default(),
            // Diagnostika uchun: QURAI_SCREEN=passport bilan kerakli ekranda ochiladi.
            screen: match std::env::var("QURAI_SCREEN").as_deref() {
                Ok("portfolio") => Screen::Portfolio,
                Ok("notices") => Screen::Notices,
                Ok("director") => Screen::Director,
                Ok("inspections") => Screen::Inspections,
                Ok("contracts") => Screen::Contracts,
                Ok("passport") => Screen::Passport,
                Ok("gantt") => Screen::Gantt,
                Ok("ppr") => Screen::Ppr,
                Ok("aicheck") => Screen::AiCheck,
                Ok("estimate") => Screen::Estimate,
                Ok("execdocs") => Screen::ExecDocs,
                Ok("journal") => Screen::Journal,
                Ok("materials") => Screen::Materials,
                Ok("warehouse") => Screen::Warehouse,
                Ok("requests") => Screen::Requests,
                Ok("purchases") => Screen::Purchases,
                Ok("sales") => Screen::Sales,
                Ok("deals") => Screen::Deals,
                Ok("timesheet") => Screen::Timesheet,
                Ok("quality") => Screen::Quality,
                Ok("safety") => Screen::Safety,
                Ok("machines") => Screen::Machines,
                Ok("analytics") => Screen::Analytics,
                Ok("foreman") => Screen::Foreman,
                Ok("supervision") => Screen::TechSupervision,
                Ok("client") => Screen::Client,
                Ok("copilot") => Screen::Copilot,
                Ok("settings") => Screen::Settings,
                _ => Screen::Dashboard,
            },
            today: chrono::Local::now().date_naive(),
            selected_task: None,
            scale: settings.default_scale,
            timeline_offset: 0.0,
            row_offset: 0.0,
            fit_timeline: true,
            px_per_day: settings.default_scale.px_per_day(),
            show_critical_only: false,
            filter_overdue: false,
            gantt_analysis: false,
            filter_section: Section::None,
            search: String::new(),
            linking_from: None,
            link_dialog: None,
            drag_acc: 0.0,
            issues: Vec::new(),
            documents: Vec::new(),
            ppr_docs: Vec::new(),
            exec_docs: Vec::new(),
            journal: Vec::new(),
            attendance: Vec::new(),
            messages: Vec::new(),
            chat_draft: String::new(),
            chat_outgoing: String::new(),
            sign_log: Vec::new(),
            chain_marks: Vec::new(),
            clashes: Vec::new(),
            price_book: Vec::new(),
            confirm_clear_prices: false,
            photo_report: None,
            materials: Vec::new(),
            stock_moves: Vec::new(),
            warehouses: Vec::new(),
            batches: Vec::new(),
            reservations: Vec::new(),
            inventories: Vec::new(),
            inventory_lines: Vec::new(),
            material_norms: Vec::new(),
            material_alts: Vec::new(),
            brigades: Vec::new(),
            suppliers: Vec::new(),
            quotes: Vec::new(),
            approvals: Vec::new(),
            checklists: Vec::new(),
            checklist_items: Vec::new(),
            check_points: Vec::new(),
            worker_permits: Vec::new(),
            ppe_issues: Vec::new(),
            work_permits: Vec::new(),
            quality_open: None,
            request_open: None,
            purchase_budgets: Vec::new(),
            warehouse_filter: None,
            requests: Vec::new(),
            purchases: Vec::new(),
            blocks: Vec::new(),
            units: Vec::new(),
            deals: Vec::new(),
            payments: Vec::new(),
            workers: Vec::new(),
            timesheet: Vec::new(),
            quality: Vec::new(),
            safety: Vec::new(),
            machines: Vec::new(),
            machine_logs: Vec::new(),
            notices: Vec::new(),
            notices_rev: 0,
            inspections: Vec::new(),
            tools: Vec::new(),
            lab_tests: Vec::new(),
            zones: Vec::new(),
            bookings: Vec::new(),
            repairs: Vec::new(),
            tool_issues: Vec::new(),
            contracts: Vec::new(),
            contract_changes: Vec::new(),
            payment_stages: Vec::new(),
            acceptances: Vec::new(),
            concrete_tests: Vec::new(),
            geodesy_points: Vec::new(),
            timesheet_week: None,
            llm: crate::llm::Config::default(),
            sync: crate::sync::Config::default(),
            hook: crate::hook::Config::default(),
            hook_pending: None,
            hook_status: None,
            sync_pending: None,
            sync_status: None,
            copilot_intent: None,
            report_kind: 0,
            // Sukut bo'yicha oy: kunlik hisobot juda tor, yillik juda keng.
            report_preset: 2,
            journal_focus: None,
            work_schedule: checks::WorkSchedule::default(),
            machine_checks: Vec::new(),
            notes: Vec::new(),
            attachments: Vec::new(),
            llm_chat: Vec::new(),
            llm_pending: None,
            llm_error: None,
            llm_tokens: 0,
            users: Vec::new(),
            current_user: None,
            sales_block: None,
            selected_unit: None,
            selected_deal: None,
            selected_ppr: None,
            elements: Vec::new(),
            element_links: Vec::new(),
            estimates: Vec::new(),
            estimate_items: Vec::new(),
            current_estimate: None,
            norms: HashMap::new(),
            selected_issue: None,
            selected_element: None,
            search_open: false,
            search_query: String::new(),
            search_focus: false,
            auto_check_project: false,
            auto_check_estimate: false,
            auto_check_ppr: false,
            check_tab: CheckTab::Issues,
            issue_sev: None,
            issue_status: None,
            issue_search: String::new(),
            toast: None,
            confirm_delete_task: None,
            confirm_delete_project: None,
            settings,
            restyle: true,
        };
        app.reload_projects();
        if app.current.is_none() {
            // Oxirgi tanlangan obyekt tiklanadi: bir necha obyektli bazada
            // har ochilganda alifbo bo'yicha birinchisiga tushib qolish
            // noqulay. Obyekt o'chirilgan bo'lsa — birinchisi.
            let last = app
                .db
                .get_setting("last_project")
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|id| app.projects.iter().any(|p| p.id == *id));
            if let Some(p) = last.or_else(|| app.projects.first().map(|p| p.id)) {
                app.select_project(p);
            }
        }
        // Til modeli sozlamasi. Yig'ilishda tarmoq qismi bo'lmasa — doim o'chiq.
        // Bo'sh sozlama bazadan kelsa — tayyor qiymat ishlatiladi: manzil va
        // model oldindan to'g'ri turgani ma'qul, foydalanuvchi faqat kalitni
        // kiritsin.
        let or_default = |key: &str, fallback: &str| -> String {
            app.db
                .get_setting(key)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        };
        app.llm = crate::llm::Config {
            enabled: cfg!(feature = "llm")
                && app.db.get_setting("llm_enabled").as_deref() == Some("1"),
            endpoint: or_default("llm_endpoint", crate::llm::DEFAULT_ENDPOINT),
            model: or_default("llm_model", crate::llm::DEFAULT_MODEL),
            api_key: app.db.get_setting("llm_key").unwrap_or_default(),
            timeout_secs: app
                .db
                .get_setting("llm_timeout")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(crate::llm::DEFAULT_TIMEOUT),
        };

        // Sinxronizatsiya sozlamasi. Parol saqlanmaydi — faqat seans belgisi.
        app.sync = crate::sync::Config {
            enabled: cfg!(feature = "sync")
                && app.db.get_setting("sync_enabled").as_deref() == Some("1"),
            url: app.db.get_setting("sync_url").unwrap_or_default(),
            login: app.db.get_setting("sync_login").unwrap_or_default(),
            token: app.db.get_setting("sync_token").unwrap_or_default(),
            last_pull: app
                .db
                .get_setting("sync_last")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0),
        };

        // Narxlar bazasi obyektdan mustaqil: bir marta o'qiladi.
        app.price_book = app.db.price_book();

        // Tashqi xabar nuqtasi: manzil kiritilib, belgilanmaguncha
        // ilova hech qayerga ulanmaydi.
        app.hook = Self::load_hook(&app.db);

        // Rollar: oxirgi tanlangan foydalanuvchi tiklanadi.
        app.reload_users();
        app.current_user = app
            .db
            .get_setting("current_user")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|id| app.users.iter().any(|u| u.id == *id));
        app.sync_audit_user();
        // Jurnal cheksiz o'smasin: ochilishda eng eskilari olib tashlanadi.
        app.db.trim_audit_log(AUDIT_KEEP);
        app
    }

    pub fn project(&self) -> Option<&Project> {
        let id = self.current?;
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn origin(&self) -> NaiveDate {
        self.project()
            .map(|p| p.start_date)
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    pub fn reload_projects(&mut self) {
        match self.db.projects() {
            Ok(v) => self.projects = v,
            Err(e) => self.notify(format!("{}: {e}", t("err_read_objects"))),
        }
        if let Some(id) = self.current {
            if !self.projects.iter().any(|p| p.id == id) {
                self.current = self.projects.first().map(|p| p.id);
                self.reload_project_data();
            }
        }
    }

    pub fn select_project(&mut self, id: i64) {
        self.current = Some(id);
        let _ = self.db.set_setting("last_project", &id.to_string());
        self.selected_task = None;
        self.linking_from = None;
        self.row_offset = 0.0;
        self.selected_issue = None;
        self.selected_element = None;
        self.current_estimate = None;
        self.auto_check_project = false;
        self.auto_check_estimate = false;
        self.auto_check_ppr = false;
        self.reload_project_data();
        // Butun grafikni ko'rsatamiz: obyekt ochilganda bajarilgan ishlar ham,
        // oldindagilari ham bir qarashda ko'rinishi kerak.
        self.fit_timeline = true;
    }

    /// Bugungi kunni ko'rinadigan joyga suradi — chekkaga emas, chapdan
    /// bir oz ichkariga, shunda o'tgan ishlar ham ko'rinib turadi.
    pub fn scroll_to_today(&mut self) {
        let off = (self.today - self.origin()).num_days() as f32;
        self.timeline_offset = (off - 40.0).max(0.0);
    }

    pub fn reload_project_data(&mut self) {
        let Some(id) = self.current else {
            self.parties.clear();
            self.tasks.clear();
            self.links.clear();
            self.schedule = Schedule::default();
            self.progress = Progress::default();
            self.clear_modules();
            return;
        };
        self.parties = self.db.parties(id).unwrap_or_default();
        self.tasks = self.db.tasks(id).unwrap_or_default();
        self.links = self.db.links(id).unwrap_or_default();
        self.recompute();
        self.reload_modules();
    }

    pub fn clear_modules(&mut self) {
        self.issues.clear();
        self.machine_checks.clear();
        self.notes.clear();
        self.attachments.clear();
        self.documents.clear();
        self.ppr_docs.clear();
        self.exec_docs.clear();
        self.journal.clear();
        self.materials.clear();
        self.stock_moves.clear();
        self.warehouses.clear();
        self.batches.clear();
        self.reservations.clear();
        self.inventories.clear();
        self.inventory_lines.clear();
        self.material_norms.clear();
        self.material_alts.clear();
        self.brigades.clear();
        self.suppliers.clear();
        self.quotes.clear();
        self.approvals.clear();
        self.checklists.clear();
        self.checklist_items.clear();
        self.check_points.clear();
        self.worker_permits.clear();
        self.ppe_issues.clear();
        self.work_permits.clear();
        self.quality_open = None;
        self.request_open = None;
        self.purchase_budgets.clear();
        self.warehouse_filter = None;
        self.requests.clear();
        self.purchases.clear();
        self.blocks.clear();
        self.units.clear();
        self.deals.clear();
        self.payments.clear();
        self.workers.clear();
        self.timesheet.clear();
        self.quality.clear();
        self.safety.clear();
        self.machines.clear();
        self.machine_logs.clear();
        self.notices.clear();
        self.inspections.clear();
        self.tools.clear();
        self.lab_tests.clear();
        self.zones.clear();
        self.bookings.clear();
        self.repairs.clear();
        self.tool_issues.clear();
        self.contracts.clear();
        self.contract_changes.clear();
        self.payment_stages.clear();
        self.acceptances.clear();
        self.concrete_tests.clear();
        self.geodesy_points.clear();
        self.sales_block = None;
        self.selected_unit = None;
        self.selected_deal = None;
        self.elements.clear();
        self.element_links.clear();
        self.estimates.clear();
        self.estimate_items.clear();
        self.current_estimate = None;
    }

    /// II-III modullar ma'lumoti: nomuvofiqliklar, elementlar, smetalar, normativlar.
    pub fn reload_modules(&mut self) {
        let Some(id) = self.current else {
            self.clear_modules();
            return;
        };
        self.issues = self.db.issues(id);
        self.machine_checks = self.db.machine_checks(id);
        self.notes = self.db.notes(id);
        self.attachments = self.db.attachments(id);
        self.documents = self.db.documents(id);
        self.ppr_docs = self.db.ppr_docs(id);
        self.exec_docs = self.db.exec_docs(id);
        self.journal = self.db.journal(id);
        self.attendance = self.db.attendance(id);
        self.messages = self.db.messages(id);
        self.sign_log = self.db.sign_log(id);
        // Boshqa obyektning foto hisoboti qolib ketmasin.
        self.photo_report = None;
        self.materials = self.db.materials(id);
        self.stock_moves = self.db.stock_moves(id);
        self.warehouses = self.db.warehouses(id);
        self.batches = self.db.batches(id);
        self.reservations = self.db.reservations(id);
        self.inventories = self.db.inventories(id);
        self.inventory_lines = self.db.inventory_lines(id);
        self.material_norms = self.db.material_norms(id);
        self.material_alts = self.db.material_alts(id);
        self.brigades = self.db.brigades(id);
        self.suppliers = self.db.suppliers(id);
        self.quotes = self.db.quotes(id);
        self.approvals = self.db.approvals(id);
        self.checklists = self.db.checklists(id);
        self.checklist_items = self.db.checklist_items(id);
        self.check_points = self.db.check_points(id);
        self.worker_permits = self.db.worker_permits(id);
        self.ppe_issues = self.db.ppe_issues(id);
        self.work_permits = self.db.work_permits(id);
        self.purchase_budgets = self.db.purchase_budgets(id);
        self.requests = self.db.requests(id);
        self.purchases = self.db.purchases(id);
        self.workers = self.db.workers(id);
        self.timesheet = self.db.timesheet(id);
        self.quality = self.db.quality_checks(id);
        self.safety = self.db.safety_events(id);
        self.machines = self.db.machines(id);
        self.machine_logs = self.db.machine_logs(id);
        self.inspections = self.db.inspections(id);
        self.tools = self.db.tools(id);
        self.lab_tests = self.db.lab_tests(id);
        self.zones = self.db.safety_zones(id);
        self.bookings = self.db.machine_bookings(id);
        self.repairs = self.db.machine_repairs(id);
        self.tool_issues = self.db.tool_issues(id);
        self.contracts = self.db.contracts(id);
        self.contract_changes = self.db.contract_changes(id);
        self.payment_stages = self.db.payment_stages(id);
        self.acceptances = self.db.work_acceptances(id);
        self.concrete_tests = self.db.concrete_tests(id);
        self.geodesy_points = self.db.geodesy_points(id);
        self.blocks = self.db.blocks(id);
        self.units = self.db.units(id);
        self.deals = self.db.deals(id);
        self.payments = self.db.payments(id);
        if self.sales_block.is_none() {
            self.sales_block = self.blocks.first().map(|b| b.id);
        }
        self.elements = self.db.elements(id);
        self.element_links = self.db.element_links(id);
        // Kolliziya bir marta hisoblanadi: elementlar kamdan-kam
        // o'zgaradi, ekran esa soniyasiga o'nlab marta qayta chiziladi.
        self.clashes = crate::clash::find(&self.elements, &self.element_links);
        self.estimates = self.db.estimates(id);
        self.norms = self.db.norms();

        // Tanlangan smeta o'chirilgan bo'lsa yoki hali tanlanmagan bo'lsa — birinchisi.
        if !self
            .current_estimate
            .map(|e| self.estimates.iter().any(|x| x.id == e))
            .unwrap_or(false)
        {
            self.current_estimate = self.estimates.first().map(|e| e.id);
        }
        self.reload_estimate_items();

        if !self
            .selected_issue
            .map(|i| self.issues.iter().any(|x| x.id == i))
            .unwrap_or(false)
        {
            self.selected_issue = None;
        }

        // Bildirishnomalar barcha modul ma'lumotidan yig'iladi — shuning
        // uchun ular oxirida, hamma narsa yuklangandan keyin hisoblanadi.
        self.refresh_notices();
    }

    /// Bildirishnomalar ro'yxatini qayta yig'adi.
    ///
    /// Yozuv o'zgartirilgan joyda chaqiriladi: ro'yxat eskirib qolsa, yon
    /// paneldagi son haqiqatdan orqada qoladi.
    pub fn refresh_notices(&mut self) {
        self.notices = crate::notify::collect(self);
        self.notices_rev = self.db.revision();
    }

    pub fn reload_estimate_items(&mut self) {
        self.estimate_items = match self.current_estimate {
            Some(eid) => self.db.estimate_items(eid),
            None => Vec::new(),
        };
    }

    pub fn estimate(&self) -> Option<&Estimate> {
        let id = self.current_estimate?;
        self.estimates.iter().find(|e| e.id == id)
    }

    /// Tekshiruv konteksti. Ma'lumot faqat o'qiladi, shuning uchun havolalar bilan.
    fn check_ctx(&self) -> checks::Ctx<'_> {
        checks::Ctx {
            project_id: self.current.unwrap_or(0),
            tasks: &self.tasks,
            elements: &self.elements,
            links: &self.element_links,
            items: &self.estimate_items,
            declared_total: self.estimate().map(|e| e.declared_total).unwrap_or(0.0),
            norms: &self.norms,
            prices: &self.price_book,
        }
    }

    /// TZ II: loyihani element grafi bo'yicha tekshirish.
    pub fn run_project_check(&mut self) {
        let Some(pid) = self.current else { return };
        if self.elements.is_empty() {
            self.notify(t("no_elements").to_string());
            return;
        }
        let found = checks::check_project(&self.check_ctx());
        let n = found.len();
        self.db
            .replace_auto_issues(pid, IssueModule::Project, &found);
        self.reload_modules();
        self.notify(if n == 0 {
            t("no_issues").to_string()
        } else {
            format!("{n} {}", t("issues_found"))
        });
    }

    /// TZ III: smetani tekshirish.
    pub fn run_estimate_check(&mut self) {
        let Some(pid) = self.current else { return };
        if self.estimate_items.is_empty() {
            self.notify(t("no_estimates").to_string());
            return;
        }
        let found = checks::check_estimate(&self.check_ctx());
        let n = found.len();
        self.db
            .replace_auto_issues(pid, IssueModule::Estimate, &found);
        self.reload_modules();
        self.notify(if n == 0 {
            t("no_issues").to_string()
        } else {
            format!("{n} {}", t("issues_found"))
        });
    }

    /// Joriy rol. Foydalanuvchi tanlanmagan bo'lsa — to'liq huquq:
    /// bitta odam ishlayotgan ilovada cheklov ma'nosiz.
    pub fn role(&self) -> crate::roles::Role {
        self.current_user
            .and_then(|id| self.users.iter().find(|u| u.id == id))
            .map(|u| u.role)
            .unwrap_or(crate::roles::Role::Admin)
    }

    /// Joriy foydalanuvchi ismi. Tanlanmagan bo'lsa — bo'sh satr:
    /// kimdir deb o'ylab topmaymiz.
    /// Hozirgi vaqt yozuvi — izoh va biriktirmalar uchun.
    ///
    /// Sana emas, **vaqt** ham kerak: bir kunda bir necha izoh bo'lishi
    /// mumkin va ular tartibi ko'rinib turishi lozim.
    pub fn stamp(&self) -> String {
        chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn current_user_name(&self) -> String {
        self.current_user
            .and_then(|id| self.users.iter().find(|u| u.id == id))
            .map(|u| u.name.clone())
            .unwrap_or_default()
    }

    /// Joriy rol shu ekranni o'zgartira oladimi.
    pub fn can_edit(&self, screen: Screen) -> bool {
        self.role().can_edit(screen)
    }

    /// Joriy ekranni o'zgartirish mumkinmi — yozuvchi amallar shu orqali tekshiriladi.
    fn writable(&mut self) -> bool {
        if self.can_edit(self.screen) {
            return true;
        }
        self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
        false
    }

    /// Til modeli sozlamasini saqlaydi.
    ///
    /// Kalit bazada saqlanadi va boshqa hech qayerga chiqmaydi: hisobotga ham,
    /// logga ham tushmaydi. Zaxira nusxada esa u ham bo'ladi — bu sozlamalarda
    /// ochiq yozilgan.
    pub fn save_llm(&mut self) {
        let db = &self.db;
        let _ = db.set_setting("llm_enabled", if self.llm.enabled { "1" } else { "0" });
        let _ = db.set_setting("llm_endpoint", &self.llm.endpoint);
        let _ = db.set_setting("llm_model", &self.llm.model);
        let _ = db.set_setting("llm_key", &self.llm.api_key);
        let _ = db.set_setting("llm_timeout", &self.llm.timeout_secs.to_string());
    }

    pub fn reload_users(&mut self) {
        self.users = self.db.users();
        // Tanlangan foydalanuvchi o'chirilgan bo'lsa — tanlovni bo'shatamiz.
        if self
            .current_user
            .is_some_and(|id| !self.users.iter().any(|u| u.id == id))
        {
            self.current_user = None;
        }
    }

    /// Foydalanuvchini almashtirish: rolning uy ekrani ochiladi.
    pub fn set_user(&mut self, id: Option<i64>) {
        self.current_user = id;
        let _ = self.db.set_setting(
            "current_user",
            &id.map(|v| v.to_string()).unwrap_or_default(),
        );
        self.sync_audit_user();
        self.screen = self.role().home();
    }

    /// Joriy foydalanuvchi nomi. Tanlanmagan bo'lsa bo'sh satr — hujjatga
    /// «Administrator» deb yozib qo'yish yolg'on bo'lardi.
    pub fn user_name(&self) -> String {
        self.current_user
            .and_then(|id| self.users.iter().find(|u| u.id == id))
            .map(|u| u.name.clone())
            .unwrap_or_default()
    }

    /// Amallar tarixiga yoziladigan nomni yangilaydi.
    ///
    /// Foydalanuvchi tanlanmagan bo'lsa jurnalda bo'sh qoladi — «Administrator»
    /// deb yozib qo'yish yolg'on bo'lardi, chunki hech kim tanlanmagan.
    pub fn sync_audit_user(&self) {
        let name = self
            .current_user
            .and_then(|id| self.users.iter().find(|u| u.id == id))
            .map(|u| format!("{} · {}", u.name, u.role.label()))
            .unwrap_or_default();
        self.db.set_audit_user(&name);
    }

    // ---------- Loyiha paketi (qurilmalar orasida almashish) ----------

    /// Maydonchada to'ldiriladigan ma'lumotni paketga chiqaradi.
    ///
    /// Paketga faqat **kunlik ijro** kiradi: jurnal, tabel, texnika smenalari,
    /// sifat va xavfsizlik yozuvlari. Grafik, smeta va shartnomalar chiqmaydi —
    /// ular ofisda yuritiladi va ikki tomondan tahrirlansa ziddiyat tug'iladi.
    pub fn export_package(&self) -> crate::package::Package {
        use crate::package::{Package, Table};

        let name = |id: Option<i64>| id.map(|t| self.task_name(t)).unwrap_or_default();

        let journal = Table {
            name: "journal".into(),
            columns: vec![
                "date".into(),
                "author".into(),
                "weather".into(),
                "temperature".into(),
                "workers".into(),
                "machines".into(),
                "task".into(),
                "volume".into(),
                "unit".into(),
                "text".into(),
                "remarks".into(),
                "gps".into(),
            ],
            rows: self
                .journal
                .iter()
                .map(|j| {
                    vec![
                        j.date.to_string(),
                        j.author.clone(),
                        j.weather.clone(),
                        j.temperature.to_string(),
                        j.workers.to_string(),
                        j.machines.to_string(),
                        name(j.task_id),
                        j.volume.to_string(),
                        j.unit.clone(),
                        j.text.clone(),
                        j.remarks.clone(),
                        j.gps.clone(),
                    ]
                })
                .collect(),
        };

        // Kun turi va smena ham ko'chadi: maydonchada belgilangan bo'sh turish
        // yoki yo'qlik ofisdagi bazada ham shunday qolishi kerak.
        let timesheet = Table {
            name: "timesheet".into(),
            columns: vec![
                "date".into(),
                "worker".into(),
                "hours".into(),
                "kind".into(),
                "shift".into(),
            ],
            rows: self
                .timesheet
                .iter()
                .map(|e| {
                    let worker = self
                        .workers
                        .iter()
                        .find(|w| w.id == e.worker_id)
                        .map(|w| w.name.clone())
                        .unwrap_or_default();
                    vec![
                        e.date.to_string(),
                        worker,
                        e.hours.to_string(),
                        e.kind.code().into(),
                        e.shift.code().into(),
                    ]
                })
                .collect(),
        };

        let machine_logs = Table {
            name: "machine_log".into(),
            columns: vec![
                "date".into(),
                "machine".into(),
                "hours".into(),
                "fuel".into(),
                "task".into(),
                "driver".into(),
                "note".into(),
                "gps".into(),
            ],
            rows: self
                .machine_logs
                .iter()
                .map(|l| {
                    let machine = self
                        .machines
                        .iter()
                        .find(|m| m.id == l.machine_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_default();
                    vec![
                        l.date.to_string(),
                        machine,
                        l.hours.to_string(),
                        l.fuel.to_string(),
                        name(l.task_id),
                        l.driver.clone(),
                        l.note.clone(),
                        l.gps.clone(),
                    ]
                })
                .collect(),
        };

        let quality = Table {
            name: "quality".into(),
            columns: vec![
                "date".into(),
                "kind".into(),
                "subject".into(),
                "inspector".into(),
                "result".into(),
                "defect".into(),
                "task".into(),
            ],
            rows: self
                .quality
                .iter()
                .map(|q| {
                    vec![
                        q.date.to_string(),
                        q.kind.code().into(),
                        q.subject.clone(),
                        q.inspector.clone(),
                        q.result.code().into(),
                        q.defect.clone(),
                        name(q.task_id),
                    ]
                })
                .collect(),
        };

        let safety = Table {
            name: "safety".into(),
            columns: vec![
                "date".into(),
                "kind".into(),
                "severity".into(),
                "place".into(),
                "description".into(),
                "measure".into(),
                "responsible".into(),
                "status".into(),
            ],
            rows: self
                .safety
                .iter()
                .map(|s| {
                    vec![
                        s.date.to_string(),
                        s.kind.code().into(),
                        s.severity.code().into(),
                        s.place.clone(),
                        s.description.clone(),
                        s.measure.clone(),
                        s.responsible.clone(),
                        s.status.code().into(),
                    ]
                })
                .collect(),
        };

        Package {
            version: crate::package::VERSION.into(),
            project: self.project().map(|p| p.name.clone()).unwrap_or_default(),
            created: self.today.to_string(),
            tables: vec![journal, timesheet, machine_logs, quality, safety],
        }
    }

    /// Paketdagi yozuvlarni bazaga qo'shadi. Mavjudlari qayta yozilmaydi.
    ///
    /// Natija: (qo'shilgan, mavjud bo'lgani). Ishchi yoki texnika topilmasa —
    /// yangisi yaratiladi: maydonchada kiritilgan yozuv yo'qolib qolmasin.
    pub fn import_package(&mut self, pkg: &crate::package::Package) -> (usize, usize) {
        use crate::domain::{
            JournalEntry, Machine, MachineKind, MachineLog, MachineStatus, QualityCheck,
            QualityKind, QualityResult, SafetyEvent, SafetyKind, Severity, Worker,
        };
        use crate::package::row_map;

        let Some(pid) = self.current else {
            return (0, 0);
        };
        let (mut added, mut existing) = (0usize, 0usize);
        let date = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or(self.today);
        let num = |s: &str| s.parse::<f64>().unwrap_or(0.0);
        let int = |s: &str| s.parse::<i64>().unwrap_or(0);

        // ---- Jurnal: sana va matn bo'yicha takrorlanmaydi.
        if let Some(t) = pkg.table("journal") {
            for row in &t.rows {
                let m = row_map(t, row);
                let d = date(m.get("date").copied().unwrap_or(""));
                let text = m.get("text").copied().unwrap_or("").to_string();
                if self.journal.iter().any(|j| j.date == d && j.text == text) {
                    existing += 1;
                    continue;
                }
                let task_id = self.task_id_by_name(m.get("task").copied().unwrap_or(""));
                self.db.insert_journal(&JournalEntry {
                    id: 0,
                    project_id: pid,
                    date: d,
                    author: m.get("author").copied().unwrap_or("").into(),
                    weather: m.get("weather").copied().unwrap_or("").into(),
                    temperature: num(m.get("temperature").copied().unwrap_or("")),
                    workers: int(m.get("workers").copied().unwrap_or("")),
                    machines: int(m.get("machines").copied().unwrap_or("")),
                    task_id,
                    volume: num(m.get("volume").copied().unwrap_or("")),
                    unit: m.get("unit").copied().unwrap_or("").into(),
                    text,
                    remarks: m.get("remarks").copied().unwrap_or("").into(),
                    photos: String::new(),
                    // Telefondan kelgan yozuvda koordinata bo'lishi mumkin;
                    // eski paketda bu ustun yo'q — bo'sh qoladi.
                    gps: m.get("gps").copied().unwrap_or("").into(),
                });
                added += 1;
            }
        }

        // ---- Kirish/chiqish: telefondan keladi, ilovada yaratilmaydi.
        if let Some(t) = pkg.table("attendance") {
            for row in &t.rows {
                let m = row_map(t, row);
                let who = m.get("worker").copied().unwrap_or("").trim().to_string();
                if who.is_empty() {
                    continue;
                }
                let at = crate::store::parse_stamp(m.get("at").copied().unwrap_or(""));
                let kind = crate::domain::InOut::parse(m.get("kind").copied().unwrap_or("in"));
                // Bir xil belgi ikki marta kelishi mumkin: baza uni o'zi
                // rad etadi, biz esa «qo'shildi» deb sanamaymiz.
                let already = self
                    .attendance
                    .iter()
                    .any(|a| a.worker == who && a.at == at && a.kind == kind);
                if already {
                    existing += 1;
                    continue;
                }
                let id = self.db.insert_attendance(&crate::domain::Attendance {
                    id: 0,
                    project_id: pid,
                    worker: who,
                    at,
                    kind,
                    gps: m.get("gps").copied().unwrap_or("").into(),
                    source: m.get("source").copied().unwrap_or("").into(),
                });
                if id > 0 {
                    added += 1;
                } else {
                    existing += 1;
                }
            }
        }

        // ---- Ariza: maydonchadan keladi (TZ IX.2, X.29).
        //
        // Raqam **shu yerda** beriladi: bir necha telefon bir vaqtda
        // yuborsa, serverda berilgan raqam takrorlanib qolardi.
        if let Some(t) = pkg.table("request") {
            for row in &t.rows {
                let m = row_map(t, row);
                let title = m.get("title").copied().unwrap_or("").trim().to_string();
                if title.is_empty() {
                    continue;
                }
                let d = date(m.get("date").copied().unwrap_or(""));
                if self
                    .requests
                    .iter()
                    .any(|q| q.date == d && q.title == title)
                {
                    existing += 1;
                    continue;
                }
                let need = m.get("need_date").copied().unwrap_or("");
                let number = format!("Z-{:03}", self.requests.len() + added + 1);
                self.db.insert_request(&crate::domain::Request {
                    id: 0,
                    project_id: pid,
                    number,
                    date: d,
                    kind: crate::domain::RequestKind::parse(
                        m.get("kind").copied().unwrap_or("material"),
                    ),
                    title,
                    material_id: None,
                    qty: num(m.get("qty").copied().unwrap_or("")),
                    unit: m.get("unit").copied().unwrap_or("").into(),
                    requester: m.get("requester").copied().unwrap_or("").into(),
                    need_date: if need.trim().is_empty() {
                        d + chrono::Duration::days(14)
                    } else {
                        date(need)
                    },
                    priority: crate::domain::Priority::Normal,
                    // Tasdiqlash ofisda qoladi: maydonchadan kelgan ariza
                    // darrov xaridga o'tmaydi.
                    status: crate::domain::RequestStatus::New,
                    task_id: None,
                    reject_reason: String::new(),
                    note: m.get("note").copied().unwrap_or("").into(),
                });
                added += 1;
            }
        }

        // ---- Tabel: bir ishchining bir kuni bitta bo'ladi.
        if let Some(t) = pkg.table("timesheet") {
            for row in &t.rows {
                let m = row_map(t, row);
                let d = date(m.get("date").copied().unwrap_or(""));
                let who = m.get("worker").copied().unwrap_or("").trim().to_string();
                if who.is_empty() {
                    continue;
                }
                let wid = match self.workers.iter().find(|w| w.name == who) {
                    Some(w) => w.id,
                    None => {
                        let id = self.db.insert_worker(&Worker {
                            id: 0,
                            project_id: pid,
                            name: who.clone(),
                            position: String::new(),
                            org: String::new(),
                            hourly_rate: 0.0,
                            active: true,
                            brigade_id: None,
                        });
                        self.workers = self.db.workers(pid);
                        id
                    }
                };
                if self
                    .timesheet
                    .iter()
                    .any(|e| e.worker_id == wid && e.date == d)
                {
                    existing += 1;
                    continue;
                }
                // Kun turi avval yoziladi: nol soatli yo'qlik kuni ham
                // saqlanib qolsin (aks holda yozuv o'chib ketardi).
                let kind = DayKind::parse(m.get("kind").copied().unwrap_or("work"));
                if kind != DayKind::Work {
                    self.db.set_timesheet_kind(pid, wid, d, kind);
                }
                let shift = Shift::parse(m.get("shift").copied().unwrap_or("day"));
                if shift != Shift::Day {
                    self.db.set_timesheet_shift(pid, wid, d, shift);
                }
                self.db
                    .set_timesheet(pid, wid, d, num(m.get("hours").copied().unwrap_or("")));
                // Nol soatli oddiy ish kuni bazada yozuv qoldirmaydi — uni
                // qo'shilgan deb sanamaymiz, aks holda har importda takrorlanardi.
                if kind != DayKind::Work || num(m.get("hours").copied().unwrap_or("")) > 0.0 {
                    added += 1;
                }
                self.timesheet = self.db.timesheet(pid);
            }
        }

        // ---- Texnika smenalari.
        if let Some(t) = pkg.table("machine_log") {
            for row in &t.rows {
                let m = row_map(t, row);
                let d = date(m.get("date").copied().unwrap_or(""));
                let who = m.get("machine").copied().unwrap_or("").trim().to_string();
                if who.is_empty() {
                    continue;
                }
                let mid = match self.machines.iter().find(|x| x.name == who) {
                    Some(x) => x.id,
                    None => {
                        let id = self.db.insert_machine(&Machine {
                            id: 0,
                            project_id: pid,
                            name: who.clone(),
                            kind: MachineKind::Other,
                            reg_no: String::new(),
                            owner: String::new(),
                            status: MachineStatus::Idle,
                            hour_rate: 0.0,
                            operator: String::new(),
                            inspection_until: None,
                            fuel_norm: 0.0,
                            service_hours: 0.0,
                            service_done: 0.0,
                            rented: false,
                            price: 0.0,
                        });
                        self.machines = self.db.machines(pid);
                        id
                    }
                };
                if self
                    .machine_logs
                    .iter()
                    .any(|l| l.machine_id == mid && l.date == d)
                {
                    existing += 1;
                    continue;
                }
                self.db.insert_machine_log(&MachineLog {
                    id: 0,
                    project_id: pid,
                    machine_id: mid,
                    date: d,
                    hours: num(m.get("hours").copied().unwrap_or("")),
                    fuel: num(m.get("fuel").copied().unwrap_or("")),
                    task_id: self.task_id_by_name(m.get("task").copied().unwrap_or("")),
                    number: String::new(),
                    driver: m.get("driver").copied().unwrap_or("").into(),
                    route: String::new(),
                    odo_start: 0.0,
                    odo_end: 0.0,
                    trips: 0,
                    cargo: 0.0,
                    note: m.get("note").copied().unwrap_or("").into(),
                    // Telefondan kelgan smenada koordinata bo'ladi;
                    // ilovaning o'z paketida u bo'sh.
                    gps: m.get("gps").copied().unwrap_or("").into(),
                });
                added += 1;
            }
        }

        // ---- Sifat nazorati.
        if let Some(t) = pkg.table("quality") {
            for row in &t.rows {
                let m = row_map(t, row);
                let d = date(m.get("date").copied().unwrap_or(""));
                let subject = m.get("subject").copied().unwrap_or("").to_string();
                if self
                    .quality
                    .iter()
                    .any(|q| q.date == d && q.subject == subject)
                {
                    existing += 1;
                    continue;
                }
                self.db.insert_quality(&QualityCheck {
                    id: 0,
                    project_id: pid,
                    kind: QualityKind::parse(m.get("kind").copied().unwrap_or("")),
                    date: d,
                    task_id: self.task_id_by_name(m.get("task").copied().unwrap_or("")),
                    material_id: None,
                    subject,
                    inspector: m.get("inspector").copied().unwrap_or("").into(),
                    result: QualityResult::parse(m.get("result").copied().unwrap_or("")),
                    defect: m.get("defect").copied().unwrap_or("").into(),
                    deadline: None,
                    checklist_id: None,
                    fixed_at: None,
                    note: String::new(),
                });
                added += 1;
            }
        }

        // ---- Xavfsizlik.
        if let Some(t) = pkg.table("safety") {
            for row in &t.rows {
                let m = row_map(t, row);
                let d = date(m.get("date").copied().unwrap_or(""));
                let description = m.get("description").copied().unwrap_or("").to_string();
                if self
                    .safety
                    .iter()
                    .any(|s| s.date == d && s.description == description)
                {
                    existing += 1;
                    continue;
                }
                self.db.insert_safety(&SafetyEvent {
                    id: 0,
                    project_id: pid,
                    date: d,
                    kind: SafetyKind::parse(m.get("kind").copied().unwrap_or("")),
                    severity: Severity::parse(m.get("severity").copied().unwrap_or("")),
                    place: m.get("place").copied().unwrap_or("").into(),
                    description,
                    responsible: m.get("responsible").copied().unwrap_or("").into(),
                    measure: m.get("measure").copied().unwrap_or("").into(),
                    deadline: None,
                    root_cause: crate::domain::RootCause::Unknown,
                    status: crate::domain::IssueStatus::parse(
                        m.get("status").copied().unwrap_or(""),
                    ),
                });
                added += 1;
            }
        }

        self.reload_modules();
        (added, existing)
    }

    /// Ish nomidan uning identifikatorini topadi (paketda nom ko'chadi, id emas).
    fn task_id_by_name(&self, name: &str) -> Option<i64> {
        let n = name.trim();
        if n.is_empty() {
            return None;
        }
        self.tasks.iter().find(|t| t.name == n).map(|t| t.id)
    }

    /// TZ II.1–2: IFC faylini o'qib, bilimlar grafiga qo'shadi.
    ///
    /// Mavjud elementlar o'chirilmaydi — import qo'shimcha qiladi. Bir xil
    /// marka ikki marta tushmasligi uchun IFC dan kelgan va o'sha markali
    /// element allaqachon bo'lsa, u qayta yozilmaydi.
    pub fn import_ifc(&mut self, path: &std::path::Path) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::AiCheck) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let Some(pid) = self.current else { return };
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            // IFC odatda UTF-8, ammo eski fayllar boshqa kodlashda bo'lishi mumkin.
            Err(_) => match std::fs::read(path) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(e) => {
                    self.notify(format!("{}: {e}", t("ifc_failed")));
                    return;
                }
            },
        };

        let model = crate::ifc::parse(&src);
        if model.entities.is_empty() {
            self.notify(t("ifc_empty").to_string());
            return;
        }
        let graph = crate::ifc::to_graph(&model, pid);
        if graph.elements.is_empty() {
            self.notify(t("ifc_no_elements").to_string());
            return;
        }

        // IFC dagi o'rin -> bazadagi identifikator.
        let mut ids: Vec<Option<i64>> = Vec::with_capacity(graph.elements.len());
        let mut added = 0usize;
        let mut skipped = 0usize;
        for e in &graph.elements {
            let existing = self
                .elements
                .iter()
                .find(|x| x.sheet == "IFC" && !e.mark.is_empty() && x.mark == e.mark)
                .map(|x| x.id);
            match existing {
                Some(id) => {
                    ids.push(Some(id));
                    skipped += 1;
                }
                None => {
                    let id = self.db.insert_element(e);
                    ids.push((id > 0).then_some(id));
                    if id > 0 {
                        added += 1;
                    }
                }
            }
        }

        let mut links = 0usize;
        for (from, to, relation) in &graph.links {
            if let (Some(Some(a)), Some(Some(b))) = (ids.get(*from), ids.get(*to)) {
                if self.db.insert_element_link(&ElementLink {
                    id: 0,
                    from_el: *a,
                    to_el: *b,
                    relation: *relation,
                }) > 0
                {
                    links += 1;
                }
            }
        }

        self.reload_modules();
        self.notify(format!(
            "{} {added} · {} {links} · {} {skipped}",
            t("ifc_added"),
            t("ifc_links"),
            t("ifc_existing")
        ));
    }

    /// DXF chizmasini o'qib, elementlarni bazaga qo'shadi (TZ II.1–2).
    ///
    /// IFC bilan bir xil qoida: allaqachon bor marka takrorlanmaydi, yangi
    /// element esa varaq nomi bilan yoziladi. Bog'lanishlar chizmadan
    /// olinmaydi — chizmada ular yozilmagan bo'ladi.
    pub fn import_dxf(&mut self, path: &std::path::Path) {
        if !self.can_edit(Screen::AiCheck) {
            self.notify(t("role_readonly").to_string());
            return;
        }
        let Some(pid) = self.current else { return };

        let src = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                self.notify(format!("{}: {e}", t("dxf_failed")));
                return;
            }
        };
        let drawing = crate::dxf::parse(&src);
        if drawing.entries.is_empty() {
            self.notify(t("dxf_empty").to_string());
            return;
        }

        // Varaq nomi — fayl nomi: element qaysi chizmadan kelgani ko'rinadi.
        let sheet = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "DXF".to_string());
        let elements = crate::dxf::to_elements(&drawing, pid, &sheet);

        let mut added = 0usize;
        let mut existing = 0usize;
        for e in &elements {
            let same = self.elements.iter().any(|x| {
                x.sheet == e.sheet
                    && ((!e.mark.is_empty() && x.mark == e.mark)
                        || (!e.room.is_empty() && x.room == e.room))
            });
            if same {
                existing += 1;
                continue;
            }
            if self.db.insert_element(e) > 0 {
                added += 1;
            }
        }

        self.reload_modules();
        self.notify(format!(
            "{} {added} · {} {existing} · {} {}",
            t("ifc_added"),
            t("ifc_existing"),
            t("dxf_geometry"),
            drawing.skipped
        ));
    }

    /// Sinxronizatsiya sozlamasini saqlaydi.
    pub fn save_sync(&self) {
        let _ = self
            .db
            .set_setting("sync_enabled", if self.sync.enabled { "1" } else { "0" });
        let _ = self.db.set_setting("sync_url", self.sync.url.trim());
        let _ = self.db.set_setting("sync_login", self.sync.login.trim());
        let _ = self.db.set_setting("sync_token", &self.sync.token);
        let _ = self
            .db
            .set_setting("sync_last", &self.sync.last_pull.to_string());
    }

    /// Serverga kiradi va seans belgisini saqlaydi.
    ///
    /// Parol hech qayerda saqlanmaydi: u faqat shu so'rovda ketadi.
    #[cfg(feature = "sync")]
    pub fn sync_login(&mut self, password: &str) {
        let url = self.sync.url.trim().to_string();
        let login = self.sync.login.trim().to_string();
        if url.is_empty() || login.is_empty() {
            self.sync_status = Some((t("sync_err_config").to_string(), true));
            return;
        }
        match crate::sync::login(&url, &login, password) {
            Ok(session) => {
                self.sync.token = session.token;
                self.sync.enabled = true;
                self.save_sync();
                self.sync_status = Some((
                    format!(
                        "{} {} · {}",
                        t("sync_signed_in"),
                        session.name,
                        session.role
                    ),
                    false,
                ));
            }
            Err(e) => {
                self.sync.token.clear();
                self.save_sync();
                self.sync_status = Some((t(e.key()).to_string(), true));
            }
        }
    }

    /// Serverdan chiqadi: belgi o'chiriladi.
    pub fn sync_logout(&mut self) {
        self.sync.token.clear();
        self.sync.enabled = false;
        self.save_sync();
        self.sync_status = Some((t("sync_signed_out").to_string(), false));
    }

    /// Sinxronizatsiyani boshlaydi: o'z paketini yuboradi va yangilarini oladi.
    ///
    /// Ish fon oqimida bajariladi — interfeys qotib qolmaydi. Natija
    /// keyingi kadrda `poll_sync` orqali olinadi.
    pub fn sync_now(&mut self) {
        if self.sync_pending.is_some() {
            return;
        }
        if !self.sync.ready() {
            self.sync_status = Some((t("sync_err_config").to_string(), true));
            return;
        }
        let Some(project) = self.project().map(project_key) else {
            self.sync_status = Some((t("no_object_selected").to_string(), true));
            return;
        };
        // Yuboriladigan paket — shu obyektning maydon ma'lumotlari.
        let pkg = self.export_package();
        let rows = pkg.row_count() as i64;
        let body = crate::package::write(&pkg);
        // Signal ro'yxati ham yuboriladi: telefon uni qayta hisoblamaydi.
        let notices: Vec<crate::sync::NoticeOut> = self
            .notices
            .iter()
            .map(|n| crate::sync::NoticeOut {
                code: n.code,
                severity: n.severity.code(),
                title: n.title.clone(),
                detail: n.detail.clone(),
                count: n.count as i64,
                days: n.days,
                source: n.source.label(),
            })
            .collect();
        // Ishlar ro'yxati — telefonda qo'lda yozish o'rniga tanlash uchun.
        let tasks: Vec<crate::sync::TaskOut> = self
            .tasks
            .iter()
            .map(|t| {
                // Tugash sanasi hisobdan olinadi: u grafikda ham shu
                // yerdan chiqadi, ikkinchi hisob bo'lmasin.
                let end = self
                    .schedule
                    .get(t.id)
                    .map(|c| self.origin() + chrono::Duration::days(c.ef.max(0)))
                    .unwrap_or(t.plan_start);
                crate::sync::TaskOut {
                    wbs: t.wbs.clone(),
                    name: t.name.clone(),
                    start: t.plan_start.to_string(),
                    end: end.to_string(),
                    progress: t.progress,
                    section: t.section.code().to_string(),
                }
            })
            .collect();
        // Faol ishchilar — telefondagi tabel shu ro'yxat bo'yicha
        // to'ldiriladi.
        let workers: Vec<crate::sync::WorkerOut> = self
            .workers
            .iter()
            .filter(|w| w.active)
            .map(|w| crate::sync::WorkerOut {
                name: w.name.clone(),
                position: w.position.clone(),
            })
            .collect();
        // Obyekt yakuni — «Hisobotlar» dagi bilan bitta funksiyadan:
        // buyurtmachi kabinetidagi son ekrandagi son bilan bir xil.
        let period = crate::reports::Preset::All.period(
            self.today,
            self.project().map(|p| p.start_date).unwrap_or(self.today),
        );
        let table = crate::reports::build(self, crate::reports::Kind::Summary, period);
        let summary: Vec<crate::sync::SummaryOut> = table
            .rows
            .iter()
            .map(|r| crate::sync::SummaryOut {
                module: cell_text(r.first()),
                indicator: cell_text(r.get(1)),
                value: cell_text(r.get(2)),
            })
            .collect();

        let since = self
            .project()
            .map(|p| self.db.last_message_id(p.id))
            .unwrap_or(0);
        self.sync_pending = Some(crate::sync::spawn(
            self.sync.clone(),
            project,
            Some((body, rows)),
            crate::sync::Refs {
                notices,
                tasks,
                workers,
                summary,
                labels: self.label_refs(),
                machines: self
                    .machines
                    .iter()
                    .map(|m| crate::sync::MachineOut {
                        name: m.name.clone(),
                        reg_no: m.reg_no.clone(),
                    })
                    .collect(),
                chain: {
                    let (count, head) = crate::signlog::head(&self.sign_log);
                    (count as i64, head)
                },
            },
            crate::sync::Chat {
                outgoing: std::mem::take(&mut self.chat_outgoing),
                since,
            },
        ));
        self.sync_status = Some((t("sync_running").to_string(), false));
    }

    /// Fon oqimidagi natijani tekshiradi va kelgan paketlarni qo'llaydi.
    pub fn poll_sync(&mut self) {
        let Some(rx) = &self.sync_pending else {
            return;
        };
        let Ok(result) = rx.try_recv() else {
            return;
        };
        self.sync_pending = None;

        match result {
            Ok(outcome) => {
                let mut added = 0usize;
                let mut skipped = 0usize;
                for item in &outcome.pulled.items {
                    // O'zimiz yuborgan paketni qaytadan qo'llash shart emas,
                    // lekin zarar ham qilmaydi: import mavjud yozuvni
                    // takrorlamaydi.
                    match crate::package::read(&item.body) {
                        Ok(pkg) => {
                            let (a, s) = self.import_package(&pkg);
                            added += a;
                            skipped += s;
                        }
                        // Buzilgan paket jimgina tashlanmaydi.
                        Err(_) => skipped += 1,
                    }
                }
                // Yangi xabarlar saqlanadi: internet yo'q paytda ham
                // yozishma ochilsin.
                if let Some(pid) = self.project().map(|p| p.id) {
                    for m in &outcome.messages {
                        self.db.insert_message(&crate::domain::ChatMessage {
                            id: 0,
                            project_id: pid,
                            server_id: m.id,
                            author: m.author.clone(),
                            role: m.role.clone(),
                            text: m.text.clone(),
                            at: m.at.clone(),
                        });
                    }
                    if !outcome.messages.is_empty() {
                        self.messages = self.db.messages(pid);
                    }
                }
                // Xabar yuborilgan bo'lsa maydon bo'shaydi.
                self.chat_draft.clear();
                // Imzo zanjiri: serverdagi belgilar saqlanadi va
                // ziddiyat darrov aytiladi — bu jimgina o'tadigan narsa
                // emas.
                self.chain_marks = outcome.chain_marks.clone();
                if let Some(old) = &outcome.chain_conflict {
                    self.notify(format!(
                        "{}: {}",
                        t("sl_conflict"),
                        &old[..8.min(old.len())]
                    ));
                }
                if outcome.pulled.last > self.sync.last_pull {
                    self.sync.last_pull = outcome.pulled.last;
                    self.save_sync();
                }
                self.reload_modules();
                self.sync_status = Some((
                    format!(
                        "{} {} · {} {} · {} {}",
                        t("sync_sent"),
                        outcome.pushed,
                        t("sync_added"),
                        added,
                        t("sync_skipped"),
                        skipped
                    ),
                    false,
                ));
            }
            Err(e) => {
                // Yuborilmagan xabar maydonda qoladi: «yuborildi» deb
                // aldab qo'yilmaydi.
                if !self.chat_outgoing.trim().is_empty() && self.chat_draft.trim().is_empty() {
                    self.chat_draft = std::mem::take(&mut self.chat_outgoing);
                }
                // Seans tugagan bo'lsa, belgi foydasiz — qayta kirish kerak.
                if e == crate::sync::Error::Auth {
                    self.sync.token.clear();
                    self.save_sync();
                }
                // Tarmoq xatosi o'tkinchi bo'lishi mumkin — buni aytamiz.
                let hint = if e.retryable() {
                    format!(" ({})", t("sync_retry"))
                } else {
                    String::new()
                };
                self.sync_status = Some((format!("{}{hint}", t(e.key())), true));
            }
        }
    }

    /// Hujjatni serverda imzolaydi (TZ VIII.22).
    ///
    /// Imzo hujjat matniga bog'lanadi: keyin hujjat o'zgarsa, xesh mos
    /// kelmaydi va buni ko'rish mumkin. Bu davlat elektron raqamli imzosi
    /// emas va shunday deb atalmaydi — u kim, qachon va nimani
    /// tasdiqlaganini qayd etadi.
    #[cfg(feature = "sync")]
    pub fn sync_sign(&mut self, document: &str, text: &str, rejected: &str) {
        if !self.sync.ready() {
            self.sync_status = Some((t("sync_err_config").to_string(), true));
            return;
        }
        let Some(project) = self.project().map(project_key) else {
            self.sync_status = Some((t("no_object_selected").to_string(), true));
            return;
        };
        match crate::sync::sign(&self.sync, &project, document, text, rejected) {
            Ok(()) => {
                // Serverdagi imzo ilovadagi daftarga ham tushadi:
                // ikkalasi bir xil matndan bir xil xesh beradi, shuning
                // uchun ular keyin solishtiriladi.
                self.sign_document(document, text, text, rejected);
                self.sync_status = Some((format!("{} {document}", t("sync_signed")), false));
            }
            Err(e) => self.sync_status = Some((t(e.key()).to_string(), true)),
        }
    }

    /// Tarmoqsiz yig'ilishda masofadan imzolash yo'q.
    #[cfg(not(feature = "sync"))]
    pub fn sync_sign(&mut self, _document: &str, _text: &str, _rejected: &str) {
        self.sync_status = Some((t("sync_err_config").to_string(), true));
    }

    /// TZ XVII: kesishgan tahlil uchun barcha modullardan ma'lumot yig'adi.
    ///
    /// Hisoblar shu yerda emas, `analytics` da bajariladi — ekran va hisobot
    /// bir xil manbadan foydalanadi.
    pub fn analytics_input<'a>(
        &'a self,
        supply: &'a [checks::SupplyLine],
        stock: &'a [checks::StockLine],
        cost: &'a checks::CostSummary,
        sales: &'a crate::sales::SalesSummary,
    ) -> crate::analytics::Input<'a> {
        crate::analytics::Input {
            today: self.today,
            tasks: &self.tasks,
            schedule: &self.schedule,
            progress: &self.progress,
            issues: &self.issues,
            exec_docs: &self.exec_docs,
            requests: &self.requests,
            purchases: &self.purchases,
            supply,
            stock,
            stock_moves: &self.stock_moves,
            materials: &self.materials,
            quality: &self.quality,
            safety: &self.safety,
            machines: &self.machines,
            machine_logs: &self.machine_logs,
            workers: &self.workers,
            timesheet: &self.timesheet,
            units: &self.units,
            deals: &self.deals,
            payments: &self.payments,
            cost,
            sales,
            contract_sum: self.project().map(|p| p.contract_sum).unwrap_or(0.0),
            paid_total: self.project().map(|p| p.paid_total).unwrap_or(0.0),
        }
    }

    /// TZ XIX-XX: obyekt bo'yicha sotuv xulosasi.
    pub fn sales(&self) -> crate::sales::SalesSummary {
        crate::sales::sales_summary(&self.units, &self.deals, &self.payments, self.today)
    }

    /// Birlik holatini amaldagi shartnomaga moslaydi va bazaga yozadi.
    ///
    /// Shartnoma holati o'zgarganda shaxmatkadagi rang ham o'zgarishi kerak,
    /// aks holda «sotilgan» kvartira bo'sh bo'lib ko'rinib qoladi.
    pub fn sync_unit_status(&mut self, unit_id: i64) {
        let deal = crate::sales::active_deal(&self.deals, unit_id).cloned();
        let Some(want) = crate::sales::status_for(deal.as_ref()) else {
            return;
        };
        let Some(u) = self.units.iter_mut().find(|u| u.id == unit_id) else {
            return;
        };
        // «Sotuvda emas» qo'lda qo'yiladi — uni shartnoma bekor qilmaydi.
        if u.status == want || u.status == crate::domain::UnitStatus::Unavailable {
            return;
        }
        u.status = want;
        let copy = u.clone();
        self.db.update_unit(&copy);
    }

    /// TZ IX-X: ariza va xaridlarni solishtirgan ta'minot holati.
    pub fn supply(&self) -> Vec<checks::SupplyLine> {
        checks::supply_status(&self.requests, &self.purchases, self.today)
    }

    /// Ishlar bo'yicha tannarx (TZ XVII.12).
    pub fn task_costs(&self) -> Vec<checks::TaskCost> {
        checks::task_costs(
            &self.tasks,
            &self.workers,
            &self.timesheet,
            &self.materials,
            &self.stock_moves,
            &self.machines,
            &self.machine_logs,
        )
    }

    /// Moliyaviy prognoz (TZ XVII.13-14, 32-34).
    ///
    /// Tannarx ishlar kesimidan yig'iladi — analitikadagi «tannarx tahlili»
    /// bilan bir xil manbadan, shuning uchun sonlar zid kelmaydi.
    pub fn finance_forecast(&self) -> checks::FinanceForecast {
        let cost_now: f64 = self.task_costs().iter().map(|c| c.total).sum();
        checks::finance_forecast(
            self.project().map(|p| p.contract_sum).unwrap_or(0.0),
            &self.contract_changes,
            &self.payment_stages,
            cost_now,
            self.progress.fact_pct,
            self.today,
        )
    }

    /// Yashirin yo'qotishlar va tejash imkoniyatlari (TZ XVII.41, 44).
    pub fn opportunities(&self) -> Vec<checks::Opportunity> {
        // Texnika oxirgi 30 kun kesimida baholanadi: bo'sh turganini
        // ko'rish uchun yaqin davr kerak.
        let from = self.today - chrono::Duration::days(30);
        let machines = checks::machine_lines(
            &self.machines,
            &self.machine_logs,
            from,
            self.today,
            self.today,
        );
        let usage = checks::consumption(
            &self.material_norms,
            &self.tasks,
            &self.materials,
            &self.stock_moves,
        );
        checks::opportunities(
            &self.stock(),
            &self.materials,
            &machines,
            &self.quotes,
            &self.purchases,
            &usage,
        )
    }

    /// Ishlar bo'yicha unumdorlik (TZ XVII.26).
    pub fn productivity(&self) -> Vec<checks::Productivity> {
        checks::productivity(&self.tasks, &self.timesheet, &self.workers)
    }

    /// Asboblarning hozirgi holati (TZ XI.34).
    pub fn tool_status(&self) -> Vec<checks::ToolStatus> {
        checks::tool_status(&self.tools, &self.tool_issues, self.today)
    }

    /// Yopilishga yaqin ishlarning sifat to'siqlari (TZ XIV.10, 35).
    pub fn task_blocks(&self) -> Vec<checks::TaskBlock> {
        checks::task_blocks(&self.tasks, &self.quality, &self.check_points, self.today)
    }

    // ---------- Til modeli bilan suhbat (TZ XVIII) ----------

    /// Modelga jo'natiladigan kontekst: obyekt bo'yicha **tayyor sonlar**.
    ///
    /// Model sonni o'zi hisoblamaydi va o'ylab topmaydi — u faqat shu yerdagi
    /// raqamlarni tushuntiradi. Shuning uchun javobni har doim tegishli ekranda
    /// tekshirib ko'rish mumkin.
    pub fn llm_context(&self) -> String {
        let Some(project) = self.project() else {
            return String::new();
        };
        let supply = self.supply();
        let stock = self.stock();
        let cost = self.cost_summary();
        let sales = self.sales();
        let inp = self.analytics_input(&supply, &stock, &cost, &sales);

        let mut out = String::new();
        out.push_str(&format!(
            "Obyekt: {} ({})\nSana: {}\n\n",
            project.name,
            project.code,
            self.today.format("%d.%m.%Y")
        ));

        // Har bo'lim bo'yicha yordamchining o'z javobi — ya'ni ekranlarda
        // ko'rinadigan aynan o'sha sonlar.
        for intent in crate::copilot::Intent::ALL {
            let a = crate::copilot::answer(*intent, &inp);
            if a.lines.is_empty() {
                continue;
            }
            out.push_str(&format!("## {}\n", a.title));
            for l in &a.lines {
                out.push_str(&format!("{}: {}\n", l.label, l.value));
            }
            out.push('\n');
        }
        crate::llm::trim_context(&out)
    }

    /// Savolni fonda modelga jo'natadi.
    ///
    /// Interfeys javob kutib qotib qolmaydi: so'rov alohida oqimda ketadi,
    /// natija esa keyingi kadrlarda [`App::poll_llm`] orqali olinadi.
    pub fn ask_llm(&mut self, question: String) {
        if self.llm_pending.is_some() {
            return;
        }
        if !self.llm.is_ready() {
            let e = crate::llm::Error::NotConfigured;
            self.llm_error = Some((e.key(), String::new(), e.retryable()));
            return;
        }
        if question.trim().is_empty() {
            return;
        }
        self.llm_error = None;
        let context = self.llm_context();
        let history = self.llm_chat.clone();
        self.llm_chat.push(crate::llm::Turn::user(question.clone()));
        self.llm_pending = Some(crate::llm::spawn(
            self.llm.clone(),
            history,
            question,
            context,
        ));
    }

    /// Fon so'rovi tugagan bo'lsa javobni oladi.
    ///
    /// Har kadrda chaqiriladi va bloklanmaydi: javob hali kelmagan bo'lsa
    /// funksiya darhol qaytadi.
    pub fn poll_llm(&mut self) -> bool {
        use std::sync::mpsc::TryRecvError;
        let Some(rx) = &self.llm_pending else {
            return false;
        };
        match rx.try_recv() {
            Ok(Ok(answer)) => {
                self.llm_tokens += answer.usage.total;
                self.llm_chat.push(crate::llm::Turn::model(answer.text));
                self.llm_pending = None;
                true
            }
            Ok(Err(e)) => {
                // Savol tarixda qoladi: foydalanuvchi uni qayta yozmasin.
                self.llm_error = Some((e.key(), e.to_string(), e.retryable()));
                self.llm_pending = None;
                true
            }
            Err(TryRecvError::Empty) => false,
            // Oqim to'xtab qolgan — bu ham xato, yashirmaymiz.
            Err(TryRecvError::Disconnected) => {
                let e = crate::llm::Error::Transport(String::new());
                self.llm_error = Some((e.key(), String::new(), e.retryable()));
                self.llm_pending = None;
                true
            }
        }
    }

    /// Suhbatni tozalaydi.
    pub fn clear_llm_chat(&mut self) {
        self.llm_chat.clear();
        self.llm_error = None;
        self.llm_tokens = 0;
    }

    /// Obyektning yakuniy qabulga tayyorligi (TZ VII.35-36).
    pub fn final_readiness(&self) -> checks::FinalReadiness {
        checks::final_readiness(&checks::FinalCtx {
            tasks: &self.tasks,
            required: &checks::required_docs(&self.tasks, &self.exec_docs, false),
            quality: &self.quality,
            lab_tests: &self.lab_tests,
            inspections: &self.inspections,
            safety: &self.safety,
            acceptances: &self.acceptances,
        })
    }

    /// Bugun ketayotgan ishlar: grafik bo'yicha bugunni qamragan va
    /// hali tugallanmaganlari.
    ///
    /// Ro'yxat bitta joyda hisoblanadi — prorab ekrani ham, kunni yakunlash
    /// tekshiruvi ham shundan oladi, shuning uchun ular bir xil ishni
    /// ko'rsatadi.
    pub fn running_today(&self) -> Vec<i64> {
        self.running_on(self.today)
    }

    /// Berilgan kunda grafik bo'yicha ketayotgan ishlar.
    pub fn running_on(&self, day: chrono::NaiveDate) -> Vec<i64> {
        let origin = self.origin();
        self.tasks
            .iter()
            .filter(|t| t.progress < 99.99)
            .filter(|t| {
                let Some(c) = self.schedule.get(t.id) else {
                    return false;
                };
                let start = origin + chrono::Duration::days(c.es);
                let end = origin + chrono::Duration::days(c.ef);
                start <= day && day <= end
            })
            .map(|t| t.id)
            .collect()
    }

    /// Kunlik hajmga sarflangan material (TZ V.10-11).
    pub fn day_material(&self, day: chrono::NaiveDate) -> Vec<checks::DayMaterial> {
        checks::day_material(&self.journal, &self.material_norms, &self.stock_moves, day)
    }

    /// Ertangi kunga reja (TZ V.16).
    pub fn tomorrow_plan(&self) -> Vec<checks::TomorrowTask> {
        let tomorrow = self.today + chrono::Duration::days(1);
        checks::tomorrow_plan(
            &self.tasks,
            &self.running_on(tomorrow),
            &self.running_today(),
            &self.material_kits(),
            &self.timesheet,
            self.today,
        )
    }

    /// Jurnal yozuvlaridagi ichki ziddiyatlar (TZ V.32).
    pub fn journal_doubts(&self) -> Vec<checks::JournalCheck> {
        checks::journal_doubts(&self.journal, &self.tasks, &self.timesheet, self.today)
    }

    /// Kunni yakunlash va hisobotni yuborishdan oldingi tekshiruv (TZ VI.33-34).
    pub fn day_close(&self) -> Vec<checks::DayIssue> {
        let open_issues = self
            .safety
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    crate::domain::IssueStatus::Open | crate::domain::IssueStatus::InWork
                )
            })
            .count();
        checks::day_close(&checks::DayCtx {
            journal: &self.journal,
            timesheet: &self.timesheet,
            tasks: &self.tasks,
            running: &self.running_today(),
            open_issues,
            day: self.today,
        })
    }

    /// Geometriya bo'yicha kolliziyalar (TZ II, VII.31).
    ///
    /// Qoidalar bo'yicha topilmalardan alohida: bu yerdagi xulosa
    /// elementlarning **o'lchamiga** tayanadi va faqat shakli tanilgan
    /// elementlarni qamraydi. Ro'yxat elementlar o'qilganda hisoblanadi
    /// va shu yerda faqat qaytariladi.
    pub fn geometry_clashes(&self) -> &[crate::clash::Clash] {
        &self.clashes
    }

    /// Prorabga beriladigan savollar (TZ V.25, XIV.37).
    ///
    /// Kun yakuni ro'yxati nima **to'ldirilmaganini** aytadi; bu esa
    /// nima **tushunarsiz** ekanini so'raydi. Ikkalasi bir joyda emas:
    /// biri belgi qo'yish uchun, ikkinchisi javob yozish uchun.
    pub fn foreman_questions(&self) -> Vec<checks::Question> {
        let finish = |id: i64| {
            self.schedule
                .get(id)
                .map(|c| self.origin() + chrono::Duration::days(c.ef.max(0)))
        };
        checks::foreman_questions(&checks::QuestionCtx {
            day: self.today,
            journal: &self.journal,
            timesheet: &self.timesheet,
            moves: &self.stock_moves,
            materials: &self.materials,
            tasks: &self.tasks,
            finish: &finish,
        })
    }

    /// Savolga javobni bugungi kunlik yozuvning izohiga qo'shadi.
    ///
    /// Javob alohida jadvalga emas, **jurnalga** yoziladi: savol
    /// jurnaldagi ziddiyatdan tug'ilgan va javobi ham o'sha yerda
    /// turishi kerak. Bugungi yozuv bo'lmasa javob ham yozilmaydi —
    /// yozuvni javob uchun o'zi yaratib qo'yish chalkashlik bo'lardi.
    pub fn answer_question(&mut self, question: &checks::Question, answer: &str) -> bool {
        let answer = answer.trim();
        if answer.is_empty() {
            return false;
        }
        let Some(mut entry) = self.journal.iter().find(|j| j.date == self.today).cloned() else {
            self.notify(t("q_needs_journal").to_string());
            return false;
        };
        let line = format!("{} — {answer}", question.text());
        entry.remarks = if entry.remarks.trim().is_empty() {
            line
        } else {
            format!(
                "{}
{line}",
                entry.remarks
            )
        };
        let ok = self.db.update_journal(&entry);
        if ok {
            self.reload_modules();
            self.notify(t("q_answered").to_string());
        }
        ok
    }

    /// Ishni yopishdan oldingi ogohlantirishlar (TZ VI.25).
    pub fn close_warnings(&self, task_id: i64) -> Vec<checks::CloseWarning> {
        checks::close_warnings(
            task_id,
            &self.task_blocks(),
            &checks::required_docs(&self.tasks, &self.exec_docs, false),
            &self.stock_moves,
            &self.timesheet,
            &self.journal,
        )
    }

    /// QR yorliqlarni PDF ga saqlaydi (TZ VI.11).
    ///
    /// Kod ichida server manzili bo'lsa havola, bo'lmasa obyekt va yozuv
    /// turi yoziladi — ikkala holatda ham telefon nimaligini ko'rsatadi.
    pub fn save_labels(&mut self, kind: crate::qr::Kind, rows: Vec<(String, String, String)>) {
        if rows.is_empty() {
            self.notify(t("qr_empty").to_string());
            return;
        }
        let project = self.project().map(project_key).unwrap_or_default();
        let base = if self.sync.ready() {
            self.sync.url.clone()
        } else {
            String::new()
        };
        let labels: Vec<crate::qr::Label> = rows
            .into_iter()
            .map(|(number, title, note)| crate::qr::Label {
                code: crate::qr::code(&base, &project, kind, &number),
                title,
                note,
            })
            .collect();

        let name = format!(
            "qurai-{}-{}-{}.pdf",
            project.to_lowercase(),
            kind.tag(),
            self.today.format("%Y-%m-%d")
        );
        let Some(path) = rfd::FileDialog::new()
            .set_title(t("qr_labels"))
            .set_file_name(name)
            .add_filter("PDF", &["pdf"])
            .save_file()
        else {
            return;
        };
        let count = labels.len();
        match crate::qr::write_labels(&path, t("qr_labels"), &labels) {
            Ok(()) => self.notify(format!("{} {count} · {}", t("qr_done"), path.display())),
            Err(e) => self.notify(format!("{}: {e}", t("export_failed"))),
        }
    }

    /// Obyekt geozonasi (TZ XIII.5, XV.5).
    ///
    /// Sozlama obyekt bo'yicha saqlanadi: bitta bazada bir necha obyekt
    /// bo'lishi mumkin va ularning joyi har xil.
    pub fn fence(&self) -> Option<crate::geo::Fence> {
        let id = self.project()?.id;
        crate::geo::Fence::parse(&self.db.get_setting(&format!("fence.{id}"))?)
    }

    /// Geozonani saqlaydi. `None` — o'chirish.
    pub fn set_fence(&mut self, fence: Option<crate::geo::Fence>) {
        let Some(id) = self.project().map(|p| p.id) else {
            return;
        };
        let value = fence.map(|f| f.store()).unwrap_or_default();
        let _ = self.db.set_setting(&format!("fence.{id}"), &value);
    }

    /// Kirish-chiqish belgilaridan kunlar (TZ XIII.4).
    pub fn attendance_days(&self) -> Vec<crate::attend::Day> {
        crate::attend::days(&self.attendance, self.fence())
    }

    /// Belgi va tabel orasidagi farqlar (TZ XIII.4).
    pub fn attendance_mismatch(&self) -> Vec<crate::attend::Mismatch> {
        crate::attend::compare(&self.attendance_days(), &self.timesheet, &self.workers)
    }

    /// Jurnal fotolariga e'tirozlar (TZ V.9, XIV.14).
    ///
    /// Fayllar diskdan o'qiladi, shuning uchun chaqiruv har kadrda emas,
    /// tugma bosilganda bajariladi.
    pub fn photo_issues(&self) -> Vec<crate::photocheck::PhotoIssue> {
        crate::photocheck::check(
            &crate::photocheck::from_journal(&self.journal),
            self.fence(),
        )
    }

    /// Serverga yuboriladigan QR yorliqlar ro'yxati (TZ VI.11).
    ///
    /// Telefon o'qigan kod nimaligini ko'rsatishi uchun kerak. Yozuvning
    /// o'zi emas, faqat qisqa nomi ketadi: baza va hisob-kitob ilovada
    /// qoladi.
    pub fn label_refs(&self) -> Vec<crate::sync::LabelOut> {
        let mut out = Vec::new();
        for u in &self.units {
            out.push(crate::sync::LabelOut {
                kind: "unit".into(),
                number: u.number.clone(),
                title: format!("{} {}", t("col_unit"), u.number),
                note: format!(
                    "{} {} · {} m² · {}",
                    u.floor,
                    t("floor_short"),
                    crate::ui::materials::trim_num(u.area),
                    u.kind.label()
                ),
            });
        }
        for b in &self.batches {
            let material = self
                .materials
                .iter()
                .find(|m| m.id == b.material_id)
                .map(|m| m.name.clone())
                .unwrap_or_default();
            out.push(crate::sync::LabelOut {
                kind: "batch".into(),
                number: b.number.clone(),
                title: material,
                note: format!("{} · {}", b.received, b.supplier),
            });
        }
        for d in &self.exec_docs {
            out.push(crate::sync::LabelOut {
                kind: "doc".into(),
                number: d.number.clone(),
                title: d.name.clone(),
                note: format!("{} · {}", d.kind.label(), d.date),
            });
        }
        for w in self.workers.iter().filter(|w| w.active) {
            out.push(crate::sync::LabelOut {
                kind: "worker".into(),
                number: w.name.clone(),
                title: w.name.clone(),
                note: w.position.clone(),
            });
        }
        out
    }

    /// Yozishmaga xabar qo'shadi (TZ VI.32).
    ///
    /// Xabar **serverga** ketadi, shuning uchun aloqa kerak. Aloqa
    /// bo'lmasa matn maydonda qoladi va sabab aytiladi — «yuborildi» deb
    /// aldab qo'yilmaydi.
    pub fn send_chat(&mut self) {
        let text = self.chat_draft.trim().to_string();
        if text.is_empty() {
            return;
        }
        if !self.sync.ready() {
            self.notify(t("chat_needs_server").to_string());
            return;
        }
        if self.sync_pending.is_some() {
            self.notify(t("sync_running").to_string());
            return;
        }
        self.chat_outgoing = text;
        self.sync_now();
    }

    /// Hujjatni imzo daftariga yozadi (TZ IV.18, V.28).
    ///
    /// Bu **hujjat butunligi** yozuvi: imzolangan matnning xeshi olinadi
    /// va yozuv oldingisiga bog'lanadi. Davlat elektron raqamli imzosi
    /// emas — kim imzolagani ism bilan yoziladi, kalit bilan emas.
    pub fn sign_document(&mut self, document: &str, subject: &str, text: &str, rejected: &str) {
        let Some(pid) = self.project().map(|p| p.id) else {
            return;
        };
        if document.trim().is_empty() || text.trim().is_empty() {
            self.notify(t("sl_nothing_to_sign").to_string());
            return;
        }
        let prev = self
            .sign_log
            .last()
            .map(|e| e.chain.clone())
            .unwrap_or_default();
        let mut entry = crate::domain::SignEntry {
            id: 0,
            project_id: pid,
            document: document.trim().to_string(),
            subject: subject.trim().to_string(),
            signer: self.current_user_name(),
            role: self.role().code().to_string(),
            // Soniyagacha: bir kunda bir necha imzo bo'lishi mumkin va
            // ularning tartibi ko'rinib turishi kerak.
            at: crate::store::parse_stamp(
                &chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            ),
            digest: qurai_hash::text(text),
            chain: String::new(),
            rejected: rejected.trim().to_string(),
        };
        entry.chain = crate::signlog::link(&prev, &entry);
        self.db.insert_sign_entry(&entry);
        self.sign_log = self.db.sign_log(pid);
        self.notify(format!("{} {document}", t("sl_signed")));
    }

    /// Imzolanadigan hujjat matni (TZ IV.18).
    ///
    /// Matn **bitta joyda** tuziladi: imzolashda ham, keyin tekshirishda
    /// ham shu funksiya chaqiriladi. Ikkita ko'rinish bo'lsa, tekshiruv
    /// hech qachon mos kelmasdi.
    pub fn document_text(&self, doc: &crate::domain::ExecDoc) -> String {
        format!(
            "{} · {} · {} · v{}",
            doc.number,
            doc.name,
            doc.date.format("%d.%m.%Y"),
            doc.version
        )
    }

    /// Imzo daftarining holati (TZ IV.18, V.32).
    ///
    /// Ikki narsa tekshiriladi: zanjir butunmi va imzolangan hujjat
    /// keyin o'zgarmaganmi. Hujjat topilmasa hukm chiqarilmaydi — u
    /// o'chirilgan bo'lishi mumkin va bu boshqa masala.
    pub fn sign_breaks(&self) -> Vec<crate::signlog::Break> {
        let today_report = self.day_report_number();
        crate::signlog::verify_with_text(&self.sign_log, |e| {
            // Kunlik hisobot: matni qayta hisoblanadi. Faqat **bugungi**
            // hisobot tekshiriladi — o'tgan kunning ma'lumotini qayta
            // tuzish uchun o'sha kundagi holat kerak va u saqlanmaydi.
            if e.document == today_report {
                return Some(self.day_report_text());
            }
            if e.document.starts_with("KUN-") {
                return None;
            }
            self.exec_docs
                .iter()
                .find(|d| d.number == e.document)
                .map(|d| self.document_text(d))
        })
    }

    /// Serverdagi belgilar bilan solishtiradi.
    ///
    /// Server daftarni ko'rmaydi — u faqat «shu uzunlikda uch shunday
    /// edi» deb eslab qoladi. Shuning uchun tekshiruv shu yerda
    /// bajariladi: har belgi uchun daftardagi o'sha uzunlikdagi uch
    /// qayta olinadi va solishtiriladi.
    pub fn chain_alerts(&self) -> Vec<String> {
        let mut out = Vec::new();
        for mark in &self.chain_marks {
            let count = mark.count.max(0) as usize;
            match crate::signlog::head_at(&self.sign_log, count) {
                Some(head) if head == mark.head => {}
                Some(_) => out.push(format!(
                    "{} {count}: {}",
                    t("sl_mark"),
                    t("sl_mark_differs")
                )),
                None => out.push(format!(
                    "{} {count}: {}",
                    t("sl_mark"),
                    t("sl_mark_missing")
                )),
            }
        }
        out
    }

    /// Tashqi xabar sozlamasini o'qiydi (TZ VI.29, XVI.45).
    fn load_hook(db: &Db) -> crate::hook::Config {
        crate::hook::Config {
            on: db.get_setting("hook.on").as_deref() == Some("1"),
            url: db.get_setting("hook.url").unwrap_or_default(),
            token: db.get_setting("hook.token").unwrap_or_default(),
        }
    }

    /// Sozlamani saqlaydi.
    pub fn save_hook(&mut self) {
        let _ = self
            .db
            .set_setting("hook.on", if self.hook.on { "1" } else { "0" });
        let _ = self.db.set_setting("hook.url", &self.hook.url);
        let _ = self.db.set_setting("hook.token", &self.hook.token);
    }

    /// Yuborilgan signallar ro'yxati.
    ///
    /// Ro'yxat cheklangan: eskisi tashlanadi, chunki bartaraf etilgan
    /// signal qaytib kelsa u **yangi xabar** bo'lishi kerak.
    fn hook_sent(&self) -> Vec<String> {
        self.db
            .get_setting("hook.sent")
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// Yangi yuborilganlarni ro'yxatga qo'shadi.
    fn remember_sent(&mut self, keys: &[String]) {
        let mut all = self.hook_sent();
        for k in keys {
            if !all.contains(k) {
                all.push(k.clone());
            }
        }
        // Oxirgi 200 tasi yetarli: undan eskisi allaqachon boshqa xabar.
        let start = all.len().saturating_sub(200);
        let _ = self.db.set_setting("hook.sent", &all[start..].join("\n"));
    }

    /// Signallarni tashqi manzilga yuboradi (TZ VI.29, XVI.45).
    ///
    /// Faqat **yangi va jiddiy** signallar ketadi: hamma narsani
    /// yuborish odamni xabarga befarq qilib qo'yadi.
    pub fn send_hook(&mut self) {
        if self.hook_pending.is_some() {
            return;
        }
        if !self.hook.ready() {
            self.hook_status = Some((crate::hook::Outcome::Off.text(), true));
            return;
        }
        let Some(project) = self.project().map(project_key) else {
            self.notify(t("no_object_selected").to_string());
            return;
        };
        let items: Vec<crate::hook::Item> = self
            .notices
            .iter()
            .filter(|n| n.severity <= crate::domain::Severity::Major)
            .map(|n| crate::hook::Item {
                code: n.code.to_string(),
                severity: n.severity.code().to_string(),
                title: n.title.clone(),
                detail: n.detail.clone(),
            })
            .collect();
        let fresh = crate::hook::fresh(&items, &self.hook_sent());
        if fresh.is_empty() {
            self.hook_status = Some((crate::hook::Outcome::Nothing.text(), false));
            return;
        }

        let cfg = self.hook.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let keys: Vec<String> = fresh.iter().map(|i| i.key()).collect();
        std::thread::spawn(move || {
            let _ = tx.send((crate::hook::send(&cfg, &project, &fresh), keys));
        });
        self.hook_pending = Some(rx);
    }

    /// Fon oqimidagi yuborish natijasini oladi.
    pub fn poll_hook(&mut self) {
        let Some(rx) = &self.hook_pending else {
            return;
        };
        let Ok((outcome, keys)) = rx.try_recv() else {
            return;
        };
        self.hook_pending = None;
        // Faqat haqiqatan yuborilgani eslab qolinadi: xato bo'lsa xabar
        // keyingi safar qayta urinishi kerak.
        if matches!(outcome, crate::hook::Outcome::Sent(_)) {
            self.remember_sent(&keys);
        }
        self.hook_status = Some((outcome.text(), outcome.bad()));
    }

    /// Narx ro'yxatini fayldan yuklaydi (TZ III.15).
    pub fn import_prices(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(t("pb_import"))
            .add_filter(
                t("import_file_filter"),
                &["xlsx", "xlsm", "xls", "ods", "csv"],
            )
            .pick_file()
        else {
            return;
        };
        match crate::import::prices_from_file(&path) {
            Ok(rows) => {
                let n = self.db.add_prices(&rows);
                self.price_book = self.db.price_book();
                self.notify(format!("{} {n}", t("pb_imported")));
            }
            Err(e) => self.notify(format!("{}: {e}", t("import_failed"))),
        }
    }

    /// Narxlar bazasini tozalaydi.
    ///
    /// Bu **qaytarilmaydigan** amal, shuning uchun tasdiqdan keyin
    /// bajariladi: tugma faqat so'roqni ochadi.
    pub fn clear_prices(&mut self) {
        let n = self.db.clear_price_book();
        self.price_book.clear();
        self.notify(format!("{} {n}", t("pb_cleared")));
    }

    /// Material uchun bazadagi narx diapazoni (TZ XII.20, III.14.3).
    pub fn market_range(&self, material: &crate::domain::Material) -> Option<crate::prices::Range> {
        let found = crate::prices::find(
            &self.price_book,
            &material.name,
            &material.code,
            &material.unit,
        );
        crate::prices::range(&found)
    }

    /// Materialning o'z kirimlaridagi narx yo'nalishi (TZ X.44).
    ///
    /// Faqat kirimlar olinadi: chiqim narxi hisob narxi bo'lib, u bozor
    /// haqida hech nima aytmaydi.
    pub fn price_trend(&self, material_id: i64) -> Option<crate::prices::Trend> {
        let history: Vec<(chrono::NaiveDate, f64)> = self
            .stock_moves
            .iter()
            .filter(|m| {
                m.material_id == material_id
                    && m.kind == crate::domain::MoveKind::In
                    && m.price > 0.0
            })
            .map(|m| (m.date, m.price))
            .collect();
        crate::prices::trend(&history)
    }

    /// Sertifikat nazorati (TZ IV.11, XII.27).
    pub fn cert_control(&self) -> Vec<checks::CertIssue> {
        checks::cert_control(
            &self.materials,
            &self.batches,
            &self.stock_moves,
            &self.stock(),
            self.today,
        )
    }

    /// Mexanik kabineti: kunlik ko'rik, TX va operator (TZ XVI.28, 31, 46).
    pub fn mech_issues(&self) -> Vec<checks::MechIssue> {
        checks::mech_issues(&checks::MechCtx {
            machines: &self.machines,
            logs: &self.machine_logs,
            checks: &self.machine_checks,
            workers: &self.workers,
            permits: &self.worker_permits,
            today: self.today,
        })
    }

    /// Park bo'yicha foydalanish va tavsiyalar (TZ XVI.39).
    pub fn park_review(&self) -> (Vec<checks::ParkLine>, Vec<checks::ParkAdvice>) {
        checks::park_review(&self.machines, &self.machine_logs, self.today, PARK_DAYS)
    }

    /// Ombor nazorati: kirim, smeta bog'lanishi, harorat, yoqilg'i
    /// (TZ XI.8, 13, 29, 31).
    pub fn stock_control(&self) -> Vec<checks::StockIssue> {
        checks::stock_control(&checks::StockCtx {
            moves: &self.stock_moves,
            materials: &self.materials,
            purchases: &self.purchases,
            warehouses: &self.warehouses,
            machine_logs: &self.machine_logs,
            today: self.today,
        })
    }

    // ---------- XVII. Kesimlar bo'yicha tahlil ----------

    /// Haftalik hisobot (TZ VIII.31, XVII.38).
    ///
    /// Bitta chaqiruv nuqtasi: buyurtmachi kabineti ham, analitika ham
    /// shundan oladi — ikki ekranda turli son bo'lishi mumkin emas.
    pub fn week_report(&self) -> checks::WeekReport {
        checks::week_report(
            self.today,
            &self.tasks,
            &self.journal,
            &self.inspections,
            &self.issues,
            &self.exec_docs,
            &self.payment_stages,
            &self.contract_changes,
            &self.acceptances,
        )
    }

    /// Mas'ullar kesimida ish, sifat va xavfsizlik (TZ XVII.23).
    pub fn contractor_report(&self) -> Vec<checks::ContractorReport> {
        checks::contractor_report(&self.tasks, &self.quality, &self.safety, self.today)
    }

    /// Yetkazib beruvchilar kesimida ishonchlilik (TZ XVII.24).
    pub fn supplier_report(&self) -> Vec<checks::SupplierReport> {
        checks::supplier_report(&self.purchases, &self.quotes, &self.quality, self.today)
    }

    /// Ko'rsatkichlar orasidagi bog'liqlik (TZ XVII.25).
    pub fn correlations(&self) -> Vec<checks::Correlation> {
        checks::correlations(
            &self.tasks,
            &self.journal,
            &self.timesheet,
            &self.task_costs(),
            self.today,
        )
    }

    /// Shartnoma o'zgarishlari bo'yicha yakun (TZ XVII.35).
    pub fn change_report(&self) -> checks::ChangeReport {
        checks::change_report(&self.contract_changes)
    }

    /// Rahbar uchun umumiy ball (TZ XVII.49).
    ///
    /// Ball yangi hisob qilmaydi: sifat va xavfsizlik ballari o'z
    /// modullaridan, qolganlari esa shu ekranlardagi sonlardan olinadi.
    pub fn executive_score(&self) -> checks::ExecutiveScore {
        let chain = self.estimate_chain();
        let earned: f64 = chain.iter().map(|l| l.earned).sum();
        let actual: f64 = chain.iter().map(|l| l.actual).sum();
        checks::executive_score(&checks::ExecCtx {
            delay_days: self.progress.delay_days,
            overdue: self.progress.overdue.len(),
            tasks: self.tasks.len(),
            earned,
            actual,
            quality_score: checks::quality_score(&self.quality, self.today).score,
            safety_score: checks::safety_score(
                &self.safety,
                &self.worker_safety(),
                &self.work_permits,
                self.today,
            )
            .score,
            supply: &self.supply(),
            required_docs: &checks::required_docs(&self.tasks, &self.exec_docs, false),
        })
    }

    /// Xaridlar tartibi: texnik kelishuv, almashtirish, shartnoma
    /// (TZ X.18-19, 22).
    pub fn supply_control(&self) -> Vec<checks::SupplyIssue> {
        checks::supply_control(
            &self.purchases,
            &self.contracts,
            &self.material_alts,
            &self.materials,
            self.today,
        )
    }

    /// Materialning loyihaga mosligi (TZ XII.8, 15, 32).
    pub fn material_fit(&self) -> Vec<checks::MaterialFit> {
        checks::material_fit(&self.materials, &self.stock_moves, self.today)
    }

    /// Ishlar bo'yicha material komplekti (TZ XII.33).
    pub fn material_kits(&self) -> Vec<checks::MaterialKit> {
        checks::material_kits(
            &self.material_norms,
            &self.tasks,
            &self.readiness(),
            self.today,
            READINESS_DAYS,
        )
    }

    /// Yetkazib beruvchilarni material kesimida solishtirish (TZ XII.34).
    pub fn maker_comparison(&self) -> Vec<checks::MakerComparison> {
        checks::maker_comparison(&self.quotes, &self.purchases)
    }

    /// Hujjatlarni imzolashdan oldingi tekshiruv (TZ IV.20).
    pub fn doc_readiness(&self) -> Vec<checks::DocCheck> {
        checks::doc_readiness(&checks::DocCtx {
            docs: &self.exec_docs,
            tasks: &self.tasks,
            inspections: &self.inspections,
            lab_tests: &self.lab_tests,
            concrete: &self.concrete_tests,
        })
    }

    /// Yashirin ish yopilmagani uchun to'silgan ishlar (TZ IV.16).
    pub fn hidden_blocks(&self) -> Vec<checks::HiddenBlock> {
        checks::hidden_blocks(&self.tasks, &self.links, &self.exec_docs)
    }

    /// Xodim ehtiyoji prognozi (TZ XIII.26-27).
    ///
    /// Bitta chaqiruv nuqtasi: tabel ekrani ham, ariza tekshiruvi ham
    /// shundan oladi — ikki joyda turli son bo'lishi mumkin emas.
    pub fn staff_forecast(&self) -> checks::StaffForecast {
        checks::staff_forecast(
            &self.tasks,
            &self.productivity(),
            &self.workers,
            self.today,
            STAFF_HORIZON,
        )
    }

    /// Ish grafigi bo'yicha e'tirozlar (TZ XIII.12).
    pub fn schedule_issues(&self) -> Vec<checks::ScheduleIssue> {
        let to = self.today;
        let from = to - chrono::Duration::days(SCHEDULE_DAYS);
        checks::schedule_issues(&self.timesheet, &self.work_schedule, from, to)
    }

    /// Xodimni boshqa obyektga ko'chiradi (TZ XIII.28).
    ///
    /// Tabel yozuvlari **ko'chirilmaydi**: ular o'sha obyektda ishlangan
    /// soatning yozuvi va o'z joyida qolishi kerak. Ko'chirilgandan keyin
    /// yangi obyektda yangi yozuvlar paydo bo'ladi.
    pub fn move_worker(&mut self, worker_id: i64, to_project: i64) -> bool {
        let Some(w) = self.workers.iter().find(|w| w.id == worker_id).cloned() else {
            return false;
        };
        if w.project_id == to_project {
            return false;
        }
        // Ko'chirish alohida amal: oddiy tahrirlash obyektga tegmaydi.
        let ok = self.db.move_worker(w.id, to_project);
        if ok {
            self.reload_modules();
        }
        ok
    }

    /// Ijro sxemalari holati (TZ IV.7).
    pub fn scheme_status(&self) -> Vec<checks::SchemeStatus> {
        checks::scheme_status(&self.exec_docs, &self.geodesy_points, &self.inspections)
    }

    /// Mualliflik nazorati kabineti (TZ IV.24).
    pub fn author_supervision(&self) -> Vec<checks::AuthorTask> {
        checks::author_supervision(
            &self.issues,
            &self.contract_changes,
            &self.documents,
            &self.inspections,
            self.today,
        )
    }

    /// Ta'minot zanjiri: ariza → taklif → xarid → ombor → to'lov
    /// (TZ X.47, XI.47).
    pub fn supply_chain(
        &self,
    ) -> (
        Vec<(checks::SupplyChain, Vec<checks::ChainGap>)>,
        checks::ChainSummary,
    ) {
        checks::supply_chain(
            &self.purchases,
            &self.requests,
            &self.quotes,
            &self.stock_moves,
            &self.quality,
            &self.materials,
            self.today,
        )
    }

    /// Texnika bo'yicha to'liq zanjir va samaradorlik (TZ XVI.21, 42, 48).
    pub fn machine_chain(&self) -> Vec<checks::MachineChain> {
        checks::machine_chain(
            &self.machines,
            &self.machine_logs,
            &self.repairs,
            &self.requests,
            self.today,
            PARK_DAYS,
        )
    }

    /// Ta'mir prognozi (TZ XVI.26).
    pub fn repair_forecast(&self) -> Vec<checks::RepairForecast> {
        checks::repair_forecast(
            &self.machines,
            &self.machine_logs,
            &self.repairs,
            self.today,
            PARK_DAYS,
        )
    }

    /// Bo'limlar kesimida sifat (TZ XIV.15).
    pub fn section_quality(&self) -> Vec<checks::SectionQuality> {
        checks::section_quality(&self.quality, &self.tasks, self.today)
    }

    /// Nuqsonlar ustuvorligi (TZ XIV.19).
    pub fn defect_priority(&self) -> Vec<checks::DefectPriority> {
        checks::defect_priority(&self.quality, &self.tasks, self.today)
    }

    /// Texnologik ketma-ketlik buzilishlari (TZ XIV.27, VII.28).
    pub fn sequence_breaks(&self) -> Vec<checks::SequenceBreak> {
        checks::sequence_breaks(&self.tasks, &self.links, &self.quality)
    }

    /// Qaror kutayotgan ishlar (TZ XVII.42).
    pub fn decisions(&self) -> Vec<checks::Decision> {
        checks::decisions(&checks::DecisionCtx {
            requests: &self.requests,
            changes: &self.contract_changes,
            acceptances: &self.acceptances,
            supply: &self.supply_control(),
            quality: &self.quality,
            docs: &self.doc_readiness(),
            mech: &self.mech_issues(),
            today: self.today,
        })
    }

    /// Ish haqi fondi tarkibi (TZ XIII.32).
    pub fn payroll_summary(&self, from: chrono::NaiveDate) -> checks::PayrollSummary {
        checks::payroll_summary(&self.timesheet, &self.workers, from, self.today)
    }

    /// Brigadalar kesimida bo'sh turish (TZ XIII.23).
    pub fn idle_by_brigade(&self, from: chrono::NaiveDate) -> Vec<checks::IdleLine> {
        checks::idle_by_brigade(&self.timesheet, &self.workers, from, self.today)
    }

    /// Kunlik xavfsizlik hisoboti (TZ XV.32).
    pub fn safety_day(&self) -> checks::SafetyDay {
        checks::safety_day(
            &self.safety,
            &self.worker_safety(),
            &self.work_permits,
            &self.machines,
            &self.machine_checks,
            &self.machine_logs,
            self.today,
        )
    }

    /// Material kartochkasi: qoldiq, narx, ta'minotchi, analog, sarf
    /// va yaqin ehtiyoj (TZ XII.11, 18, 22, 25-27).
    pub fn material_card(&self, material_id: i64) -> Option<checks::MaterialCard> {
        checks::material_card(
            &checks::CardCtx {
                materials: &self.materials,
                stock: &self.stock(),
                purchases: &self.purchases,
                alts: &self.material_alts,
                tasks: &self.tasks,
                readiness: &self.readiness(),
                consumption: &self.consumption(),
            },
            material_id,
        )
    }

    /// Hujjat matritsasi: ishlar × hujjat turlari (TZ IV.4).
    pub fn document_matrix(&self) -> Vec<checks::MatrixRow> {
        checks::document_matrix(&self.tasks, &self.exec_docs)
    }

    /// Nazoratsiz hisobdan chiqarishlar (TZ XI.23).
    pub fn write_offs(&self) -> Vec<checks::WriteOff> {
        checks::write_offs(&self.stock_moves, self.today, SCHEDULE_DAYS)
    }

    /// Loyiha va smeta hajmlarini solishtirish (TZ III.7).
    pub fn project_volumes(&self) -> Vec<checks::VolumeLine> {
        checks::project_volumes(&self.elements, &self.estimate_items)
    }

    /// GPR da bor, smetada yo'q ishlar (TZ III.10).
    pub fn missing_works(&self) -> Vec<checks::MissingWork> {
        checks::missing_works(&self.tasks, &self.estimate_items)
    }

    /// PPR nazorati: ish boshlanishidan oldin tasdiqlanganmi
    /// (TZ VII.27, XIV.26).
    pub fn ppr_control(&self) -> Vec<checks::PprIssue> {
        checks::ppr_control(&self.tasks, &self.ppr_docs)
    }

    /// Yetkazib beruvchilar kartochkasi va tekshiruvi (TZ X.7, 16).
    pub fn supplier_cards(&self) -> Vec<checks::SupplierCard> {
        checks::supplier_cards(&self.suppliers, &self.purchases, self.today)
    }

    /// Maxsus jurnal yozuvlari (TZ IV.14).
    pub fn special_journal(&self, kind: checks::SpecialJournal) -> Vec<checks::JournalLine> {
        checks::special_journal(
            kind,
            &self.concrete_tests,
            &self.quality,
            &self.exec_docs,
            &self.geodesy_points,
            &self.tasks,
        )
    }

    /// Hujjat ortida qurilish yozuvi bormi (TZ IV.22).
    pub fn doc_evidence(&self) -> Vec<checks::DocEvidence> {
        checks::doc_evidence(
            &self.exec_docs,
            &self.journal,
            &self.timesheet,
            &self.stock_moves,
        )
    }

    /// Kechikish sabablari (TZ XVII.7).
    pub fn delay_causes(&self) -> Vec<checks::TaskDelay> {
        checks::delay_causes(&checks::DelayCtx {
            tasks: &self.tasks,
            links: &self.links,
            overdue: &self.progress.overdue,
            readiness: &self.readiness(),
            materials: &self.materials,
            blocks: &self.task_blocks(),
            required: &checks::required_docs(&self.tasks, &self.exec_docs, false),
            timesheet: &self.timesheet,
            workers: &self.workers,
            machines: &self.machines,
            machine_logs: &self.machine_logs,
            today: self.today,
        })
    }

    /// Risklar prognozi (TZ XVII.10).
    pub fn risk_forecast(&self) -> Vec<checks::RiskLine> {
        let plan = checks::purchase_plan(
            &self.materials,
            &self.stock(),
            &self.purchases,
            &self.requests,
            &self.material_norms,
            &self.tasks,
            self.today,
            PLAN_HORIZON,
        );
        let safety_open = self
            .safety
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    crate::domain::IssueStatus::Open | crate::domain::IssueStatus::InWork
                )
            })
            .count();
        checks::risk_forecast(
            self.progress.delay_days,
            &plan,
            &self.materials,
            &self.contract_changes,
            &self.quality_week(),
            safety_open,
            self.today,
        )
    }

    /// Haftalik sifat hisoboti (TZ XIV.37).
    pub fn quality_week(&self) -> checks::QualityWeek {
        checks::quality_week(&self.quality, self.today)
    }

    /// Direktor uchun kunlik xulosa (TZ V.24).
    pub fn day_report(&self) -> checks::DayReport {
        checks::day_report(
            self.today,
            self.running_today().len(),
            &self.journal,
            &self.timesheet,
            &self.machine_logs,
            &self.stock_moves,
            &self.materials,
            &self.safety,
            &self.quality,
            &self.exec_docs,
            self.day_close().len(),
        )
    }

    /// Kunlik hisobotning imzolanadigan matni (TZ V.28).
    ///
    /// Matn **bitta joyda** tuziladi: imzolashda ham, keyin tekshirishda
    /// ham shu funksiya chaqiriladi. Ichida faqat sanaladigan qiymatlar
    /// bor — matn o'zgarsa, demak kun ma'lumoti o'zgargan va imzo
    /// endi boshqa narsani tasdiqlaydi.
    pub fn day_report_text(&self) -> String {
        let d = self.day_report();
        format!(
            "{date} · {running}/{logged} · {workers} · {hours:.1} · {machines} · \
{material:.0} · {safety} · {quality} · {docs}",
            date = d.day,
            running = d.logged,
            logged = d.running,
            workers = d.workers,
            hours = d.hours,
            machines = d.machines,
            material = d.material_cost,
            safety = d.safety_new,
            quality = d.quality_new,
            docs = d.docs_signed,
        )
    }

    /// Kunlik hisobot hujjatining raqami.
    pub fn day_report_number(&self) -> String {
        format!("KUN-{}", self.today)
    }

    /// Kunlik hisobotni imzo daftariga yozadi (TZ V.28, V.32).
    pub fn sign_day_report(&mut self) {
        let number = self.day_report_number();
        let text = self.day_report_text();
        self.sign_document(&number, t("jr_day_report"), &text, "");
    }

    /// Bugungi hisobot imzolanganmi.
    pub fn day_report_signed(&self) -> bool {
        let number = self.day_report_number();
        self.sign_log.iter().any(|e| e.document == number)
    }

    /// Bugungi jurnal yozuvidan ariza takliflari (TZ V.17).
    pub fn journal_requests(&self) -> Vec<checks::JournalRequest> {
        checks::journal_requests(
            &self.journal,
            &self.material_norms,
            &self.tasks,
            &self.stock(),
            self.today,
        )
    }

    /// Loyiha hujjatlari versiyalari nazorati (TZ VII.30, 32).
    pub fn version_issues(&self) -> Vec<checks::VersionIssue> {
        checks::version_issues(&self.documents, &self.tasks, self.today)
    }

    /// Smetaning chuqur tekshiruvi (TZ III.9, 11, 13, 16).
    pub fn estimate_deep(&self) -> Vec<checks::DeepIssue> {
        checks::estimate_deep(&self.estimate_items, &self.tasks, &self.links, &self.quotes)
    }

    /// Smetadan faktgacha bo'lgan zanjir (TZ III.33).
    pub fn estimate_chain(&self) -> Vec<checks::ChainLine> {
        checks::estimate_chain(
            &self.estimate_items,
            &self.tasks,
            &self.requests,
            &self.purchases,
            &self.stock_moves,
            &self.materials,
            &self.task_costs(),
        )
    }

    /// Xavfsizlik bo'yicha ogohlantirishlar (TZ XV.36).
    pub fn safety_risks(&self) -> Vec<checks::SafetyRisk> {
        checks::safety_risks(
            &self.zones,
            &self.safety,
            &self.worker_safety(),
            &self.work_permits,
            self.today,
        )
    }

    /// Nuqson ehtimoli yuqori ishlar (TZ XIV.32).
    pub fn quality_risks(&self) -> Vec<checks::QualityRisk> {
        let consumption = checks::consumption(
            &self.material_norms,
            &self.tasks,
            &self.materials,
            &self.stock_moves,
        );
        let start = self.project().map(|p| p.start_date);
        let schedule = &self.schedule;
        let today = self.today;
        checks::quality_risks(
            &self.tasks,
            &self.quality,
            &consumption,
            &self.ppr_docs,
            &self.inspections,
            &self.progress.overdue,
            |id| match (start, schedule.get(id)) {
                (Some(s), Some(c)) => (today - (s + chrono::Duration::days(c.ef)))
                    .num_days()
                    .max(0),
                _ => 0,
            },
        )
    }

    /// Inventarizatsiya farqlari bo'yicha kamomad (TZ XI.26).
    pub fn shortages(&self) -> Vec<checks::ShortageLine> {
        checks::shortages(&self.inventories, &self.inventory_lines, &self.materials)
    }

    /// TZ XI: ombor qoldiqlari. Materiallar, harakatlar va rezervlardan hisoblanadi.
    pub fn stock(&self) -> Vec<checks::StockLine> {
        checks::stock_balances(
            &self.materials,
            &self.stock_moves,
            &self.reservations,
            self.today,
        )
    }

    /// Bitta ombor kesimidagi qoldiq (TZ XI.3).
    pub fn stock_in(&self, warehouse: Option<i64>) -> Vec<checks::StockLine> {
        checks::stock_balances_in(
            &self.materials,
            &self.stock_moves,
            &self.reservations,
            self.today,
            warehouse,
        )
    }

    /// Partiyalar bo'yicha qoldiq va FEFO navbati (TZ XI.9, XI.28).
    /// Yaqinda boshlanadigan ishlar uchun material yetarlimi (TZ XII.28).
    pub fn readiness(&self) -> Vec<crate::checks::Readiness> {
        crate::checks::readiness(
            &self.material_norms,
            &self.tasks,
            &self.stock(),
            self.today,
            READINESS_DAYS,
        )
    }

    /// Ishchilarning ruxsat va SIZ holati (TZ XV.4–9).
    pub fn worker_safety(&self) -> Vec<crate::checks::WorkerSafety> {
        crate::checks::worker_safety(
            &self.workers,
            &self.worker_permits,
            &self.ppe_issues,
            self.today,
        )
    }

    /// Normativ va haqiqiy sarf (TZ XI.14–15).
    pub fn consumption(&self) -> Vec<crate::checks::ConsumptionLine> {
        crate::checks::consumption(
            &self.material_norms,
            &self.tasks,
            &self.materials,
            &self.stock_moves,
        )
    }

    pub fn batch_lines(&self) -> Vec<checks::BatchLine> {
        checks::batch_balances(&self.batches, &self.stock_moves, self.today)
    }

    /// TZ III.31: smetaning pul ko'rinishidagi xulosasi.
    pub fn cost_summary(&self) -> checks::CostSummary {
        checks::cost_summary(&self.check_ctx())
    }

    /// Ekran filtrlaridan o'tgan nomuvofiqliklar.
    pub fn filtered_issues(&self, module: IssueModule) -> Vec<&Issue> {
        let q = self.issue_search.trim().to_lowercase();
        let mut out: Vec<&Issue> = self
            .issues
            .iter()
            .filter(|i| i.module == module)
            .filter(|i| self.issue_sev.map(|s| i.severity == s).unwrap_or(true))
            .filter(|i| self.issue_status.map(|s| i.status == s).unwrap_or(true))
            .filter(|i| {
                q.is_empty()
                    || i.title.to_lowercase().contains(&q)
                    || i.element.to_lowercase().contains(&q)
                    || i.code.to_lowercase().contains(&q)
                    || i.location.to_lowercase().contains(&q)
            })
            .collect();
        // Avval eng muhimi, keyin barqaror tartib uchun kod bo'yicha.
        out.sort_by(|a, b| {
            a.severity
                .rank()
                .cmp(&b.severity.rank())
                .then_with(|| a.code.cmp(&b.code))
        });
        out
    }

    /// TZ III.2: smetani fayldan import qilish.
    pub fn import_estimate(&mut self, path: &std::path::Path) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Estimate) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let Some(pid) = self.current else { return };
        let imported = match crate::import::estimate_from_file(path) {
            Ok(v) => v,
            Err(e) => {
                self.notify(format!("{}: {e}", t("import_failed")));
                return;
            }
        };

        let eid = self.db.insert_estimate(&Estimate {
            id: 0,
            project_id: pid,
            name: imported.name,
            currency: self.settings.default_currency.clone(),
            declared_total: imported.declared_total,
            overhead_pct: 0.0,
            profit_pct: 0.0,
            vat_pct: 0.0,
            added_at: String::new(),
        });
        if eid == 0 {
            self.notify(t("import_failed").to_string());
            return;
        }

        let n = self.db.insert_estimate_items(eid, &imported.items);
        self.current_estimate = Some(eid);
        self.reload_modules();

        let mut msg = format!("{n} {}", t("import_done"));
        if imported.skipped > 0 {
            msg.push_str(&format!(", {} {}", imported.skipped, t("import_skipped")));
        }
        self.notify(msg);
    }

    /// TZ I.3: PPR ni grafik bilan solishtirish.
    pub fn run_ppr_check(&mut self) {
        let Some(pid) = self.current else { return };
        if self.tasks.is_empty() {
            self.notify(t("no_tasks").to_string());
            return;
        }
        let found = checks::check_ppr(&checks::PprCtx {
            project_id: pid,
            tasks: &self.tasks,
            links: &self.links,
            schedule: &self.schedule,
            docs: &self.ppr_docs,
            origin: self.origin(),
            today: self.today,
            avail_workers: self.settings.avail_workers,
            avail_machines: self.settings.avail_machines,
            norms: &self.norms,
        });
        let n = found.len();
        self.db.replace_auto_issues(pid, IssueModule::Ppr, &found);
        self.reload_modules();
        self.notify(if n == 0 {
            t("no_issues").to_string()
        } else {
            format!("{n} {}", t("issues_found"))
        });
    }

    /// TZ V: jurnaldagi haqiqiy hajmlarni GPR ga o'tkazadi.
    /// Hajmi ko'rsatilmagan ishda foizni hisoblab bo'lmaydi — bunday ishlar
    /// tegilmaydi va soni foydalanuvchiga aytiladi.
    pub fn apply_journal_to_tasks(&mut self) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Journal) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        use std::collections::HashMap as Map;
        let mut done: Map<i64, f64> = Map::new();
        let mut first: Map<i64, NaiveDate> = Map::new();
        let mut last: Map<i64, NaiveDate> = Map::new();
        for e in &self.journal {
            let Some(tid) = e.task_id else { continue };
            *done.entry(tid).or_default() += e.volume;
            first
                .entry(tid)
                .and_modify(|d| {
                    if e.date < *d {
                        *d = e.date;
                    }
                })
                .or_insert(e.date);
            last.entry(tid)
                .and_modify(|d| {
                    if e.date > *d {
                        *d = e.date;
                    }
                })
                .or_insert(e.date);
        }

        let mut updated = 0;
        let mut no_volume = 0;
        for task in self.tasks.iter_mut() {
            let Some(volume_done) = done.get(&task.id) else {
                continue;
            };
            if task.volume <= 0.0 {
                no_volume += 1;
                continue;
            }
            let pct = (volume_done / task.volume * 100.0).clamp(0.0, 100.0);
            let mut changed = (pct - task.progress).abs() > 0.01;
            task.progress = pct;
            if task.fact_start.is_none() {
                if let Some(d) = first.get(&task.id) {
                    task.fact_start = Some(*d);
                    changed = true;
                }
            }
            if pct >= 99.999 && task.fact_end.is_none() {
                if let Some(d) = last.get(&task.id) {
                    task.fact_end = Some(*d);
                    changed = true;
                }
            }
            if changed {
                let _ = self.db.update_task(task);
                updated += 1;
            }
        }
        self.recompute();

        let mut msg = format!("{updated} {}", t("journal_applied"));
        if no_volume > 0 {
            msg.push_str(&format!(", {no_volume} {}", t("journal_no_volume")));
        }
        self.notify(msg);
    }

    /// Modul ekrani ochilganda tekshiruvni bir marta avtomatik ishga tushiradi.
    /// Saqlangan natija bo'lsa yoki tekshiradigan ma'lumot yo'q bo'lsa — tegmaydi.
    pub fn auto_check(&mut self, module: IssueModule) {
        let (ran, has_input) = match module {
            IssueModule::Project => (&mut self.auto_check_project, !self.elements.is_empty()),
            IssueModule::Estimate => (
                &mut self.auto_check_estimate,
                !self.estimate_items.is_empty(),
            ),
            IssueModule::Ppr => (&mut self.auto_check_ppr, !self.tasks.is_empty()),
            _ => return,
        };
        if *ran || !has_input {
            return;
        }
        // Bayroqni oldindan qo'yamiz: natija bo'sh chiqsa ham qayta ishlamasin.
        *ran = true;
        if self.issues.iter().any(|i| i.module == module) {
            return; // saqlangan natija bor — foydalanuvchi holatini buzmaymiz
        }
        match module {
            IssueModule::Project => self.run_project_check(),
            IssueModule::Estimate => self.run_estimate_check(),
            IssueModule::Ppr => self.run_ppr_check(),
            _ => {}
        }
    }

    pub fn save_issue(&mut self, issue: Issue) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.writable() {
            return;
        }
        if self.db.update_issue(&issue) {
            if let Some(slot) = self.issues.iter_mut().find(|x| x.id == issue.id) {
                *slot = issue;
            }
        } else {
            self.notify(t("save_failed").to_string());
        }
    }

    pub fn save_norm(&mut self, key: &str, norm: Norm) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::AiCheck) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        if self.db.save_norm(key, &norm) {
            self.norms.insert(key.to_string(), norm);
            self.notify(t("norm_saved").to_string());
        } else {
            self.notify(t("save_failed").to_string());
        }
    }

    /// Barcha ishlarning mahkamlanishini bekor qiladi va grafikni CPM bo'yicha
    /// qayta tiklaydi. Polosani tasodifan sudrab yuborish ishni mahkamlab
    /// qo'yadi — bunday ishlarni birma-bir qidirmaslik uchun kerak.
    pub fn unpin_all(&mut self) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let ids: Vec<i64> = self
            .tasks
            .iter()
            .filter(|t| t.pinned)
            .map(|t| t.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            if let Some(task) = self.tasks.iter_mut().find(|t| t.id == *id) {
                task.pinned = false;
                let _ = self.db.update_task(task);
            }
        }
        self.recompute();
        self.notify(format!("{} {}", ids.len(), t("unpinned_msg")));
    }

    /// Ishni ±N kunga suradi (klaviatura bilan boshqarish uchun).
    /// Surilgan ish mahkamlanadi — aks holda CPM uni qaytarib qo'yardi.
    pub fn nudge_task(&mut self, id: i64, days: i64) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let Some(mut task) = self.task(id).cloned() else {
            return;
        };
        task.plan_start += chrono::Duration::days(days);
        task.pinned = true;
        self.save_task(task);
    }

    /// Ishni ro'yxatda yuqoriga yoki pastga suradi.
    pub fn reorder_task(&mut self, id: i64, up: bool) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let ids: Vec<i64> = self.tasks.iter().map(|t| t.id).collect();
        let Some(i) = ids.iter().position(|x| *x == id) else {
            return;
        };
        let j = if up {
            i.checked_sub(1)
        } else {
            (i + 1 < ids.len()).then_some(i + 1)
        };
        let Some(j) = j else { return };
        let mut order = ids.clone();
        order.swap(i, j);
        let _ = self.db.set_task_orders(&order);
        self.reload_project_data();
        self.selected_task = Some(id);
    }

    /// Gantt ro'yxatida ishni ko'rinadigan qilib aylantiradi.
    pub fn scroll_to_task(&mut self, id: i64) {
        if let Some(idx) = self.visible_tasks().iter().position(|t| t.id == id) {
            self.row_offset = (idx as f32 - 5.0).max(0.0);
        }
    }

    /// Пересчет сетевого графика и показателей план/факт.
    pub fn recompute(&mut self) {
        let origin = self.origin();
        self.schedule = cpm::compute(&self.tasks, &self.links, origin);
        self.progress = cpm::progress(&self.tasks, &self.schedule, origin, self.today);

        // Ранние даты CPM становятся плановыми датами работ — так график
        // автоматически пересчитывается при изменении длительностей и связей.
        for t in self.tasks.iter_mut() {
            // Mahkamlangan ishda plan_start — foydalanuvchi qo'ygan chegara.
            // Uni CPM natijasi bilan almashtirsak, ish predshestvennik ortidan
            // oldinga surilib qolar va predshestvennik qaytganda ham qaytmasdi.
            if t.pinned {
                continue;
            }
            if let Some(c) = self.schedule.get(t.id) {
                let new_start = origin + chrono::Duration::days(c.es);
                if new_start != t.plan_start {
                    t.plan_start = new_start;
                    let _ = self.db.update_task(t);
                }
            }
        }
    }

    /// Obyektni o'chiradi: bog'liq ma'lumot sxemadagi CASCADE bilan ketadi.
    pub fn delete_project(&mut self, id: i64) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Passport) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        if let Err(e) = self.db.delete_project(id) {
            self.notify(format!("{}: {e}", t("save_failed")));
            return;
        }
        if self.current == Some(id) {
            self.current = None;
        }
        self.reload_projects();
        if self.current.is_none() {
            self.current = self.projects.first().map(|x| x.id);
        }
        self.reload_project_data();
        self.notify(t("object_deleted").to_string());
    }

    pub fn save_task(&mut self, task: Task) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        if let Err(e) = self.db.update_task(&task) {
            self.notify(format!("{}: {e}", t("err_save_task")));
            return;
        }
        if let Some(slot) = self.tasks.iter_mut().find(|x| x.id == task.id) {
            *slot = task;
        }
        self.recompute();
    }

    pub fn add_task(&mut self) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let Some(pid) = self.current else { return };
        let task = Task {
            id: 0,
            project_id: pid,
            wbs: format!("{}", self.tasks.len() + 1),
            name: t("task_new_name").into(),
            section: Section::None,
            responsible: String::new(),
            duration: self.settings.default_task_days,
            plan_start: self.origin(),
            fact_start: None,
            fact_end: None,
            progress: 0.0,
            pinned: false,
            volume: 0.0,
            unit: String::new(),
        };
        match self.db.insert_task(&task) {
            Ok(id) => {
                self.selected_task = Some(id);
                self.reload_project_data();
            }
            Err(e) => self.notify(format!("{}: {e}", t("err_add_task"))),
        }
    }

    pub fn delete_task(&mut self, id: i64) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        if let Err(e) = self.db.delete_task(id) {
            self.notify(format!("{}: {e}", t("err_del_task")));
            return;
        }
        if self.selected_task == Some(id) {
            self.selected_task = None;
        }
        self.reload_project_data();
        self.notify(t("task_deleted").to_string());
    }

    pub fn add_link(&mut self, pred: i64, succ: i64, kind: LinkType, lag: i64) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        if pred == succ {
            self.notify(t("link_self").into());
            return;
        }
        let had_cycles = !self.schedule.cycles.is_empty();
        let l = Link {
            id: 0,
            pred,
            succ,
            kind,
            lag,
        };
        let id = match self.db.insert_link(&l) {
            Ok(id) => id,
            Err(e) => {
                self.notify(format!("{}: {e}", t("err_add_link")));
                return;
            }
        };
        self.reload_project_data();

        // Yangi bog'lanish sikl hosil qilgan bo'lsa, uni saqlab qo'ymaymiz:
        // sikldagi ishlar uchun CPM natijasi ishonchsiz bo'lardi.
        if !had_cycles && !self.schedule.cycles.is_empty() {
            let _ = self.db.delete_link(id);
            self.reload_project_data();
            self.notify(t("link_cycle").into());
        } else {
            self.notify(t("link_added").into());
        }
    }

    pub fn delete_link(&mut self, id: i64) {
        // Rol cheklovi: bu amal yozuvchi.
        if !self.can_edit(Screen::Gantt) {
            self.notify(format!("{} — {}", t("role_readonly"), self.role().label()));
            return;
        }
        let _ = self.db.delete_link(id);
        self.reload_project_data();
    }

    pub fn task(&self, id: i64) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn task_name(&self, id: i64) -> String {
        self.task(id).map(|t| t.name.clone()).unwrap_or_default()
    }

    /// Работы, прошедшие фильтры экрана ГПР.
    pub fn visible_tasks(&self) -> Vec<&Task> {
        let q = self.search.trim().to_lowercase();
        self.tasks
            .iter()
            .filter(|t| {
                if self.show_critical_only && !self.schedule.is_critical(t.id) {
                    return false;
                }
                if self.filter_overdue && !self.progress.overdue.contains(&t.id) {
                    return false;
                }
                if self.filter_section != Section::None && t.section != self.filter_section {
                    return false;
                }
                if !q.is_empty()
                    && !t.name.to_lowercase().contains(&q)
                    && !t.responsible.to_lowercase().contains(&q)
                    && !t.wbs.to_lowercase().contains(&q)
                {
                    return false;
                }
                true
            })
            .collect()
    }

    /// Sozlamani saqlaydi va global holatga qo'llaydi.
    pub fn save_settings(&mut self) {
        self.settings.apply_globals();
        self.settings.save(&self.db);
        self.restyle = true;
    }

    pub fn notify(&mut self, msg: String) {
        self.toast = Some((msg, 4.0));
    }
}
