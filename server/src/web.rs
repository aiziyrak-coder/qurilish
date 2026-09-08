//! Telefon brauzeri uchun ko'rinish (TZ VI.5, XIII.7, VIII.22).
//!
//! Bu **mobil klient emas**, mobil ko'rinish: oddiy sahifalar, telefon
//! ekraniga moslangan. Shunday qilingani ataylab — maydonchada ishlayotgan
//! odam ilova o'rnatmasdan, brauzerdan kirib ishlatadi va yangilanish
//! serverda bir joyda bo'ladi.
//!
//! Sahifalar ataylab sodda: JavaScript yo'q, faqat forma. Sekin internetda
//! ham ochiladi va nima yuborilayotgani ko'rinib turadi.

use crate::auth::{self, Access};
use crate::state::AppState;
use crate::store::{Signature, User};
use axum::extract::{Form, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::sync::Arc;

/// Seans belgisi shu nomdagi cookie da saqlanadi.
const COOKIE: &str = "qurai_session";

/// HTML ga tushadigan matnni xavfsiz ko'rinishga keltiradi.
///
/// Foydalanuvchi kiritgan har qanday matn shu yerdan o'tadi: aks holda
/// izohdagi belgi sahifani buzishi mumkin edi.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Sahifa qolipi: sarlavha va tarkib.
fn page(title: &str, body: &str) -> Html<String> {
    Html(format!(
        "<!doctype html><html lang=\"uz\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<meta name=\"theme-color\" content=\"#2563eb\">\
<link rel=\"manifest\" href=\"/manifest.webmanifest\">\
<link rel=\"icon\" href=\"/icon.svg\" type=\"image/svg+xml\">\
<title>{}</title><style>{}</style></head><body>{}{}</body></html>",
        esc(title),
        STYLE,
        body,
        SW_REGISTER
    ))
}

/// Uslub: bitta ustun, katta tugmalar — telefon uchun.
const STYLE: &str = "
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body { margin:0; padding:16px; font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
       max-width: 720px; margin-inline:auto; line-height:1.5; }
h1 { font-size:20px; margin:0 0 4px; }
h2 { font-size:16px; margin:20px 0 8px; }
.muted { color:#6b7280; font-size:13px; }
.card { border:1px solid #d1d5db; border-radius:10px; padding:12px; margin:10px 0; }
.row { display:flex; justify-content:space-between; gap:10px; align-items:center; }
input, select, textarea { width:100%; padding:10px; font-size:16px; border:1px solid #9ca3af;
       border-radius:8px; background:transparent; color:inherit; }
button { padding:12px 16px; font-size:16px; border-radius:8px; border:1px solid #2563eb;
       background:#2563eb; color:#fff; width:100%; margin-top:10px; }
button.ghost { background:transparent; color:inherit; border-color:#9ca3af; }
a { color:#2563eb; }
.err { color:#b91c1c; }
.ok { color:#15803d; }
label { display:block; margin-top:10px; font-size:13px; }
table { width:100%; border-collapse:collapse; font-size:14px; }
td, th { text-align:left; padding:6px 4px; border-bottom:1px solid #e5e7eb; }
";

/// Xizmat ishchisini ro'yxatdan o'tkazadigan kichik skript.
///
/// U bo'lmasa ham sahifalar ishlaydi: bu qo'shimcha qatlam, shart emas.
/// Har ochilganda navbat ham tekshiriladi — aloqa qaytgan bo'lsa
/// yuborilmagan yozuvlar o'zi ketadi.
const SW_REGISTER: &str = "<script>\
if('serviceWorker' in navigator){navigator.serviceWorker.register('/sw.js').then(function(r){\
if(navigator.serviceWorker.controller){navigator.serviceWorker.controller.postMessage('drain');}\
if(r.sync){try{r.sync.register('qurai-queue');}catch(e){}}}).catch(function(){});}\
</script>";

/// Formaga bir martalik belgi qo'yadi (TZ VI.28).
///
/// Belgi aloqasiz navbatdan qayta kelgan formani ushlash uchun: server
/// bir xil belgini ikkinchi marta qabul qilmaydi. Shu sababli
/// «yuborilmadi» deb qayta bosilgan tugma ham ikkita yozuv yaratmaydi.
fn nonce_field() -> String {
    format!(
        "<input type={q}hidden{q} name={q}nonce{q} value={q}{}{q}>",
        auth::new_token(),
        q = "\""
    )
}

/// Cookie dan seans belgisini oladi.
fn token_of(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE)
        .map(|(_, v)| v.to_string())
}

/// Sahifani ochayotgan odam.
fn viewer(state: &AppState, headers: &HeaderMap) -> Option<User> {
    let token = token_of(headers)?;
    state.store.session_user(&token, &crate::now())
}

/// Kirish sahifasi.
fn login_page(message: &str) -> Html<String> {
    let note = if message.is_empty() {
        String::new()
    } else {
        format!("<p class=\"err\">{}</p>", esc(message))
    };
    page(
        "QURAi — kirish",
        &format!(
            "<h1>QURAi</h1><p class=\"muted\">Qurilish maydonchasi uchun ko'rinish</p>{note}\
<form method=\"post\" action=\"/login\">\
<label>Login<input name=\"login\" autocomplete=\"username\" autocapitalize=\"none\" required></label>\
<label>Parol<input name=\"password\" type=\"password\" autocomplete=\"current-password\" required></label>\
<button type=\"submit\">Kirish</button></form>"
        ),
    )
}

/// `GET /`
pub async fn index(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    let projects = state.store.projects();
    let mut body = format!(
        "<div class=\"row\"><h1>Obyektlar</h1>\
<form method=\"post\" action=\"/logout\" style=\"width:auto\">\
<button class=\"ghost\" style=\"width:auto;margin:0\">Chiqish</button></form></div>\
<p class=\"muted\">{} · {}</p>",
        esc(&user.name),
        esc(&role_name(&user.role))
    );
    if projects.is_empty() {
        body.push_str("<p class=\"muted\">Hali biror qurilma ma'lumot yubormagan.</p>");
    }
    for (code, last, at) in projects {
        body.push_str(&format!(
            "<div class=\"card\"><div class=\"row\"><b>{}</b>\
<span class=\"muted\">#{last}</span></div>\
<div class=\"muted\">oxirgi yangilanish: {}</div>\
<a href=\"/o/{}\">Ochish</a></div>",
            esc(&code),
            esc(&at),
            esc(&code)
        ));
    }
    page("QURAi — obyektlar", &body).into_response()
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub login: String,
    pub password: String,
}

/// `POST /login`
pub async fn login(State(state): State<Arc<AppState>>, Form(form): Form<LoginForm>) -> Response {
    let Some((user, hash)) = state.store.user_by_login(&form.login) else {
        return login_page("Login yoki parol noto'g'ri").into_response();
    };
    if !user.active || !auth::verify_password(&form.password, &hash) {
        state
            .store
            .log(&user.login, "kirish-rad", "web", &crate::now());
        return login_page("Login yoki parol noto'g'ri").into_response();
    }
    let token = auth::new_token();
    state
        .store
        .open_session(user.id, &token, &crate::plus_days(auth::SESSION_DAYS));
    state.store.log(&user.login, "kirdi", "web", &crate::now());

    // `HttpOnly` — belgini sahifa kodidan o'qib bo'lmaydi.
    // `SameSite=Lax` — boshqa saytdan yuborilgan so'rovda ishlatilmaydi.
    let cookie = format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        auth::SESSION_DAYS * 24 * 3600
    );
    let mut resp = Redirect::to("/").into_response();
    if let Ok(v) = cookie.parse() {
        resp.headers_mut().insert(header::SET_COOKIE, v);
    }
    resp
}

/// `POST /logout`
pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(token) = token_of(&headers) {
        state.store.close_session(&token);
    }
    let mut resp = Redirect::to("/").into_response();
    if let Ok(v) = format!("{COOKIE}=; Path=/; HttpOnly; Max-Age=0").parse() {
        resp.headers_mut().insert(header::SET_COOKIE, v);
    }
    resp
}

/// `GET /o/{project}` — obyekt sahifasi.
pub async fn object(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    let last = state.store.last_change(&project);
    let changes = state.store.changes_since(&project, (last - 10).max(0), 10);
    let signatures = state.store.signatures(&project);

    let mut body = format!(
        "<div class=\"row\"><h1>{}</h1><a href=\"/\">Ortga</a></div>\
<p class=\"muted\">{} · {}</p>",
        esc(&project),
        esc(&user.name),
        esc(&role_name(&user.role))
    );

    // ---- Bugun nima qilish kerak (ilova hisoblagan signal)
    let (notices, computed) = state.store.notices(&project);
    if notices.is_empty() {
        body.push_str(&format!(
            "<div class=\"card\"><b>Bugun nima qilish kerak</b>\
<div class=\"muted\">{}</div></div>",
            if computed.is_empty() {
                "Ilova hali signal yubormagan.".to_string()
            } else {
                format!("Signal yo'q · {}", esc(&computed))
            }
        ));
    } else {
        body.push_str(&format!(
            "<h2>Bugun nima qilish kerak</h2>\
<p class=\"muted\">Ilova hisoblab yuborgan ro'yxat · {}</p>",
            esc(&computed)
        ));
        for n in notices.iter().take(10) {
            let colour = match n.severity.as_str() {
                "critical" => "#b91c1c",
                "major" => "#b45309",
                _ => "#2563eb",
            };
            let extra = if n.days > 0 {
                format!("{} kun · {}", n.days, n.count)
            } else {
                n.count.to_string()
            };
            body.push_str(&format!(
                "<div class=\"card\"><div class=\"row\">\
<b style=\"color:{colour}\">{}</b><span class=\"muted\">{}</span></div>\
<div class=\"muted\">{} · {}</div></div>",
                esc(&n.title),
                esc(&extra),
                esc(&n.source),
                esc(&n.detail)
            ));
        }
    }

    // ---- Kunlik yozuv havolasi (yozish huquqi borlarga)
    if auth::can(&user.role, Access::Write) {
        body.push_str(&format!(
            "<div class=\"card\"><b>Kunlik yozuv</b><div class=\"muted\">Maydonchadan to'g'ridan-to'g'ri kiritish</div><a href=\"/o/{p}/journal\">Ochish</a></div><div class=\"card\"><b>Tabel</b><div class=\"muted\">Bugungi soat va kun turi</div><a href=\"/o/{p}/timesheet\">Ochish</a></div><div class=\"card\"><b>Ishlar</b><div class=\"muted\">Muddat va bajarilish</div><a href=\"/o/{p}/tasks\">Ochish</a></div><div class=\"card\"><b>Kirish / chiqish</b><div class=\"muted\">Maydonchaga kelish va ketish belgisi</div><a href=\"/o/{p}/checkin\">Ochish</a></div><div class=\"card\"><b>QR o'qish</b><div class=\"muted\">Yorliqdagi kod nimaligini ko'rsatadi</div><a href=\"/o/{p}/scan\">Ochish</a></div><div class=\"card\"><b>Ofis bilan yozishma</b><div class=\"muted\">Savol bering, javobni shu yerda oling</div><a href=\"/o/{p}/chat\">Ochish</a></div>",
            p = esc(&project)
        ));
    }

    // ---- Buyurtmachi kabineti: hamma rolga ko'rinadi, chunki u faqat
    // o'qish uchun va obyekt holatini bir qarashda beradi.
    body.push_str(&format!(
        "<div class=\"card\"><b>Obyekt holati</b><div class=\"muted\">Ko'rsatkichlar va imzolangan hujjatlar</div><a href=\"/o/{}/client\">Ochish</a></div>",
        esc(&project)
    ));

    // ---- Oxirgi o'zgarishlar
    body.push_str("<h2>Oxirgi o'zgarishlar</h2>");
    if changes.is_empty() {
        body.push_str("<p class=\"muted\">Hali o'zgarish yo'q.</p>");
    } else {
        body.push_str("<table><tr><th>№</th><th>Kim</th><th>Qachon</th><th>Qator</th></tr>");
        for c in changes.iter().rev() {
            body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                c.id,
                esc(&c.author),
                esc(&c.at),
                c.rows
            ));
        }
        body.push_str("</table>");
    }

    // ---- Imzo
    body.push_str("<h2>Imzolar</h2>");
    if signatures.is_empty() {
        body.push_str("<p class=\"muted\">Hali imzo yo'q.</p>");
    } else {
        body.push_str("<table><tr><th>Hujjat</th><th>Kim</th><th>Qachon</th><th>Holat</th></tr>");
        for s in signatures.iter().take(20) {
            body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"{}\">{}</td></tr>",
                esc(&s.document),
                esc(&s.user),
                esc(&s.at),
                if s.rejected.is_empty() { "ok" } else { "err" },
                if s.rejected.is_empty() {
                    "tasdiqlandi".to_string()
                } else {
                    esc(&s.rejected)
                }
            ));
        }
        body.push_str("</table>");
    }

    // ---- Imzolash formasi (faqat huquqi borlarga)
    if auth::can(&user.role, Access::Sign) {
        body.push_str(&format!(
            "<h2>Hujjatni imzolash</h2>\
<p class=\"muted\">Imzo hujjat matniga bog'lanadi: matn keyin o'zgarsa, bu ko'rinadi.</p>\
<form method=\"post\" action=\"/o/{}/sign\">\
<label>Hujjat raqami<input name=\"document\" required></label>\
<label>Hujjat matni yoki mazmuni<textarea name=\"text\" rows=\"4\" required></textarea></label>\
<label>Rad etish sababi (bo'sh bo'lsa — tasdiqlash)<input name=\"rejected\"></label>\
<button type=\"submit\">Imzolash</button></form>",
            esc(&project)
        ));
    } else {
        body.push_str(
            "<p class=\"muted\">Sizning rolingiz hujjat imzolamaydi — bu ko'rish uchun.</p>",
        );
    }

    page(&format!("QURAi — {project}"), &body).into_response()
}

#[derive(Deserialize)]
pub struct SignForm {
    pub document: String,
    pub text: String,
    #[serde(default)]
    pub rejected: String,
}

/// `POST /o/{project}/sign`
pub async fn sign(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    Form(form): Form<SignForm>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Sign) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Sizning rolingiz hujjat imzolamaydi.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    if form.document.trim().is_empty() || form.text.trim().is_empty() {
        return page(
            "QURAi",
            "<p class=\"err\">Hujjat raqami va matni to'ldirilmagan.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }

    let sig = Signature {
        id: 0,
        project: project.clone(),
        document: form.document.trim().to_string(),
        user: user.login.clone(),
        role: user.role.clone(),
        at: crate::now(),
        digest: auth::digest(form.text.trim()),
        rejected: form.rejected.trim().to_string(),
    };
    let _ = state.store.sign(&sig);
    state.store.log(
        &user.login,
        if sig.rejected.is_empty() {
            "imzo"
        } else {
            "rad"
        },
        &format!("{} / {}", sig.project, sig.document),
        &crate::now(),
    );
    Redirect::to(&format!("/o/{project}")).into_response()
}

// ================================================================ Kunlik yozuv

/// Telefondan kiritilgan kunlik yozuv (TZ V, VI.5).
#[derive(Deserialize)]
pub struct JournalForm {
    pub date: String,
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub volume: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub workers: String,
    #[serde(default)]
    pub machines: String,
    #[serde(default)]
    pub weather: String,
    pub text: String,
    #[serde(default)]
    pub remarks: String,
    /// Formaning bir martalik belgisi (aloqasiz navbat uchun).
    #[serde(default)]
    pub nonce: String,
    /// Telefon bergan koordinata: `kenglik,uzunlik,aniqlik`. Brauzer
    /// ruxsat bermasa yoki JavaScript o'chirilgan bo'lsa — bo'sh keladi
    /// va yozuv baribir yuboriladi.
    #[serde(default)]
    pub gps: String,
}

/// Paket qatoridagi maydonni xavfsiz ko'rinishga keltiradi.
///
/// Ajratgich — tabulyatsiya, shuning uchun matndagi tabulyatsiya va qator
/// ko'chirish qochiriladi. Qoida desktop ilovaning `package` moduli bilan
/// bir xil: aks holda paket u yerda buzilib o'qilardi.
fn pkg_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "")
}

/// Kunlik yozuvdan desktop ilova tushunadigan paket tuzadi.
///
/// Server yangi format o'ylab topmaydi: bu aynan ilovaning almashish
/// formati, shuning uchun telefondan kelgan yozuv ofisdagi bazaga oddiy
/// paket kabi qo'shiladi.
pub fn journal_package(project: &str, author: &str, at: &str, f: &JournalForm) -> String {
    let cols = [
        "date",
        "author",
        "weather",
        "temperature",
        "workers",
        "machines",
        "task",
        "volume",
        "unit",
        "text",
        "remarks",
        "gps",
    ];
    // Bo'sh son — nol emas, «kiritilmagan». Paketda nol bo'lib ketmasligi
    // uchun raqamsiz maydon bo'sh qoldirilmaydi: ilova uni nol deb o'qiydi
    // va buni bilib turamiz.
    let num = |s: &str| {
        let v = s.trim().replace(',', ".");
        if v.is_empty() {
            "0".to_string()
        } else {
            v
        }
    };
    let row = [
        f.date.trim().to_string(),
        author.to_string(),
        f.weather.trim().to_string(),
        "0".to_string(),
        num(&f.workers),
        num(&f.machines),
        f.task.trim().to_string(),
        num(&f.volume),
        f.unit.trim().to_string(),
        f.text.trim().to_string(),
        f.remarks.trim().to_string(),
        clean_gps(&f.gps),
    ];
    format!(
        "QURAI-PACKAGE\t1\nPROJECT\t{}\nCREATED\t{}\n\n#journal\n{}\n{}\n",
        pkg_escape(project),
        pkg_escape(at),
        cols.join("\t"),
        row.iter()
            .map(|c| pkg_escape(c))
            .collect::<Vec<_>>()
            .join("\t")
    )
}

/// Brauzerdan kelgan koordinatani tozalaydi.
///
/// Serverga kelgan matnga ishonilmaydi: bu yerdan faqat
/// `kenglik,uzunlik[,aniqlik]` ko'rinishidagi haqiqiy son o'tadi. Noto'g'ri
/// bo'lsa — bo'sh qaytadi, ya'ni «joy noma'lum». Yolg'on koordinatadan
/// ko'ra koordinatasiz yozuv yaxshiroq.
pub fn clean_gps(raw: &str) -> String {
    let nums: Vec<f64> = raw
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.trim().is_empty())
        .take(3)
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    let (Some(lat), Some(lon)) = (nums.first().copied(), nums.get(1).copied()) else {
        return String::new();
    };
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return String::new();
    }
    // Nol-nol — okeandagi nuqta; amalda «to'ldirilmagan» degani.
    if lat.abs() < 1e-9 && lon.abs() < 1e-9 {
        return String::new();
    }
    match nums.get(2) {
        Some(acc) if *acc > 0.0 => format!("{lat:.6},{lon:.6},{acc:.0}"),
        _ => format!("{lat:.6},{lon:.6}"),
    }
}

/// Joyni telefondan so'raydigan bo'lak (TZ V.13, VI.19, XIII.27).
///
/// Sahifalar ataylab sodda va bu yerda ham shunday qoladi: JavaScript
/// **shart emas**. Skript faqat yashirin maydonni to'ldiradi — u ishlamasa
/// forma xuddi avvalgidek yuboriladi, shunchaki koordinatasiz. Odam nima
/// yuborilayotganini ko'rib turadi: holat matni ekranda yoziladi.
///
/// Koordinatani brauzer **ruxsat so'rab** beradi va u faqat shu formaga
/// qo'shiladi; boshqa hech qayerga yuborilmaydi.
fn geo_block() -> String {
    let script = "(function(){\
var o=document.getElementById('gps'),t=document.getElementById('geotext');\
if(!o||!t){return;}\
if(!navigator.geolocation){t.textContent='Joy: bu brauzer koordinata bermaydi.';return;}\
t.textContent='Joy: aniqlanmoqda\u{2026}';\
navigator.geolocation.getCurrentPosition(function(p){\
var c=p.coords,a=Math.round(c.accuracy||0);\
o.value=c.latitude.toFixed(6)+','+c.longitude.toFixed(6)+','+a;\
t.textContent='Joy: '+c.latitude.toFixed(5)+', '+c.longitude.toFixed(5)+' (\u{b1}'+a+' m)';\
},function(){t.textContent='Joy: aniqlanmadi \u{2014} yozuv koordinatasiz yuboriladi.';},\
{enableHighAccuracy:true,timeout:10000,maximumAge:60000});})();";
    format!(
        "<input type=\"hidden\" name=\"gps\" id=\"gps\">\
<p class=\"muted\" id=\"geotext\">Joy: brauzer ruxsat so'raydi. Ruxsat bermasangiz ham yozuv yuboriladi.</p>\
<script>{script}</script>"
    )
}

/// `GET /o/{project}/journal` — kunlik yozuv formasi.
pub async fn journal_form(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Sizning rolingiz kunlik yozuv kiritmaydi.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    let today = crate::now().split('T').next().unwrap_or("").to_string();
    // Ish grafikdan tanlanadi: qo'lda yozilgan nom keyin grafikka
    // ulanmay qolardi. Ro'yxat bo'sh bo'lsa — oddiy maydon.
    let tasks = state.store.tasks(&project);
    let task_field = if tasks.is_empty() {
        "<label>Ish (grafikdagi nomi)<input name=\"task\" placeholder=\"Monolit karkas, 4-6 qavat\"></label>"
            .to_string()
    } else {
        let options: String = tasks
            .iter()
            .map(|t| {
                let label = if t.wbs.trim().is_empty() {
                    t.name.clone()
                } else {
                    format!("{} {}", t.wbs, t.name)
                };
                format!(
                    "<option value=\"{}\">{}</option>",
                    esc(&t.name),
                    esc(&label)
                )
            })
            .collect();
        format!(
            "<label>Ish<select name=\"task\"><option value=\"\">— tanlanmagan —</option>{options}</select></label>"
        )
    };
    let body = format!(
        "<div class=\"row\"><h1>Kunlik yozuv</h1><a href=\"/o/{p}\">Ortga</a></div>\
<p class=\"muted\">{obj} · {name}</p>\
<form method=\"post\" action=\"/o/{p}/journal\">\
<label>Sana<input type=\"date\" name=\"date\" value=\"{today}\" required></label>\
{task_field}\
<div class=\"row\"><label style=\"flex:1\">Hajm<input name=\"volume\" inputmode=\"decimal\"></label>\
<label style=\"flex:1\">Birlik<input name=\"unit\" placeholder=\"m3\"></label></div>\
<div class=\"row\"><label style=\"flex:1\">Ishchi<input name=\"workers\" inputmode=\"numeric\"></label>\
<label style=\"flex:1\">Texnika<input name=\"machines\" inputmode=\"numeric\"></label></div>\
<label>Ob-havo<input name=\"weather\"></label>\
<label>Nima qilindi<textarea name=\"text\" rows=\"3\" required></textarea></label>\
<label>Muammo yoki izoh<textarea name=\"remarks\" rows=\"2\"></textarea></label>\
{geo}{nonce}\
<button type=\"submit\">Yuborish</button></form>\
<p class=\"muted\">Yozuv obyekt paketiga qo'shiladi va ofisdagi ilova uni keyingi sinxronizatsiyada oladi.</p>",
        p = esc(&project),
        obj = esc(&project),
        name = esc(&user.name),
        today = esc(&today),
        task_field = task_field,
        geo = geo_block(),
        nonce = nonce_field(),
    );
    page("QURAi — kunlik yozuv", &body).into_response()
}

/// `POST /o/{project}/journal`
pub async fn journal_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    Form(form): Form<JournalForm>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Huquq yo'q.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    if form.date.trim().is_empty() || form.text.trim().is_empty() {
        return page(
            "QURAi",
            "<p class=\"err\">Sana va bajarilgan ish to'ldirilmagan.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }

    let at = crate::now();
    // Aloqasiz navbatdan qayta kelgan forma ikkinchi yozuv yaratmaydi.
    if !form.nonce.trim().is_empty() && !state.store.use_nonce(&form.nonce, &at) {
        return Redirect::to(&format!("/o/{project}")).into_response();
    }
    let body = journal_package(&project, &user.name, &at, &form);
    let _ = state
        .store
        .push_change(project.trim(), &body, &user.login, &at, 1);
    state
        .store
        .log(&user.login, "kunlik-yozuv", project.trim(), &at);
    Redirect::to(&format!("/o/{project}")).into_response()
}

// ================================================================ Tabel

/// Telefondan kiritilgan tabel (TZ XIII.7).
///
/// Forma ishchilar ro'yxati bo'yicha to'ldiriladi: har ishchi uchun soat
/// va kun turi. Bo'sh qoldirilgan qator **yuborilmaydi** — bo'sh katak
/// «ishlamagan» degani emas, «to'ldirilmagan» degani.
#[derive(Deserialize)]
pub struct TimesheetForm {
    pub date: String,
    /// `hours[<ishchi nomi>]` ko'rinishidagi maydonlar.
    #[serde(flatten)]
    pub fields: std::collections::BTreeMap<String, String>,
}

/// Tabel paketini tuzadi.
///
/// Ustunlar desktop ilovaning paketidagi bilan aynan bir xil: sana,
/// ishchi, soat, kun turi va smena.
pub fn timesheet_package(
    project: &str,
    at: &str,
    date: &str,
    rows: &[(String, String, String)],
) -> String {
    let mut out = format!(
        "QURAI-PACKAGE\t1\nPROJECT\t{}\nCREATED\t{}\n\n#timesheet\ndate\tworker\thours\tkind\tshift\n",
        pkg_escape(project),
        pkg_escape(at)
    );
    for (worker, hours, kind) in rows {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\tday\n",
            pkg_escape(date),
            pkg_escape(worker),
            pkg_escape(hours),
            pkg_escape(kind)
        ));
    }
    out
}

/// `GET /o/{project}/timesheet`
pub async fn timesheet_form(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Sizning rolingiz tabel to'ldirmaydi.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }

    let workers = state.store.workers(&project);
    if workers.is_empty() {
        return page(
            "QURAi — tabel",
            &format!(
                "<div class=\"row\"><h1>Tabel</h1><a href=\"/o/{p}\">Ortga</a></div>\
<p class=\"muted\">Ishchilar ro'yxati hali kelmagan. Ofisdagi ilova sinxronizatsiya qilgach, \
ro'yxat shu yerda chiqadi.</p>",
                p = esc(&project)
            ),
        )
        .into_response();
    }

    let today = crate::now().split('T').next().unwrap_or("").to_string();
    let mut rows = String::new();
    for w in &workers {
        rows.push_str(&format!(
            "<div class=\"card\"><div class=\"row\"><b>{}</b>\
<span class=\"muted\">{}</span></div>\
<div class=\"row\">\
<label style=\"flex:1\">Soat<input name=\"h_{key}\" inputmode=\"decimal\" placeholder=\"—\"></label>\
<label style=\"flex:1\">Kun turi<select name=\"k_{key}\">\
<option value=\"work\">Ish</option>\
<option value=\"downtime\">Bo'sh turish</option>\
<option value=\"absent\">Kelmadi</option>\
<option value=\"sick\">Kasal</option>\
<option value=\"vacation\">Ta'til</option>\
<option value=\"trip\">Xizmat safari</option>\
</select></label></div></div>",
            esc(&w.name),
            esc(&w.position),
            key = esc(&w.name)
        ));
    }

    let body = format!(
        "<div class=\"row\"><h1>Tabel</h1><a href=\"/o/{p}\">Ortga</a></div>\
<p class=\"muted\">{obj} · {name}</p>\
<form method=\"post\" action=\"/o/{p}/timesheet\">\
{nonce}\
<label>Sana<input type=\"date\" name=\"date\" value=\"{today}\" required></label>\
{rows}\
<button type=\"submit\">Yuborish</button></form>\
<p class=\"muted\">Soat kiritilmagan ishchi yuborilmaydi: bo'sh katak «ishlamagan» emas, \
«to'ldirilmagan» degani.</p>",
        p = esc(&project),
        obj = esc(&project),
        name = esc(&user.name),
        today = esc(&today),
        rows = rows,
        nonce = nonce_field(),
    );
    page("QURAi — tabel", &body).into_response()
}

/// `POST /o/{project}/timesheet`
pub async fn timesheet_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    Form(form): Form<TimesheetForm>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Huquq yo'q.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    if form.date.trim().is_empty() {
        return page(
            "QURAi",
            "<p class=\"err\">Sana to'ldirilmagan.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }

    // `h_<ism>` — soat, `k_<ism>` — kun turi.
    let mut rows: Vec<(String, String, String)> = Vec::new();
    for (key, value) in &form.fields {
        let Some(worker) = key.strip_prefix("h_") else {
            continue;
        };
        let hours = value.trim().replace(',', ".");
        if hours.is_empty() {
            continue;
        }
        // Son bo'lmagan qiymat qabul qilinmaydi: noto'g'ri soat tabelga
        // nol bo'lib tushib, ish haqini buzardi.
        if hours.parse::<f64>().is_err() {
            continue;
        }
        let kind = form
            .fields
            .get(&format!("k_{worker}"))
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| "work".to_string());
        rows.push((worker.to_string(), hours, kind));
    }

    if rows.is_empty() {
        return page(
            "QURAi",
            "<p class=\"err\">Hech kimga soat kiritilmadi.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }

    let at = crate::now();
    // Aloqasiz navbatdan qayta kelgan forma ikkinchi yozuv yaratmaydi.
    let nonce = form.fields.get("nonce").cloned().unwrap_or_default();
    if !nonce.trim().is_empty() && !state.store.use_nonce(&nonce, &at) {
        return Redirect::to(&format!("/o/{project}")).into_response();
    }
    let body = timesheet_package(&project, &at, form.date.trim(), &rows);
    let _ = state
        .store
        .push_change(project.trim(), &body, &user.login, &at, rows.len() as i64);
    state.store.log(&user.login, "tabel", project.trim(), &at);
    Redirect::to(&format!("/o/{project}")).into_response()
}

// ================================================================ Grafik

/// `GET /o/{project}/tasks` — ishlar grafigi telefonda (faqat ko'rish).
///
/// Grafikning chizmasi telefonda ko'rsatilmaydi: kichik ekranda u
/// o'qilmaydi. Buning o'rniga muddat bo'yicha tartiblangan ro'yxat
/// beriladi — maydonchada aynan shu kerak: nima ketyapti va nimaning
/// muddati o'tgan.
pub async fn tasks_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    let tasks = state.store.tasks(&project);
    let today = crate::now().split('T').next().unwrap_or("").to_string();

    let mut body = format!(
        "<div class=\"row\"><h1>Ishlar</h1><a href=\"/o/{p}\">Ortga</a></div>\
<p class=\"muted\">{obj} · {name}</p>",
        p = esc(&project),
        obj = esc(&project),
        name = esc(&user.name)
    );

    if tasks.is_empty() {
        body.push_str(
            "<p class=\"muted\">Ishlar ro'yxati hali kelmagan. Ofisdagi ilova \
sinxronizatsiya qilgach, ro'yxat shu yerda chiqadi.</p>",
        );
        return page("QURAi — ishlar", &body).into_response();
    }

    // Tugallanmaganlari oldinda, muddat bo'yicha.
    let mut list = tasks;
    list.sort_by(|a, b| {
        (a.progress >= 99.99)
            .cmp(&(b.progress >= 99.99))
            .then(a.end.cmp(&b.end))
    });

    body.push_str("<table><tr><th>Ish</th><th>Muddat</th><th>%</th></tr>");
    for t in list.iter().take(100) {
        // Muddati o'tgan va tugallanmagan ish ajratib ko'rsatiladi.
        let late = !t.end.is_empty() && t.end < today && t.progress < 99.99;
        let name = if t.wbs.trim().is_empty() {
            t.name.clone()
        } else {
            format!("{} {}", t.wbs, t.name)
        };
        body.push_str(&format!(
            "<tr><td>{}<div class=\"muted\">{}</div></td>\
<td class=\"{}\">{}</td><td>{:.0}</td></tr>",
            esc(&name),
            esc(&t.section),
            if late { "err" } else { "" },
            esc(&short_date(&t.end)),
            t.progress
        ));
    }
    body.push_str("</table>");
    page("QURAi — ishlar", &body).into_response()
}

/// `2026-09-07` → `07.09`. Bo'sh yoki boshqa ko'rinishdagi sana
/// o'zgarmasdan qoladi: taxmin qilib format o'zgartirish xato beradi.
fn short_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() == 3 && parts[1].len() == 2 && parts[2].len() == 2 {
        format!("{}.{}", parts[2], parts[1])
    } else {
        iso.to_string()
    }
}

// ================================================================ Buyurtmachi

/// `GET /o/{project}/client` — buyurtmachi kabineti (TZ VIII.35).
///
/// Kabinet **faqat o'qish uchun**: bu yerdan hech narsa o'zgartirilmaydi.
/// Sonlar ilova hisoblab yuborgan yakundan olinadi — pudratchi ekranidagi
/// son bilan bir xil bo'lishi shart, aks holda ikki tomon ikki xil raqam
/// bilan gaplashadi.
pub async fn client_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    let (rows, at) = state.store.summary(&project);
    let signatures = state.store.signatures(&project);

    let mut body = format!(
        "<div class=\"row\"><h1>{}</h1><a href=\"/\">Ortga</a></div>\
<p class=\"muted\">{} · {}</p>",
        esc(&project),
        esc(&user.name),
        esc(&role_name(&user.role))
    );

    if rows.is_empty() {
        body.push_str(
            "<p class=\"muted\">Yakun hali yuborilmagan. Ofisdagi ilova \
sinxronizatsiya qilgach, ko'rsatkichlar shu yerda chiqadi.</p>",
        );
        return page("QURAi — buyurtmachi", &body).into_response();
    }

    body.push_str(&format!(
        "<h2>Obyekt holati</h2><p class=\"muted\">Ilova hisoblab yuborgan · {}</p>\
<table><tr><th>Bo'lim</th><th>Ko'rsatkich</th><th>Qiymat</th></tr>",
        esc(&at)
    ));
    for r in &rows {
        body.push_str(&format!(
            "<tr><td class=\"muted\">{}</td><td>{}</td><td><b>{}</b></td></tr>",
            esc(&r.module),
            esc(&r.indicator),
            esc(&r.value)
        ));
    }
    body.push_str("</table>");

    // Imzolar buyurtmachiga ham ko'rinadi: hujjat kim tomonidan
    // tasdiqlanganini bilish uning haqqi.
    if !signatures.is_empty() {
        body.push_str("<h2>Imzolangan hujjatlar</h2><table><tr><th>Hujjat</th><th>Kim</th><th>Qachon</th></tr>");
        for s in signatures.iter().filter(|s| s.rejected.is_empty()).take(20) {
            body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc(&s.document),
                esc(&s.user),
                esc(&s.at)
            ));
        }
        body.push_str("</table>");
    }
    body.push_str(
        "<p class=\"muted\">Bu sahifa faqat ko'rish uchun: bu yerdan hech narsa \
o'zgartirilmaydi.</p>",
    );
    page("QURAi — buyurtmachi", &body).into_response()
}

// ================================================================ PWA

/// Telefonga o'rnatiladigan ilova tavsifi (TZ VI.1, XI.38).
///
/// Bu **native ilova emas** va shunday deyilmaydi. Bu — brauzer
/// o'rnatadigan sahifa: bosh ekranda belgi paydo bo'ladi, o'z oynasida
/// ochiladi va aloqa yo'qolganda ham ishlaydi. Native ilova do'konga
/// chiqarish, imzo va alohida loyihani talab qiladi; bu esa bugun
/// ishlaydi va yangilanish serverda bir joyda bo'ladi.
pub async fn manifest() -> Response {
    let body = r##"{
  "name": "QURAi — qurilish maydonchasi",
  "short_name": "QURAi",
  "start_url": "/",
  "scope": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#2563eb",
  "lang": "uz",
  "icons": [
    { "src": "/icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "any" },
    { "src": "/icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "maskable" }
  ]
}"##;
    ([(header::CONTENT_TYPE, "application/manifest+json")], body).into_response()
}

/// Ilova belgisi.
///
/// SVG ataylab: bitta fayl har o'lchamda aniq chiqadi va omborda ikkilik
/// rasm saqlanmaydi.
pub async fn icon() -> Response {
    let body = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192">
<rect width="192" height="192" rx="36" fill="#2563eb"/>
<path d="M40 132V60l56-24 56 24v72" fill="none" stroke="#fff" stroke-width="12" stroke-linejoin="round"/>
<path d="M40 132h112" stroke="#fff" stroke-width="12" stroke-linecap="round"/>
<path d="M74 132V92h44v40" fill="none" stroke="#fff" stroke-width="12" stroke-linejoin="round"/>
</svg>"##;
    ([(header::CONTENT_TYPE, "image/svg+xml")], body).into_response()
}

/// Aloqa yo'q paytida ko'rsatiladigan sahifa.
pub async fn offline_page() -> Response {
    page(
        "QURAi — aloqa yo'q",
        "<h1>Aloqa yo'q</h1>\
<p class=\"muted\">Bu sahifa hali ochilmagan, shuning uchun xotirada ham yo'q. \
Avval ochilgan sahifalar aloqasiz ham ochiladi.</p>\
<p class=\"muted\">Yuborilmagan yozuvlar navbatda turadi va aloqa qaytganda o'zi ketadi.</p>\
<p><a href=\"/\">Bosh sahifa</a></p>",
    )
    .into_response()
}

/// Xizmat ishchisi: aloqasiz ishlash (TZ VI.28, VI.38).
///
/// Ikki vazifa bajaradi:
///
/// 1. **O'qish** — ochilgan sahifa xotirada qoladi va aloqa yo'qolganda
///    o'sha ko'rinishda ochiladi. Eskirgan ma'lumot ekanini sahifaning
///    o'zi aytadi, chunki unda sana turadi.
/// 2. **Yozish** — yuborilmagan forma navbatga tushadi va aloqa
///    qaytganda o'zi ketadi. Takror yuborilishdan `nonce` himoya qiladi:
///    server bir xil belgini ikkinchi marta qabul qilmaydi.
///
/// Shu sababli maydonchada «yubordim, lekin yo'qoldi» holati bo'lmaydi va
/// «ikki marta yozildi» ham bo'lmaydi.
pub async fn service_worker() -> Response {
    let js = r#"
const CACHE = 'qurai-v1';
const OFFLINE = '/offline';

self.addEventListener('install', e => {
  e.waitUntil(caches.open(CACHE).then(c => c.addAll([OFFLINE, '/icon.svg'])).then(() => self.skipWaiting()));
});
self.addEventListener('activate', e => {
  e.waitUntil(caches.keys().then(ks => Promise.all(ks.filter(k => k !== CACHE).map(k => caches.delete(k)))).then(() => self.clients.claim()));
});

// ---- Navbat: yuborilmagan formalar IndexedDB da turadi.
function db() {
  return new Promise((ok, no) => {
    const r = indexedDB.open('qurai-queue', 1);
    r.onupgradeneeded = () => r.result.createObjectStore('q', { autoIncrement: true });
    r.onsuccess = () => ok(r.result);
    r.onerror = () => no(r.error);
  });
}
async function enqueue(item) {
  const d = await db();
  await new Promise((ok, no) => {
    const tx = d.transaction('q', 'readwrite');
    tx.objectStore('q').add(item);
    tx.oncomplete = ok; tx.onerror = () => no(tx.error);
  });
}
async function drain() {
  const d = await db();
  const items = await new Promise((ok, no) => {
    const tx = d.transaction('q', 'readonly');
    const rq = tx.objectStore('q').getAll();
    rq.onsuccess = () => ok(rq.result || []);
    rq.onerror = () => no(rq.error);
  });
  const keys = await new Promise((ok, no) => {
    const tx = d.transaction('q', 'readonly');
    const rq = tx.objectStore('q').getAllKeys();
    rq.onsuccess = () => ok(rq.result || []);
    rq.onerror = () => no(rq.error);
  });
  for (let i = 0; i < items.length; i++) {
    try {
      const r = await fetch(items[i].url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: items[i].body,
        credentials: 'include',
      });
      // Server javob bergan bo'lsa — yozuv yetdi. Takror yuborilsa ham
      // server uni `nonce` bo'yicha rad etadi, shuning uchun o'chirish
      // xavfsiz.
      if (r.ok || r.status === 303 || r.status === 302) {
        const d2 = await db();
        await new Promise(ok2 => {
          const tx = d2.transaction('q', 'readwrite');
          tx.objectStore('q').delete(keys[i]);
          tx.oncomplete = ok2; tx.onerror = ok2;
        });
      }
    } catch (e) { return; } // aloqa hali yo'q — keyingi safar
  }
}
self.addEventListener('sync', e => { if (e.tag === 'qurai-queue') e.waitUntil(drain()); });
self.addEventListener('message', e => { if (e.data === 'drain') e.waitUntil ? e.waitUntil(drain()) : drain(); });

self.addEventListener('fetch', e => {
  const req = e.request;
  if (req.method === 'POST') {
    e.respondWith(fetch(req.clone()).catch(async () => {
      const body = await req.clone().text();
      await enqueue({ url: req.url, body: body });
      if (self.registration.sync) { try { await self.registration.sync.register('qurai-queue'); } catch (x) {} }
      return new Response(
        '<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">' +
        '<body style="font-family:system-ui;padding:16px;max-width:720px;margin:auto">' +
        '<h1 style="font-size:20px">Navbatga qo\'yildi</h1>' +
        '<p style="color:#6b7280">Aloqa yo\'q. Yozuv telefonda saqlandi va aloqa qaytganda o\'zi yuboriladi. ' +
        'Ikki marta yozilmaydi: har formaning o\'z belgisi bor.</p>' +
        '<p><a href="/">Bosh sahifa</a></p></body>',
        { headers: { 'Content-Type': 'text/html; charset=utf-8' } });
    }));
    return;
  }
  if (req.method !== 'GET') return;
  e.respondWith(
    fetch(req).then(r => {
      if (r.ok && new URL(req.url).origin === self.location.origin) {
        const copy = r.clone();
        caches.open(CACHE).then(c => c.put(req, copy));
      }
      return r;
    }).catch(() => caches.match(req).then(hit => hit || caches.match(OFFLINE)))
  );
});
"#;
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        js,
    )
        .into_response()
}

// ================================================================ Chat

/// Telefondan yuborilgan xabar.
#[derive(Deserialize)]
pub struct ChatForm {
    pub text: String,
    #[serde(default)]
    pub nonce: String,
}

/// `GET /o/{project}/chat` — ofis bilan yozishma (TZ VI.32).
///
/// Bu **obyekt bo'yicha umumiy suhbat**, shaxsiy yozishma emas: bir
/// obyektda ishlayotgan hamma ko'radi. Shunday qilingani ataylab —
/// maydonchadagi savol ko'pincha bir odamga emas, ofisga qaratilgan
/// bo'ladi va javob hammaga kerak.
pub async fn chat_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    let list: String = state
        .store
        .recent_messages(&project, 50)
        .iter()
        .map(|m| {
            format!(
                "<div class=\"card\"><div class=\"row\"><b>{}</b><span class=\"muted\">{}</span></div>\
<div class=\"muted\">{}</div><div>{}</div></div>",
                esc(&m.author),
                esc(&short_stamp(&m.at)),
                esc(&role_name(&m.role)),
                esc(&m.text)
            )
        })
        .collect();
    let list = if list.is_empty() {
        "<p class=\"muted\">Hali xabar yo'q.</p>".to_string()
    } else {
        list
    };
    let form = if auth::can(&user.role, Access::Write) {
        format!(
            "<form method=\"post\" action=\"/o/{p}/chat\">\
<label>Xabar<textarea name=\"text\" rows=\"3\" required></textarea></label>{nonce}\
<button type=\"submit\">Yuborish</button></form>",
            p = esc(&project),
            nonce = nonce_field()
        )
    } else {
        "<p class=\"muted\">Sizning rolingiz xabar yozmaydi — faqat o'qiydi.</p>".to_string()
    };
    let body = format!(
        "<div class=\"row\"><h1>Ofis bilan yozishma</h1><a href=\"/o/{p}\">Ortga</a></div>\
<p class=\"muted\">Obyekt bo'yicha umumiy suhbat: bu yerdagi xabarni obyektga kirishi bor hamma ko'radi.</p>\
{form}<h2>Xabarlar</h2>{list}",
        p = esc(&project),
    );
    page("QURAi — yozishma", &body).into_response()
}

/// `POST /o/{project}/chat`
pub async fn chat_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    Form(form): Form<ChatForm>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Sizning rolingiz xabar yozmaydi.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    let text = form.text.trim();
    if text.is_empty() {
        return Redirect::to(&format!("/o/{project}/chat")).into_response();
    }
    let at = crate::now();
    if !form.nonce.trim().is_empty() && !state.store.use_nonce(&form.nonce, &at) {
        return Redirect::to(&format!("/o/{project}/chat")).into_response();
    }
    let _ = state.store.add_message(&crate::store::Message {
        id: 0,
        project: project.trim().to_string(),
        author: user.name.clone(),
        role: user.role.clone(),
        // Uzun xabar kesiladi: bu chat, hujjat emas.
        text: text.chars().take(2_000).collect(),
        at: at.clone(),
    });
    state.store.log(&user.login, "xabar", project.trim(), &at);
    Redirect::to(&format!("/o/{project}/chat")).into_response()
}

/// Vaqtdan «08.09 07:41».
fn short_stamp(iso: &str) -> String {
    let (date, time) = match iso.split_once('T') {
        Some((d, t)) => (d, t),
        None => (iso, ""),
    };
    let day: Vec<&str> = date.split('-').collect();
    let hm: String = time.chars().take(5).collect();
    if day.len() == 3 {
        format!("{}.{} {hm}", day[2], day[1])
    } else {
        format!("{date} {hm}")
    }
}

// ================================================== Kirish/chiqish va QR

/// Kirish yoki chiqish belgisi formasi.
#[derive(Deserialize)]
pub struct CheckinForm {
    pub worker: String,
    /// `in` yoki `out`.
    pub kind: String,
    #[serde(default)]
    pub nonce: String,
    #[serde(default)]
    pub gps: String,
    #[serde(default)]
    pub source: String,
}

/// Sahifaga `?w=` bilan kelgan ishchi (QR dan o'tganda).
#[derive(Deserialize, Default)]
pub struct WhoQuery {
    #[serde(default)]
    pub w: String,
    #[serde(default)]
    pub src: String,
}

/// Belgidan desktop ilova tushunadigan paket tuzadi.
///
/// Ustunlar ilovaning `attendance` jadvali bilan bir xil: server yangi
/// format o'ylab topmaydi.
pub fn attendance_package(project: &str, at: &str, a: &crate::store::Attendance) -> String {
    let cols = ["worker", "at", "kind", "gps", "source"];
    let row = [
        a.worker.clone(),
        a.at.clone(),
        a.kind.clone(),
        a.gps.clone(),
        a.source.clone(),
    ];
    format!(
        "QURAI-PACKAGE\t1\nPROJECT\t{}\nCREATED\t{}\n\n#attendance\n{}\n{}\n",
        pkg_escape(project),
        pkg_escape(at),
        cols.join("\t"),
        row.iter()
            .map(|c| pkg_escape(c))
            .collect::<Vec<_>>()
            .join("\t")
    )
}

/// Vaqtdan soat-daqiqa.
fn short_time(iso: &str) -> String {
    iso.split('T')
        .nth(1)
        .map(|t| t.chars().take(5).collect())
        .unwrap_or_default()
}

/// `GET /o/{project}/checkin` — maydonchaga kirish va undan chiqish.
///
/// TZ XIII.4–XIII.6: belgi telefondan qo'yiladi, joy bilan birga, va QR
/// orqali ham ochiladi (`?w=<ism>`). Ro'yxat ofisdagi ilovadan keladi —
/// bu yerda yangi ishchi yaratilmaydi: tabel ilovada yuritiladi.
pub async fn checkin_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    axum::extract::Query(q): axum::extract::Query<WhoQuery>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Sizning rolingiz belgi qo'ymaydi.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }

    let workers = state.store.workers(&project);
    let chosen = q.w.trim().to_string();
    let known = workers.iter().any(|w| w.name == chosen);

    let body = if !chosen.is_empty() && known {
        // QR dan yoki ro'yxatdan tanlangan bitta ishchi.
        let last = state.store.last_attendance(&project, &chosen);
        let next = match last.as_ref().map(|a| a.kind.as_str()) {
            Some("in") => "out",
            _ => "in",
        };
        let state_line = match last {
            Some(a) => format!(
                "<p class=\"muted\">Oxirgi belgi: {} · {}</p>",
                if a.kind == "in" { "kirish" } else { "chiqish" },
                esc(&short_time(&a.at))
            ),
            None => "<p class=\"muted\">Bugun belgi qo'yilmagan.</p>".to_string(),
        };
        let (label, other) = if next == "in" {
            ("Kirdim", "out")
        } else {
            ("Chiqdim", "in")
        };
        format!(
            "<div class=\"row\"><h1>{who}</h1><a href=\"/o/{p}/checkin\">Ro'yxat</a></div>{state_line}\
<form method=\"post\" action=\"/o/{p}/checkin\">\
<input type=\"hidden\" name=\"worker\" value=\"{who}\">\
<input type=\"hidden\" name=\"source\" value=\"{src}\">\
{geo}{nonce}\
<button type=\"submit\" name=\"kind\" value=\"{next}\">{label}</button>\
<button class=\"ghost\" type=\"submit\" name=\"kind\" value=\"{other}\">Aksincha belgilash</button>\
</form>",
            who = esc(&chosen),
            p = esc(&project),
            src = if q.src == "qr" { "qr" } else { "list" },
            geo = geo_block(),
            nonce = nonce_field(),
        )
    } else {
        let note = if chosen.is_empty() {
            String::new()
        } else {
            format!("<p class=\"err\">Ro'yxatda topilmadi: {}</p>", esc(&chosen))
        };
        let list: String = workers
            .iter()
            .map(|w| {
                format!(
                    "<div class=\"card\"><div class=\"row\"><div><b>{}</b><br><span class=\"muted\">{}</span></div>\
<a href=\"/o/{}/checkin?w={}\">Belgilash</a></div></div>",
                    esc(&w.name),
                    esc(&w.position),
                    esc(&project),
                    urlencode(&w.name)
                )
            })
            .collect();
        let list = if workers.is_empty() {
            "<p class=\"muted\">Ishchilar ro'yxati hali kelmagan: ofisdagi ilova sinxronizatsiya qilganda paydo bo'ladi.</p>".to_string()
        } else {
            list
        };
        let recent: String = state
            .store
            .attendance(&project, 20)
            .iter()
            .map(|a| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc(&short_time(&a.at)),
                    esc(&a.worker),
                    if a.kind == "in" { "kirish" } else { "chiqish" }
                )
            })
            .collect();
        let recent = if recent.is_empty() {
            String::new()
        } else {
            format!(
                "<h2>Oxirgi belgilar</h2><table><tr><th>Vaqt</th><th>Ishchi</th><th>Belgi</th></tr>{recent}</table>"
            )
        };
        format!(
            "<div class=\"row\"><h1>Kirish / chiqish</h1><a href=\"/o/{p}\">Ortga</a></div>\
{note}<p class=\"muted\">Ishchini tanlang yoki uning QR yorlig'ini o'qing.</p>\
<p><a href=\"/o/{p}/scan\">QR o'qish</a></p>{list}{recent}",
            p = esc(&project),
        )
    };
    page("QURAi — kirish/chiqish", &body).into_response()
}

/// `POST /o/{project}/checkin`
pub async fn checkin_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    Form(form): Form<CheckinForm>,
) -> Response {
    let Some(user) = viewer(&state, &headers) else {
        return login_page("").into_response();
    };
    if !auth::can(&user.role, Access::Write) {
        return (
            StatusCode::FORBIDDEN,
            page(
                "QURAi",
                "<p class=\"err\">Huquq yo'q.</p><a href=\"/\">Ortga</a>",
            ),
        )
            .into_response();
    }
    let worker = form.worker.trim().to_string();
    // Ro'yxatda yo'q ishchiga belgi qo'yilmaydi: tabel ilovada yuritiladi
    // va u yerda bo'lmagan odam bu yerda paydo bo'lmasligi kerak.
    if worker.is_empty()
        || !state
            .store
            .workers(&project)
            .iter()
            .any(|w| w.name == worker)
    {
        return page(
            "QURAi",
            "<p class=\"err\">Ishchi ro'yxatda yo'q.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }
    let kind = if form.kind.trim() == "out" {
        "out"
    } else {
        "in"
    };
    let at = crate::now();
    if !form.nonce.trim().is_empty() && !state.store.use_nonce(&form.nonce, &at) {
        return Redirect::to(&format!("/o/{project}/checkin")).into_response();
    }
    let record = crate::store::Attendance {
        project: project.trim().to_string(),
        worker,
        kind: kind.to_string(),
        at: at.clone(),
        gps: clean_gps(&form.gps),
        source: if form.source.trim() == "qr" {
            "qr".into()
        } else {
            "list".into()
        },
    };
    if state.store.add_attendance(&record).is_err() {
        return page(
            "QURAi",
            "<p class=\"err\">Saqlanmadi.</p><a href=\"/\">Ortga</a>",
        )
        .into_response();
    }
    let body = attendance_package(project.trim(), &at, &record);
    let _ = state
        .store
        .push_change(project.trim(), &body, &user.login, &at, 1);
    state.store.log(&user.login, "belgi", project.trim(), &at);
    Redirect::to(&format!("/o/{project}/checkin")).into_response()
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

/// `GET /o/{project}/scan` — QR o'qish (TZ VI.11, XI.5, XIII.6).
///
/// Kamera brauzerning o'zida ishlaydi: `BarcodeDetector` bo'lsa rasm
/// jonli o'qiladi, bo'lmasa kod qo'lda kiritiladi. Ikkinchi yo'l har doim
/// ochiq — shuning uchun sahifa hech qachon «ishlamaydi» holatiga
/// tushmaydi. Rasm hech qayerga yuborilmaydi: faqat kod matni o'qiladi.
pub async fn scan_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> Response {
    if viewer(&state, &headers).is_none() {
        return login_page("").into_response();
    }
    let script = "(function(){\
var v=document.getElementById('cam'),b=document.getElementById('start'),t=document.getElementById('camtext'),f=document.getElementById('code');\
if(!('BarcodeDetector' in window)||!navigator.mediaDevices){t.textContent='Bu brauzer kamerani o\\u2018qiy olmaydi \\u2014 kodni qo\\u2018lda kiriting.';b.style.display='none';return;}\
b.onclick=function(){b.disabled=true;t.textContent='Kamera ochilmoqda\\u2026';\
navigator.mediaDevices.getUserMedia({video:{facingMode:'environment'}}).then(function(st){\
v.srcObject=st;v.style.display='block';v.play();\
var d=new window.BarcodeDetector({formats:['qr_code']});\
var timer=setInterval(function(){d.detect(v).then(function(codes){\
if(codes&&codes.length){clearInterval(timer);st.getTracks().forEach(function(x){x.stop();});\
f.value=codes[0].rawValue;t.textContent='O\\u2018qildi';f.form.submit();}}).catch(function(){});},400);\
}).catch(function(){b.disabled=false;t.textContent='Kamera ochilmadi \\u2014 kodni qo\\u2018lda kiriting.';});};})();";
    let body = format!(
        "<div class=\"row\"><h1>QR o'qish</h1><a href=\"/o/{p}\">Ortga</a></div>\
<form method=\"get\" action=\"/o/{p}/find\">\
<button type=\"button\" id=\"start\">Kamerani ochish</button>\
<p class=\"muted\" id=\"camtext\">Kamera ruxsat so'raydi. Rasm hech qayerga yuborilmaydi.</p>\
<video id=\"cam\" playsinline muted style=\"display:none;width:100%;border-radius:10px\"></video>\
<label>Kod<input name=\"code\" id=\"code\" placeholder=\"QURAI:OBY-1:batch:P-12\" autocapitalize=\"none\"></label>\
<button type=\"submit\">Ochish</button></form>\
<script>{script}</script>",
        p = esc(&project),
    );
    page("QURAi — QR", &body).into_response()
}

/// Kod ichidan `tur` va `raqam` ni ajratadi.
///
/// Ikki ko'rinish qabul qilinadi: `QURAI:obyekt:tur:raqam` va shu
/// ma'lumot bor havola (`.../o/obyekt?batch=P-12`). Boshqa har qanday
/// matn — noma'lum kod, va shunday deyiladi.
pub fn decode_label(code: &str) -> Option<(String, String)> {
    let code = code.trim();
    if let Some(rest) = code.strip_prefix("QURAI:") {
        let parts: Vec<&str> = rest.splitn(3, ':').collect();
        if parts.len() == 3 {
            return Some((parts[1].to_string(), parts[2].to_string()));
        }
        return None;
    }
    let query = code.split_once('?').map(|(_, q)| q)?;
    let (kind, number) = query.split_once('=')?;
    let number = number.split('&').next().unwrap_or("");
    Some((kind.to_string(), urldecode(number)))
}

/// Manzildagi `%XX` belgilarni ochadi.
fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// `?code=` bilan keladigan so'rov.
#[derive(Deserialize, Default)]
pub struct CodeQuery {
    #[serde(default)]
    pub code: String,
}

/// `GET /o/{project}/find` — o'qilgan kod nimaligini ko'rsatadi.
pub async fn find_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project): Path<String>,
    axum::extract::Query(q): axum::extract::Query<CodeQuery>,
) -> Response {
    if viewer(&state, &headers).is_none() {
        return login_page("").into_response();
    }
    let Some((kind, number)) = decode_label(&q.code) else {
        return page(
            "QURAi — QR",
            &format!(
                "<div class=\"row\"><h1>Noma'lum kod</h1><a href=\"/o/{p}/scan\">Qayta</a></div>\
<p class=\"muted\">Bu kod QURAi yorlig'iga o'xshamaydi:</p><p><code>{c}</code></p>",
                p = esc(&project),
                c = esc(&q.code)
            ),
        )
        .into_response();
    };

    // Ishchi — darrov belgilash sahifasiga.
    if kind == "worker" {
        return Redirect::to(&format!(
            "/o/{}/checkin?w={}&src=qr",
            project,
            urlencode(&number)
        ))
        .into_response();
    }

    let label = state.store.label(&project, &kind, &number);
    let card = match label {
        Some(l) => format!(
            "<div class=\"card\"><b>{}</b><br><span class=\"muted\">{}</span></div>",
            esc(&l.title),
            esc(&l.note)
        ),
        None => "<p class=\"muted\">Bu yorliq haqidagi ma'lumot serverga hali kelmagan: ofisdagi ilova sinxronizatsiya qilganda paydo bo'ladi.</p>".to_string(),
    };
    let body = format!(
        "<div class=\"row\"><h1>{k}</h1><a href=\"/o/{p}/scan\">Yana o'qish</a></div>\
<p class=\"muted\">{n}</p>{card}<p><a href=\"/o/{p}\">Obyektga</a></p>",
        k = esc(&kind_name(&kind)),
        n = esc(&number),
        p = esc(&project),
    );
    page("QURAi — yorliq", &body).into_response()
}

/// Yorliq turining nomi.
fn kind_name(kind: &str) -> String {
    match kind {
        "unit" => "Kvartira",
        "batch" => "Material partiyasi",
        "doc" => "Hujjat",
        "worker" => "Ishchi",
        other => other,
    }
    .to_string()
}

/// Rol kodining o'qiladigan nomi.
fn role_name(role: &str) -> String {
    match role {
        "admin" => "Administrator",
        "director" => "Direktor",
        "pm" => "Loyiha rahbari",
        "foreman" => "Prorab",
        "brigadier" => "Brigadir",
        "supervisor" => "Texnik nazorat",
        "designer" => "Mualliflik nazorati",
        "estimator" => "Smetachi",
        "supply" => "Ta'minotchi",
        "storekeeper" => "Omborchi",
        "client" => "Buyurtmachi",
        "accountant" => "Buxgalter",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Foydalanuvchi matni sahifani buzmaydi.
    #[test]
    fn user_text_is_escaped() {
        assert_eq!(esc("<script>"), "&lt;script&gt;");
        assert_eq!(esc("a & b"), "a &amp; b");
        assert_eq!(esc("\"tirnoq\""), "&quot;tirnoq&quot;");
        assert_eq!(esc("oddiy"), "oddiy");
    }

    /// Cookie dan belgi to'g'ri ajratiladi.
    #[test]
    fn session_token_is_read_from_the_cookie() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            "boshqa=1; qurai_session=abc123; yana=2".parse().unwrap(),
        );
        assert_eq!(token_of(&h), Some("abc123".to_string()));

        let empty = HeaderMap::new();
        assert_eq!(token_of(&empty), None);
    }

    /// Rol nomi tanilmasa, kodi ko'rsatiladi — o'ylab topilmaydi.
    #[test]
    fn unknown_role_shows_its_code() {
        assert_eq!(role_name("foreman"), "Prorab");
        assert_eq!(role_name("yangi-rol"), "yangi-rol");
    }
}
