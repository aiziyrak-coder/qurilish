//! Tashqi xabar yuborish nuqtasi (TZ VI.29, XVI.45, XI.29).
//!
//! TZ da «push (SMS / Telegram)» deb yozilgan. SMS ham, Telegram ham
//! tashqi xizmat: shartnoma, hisob va kalit talab qiladi, va ularning
//! qaysi biri ishlatilishi har tashkilotda boshqacha. Shu sababli bu
//! yerda **aniq bir xizmat emas, ulanish nuqtasi** qilingan: ilova yangi
//! signalni oddiy HTTP so'rovi bilan ko'rsatilgan manzilga yuboradi.
//! Bu manzil tashkilotning o'z boti, korporativ shlyuzi yoki
//! avtomatlashtirish xizmati bo'lishi mumkin.
//!
//! Uchta qat'iy qoida:
//!
//! 1. **Standart holatda o'chiq.** Manzil kiritilib, belgilanmaguncha
//!    ilova hech qayerga ulanmaydi. Ilova butunlay lokal ishlaydi.
//! 2. **Kalit faqat sarlavhada.** `Authorization` sarlavhasida ketadi;
//!    manzilga ham, so'rov tanasiga ham, jurnalga ham yozilmaydi —
//!    manzillar va loglar boshqalarga ko'rinadi.
//! 3. **Bir signal bir marta.** Yuborilgani eslab qolinadi: aks holda
//!    har sinxronizatsiyada bir xil xabar qayta ketardi va odam ularga
//!    e'tibor bermay qo'yardi.

/// Yuborish sozlamasi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    /// Yuborish yoqilganmi.
    pub on: bool,
    /// Manzil (`https://...`).
    pub url: String,
    /// Kalit — faqat `Authorization` sarlavhasida ketadi.
    pub token: String,
}

impl Config {
    /// Yuborishga tayyormi.
    ///
    /// Kalit majburiy emas: ichki tarmoqdagi qabul qiluvchi uni talab
    /// qilmasligi mumkin. Manzil esa albatta kerak va u `http` bilan
    /// boshlanishi shart — aks holda bu manzil emas.
    pub fn ready(&self) -> bool {
        self.on && (self.url.starts_with("http://") || self.url.starts_with("https://"))
    }
}

/// Yuboriladigan bitta signal.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub code: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

impl Item {
    /// Signalni takrorlanmas qilib belgilaydigan qiymat.
    ///
    /// Faqat kod yetarli emas: bitta koddagi signal mazmuni o'zgarishi
    /// mumkin (masalan «3 ta muddati o'tgan» → «5 ta»). Shuning uchun
    /// matn ham hisobga olinadi.
    pub fn key(&self) -> String {
        qurai_hash::text(&format!("{}|{}|{}", self.code, self.title, self.detail))
    }
}

/// Yuborish natijasi.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Sozlanmagan yoki o'chiq.
    Off,
    /// Yangi signal yo'q.
    Nothing,
    Sent(usize),
    Failed(String),
}

impl Outcome {
    pub fn text(&self) -> String {
        use crate::i18n::t;
        match self {
            Outcome::Off => t("hook_off").to_string(),
            Outcome::Nothing => t("hook_nothing").to_string(),
            Outcome::Sent(n) => format!("{} {n}", t("hook_sent")),
            Outcome::Failed(e) => format!("{}: {e}", t("hook_failed")),
        }
    }

    pub fn bad(&self) -> bool {
        matches!(self, Outcome::Failed(_))
    }
}

/// So'rov tanasi.
///
/// Shakli ataylab sodda va yassi: qabul qiluvchi tomonda ko'pincha
/// oddiy skript turadi va u murakkab tuzilmani kutmaydi.
pub fn body(project: &str, items: &[Item]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|i| {
            format!(
                "{{\"code\":{},\"severity\":{},\"title\":{},\"detail\":{}}}",
                json(&i.code),
                json(&i.severity),
                json(&i.title),
                json(&i.detail)
            )
        })
        .collect();
    format!(
        "{{\"source\":\"QURAi\",\"project\":{},\"items\":[{}]}}",
        json(project),
        rows.join(",")
    )
}

/// Matnni JSON satriga aylantiradi.
fn json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Hali yuborilmagan signallarni ajratadi.
pub fn fresh(items: &[Item], already: &[String]) -> Vec<Item> {
    items
        .iter()
        .filter(|i| !already.contains(&i.key()))
        .cloned()
        .collect()
}

/// Bir obyekt uchun bir yuborishda ketadigan eng ko'p signal.
///
/// Chegara bo'lmasa, birinchi ulanishda o'nlab xabar ketib, ular
/// o'qilmasdan qolardi.
pub const MAX_ITEMS: usize = 20;

/// Signallarni yuboradi.
///
/// Tarmoq ishi shu yerda, chaqiruvchi tomonda esa faqat natija — shuning
/// uchun bu funksiya fon oqimidan chaqiriladi.
#[cfg(feature = "sync")]
pub fn send(cfg: &Config, project: &str, items: &[Item]) -> Outcome {
    if !cfg.ready() {
        return Outcome::Off;
    }
    if items.is_empty() {
        return Outcome::Nothing;
    }
    let slice: Vec<Item> = items.iter().take(MAX_ITEMS).cloned().collect();
    let mut req = ureq::post(&cfg.url)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(15));
    if !cfg.token.trim().is_empty() {
        // Kalit faqat shu yerda. Manzilga qo'shilsa jurnalga tushardi.
        req = req.set("Authorization", &format!("Bearer {}", cfg.token.trim()));
    }
    match req.send_string(&body(project, &slice)) {
        Ok(_) => Outcome::Sent(slice.len()),
        // Xato matnida kalit bo'lmasligi kerak: `ureq` javob tanasini
        // qaytaradi, kalit esa faqat sarlavhada edi.
        Err(e) => Outcome::Failed(short_error(&e.to_string())),
    }
}

/// Tarmoqsiz yig'ilishda yuborish yo'q.
#[cfg(not(feature = "sync"))]
pub fn send(_cfg: &Config, _project: &str, _items: &[Item]) -> Outcome {
    Outcome::Off
}

/// Uzun xato matnini ekranga sig'diradi.
fn short_error(e: &str) -> String {
    let one: String = e.split('\n').next().unwrap_or(e).trim().to_string();
    if one.chars().count() <= 120 {
        return one;
    }
    one.chars().take(117).chain("...".chars()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(code: &str, title: &str) -> Item {
        Item {
            code: code.into(),
            severity: "critical".into(),
            title: title.into(),
            detail: String::new(),
        }
    }

    /// Sozlanmaguncha hech qayerga ulanmaydi.
    #[test]
    fn nothing_is_sent_until_it_is_configured() {
        let mut cfg = Config::default();
        assert!(!cfg.ready());
        cfg.url = "https://example.invalid/hook".into();
        // Manzil bor, lekin yoqilmagan.
        assert!(!cfg.ready());
        cfg.on = true;
        assert!(cfg.ready());
        // Manzil noto'g'ri bo'lsa — tayyor emas.
        cfg.url = "example.invalid".into();
        assert!(!cfg.ready());

        assert_eq!(
            send(&Config::default(), "OBY-1", &[item("a", "b")]),
            Outcome::Off
        );
    }

    /// Bir signal ikki marta yuborilmaydi, o'zgargani esa yuboriladi.
    #[test]
    fn the_same_alert_is_not_sent_twice() {
        let a = item("overdue", "3 ta ish muddati o'tgan");
        let b = item("stock", "Sement tugadi");
        let sent = vec![a.key()];

        let out = fresh(&[a.clone(), b.clone()], &sent);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].code, "stock");

        // Matn o'zgardi — bu boshqa xabar va u yuboriladi.
        let mut a2 = a.clone();
        a2.title = "5 ta ish muddati o'tgan".into();
        assert_ne!(a.key(), a2.key());
        assert_eq!(fresh(&[a2], &sent).len(), 1);
    }

    /// So'rov tanasi to'g'ri JSON bo'ladi va kalit unga tushmaydi.
    #[test]
    fn the_request_body_is_valid_json_without_the_key() {
        let mut i = item("overdue", "Muddat o'tdi");
        i.detail = "«Beton» ishi\n2 kun".into();
        let text = body("OBY \"1\"", &[i]);
        let v: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(v["project"], "OBY \"1\"");
        assert_eq!(v["items"][0]["code"], "overdue");
        assert_eq!(v["items"][0]["detail"], "«Beton» ishi\n2 kun");
        assert!(!text.contains("Bearer"), "kalit tanaga tushdi: {text}");
    }

    /// Uzun xato matni ekranni buzmaydi.
    #[test]
    fn a_long_error_is_trimmed() {
        let long = "x".repeat(500);
        let s = short_error(&long);
        assert!(s.chars().count() <= 120, "{}", s.chars().count());
        assert!(s.ends_with("..."));
        assert_eq!(short_error("bitta qator\nikkinchi"), "bitta qator");
    }
}
