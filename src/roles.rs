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

/// Rolning jurnaldagi vazifasi (TZ V.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalRole {
    /// Kunlik yozuvni kiritadi.
    Writes,
    /// Yozilganini tekshiradi: shubhalar va kun tahlili.
    Checks,
    /// Faqat ko'radi.
    Reads,
}

impl JournalRole {
    /// Ekran qaysi tabdan ochiladi.
    pub fn tab(self) -> u8 {
        match self {
            JournalRole::Writes => 0,
            JournalRole::Checks => 1,
            JournalRole::Reads => 0,
        }
    }

    /// Ko'rinish nima uchun shunday ekanini tushuntiruvchi kalit.
    pub fn hint(self) -> &'static str {
        match self {
            JournalRole::Writes => "jr_role_writes",
            JournalRole::Checks => "jr_role_checks",
            JournalRole::Reads => "jr_role_reads",
        }
    }
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

    /// Rol kundalik ishida ochadigan bo'limlar.
    ///
    /// Birinchisi — rolning uy ekrani (`home`). Ro'yxat ekranni
    /// **yashirmaydi**: yon panelda u «Sizning ishingiz» bo'lib tepada
    /// turadi, qolgan bo'limlar esa pastda yopiq sarlavha ostida qoladi
    /// va bir bosishda ochiladi. Sababi roli hujjatining boshida yozilgan:
    /// yashirilgan ma'lumot ishonchni yo'qotadi.
    ///
    /// Administrator uchun ro'yxat bo'sh: unda ajratadigan ish yo'q,
    /// shuning uchun yon panel odatdagidek to'liq ko'rinadi.
    ///
    /// Qoida: rol o'zgartira oladigan har bir ekran shu ro'yxatda bo'lishi
    /// shart — aks holda ruxsat berilgan ish ko'rinmay qolardi.
    pub fn screens(self) -> &'static [Screen] {
        use Screen as S;
        match self {
            Role::Admin => &[],
            Role::Director => &[
                S::Analytics,
                S::Director,
                S::Portfolio,
                S::Passport,
                S::Contracts,
                S::Deals,
                S::Estimate,
                S::Purchases,
                S::Reports,
                S::Notices,
            ],
            Role::ProjectManager => &[
                S::Gantt,
                S::Ppr,
                S::Passport,
                S::Requests,
                S::ExecDocs,
                S::Analytics,
                S::Journal,
                S::Reports,
                S::Notices,
            ],
            Role::Foreman => &[
                S::Foreman,
                S::Journal,
                S::Timesheet,
                S::Machines,
                S::Gantt,
                S::Requests,
                S::Warehouse,
                S::Safety,
                S::Notices,
            ],
            // Brigadir prorabdan tor: faqat o'z brigadasi va kunlik yozuv.
            Role::Brigadier => &[S::Foreman, S::Journal, S::Timesheet, S::Notices],
            Role::Supervisor => &[
                S::TechSupervision,
                S::Inspections,
                S::ExecDocs,
                S::Quality,
                S::Safety,
                S::AiCheck,
                S::Ppr,
                S::Journal,
                S::Notices,
            ],
            Role::Designer => &[S::AiCheck, S::Ppr, S::Passport, S::Journal, S::Notices],
            Role::Estimator => &[
                S::Estimate,
                S::Purchases,
                S::Materials,
                S::Reports,
                S::Notices,
            ],
            Role::Supply => &[
                S::Requests,
                S::Purchases,
                S::Materials,
                S::Warehouse,
                S::Notices,
            ],
            Role::Storekeeper => &[S::Warehouse, S::Materials, S::Requests, S::Notices],
            Role::Mechanic => &[S::Machines, S::Journal, S::Notices],
            Role::QualityEngineer => &[
                S::Quality,
                S::Inspections,
                S::ExecDocs,
                S::Journal,
                S::Notices,
            ],
            Role::SafetyEngineer => &[S::Safety, S::Journal, S::Timesheet, S::Notices],
            Role::Hr => &[S::Timesheet, S::Journal, S::Notices],
            Role::Accountant => &[
                S::Deals,
                S::Contracts,
                S::Purchases,
                S::Reports,
                S::Analytics,
                S::Notices,
            ],
            Role::SalesManager => &[S::Sales, S::Deals, S::Client, S::Reports, S::Notices],
            Role::Client => &[
                S::Client,
                S::Dashboard,
                S::Reports,
                S::Analytics,
                S::Notices,
            ],
        }
    }

    /// Shu rol uchun ekranni **o'zgartirish** mumkinmi.
    /// Rolning kunlik jurnaldagi vazifasi (TZ V.2).
    ///
    /// Jurnal bitta, ko'rinish esa uch xil: kimdir **yozadi** (prorab,
    /// brigadir), kimdir **tekshiradi** (texnik nazorat, mualliflik),
    /// qolganlar **ko'radi**. Bu huquq emas — huquqni `can_edit` beradi;
    /// bu shunchaki ekran qaysi tabdan ochilishini belgilaydi, chunki
    /// har rol jurnalga boshqa savol bilan keladi.
    pub fn journal_role(self) -> JournalRole {
        match self {
            Role::Foreman | Role::Brigadier | Role::Admin => JournalRole::Writes,
            Role::Supervisor | Role::Designer | Role::ProjectManager => JournalRole::Checks,
            _ => JournalRole::Reads,
        }
    }

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
                S::Passport | S::Deals | S::Analytics | S::Estimate | S::Purchases | S::Contracts
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
                S::TechSupervision
                    | S::Inspections
                    | S::ExecDocs
                    | S::Quality
                    | S::Safety
                    | S::AiCheck
                    | S::Ppr
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
            Role::QualityEngineer => matches!(screen, S::Quality | S::Inspections | S::ExecDocs),
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

    /// Rolning uy ekrani uning o'z bo'limlari ichida bo'ladi.
    ///
    /// Aks holda rol tanlanganda ochilgan ekran yon paneldagi «Sizning
    /// ishingiz» ro'yxatida ko'rinmay, odam qayerda turganini bilmasdi.
    #[test]
    fn the_home_screen_is_the_first_of_the_role_screens() {
        for r in Role::ALL {
            let own = r.screens();
            if own.is_empty() {
                // Administrator: ajratadigan ish yo'q.
                assert_eq!(*r, Role::Admin, "{:?} uchun bo'lim ro'yxati bo'sh", r);
                continue;
            }
            assert_eq!(
                own[0],
                r.home(),
                "{:?}: uy ekrani ro'yxatning birinchisi emas",
                r
            );
        }
    }

    /// Rol o'zgartira oladigan ekran uning ro'yxatida bo'lishi shart.
    ///
    /// Bo'lmasa, ruxsat berilgan ish yon panelda ko'rinmay qolardi: odam
    /// uni qidirib topishi kerak bo'lardi.
    #[test]
    fn every_editable_screen_is_in_the_role_list() {
        for r in Role::ALL {
            if r.screens().is_empty() {
                continue;
            }
            // Barcha ekranlar yon panel ro'yxatidan olinadi.
            for (_, screens) in crate::app::NAV_GROUPS {
                for screen in *screens {
                    if r.can_edit(*screen) {
                        assert!(
                            r.screens().contains(screen),
                            "{:?}: {:?} ni o'zgartira oladi, lekin ro'yxatida yo'q",
                            r,
                            screen
                        );
                    }
                }
            }
        }
    }

    /// Ro'yxatda bir ekran ikki marta turmaydi.
    #[test]
    fn the_role_list_has_no_repeats() {
        for r in Role::ALL {
            let own = r.screens();
            for (i, a) in own.iter().enumerate() {
                assert!(!own[i + 1..].contains(a), "{:?}: {:?} ikki marta", r, a);
            }
        }
    }

    /// TZ V.2: jurnalga yozadigan rol uni o'zgartira ham oladi —
    /// ko'rinish huquqdan ajralib ketmasin.
    #[test]
    fn journal_writer_can_actually_write() {
        for r in Role::ALL {
            if r.journal_role() == JournalRole::Writes {
                assert!(
                    r.can_edit(Screen::Journal),
                    "{:?} yozuvchi ko'rinishda, lekin huquqi yo'q",
                    r
                );
            }
            // Har rolning tushuntirish kaliti bor.
            assert!(!r.journal_role().hint().is_empty());
        }
        // Ko'ruvchi rollar jurnalni o'zgartirmaydi.
        assert_eq!(Role::Client.journal_role(), JournalRole::Reads);
        assert!(!Role::Client.can_edit(Screen::Journal));
        // Texnik nazorat tekshiradi: ekran kun tahlilidan ochiladi.
        assert_eq!(Role::Supervisor.journal_role(), JournalRole::Checks);
        assert_eq!(Role::Supervisor.journal_role().tab(), 1);
    }

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
        let foreman = screens
            .iter()
            .filter(|s| Role::Foreman.can_edit(**s))
            .count();
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
