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

/// `GET /api/health` — server tirikmi.
pub async fn health(State(state): State<Arc<AppState>>) -> axum::response::Response {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "projects": state.store.projects().len(),
    }))
    .into_response()
}
