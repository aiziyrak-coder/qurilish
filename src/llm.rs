//! OpenAI bilan integratsiya (TZ XVIII).
//!
//! **Sukut bo'yicha o'chiq.** Sozlamada yoqilmagunicha va kalit kiritilmaguncha
//! ilova hech qayerga ulanmaydi — yordamchi qoidalar bilan ishlayveradi.
//! Yoqilganda obyekt ma'lumotining bir qismi tashqi xizmatga jo'natiladi. Bu
//! qaytarib bo'lmaydigan qadam, shuning uchun sozlamada ochiq ogohlantirish
//! turadi va yoqishni foydalanuvchining o'zi tanlaydi.
//!
//! Modul uch qismga bo'lingan:
//! 1. **So'rovni tayyorlash va javobni o'qish** (`build_request`, `parse_reply`)
//!    — tarmoqsiz sinaladi;
//! 2. **Jo'natish** (`ask`) — sozlama to'liq bo'lgandagina ishlaydi;
//! 3. **Fonda ishlatish** (`spawn`) — interfeys javob kutib qotib qolmasin.
//!
//! Uch qat'iy qoida:
//! - Model **sonlarni o'ylab topmaydi**: so'rovga ilova hisoblab bergan tayyor
//!   sonlar biriktiriladi va modeldan faqat shularga tayanish so'raladi.
//! - **Kalit hech qayerga chiqmaydi**: u faqat `Authorization` sarlavhasida
//!   boradi; so'rov tanasiga ham, xato matniga ham, logga ham tushmaydi.
//! - **Xato yashirilmaydi**: nima bo'lgani aniq aytiladi — kalit noto'g'rimi,
//!   limit tugadimi, tarmoq yo'qmi.

// `llm` xususiyatisiz yig'ilganda so'rov tayyorlash va javobni o'qish faqat
// testlarda ishlatiladi — bu ataylab: integratsiya sirti doim sinaladi, tarmoq
// kodi esa yig'ilishga kirmaydi.
#![cfg_attr(not(feature = "llm"), allow(dead_code))]

use serde::{Deserialize, Serialize};

/// OpenAI ning standart manzili.
pub const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// Sukut bo'yicha model.
pub const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// Tanlash uchun tayyor modellar. Ro'yxat yopiq emas — sozlamada istalgan
/// nom yozilishi mumkin, chunki xizmat modellari vaqt o'tishi bilan
/// yangilanadi va dastur ularning ro'yxatini bilishi shart emas.
/// Varaq o'qish uchun sukut bo'yicha model.
///
/// Haqiqiy loyihada sinalgan: kichik model (`gpt-4o-mini`) sonlarni
/// adashtirdi va qatorlarni tashlab ketdi, `gpt-4.1` esa varaqni to'liq
/// o'qidi. Shuning uchun o'qish suhbat modelidan alohida sozlanadi.
pub const EXTRACT_MODEL: &str = "gpt-4.1";

pub const MODELS: &[&str] = &[
    "gpt-4o-mini",
    "gpt-4o",
    "gpt-4.1-mini",
    "gpt-4.1",
    "o4-mini",
];

/// Sukut bo'yicha kutish muddati, soniyada.
pub const DEFAULT_TIMEOUT: u64 = 45;

/// Kontekstning eng katta uzunligi, belgi. Undan uzuni qirqiladi: uzun
/// kontekst pul turadi va javobni yaxshilamaydi.
pub const MAX_CONTEXT: usize = 12_000;

/// Suhbat tarixidan modelga jo'natiladigan gaplar soni. Butun tarix
/// jo'natilsa har savol qimmatlashib boraveradi.
pub const HISTORY_TURNS: usize = 8;

/// Sozlama. API kaliti bazada saqlanadi va hech qayerga chiqarilmaydi:
/// logga ham, hisobotga ham, so'rov tanasiga ham tushmaydi.
#[derive(Debug, Clone)]
pub struct Config {
    pub enabled: bool,
    /// OpenAI-mos `/chat/completions` manzili.
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    /// Javobni kutish muddati, soniyada.
    pub timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: false,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            model: DEFAULT_MODEL.to_string(),
            api_key: String::new(),
            timeout_secs: DEFAULT_TIMEOUT,
        }
    }
}

impl Config {
    /// Yoqilgan va to'liq to'ldirilganmi.
    pub fn is_ready(&self) -> bool {
        self.enabled
            && !self.endpoint.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.api_key.trim().is_empty()
    }

    /// Kalitni qo'yadi va integratsiyani **yoqadi**.
    ///
    /// Sababi: odam kalitni ataylab kiritadi, bu uning aniq qarori. Avval
    /// kalit kiritilgandan keyin ham alohida tugmani topib yoqish kerak
    /// edi va «kalit qo'ydim, lekin ishlamayapti» degan holat chiqardi.
    ///
    /// Kalit o'chirilsa integratsiya ham o'chadi: kalitsiz u baribir
    /// ishlamaydi va «yoqilgan» deb turgani yolg'on bo'lardi.
    ///
    /// Sukut bo'yicha holat o'zgarmaydi: kalit kiritilmaguncha ilova
    /// hech qayerga ulanmaydi.
    pub fn set_key(&mut self, key: &str) {
        self.api_key = key.trim().to_string();
        self.enabled = !self.api_key.is_empty();
    }

    /// Kalitning faqat oxirgi to'rt belgisi ko'rsatiladi.
    pub fn masked_key(&self) -> String {
        let k = self.api_key.trim();
        if k.is_empty() {
            return String::new();
        }
        let tail: String = k
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("••••{tail}")
    }

    /// Kutish muddati chegaralarda: juda kichigi so'rovni uzib qo'yadi,
    /// juda kattasi esa interfeysda uzoq kutishga olib keladi.
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs.clamp(5, 180))
    }
}

/// Suhbatdagi bitta gap.
#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    /// `true` — foydalanuvchi savoli, `false` — model javobi.
    pub from_user: bool,
    pub text: String,
}

impl Turn {
    pub fn user(text: impl Into<String>) -> Turn {
        Turn {
            from_user: true,
            text: text.into(),
        }
    }

    pub fn model(text: impl Into<String>) -> Turn {
        Turn {
            from_user: false,
            text: text.into(),
        }
    }

    fn role(&self) -> &'static str {
        if self.from_user {
            "user"
        } else {
            "assistant"
        }
    }
}

/// Nima yuz berganini aniq aytadigan xato.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Sozlama to'liq emas yoki o'chiq.
    NotConfigured,
    /// Kalit qabul qilinmadi (401/403).
    Auth,
    /// So'rovlar chegarasi yoki hisobdagi mablag' tugadi (429).
    RateLimit,
    /// Xizmat tomonidagi xato (5xx).
    Service(u16),
    /// So'rov noto'g'ri tuzilgan yoki model nomi noma'lum (400/404).
    BadRequest(String),
    /// Tarmoq yo'q yoki javob kelmadi.
    Transport(String),
    /// Javob kutilgan ko'rinishda emas.
    BadReply(String),
}

impl Error {
    /// Xatoni tarjima kaliti bilan ko'rsatish uchun.
    pub fn key(&self) -> &'static str {
        match self {
            Error::NotConfigured => "llm_err_not_configured",
            Error::Auth => "llm_err_auth",
            Error::RateLimit => "llm_err_rate",
            Error::Service(_) => "llm_err_service",
            Error::BadRequest(_) => "llm_err_request",
            Error::Transport(_) => "llm_err_transport",
            Error::BadReply(_) => "llm_err_reply",
        }
    }

    /// Qayta urinib ko'rish ma'noli bo'ladimi.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Error::RateLimit | Error::Service(_) | Error::Transport(_)
        )
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotConfigured => write!(f, "not_configured"),
            Error::Auth => write!(f, "auth"),
            Error::RateLimit => write!(f, "rate_limit"),
            Error::Service(code) => write!(f, "service {code}"),
            Error::BadRequest(e) | Error::Transport(e) | Error::BadReply(e) => write!(f, "{e}"),
        }
    }
}

/// Sarflangan tokenlar — xarajat ko'rinib tursin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

/// Modeldan kelgan javob.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub text: String,
    pub usage: Usage,
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
    content: Option<String>,
}

#[derive(Deserialize, Default)]
struct ReplyUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Deserialize)]
struct Reply {
    #[serde(default)]
    choices: Vec<ReplyChoice>,
    #[serde(default)]
    usage: Option<ReplyUsage>,
    #[serde(default)]
    error: Option<ServiceError>,
}

#[derive(Deserialize)]
struct ServiceError {
    message: String,
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

/// Kontekstni chegaraga sig'diradi.
///
/// Qirqish **oxiridan** amalga oshiriladi: kontekst boshida umumiy holat,
/// oxirida tafsilot turadi — umumiy holat muhimroq.
pub fn trim_context(context: &str) -> String {
    let c = context.trim();
    if c.chars().count() <= MAX_CONTEXT {
        return c.to_string();
    }
    let cut: String = c.chars().take(MAX_CONTEXT).collect();
    format!("{cut}\n…")
}

/// So'rov tanasini tuzadi.
///
/// `context` — ilova hisoblab bergan sonlar. Model shu sonlardan tashqariga
/// chiqmasligi kerak. `history` — oldingi savol-javoblar; ulardan faqat
/// oxirgilari olinadi, aks holda har savol qimmatlashib boraveradi.
pub fn build_request(
    cfg: &Config,
    history: &[Turn],
    question: &str,
    context: &str,
) -> Result<String, Error> {
    if !cfg.is_ready() {
        return Err(Error::NotConfigured);
    }
    if question.trim().is_empty() {
        return Err(Error::BadRequest("empty question".into()));
    }

    let user = format!(
        "Savol: {}\n\nDastur bazasidan olingan ma'lumot:\n{}",
        question.trim(),
        trim_context(context)
    );

    let recent: Vec<&Turn> = history
        .iter()
        .rev()
        .take(HISTORY_TURNS)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let mut messages = vec![Message {
        role: "system",
        content: SYSTEM_PROMPT,
    }];
    for turn in &recent {
        if turn.text.trim().is_empty() {
            continue;
        }
        messages.push(Message {
            role: turn.role(),
            content: turn.text.trim(),
        });
    }
    messages.push(Message {
        role: "user",
        content: &user,
    });

    let req = Request {
        model: cfg.model.trim(),
        messages,
        temperature: 0.2,
    };
    serde_json::to_string(&req).map_err(|e| Error::BadReply(e.to_string()))
}

/// Javob tanasidan matnni va sarflangan tokenlarni ajratadi.
pub fn parse_reply(body: &str) -> Result<Answer, Error> {
    let reply: Reply = serde_json::from_str(body).map_err(|e| Error::BadReply(e.to_string()))?;

    // Xizmat 200 bilan ham xato qaytarishi mumkin — buni yashirmaymiz.
    if let Some(err) = reply.error {
        return Err(Error::BadRequest(err.message));
    }

    let text = reply
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() {
        return Err(Error::BadReply("empty".into()));
    }
    let u = reply.usage.unwrap_or_default();
    Ok(Answer {
        text,
        usage: Usage {
            prompt: u.prompt_tokens,
            completion: u.completion_tokens,
            total: u.total_tokens,
        },
    })
}

/// HTTP holat kodini tushunarli xatoga aylantiradi.
pub fn error_for_status(code: u16, body: &str) -> Error {
    // Xizmat xato sababini tanada yozib yuboradi — o'sha matnni olamiz.
    let detail = serde_json::from_str::<Reply>(body)
        .ok()
        .and_then(|r| r.error.map(|e| e.message))
        .unwrap_or_else(|| body.chars().take(200).collect());
    match code {
        401 | 403 => Error::Auth,
        429 => Error::RateLimit,
        500..=599 => Error::Service(code),
        _ => Error::BadRequest(detail),
    }
}

/// Savolni tashqi xizmatga jo'natadi.
///
/// Bu — ilovadagi yagona tarmoqqa chiqish nuqtasi. Sozlama o'chiq bo'lsa
/// funksiya hech narsa qilmaydi va shu bilan ilova to'liq lokal qoladi.
#[cfg(feature = "llm")]
pub fn ask(cfg: &Config, history: &[Turn], question: &str, context: &str) -> Result<Answer, Error> {
    let body = build_request(cfg, history, question, context)?;
    let agent = ureq::AgentBuilder::new().timeout(cfg.timeout()).build();
    let result = agent
        .post(cfg.endpoint.trim())
        .set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("Content-Type", "application/json")
        .send_string(&body);

    match result {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|e| Error::Transport(e.to_string()))?;
            parse_reply(&text)
        }
        // Xizmat javob berdi, lekin xato kodi bilan.
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(error_for_status(code, &body))
        }
        // Tarmoq yo'q, DNS ishlamadi yoki vaqt tugadi. Xato matnida manzil
        // bo'lishi mumkin, lekin kalit hech qachon unga tushmaydi.
        Err(e) => Err(Error::Transport(e.to_string())),
    }
}

/// Tarmoq qismi yig'ilishga kiritilmagan holat.
#[cfg(not(feature = "llm"))]
pub fn ask(
    _cfg: &Config,
    _history: &[Turn],
    _question: &str,
    _context: &str,
) -> Result<Answer, Error> {
    Err(Error::NotConfigured)
}

// ============================================================ Hujjat o'qish

/// Hujjat o'qish so'rovining qismi.
pub enum Part {
    Text(String),
    /// PDF fayl — model varaqni **ko'rib** o'qiydi.
    Pdf {
        name: String,
        data: Vec<u8>,
    },
}

/// Varaq o'qish uzoqroq davom etadi: model chizmani ko'rib chiqadi.
pub const EXTRACT_TIMEOUT: u64 = 300;

/// Baytlarni base64 ga o'giradi (PDF so'rov tanasida shunday ketadi).
pub fn base64(data: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(ABC[(n >> 18) as usize & 63] as char);
        out.push(ABC[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ABC[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ABC[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Hujjat o'qish so'rovining tanasi: javob faqat JSON bo'lishi so'raladi.
///
/// Kalit bu yerga **kirmaydi** — u faqat `Authorization` sarlavhasida
/// ketadi.
pub fn build_extract(cfg: &Config, system: &str, parts: &[Part]) -> Result<String, Error> {
    if !cfg.is_ready() {
        return Err(Error::NotConfigured);
    }
    let content: Vec<serde_json::Value> = parts
        .iter()
        .map(|p| match p {
            Part::Text(text) => serde_json::json!({"type": "text", "text": text}),
            Part::Pdf { name, data } => serde_json::json!({
                "type": "file",
                "file": {
                    "filename": name,
                    "file_data": format!("data:application/pdf;base64,{}", base64(data)),
                }
            }),
        })
        .collect();
    let model = cfg.model.trim();
    let mut body = serde_json::json!({
        "model": model,
        "response_format": {"type": "json_object"},
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": content},
        ],
    });
    // Harorat faqat uni qabul qiladigan modellarga yuboriladi: mulohaza
    // modellari (`o4-mini`, `gpt-5`) noldan boshqa qiymatni ham, nolni ham
    // rad etadi va butun so'rov xato bilan qaytadi.
    if model.starts_with("gpt-4") || model.starts_with("gpt-3") {
        body["temperature"] = serde_json::json!(0);
    }
    Ok(body.to_string())
}

/// Hujjat qismlarini modelga jo'natadi va JSON javobni qaytaradi.
#[cfg(feature = "llm")]
pub fn extract(cfg: &Config, system: &str, parts: &[Part]) -> Result<Answer, Error> {
    let body = build_extract(cfg, system, parts)?;
    let timeout = cfg
        .timeout()
        .max(std::time::Duration::from_secs(EXTRACT_TIMEOUT));
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let result = agent
        .post(cfg.endpoint.trim())
        .set("Authorization", &format!("Bearer {}", cfg.api_key.trim()))
        .set("Content-Type", "application/json")
        .send_string(&body);
    match result {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|e| Error::Transport(e.to_string()))?;
            parse_reply(&text)
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(error_for_status(code, &body))
        }
        Err(e) => Err(Error::Transport(e.to_string())),
    }
}

#[cfg(not(feature = "llm"))]
pub fn extract(_cfg: &Config, _system: &str, _parts: &[Part]) -> Result<Answer, Error> {
    Err(Error::NotConfigured)
}

/// So'rovni alohida oqimda bajaradi va natijani kanal orqali qaytaradi.
///
/// Interfeys javob kutib qotib qolmasligi kerak: egui har kadrda qayta
/// chiziladi, shuning uchun so'rov fonda ketadi va tayyor bo'lganda ekran
/// uni oladi.
pub fn spawn(
    cfg: Config,
    history: Vec<Turn>,
    question: String,
    context: String,
) -> std::sync::mpsc::Receiver<Result<Answer, Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let out = ask(&cfg, &history, &question, &context);
        // Qabul qiluvchi yopilgan bo'lishi mumkin (ekran almashtirildi) —
        // bu xato emas, shunchaki javob endi kerak emas.
        let _ = tx.send(out);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kalit qo'yilsa integratsiya o'zi yoqiladi, olinsa — o'chadi.
    #[test]
    fn a_key_switches_the_integration_on_and_off() {
        let mut c = Config::default();
        // Sukut bo'yicha ilova hech qayerga ulanmaydi.
        assert!(!c.enabled);
        assert!(!c.is_ready());

        c.set_key("  sk-test-kalit-1234  ");
        assert_eq!(c.api_key, "sk-test-kalit-1234", "bo'shliq olib tashlanmadi");
        assert!(c.enabled, "kalit qo'yildi, lekin yoqilmadi");
        assert!(c.is_ready());

        // Kalit olindi — «yoqilgan» deb turish yolg'on bo'lardi.
        c.set_key("   ");
        assert!(c.api_key.is_empty());
        assert!(!c.enabled);
        assert!(!c.is_ready());
    }

    fn cfg() -> Config {
        Config {
            enabled: true,
            endpoint: "https://example.invalid/v1/chat/completions".into(),
            model: "test-model".into(),
            api_key: "sk-1234567890abcd".into(),
            timeout_secs: DEFAULT_TIMEOUT,
        }
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// Varaq PDF holida ketadi, javob JSON so'raladi, kalit tanada yo'q.
    #[test]
    fn an_extract_request_carries_the_sheet_but_not_the_key() {
        let mut c = cfg();
        c.api_key = "sk-maxfiy-kalit".into();
        let parts = [
            Part::Text("varaq matni".into()),
            Part::Pdf {
                name: "p13.pdf".into(),
                data: b"%PDF-1.4".to_vec(),
            },
        ];
        let body = build_extract(&c, "qoida", &parts).unwrap();
        assert!(body.contains("json_object"));
        assert!(body.contains("data:application/pdf;base64,JVBERi0xLjQ="));
        assert!(body.contains("varaq matni"));
        assert!(!body.contains("sk-maxfiy-kalit"));
        // `test-model` haroratni qabul qilishi noma'lum — yuborilmaydi.
        assert!(!body.contains("temperature"));
        c.model = "gpt-4.1".into();
        assert!(build_extract(&c, "q", &parts)
            .unwrap()
            .contains("\"temperature\":0"));
        // O'chiq sozlamada so'rov tuzilmaydi.
        c.enabled = false;
        assert_eq!(build_extract(&c, "q", &parts), Err(Error::NotConfigured));
    }

    #[test]
    fn disabled_config_is_not_ready() {
        let mut c = cfg();
        c.enabled = false;
        assert!(!c.is_ready());
        assert_eq!(
            build_request(&c, &[], "savol", "ma'lumot"),
            Err(Error::NotConfigured)
        );
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
        // Manzil va model tayyor turadi — foydalanuvchi faqat kalit kiritadi.
        assert_eq!(c.endpoint, DEFAULT_ENDPOINT);
        assert_eq!(c.model, DEFAULT_MODEL);
        // Yoqilmaganda hech qayerga ulanilmaydi.
        assert_eq!(ask(&c, &[], "savol", ""), Err(Error::NotConfigured));
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
        let body = build_request(&cfg(), &[], "Nima kechikkan?", "Kechikish: 6 kun").unwrap();
        assert!(body.contains("test-model"));
        assert!(body.contains("O'YLAB TOPMANG"));
        assert!(body.contains("Kechikish: 6 kun"));
        assert!(body.contains("Nima kechikkan?"));
        // Kalit so'rov tanasiga tushmaydi — u faqat sarlavhada boradi.
        assert!(!body.contains("sk-1234567890abcd"));
    }

    #[test]
    fn history_is_sent_but_kept_short() {
        // Tarixdan faqat oxirgilari jo'natiladi.
        let mut history = Vec::new();
        for i in 0..20 {
            history.push(Turn::user(format!("savol-{i}")));
            history.push(Turn::model(format!("javob-{i}")));
        }
        let body = build_request(&cfg(), &history, "oxirgi savol", "kontekst").unwrap();
        // Oxirgi HISTORY_TURNS ta gap boradi: 40 tadan so'nggi 8 tasi.
        assert!(body.contains("javob-19"), "oxirgi gaplar jo'natilmadi");
        assert!(body.contains("savol-16"), "oxirgi savol jo'natilmadi");
        assert!(!body.contains("savol-0"), "butun tarix jo'natilyapti");
        assert!(!body.contains("javob-9"), "eski gaplar ham ketyapti");
        assert!(body.contains("assistant"));
        // Bo'sh gap jo'natilmaydi.
        let body = build_request(&cfg(), &[Turn::user("   ")], "savol", "").unwrap();
        assert_eq!(body.matches("\"role\":\"user\"").count(), 1);
    }

    #[test]
    fn empty_question_is_refused() {
        assert!(matches!(
            build_request(&cfg(), &[], "   ", "kontekst"),
            Err(Error::BadRequest(_))
        ));
    }

    #[test]
    fn long_context_is_trimmed() {
        let long = "a".repeat(MAX_CONTEXT * 2);
        let cut = trim_context(&long);
        assert!(cut.chars().count() <= MAX_CONTEXT + 2);
        assert!(cut.ends_with('…'));
        // Chegaradan qisqasi o'zgarmaydi.
        assert_eq!(trim_context("  qisqa  "), "qisqa");
    }

    #[test]
    fn reply_text_and_usage_are_extracted() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"  Uch ish kechikkan.  "}}],
                       "usage":{"prompt_tokens":120,"completion_tokens":30,"total_tokens":150}}"#;
        let a = parse_reply(body).unwrap();
        assert_eq!(a.text, "Uch ish kechikkan.");
        assert_eq!(a.usage.total, 150);
        assert_eq!(a.usage.prompt, 120);

        // Tokenlar ko'rsatilmasa ham javob o'qiladi.
        let plain = r#"{"choices":[{"message":{"content":"Javob"}}]}"#;
        assert_eq!(parse_reply(plain).unwrap().usage, Usage::default());
    }

    #[test]
    fn broken_reply_is_reported_not_panicked() {
        assert!(matches!(
            parse_reply("bu json emas"),
            Err(Error::BadReply(_))
        ));
        assert!(matches!(
            parse_reply(r#"{"choices":[]}"#),
            Err(Error::BadReply(_))
        ));
        assert!(matches!(
            parse_reply(r#"{"choices":[{"message":{"content":"   "}}]}"#),
            Err(Error::BadReply(_))
        ));
        // Xizmat 200 bilan xato qaytarsa ham buni ko'rsatamiz.
        let err = r#"{"error":{"message":"model not found","type":"invalid_request_error"}}"#;
        assert_eq!(
            parse_reply(err),
            Err(Error::BadRequest("model not found".into()))
        );
    }

    #[test]
    fn http_errors_say_what_happened() {
        let body = r#"{"error":{"message":"Incorrect API key provided: sk-xxx"}}"#;
        assert_eq!(error_for_status(401, body), Error::Auth);
        assert_eq!(error_for_status(403, "{}"), Error::Auth);
        assert_eq!(error_for_status(429, "{}"), Error::RateLimit);
        assert_eq!(error_for_status(503, "{}"), Error::Service(503));
        // Noma'lum model — sabab matni saqlanadi.
        let bad = error_for_status(404, r#"{"error":{"message":"The model does not exist"}}"#);
        assert_eq!(bad, Error::BadRequest("The model does not exist".into()));

        // Har xato o'z tarjima kalitiga ega va qaytarish mumkinligi belgilangan.
        for e in [
            Error::NotConfigured,
            Error::Auth,
            Error::RateLimit,
            Error::Service(500),
            Error::BadRequest("x".into()),
            Error::Transport("x".into()),
            Error::BadReply("x".into()),
        ] {
            assert!(e.key().starts_with("llm_err_"));
        }
        assert!(Error::RateLimit.retryable());
        assert!(!Error::Auth.retryable());
    }

    #[test]
    fn timeout_stays_within_sane_bounds() {
        let mut c = cfg();
        c.timeout_secs = 0;
        assert_eq!(c.timeout().as_secs(), 5);
        c.timeout_secs = 10_000;
        assert_eq!(c.timeout().as_secs(), 180);
        c.timeout_secs = 30;
        assert_eq!(c.timeout().as_secs(), 30);
    }

    /// O'chiq sozlamada fon oqimi ham hech qayerga ulanmaydi.
    #[test]
    fn background_call_respects_the_switch() {
        let rx = spawn(Config::default(), Vec::new(), "savol".into(), String::new());
        let got = rx.recv().expect("fon oqimi javob bermadi");
        assert_eq!(got, Err(Error::NotConfigured));
    }
}
