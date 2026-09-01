//! Til modeli bilan bog'lanish nuqtasi (TZ XVIII).
//!
//! **Sukut bo'yicha o'chiq.** Yoqilmagan bo'lsa ilova hech qayerga ulanmaydi va
//! yordamchi qoidalar bilan ishlayveradi. Yoqilganda obyekt ma'lumotining bir
//! qismi tashqi xizmatga jo'natiladi — bu qaytarib bo'lmaydigan qadam, shuning
//! uchun sozlamalarda ochiq ogohlantirish turadi va yoqishni foydalanuvchining
//! o'zi tanlaydi.
//!
//! Modul ikki qismga bo'lingan: **so'rovni tayyorlash va javobni o'qish**
//! (`build_request`, `parse_reply`) — bular tarmoqsiz sinaladi; va **jo'natish**
//! (`ask`) — u faqat sozlama to'liq bo'lganda ishlaydi.
//!
//! Muhim qoida: model **sonlarni o'ylab topmaydi**. So'rovga obyekt bo'yicha
//! tayyor hisoblar biriktiriladi va modeldan faqat shu sonlarga tayanib javob
//! berish so'raladi. Shu sabab javob ham tekshirilishi mumkin bo'lib qoladi.

// `llm` xususiyatisiz yig'ilganda so'rov tayyorlash va javobni o'qish faqat
// testlarda ishlatiladi — bu ataylab: integratsiya sirti doim sinaladi, tarmoq
// kodi esa yig'ilishga kirmaydi.
#![cfg_attr(not(feature = "llm"), allow(dead_code))]

use serde::{Deserialize, Serialize};

/// Sozlama. API kaliti bazada saqlanadi va hech qayerga chiqarilmaydi:
/// logga ham, hisobotga ham, zaxira nusxadan boshqa joyga ham tushmaydi.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub enabled: bool,
    /// OpenAI-mos `/chat/completions` manzili.
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

impl Config {
    /// Yoqilgan va to'liq to'ldirilganmi.
    pub fn is_ready(&self) -> bool {
        self.enabled
            && !self.endpoint.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.api_key.trim().is_empty()
    }

    /// Kalitning faqat oxirgi to'rt belgisi ko'rsatiladi.
    pub fn masked_key(&self) -> String {
        let k = self.api_key.trim();
        if k.is_empty() {
            return String::new();
        }
        let tail: String = k.chars().rev().take(4).collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("••••{tail}")
    }
}

/// Nima yuz berganini aniq aytadigan xato.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Sozlama to'liq emas yoki o'chiq.
    NotConfigured,
    /// Tarmoq yoki xizmat xatosi.
    Transport(String),
    /// Javob kutilgan ko'rinishda emas.
    BadReply(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotConfigured => write!(f, "not_configured"),
            Error::Transport(e) => write!(f, "{e}"),
            Error::BadReply(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    /// Javob qisqa va aniq bo'lsin.
    temperature: f32,
}

#[derive(Deserialize)]
struct ReplyChoice {
    message: ReplyMessage,
}

#[derive(Deserialize)]
struct ReplyMessage {
    content: String,
}

#[derive(Deserialize)]
struct Reply {
    choices: Vec<ReplyChoice>,
}

/// Modelga beriladigan ko'rsatma.
///
/// Ikki talab qat'iy: sonni o'ylab topmaslik va bilmagan narsani ochiq aytish.
/// Bular TZ II.17 va III.32 ning davomi — dastur normativ ham, hukm ham
/// o'ylab topmaydi.
pub const SYSTEM_PROMPT: &str = "Siz qurilish loyihasini boshqarish dasturining yordamchisisiz. \
Javobni faqat berilgan ma'lumotga tayanib bering. \
Ma'lumotda yo'q sonni yoki normativ hujjatni O'YLAB TOPMANG — bilmasangiz, \
«bu ma'lumot dasturda yo'q» deb yozing. \
Aybdorni belgilamang va hukm chiqarmang: faktni ayting, keyin nima qilish mumkinligini taklif qiling. \
Qisqa yozing: 3-6 gap. Foydalanuvchi qaysi tilda so'rasa, o'sha tilda javob bering.";

/// So'rov tanasini tuzadi.
///
/// `context` — ilova hisoblab bergan sonlar (yordamchi javobi matni). Model
/// shu sonlardan tashqariga chiqmasligi kerak.
pub fn build_request(cfg: &Config, question: &str, context: &str) -> Result<String, Error> {
    if !cfg.is_ready() {
        return Err(Error::NotConfigured);
    }
    let user = format!(
        "Savol: {}\n\nDastur bazasidan olingan ma'lumot:\n{}",
        question.trim(),
        context.trim()
    );
    let req = Request {
        model: cfg.model.trim(),
        messages: vec![
            Message {
                role: "system",
                content: SYSTEM_PROMPT,
            },
            Message {
                role: "user",
                content: &user,
            },
        ],
        temperature: 0.2,
    };
    serde_json::to_string(&req).map_err(|e| Error::BadReply(e.to_string()))
}

/// Javob tanasidan matnni ajratadi.
pub fn parse_reply(body: &str) -> Result<String, Error> {
    let reply: Reply =
        serde_json::from_str(body).map_err(|e| Error::BadReply(e.to_string()))?;
    let text = reply
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();
    if text.is_empty() {
        return Err(Error::BadReply("empty".into()));
    }
    Ok(text)
}

/// Savolni tashqi xizmatga jo'natadi.
///
/// Bu — ilovadagi yagona tarmoqqa chiqish nuqtasi. Sozlama o'chiq bo'lsa
/// funksiya hech narsa qilmaydi va shu bilan ilova to'liq lokal qoladi.
#[cfg(feature = "llm")]
pub fn ask(cfg: &Config, question: &str, context: &str) -> Result<String, Error> {
    let body = build_request(cfg, question, context)?;
    let resp = ureq::post(cfg.endpoint.trim())
        .set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| Error::Transport(e.to_string()))?;
    let text = resp
        .into_string()
        .map_err(|e| Error::Transport(e.to_string()))?;
    parse_reply(&text)
}

/// Tarmoq qismi yig'ilishga kiritilmagan holat.
///
/// Sukut bo'yicha ilova umuman tarmoq kutubxonasisiz yig'iladi — «o'chiq»
/// degani shu: kod ham yo'q, bog'liqlik ham yo'q.
#[cfg(not(feature = "llm"))]
pub fn ask(_cfg: &Config, _question: &str, _context: &str) -> Result<String, Error> {
    Err(Error::NotConfigured)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            enabled: true,
            endpoint: "https://example.invalid/v1/chat/completions".into(),
            model: "test-model".into(),
            api_key: "sk-1234567890abcd".into(),
        }
    }

    #[test]
    fn disabled_config_is_not_ready() {
        let mut c = cfg();
        c.enabled = false;
        assert!(!c.is_ready());
        assert_eq!(build_request(&c, "savol", "ma'lumot"), Err(Error::NotConfigured));
        // Yoqilgan, ammo to'ldirilmagan sozlama ham tayyor emas.
        let mut c = cfg();
        c.api_key = "  ".into();
        assert!(!c.is_ready());
    }

    #[test]
    fn default_config_is_off() {
        let c = Config::default();
        assert!(!c.enabled);
        assert!(!c.is_ready());
        // Yoqilmaganda hech qayerga ulanilmaydi.
        assert_eq!(ask(&c, "savol", ""), Err(Error::NotConfigured));
    }

    #[test]
    fn key_is_masked_not_shown() {
        let c = cfg();
        let m = c.masked_key();
        assert_eq!(m, "••••abcd");
        assert!(!m.contains("sk-"));
        assert!(!m.contains("1234567890"));
        assert!(Config::default().masked_key().is_empty());
    }

    #[test]
    fn request_carries_rules_and_context() {
        let body = build_request(&cfg(), "Nima kechikkan?", "Kechikish: 6 kun").unwrap();
        assert!(body.contains("test-model"));
        assert!(body.contains("O'YLAB TOPMANG"));
        assert!(body.contains("Kechikish: 6 kun"));
        assert!(body.contains("Nima kechikkan?"));
        // Kalit so'rov tanasiga tushmaydi — u faqat sarlavhada boradi.
        assert!(!body.contains("sk-1234567890abcd"));
    }

    #[test]
    fn reply_text_is_extracted() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"  Uch ish kechikkan.  "}}]}"#;
        assert_eq!(parse_reply(body).unwrap(), "Uch ish kechikkan.");
    }

    #[test]
    fn broken_reply_is_reported_not_panicked() {
        assert!(matches!(parse_reply("bu json emas"), Err(Error::BadReply(_))));
        assert!(matches!(parse_reply(r#"{"choices":[]}"#), Err(Error::BadReply(_))));
        assert!(matches!(
            parse_reply(r#"{"choices":[{"message":{"content":"   "}}]}"#),
            Err(Error::BadReply(_))
        ));
    }
}
