//! Rollar va kirish huquqlari (TZ VI–VIII).
//!
//! Rol ekranlarni **yashirmaydi**, balki nimani o'zgartirish mumkinligini
//! belgilaydi. Sababi oddiy: buyurtmachi grafikni ko'rishi kerak, lekin uni
//! tahrirlamasligi kerak; prorab jurnalni to'ldiradi, lekin smetani emas.
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
    /// Hamma narsani ko'radi va o'zgartiradi.
    Admin,
    /// Ijro: jurnal, tabel, smenalar, bajarilish foizi.
    Foreman,
    /// Nazorat: hujjatlarni imzolaydi, sifat va xavfsizlikni yuritadi.
    Supervisor,
    /// Sotuv: kvartiralar, shartnomalar, to'lovlar.
    Sales,
    /// Buyurtmachi: faqat ko'radi.
    Client,
}

impl Role {
    pub const ALL: &'static [Role] = &[
        Role::Admin,
        Role::Foreman,
        Role::Supervisor,
        Role::Sales,
        Role::Client,
    ];

    /// Bazada saqlanadigan barqaror kod.
    pub fn code(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Foreman => "foreman",
            Role::Supervisor => "supervisor",
            Role::Sales => "sales",
            Role::Client => "client",
        }
    }

    pub fn parse(s: &str) -> Role {
        match s {
            "foreman" => Role::Foreman,
            "supervisor" => Role::Supervisor,
            "sales" => Role::Sales,
            "client" => Role::Client,
            _ => Role::Admin,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => t("role_admin"),
            Role::Foreman => t("role_foreman"),
            Role::Supervisor => t("role_supervisor"),
            Role::Sales => t("role_sales"),
            Role::Client => t("role_client"),
        }
    }

    /// Rol nima qila olishini bir gapda tushuntiradi.
    pub fn hint(self) -> &'static str {
        match self {
            Role::Admin => t("role_admin_hint"),
            Role::Foreman => t("role_foreman_hint"),
            Role::Supervisor => t("role_supervisor_hint"),
            Role::Sales => t("role_sales_hint"),
            Role::Client => t("role_client_hint"),
        }
    }

    /// Rol ochilganda qaysi ekran ko'rsatiladi.
    pub fn home(self) -> Screen {
        match self {
            Role::Admin => Screen::Dashboard,
            Role::Foreman => Screen::Foreman,
            Role::Supervisor => Screen::TechSupervision,
            Role::Sales => Screen::Sales,
            Role::Client => Screen::Client,
        }
    }

    /// Shu rol uchun ekranni **o'zgartirish** mumkinmi.
    ///
    /// Ko'rish har doim mumkin: yashirilgan ma'lumot ishonchni yo'qotadi va
    /// odamlar baribir bir-biridan so'rab oladi. Cheklov faqat yozishda.
    pub fn can_edit(self, screen: Screen) -> bool {
        use Screen as S;
        match self {
            Role::Admin => true,
            // Buyurtmachi hech narsani o'zgartirmaydi.
            Role::Client => false,
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
            Role::Supervisor => matches!(
                screen,
                S::TechSupervision
                    | S::ExecDocs
                    | S::Quality
                    | S::Safety
                    | S::AiCheck
                    | S::Estimate
                    | S::Ppr
            ),
            Role::Sales => matches!(screen, S::Sales | S::Deals | S::Client),
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
    fn codes_round_trip() {
        for r in Role::ALL {
            assert_eq!(Role::parse(r.code()), *r);
        }
        // Notanish kod xavfsiz tomonga emas, egasiga tushadi: baza buzilgan
        // bo'lsa ham ilova ochiladi, ammo bu holat sozlamalarda ko'rinadi.
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

    #[test]
    fn supervisor_signs_documents_not_timesheet() {
        assert!(Role::Supervisor.can_edit(Screen::ExecDocs));
        assert!(Role::Supervisor.can_edit(Screen::Quality));
        assert!(!Role::Supervisor.can_edit(Screen::Timesheet));
    }

    #[test]
    fn sales_role_is_limited_to_sales() {
        assert!(Role::Sales.can_edit(Screen::Deals));
        assert!(!Role::Sales.can_edit(Screen::Gantt));
        assert!(!Role::Sales.can_edit(Screen::Warehouse));
    }

    /// Har bir rolning uy ekranini o'zi tahrirlay olishi kerak —
    /// aks holda odam ochilgan joyida hech narsa qila olmaydi.
    #[test]
    fn every_role_can_work_on_its_home_screen() {
        for r in Role::ALL {
            if *r == Role::Client {
                // Buyurtmachi kabineti ataylab faqat o'qish uchun.
                continue;
            }
            assert!(r.can_edit(r.home()), "{r:?} uy ekranida ishlay olmaydi");
        }
    }
}
