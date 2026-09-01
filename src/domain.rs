//! II–XVI modullarning domen turlari.
//!
//! Har bir sanoq turida bazada saqlanadigan barqaror `code()` va ekranga
//! chiqadigan tarjima qilingan `label()` bor — til almashtirish ma'lumotni buzmaydi.

// IV-XVI modullar turlari tayyor, ekranlari keyingi bosqichda ulanadi.
#![allow(dead_code)]

use crate::i18n::t;
use chrono::NaiveDate;

/// Sanoq turlari uchun umumiy shakl: kod, tarjima, ro'yxat.
macro_rules! enum_kind {
    ($name:ident { $($v:ident => $code:literal, $key:literal;)+ }) => {
        // `Ord` — sanoq turlarini tartiblash va dedup qilish uchun
        // (masalan IFC importidagi bog'lanishlar ro'yxati).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($v,)+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v,)+];

            pub fn code(self) -> &'static str {
                match self { $($name::$v => $code,)+ }
            }

            pub fn label(self) -> &'static str {
                match self { $($name::$v => t($key),)+ }
            }

            pub fn parse(s: &str) -> $name {
                match s {
                    $($code => $name::$v,)+
                    _ => $name::ALL[0],
                }
            }
        }
    };
}

// ---------- II–III. Tekshiruv natijalari ----------

enum_kind!(Severity {
    Critical => "critical", "sev_critical";
    Major    => "major",    "sev_major";
    Warning  => "warning",  "sev_warning";
    Info     => "info",     "sev_info";
    Ok       => "ok",       "sev_ok";
});

impl Severity {
    pub fn color(self) -> egui::Color32 {
        match self {
            Severity::Critical => egui::Color32::from_rgb(198, 40, 40),
            Severity::Major => egui::Color32::from_rgb(214, 110, 30),
            Severity::Warning => egui::Color32::from_rgb(190, 150, 20),
            Severity::Info => egui::Color32::from_rgb(50, 110, 200),
            Severity::Ok => egui::Color32::from_rgb(30, 140, 80),
        }
    }

    /// Muhimlik bo'yicha tartib — hisobotlarda saralash uchun.
    pub fn rank(self) -> u8 {
        match self {
            Severity::Critical => 0,
            Severity::Major => 1,
            Severity::Warning => 2,
            Severity::Info => 3,
            Severity::Ok => 4,
        }
    }
}

enum_kind!(IssueStatus {
    Open     => "open",     "ist_open";
    InWork   => "in_work",  "ist_in_work";
    Fixed    => "fixed",    "ist_fixed";
    Rejected => "rejected", "ist_rejected";
});

enum_kind!(IssueModule {
    Project  => "project",  "imod_project";
    Estimate => "estimate", "imod_estimate";
    Ppr      => "ppr",      "imod_ppr";
    Quality  => "quality",  "imod_quality";
    Safety   => "safety",   "imod_safety";
});

/// Loyiha yoki smeta tekshiruvida topilgan nomuvofiqlik (TZ II.16, III).
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: i64,
    pub project_id: i64,
    pub module: IssueModule,
    pub section: crate::model::Section,
    /// Ichki raqam: VK-00452 ko'rinishida.
    pub code: String,
    pub sheet: String,
    pub location: String,
    pub element: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    /// Normativ havolasi. TZ II.17: AI normativni o'ylab topmasligi shart,
    /// shuning uchun hujjat, band va matn alohida saqlanadi.
    pub norm_doc: String,
    pub norm_clause: String,
    pub norm_text: String,
    pub recommendation: String,
    pub responsible: String,
    /// Bartaraf etish muddati (TZ II.19: «nima, kim va qachongacha»).
    pub deadline: Option<NaiveDate>,
    pub status: IssueStatus,
    /// Qoida avtomatik topganmi yoki qo'lda kiritilganmi.
    pub auto: bool,
    pub created_at: String,
}

// ---------- II. Loyiha bilimlar grafi ----------

enum_kind!(ElementKind {
    Room     => "room",     "ek_room";
    Wall     => "wall",     "ek_wall";
    Door     => "door",     "ek_door";
    Window   => "window",   "ek_window";
    Column   => "column",   "ek_column";
    Beam     => "beam",     "ek_beam";
    Slab     => "slab",     "ek_slab";
    Opening  => "opening",  "ek_opening";
    Pipe     => "pipe",     "ek_pipe";
    Duct     => "duct",     "ek_duct";
    Cable    => "cable",    "ek_cable";
    Device   => "device",   "ek_device";
    Other    => "other",    "ek_other";
});

/// Loyihaning bitta elementi — bilimlar grafining tuguni (TZ II.18).
#[derive(Debug, Clone)]
pub struct Element {
    pub id: i64,
    pub project_id: i64,
    pub section: crate::model::Section,
    pub kind: ElementKind,
    /// Chizmadagi marka: OK-12, K1-17, KJ-3.
    pub mark: String,
    pub room: String,
    pub axis: String,
    pub level: String,
    /// Asosiy o'lchov (diametr, kesim, kenglik) va qiymati.
    pub size: f64,
    pub unit: String,
    /// Qo'shimcha parametrlar: uklon, quvvat, miqdor.
    pub value: f64,
    pub value_name: String,
    pub sheet: String,
    pub note: String,
}

enum_kind!(Relation {
    Contains  => "contains",  "rel_contains";
    Serves    => "serves",    "rel_serves";
    Crosses   => "crosses",   "rel_crosses";
    SupportedBy => "supported_by", "rel_supported_by";
    PoweredBy => "powered_by", "rel_powered_by";
    Related   => "related",   "rel_related";
});

/// Elementlar orasidagi bog'lanish — grafning qirrasi.
#[derive(Debug, Clone)]
pub struct ElementLink {
    pub id: i64,
    pub from_el: i64,
    pub to_el: i64,
    pub relation: Relation,
}

/// Loyiha hujjati (TZ II.1: PDF, DWG, IFC, XLSX va boshqalar).
#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub project_id: i64,
    pub section: crate::model::Section,
    pub name: String,
    pub format: String,
    pub path: String,
    pub sheets: i64,
    pub added_at: String,
}

// ---------- III. Smeta ----------

/// Smeta (TZ III).
#[derive(Debug, Clone)]
pub struct Estimate {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub currency: String,
    /// Hujjatda ko'rsatilgan yakuniy summa — hisoblangani bilan solishtiriladi.
    pub declared_total: f64,
    pub added_at: String,
}

/// Smeta pozitsiyasi.
#[derive(Debug, Clone)]
pub struct EstimateItem {
    pub id: i64,
    pub estimate_id: i64,
    pub pos: i64,
    pub section: crate::model::Section,
    /// Rasenka kodi.
    pub code: String,
    pub name: String,
    pub unit: String,
    pub qty: f64,
    pub price: f64,
    /// Hujjatdagi summa — qty*price bilan solishtiriladi.
    pub cost: f64,
    pub note: String,
}

impl EstimateItem {
    pub fn computed(&self) -> f64 {
        self.qty * self.price
    }
}

// ---------- I.3. PPR va texnologik kartalar ----------

enum_kind!(PprKind {
    Ppr         => "ppr",     "ppr_kind_ppr";
    TechCard    => "tech",    "ppr_kind_tech";
    QualityCard => "quality", "ppr_kind_quality";
    SafetyCard  => "safety",  "ppr_kind_safety";
});

/// PPR, texnologik karta, sifat yoki xavfsizlik kartasi (TZ I.3).
#[derive(Debug, Clone)]
pub struct PprDoc {
    pub id: i64,
    pub project_id: i64,
    pub kind: PprKind,
    pub number: String,
    pub name: String,
    pub section: crate::model::Section,
    /// Qaysi GPR ishiga tegishli. Bog'lanmagan karta tekshiruvda belgilanadi.
    pub task_id: Option<i64>,
    /// Karta bo'yicha talab qilinadigan resurs — yetarlilik shu bo'yicha hisoblanadi.
    pub workers: i64,
    pub machines: i64,
    pub path: String,
    pub approved: bool,
    /// Kartani ishlab chiqqan mutaxassis yoki tashkilot.
    pub author: String,
    /// Tasdiqlangan sana — bo'sh bo'lsa, karta hali tasdiqlanmagan.
    pub approved_at: Option<NaiveDate>,
    pub note: String,
}

// ---------- IV. Ijro hujjatlari ----------

enum_kind!(ExecDocKind {
    Hidden     => "hidden",     "edk_hidden";
    Acceptance => "acceptance", "edk_acceptance";
    Test       => "test",       "edk_test";
    Passport   => "passport",   "edk_passport";
    Scheme     => "scheme",     "edk_scheme";
    Other      => "other",      "edk_other";
});

enum_kind!(ExecDocStatus {
    Draft    => "draft",    "eds_draft";
    OnReview => "review",   "eds_review";
    Signed   => "signed",   "eds_signed";
    Rejected => "rejected", "eds_rejected";
});

/// Ijro hujjati (TZ IV).
#[derive(Debug, Clone)]
pub struct ExecDoc {
    pub id: i64,
    pub project_id: i64,
    pub kind: ExecDocKind,
    pub number: String,
    pub name: String,
    pub date: NaiveDate,
    /// Qaysi GPR ishiga tegishli.
    pub task_id: Option<i64>,
    pub status: ExecDocStatus,
    pub responsible: String,
    pub note: String,
}

// ---------- V. Kundalik ish jurnali ----------

/// Jurnal yozuvi (TZ V).
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub id: i64,
    pub project_id: i64,
    pub date: NaiveDate,
    pub author: String,
    pub weather: String,
    pub temperature: f64,
    pub workers: i64,
    pub machines: i64,
    pub task_id: Option<i64>,
    pub volume: f64,
    pub unit: String,
    pub text: String,
    pub remarks: String,
    /// Biriktirilgan fotolar — fayl yo'llari, nuqtali vergul bilan ajratilgan.
    /// Fayllar ko'chirilmaydi (TZ V: fotofiksatsiya), faqat yo'li saqlanadi.
    pub photos: String,
}

// ---------- IX. Arizalar ----------

enum_kind!(RequestKind {
    Material => "material", "rk_material";
    Machine  => "machine",  "rk_machine";
    Labor    => "labor",    "rk_labor";
    Document => "document", "rk_document";
    Other    => "other",    "rk_other";
});

enum_kind!(RequestStatus {
    New       => "new",       "rs_new";
    Approved  => "approved",  "rs_approved";
    InPurchase => "purchase", "rs_purchase";
    Delivered => "delivered", "rs_delivered";
    Closed    => "closed",    "rs_closed";
    Rejected  => "rejected",  "rs_rejected";
});

enum_kind!(Priority {
    Low    => "low",    "pr_low";
    Normal => "normal", "pr_normal";
    High   => "high",   "pr_high";
    Urgent => "urgent", "pr_urgent";
});

/// Ariza (TZ IX).
#[derive(Debug, Clone)]
pub struct Request {
    pub id: i64,
    pub project_id: i64,
    pub number: String,
    pub date: NaiveDate,
    pub kind: RequestKind,
    pub title: String,
    pub material_id: Option<i64>,
    pub qty: f64,
    pub unit: String,
    pub requester: String,
    pub need_date: NaiveDate,
    pub priority: Priority,
    pub status: RequestStatus,
    pub task_id: Option<i64>,
    pub note: String,
}

// ---------- X. Xaridlar ----------

enum_kind!(PurchaseStatus {
    Draft     => "draft",     "ps_draft";
    Ordered   => "ordered",   "ps_ordered";
    Paid      => "paid",      "ps_paid";
    Delivered => "delivered", "ps_delivered";
    Closed    => "closed",    "ps_closed";
});

/// Xarid (TZ X).
#[derive(Debug, Clone)]
pub struct Purchase {
    pub id: i64,
    pub project_id: i64,
    pub request_id: Option<i64>,
    pub number: String,
    pub date: NaiveDate,
    pub supplier: String,
    pub title: String,
    pub qty: f64,
    pub unit: String,
    pub price: f64,
    pub currency: String,
    pub delivery_date: NaiveDate,
    pub status: PurchaseStatus,
    pub note: String,
}

impl Purchase {
    pub fn amount(&self) -> f64 {
        self.qty * self.price
    }
}

// ---------- XI–XII. Ombor va materiallar ----------

/// Material (TZ XII).
#[derive(Debug, Clone)]
pub struct Material {
    pub id: i64,
    pub project_id: i64,
    pub code: String,
    pub name: String,
    pub unit: String,
    pub section: crate::model::Section,
    /// Texnik tavsif: marka, sinf, GOST.
    pub spec: String,
    pub cert_no: String,
    /// Sertifikat yoki yaroqlilik muddati — FEFO nazorati uchun (TZ XI).
    pub cert_until: Option<NaiveDate>,
    /// Minimal zaxira: undan pastda ogohlantirish beriladi.
    pub min_stock: f64,
    pub price: f64,
    pub note: String,
}

enum_kind!(MoveKind {
    In       => "in",       "mk_in";
    Out      => "out",      "mk_out";
    WriteOff => "writeoff", "mk_writeoff";
});

/// Ombor harakati (TZ XI).
#[derive(Debug, Clone)]
pub struct StockMove {
    pub id: i64,
    pub project_id: i64,
    pub material_id: i64,
    pub date: NaiveDate,
    pub kind: MoveKind,
    pub qty: f64,
    pub price: f64,
    pub document: String,
    pub counterparty: String,
    pub task_id: Option<i64>,
    pub note: String,
}

// ---------- XIX–XX. Sotuv ----------

/// Blok (podez, kirish). Shaxmatka har blok uchun alohida quriladi.
#[derive(Debug, Clone)]
pub struct Block {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    /// Qavatlar soni — shaxmatkaning balandligi.
    pub floors: i64,
    /// Yerto'la va nolinchi qavatlar uchun: shaxmatka qaysi qavatdan boshlanadi.
    pub first_floor: i64,
    pub note: String,
}

enum_kind!(UnitKind {
    Flat       => "flat",    "uk_flat";
    Commercial => "comm",    "uk_comm";
    Office     => "office",  "uk_office";
    Parking    => "parking", "uk_parking";
    Storage    => "storage", "uk_storage";
});

enum_kind!(UnitStatus {
    Free        => "free",     "us_free";
    Reserved    => "reserved", "us_reserved";
    Contract    => "contract", "us_contract";
    Sold        => "sold",     "us_sold";
    Unavailable => "off",      "us_off";
});

/// Sotuv birligi: kvartira, tijorat joyi, ofis, avtoturargoh yoki ombor.
#[derive(Debug, Clone)]
pub struct Unit {
    pub id: i64,
    pub project_id: i64,
    pub block_id: i64,
    pub number: String,
    pub floor: i64,
    /// Qavatdagi tartib raqami — shaxmatkada ustunni belgilaydi.
    pub position: i64,
    pub kind: UnitKind,
    pub rooms: i64,
    /// Umumiy maydon, m².
    pub area: f64,
    /// Yashash maydoni, m².
    pub area_living: f64,
    pub price_per_m2: f64,
    pub status: UnitStatus,
    /// Planirovka turi yoki chizma fayli yo'li.
    pub layout: String,
    pub note: String,
}

impl UnitKind {
    /// Shaxmatka katagi uchun qisqa belgi — joy tor.
    pub fn short_key(self) -> &'static str {
        match self {
            UnitKind::Flat => "uks_flat",
            UnitKind::Commercial => "uks_comm",
            UnitKind::Office => "uks_office",
            UnitKind::Parking => "uks_parking",
            UnitKind::Storage => "uks_storage",
        }
    }
}

impl Unit {
    /// Kvartiraning to'liq narxi. Shartnomadagi narx bundan farq qilishi mumkin —
    /// chegirma yoki kelishuv bo'lsa, u shartnomada saqlanadi.
    pub fn price(&self) -> f64 {
        self.area * self.price_per_m2
    }
}

enum_kind!(PayKind {
    Cash        => "cash",        "pk_cash";
    Installment => "installment", "pk_installment";
    Credit      => "credit",      "pk_credit";
    Subsidy     => "subsidy",     "pk_subsidy";
    Barter      => "barter",      "pk_barter";
    Mixed       => "mixed",       "pk_mixed";
});

enum_kind!(DealStatus {
    Reserved  => "reserved",  "ds_reserved";
    Signed    => "signed",    "ds_signed";
    Completed => "completed", "ds_completed";
    Cancelled => "cancelled", "ds_cancelled";
});

/// Sotuv shartnomasi (yoki band qilish).
#[derive(Debug, Clone)]
pub struct Deal {
    pub id: i64,
    pub project_id: i64,
    pub unit_id: i64,
    pub number: String,
    pub date: NaiveDate,
    pub client: String,
    pub phone: String,
    /// Mijoz hujjati raqami — shartnomaga havola uchun.
    pub client_doc: String,
    pub pay_kind: PayKind,
    /// Kelishilgan narx (chegirmagacha).
    pub price: f64,
    pub discount: f64,
    /// Boshlang'ich to'lov.
    pub prepayment: f64,
    /// Muddatli to'lov oylari soni; 0 — bo'lib to'lash yo'q.
    pub months: i64,
    pub status: DealStatus,
    pub manager: String,
    pub note: String,
}

impl Deal {
    /// To'lanishi kerak bo'lgan yakuniy summa.
    pub fn total(&self) -> f64 {
        (self.price - self.discount).max(0.0)
    }
}

/// To'lov grafigining bir qatori: reja va fakt bir yozuvda.
#[derive(Debug, Clone)]
pub struct Payment {
    pub id: i64,
    pub project_id: i64,
    pub deal_id: i64,
    /// Reja bo'yicha to'lov sanasi.
    pub due: NaiveDate,
    pub planned: f64,
    pub paid: f64,
    /// Haqiqatda to'langan sana.
    pub paid_date: Option<NaiveDate>,
    pub kind: PayKind,
    pub document: String,
    pub note: String,
}

// ---------- XIII. Tabel ----------

/// Ishchi (TZ XIII).
#[derive(Debug, Clone)]
pub struct Worker {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub position: String,
    pub org: String,
    pub hourly_rate: f64,
    pub active: bool,
}

/// Tabel yozuvi: bir ishchi, bir kun.
#[derive(Debug, Clone)]
pub struct TimesheetEntry {
    pub id: i64,
    pub project_id: i64,
    pub worker_id: i64,
    pub date: NaiveDate,
    pub hours: f64,
    pub task_id: Option<i64>,
    pub note: String,
}

// ---------- XIV. Sifat ----------

enum_kind!(QualityKind {
    Input       => "input",       "qk_input";
    Operational => "operational", "qk_operational";
    Acceptance  => "acceptance",  "qk_acceptance";
});

enum_kind!(QualityResult {
    Pass        => "pass",        "qr_pass";
    Conditional => "conditional", "qr_conditional";
    Fail        => "fail",        "qr_fail";
});

/// Sifat nazorati yozuvi (TZ XIV).
#[derive(Debug, Clone)]
pub struct QualityCheck {
    pub id: i64,
    pub project_id: i64,
    pub kind: QualityKind,
    pub date: NaiveDate,
    pub task_id: Option<i64>,
    pub material_id: Option<i64>,
    pub subject: String,
    pub inspector: String,
    pub result: QualityResult,
    pub defect: String,
    pub deadline: Option<NaiveDate>,
    pub note: String,
}

// ---------- XV. Xavfsizlik ----------

enum_kind!(SafetyKind {
    Violation  => "violation",  "sk_violation";
    NearMiss   => "near_miss",  "sk_near_miss";
    Incident   => "incident",   "sk_incident";
    Inspection => "inspection", "sk_inspection";
    Training   => "training",   "sk_training";
});

/// Xavfsizlik hodisasi (TZ XV).
#[derive(Debug, Clone)]
pub struct SafetyEvent {
    pub id: i64,
    pub project_id: i64,
    pub date: NaiveDate,
    pub kind: SafetyKind,
    pub severity: Severity,
    pub place: String,
    pub description: String,
    pub responsible: String,
    pub measure: String,
    pub deadline: Option<NaiveDate>,
    pub status: IssueStatus,
}

// ---------- XVI. Mashinalar ----------

enum_kind!(MachineKind {
    Crane     => "crane",     "mck_crane";
    Excavator => "excavator", "mck_excavator";
    Loader    => "loader",    "mck_loader";
    Truck     => "truck",     "mck_truck";
    Concrete  => "concrete",  "mck_concrete";
    Lift      => "lift",      "mck_lift";
    Other     => "other",     "mck_other";
});

enum_kind!(MachineStatus {
    Working => "working", "ms_working";
    Idle    => "idle",    "ms_idle";
    Repair  => "repair",  "ms_repair";
    Off     => "off",     "ms_off";
});

/// Texnika (TZ XVI).
#[derive(Debug, Clone)]
pub struct Machine {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub kind: MachineKind,
    pub reg_no: String,
    pub owner: String,
    pub status: MachineStatus,
    pub hour_rate: f64,
    pub operator: String,
    /// Navbatdagi texnik ko'rik sanasi.
    pub inspection_until: Option<NaiveDate>,
}

/// Texnika smenasi.
#[derive(Debug, Clone)]
pub struct MachineLog {
    pub id: i64,
    pub project_id: i64,
    pub machine_id: i64,
    pub date: NaiveDate,
    pub hours: f64,
    pub fuel: f64,
    pub task_id: Option<i64>,
    pub note: String,
}
