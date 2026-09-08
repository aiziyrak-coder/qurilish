//! Server bilan sinxronizatsiya (TZ VI.36, umumiy «server va jonli almashish»).
//!
//! Ilova serversiz ham to'liq ishlaydi — bu asosiy holat. Server qo'shilganda
//! esa qurilmalar orasidagi almashish fayl tashishdan avtomatik almashishga
//! aylanadi: maydonchadagi kunlik ijro paketi serverga chiqadi, ofisdagi
//! nusxa uni oladi.
//!
//! Qat'iy qoidalar:
//!
//! 1. **Sukut bo'yicha o'chiq.** Manzil kiritilib, kirish qilinmaguncha ilova
//!    hech qayerga ulanmaydi.
//! 2. **Belgi manzilga yozilmaydi** — u faqat `Authorization` sarlavhasida
//!    ketadi va logga tushmaydi.
//! 3. **Paket serverda ochilmaydi.** Ma'lumotni tushunish ilovaning ishi;
//!    server faqat tartib bilan saqlaydi va tarqatadi.
//! 4. **Hech narsa jimgina o'chirilmaydi.** Kelgan paket mavjud yozuvlarga
//!    qo'shiladi (`import_package` qoidasi bilan), ustiga yozmaydi.

use std::sync::mpsc::Receiver;

/// So'rov kutish muddati, sekundda.
pub const TIMEOUT: u64 = 30;

/// Sozlama.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    /// Sinxronizatsiya yoqilganmi.
    pub enabled: bool,
    /// Server manzili: `http://10.0.0.5:8080`.
    pub url: String,
    /// Kirish nomi — kirish oynasini to'ldirish uchun eslab qolinadi.
    pub login: String,
    /// Seans belgisi. **Parol saqlanmaydi**, faqat belgi.
    pub token: String,
    /// Oxirgi olingan paket raqami.
    pub last_pull: i64,
}

impl Config {
    /// Ishlashga tayyormi: yoqilgan, manzil va belgi bor.
    pub fn ready(&self) -> bool {
        self.enabled && !self.url.trim().is_empty() && !self.token.trim().is_empty()
    }

    /// Manzilni yo'l bilan birlashtiradi.
    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.url.trim().trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

/// Xato turlari — har biriga aniq javob beriladi.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Sozlanmagan: manzil yoki kirish yo'q.
    NotConfigured,
    /// Login yoki parol noto'g'ri, yoki seans muddati tugagan.
    Auth,
    /// Rolda huquq yo'q.
    Forbidden,
    /// Server xatosi.
    Service(u16),
    /// Tarmoq: server topilmadi yoki javob bermadi.
    Transport(String),
    /// Javobni tushunib bo'lmadi.
    BadReply(String),
}

impl Error {
    /// Foydalanuvchiga ko'rsatiladigan xabar kaliti.
    pub fn key(&self) -> &'static str {
        match self {
            Error::NotConfigured => "sync_err_config",
            Error::Auth => "sync_err_auth",
            Error::Forbidden => "sync_err_forbidden",
            Error::Service(_) => "sync_err_service",
            Error::Transport(_) => "sync_err_transport",
            Error::BadReply(_) => "sync_err_reply",
        }
    }

    /// Qayta urinib ko'rish ma'noliroqmi.
    pub fn retryable(&self) -> bool {
        matches!(self, Error::Transport(_) | Error::Service(_))
    }
}

/// Kirish natijasi.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub token: String,
    pub name: String,
    pub role: String,
    pub until: String,
}

/// Serverdan olingan bitta paket.
#[derive(Debug, Clone, PartialEq)]
pub struct Incoming {
    pub id: i64,
    pub body: String,
    pub author: String,
    pub at: String,
}

/// Olib kelish natijasi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pulled {
    pub items: Vec<Incoming>,
    /// Oxirgi olingan paket raqami.
    pub last: i64,
    /// Serverda yana paket bormi.
    pub more: bool,
}

/// Javob kodini xatoga o'giradi.
pub fn error_for_status(code: u16) -> Error {
    match code {
        401 => Error::Auth,
        403 => Error::Forbidden,
        other => Error::Service(other),
    }
}

/// Kirish so'rovining tanasi.
///
/// Parol faqat shu yerda, so'rov tanasida ketadi va hech qayerda
/// saqlanmaydi.
pub fn login_body(login: &str, password: &str) -> String {
    format!(
        "{{\"login\":{},\"password\":{}}}",
        json_string(login),
        json_string(password)
    )
}

/// Paket yuborish so'rovining tanasi.
pub fn push_body(project: &str, body: &str, rows: i64) -> String {
    format!(
        "{{\"project\":{},\"body\":{},\"rows\":{rows}}}",
        json_string(project),
        json_string(body)
    )
}

/// Imzo so'rovining tanasi.
pub fn sign_body(project: &str, document: &str, text: &str, rejected: &str) -> String {
    format!(
        "{{\"project\":{},\"document\":{},\"text\":{},\"rejected\":{}}}",
        json_string(project),
        json_string(document),
        json_string(text),
        json_string(rejected)
    )
}

/// Signal ro'yxati so'rovining tanasi.
///
/// Signal serverda hisoblanmaydi: ilova o'zi hisoblab, tayyor ro'yxatni
/// yuboradi. Shu sababli telefonda ko'ringan son ofisdagi ekran bilan
/// bir xil bo'ladi.
pub fn notices_body(project: &str, items: &[NoticeOut]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|n| {
            format!(
                "{{\"code\":{},\"severity\":{},\"title\":{},\"detail\":{},\"count\":{},\"days\":{},\"source\":{}}}",
                json_string(n.code),
                json_string(n.severity),
                json_string(&n.title),
                json_string(&n.detail),
                n.count,
                n.days,
                json_string(n.source)
            )
        })
        .collect();
    format!(
        "{{\"project\":{},\"items\":[{}]}}",
        json_string(project),
        rows.join(",")
    )
}

/// Telefonda tanlash uchun yuboriladigan ish.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOut {
    pub wbs: String,
    pub name: String,
    /// Reja sanalari — telefonda muddat ko'rinishi uchun.
    pub start: String,
    pub end: String,
    pub progress: f64,
    pub section: String,
}

/// Ishlar ro'yxati so'rovining tanasi.
pub fn tasks_body(project: &str, items: &[TaskOut]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|t| {
            format!(
                "{{\"wbs\":{},\"name\":{},\"start\":{},\"end\":{},\"progress\":{},\"section\":{}}}",
                json_string(&t.wbs),
                json_string(&t.name),
                json_string(&t.start),
                json_string(&t.end),
                t.progress,
                json_string(&t.section)
            )
        })
        .collect();
    format!(
        "{{\"project\":{},\"items\":[{}]}}",
        json_string(project),
        rows.join(",")
    )
}

/// Buyurtmachi kabineti uchun yakun qatori.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryOut {
    pub module: String,
    pub indicator: String,
    pub value: String,
}

/// Yakun so'rovining tanasi.
pub fn summary_body(project: &str, items: &[SummaryOut]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|r| {
            format!(
                "{{\"module\":{},\"indicator\":{},\"value\":{}}}",
                json_string(&r.module),
                json_string(&r.indicator),
                json_string(&r.value)
            )
        })
        .collect();
    format!(
        "{{\"project\":{},\"items\":[{}]}}",
        json_string(project),
        rows.join(",")
    )
}

/// Tabel uchun yuboriladigan ishchi.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerOut {
    pub name: String,
    pub position: String,
}

/// Ishchilar ro'yxati so'rovining tanasi.
pub fn workers_body(project: &str, items: &[WorkerOut]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|w| {
            format!(
                "{{\"name\":{},\"position\":{}}}",
                json_string(&w.name),
                json_string(&w.position)
            )
        })
        .collect();
    format!(
        "{{\"project\":{},\"items\":[{}]}}",
        json_string(project),
        rows.join(",")
    )
}

/// Serverga yuboriladigan bitta QR yorliq (TZ VI.11).
#[derive(Debug, Clone, PartialEq)]
pub struct LabelOut {
    pub kind: String,
    pub number: String,
    pub title: String,
    pub note: String,
}

/// Yorliqlar so'rovining tanasi.
pub fn labels_body(project: &str, items: &[LabelOut]) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|l| {
            format!(
                "{{\"kind\":{},\"number\":{},\"title\":{},\"note\":{}}}",
                json_string(&l.kind),
                json_string(&l.number),
                json_string(&l.title),
                json_string(&l.note)
            )
        })
        .collect();
    format!(
        "{{\"project\":{},\"items\":[{}]}}",
        json_string(project),
        rows.join(",")
    )
}

/// Xabar yuborish so'rovining tanasi (TZ VI.32).
pub fn message_body(project: &str, text: &str) -> String {
    format!(
        "{{\"project\":{},\"text\":{}}}",
        json_string(project),
        json_string(text)
    )
}

/// Serverdan kelgan xabar.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageIn {
    pub id: i64,
    pub author: String,
    pub role: String,
    pub text: String,
    pub at: String,
}

/// Xabarlar javobini o'qiydi.
///
/// Kutilmagan qator tashlab yuboriladi, xato ko'tarilmaydi: chat
/// ishlamay qolgani butun sinxronizatsiyani to'xtatmasligi kerak.
pub fn parse_messages(body: &str) -> Vec<MessageIn> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    v["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m["id"].as_i64().unwrap_or(0);
            let text = m["text"].as_str().unwrap_or_default();
            (id > 0 && !text.trim().is_empty()).then(|| MessageIn {
                id,
                author: m["author"].as_str().unwrap_or_default().to_string(),
                role: m["role"].as_str().unwrap_or_default().to_string(),
                text: text.to_string(),
                at: m["at"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Har sinxronizatsiyada serverga to'liq yuboriladigan ma'lumotnomalar.
///
/// Ular bitta tuzilmaga yig'ilgan: har yangi ro'yxat qo'shilganda
/// funksiya imzosi o'zgaravermasin va chaqiruv joyida nima
/// yuborilayotgani nomi bilan ko'rinib tursin.
#[derive(Debug, Clone, Default)]
pub struct Refs {
    pub notices: Vec<NoticeOut>,
    pub tasks: Vec<TaskOut>,
    pub workers: Vec<WorkerOut>,
    pub summary: Vec<SummaryOut>,
    pub labels: Vec<LabelOut>,
    /// Imzo daftarining uzunligi va uchi (TZ IV.18, V.32).
    pub chain: (i64, String),
}

/// Serverda qayd etilgan imzo zanjiri belgisi (TZ IV.18, V.32).
#[derive(Debug, Clone, PartialEq)]
pub struct ChainMark {
    pub count: i64,
    pub head: String,
    pub at: String,
}

/// Zanjir belgisi so'rovining tanasi.
pub fn chain_body(project: &str, count: i64, head: &str) -> String {
    format!(
        "{{\"project\":{},\"count\":{count},\"head\":{}}}",
        json_string(project),
        json_string(head)
    )
}

/// Zanjir belgilari javobini o'qiydi.
pub fn parse_chain(body: &str) -> Vec<ChainMark> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    v["marks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let count = m["count"].as_i64()?;
            Some(ChainMark {
                count,
                head: m["head"].as_str().unwrap_or_default().to_string(),
                at: m["at"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Serverga yuboriladigan bitta signal.
#[derive(Debug, Clone, PartialEq)]
pub struct NoticeOut {
    pub code: &'static str,
    pub severity: &'static str,
    pub title: String,
    pub detail: String,
    pub count: i64,
    pub days: i64,
    pub source: &'static str,
}

/// Matnni JSON satriga aylantiradi.
fn json_string(s: &str) -> String {
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

/// Kirish javobini o'qiydi.
pub fn parse_login(body: &str) -> Result<Session, Error> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| Error::BadReply(e.to_string()))?;
    let token = v["token"].as_str().unwrap_or_default().to_string();
    if token.is_empty() {
        return Err(Error::BadReply("belgi yo'q".into()));
    }
    Ok(Session {
        token,
        name: v["name"].as_str().unwrap_or_default().to_string(),
        role: v["role"].as_str().unwrap_or_default().to_string(),
        until: v["until"].as_str().unwrap_or_default().to_string(),
    })
}

/// Olib kelish javobini o'qiydi.
pub fn parse_pull(body: &str) -> Result<Pulled, Error> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| Error::BadReply(e.to_string()))?;
    let mut out = Pulled {
        last: v["last"].as_i64().unwrap_or(0),
        more: v["more"].as_bool().unwrap_or(false),
        ..Default::default()
    };
    for item in v["changes"].as_array().into_iter().flatten() {
        let body = item["body"].as_str().unwrap_or_default();
        if body.trim().is_empty() {
            continue;
        }
        out.items.push(Incoming {
            id: item["id"].as_i64().unwrap_or(0),
            body: body.to_string(),
            author: item["author"].as_str().unwrap_or_default().to_string(),
            at: item["at"].as_str().unwrap_or_default().to_string(),
        });
    }
    Ok(out)
}

// ================================================================ Tarmoq

/// Kirish: login va parol bilan seans ochadi.
#[cfg(feature = "sync")]
pub fn login(url: &str, login: &str, password: &str) -> Result<Session, Error> {
    let cfg = Config {
        url: url.to_string(),
        ..Default::default()
    };
    let body = login_body(login, password);
    let text = send(&cfg, "POST", "api/login", None, Some(&body))?;
    parse_login(&text)
}

/// Paketni serverga yuboradi va uning raqamini qaytaradi.
#[cfg(feature = "sync")]
pub fn push(cfg: &Config, project: &str, body: &str, rows: i64) -> Result<i64, Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    let text = send(
        cfg,
        "POST",
        "api/push",
        Some(&cfg.token),
        Some(&push_body(project, body, rows)),
    )?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Error::BadReply(e.to_string()))?;
    Ok(v["id"].as_i64().unwrap_or(0))
}

/// Serverdan yangi paketlarni oladi.
#[cfg(feature = "sync")]
pub fn pull(cfg: &Config, project: &str, since: i64) -> Result<Pulled, Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    // Obyekt kodi manzilga tushadi — u maxfiy emas; belgi esa sarlavhada.
    let path = format!("api/pull?project={}&since={since}", urlencode(project));
    let text = send(cfg, "GET", &path, Some(&cfg.token), None)?;
    parse_pull(&text)
}

/// Signal ro'yxatini serverga yuboradi.
#[cfg(feature = "sync")]
pub fn send_notices(cfg: &Config, project: &str, items: &[NoticeOut]) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/notices",
        Some(&cfg.token),
        Some(&notices_body(project, items)),
    )?;
    Ok(())
}

/// Ishlar ro'yxatini serverga yuboradi.
#[cfg(feature = "sync")]
pub fn send_tasks(cfg: &Config, project: &str, items: &[TaskOut]) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/tasks",
        Some(&cfg.token),
        Some(&tasks_body(project, items)),
    )?;
    Ok(())
}

/// Ishchilar ro'yxatini serverga yuboradi.
#[cfg(feature = "sync")]
pub fn send_workers(cfg: &Config, project: &str, items: &[WorkerOut]) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/workers",
        Some(&cfg.token),
        Some(&workers_body(project, items)),
    )?;
    Ok(())
}

/// Obyekt yakunini serverga yuboradi.
#[cfg(feature = "sync")]
pub fn send_summary(cfg: &Config, project: &str, items: &[SummaryOut]) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/summary",
        Some(&cfg.token),
        Some(&summary_body(project, items)),
    )?;
    Ok(())
}

/// QR yorliqlar ro'yxatini serverga yuboradi.
#[cfg(feature = "sync")]
pub fn send_labels(cfg: &Config, project: &str, items: &[LabelOut]) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/labels",
        Some(&cfg.token),
        Some(&labels_body(project, items)),
    )?;
    Ok(())
}

/// Ofisdan xabar yuboradi (TZ VI.32).
#[cfg(feature = "sync")]
pub fn send_message(cfg: &Config, project: &str, text: &str) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/message",
        Some(&cfg.token),
        Some(&message_body(project, text)),
    )?;
    Ok(())
}

/// `since` dan keyingi xabarlarni oladi (TZ VI.32).
#[cfg(feature = "sync")]
pub fn fetch_messages(cfg: &Config, project: &str, since: i64) -> Result<Vec<MessageIn>, Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    let path = format!("api/messages?project={}&since={since}", urlencode(project));
    let body = send(cfg, "GET", &path, Some(&cfg.token), None)?;
    Ok(parse_messages(&body))
}

/// Imzo daftarining uchini serverga qayd etadi.
///
/// `Ok(Some(eski))` — shu uzunlik uchun serverda **boshqa** uch turibdi,
/// ya'ni daftar keyinchalik o'zgartirilgan.
#[cfg(feature = "sync")]
pub fn send_chain(
    cfg: &Config,
    project: &str,
    count: i64,
    head: &str,
) -> Result<Option<String>, Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    let body = send(
        cfg,
        "POST",
        "api/chain",
        Some(&cfg.token),
        Some(&chain_body(project, count, head)),
    )?;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    Ok(v["conflict"].as_str().map(|s| s.to_string()))
}

/// Serverdagi zanjir belgilarini oladi.
#[cfg(feature = "sync")]
pub fn fetch_chain(cfg: &Config, project: &str) -> Result<Vec<ChainMark>, Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    let path = format!("api/chain?project={}", urlencode(project));
    let body = send(cfg, "GET", &path, Some(&cfg.token), None)?;
    Ok(parse_chain(&body))
}

/// Hujjatni masofadan imzolaydi.
#[cfg(feature = "sync")]
pub fn sign(
    cfg: &Config,
    project: &str,
    document: &str,
    text: &str,
    rejected: &str,
) -> Result<(), Error> {
    if !cfg.ready() {
        return Err(Error::NotConfigured);
    }
    send(
        cfg,
        "POST",
        "api/sign",
        Some(&cfg.token),
        Some(&sign_body(project, document, text, rejected)),
    )?;
    Ok(())
}

/// So'rov yuboradi.
#[cfg(feature = "sync")]
fn send(
    cfg: &Config,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<String, Error> {
    let url = cfg.endpoint(path);
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(TIMEOUT))
        .build();
    let mut req = agent.request(method, &url);
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    let result = match body {
        Some(b) => req.set("Content-Type", "application/json").send_string(b),
        None => req.call(),
    };
    match result {
        Ok(resp) => resp
            .into_string()
            .map_err(|e| Error::BadReply(e.to_string())),
        Err(ureq::Error::Status(code, _)) => Err(error_for_status(code)),
        Err(e) => Err(Error::Transport(e.to_string())),
    }
}

/// Manzil uchun matnni xavfsiz ko'rinishga keltiradi.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Sinxronizatsiya natijasi — fon oqimidan qaytadi.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// Yuborilgan paket raqami (nol — yuborilmadi).
    pub pushed: i64,
    /// Olingan paketlar.
    pub pulled: Pulled,
    /// Yangi kelgan xabarlar (TZ VI.32).
    pub messages: Vec<MessageIn>,
    /// Serverda qayd etilgan zanjir belgilari.
    pub chain_marks: Vec<ChainMark>,
    /// Server shu uzunlik uchun boshqa uchni eslab qolgan bo'lsa — o'sha uch.
    pub chain_conflict: Option<String>,
}

/// Sinxronizatsiyaning yozishma qismi.
#[derive(Debug, Clone, Default)]
pub struct Chat {
    /// Yuboriladigan xabar; bo'sh bo'lsa hech narsa yuborilmaydi.
    pub outgoing: String,
    /// Qaysi raqamdan keyingi xabarlar so'ralsin.
    pub since: i64,
}

/// Fon oqimida yuboradi va oladi.
///
/// Interfeys qotib qolmasligi uchun tarmoq ishi alohida oqimda bajariladi;
/// natija kanal orqali qaytadi.
#[cfg(feature = "sync")]
pub fn spawn(
    cfg: Config,
    project: String,
    outgoing: Option<(String, i64)>,
    refs: Refs,
    chat: Chat,
) -> Receiver<Result<Outcome, Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let mut pushed = 0;
            if let Some((body, rows)) = outgoing {
                if !body.trim().is_empty() {
                    pushed = push(&cfg, &project, &body, rows)?;
                }
            }
            // Signal ro'yxati — har safar to'liq: bartaraf etilgani
            // telefonda ham yo'qolishi kerak.
            send_notices(&cfg, &project, &refs.notices)?;
            // Ish ro'yxati — telefonda tanlash uchun; grafik ofisda
            // yuritiladi, shuning uchun yo'nalish bir tomonlama.
            send_tasks(&cfg, &project, &refs.tasks)?;
            send_workers(&cfg, &project, &refs.workers)?;
            // Buyurtmachi kabineti uchun yakun: sonlar ilovada
            // hisoblanadi, kabinet faqat ko'rsatadi.
            send_summary(&cfg, &project, &refs.summary)?;
            // QR yorliqlar: telefon o'qigan kod nimaligini ko'rsatishi uchun.
            send_labels(&cfg, &project, &refs.labels)?;
            // Yozishma: avval yuboriladi, keyin olinadi — shunda o'z
            // xabaring darrov ro'yxatda ko'rinadi.
            if !chat.outgoing.trim().is_empty() {
                send_message(&cfg, &project, &chat.outgoing)?;
            }
            let messages = fetch_messages(&cfg, &project, chat.since)?;
            // Imzo daftarining uchi: dalilning bir uchi ilovadan
            // tashqarida tursin.
            let chain_conflict = send_chain(&cfg, &project, refs.chain.0, &refs.chain.1)?;
            let chain_marks = fetch_chain(&cfg, &project)?;
            let pulled = pull(&cfg, &project, cfg.last_pull)?;
            Ok(Outcome {
                pushed,
                pulled,
                messages,
                chain_marks,
                chain_conflict,
            })
        })();
        let _ = tx.send(result);
    });
    rx
}

/// Tarmoqsiz yig'ilishda sinxronizatsiya yo'q.
#[cfg(not(feature = "sync"))]
pub fn spawn(
    _cfg: Config,
    _project: String,
    _outgoing: Option<(String, i64)>,
    _refs: Refs,
    _chat: Chat,
) -> Receiver<Result<Outcome, Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = tx.send(Err(Error::NotConfigured));
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Yorliqlar so'rovi serverning kutgan shaklida bo'ladi.
    #[test]
    fn labels_are_sent_in_the_shape_the_server_expects() {
        let body = labels_body(
            "OBY-1",
            &[LabelOut {
                kind: "batch".into(),
                number: "P-12".into(),
                title: "Sement \"M400\"".into(),
                note: String::new(),
            }],
        );
        assert!(body.starts_with("{\"project\":\"OBY-1\""), "{body}");
        assert!(body.contains("\"kind\":\"batch\""), "{body}");
        assert!(body.contains("\"number\":\"P-12\""), "{body}");
        // Qo'shtirnoq qochiriladi: aks holda so'rov buzilardi.
        assert!(body.contains("Sement \\\"M400\\\""), "{body}");
        // Bo'sh ro'yxat ham to'g'ri shaklda ketadi — bu «yorliq yo'q» degani.
        assert!(labels_body("OBY-1", &[]).contains("\"items\":[]"));
    }

    /// Xabar so'rovi va javobi.
    #[test]
    fn messages_survive_a_broken_reply() {
        let body = message_body("OBY-1", "Sement tugadi");
        assert!(body.contains("\"text\":\"Sement tugadi\""), "{body}");

        let list = parse_messages(
            r#"{"messages":[
                {"id":7,"author":"Alisher","role":"foreman","text":"Sement tugadi","at":"2026-09-08T07:41:12Z"},
                {"id":8,"author":"Ofis","role":"pm","text":"","at":"2026-09-08T08:00:00Z"},
                {"id":0,"author":"X","role":"","text":"raqamsiz","at":""}
            ]}"#,
        );
        // Bo'sh matn va raqamsiz yozuv olinmaydi.
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, 7);
        assert_eq!(list[0].author, "Alisher");

        // Buzilgan javob xatoga olib kelmaydi — chat butun
        // sinxronizatsiyani to'xtatmasligi kerak.
        assert!(parse_messages("bu JSON emas").is_empty());
        assert!(parse_messages("{}").is_empty());
        assert!(parse_messages(r#"{"messages":"matn"}"#).is_empty());
    }

    /// Haqiqiy server bilan boshdan-oxir: kirish, yuborish, olish.
    ///
    /// Sinov `target` papkasidagi server dasturini ishga tushiradi. U
    /// yig'ilmagan bo'lsa sinov o'tkazib yuboriladi — bu muhit masalasi,
    /// kod xatosi emas. Shu sababli sinov `cargo test` da ham, faqat
    /// desktop yig'ilgan muhitda ham to'g'ri ishlaydi.
    #[cfg(feature = "sync")]
    #[test]
    fn desktop_and_server_talk_to_each_other() {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Server dasturi qayerda turishi mumkin.
        let exe = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join(if cfg!(windows) {
                "qurai-server.exe"
            } else {
                "qurai-server"
            });
        if !exe.exists() {
            return;
        }

        let dir = std::env::temp_dir().join(format!("qurai_e2e_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("server.db");
        let _ = std::fs::remove_file(&db);
        // Har ishga tushirishda boshqa port: sinovlar bir-biriga xalaqit
        // bermasin.
        let port = 34_000 + (std::process::id() % 2_000) as u16;
        let bind = format!("127.0.0.1:{port}");

        // ---- Foydalanuvchi qo'shamiz (parol standart oqimdan beriladi).
        let mut add = Command::new(&exe)
            .args(["--add-user", "prorab", "Alisher", "foreman"])
            .env("QURAI_DB", &db)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("qo'shish");
        if let Some(mut stdin) = add.stdin.take() {
            let _ = writeln!(stdin, "sinov-paroli-1");
        }
        assert!(
            add.wait().expect("kutish").success(),
            "foydalanuvchi qo'shilmadi"
        );

        // ---- Serverni ishga tushiramiz.
        let mut server = Command::new(&exe)
            .env("QURAI_DB", &db)
            .env("QURAI_BIND", &bind)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("server");

        // Server ko'tarilguncha kutamiz: ulanish urinishi bilan tekshiramiz.
        let url = format!("http://{bind}");
        let mut ready = false;
        for _ in 0..50 {
            if std::net::TcpStream::connect(&bind).is_ok() {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let result = (|| -> Result<(), String> {
            if !ready {
                return Err("server ko'tarilmadi".into());
            }

            // ---- Kirish
            let session =
                login(&url, "prorab", "sinov-paroli-1").map_err(|e| format!("kirish: {e:?}"))?;
            if session.role != "foreman" {
                return Err(format!("rol: {}", session.role));
            }

            let cfg = Config {
                enabled: true,
                url: url.clone(),
                login: "prorab".into(),
                token: session.token,
                last_pull: 0,
            };

            // ---- Paket yuboramiz
            let body = "#TABLE\tjournal\nsana\tmatn\n2026-09-04\tbeton quyildi\n";
            let id = push(&cfg, "OBY-1", body, 1).map_err(|e| format!("yuborish: {e:?}"))?;
            if id <= 0 {
                return Err("paket raqami yo'q".into());
            }

            // ---- Qaytarib olamiz
            let pulled = pull(&cfg, "OBY-1", 0).map_err(|e| format!("olish: {e:?}"))?;
            if pulled.items.len() != 1 {
                return Err(format!("paketlar soni: {}", pulled.items.len()));
            }
            if pulled.items[0].body != body {
                return Err("paket o'zgarib qolgan".into());
            }
            if pulled.items[0].author != "prorab" {
                return Err(format!("muallif: {}", pulled.items[0].author));
            }

            // ---- Belgidan keyin yangi paket yo'q
            let again = pull(&cfg, "OBY-1", pulled.last).map_err(|e| format!("{e:?}"))?;
            if !again.items.is_empty() {
                return Err("paket ikki marta keldi".into());
            }

            // ---- Signal ro'yxati yuboriladi
            let notices = vec![NoticeOut {
                code: "NT-1",
                severity: "major",
                title: "Muddati o'tgan ish".into(),
                detail: "3 ta ish".into(),
                count: 3,
                days: 5,
                source: "Grafik",
            }];
            send_notices(&cfg, "OBY-1", &notices).map_err(|e| format!("signal: {e:?}"))?;
            // Bo'sh ro'yxat ham qabul qilinadi: «hisoblandi, signal yo'q».
            send_notices(&cfg, "OBY-1", &[]).map_err(|e| format!("bo'sh signal: {e:?}"))?;

            // ---- Ishlar ro'yxati (telefonda tanlash uchun)
            let tasks = vec![TaskOut {
                wbs: "1.1".into(),
                name: "Yer ishlari".into(),
                start: "2026-09-01".into(),
                end: "2026-09-10".into(),
                progress: 40.0,
                section: "KJ".into(),
            }];
            send_tasks(&cfg, "OBY-1", &tasks).map_err(|e| format!("ishlar: {e:?}"))?;
            let workers = vec![WorkerOut {
                name: "Alisher".into(),
                position: "Beton quyuvchi".into(),
            }];
            send_workers(&cfg, "OBY-1", &workers).map_err(|e| format!("ishchilar: {e:?}"))?;
            let summary = vec![SummaryOut {
                module: "I.2".into(),
                indicator: "Bajarilish".into(),
                value: "42 %".into(),
            }];
            send_summary(&cfg, "OBY-1", &summary).map_err(|e| format!("yakun: {e:?}"))?;

            // ---- Prorab imzolamaydi: server rad etadi
            match sign(&cfg, "OBY-1", "AOSR-1", "matn", "") {
                Err(Error::Forbidden) => {}
                other => return Err(format!("imzo huquqi tekshirilmadi: {other:?}")),
            }

            // ---- Noto'g'ri parol bilan kirilmaydi
            match login(&url, "prorab", "boshqa-parol") {
                Err(Error::Auth) => {}
                other => return Err(format!("noto'g'ri parol o'tdi: {other:?}")),
            }
            Ok(())
        })();

        let _ = server.kill();
        let _ = server.wait();
        let _ = std::fs::remove_dir_all(&dir);
        result.expect("boshdan-oxir sinov");
    }

    /// Signal ro'yxati to'g'ri JSON bo'lib ketadi va matndagi belgilar
    /// so'rovni buzmaydi.
    #[test]
    fn notices_body_is_valid_json() {
        let items = vec![
            NoticeOut {
                code: "NT-1",
                severity: "critical",
                title: "Muddati o'tgan \"ish\"".into(),
                detail: "3 ta\tish".into(),
                count: 3,
                days: 5,
                source: "Grafik",
            },
            NoticeOut {
                code: "NT-2",
                severity: "major",
                title: "Sertifikat".into(),
                detail: String::new(),
                count: 1,
                days: 0,
                source: "Ombor",
            },
        ];
        let body = notices_body("OBY-1", &items);
        let v: serde_json::Value = serde_json::from_str(&body).expect("to'g'ri JSON");
        assert_eq!(v["project"], "OBY-1");
        let arr = v["items"].as_array().expect("ro'yxat");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "Muddati o'tgan \"ish\"");
        assert_eq!(arr[0]["days"], 5);
        assert_eq!(arr[1]["detail"], "");

        // Bo'sh ro'yxat ham to'g'ri so'rov: «hisoblandi, signal yo'q».
        let empty = notices_body("OBY-1", &[]);
        let v: serde_json::Value = serde_json::from_str(&empty).expect("to'g'ri JSON");
        assert!(v["items"].as_array().unwrap().is_empty());
    }

    #[test]
    fn config_is_not_ready_until_everything_is_set() {
        let mut c = Config::default();
        assert!(!c.ready());
        c.enabled = true;
        assert!(!c.ready());
        c.url = "http://10.0.0.5:8080".into();
        assert!(!c.ready());
        c.token = "belgi".into();
        assert!(c.ready());
        // Bo'sh joy manzilni to'ldirmaydi.
        c.url = "   ".into();
        assert!(!c.ready());
    }

    #[test]
    fn endpoint_joins_without_double_slash() {
        let c = Config {
            url: "http://server:8080/".into(),
            ..Default::default()
        };
        assert_eq!(c.endpoint("api/pull"), "http://server:8080/api/pull");
        assert_eq!(c.endpoint("/api/push"), "http://server:8080/api/push");
    }

    /// Parol so'rov tanasida ketadi va hech qayerga yozilmaydi.
    #[test]
    fn password_is_only_in_the_request_body() {
        let body = login_body("prorab", "maxfiy-parol");
        assert!(body.contains("maxfiy-parol"));

        // Sozlamada parol uchun joy yo'q — faqat belgi.
        let c = Config {
            enabled: true,
            url: "http://x".into(),
            login: "prorab".into(),
            token: "belgi".into(),
            last_pull: 5,
        };
        let text = format!("{c:?}");
        assert!(!text.contains("maxfiy-parol"));
    }

    /// Matndagi belgilar JSON ni buzmaydi.
    #[test]
    fn text_with_quotes_stays_valid_json() {
        let body = push_body("OBY-1", "qator\t\"tirnoq\"\nyangi", 3);
        let v: serde_json::Value = serde_json::from_str(&body).expect("to'g'ri JSON");
        assert_eq!(v["project"], "OBY-1");
        assert_eq!(v["body"], "qator\t\"tirnoq\"\nyangi");
        assert_eq!(v["rows"], 3);
    }

    #[test]
    fn login_reply_is_read() {
        let s = parse_login(
            r#"{"token":"abc","name":"Alisher","role":"foreman","until":"2026-09-18T00:00:00Z"}"#,
        )
        .expect("javob");
        assert_eq!(s.token, "abc");
        assert_eq!(s.role, "foreman");

        // Belgisiz javob qabul qilinmaydi.
        assert!(parse_login(r#"{"name":"x"}"#).is_err());
        assert!(parse_login("buzilgan").is_err());
    }

    #[test]
    fn pull_reply_is_read_and_empty_bodies_are_skipped() {
        let p = parse_pull(
            r#"{"changes":[
                {"id":1,"body":"birinchi","author":"a","at":"t"},
                {"id":2,"body":"   ","author":"a","at":"t"},
                {"id":3,"body":"uchinchi","author":"b","at":"t"}
            ],"last":3,"more":false}"#,
        )
        .expect("javob");
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].body, "birinchi");
        assert_eq!(p.items[1].id, 3);
        assert_eq!(p.last, 3);
        assert!(!p.more);
    }

    #[test]
    fn status_codes_become_clear_errors() {
        assert_eq!(error_for_status(401), Error::Auth);
        assert_eq!(error_for_status(403), Error::Forbidden);
        assert_eq!(error_for_status(500), Error::Service(500));
        assert!(!Error::Auth.retryable());
        assert!(Error::Service(500).retryable());
        assert!(Error::Transport("x".into()).retryable());
        // Har xatoning o'z xabari bor.
        for e in [
            Error::NotConfigured,
            Error::Auth,
            Error::Forbidden,
            Error::Service(500),
            Error::Transport("x".into()),
            Error::BadReply("x".into()),
        ] {
            assert_ne!(crate::i18n::t(e.key()), "?", "{:?}", e);
        }
    }

    #[test]
    fn url_encoding_keeps_the_address_safe() {
        assert_eq!(urlencode("OBY-1"), "OBY-1");
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("Дом/1"), "%D0%94%D0%BE%D0%BC%2F1");
    }
}
