//! Desktop ilova bilan aloqa: kirish, sinxronizatsiya va imzo.
//!
//! Aloqa oddiy JSON orqali. Har so'rovda seans belgisi `Authorization:
//! Bearer <belgi>` sarlavhasida keladi — u manzilga yozilmaydi, chunki
//! manzillar jurnalga tushadi.
//!
//! Sinxronizatsiya modeli **oddiy va tekshiriladigan**: qurilma o'z
//! o'zgarishlarini paket qilib yuboradi (`push`), boshqalari esa o'zidan
//! keyingi paketlarni oladi (`pull`). Har paket tartib raqamiga ega,
//! shuning uchun hech narsa yo'qolmaydi va ikki marta qo'llanmaydi.
//!
//! Server paketni **ochmaydi va o'zgartirmaydi** — u faqat tartib bilan
//! saqlaydi. Ma'lumotni tushunish desktop ilovaning ishi.

use crate::auth::{self, Access};
use crate::state::AppState;
use crate::store::Signature;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Bitta so'rovda beriladigan eng ko'p paket soni.
///
/// Chegara bo'lmasa, uzoq vaqt ulanmagan qurilma butun tarixni bir
/// so'rovda so'rab, xotirani to'ldirib yuborardi.
pub const PULL_LIMIT: i64 = 200;

/// Paketning eng katta hajmi (bayt).
pub const MAX_BODY: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
pub struct LoginReq {
    pub login: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub token: String,
    pub name: String,
    pub role: String,
    pub until: String,
}

#[derive(Deserialize)]
pub struct PushReq {
    pub project: String,
    pub body: String,
    #[serde(default)]
    pub rows: i64,
}

#[derive(Deserialize)]
pub struct PullQuery {
    pub project: String,
    #[serde(default)]
    pub since: i64,
}

#[derive(Deserialize)]
pub struct SignReq {
    pub project: String,
    pub document: String,
    /// Imzolanayotgan matn — xesh shundan olinadi.
    pub text: String,
    #[serde(default)]
    pub rejected: String,
}

#[derive(Deserialize)]
pub struct NoticesReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<NoticeItem>,
}

#[derive(Deserialize)]
pub struct NoticeItem {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub days: i64,
    #[serde(default)]
    pub source: String,
}

/// Bir obyekt uchun qabul qilinadigan eng ko'p signal soni.
///
/// Chegara bo'lmasa, xato hisob butun jadvalni to'ldirib yuborardi.
pub const MAX_NOTICES: usize = 200;

#[derive(Deserialize)]
pub struct TasksReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<TaskItem>,
}

#[derive(Deserialize)]
pub struct TaskItem {
    #[serde(default)]
    pub wbs: String,
    pub name: String,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub section: String,
}

/// Bir obyekt uchun qabul qilinadigan eng ko'p ish soni.
pub const MAX_TASKS: usize = 2_000;

#[derive(Deserialize)]
pub struct WorkersReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<WorkerItem>,
}

#[derive(Deserialize)]
pub struct WorkerItem {
    pub name: String,
    #[serde(default)]
    pub position: String,
}

#[derive(Deserialize)]
pub struct ChainReq {
    pub project: String,
    pub count: i64,
    pub head: String,
}

#[derive(Deserialize)]
pub struct ProjectQuery {
    pub project: String,
}

#[derive(Deserialize)]
pub struct MessageReq {
    pub project: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub project: String,
    #[serde(default)]
    pub since: i64,
}

/// Bitta so'rovda beriladigan eng ko'p xabar soni.
pub const MESSAGE_LIMIT: i64 = 200;

#[derive(Deserialize)]
pub struct MachinesReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<MachineItem>,
}

#[derive(Deserialize)]
pub struct MachineItem {
    pub name: String,
    #[serde(default)]
    pub reg_no: String,
}

#[derive(Deserialize)]
pub struct LabelsReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<LabelItem>,
}

#[derive(Deserialize)]
pub struct LabelItem {
    pub kind: String,
    pub number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Deserialize)]
pub struct SummaryReq {
    pub project: String,
    #[serde(default)]
    pub items: Vec<SummaryItem>,
}

#[derive(Deserialize)]
pub struct SummaryItem {
    #[serde(default)]
    pub module: String,
    pub indicator: String,
    #[serde(default)]
    pub value: String,
}

/// Xatoni bir xil ko'rinishda qaytaradi.
fn err(code: StatusCode, message: &str) -> axum::response::Response {
    (code, Json(json!({ "error": message }))).into_response()
}

/// Sarlavhadan seans belgisini oladi.
fn token_of(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = raw.strip_prefix("Bearer ").unwrap_or(raw).trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// So'rov egasini aniqlaydi. Belgi yo'q yoki eskirgan bo'lsa — `None`.
fn caller(state: &AppState, headers: &HeaderMap) -> Option<crate::store::User> {
    let token = token_of(headers)?;
    state.store.session_user(&token, &crate::now())
}

/// `POST /api/login`
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> axum::response::Response {
    let Some((user, hash)) = state.store.user_by_login(&req.login) else {
        // Login topilmagani ham, parol xato ekani ham bir xil javob
        // beradi: farqi hujumchiga mavjud loginni aytib qo'yardi.
        return err(StatusCode::UNAUTHORIZED, "login yoki parol noto'g'ri");
    };
    if !user.active || !auth::verify_password(&req.password, &hash) {
        state
            .store
            .log(&user.login, "kirish-rad", "", &crate::now());
        return err(StatusCode::UNAUTHORIZED, "login yoki parol noto'g'ri");
    }

    let token = auth::new_token();
    let until = crate::plus_days(auth::SESSION_DAYS);
    if !state.store.open_session(user.id, &token, &until) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "seans ochilmadi");
    }
    state.store.log(&user.login, "kirdi", "", &crate::now());

    Json(LoginResp {
        token,
        name: user.name,
        role: user.role,
        until,
    })
    .into_response()
}

/// `POST /api/logout`
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    if let Some(token) = token_of(&headers) {
        state.store.close_session(&token);
    }
    Json(json!({ "ok": true })).into_response()
}

/// `GET /api/projects`
pub async fn projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    let Some(_user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    let list: Vec<_> = state
        .store
        .projects()
        .into_iter()
        .map(|(code, last, at)| json!({ "project": code, "last": last, "at": at }))
        .collect();
    Json(json!({ "projects": list })).into_response()
}

/// `POST /api/push` — qurilmadan kelgan o'zgarishlar paketi.
pub async fn push(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<PushReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.body.len() > MAX_BODY {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "paket juda katta");
    }
    if req.body.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "paket bo'sh");
    }

    match state.store.push_change(
        req.project.trim(),
        &req.body,
        &user.login,
        &crate::now(),
        req.rows,
    ) {
        Ok(id) => {
            state
                .store
                .log(&user.login, "paket", req.project.trim(), &crate::now());
            Json(json!({ "id": id })).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `GET /api/pull?project=..&since=..`
pub async fn pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PullQuery>,
) -> axum::response::Response {
    let Some(_user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    let changes = state
        .store
        .changes_since(q.project.trim(), q.since, PULL_LIMIT);
    let last = changes
        .last()
        .map(|c| c.id)
        .unwrap_or_else(|| q.since.max(0));
    let items: Vec<_> = changes
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "body": c.body,
                "author": c.author,
                "at": c.at,
                "rows": c.rows,
            })
        })
        .collect();
    Json(json!({
        "changes": items,
        "last": last,
        // Serverda yana paket qolgani — qurilma yana so'raydi.
        "more": items.len() as i64 == PULL_LIMIT,
    }))
    .into_response()
}

/// `POST /api/sign` — masofadan imzolash.
pub async fn sign(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SignReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Sign) {
        return err(StatusCode::FORBIDDEN, "bu rol hujjat imzolamaydi");
    }
    if req.document.trim().is_empty() || req.text.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "hujjat ko'rsatilmagan");
    }

    let sig = Signature {
        id: 0,
        project: req.project.trim().to_string(),
        document: req.document.trim().to_string(),
        user: user.login.clone(),
        role: user.role.clone(),
        at: crate::now(),
        // Xesh imzo nimani tasdiqlaganini belgilaydi: hujjat keyin
        // o'zgarsa, xesh mos kelmaydi va buni ko'rish mumkin.
        digest: auth::digest(&req.text),
        rejected: req.rejected.trim().to_string(),
    };
    match state.store.sign(&sig) {
        Ok(_) => {
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
            Json(json!({ "ok": true, "digest": sig.digest })).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `GET /api/signatures?project=..`
pub async fn signatures(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PullQuery>,
) -> axum::response::Response {
    let Some(_user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    let list: Vec<_> = state
        .store
        .signatures(q.project.trim())
        .into_iter()
        .map(|s| {
            json!({
                "document": s.document,
                "user": s.user,
                "role": s.role,
                "at": s.at,
                "digest": s.digest,
                "rejected": s.rejected,
            })
        })
        .collect();
    Json(json!({ "signatures": list })).into_response()
}

/// `POST /api/notices` — desktop hisoblagan signal ro'yxati.
///
/// Server signalni o'zi hisoblamaydi va o'zgartirmaydi: u ilova
/// yuborgan ro'yxatni saqlaydi va telefonga ko'rsatadi. Shuning uchun
/// telefondagi son ofisdagi ekran bilan bir xil bo'ladi.
pub async fn notices(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<NoticesReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_NOTICES {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "signal juda ko'p");
    }

    let items: Vec<crate::store::Notice> = req
        .items
        .into_iter()
        .filter(|i| !i.title.trim().is_empty())
        .map(|i| crate::store::Notice {
            code: i.code,
            severity: i.severity,
            title: i.title,
            detail: i.detail,
            count: i.count,
            days: i.days,
            source: i.source,
        })
        .collect();
    match state
        .store
        .set_notices(req.project.trim(), &crate::now(), &items)
    {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/tasks` — grafikdagi ishlar ro'yxati (telefonda tanlash uchun).
pub async fn tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TasksReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_TASKS {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "ish juda ko'p");
    }
    let items: Vec<crate::store::TaskRef> = req
        .items
        .into_iter()
        .filter(|t| !t.name.trim().is_empty())
        .map(|t| crate::store::TaskRef {
            wbs: t.wbs,
            name: t.name,
            start: t.start,
            end: t.end,
            progress: t.progress,
            section: t.section,
        })
        .collect();
    match state.store.set_tasks(req.project.trim(), &items) {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/workers` — tabel uchun ishchilar ro'yxati.
pub async fn workers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<WorkersReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_TASKS {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "ishchi juda ko'p");
    }
    let items: Vec<crate::store::WorkerRef> = req
        .items
        .into_iter()
        .filter(|w| !w.name.trim().is_empty())
        .map(|w| crate::store::WorkerRef {
            name: w.name,
            position: w.position,
        })
        .collect();
    match state.store.set_workers(req.project.trim(), &items) {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/machines` — telefondagi smena formasi uchun texnika ro'yxati.
pub async fn machines(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<MachinesReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_TASKS {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "texnika juda ko'p");
    }
    let items: Vec<crate::store::MachineRef> = req
        .items
        .into_iter()
        .filter(|m| !m.name.trim().is_empty())
        .map(|m| crate::store::MachineRef {
            name: m.name,
            reg_no: m.reg_no,
        })
        .collect();
    match state.store.set_machines(req.project.trim(), &items) {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/labels` — QR yorliqlar ro'yxati (TZ VI.11, XI.5).
///
/// Telefon o'qigan kod nimaligini ko'rsatishi uchun kerak: serverda
/// yozuvning o'zi emas, uning qisqa nomi turadi.
pub async fn labels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LabelsReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_TASKS {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "yorliq juda ko'p");
    }
    let items: Vec<crate::store::LabelRef> = req
        .items
        .into_iter()
        .filter(|l| !l.kind.trim().is_empty() && !l.number.trim().is_empty())
        .map(|l| crate::store::LabelRef {
            kind: l.kind,
            number: l.number,
            title: l.title,
            note: l.note,
        })
        .collect();
    match state.store.set_labels(req.project.trim(), &items) {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/summary` — obyekt yakuni (buyurtmachi kabineti uchun).
pub async fn summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SummaryReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt ko'rsatilmagan");
    }
    if req.items.len() > MAX_NOTICES {
        return err(StatusCode::PAYLOAD_TOO_LARGE, "qator juda ko'p");
    }
    let rows: Vec<crate::store::SummaryRow> = req
        .items
        .into_iter()
        .filter(|i| !i.indicator.trim().is_empty())
        .map(|i| crate::store::SummaryRow {
            module: i.module,
            indicator: i.indicator,
            value: i.value,
        })
        .collect();
    match state
        .store
        .set_summary(req.project.trim(), &crate::now(), &rows)
    {
        Ok(n) => Json(json!({ "saved": n })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `POST /api/message` — ofisdan xabar yuborish (TZ VI.32).
pub async fn message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<MessageReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol xabar yozmaydi");
    }
    let text = req.text.trim();
    if req.project.trim().is_empty() || text.is_empty() {
        return err(StatusCode::BAD_REQUEST, "obyekt yoki matn bo'sh");
    }
    let at = crate::now();
    match state.store.add_message(&crate::store::Message {
        id: 0,
        project: req.project.trim().to_string(),
        author: user.name.clone(),
        role: user.role.clone(),
        text: text.chars().take(2_000).collect(),
        at,
    }) {
        Ok(id) => Json(json!({ "id": id })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `GET /api/messages?project=&since=` — yangi xabarlar.
pub async fn messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<MessagesQuery>,
) -> axum::response::Response {
    if caller(&state, &headers).is_none() {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    }
    let list: Vec<_> = state
        .store
        .messages(q.project.trim(), q.since, MESSAGE_LIMIT)
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "author": m.author,
                "role": m.role,
                "text": m.text,
                "at": m.at,
            })
        })
        .collect();
    Json(json!({ "messages": list })).into_response()
}

/// `POST /api/chain` — imzo daftarining uchini qayd etadi (TZ IV.18, V.32).
///
/// Server daftarni ko'rmaydi va tekshira olmaydi — u faqat **eslab
/// qoladi**. Ilova keyin o'sha uzunlikda boshqa uch bilan kelsa,
/// daftar o'zgartirilgani shundan bilinadi.
pub async fn chain_mark(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ChainReq>,
) -> axum::response::Response {
    let Some(user) = caller(&state, &headers) else {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    };
    if !auth::can(&user.role, Access::Write) {
        return err(StatusCode::FORBIDDEN, "bu rol ma'lumot yubormaydi");
    }
    if req.project.trim().is_empty() || req.count < 0 {
        return err(StatusCode::BAD_REQUEST, "obyekt yoki uzunlik noto'g'ri");
    }
    match state.store.mark_chain(
        req.project.trim(),
        req.count,
        req.head.trim(),
        &crate::now(),
    ) {
        Ok(None) => Json(json!({ "ok": true })).into_response(),
        Ok(Some(old)) => {
            state.store.log(
                &user.login,
                "zanjir-ziddiyat",
                req.project.trim(),
                &crate::now(),
            );
            Json(json!({ "ok": false, "conflict": old })).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// `GET /api/chain?project=` — qayd etilgan belgilar.
pub async fn chain_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ProjectQuery>,
) -> axum::response::Response {
    if caller(&state, &headers).is_none() {
        return err(StatusCode::UNAUTHORIZED, "kirish kerak");
    }
    let list: Vec<_> = state
        .store
        .chain_marks(q.project.trim())
        .into_iter()
        .map(|(count, head, at)| json!({ "count": count, "head": head, "at": at }))
        .collect();
    Json(json!({ "marks": list })).into_response()
}

/// `GET /api/health` — server tirikmi.
pub async fn health(State(state): State<Arc<AppState>>) -> axum::response::Response {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "projects": state.store.projects().len(),
    }))
    .into_response()
}
