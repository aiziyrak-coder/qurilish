//! Kirish nazorati: parol, seans belgisi va rol huquqi.
//!
//! Uch qoida, ular buzilmaydi:
//!
//! 1. **Parol hech qayerda ochiq saqlanmaydi.** Bazada faqat Argon2id
//!    xeshi turadi; parolning o'zi logga ham, javobga ham tushmaydi.
//! 2. **Belgi taxmin qilib bo'lmaydigan bo'lishi kerak.** U tizim
//!    tasodifiy sonlar manbaidan olinadi, sanadan yoki foydalanuvchi
//!    nomidan hosil qilinmaydi.
//! 3. **Huquq rol bo'yicha beriladi va serverda tekshiriladi.** Desktop
//!    ilovadagi rol — ish taqsimoti; bu yerdagi rol esa haqiqiy chegara,
//!    chunki bazaga faqat server tegadi.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};

/// Seans muddati — kunlarda.
///
/// Qurilish maydonchasida odam har kuni qayta kirmasligi kerak, lekin
/// yo'qolgan telefon abadiy ochiq qolmasligi ham kerak.
pub const SESSION_DAYS: i64 = 14;

/// Parol uchun eng kichik uzunlik.
pub const MIN_PASSWORD: usize = 8;

/// Paroldan Argon2id xeshi hosil qiladi.
pub fn hash_password(password: &str) -> Result<String, String> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(format!("parol {MIN_PASSWORD} belgidan qisqa"));
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Parolni xesh bilan solishtiradi.
///
/// Xato holatlar bir xil `false` beradi: qaysi biri noto'g'ri ekanini
/// aytish hujumchiga yordam berardi.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Yangi seans belgisi: 32 bayt tasodif, o'n oltilik ko'rinishda.
pub fn new_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Rollar va ularning huquqi.
///
/// Ro'yxat desktop ilovadagi rol kodlari bilan bir xil — ikki joyda ikki
/// xil rol nomi bo'lishi chalkashlik keltirardi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Faqat ko'radi.
    Read,
    /// Ma'lumot yuborishi mumkin (kunlik ijro).
    Write,
    /// Hujjat imzolaydi.
    Sign,
    /// Foydalanuvchilarni boshqaradi.
    Admin,
}

/// Rol kodiga tegishli huquqlar.
pub fn access(role: &str) -> Vec<Access> {
    match role {
        "admin" => vec![Access::Read, Access::Write, Access::Sign, Access::Admin],
        // Texnik nazorat va mualliflik nazorati imzolaydi.
        "supervisor" | "designer" | "director" | "pm" => {
            vec![Access::Read, Access::Write, Access::Sign]
        }
        // Maydonda ishlaydiganlar ma'lumot yuboradi.
        "foreman" | "brigadier" | "supply" | "storekeeper" | "estimator" => {
            vec![Access::Read, Access::Write]
        }
        // Buyurtmachi va buxgalter faqat ko'radi.
        _ => vec![Access::Read],
    }
}

/// Rolda shu huquq bormi.
pub fn can(role: &str, what: Access) -> bool {
    access(role).contains(&what)
}

/// Matnning xesh-yig'indisi — imzo nimani tasdiqlaganini belgilaydi.
///
/// Bu kriptografik imzo emas va shunday deb atalmaydi ham: u hujjat
/// keyin o'zgarganini **ko'rsatadi**. Haqiqiy elektron raqamli imzo
/// davlat kalitlari bilan ishlaydi va u alohida masala.
pub fn digest(text: &str) -> String {
    // FNV-1a: kichik, tashqi kutubxonasiz va shu vazifaga yetarli.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_verifies_and_hides_the_password() {
        let hash = hash_password("juda-yaxshi-parol").expect("xesh");
        assert!(verify_password("juda-yaxshi-parol", &hash));
        assert!(!verify_password("boshqa-parol", &hash));
        // Xeshda parolning o'zi yo'q.
        assert!(!hash.contains("juda-yaxshi-parol"));
        // Har safar boshqa tuz — bir xil parol turlicha xeshlanadi.
        let again = hash_password("juda-yaxshi-parol").expect("xesh");
        assert_ne!(hash, again);
        assert!(verify_password("juda-yaxshi-parol", &again));
    }

    #[test]
    fn short_password_is_refused() {
        assert!(hash_password("qisqa").is_err());
        assert!(hash_password("").is_err());
    }

    #[test]
    fn broken_hash_does_not_let_anyone_in() {
        assert!(!verify_password("parol", "buzilgan-xesh"));
        assert!(!verify_password("", ""));
    }

    #[test]
    fn tokens_are_long_and_unique() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn roles_get_the_rights_they_need() {
        assert!(can("admin", Access::Admin));
        assert!(can("supervisor", Access::Sign));
        assert!(!can("foreman", Access::Sign));
        assert!(can("foreman", Access::Write));
        assert!(!can("client", Access::Write));
        assert!(can("client", Access::Read));
        // Noma'lum rol — eng kam huquq.
        assert_eq!(access("yo-q-rol"), vec![Access::Read]);
    }

    #[test]
    fn digest_changes_when_the_text_changes() {
        let a = digest("AOSR-001: beton quyish, 10 m3");
        assert_eq!(a, digest("AOSR-001: beton quyish, 10 m3"));
        assert_ne!(a, digest("AOSR-001: beton quyish, 11 m3"));
        assert_eq!(a.len(), 16);
    }
}
