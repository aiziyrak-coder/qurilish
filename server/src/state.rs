//! Server holati va sozlamalari.
//!
//! Sozlamalar **muhit o'zgaruvchilaridan** olinadi: parol va yo'llar kodga
//! yozilmaydi, shuning uchun ularni omborga tushirib yuborish ham mumkin
//! emas.

use crate::store::Store;
use std::path::PathBuf;

/// Ishlash sozlamalari.
#[derive(Debug, Clone)]
pub struct Config {
    /// Qaysi manzilda tinglaydi.
    pub bind: String,
    /// Baza fayli.
    pub db: PathBuf,
    /// Birinchi ishga tushirishda yaratiladigan administrator logini.
    pub admin_login: String,
    /// Server HTTPS beruvchi teskari proksi ortida turadimi.
    ///
    /// Bu **sozlama**, taxmin emas: `X-Forwarded-Proto` sarlavhasiga
    /// ishonib bo'lmaydi — proksisiz ochilgan serverga uni mijozning o'zi
    /// yuborib qo'yishi mumkin. Yoqilganda seans cookie siga `Secure`
    /// qo'yiladi, ya'ni belgi HTTP orqali hech qachon ketmaydi.
    pub https: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            // Sukut bo'yicha faqat shu kompyuterdan ochiladi. Tarmoqqa
            // chiqarish ataylab qilinadigan ish: `QURAI_BIND=0.0.0.0:8080`.
            bind: "127.0.0.1:8080".into(),
            db: PathBuf::from("qurai-server.db"),
            admin_login: "admin".into(),
            // Sukut bo'yicha o'chiq: mahalliy ishlatishda server HTTP
            // beradi va `Secure` qo'yilsa cookie umuman ishlamay qolardi.
            https: false,
        }
    }
}

impl Config {
    /// Muhit o'zgaruvchilaridan sozlamalarni oladi.
    pub fn from_env() -> Self {
        let d = Config::default();
        Config {
            bind: std::env::var("QURAI_BIND").unwrap_or(d.bind),
            db: std::env::var("QURAI_DB").map(PathBuf::from).unwrap_or(d.db),
            admin_login: std::env::var("QURAI_ADMIN").unwrap_or(d.admin_login),
            https: matches!(
                std::env::var("QURAI_HTTPS").unwrap_or_default().as_str(),
                "1" | "true" | "yes" | "ha"
            ),
        }
    }
}

/// So'rovlar orasida bo'lishadigan holat.
pub struct AppState {
    pub store: Store,
    pub config: Config,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sukut bo'yicha server tashqi tarmoqqa chiqmaydi.
    #[test]
    fn default_binding_is_local_only() {
        let c = Config::default();
        assert!(c.bind.starts_with("127.0.0.1"), "{}", c.bind);
        // HTTPS ham sukut bo'yicha o'chiq: mahalliy server HTTP beradi.
        assert!(!c.https);
    }
}
