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
    /// Maydoncha vaqt mintaqasi — UTC dan siljish, daqiqalarda.
    ///
    /// Vaqtlar bazaga **UTC** da yoziladi: shunda ular bir xil tartibda
    /// saralanadi va qaysi mintaqada yozilgani haqida savol qolmaydi.
    /// Lekin telefondagi odam maydoncha vaqtida yashaydi: «bugun» qaysi
    /// kun ekani va belgi soat nechada qo'yilgani shu siljish bilan
    /// hisoblanadi. Siljishsiz, Toshkentda (UTC+5) yarim tundan ertalabki
    /// beshgacha server hali **kechagi** kunda turardi.
    pub tz_minutes: i32,
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
            // Sukut bo'yicha serverning o'z mintaqasi: server odatda
            // maydoncha bilan bir mamlakatda turadi va shunda hech narsa
            // sozlash kerak emas.
            tz_minutes: local_offset_minutes(),
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
            tz_minutes: std::env::var("QURAI_TZ")
                .ok()
                .and_then(|v| parse_offset(&v))
                .unwrap_or(d.tz_minutes),
        }
    }
}

/// Serverning o'z mintaqasi — UTC dan siljish, daqiqalarda.
fn local_offset_minutes() -> i32 {
    chrono::Local::now().offset().local_minus_utc() / 60
}

/// `+05:00`, `+0500`, `+5`, `-03:30`, `UTC` yoki `0` ni daqiqaga aylantiradi.
///
/// Tushunarsiz qiymat **rad etiladi** va sukutdagi siljish qoladi: noto'g'ri
/// tushunilgan sozlama jimgina boshqa vaqt mintaqasini berib qo'yardi.
pub fn parse_offset(text: &str) -> Option<i32> {
    let t = text.trim();
    if t.eq_ignore_ascii_case("utc") || t.eq_ignore_ascii_case("z") {
        return Some(0);
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1, r),
        None => (1, t.strip_prefix('+').unwrap_or(t)),
    };
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return None;
    }
    let (h, m) = match rest.split_once(':') {
        Some((h, m)) => (h, m),
        // `+0500` yoki `+5`.
        None if rest.len() == 4 => rest.split_at(2),
        None => (rest, "0"),
    };
    let hours: i32 = h.parse().ok()?;
    let minutes: i32 = m.parse().ok()?;
    if !(0..=59).contains(&minutes) || hours > 14 {
        return None;
    }
    Some(sign * (hours * 60 + minutes))
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

    /// Vaqt mintaqasi sozlamasi tushunilgan shakllarni qabul qiladi.
    #[test]
    fn the_time_zone_setting_is_read_or_refused() {
        assert_eq!(parse_offset("+05:00"), Some(300));
        assert_eq!(parse_offset("+0500"), Some(300));
        assert_eq!(parse_offset("+5"), Some(300));
        assert_eq!(parse_offset("5"), Some(300));
        assert_eq!(parse_offset("-03:30"), Some(-210));
        assert_eq!(parse_offset("UTC"), Some(0));
        assert_eq!(parse_offset("+00:00"), Some(0));
        assert_eq!(parse_offset(" +05:00 "), Some(300));

        // Tushunarsiz qiymat qabul qilinmaydi — jimgina nolga aylanmaydi.
        assert_eq!(parse_offset(""), None);
        assert_eq!(parse_offset("Tashkent"), None);
        assert_eq!(parse_offset("+05:99"), None);
        assert_eq!(parse_offset("+99"), None);
        assert_eq!(parse_offset("+"), None);
    }
}
