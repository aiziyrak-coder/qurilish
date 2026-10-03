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
//! 4. **Kirish urinishi sanaladi va cheklanadi.** Parol xeshi sekin
//!    bo'lgani bitta urinishni qimmat qiladi, lekin urinish **sonini**
//!    cheklamaydi.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier};

/// Seans muddati — kunlarda.
///
/// Qurilish maydonchasida odam har kuni qayta kirmasligi kerak, lekin
/// yo'qolgan telefon abadiy ochiq qolmasligi ham kerak.
pub const SESSION_DAYS: i64 = 14;

/// Parol uchun eng kichik uzunlik.
pub const MIN_PASSWORD: usize = 8;

/// Urinishlar sanaladigan oyna — daqiqalarda.
pub const TRY_WINDOW_MINUTES: i64 = 15;

/// Shu oynada bitta login uchun yo'l qo'yiladigan eng ko'p xato urinish.
///
/// Odam parolni bir-ikki marta chalkashtirishi normal; o'ntadan keyin esa
/// bu parolni taxmin qilishga o'xshaydi.
pub const MAX_TRIES: i64 = 10;

/// Chegara oshganda ko'rsatiladigan matn.
///
/// Matn **nima bo'lganini** aytadi, lekin login mavjudligini oshkor
/// qilmaydi: u xato urinishdan keyin ham, yo'q login uchun ham bir xil.
pub const TOO_MANY_MESSAGE: &str = "Juda ko'p urinish. 15 daqiqadan keyin qayta urinib ko'ring.";

/// Login uchun urinish chegarasi oshib ketdimi.
///
/// Hisob bitta joyda: web sahifa va JSON aloqasi bir xil qoidaga bo'ysunadi.
///
/// Chegaraning narxi ochiq aytiladi: birov sizning loginingizni ataylab
/// xato parol bilan urib, uni 15 daqiqaga to'sib qo'yishi mumkin. Parolni
/// cheksiz taxmin qilishga ruxsat berishdan ko'ra, bu arzonroq.
///
/// Bu chegara **bitta login** bo'yicha ishlaydi. Bitta parolni ko'p login
/// ustida sinab ko'rish (spraying) bu yerda to'xtamaydi — uning uchun
/// so'rov manzili kerak, u esa teskari proksining ishi.
pub fn too_many_tries(store: &crate::store::Store, login: &str) -> bool {
    let since = crate::minus_minutes(TRY_WINDOW_MINUTES);
    store.login_fails(login, &since) >= MAX_TRIES
}

/// Noma'lum login uchun solishtiriladigan qo'g'irchoq xesh.
///
/// Kerakligi: avval login topilmasa javob **darhol** qaytardi, mavjud
/// login esa Argon2 hisobini kutardi. Shu vaqt farqidan mavjud loginlar
/// ro'yxatini yig'ish mumkin edi. Endi ikki holatda ham bir xil hisob
/// bajariladi.
///
/// Xesh tasodifiy paroldan olinadi va hech qayerda saqlanmaydi — unga mos
/// keladigan parol hech kimda yo'q.
fn dummy_hash() -> &'static str {
    static H: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    H.get_or_init(|| hash_password(&new_token()).unwrap_or_default())
}

/// Parolni tekshiradi; xesh bo'lmaganda ham **xuddi shuncha** vaqt ketadi.
pub fn verify_password_or_dummy(password: &str, hash: Option<&str>) -> bool {
    match hash {
        Some(h) => verify_password(password, h),
        None => {
            // Natijasi tashlanadi: bu yerda faqat vaqt tenglashtiriladi.
            let _ = verify_password(password, dummy_hash());
            false
        }
    }
}

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
    // SHA-256. Bu yerda xesh **buzilishni ko'rsatish** uchun turadi:
    // imzolangan matn keyin o'zgarsa, xesh mos kelmasligi kerak va unga
    // mos matn tanlab bo'lmasligi kerak. Oddiy yig'indi (avvalgi FNV)
    // birinchi shartni bajarardi, ikkinchisini esa yo'q.
    //
    // Xesh ilovadagi bilan bitta kutubxonadan olinadi: ikkita mustaqil
    // amalga oshirish bir kun kelib ajralib qolardi.
    qurai_hash::text(text)
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

    /// Noma'lum login ham haqiqiy Argon2 hisobidan o'tadi.
    ///
    /// Vaqtni soat bilan o'lchash sinovni ishonchsiz qilardi (mashina
    /// yuklamasiga bog'liq), shuning uchun **mexanizm** tekshiriladi:
    /// qo'g'irchoq xesh haqiqiy Argon2id xeshi, ya'ni uni solishtirish
    /// haqiqiy hisob talab qiladi.
    #[test]
    fn an_unknown_login_still_does_the_same_work() {
        assert!(dummy_hash().starts_with("$argon2id$"), "{}", dummy_hash());
        // Qo'g'irchoq xesh bilan hech qanday parol mos kelmaydi.
        assert!(!verify_password_or_dummy("parol", None));
        assert!(!verify_password_or_dummy("", None));
        // Xesh bor bo'lsa oddiy tekshiruv ishlaydi.
        let h = hash_password("juda-yaxshi-parol").expect("xesh");
        assert!(verify_password_or_dummy("juda-yaxshi-parol", Some(&h)));
        assert!(!verify_password_or_dummy("boshqa-parol", Some(&h)));
        // Qo'g'irchoq xesh bir marta hosil qilinadi.
        assert_eq!(dummy_hash(), dummy_hash());
    }

    /// Chegara matni login mavjudligini oshkor qilmaydi.
    #[test]
    fn the_limit_message_tells_nothing_about_the_login() {
        assert!(!TOO_MANY_MESSAGE.to_lowercase().contains("login yo"));
        assert!(TOO_MANY_MESSAGE.contains("urinish"));
        // Matndagi daqiqa chegara bilan mos: ikkisi ajralib ketmasligi kerak.
        assert!(
            TOO_MANY_MESSAGE.contains(&TRY_WINDOW_MINUTES.to_string()),
            "{TOO_MANY_MESSAGE}"
        );
    }

    #[test]
    fn digest_changes_when_the_text_changes() {
        let a = digest("AOSR-001: beton quyish, 10 m3");
        assert_eq!(a, digest("AOSR-001: beton quyish, 10 m3"));
        assert_ne!(a, digest("AOSR-001: beton quyish, 11 m3"));
        // SHA-256 — 64 ta o'n oltilik belgi.
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        // Ilova va server bitta xeshni beradi.
        assert_eq!(a, qurai_hash::text("AOSR-001: beton quyish, 10 m3"));
    }
}
