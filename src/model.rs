//! Домены платформы: объект, участники, работы ГПР.

use crate::i18n::t;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Статус объекта (ТЗ I.1 — Паспорт объекта).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectStatus {
    Design,
    Tender,
    InProgress,
    Suspended,
    Completed,
}

impl ObjectStatus {
    pub fn label(self) -> &'static str {
        match self {
            ObjectStatus::Design => t("status_design"),
            ObjectStatus::Tender => t("status_tender"),
            ObjectStatus::InProgress => t("status_in_progress"),
            ObjectStatus::Suspended => t("status_suspended"),
            ObjectStatus::Completed => t("status_completed"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ObjectStatus::Design => "design",
            ObjectStatus::Tender => "tender",
            ObjectStatus::InProgress => "in_progress",
            ObjectStatus::Suspended => "suspended",
            ObjectStatus::Completed => "completed",
        }
    }

    pub fn parse(s: &str) -> ObjectStatus {
        match s {
            "design" => ObjectStatus::Design,
            "tender" => ObjectStatus::Tender,
            "suspended" => ObjectStatus::Suspended,
            "completed" => ObjectStatus::Completed,
            _ => ObjectStatus::InProgress,
        }
    }
}

/// Роль участника проекта: заказчик, подрядчик, технадзор и т.д.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyRole {
    Client,
    Contractor,
    Subcontractor,
    Designer,
    TechSupervision,
    AuthorSupervision,
}

impl PartyRole {
    pub const ALL: [PartyRole; 6] = [
        PartyRole::Client,
        PartyRole::Contractor,
        PartyRole::Subcontractor,
        PartyRole::Designer,
        PartyRole::TechSupervision,
        PartyRole::AuthorSupervision,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PartyRole::Client => t("role_client"),
            PartyRole::Contractor => t("role_contractor"),
            PartyRole::Subcontractor => t("role_subcontractor"),
            PartyRole::Designer => t("role_designer"),
            PartyRole::TechSupervision => t("role_tech_supervision"),
            PartyRole::AuthorSupervision => t("role_author_supervision"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PartyRole::Client => "client",
            PartyRole::Contractor => "contractor",
            PartyRole::Subcontractor => "subcontractor",
            PartyRole::Designer => "designer",
            PartyRole::TechSupervision => "tech_supervision",
            PartyRole::AuthorSupervision => "author_supervision",
        }
    }

    pub fn parse(s: &str) -> PartyRole {
        match s {
            "client" => PartyRole::Client,
            "subcontractor" => PartyRole::Subcontractor,
            "designer" => PartyRole::Designer,
            "tech_supervision" => PartyRole::TechSupervision,
            "author_supervision" => PartyRole::AuthorSupervision,
            _ => PartyRole::Contractor,
        }
    }
}

/// Раздел проектной документации (ТЗ II).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Section {
    None,
    Ar,
    Kj,
    Km,
    Vk,
    Ov,
    Eom,
    Ss,
    Pb,
}

impl Section {
    pub const ALL: [Section; 9] = [
        Section::None,
        Section::Ar,
        Section::Kj,
        Section::Km,
        Section::Vk,
        Section::Ov,
        Section::Eom,
        Section::Ss,
        Section::Pb,
    ];

    /// Bazada saqlanadigan barqaror kod — tilga bog'liq emas.
    pub fn code(self) -> &'static str {
        match self {
            Section::None => "NONE",
            Section::Ar => "AR",
            Section::Kj => "KJ",
            Section::Km => "KM",
            Section::Vk => "VK",
            Section::Ov => "OV",
            Section::Eom => "EOM",
            Section::Ss => "SS",
            Section::Pb => "PB",
        }
    }

    /// Ekranda ko'rinadigan qisqartma: lotin (uz) yoki kirill (ru).
    pub fn label(self) -> &'static str {
        let ru = crate::i18n::lang() == crate::i18n::Lang::Ru;
        match self {
            Section::None => "—",
            Section::Ar => if ru { "АР" } else { "AR" },
            Section::Kj => if ru { "КЖ" } else { "KJ" },
            Section::Km => if ru { "КМ" } else { "KM" },
            Section::Vk => if ru { "ВК" } else { "VK" },
            Section::Ov => if ru { "ОВ" } else { "OV" },
            Section::Eom => if ru { "ЭОМ" } else { "EOM" },
            Section::Ss => if ru { "СС" } else { "SS" },
            Section::Pb => if ru { "ПБ" } else { "PB" },
        }
    }

    /// Kodni ham, eski kirill qisqartmalarini ham tushunadi.
    pub fn parse(s: &str) -> Section {
        match s {
            "AR" | "АР" => Section::Ar,
            "KJ" | "КЖ" => Section::Kj,
            "KM" | "КМ" => Section::Km,
            "VK" | "ВК" => Section::Vk,
            "OV" | "ОВ" => Section::Ov,
            "EOM" | "ЭОМ" => Section::Eom,
            "SS" | "СС" => Section::Ss,
            "PB" | "ПБ" => Section::Pb,
            _ => Section::None,
        }
    }

    /// Цвет раздела для полос диаграммы Ганта.
    pub fn color(self) -> egui::Color32 {
        match self {
            Section::None => egui::Color32::from_rgb(120, 128, 140),
            Section::Ar => egui::Color32::from_rgb(86, 148, 232),
            Section::Kj => egui::Color32::from_rgb(120, 110, 200),
            Section::Km => egui::Color32::from_rgb(96, 170, 160),
            Section::Vk => egui::Color32::from_rgb(70, 160, 210),
            Section::Ov => egui::Color32::from_rgb(214, 150, 70),
            Section::Eom => egui::Color32::from_rgb(212, 180, 60),
            Section::Ss => egui::Color32::from_rgb(160, 140, 190),
            Section::Pb => egui::Color32::from_rgb(210, 100, 90),
        }
    }
}

/// Паспорт объекта.
#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub address: String,
    /// Obyekt turi: turar-joy, savdo, sanoat va h.k.
    pub object_type: String,
    pub floors: i64,
    /// Umumiy maydon, m².
    pub area_total: f64,
    pub status: ObjectStatus,
    pub start_date: NaiveDate,
    pub planned_end: NaiveDate,
    pub contract_sum: f64,
    /// Haqiqatda to'langan summa — moliyalashtirish nazorati uchun.
    pub paid_total: f64,
    pub currency: String,
    pub funding_source: String,
    pub notes: String,
}

/// Участник проекта: организация и ответственное лицо.
#[derive(Debug, Clone)]
pub struct Party {
    pub id: i64,
    pub project_id: i64,
    pub role: PartyRole,
    pub name: String,
    pub person: String,
    pub phone: String,
    pub email: String,
}

/// Тип связи между работами ГПР.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    /// Finish to Start
    Fs,
    /// Start to Start
    Ss,
    /// Finish to Finish
    Ff,
    /// Start to Finish
    Sf,
}

impl LinkType {
    pub const ALL: [LinkType; 4] = [LinkType::Fs, LinkType::Ss, LinkType::Ff, LinkType::Sf];

    pub fn label(self) -> &'static str {
        match self {
            LinkType::Fs => t("link_fs"),
            LinkType::Ss => t("link_ss"),
            LinkType::Ff => t("link_ff"),
            LinkType::Sf => t("link_sf"),
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            LinkType::Fs => "FS",
            LinkType::Ss => "SS",
            LinkType::Ff => "FF",
            LinkType::Sf => "SF",
        }
    }

    pub fn parse(s: &str) -> LinkType {
        match s {
            "SS" => LinkType::Ss,
            "FF" => LinkType::Ff,
            "SF" => LinkType::Sf,
            _ => LinkType::Fs,
        }
    }
}

/// Зависимость работ: `pred` -> `succ` с типом связи и лагом в днях.
#[derive(Debug, Clone)]
pub struct Link {
    pub id: i64,
    pub pred: i64,
    pub succ: i64,
    pub kind: LinkType,
    pub lag: i64,
}

/// Работа ГПР.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub wbs: String,
    pub name: String,
    pub section: Section,
    pub responsible: String,
    /// Плановая длительность в календарных днях.
    pub duration: i64,
    /// Плановое начало (пересчитывается CPM, если работа не закреплена).
    pub plan_start: NaiveDate,
    pub fact_start: Option<NaiveDate>,
    pub fact_end: Option<NaiveDate>,
    /// Процент выполнения 0..100.
    pub progress: f64,
    /// Работа закреплена вручную: CPM не двигает её начало раньше этой даты.
    pub pinned: bool,
    pub volume: f64,
    pub unit: String,
}
