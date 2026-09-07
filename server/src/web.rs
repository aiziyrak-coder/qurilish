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
<title>{}</title><style>{}</style></head><body>{}</body></html>",
        esc(title),
        STYLE,
        body
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
            "<div class=\"card\"><b>Kunlik yozuv</b><div class=\"muted\">Maydonchadan to'g'ridan-to'g'ri kiritish</div><a href=\"/o/{p}/journal\">Ochish</a></div><div class=\"card\"><b>Tabel</b><div class=\"muted\">Bugungi soat va kun turi</div><a href=\"/o/{p}/timesheet\">Ochish</a></div>",
            p = esc(&project)
        ));
    }

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
<button type=\"submit\">Yuborish</button></form>\
<p class=\"muted\">Yozuv obyekt paketiga qo'shiladi va ofisdagi ilova uni keyingi sinxronizatsiyada oladi.</p>",
        p = esc(&project),
        obj = esc(&project),
        name = esc(&user.name),
        today = esc(&today),
        task_field = task_field,
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
    let body = timesheet_package(&project, &at, form.date.trim(), &rows);
    let _ = state
        .store
        .push_change(project.trim(), &body, &user.login, &at, rows.len() as i64);
    state.store.log(&user.login, "tabel", project.trim(), &at);
    Redirect::to(&format!("/o/{project}")).into_response()
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
