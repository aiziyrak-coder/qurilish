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
    Client   => "client",   "imod_client";
    Tech     => "tech",     "imod_tech";
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
    /// Ustama xarajatlar, to'g'ridan-to'g'ri xarajatlarga foizda (TZ III.18).
    pub overhead_pct: f64,
    /// Smeta foydasi, foizda (TZ III.19).
    pub profit_pct: f64,
    /// QQS stavkasi, foizda (TZ III.20).
    pub vat_pct: f64,
    pub added_at: String,
}

/// Smetaning tuzilishi: to'g'ridan-to'g'ri xarajatdan yakuniy summagacha.
///
/// Har bosqich alohida ko'rsatiladi — «qancha» degan savolga «shundan qancha
/// va nima uchun» deb javob berish uchun.
#[derive(Debug, Clone, Default)]
pub struct EstimateTotals {
    /// Pozitsiyalar yig'indisi.
    pub direct: f64,
    pub overhead: f64,
    pub profit: f64,
    /// QQS gacha bo'lgan summa.
    pub before_vat: f64,
    pub vat: f64,
    pub total: f64,
}

impl Estimate {
    /// Pozitsiyalar yig'indisidan yakuniy summani hisoblaydi.
    ///
    /// Ketma-ketlik smeta qoidasi bo'yicha: ustama to'g'ridan-to'g'ri
    /// xarajatga, foyda esa ustama bilan birga olingan summaga qo'yiladi,
    /// QQS eng oxirida.
    pub fn totals(&self, direct: f64) -> EstimateTotals {
        let overhead = direct * self.overhead_pct / 100.0;
        let profit = (direct + overhead) * self.profit_pct / 100.0;
        let before_vat = direct + overhead + profit;
        let vat = before_vat * self.vat_pct / 100.0;
        EstimateTotals {
            direct,
            overhead,
            profit,
            before_vat,
            vat,
            total: before_vat + vat,
        }
    }
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
    /// Qaysi GPR ishiga tegishli (TZ III.26). Bog'lanmagan pozitsiya
    /// tekshiruvda ko'rinadi: uni kim va qachon bajarishi noma'lum.
    pub task_id: Option<i64>,
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

enum_kind!(ApprovalDecision {
    Pending  => "pending",  "ad_pending";
    Approved => "approved", "ad_approved";
    Rejected => "rejected", "ad_rejected";
});

/// Kelishuv marshrutining bitta bosqichi (TZ IX.8).
///
/// Bosqichlar ariza summasiga qarab avtomatik ochiladi va tartib bilan
/// o'tiladi: oldingisi kelishmaguncha keyingisiga navbat kelmaydi.
#[derive(Debug, Clone)]
pub struct Approval {
    pub id: i64,
    pub project_id: i64,
    pub request_id: i64,
    /// Navbat raqami: 1, 2, 3.
    pub step: i64,
    /// Kim kelishishi kerak — rol kodi bilan saqlanadi.
    pub role: String,
    /// Kim kelishdi (ism). Qaror qabul qilinganda to'ldiriladi.
    pub approver: String,
    pub decision: ApprovalDecision,
    pub decided_at: Option<NaiveDate>,
    pub comment: String,
}

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
    /// Rad etish sababi (TZ IX.32): rad etilgan ariza sababsiz qolmasligi kerak.
    pub reject_reason: String,
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
    /// Haqiqatda kelgan miqdor (TZ X.30): qisman yetkazish oddiy holat.
    pub delivered_qty: f64,
    /// Qaysi bo'limga tegishli — byudjet shu kesimda nazorat qilinadi.
    pub section: crate::model::Section,
    pub note: String,
}

impl Purchase {
    pub fn amount(&self) -> f64 {
        self.qty * self.price
    }

    /// Yetkazilmay qolgan miqdor (TZ X.30).
    pub fn remaining(&self) -> f64 {
        (self.qty - self.delivered_qty).max(0.0)
    }

    /// Buyurtma to'liq yetkazilgan.
    pub fn fully_delivered(&self) -> bool {
        self.qty > 0.0 && self.delivered_qty + 0.0001 >= self.qty
    }

    /// Qisman yetkazilgan: bir qismi keldi, qolgani yo'lda.
    pub fn partial(&self) -> bool {
        self.delivered_qty > 0.0001 && !self.fully_delivered()
    }
}

/// Yetkazib beruvchi (TZ X.7–8, 16).
///
/// Tarix alohida saqlanmaydi — u xaridlardan hisoblanadi, shuning uchun
/// kartochka va haqiqiy buyurtmalar hech qachon bir-biriga zid bo'lmaydi.
#[derive(Debug, Clone)]
pub struct Supplier {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    /// STIR (INN).
    pub inn: String,
    pub contact: String,
    pub phone: String,
    /// Ishlash taqiqlangan: nomi ro'yxatda qoladi, lekin ogohlantiriladi.
    pub blocked: bool,
    pub note: String,
}

/// Tijorat taklifi — KP (TZ X.9–12, 15).
#[derive(Debug, Clone)]
pub struct Quote {
    pub id: i64,
    pub project_id: i64,
    /// Qaysi arizaga javoban. Bog'lanmagan taklif ham bo'lishi mumkin.
    pub request_id: Option<i64>,
    pub supplier: String,
    pub title: String,
    pub qty: f64,
    pub unit: String,
    pub price: f64,
    pub currency: String,
    /// Yetkazish muddati, kunlarda — narx bilan birga solishtiriladi.
    pub delivery_days: i64,
    /// Taklif shu sanagacha kuchda.
    pub valid_until: Option<NaiveDate>,
    /// Tanlangan taklif: shundan xarid ochiladi.
    pub chosen: bool,
    pub date: NaiveDate,
    pub note: String,
}

impl Quote {
    pub fn amount(&self) -> f64 {
        self.qty * self.price
    }
}

/// Bo'lim bo'yicha xarid byudjeti (TZ X.34–35).
#[derive(Debug, Clone)]
pub struct PurchaseBudget {
    pub id: i64,
    pub project_id: i64,
    pub section: crate::model::Section,
    pub planned: f64,
    pub note: String,
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
    /// Smetadagi rasenka kodi (TZ XII.7) — smeta bilan bog'lanish.
    pub estimate_code: String,
    /// Loyiha spetsifikatsiyasidagi pozitsiya yoki chizma varag'i (TZ XII.5–6).
    pub spec_ref: String,
    /// Maxsus talab: sovuqqa chidamlilik, yong'inga qarshilik va h.k. (TZ XII.31).
    pub special: String,
    /// Obyektda ishlatish taqiqlangan (TZ XII.38).
    pub banned: bool,
    /// Taqiq sababi. Taqiq bo'lsa bo'sh qolmasligi kerak.
    pub ban_reason: String,
    pub note: String,
}

/// Almashtiruvchi material — analog (TZ XII.9–11).
///
/// Analog **tasdiqlangan** bo'lishi kerak: kim va qachon ruxsat berganini
/// bilmasak, almashtirish loyihadan chetga chiqish bo'lib qoladi.
#[derive(Debug, Clone)]
pub struct MaterialAlt {
    pub id: i64,
    pub project_id: i64,
    /// Asosiy material.
    pub material_id: i64,
    /// O'rniga ishlatilishi mumkin bo'lgan material.
    pub alt_id: i64,
    /// Kim tasdiqlagan. Bo'sh — hali tasdiqlanmagan.
    pub approved_by: String,
    pub approved_at: Option<NaiveDate>,
    pub note: String,
}

impl MaterialAlt {
    pub fn approved(&self) -> bool {
        !self.approved_by.trim().is_empty() && self.approved_at.is_some()
    }
}

enum_kind!(MoveKind {
    In       => "in",       "mk_in";
    Return   => "return",   "mk_return";
    Out      => "out",      "mk_out";
    WriteOff => "writeoff", "mk_writeoff";
    // ToSupplier — yetkazib beruvchiga qaytarish (TZ XII.37): brak yoki
    // ortiqcha kelgan material omborni tark etadi, lekin ishga ketmaydi.
    ToSupplier => "to_supplier", "mk_to_supplier";
});

/// Ombor harakati (TZ XI).
#[derive(Debug, Clone)]
pub struct StockMove {
    pub id: i64,
    pub project_id: i64,
    pub material_id: i64,
    /// Qaysi omborda (TZ XI.3). Eski yozuvlarda bo'lmasligi mumkin.
    pub warehouse_id: Option<i64>,
    /// Qaysi partiyadan (TZ XI.9).
    pub batch_id: Option<i64>,
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

// ---------- XI. Omborlar, partiyalar, rezerv ----------

enum_kind!(WarehouseKind {
    Central   => "central",   "wk_central";
    Object    => "object",    "wk_object";
    Temporary => "temporary", "wk_temp";
    Open      => "open",      "wk_open";
    Fuel      => "fuel",      "wk_fuel";
    Tool      => "tool",      "wk_tool";
    Equipment => "equipment", "wk_equip";
    Workwear  => "workwear",  "wk_wear";
    Returns   => "returns",   "wk_returns";
});

/// Ombor (TZ XI.3). Bir obyektda bir nechta bo'lishi mumkin: markaziy,
/// obyektdagi, vaqtinchalik, ochiq maydon, YoMM, asbob, jihoz, ish kiyimi va
/// qaytgan material ombori.
#[derive(Debug, Clone)]
pub struct Warehouse {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub kind: WarehouseKind,
    pub responsible: String,
    pub note: String,
}

/// Material partiyasi (TZ XI.9–10).
///
/// Sertifikat va yaroqlilik muddati aynan partiyaga bog'lanadi: bir material
/// turli partiyalarda turli sertifikat bilan kelishi mumkin.
#[derive(Debug, Clone)]
pub struct Batch {
    pub id: i64,
    pub project_id: i64,
    pub material_id: i64,
    pub number: String,
    pub received: NaiveDate,
    pub supplier: String,
    pub cert_no: String,
    pub cert_until: Option<NaiveDate>,
    /// Yaroqlilik muddati — FEFO shu bo'yicha ishlaydi.
    pub expires: Option<NaiveDate>,
    pub note: String,
}

/// Rezerv (TZ XI.17): material aniq ish uchun band qilinadi.
#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: i64,
    pub project_id: i64,
    pub material_id: i64,
    pub task_id: Option<i64>,
    pub qty: f64,
    pub date: NaiveDate,
    /// Shu sanadan keyin rezerv o'z-o'zidan kuchini yo'qotadi.
    pub until: Option<NaiveDate>,
    pub note: String,
}

/// Ishga material sarf normasi (TZ XI.15, XII.21).
///
/// Bir birlik ish hajmiga qancha material ketishi kerakligi. Normativ sarf
/// shundan hisoblanadi: `per_unit × bajarilgan hajm`. Norma smetadan yoki
/// SNiP/ГЭСН dan olinadi, lekin bu yerda qo'lda kiritiladi — obyektning
/// haqiqiy sharoiti normadan farq qilishi mumkin.
#[derive(Debug, Clone)]
pub struct MaterialNorm {
    pub id: i64,
    pub project_id: i64,
    pub task_id: i64,
    pub material_id: i64,
    /// Ishning bir birligiga sarf (masalan 1 m3 monolitga 0.105 t armatura).
    pub per_unit: f64,
    /// Ruxsat etilgan ortiqcha sarf, foizda (texnologik yo'qotish).
    pub tolerance: f64,
    pub note: String,
}

/// Inventarizatsiya (TZ XI.24–25).
#[derive(Debug, Clone)]
pub struct Inventory {
    pub id: i64,
    pub project_id: i64,
    pub warehouse_id: Option<i64>,
    pub date: NaiveDate,
    pub responsible: String,
    /// Yopilgan inventarizatsiya o'zgartirilmaydi va farqlar harakatga aylanadi.
    pub closed: bool,
    pub note: String,
}

/// Inventarizatsiyaning bir qatori: hisobdagi va haqiqiy miqdor.
#[derive(Debug, Clone)]
pub struct InventoryLine {
    pub id: i64,
    pub inventory_id: i64,
    pub material_id: i64,
    /// Hisob bo'yicha qoldiq — inventarizatsiya ochilganda yozib qo'yiladi.
    pub book: f64,
    pub fact: f64,
    pub note: String,
}

impl InventoryLine {
    /// Farq: manfiy — kamomad, musbat — ortiqcha.
    pub fn diff(&self) -> f64 {
        self.fact - self.book
    }
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
    /// Qaysi brigadada (TZ XIII.8). Brigadasiz ishchi ham bo'lishi mumkin.
    pub brigade_id: Option<i64>,
}

/// Brigada (TZ XIII.8): ishchilar guruhi va uning brigadiri.
#[derive(Debug, Clone)]
pub struct Brigade {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub foreman: String,
    /// Brigada qaysi ishga biriktirilgan (TZ XIII.10). Majburiy emas.
    pub task_id: Option<i64>,
    pub note: String,
}

enum_kind!(Shift {
    Day     => "day",     "sh_day";
    Evening => "evening", "sh_evening";
    Night   => "night",   "sh_night";
});

impl Shift {
    /// Tungi smena soatlariga qo'shimcha haq (TZ XIII.14).
    ///
    /// Koeffitsiyent kodda turibdi va ekranda ochiq yozilgan — sozlamada
    /// o'zgartirilmaydi, chunki uni tashkilot o'z ichki hujjati bilan
    /// belgilaydi va bu yerda faqat hisob-kitob ko'rsatiladi.
    pub fn rate(self) -> f64 {
        match self {
            Shift::Day => 1.0,
            Shift::Evening => 1.2,
            Shift::Night => 1.5,
        }
    }
}

enum_kind!(DayKind {
    Work     => "work",     "dk_work";
    Downtime => "downtime", "dk_downtime";
    Vacation => "vacation", "dk_vacation";
    Sick     => "sick",     "dk_sick";
    Trip     => "trip",     "dk_trip";
    Absent   => "absent",   "dk_absent";
});

impl DayKind {
    /// Shu kun uchun ish haqi hisoblanadimi.
    ///
    /// Bo'sh turish va xizmat safari to'lanadi (ishchi aybdor emas), sababsiz
    /// yo'qlik to'lanmaydi. Ta'til va kasallik varaqasi bu yerda emas,
    /// buxgalteriyada hisoblanadi — shuning uchun tabel summasiga kirmaydi.
    pub fn paid(self) -> bool {
        matches!(self, DayKind::Work | DayKind::Downtime | DayKind::Trip)
    }

    /// Ishchi haqiqatda ishlaganmi — unumdorlik shundan hisoblanadi.
    pub fn worked(self) -> bool {
        matches!(self, DayKind::Work | DayKind::Trip)
    }

    /// Yo'qlik turi (TZ XIII.16–19).
    pub fn absence(self) -> bool {
        matches!(
            self,
            DayKind::Vacation | DayKind::Sick | DayKind::Trip | DayKind::Absent
        )
    }
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
    /// Kun turi: ish, bo'sh turish, ta'til, kasallik, safar, yo'qlik.
    pub kind: DayKind,
    pub shift: Shift,
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
    /// Qaysi chek-list bo'yicha tekshirildi (TZ XIV.8).
    pub checklist_id: Option<i64>,
    /// Nuqson bartaraf etilgan sana. Bo'sh — hali ochiq (TZ XIV.20, 35).
    pub fixed_at: Option<NaiveDate>,
    pub note: String,
}

impl QualityCheck {
    /// Yopilmagan nuqson: natija salbiy va bartaraf etilmagan.
    pub fn open_defect(&self) -> bool {
        self.result != QualityResult::Pass && self.fixed_at.is_none()
    }
}

/// Chek-list namunasi (TZ XIV.8): bo'lim yoki ish turi uchun nazorat ro'yxati.
#[derive(Debug, Clone)]
pub struct Checklist {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub section: crate::model::Section,
    pub kind: QualityKind,
    pub note: String,
}

/// Chek-list bandi — namunadagi bitta nazorat nuqtasi.
#[derive(Debug, Clone)]
pub struct ChecklistItem {
    pub id: i64,
    pub checklist_id: i64,
    pub pos: i64,
    pub text: String,
    /// Normativ havolasi: hujjat va band.
    pub norm_doc: String,
    pub norm_clause: String,
}

enum_kind!(PointResult {
    Pending => "pending", "pt_pending";
    Pass    => "pass",    "pt_pass";
    Fail    => "pass_no", "pt_fail";
    Na      => "na",      "pt_na";
});

/// Tekshiruvdagi nazorat nuqtasi.
///
/// Matn namunadan **ko'chirib olinadi**: namuna keyin o'zgarsa ham,
/// o'tkazilgan tekshiruv qanday bo'lgan bo'lsa shundayligicha qoladi.
#[derive(Debug, Clone)]
pub struct CheckPoint {
    pub id: i64,
    pub check_id: i64,
    pub pos: i64,
    pub text: String,
    pub norm_doc: String,
    pub norm_clause: String,
    pub result: PointResult,
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

enum_kind!(PermitKind {
    Induction => "induction", "pk_induction";
    Height    => "height",    "pk_height";
    Electric  => "electric",  "pk_electric";
    HotWork   => "hot_work",  "pk_hot_work";
    Lifting   => "lifting",   "pk_lifting";
    Confined  => "confined",  "pk_confined";
    Excavation => "excavation", "pk_excavation";
    Medical   => "medical",   "pk_medical";
});

/// Ishchining ruxsati yoki guvohnomasi (TZ XV.4–5).
///
/// Muddat majburiy: muddatsiz ruxsat nazorat qilinmaydi, u esa qog'ozda
/// bor, amalda yo'q degani.
#[derive(Debug, Clone)]
pub struct WorkerPermit {
    pub id: i64,
    pub project_id: i64,
    pub worker_id: i64,
    pub kind: PermitKind,
    /// Guvohnoma yoki protokol raqami.
    pub number: String,
    pub issued: NaiveDate,
    pub valid_until: NaiveDate,
    pub note: String,
}

impl WorkerPermit {
    pub fn expired(&self, today: NaiveDate) -> bool {
        self.valid_until < today
    }

    /// Shu kun ichida tugaydi — oldindan ogohlantirish uchun.
    pub fn expires_soon(&self, today: NaiveDate, days: i64) -> bool {
        !self.expired(today) && (self.valid_until - today).num_days() <= days
    }
}

enum_kind!(PpeItem {
    Helmet  => "helmet",  "ppe_helmet";
    Vest    => "vest",    "ppe_vest";
    Boots   => "boots",   "ppe_boots";
    Gloves  => "gloves",  "ppe_gloves";
    Glasses => "glasses", "ppe_glasses";
    Harness => "harness", "ppe_harness";
    Mask    => "mask",    "ppe_mask";
    Ears    => "ears",    "ppe_ears";
});

impl PpeItem {
    /// Har bir ishchida bo'lishi shart bo'lgan SIZ (TZ XV.8).
    ///
    /// Qolganlari ish turiga qarab beriladi: arqon faqat balandlikda,
    /// niqob changli ishda.
    pub const REQUIRED: &'static [PpeItem] = &[
        PpeItem::Helmet,
        PpeItem::Vest,
        PpeItem::Boots,
        PpeItem::Gloves,
    ];

    /// Odatdagi xizmat muddati, oylarda.
    pub fn months(self) -> i64 {
        match self {
            PpeItem::Helmet => 24,
            PpeItem::Vest | PpeItem::Boots => 12,
            PpeItem::Gloves | PpeItem::Mask => 1,
            PpeItem::Glasses | PpeItem::Ears => 12,
            PpeItem::Harness => 12,
        }
    }
}

/// Ishchiga berilgan SIZ (TZ XV.8–9).
#[derive(Debug, Clone)]
pub struct PpeIssue {
    pub id: i64,
    pub project_id: i64,
    pub worker_id: i64,
    pub item: PpeItem,
    pub issued: NaiveDate,
    /// Xizmat muddati, oylarda. Nol — muddatsiz.
    pub months: i64,
    pub note: String,
}

impl PpeIssue {
    /// Xizmat muddati tugaydigan sana. Muddatsiz bo'lsa `None`.
    pub fn until(&self) -> Option<NaiveDate> {
        (self.months > 0).then(|| {
            // Oylarni kunga aylantirishda o'rtacha oy uzunligi ishlatiladi:
            // SIZ muddati kun aniqligida yuritilmaydi.
            self.issued + chrono::Duration::days(self.months * 30)
        })
    }

    pub fn expired(&self, today: NaiveDate) -> bool {
        self.until().is_some_and(|d| d < today)
    }
}

enum_kind!(PermitStatus {
    Draft   => "draft",   "wps_draft";
    Open    => "open",    "wps_open";
    Closed  => "closed",  "wps_closed";
    Stopped => "stopped", "wps_stopped";
});

/// Naryad-dopusk — yuqori xavfli ishga ruxsat (TZ XV.10–11).
///
/// Bu hujjat aniq ish, aniq muddat va aniq odamlar uchun beriladi: muddati
/// o'tgan yoki ruxsatsiz ishchi kiritilgan naryad ish boshlashga asos emas.
#[derive(Debug, Clone)]
pub struct WorkPermit {
    pub id: i64,
    pub project_id: i64,
    pub number: String,
    /// Qanday xavfli ish — ishchining ruxsati shu turga tekshiriladi.
    pub kind: PermitKind,
    pub task_id: Option<i64>,
    pub place: String,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    /// Naryadni bergan mas'ul.
    pub issuer: String,
    /// Ish boshida nazorat qiladigan mas'ul.
    pub supervisor: String,
    /// Bajaruvchilar — ishchi id lari vergul bilan.
    pub workers: String,
    /// Xavfsizlik chora-tadbirlari.
    pub measures: String,
    pub status: PermitStatus,
    pub note: String,
}

impl WorkPermit {
    /// Bajaruvchilar ro'yxati.
    pub fn worker_ids(&self) -> Vec<i64> {
        self.workers
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    }

    pub fn set_workers(&mut self, ids: &[i64]) {
        self.workers = ids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",");
    }
}

enum_kind!(AuditAction {
    Insert => "insert", "au_insert";
    Update => "update", "au_update";
    Delete => "delete", "au_delete";
});

/// Amallar tarixi yozuvi (umumiy talab: kim nima o'zgartirgani).
///
/// Baza fayli ochiq bo'lgani uchun bu jurnal himoya emas — u **tiklash va
/// tushuntirish** vositasi: son qayerdan kelganini va uni kim kiritganini
/// keyin ham aytib bera oladi. Yozuvning o'zi ham tahrirlanadi, buni
/// yashirmaymiz.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub id: i64,
    /// Sana va vaqt, `YYYY-MM-DD HH:MM:SS`.
    pub at: String,
    /// Kim: tanlangan foydalanuvchi nomi va roli.
    pub user: String,
    pub action: AuditAction,
    /// Qaysi jadval o'zgardi.
    pub table_name: String,
    pub row_id: i64,
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
    /// Yoqilg'i sarf normasi, litr/motosoat (TZ XVI.17). Nol — norma yo'q.
    pub fuel_norm: f64,
    /// Rejali TX oralig'i, motosoatda (TZ XVI.25). Nol — reja yuritilmaydi.
    pub service_hours: f64,
    /// Oxirgi TX o'tkazilgandagi umumiy motosoat.
    pub service_done: f64,
    /// Ijaraga olingan texnika (TZ XVI.32).
    pub rented: bool,
}

/// Yo'l varaqasining holati (TZ XVI.19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaybillState {
    /// Raqami yo'q — bu shunchaki smena yozuvi.
    None,
    /// Ochiq: qaytish ko'rsatkichlari to'ldirilmagan.
    Open,
    /// Yopilgan.
    Closed,
}

/// Texnika smenasi — yo'l varaqasi (TZ XVI.19–22).
#[derive(Debug, Clone)]
pub struct MachineLog {
    pub id: i64,
    pub project_id: i64,
    pub machine_id: i64,
    pub date: NaiveDate,
    pub hours: f64,
    pub fuel: f64,
    pub task_id: Option<i64>,
    /// Yo'l varaqasi raqami. Bo'sh bo'lsa — oddiy smena yozuvi.
    pub number: String,
    pub driver: String,
    /// Marshrut: qayerdan qayerga.
    pub route: String,
    /// Spidometr ko'rsatkichi smena boshida va oxirida (TZ XVI.14).
    pub odo_start: f64,
    pub odo_end: f64,
    /// Reys soni va tashilgan yuk (TZ XVI.22).
    pub trips: i64,
    pub cargo: f64,
    pub note: String,
}

impl MachineLog {
    /// Yurgan masofa. Spidometr orqaga ketgan bo'lsa nol qaytadi.
    pub fn distance(&self) -> f64 {
        (self.odo_end - self.odo_start).max(0.0)
    }

    pub fn state(&self) -> WaybillState {
        if self.number.trim().is_empty() {
            WaybillState::None
        } else if self.odo_end > 0.0 || self.hours > 0.0 {
            WaybillState::Closed
        } else {
            WaybillState::Open
        }
    }
}

// ================================================================ VII. Texnik nazorat

enum_kind!(InspectionKind {
    Hidden   => "hidden",   "ik_hidden";
    Physical => "physical", "ik_physical";
    Concrete => "concrete", "ik_concrete";
    Geodesy  => "geodesy",  "ik_geodesy";
    Material => "material", "ik_material";
    Volume   => "volume",   "ik_volume";
    Final    => "final",    "ik_final";
});

enum_kind!(InspectionResult {
    Waiting     => "waiting",     "ir_waiting";
    Pass        => "pass",        "ir_pass";
    Conditional => "conditional", "ir_conditional";
    Fail        => "fail",        "ir_fail";
});

/// Texnik nazorat tekshiruvi (TZ VII.3-6, 12-14).
///
/// Bitta yozuv butun tekshiruv hayotini saqlaydi: qachonga rejalashtirilgan,
/// kim chaqirgan, qachon o'tkazilgan va natija nima. Rejalashtirilgan sana
/// o'tib ketgani — kalendarning asosiy signali.
#[derive(Debug, Clone)]
pub struct Inspection {
    pub id: i64,
    pub project_id: i64,
    /// Qaysi ishni tekshirish — GPR bilan bog'lanish (TZ VII.28).
    pub task_id: Option<i64>,
    pub kind: InspectionKind,
    pub number: String,
    /// Rejalashtirilgan sana.
    pub planned: NaiveDate,
    /// O'tkazilgan sana. Bo'sh — hali o'tkazilmagan.
    pub done: Option<NaiveDate>,
    /// Kim chaqirdi (pudratchi vakili) — TZ VII.4.
    pub requested_by: String,
    /// Kim tekshirdi.
    pub inspector: String,
    pub place: String,
    pub result: InspectionResult,
    /// Salbiy natijada — bartaraf etish muddati.
    pub deadline: Option<NaiveDate>,
    /// Bartaraf etilgan sana.
    pub fixed_at: Option<NaiveDate>,
    pub note: String,
}

impl Inspection {
    /// Hali o'tkazilmagan tekshiruv.
    pub fn open(&self) -> bool {
        self.done.is_none()
    }

    /// Muddati o'tgan, lekin o'tkazilmagan.
    pub fn overdue(&self, today: NaiveDate) -> bool {
        self.open() && self.planned < today
    }

    /// Salbiy natija va hali bartaraf etilmagan.
    pub fn open_defect(&self) -> bool {
        !self.open() && self.result != InspectionResult::Pass && self.fixed_at.is_none()
    }
}

/// Beton namunasining sinovi (TZ VII.12).
///
/// Mustahkamlik loyiha markasidan hisoblanmaydi — laboratoriya natijasi
/// kiritiladi. Ilova faqat solishtiradi va foizni ko'rsatadi.
#[derive(Debug, Clone)]
pub struct ConcreteTest {
    pub id: i64,
    pub project_id: i64,
    pub inspection_id: Option<i64>,
    pub task_id: Option<i64>,
    /// Namuna raqami.
    pub sample: String,
    /// Loyiha markasi, masalan «B25».
    pub grade: String,
    pub structure: String,
    /// Beton quyilgan sana.
    pub poured: NaiveDate,
    /// Sinov yoshi, kunlarda (odatda 7 yoki 28).
    pub age_days: i64,
    /// Talab qilinadigan mustahkamlik, MPa.
    pub required: f64,
    /// Laboratoriya natijasi, MPa. Natija hali yo'q bo'lsa — `None`.
    pub actual: Option<f64>,
    pub lab: String,
    pub note: String,
}

impl ConcreteTest {
    /// Sinov sanasi: quyilgan kun + yosh.
    pub fn test_date(&self) -> NaiveDate {
        self.poured + chrono::Duration::days(self.age_days)
    }

    /// Natija talabga javob beradimi. Natija yo'q bo'lsa — `None`.
    pub fn passed(&self) -> Option<bool> {
        self.actual.map(|a| a + 1e-9 >= self.required)
    }

    /// Talabdan foizda: 100 % — aynan loyiha markasi.
    pub fn pct(&self) -> Option<f64> {
        if self.required <= 0.0 {
            return None;
        }
        self.actual.map(|a| a / self.required * 100.0)
    }
}

/// Geodezik o'lchov nuqtasi (TZ VII.14).
#[derive(Debug, Clone)]
pub struct GeodesyPoint {
    pub id: i64,
    pub project_id: i64,
    pub inspection_id: Option<i64>,
    pub mark: String,
    pub axis: String,
    pub level: String,
    /// Loyiha qiymati.
    pub design: f64,
    /// O'lchangan qiymat.
    pub fact: f64,
    /// Ruxsat etilgan chetlanish (musbat son).
    pub tolerance: f64,
    pub unit: String,
    pub measured: NaiveDate,
    pub surveyor: String,
    pub note: String,
}

impl GeodesyPoint {
    /// Chetlanish: fakt minus loyiha.
    pub fn deviation(&self) -> f64 {
        self.fact - self.design
    }

    /// Dopusk ichidami.
    pub fn within(&self) -> bool {
        self.deviation().abs() <= self.tolerance + 1e-9
    }
}

// ================================================================ VIII. Buyurtmachi

enum_kind!(ContractKind {
    General => "general", "ck_general";
    Sub     => "sub",     "ck_sub";
    Supply  => "supply",  "ck_supply";
    Design  => "design",  "ck_design";
    Service => "service", "ck_service";
});

enum_kind!(ContractStatus {
    Draft     => "draft",     "cs_draft";
    Active    => "active",    "cs_active";
    Suspended => "suspended", "cs_suspended";
    Closed    => "closed",    "cs_closed";
});

/// Shartnoma (TZ VIII.27-28).
///
/// Obyekt pasportidagi shartnoma summasi umumiy raqam; bu yerda esa har bir
/// shartnoma alohida turadi — bosh pudrat, subpudrat, yetkazib berish.
#[derive(Debug, Clone)]
pub struct Contract {
    pub id: i64,
    pub project_id: i64,
    pub number: String,
    pub name: String,
    pub kind: ContractKind,
    /// Ikkinchi tomon (TZ I.1 ishtirokchilari).
    pub party_id: Option<i64>,
    pub signed: NaiveDate,
    pub start: NaiveDate,
    /// Shartnoma bo'yicha tugash sanasi.
    pub end: NaiveDate,
    pub sum: f64,
    /// Avans ulushi, foizda.
    pub advance_pct: f64,
    /// Kafolat ushlanmasi, foizda.
    pub retention_pct: f64,
    pub currency: String,
    pub status: ContractStatus,
    pub note: String,
}

impl Contract {
    /// Shartnomadagi dastlabki summa.
    ///
    /// `sum` maydoni to'g'ridan-to'g'ri ham o'qiladi; bu nom hisobotlarda
    /// «amaldagi summa» bilan chalkashmaslik uchun.
    pub fn base_sum(&self) -> f64 {
        self.sum
    }

    /// Avans summasi.
    pub fn advance(&self) -> f64 {
        self.sum * self.advance_pct / 100.0
    }

    /// Kafolat ushlanmasi summasi.
    pub fn retention(&self) -> f64 {
        self.sum * self.retention_pct / 100.0
    }

    /// Muddati o'tgan, lekin yopilmagan.
    pub fn overdue(&self, today: NaiveDate) -> bool {
        self.status == ContractStatus::Active && self.end < today
    }
}

enum_kind!(ChangeKind {
    Extra  => "extra",  "cch_extra";
    Reduce => "reduce", "cch_reduce";
    Price  => "price",  "cch_price";
    Term   => "term",   "cch_term";
});

enum_kind!(ChangeStatus {
    Draft    => "draft",    "chs_draft";
    Sent     => "sent",     "chs_sent";
    Approved => "approved", "chs_approved";
    Rejected => "rejected", "chs_rejected";
});

/// Shartnoma qiymati yoki muddatining o'zgarishi (TZ VIII.11-12).
///
/// Qo'shimcha ish ham, arzonlashtirish ham, muddat surilishi ham bitta
/// ro'yxatda: buyurtmachiga «shartnomadan qancha chetga chiqdik» degan savol
/// bitta joydan ko'rinishi kerak.
#[derive(Debug, Clone)]
pub struct ContractChange {
    pub id: i64,
    pub project_id: i64,
    pub contract_id: Option<i64>,
    pub number: String,
    pub kind: ChangeKind,
    pub date: NaiveDate,
    pub description: String,
    /// Summaga ta'siri. Kamaytirish uchun manfiy.
    pub amount: f64,
    /// Muddatga ta'siri, kunlarda.
    pub days: i64,
    pub reason: String,
    pub status: ChangeStatus,
    pub decided_at: Option<NaiveDate>,
    pub decided_by: String,
    pub note: String,
}

impl ContractChange {
    /// Tasdiqlangan o'zgarish shartnomaga qo'shiladi.
    pub fn counts(&self) -> bool {
        self.status == ChangeStatus::Approved
    }

    /// Qaror kutilmoqda.
    pub fn pending(&self) -> bool {
        self.status == ChangeStatus::Sent
    }
}

/// To'lov jadvalidagi bosqich (TZ VIII.29-30).
#[derive(Debug, Clone)]
pub struct PaymentStage {
    pub id: i64,
    pub project_id: i64,
    pub contract_id: Option<i64>,
    pub number: String,
    /// Asos: avans, bajarilgan ish, kafolat qaytarilishi.
    pub basis: String,
    pub due: NaiveDate,
    pub amount: f64,
    /// Haqiqatda to'langan summa.
    pub paid: f64,
    pub paid_at: Option<NaiveDate>,
    pub note: String,
}

impl PaymentStage {
    /// To'lanmagan qoldiq.
    pub fn left(&self) -> f64 {
        (self.amount - self.paid).max(0.0)
    }

    /// To'liq to'langan.
    pub fn closed(&self) -> bool {
        self.paid + 0.01 >= self.amount
    }

    /// Muddati o'tgan qarz.
    pub fn overdue(&self, today: NaiveDate) -> bool {
        !self.closed() && self.due < today
    }

    /// Kechikish, kunlarda. Muddati o'tmagan bo'lsa nol.
    pub fn delay_days(&self, today: NaiveDate) -> i64 {
        if self.overdue(today) {
            (today - self.due).num_days()
        } else {
            0
        }
    }
}

enum_kind!(AcceptState {
    Submitted => "submitted", "as_submitted";
    Accepted  => "accepted",  "as_accepted";
    Rejected  => "rejected",  "as_rejected";
});

/// Bajarilgan ishni buyurtmachiga topshirish (TZ VIII.21-22).
///
/// Bu yerdagi «kelishuv» — ilova ichidagi qaror, elektron raqamli imzo emas.
/// Kim va qachon qabul qilgani yoziladi; yuridik imzo hujjat ustida qoladi.
#[derive(Debug, Clone)]
pub struct WorkAcceptance {
    pub id: i64,
    pub project_id: i64,
    pub task_id: Option<i64>,
    pub number: String,
    pub date: NaiveDate,
    pub volume: f64,
    pub unit: String,
    pub amount: f64,
    pub state: AcceptState,
    pub decided_at: Option<NaiveDate>,
    pub decided_by: String,
    pub comment: String,
}

impl WorkAcceptance {
    /// Qaror kutilmoqda.
    pub fn pending(&self) -> bool {
        self.state == AcceptState::Submitted
    }
}
