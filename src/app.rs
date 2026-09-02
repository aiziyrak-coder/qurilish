//! Состояние приложения и связка модулей с хранилищем.

use crate::checks::{self, Norm};
use crate::cpm::{self, Progress, Schedule};
use crate::db::Db;
use crate::domain::{
    Batch, Block, DayKind, Deal, Document, Element, ElementLink, Estimate, EstimateItem, ExecDoc,
    Inventory, InventoryLine, Issue, IssueModule, IssueStatus, JournalEntry, Machine, MachineLog,
    Material, Payment, PprDoc, Purchase, QualityCheck, Request, Reservation, SafetyEvent, Severity,
    Shift, StockMove, TimesheetEntry, Unit, Warehouse, Worker,
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
    Copilot,
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
            Screen::Copilot => t("screen_copilot"),
            Screen::Sales => t("screen_sales"),
            Screen::Deals => t("screen_deals"),
            Screen::Settings => t("screen_settings"),
        }
    }

    /// TZ dagi bo'lim raqami. Umumiy ko'rinish va sozlamalar TZ moduli emas.
    pub fn numeral(self) -> &'static str {
        match self {
            Screen::Dashboard | Screen::Settings => "",
            Screen::Passport => "I.1",
            Screen::Gantt => "I.2",
            Screen::Ppr => "I.3",
            Screen::AiCheck => "II",
            Screen::Estimate => "III",
            Screen::ExecDocs => "IV",
            Screen::Journal => "V",
            Screen::Foreman => "VI",
            Screen::TechSupervision => "VII",
            Screen::Client => "VIII",
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
            | Screen::Settings => Readiness::Ready,
        }
    }
}

/// Navigatsiya guruhlari — TZ ning mantiqiy bloklari.
pub const NAV_GROUPS: &[(&str, &[Screen])] = &[
    (
        "nav_object",
        &[
            Screen::Dashboard,
            Screen::Passport,
            Screen::Gantt,
            Screen::Ppr,
        ],
    ),
    ("nav_ai", &[Screen::AiCheck, Screen::Estimate]),
    (
        "nav_exec",
        &[
            Screen::ExecDocs,
            Screen::Journal,
            Screen::Quality,
            Screen::Safety,
        ],
    ),
    (
        "nav_cabinets",
        &[Screen::Foreman, Screen::TechSupervision, Screen::Client],
    ),
    (
        "nav_supply",
        &[
            Screen::Requests,
            Screen::Purchases,
            Screen::Warehouse,
            Screen::Materials,
        ],
    ),
    ("nav_sales", &[Screen::Sales, Screen::Deals]),
    ("nav_resources", &[Screen::Timesheet, Screen::Machines]),
    ("nav_analytics", &[Screen::Analytics, Screen::Copilot]),
    ("nav_system", &[Screen::Settings]),
];

/// AI tekshiruv ekranidagi ochiq bo'lim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckTab {
    Issues,
    Action,
    Graph,
    Elements,
    Relations,
    Norms,
}

impl CheckTab {
    pub const ALL: [CheckTab; 6] = [
        CheckTab::Issues,
        CheckTab::Action,
        CheckTab::Graph,
        CheckTab::Elements,
        CheckTab::Relations,
        CheckTab::Norms,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CheckTab::Issues => t("tab_issues"),
            CheckTab::Action => t("tab_action"),
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
    /// Tabelda ko'rsatilayotgan hafta boshi (dushanba).
    pub timesheet_week: Option<chrono::NaiveDate>,
    /// Ilova foydalanuvchilari va joriy tanlangani (TZ VI–VIII).
    /// Til modeli sozlamasi. Sukut bo'yicha o'chiq — ilova lokal qoladi.
    pub llm: crate::llm::Config,
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
            timesheet_week: None,
            llm: crate::llm::Config::default(),
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
            if let Some(p) = app.projects.first() {
                app.select_project(p.id);
            }
        }
        // Til modeli sozlamasi. Yig'ilishda tarmoq qismi bo'lmasa — doim o'chiq.
        app.llm = crate::llm::Config {
            enabled: cfg!(feature = "llm")
                && app.db.get_setting("llm_enabled").as_deref() == Some("1"),
            endpoint: app.db.get_setting("llm_endpoint").unwrap_or_default(),
            model: app.db.get_setting("llm_model").unwrap_or_default(),
            api_key: app.db.get_setting("llm_key").unwrap_or_default(),
        };

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

    fn clear_modules(&mut self) {
        self.issues.clear();
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
        self.documents = self.db.documents(id);
        self.ppr_docs = self.db.ppr_docs(id);
        self.exec_docs = self.db.exec_docs(id);
        self.journal = self.db.journal(id);
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
        self.blocks = self.db.blocks(id);
        self.units = self.db.units(id);
        self.deals = self.db.deals(id);
        self.payments = self.db.payments(id);
        if self.sales_block.is_none() {
            self.sales_block = self.blocks.first().map(|b| b.id);
        }
        self.elements = self.db.elements(id);
        self.element_links = self.db.element_links(id);
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
                    driver: String::new(),
                    route: String::new(),
                    odo_start: 0.0,
                    odo_end: 0.0,
                    trips: 0,
                    cargo: 0.0,
                    note: String::new(),
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
