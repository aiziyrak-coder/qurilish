//! Rollar va kirish huquqlari (TZ VI–VIII, IX.2, XI.45, XIV.38, XV.38, XVI.46).
//!
//! Rol ekranlarni **yashirmaydi**, balki nimani o'zgartirish mumkinligini
//! belgilaydi. Sababi oddiy: buyurtmachi grafikni ko'rishi kerak, lekin uni
//! tahrirlamasligi kerak; omborchi qoldiqni yuritadi, lekin smetani emas.
//! Yashirilgan ma'lumot ishonchni yo'qotadi va odamlar baribir bir-biridan
//! so'rab oladi — shuning uchun cheklov faqat yozishda.
//!
//! Rollar TZ dagi ish taqsimotidan olingan: har bir modulda «kim ishlaydi» va
//! «kabinet» deb ko'rsatilgan shaxslar alohida rolga aylantirilgan.
//!
//! Bu ilova ichidagi ish taqsimoti, xavfsizlik chegarasi emas: baza fayli
//! ochiq turibdi, shuning uchun rol parol bilan himoyalanmagan va bu haqda
//! sozlamalarda ochiq yozilgan. Haqiqiy kirish nazorati server qismi bilan
//! keladi.

use crate::app::Screen;
use crate::i18n::t;

/// Ilovadagi rol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Tizim administratori — hamma narsani o'zgartiradi.
    Admin,
    /// Direktor: moliya, shartnoma va umumiy qarorlar.
    Director,
    /// Loyiha rahbari: muddat, PPR, ta'minot rejasi.
    ProjectManager,
    /// Prorab: kunlik ijro.
    Foreman,
    /// Brigadir: faqat o'z brigadasi bo'yicha tabel va jurnal.
    Brigadier,
    /// Texnik nazorat: hujjat, sifat, xavfsizlik.
    Supervisor,
    /// Mualliflik nazorati: loyiha yechimlari va nomuvofiqliklar.
    Designer,
    /// Smetachi: smeta va qiymat.
    Estimator,
    /// Ta'minotchi: arizalar va xaridlar.
    Supply,
    /// Omborchi: ombor va material katalogi.
    Storekeeper,
    /// Mexanik: texnika va smenalar.
    Mechanic,
    /// Sifat muhandisi.
    QualityEngineer,
    /// Mehnat muhofazasi muhandisi.
    SafetyEngineer,
    /// Kadrlar / tabelchi.
    Hr,
    /// Buxgalter: pul harakati ko'rinishlari.
    Accountant,
    /// Sotuv menejeri: kvartiralar, shartnomalar, to'lovlar.
    SalesManager,
    /// Buyurtmachi: faqat ko'radi.
    Client,
}

impl Role {
    pub const ALL: &'static [Role] = &[
        Role::Admin,
        Role::Director,
        Role::ProjectManager,
        Role::Foreman,
        Role::Brigadier,
        Role::Supervisor,
        Role::Designer,
        Role::Estimator,
        Role::Supply,
        Role::Storekeeper,
        Role::Mechanic,
        Role::QualityEngineer,
        Role::SafetyEngineer,
        Role::Hr,
        Role::Accountant,
        Role::SalesManager,
        Role::Client,
    ];

    /// Bazada saqlanadigan barqaror kod.
    pub fn code(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Director => "director",
            Role::ProjectManager => "pm",
            Role::Foreman => "foreman",
            Role::Brigadier => "brigadier",
            Role::Supervisor => "supervisor",
            Role::Designer => "designer",
            Role::Estimator => "estimator",
            Role::Supply => "supply",
            Role::Storekeeper => "storekeeper",
            Role::Mechanic => "mechanic",
            Role::QualityEngineer => "quality",
            Role::SafetyEngineer => "safety",
            Role::Hr => "hr",
            Role::Accountant => "accountant",
            Role::SalesManager => "sales",
            Role::Client => "client",
        }
    }

    pub fn parse(s: &str) -> Role {
        Role::ALL
            .iter()
            .copied()
            .find(|r| r.code() == s)
            // Notanish kod egasiga tushadi: baza buzilgan bo'lsa ham ilova
            // ochiladi, ammo bu holat sozlamalarda ko'rinib turadi.
            .unwrap_or(Role::Admin)
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => t("ur_admin"),
            Role::Director => t("ur_director"),
            Role::ProjectManager => t("ur_pm"),
            Role::Foreman => t("ur_foreman"),
            Role::Brigadier => t("ur_brigadier"),
            Role::Supervisor => t("ur_supervisor"),
            Role::Designer => t("ur_designer"),
            Role::Estimator => t("ur_estimator"),
            Role::Supply => t("ur_supply"),
            Role::Storekeeper => t("ur_storekeeper"),
            Role::Mechanic => t("ur_mechanic"),
            Role::QualityEngineer => t("ur_quality"),
            Role::SafetyEngineer => t("ur_safety"),
            Role::Hr => t("ur_hr"),
            Role::Accountant => t("ur_accountant"),
            Role::SalesManager => t("ur_sales"),
            Role::Client => t("ur_client"),
        }
    }

    /// Rol nima qila olishini bir gapda tushuntiradi.
    pub fn hint(self) -> &'static str {
        match self {
            Role::Admin => t("ur_admin_hint"),
            Role::Director => t("ur_director_hint"),
            Role::ProjectManager => t("ur_pm_hint"),
            Role::Foreman => t("ur_foreman_hint"),
            Role::Brigadier => t("ur_brigadier_hint"),
            Role::Supervisor => t("ur_supervisor_hint"),
            Role::Designer => t("ur_designer_hint"),
            Role::Estimator => t("ur_estimator_hint"),
            Role::Supply => t("ur_supply_hint"),
            Role::Storekeeper => t("ur_storekeeper_hint"),
            Role::Mechanic => t("ur_mechanic_hint"),
            Role::QualityEngineer => t("ur_quality_hint"),
            Role::SafetyEngineer => t("ur_safety_hint"),
            Role::Hr => t("ur_hr_hint"),
            Role::Accountant => t("ur_accountant_hint"),
            Role::SalesManager => t("ur_sales_hint"),
            Role::Client => t("ur_client_hint"),
        }
    }

    /// Rol ochilganda qaysi ekran ko'rsatiladi.
    pub fn home(self) -> Screen {
        match self {
            Role::Admin => Screen::Dashboard,
            Role::Director => Screen::Analytics,
            Role::ProjectManager => Screen::Gantt,
            Role::Foreman | Role::Brigadier => Screen::Foreman,
            Role::Supervisor => Screen::TechSupervision,
            Role::Designer => Screen::AiCheck,
            Role::Estimator => Screen::Estimate,
            Role::Supply => Screen::Requests,
            Role::Storekeeper => Screen::Warehouse,
            Role::Mechanic => Screen::Machines,
            Role::QualityEngineer => Screen::Quality,
            Role::SafetyEngineer => Screen::Safety,
            Role::Hr => Screen::Timesheet,
            Role::Accountant => Screen::Deals,
            Role::SalesManager => Screen::Sales,
            Role::Client => Screen::Client,
        }
    }

    /// Shu rol uchun ekranni **o'zgartirish** mumkinmi.
    pub fn can_edit(self, screen: Screen) -> bool {
        use Screen as S;
        match self {
            Role::Admin => true,
            // Ko'ruvchi rollar: hech narsani o'zgartirmaydi.
            Role::Client | Role::Accountant => false,
            // Direktor kunlik ijroga aralashmaydi, lekin pasport, moliya va
            // shartnomalar bo'yicha qaror qabul qiladi.
            Role::Director => matches!(
                screen,
                S::Passport | S::Deals | S::Analytics | S::Estimate | S::Purchases
            ),
            Role::ProjectManager => matches!(
                screen,
                S::Gantt | S::Ppr | S::Passport | S::Requests | S::ExecDocs | S::Analytics
            ),
            Role::Foreman => matches!(
                screen,
                S::Foreman
                    | S::Journal
                    | S::Timesheet
                    | S::Machines
                    | S::Gantt
                    | S::Requests
                    | S::Warehouse
                    | S::Safety
            ),
            // Brigadir prorabdan tor: faqat o'z brigadasi va kunlik yozuv.
            Role::Brigadier => matches!(screen, S::Foreman | S::Journal | S::Timesheet),
            Role::Supervisor => matches!(
                screen,
                S::TechSupervision | S::ExecDocs | S::Quality | S::Safety | S::AiCheck | S::Ppr
            ),
            // Mualliflik nazorati loyiha yechimlarini yuritadi, ijroga tegmaydi.
            Role::Designer => matches!(screen, S::AiCheck | S::Ppr | S::Passport),
            Role::Estimator => matches!(screen, S::Estimate | S::Purchases | S::Materials),
            Role::Supply => matches!(
                screen,
                S::Requests | S::Purchases | S::Materials | S::Warehouse
            ),
            Role::Storekeeper => matches!(screen, S::Warehouse | S::Materials),
            Role::Mechanic => matches!(screen, S::Machines),
            Role::QualityEngineer => matches!(screen, S::Quality | S::ExecDocs),
            Role::SafetyEngineer => matches!(screen, S::Safety),
            Role::Hr => matches!(screen, S::Timesheet),
            Role::SalesManager => matches!(screen, S::Sales | S::Deals | S::Client),
        }
    }
}

/// Ilova foydalanuvchisi.
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub role: Role,
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_round_trip() {
        let mut codes: Vec<&str> = Role::ALL.iter().map(|r| r.code()).collect();
        let n = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), n, "takroriy kod bor");
        for r in Role::ALL {
            assert_eq!(Role::parse(r.code()), *r);
        }
        assert_eq!(Role::parse("kim bilsin"), Role::Admin);
    }

    #[test]
    fn admin_edits_everything_client_edits_nothing() {
        for s in [
            Screen::Gantt,
            Screen::Estimate,
            Screen::Deals,
            Screen::Safety,
            Screen::Settings,
        ] {
            assert!(Role::Admin.can_edit(s), "{s:?}");
            assert!(!Role::Client.can_edit(s), "{s:?}");
        }
    }

    #[test]
    fn foreman_writes_execution_not_money() {
        assert!(Role::Foreman.can_edit(Screen::Journal));
        assert!(Role::Foreman.can_edit(Screen::Timesheet));
        assert!(!Role::Foreman.can_edit(Screen::Estimate));
        assert!(!Role::Foreman.can_edit(Screen::Deals));
    }

    /// Brigadir prorabdan tor bo'lishi kerak — teng emas.
    #[test]
    fn brigadier_is_narrower_than_foreman() {
        let screens = [
            Screen::Foreman,
            Screen::Journal,
            Screen::Timesheet,
            Screen::Gantt,
            Screen::Warehouse,
            Screen::Requests,
        ];
        let foreman = screens.iter().filter(|s| Role::Foreman.can_edit(**s)).count();
        let brig = screens
            .iter()
            .filter(|s| Role::Brigadier.can_edit(**s))
            .count();
        assert!(brig < foreman, "brigadir prorab bilan teng");
        assert!(!Role::Brigadier.can_edit(Screen::Gantt));
    }

    #[test]
    fn each_specialist_owns_its_module_only() {
        assert!(Role::Storekeeper.can_edit(Screen::Warehouse));
        assert!(!Role::Storekeeper.can_edit(Screen::Gantt));
        assert!(Role::Mechanic.can_edit(Screen::Machines));
        assert!(!Role::Mechanic.can_edit(Screen::Warehouse));
        assert!(Role::SafetyEngineer.can_edit(Screen::Safety));
        assert!(!Role::SafetyEngineer.can_edit(Screen::Quality));
        assert!(Role::Hr.can_edit(Screen::Timesheet));
        assert!(!Role::Hr.can_edit(Screen::Journal));
        assert!(Role::Estimator.can_edit(Screen::Estimate));
        assert!(!Role::Estimator.can_edit(Screen::Gantt));
    }

    /// Buxgalter pulni ko'radi, lekin yozmaydi — yozuv boshqa modullardan keladi.
    #[test]
    fn accountant_is_read_only() {
        for s in [Screen::Deals, Screen::Purchases, Screen::Timesheet] {
            assert!(!Role::Accountant.can_edit(s));
        }
    }

    /// Har bir rolning uy ekranini o'zi tahrirlay olishi kerak —
    /// aks holda odam ochilgan joyida hech narsa qila olmaydi.
    #[test]
    fn every_role_can_work_on_its_home_screen() {
        for r in Role::ALL {
            // Faqat ko'ruvchi rollar ataylab istisno.
            if matches!(r, Role::Client | Role::Accountant) {
                continue;
            }
            assert!(r.can_edit(r.home()), "{r:?} uy ekranida ishlay olmaydi");
        }
    }

    /// Har bir ekranni kamida bitta ixtisoslashgan rol yurita olishi kerak,
    /// aks holda ish faqat administratorga qolib ketadi.
    #[test]
    fn every_editable_screen_has_an_owner() {
        let screens = [
            Screen::Passport,
            Screen::Gantt,
            Screen::Ppr,
            Screen::AiCheck,
            Screen::Estimate,
            Screen::ExecDocs,
            Screen::Journal,
            Screen::Requests,
            Screen::Purchases,
            Screen::Warehouse,
            Screen::Materials,
            Screen::Timesheet,
            Screen::Quality,
            Screen::Safety,
            Screen::Machines,
            Screen::Sales,
            Screen::Deals,
        ];
        for s in screens {
            let owner = Role::ALL
                .iter()
                .filter(|r| **r != Role::Admin)
                .any(|r| r.can_edit(s));
            assert!(owner, "{s:?} uchun ixtisoslashgan rol yo'q");
        }
    }
}
