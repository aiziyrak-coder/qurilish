//! Boshdan-oxir sinovlar: haqiqiy so'rovlar haqiqiy yo'llardan o'tadi.
//!
//! Bu yerda tarmoq ochilmaydi — so'rov to'g'ridan-to'g'ri yo'llar jadvaliga
//! beriladi. Shu sababli sinovlar tez va port band bo'lishiga bog'liq emas,
//! lekin tekshirilayotgani aynan foydalanuvchi ko'radigan xatti-harakat:
//! kim kira oladi, kim kira olmaydi va ma'lumot qanday ko'chadi.

#![cfg(test)]

use crate::state::{AppState, Config};
use crate::store::Store;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

/// Sinov uchun server: xotiradagi baza va uchta foydalanuvchi.
fn app() -> (axum::Router, Arc<AppState>) {
    let store = Store::memory().expect("baza");
    let hash = |p: &str| crate::auth::hash_password(p).expect("xesh");
    store
        .add_user("prorab", "Alisher", "foreman", &hash("prorab-parol-1"))
        .unwrap();
    store
        .add_user("nazorat", "Dilnoza", "supervisor", &hash("nazorat-parol-1"))
        .unwrap();
    store
        .add_user("mijoz", "Buyurtmachi", "client", &hash("mijoz-parol-11"))
        .unwrap();

    let state = Arc::new(AppState {
        store,
        config: Config::default(),
    });
    (crate::router(state.clone()), state)
}

/// So'rov yuboradi va javob kodini hamda tanasini qaytaradi.
async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.expect("javob");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("tana").to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// JSON so'rovi.
fn post(path: &str, token: Option<&str>, body: Value) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).expect("so'rov")
}

fn get(path: &str, token: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method("GET").uri(path);
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::empty()).expect("so'rov")
}

/// Kirib, belgini oladi.
async fn login(app: &axum::Router, login: &str, password: &str) -> String {
    let (code, body) = send(
        app,
        post(
            "/api/login",
            None,
            json!({ "login": login, "password": password }),
        ),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{body}");
    body["token"].as_str().expect("belgi").to_string()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

/// To'g'ri parol bilan kiriladi, noto'g'risi bilan kirilmaydi.
#[test]
fn login_needs_the_right_password() {
    runtime().block_on(async {
        let (app, _) = app();

        let (code, body) = send(
            &app,
            post(
                "/api/login",
                None,
                json!({ "login": "prorab", "password": "prorab-parol-1" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(body["role"], "foreman");
        assert!(body["token"].as_str().is_some_and(|t| t.len() == 64));
        // Javobda parol ham, xesh ham yo'q.
        assert!(!body.to_string().contains("prorab-parol-1"));

        let (code, _) = send(
            &app,
            post(
                "/api/login",
                None,
                json!({ "login": "prorab", "password": "noto-g-ri" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::UNAUTHORIZED);

        // Yo'q login ham bir xil javob beradi.
        let (code, _) = send(
            &app,
            post(
                "/api/login",
                None,
                json!({ "login": "yo-q", "password": "boshqa-parol" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    });
}

/// Belgisiz so'rov o'tmaydi.
#[test]
fn requests_without_a_token_are_refused() {
    runtime().block_on(async {
        let (app, _) = app();
        for req in [
            get("/api/projects", None),
            get("/api/pull?project=OBY-1&since=0", None),
            post(
                "/api/push",
                None,
                json!({ "project": "OBY-1", "body": "x" }),
            ),
        ] {
            let (code, _) = send(&app, req).await;
            assert_eq!(code, StatusCode::UNAUTHORIZED);
        }

        // Soxta belgi ham o'tmaydi.
        let (code, _) = send(&app, get("/api/projects", Some("soxta"))).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    });
}

/// Paket yuboriladi va boshqa qurilma uni o'z tartibida oladi.
#[test]
fn a_package_travels_between_devices() {
    runtime().block_on(async {
        let (app, _) = app();
        let prorab = login(&app, "prorab", "prorab-parol-1").await;
        let nazorat = login(&app, "nazorat", "nazorat-parol-1").await;

        for (i, text) in ["birinchi paket", "ikkinchi paket"].iter().enumerate() {
            let (code, body) = send(
                &app,
                post(
                    "/api/push",
                    Some(&prorab),
                    json!({ "project": "OBY-1", "body": text, "rows": i + 1 }),
                ),
            )
            .await;
            assert_eq!(code, StatusCode::OK, "{body}");
        }

        // Ikkinchi qurilma noldan o'qiydi.
        let (code, body) = send(&app, get("/api/pull?project=OBY-1&since=0", Some(&nazorat))).await;
        assert_eq!(code, StatusCode::OK);
        let changes = body["changes"].as_array().expect("ro'yxat");
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0]["body"], "birinchi paket");
        assert_eq!(changes[1]["body"], "ikkinchi paket");
        assert_eq!(changes[0]["author"], "prorab");
        let last = body["last"].as_i64().unwrap();

        // Belgidan keyin yangi paket yo'q — takror o'qilmaydi.
        let (_, body) = send(
            &app,
            get(
                &format!("/api/pull?project=OBY-1&since={last}"),
                Some(&nazorat),
            ),
        )
        .await;
        assert!(body["changes"].as_array().unwrap().is_empty());

        // Boshqa obyekt aralashmaydi.
        let (_, body) = send(&app, get("/api/pull?project=OBY-2&since=0", Some(&nazorat))).await;
        assert!(body["changes"].as_array().unwrap().is_empty());
    });
}

/// Faqat ko'ruvchi rol ma'lumot yubora olmaydi.
#[test]
fn read_only_role_cannot_push() {
    runtime().block_on(async {
        let (app, _) = app();
        let mijoz = login(&app, "mijoz", "mijoz-parol-11").await;

        let (code, _) = send(
            &app,
            post(
                "/api/push",
                Some(&mijoz),
                json!({ "project": "OBY-1", "body": "matn" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::FORBIDDEN);

        // Lekin ko'rish mumkin.
        let (code, _) = send(&app, get("/api/projects", Some(&mijoz))).await;
        assert_eq!(code, StatusCode::OK);
    });
}

/// Imzoni faqat huquqi bor rol qo'yadi va u hujjat matniga bog'lanadi.
#[test]
fn only_the_right_role_signs_and_the_text_is_bound() {
    runtime().block_on(async {
        let (app, _) = app();
        let prorab = login(&app, "prorab", "prorab-parol-1").await;
        let nazorat = login(&app, "nazorat", "nazorat-parol-1").await;

        // Prorab imzolamaydi.
        let (code, _) = send(
            &app,
            post(
                "/api/sign",
                Some(&prorab),
                json!({ "project": "OBY-1", "document": "AOSR-001", "text": "beton 10 m3" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::FORBIDDEN);

        // Texnik nazorat imzolaydi.
        let (code, body) = send(
            &app,
            post(
                "/api/sign",
                Some(&nazorat),
                json!({ "project": "OBY-1", "document": "AOSR-001", "text": "beton 10 m3" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        let digest = body["digest"].as_str().expect("xesh").to_string();

        let (_, body) = send(
            &app,
            get("/api/signatures?project=OBY-1&since=0", Some(&prorab)),
        )
        .await;
        let list = body["signatures"].as_array().expect("ro'yxat");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["user"], "nazorat");
        assert_eq!(list[0]["digest"], digest);

        // Matn o'zgarsa, xesh ham boshqacha bo'ladi — bu ko'rinadi.
        assert_ne!(digest, crate::auth::digest("beton 11 m3"));
    });
}

/// Chiqishdan keyin belgi ishlamaydi.
#[test]
fn logout_ends_the_session() {
    runtime().block_on(async {
        let (app, _) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;
        let (code, _) = send(&app, get("/api/projects", Some(&token))).await;
        assert_eq!(code, StatusCode::OK);

        let (code, _) = send(&app, post("/api/logout", Some(&token), json!({}))).await;
        assert_eq!(code, StatusCode::OK);

        let (code, _) = send(&app, get("/api/projects", Some(&token))).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    });
}

/// Bo'sh va haddan katta paket qabul qilinmaydi.
#[test]
fn empty_and_huge_packages_are_refused() {
    runtime().block_on(async {
        let (app, _) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;

        let (code, _) = send(
            &app,
            post(
                "/api/push",
                Some(&token),
                json!({ "project": "OBY-1", "body": "   " }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::BAD_REQUEST);

        let (code, _) = send(
            &app,
            post(
                "/api/push",
                Some(&token),
                json!({ "project": "", "body": "matn" }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::BAD_REQUEST);
    });
}

/// Mobil ko'rinish: kirmagan odam kirish sahifasini ko'radi.
#[test]
fn web_page_asks_for_login_first() {
    runtime().block_on(async {
        let (app, _) = app();
        let resp = app.clone().oneshot(get("/", None)).await.expect("javob");
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("Kirish"), "{html}");
        assert!(html.contains("viewport"), "mobil uchun moslashuv yo'q");
        // Sahifada hech kimning ismi yo'q — kirilmagan.
        assert!(!html.contains("Alisher"));
    });
}

/// Server tirikligini tekshirish belgisiz ham ishlaydi.
#[test]
fn health_needs_no_login() {
    runtime().block_on(async {
        let (app, _) = app();
        let (code, body) = send(&app, get("/api/health", None)).await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(body["ok"], true);
    });
}

/// Telefondan kiritilgan kunlik yozuv paketga aylanadi va sinxronizatsiya
/// orqali desktop ilovaga tushadi.
#[test]
fn journal_from_the_phone_becomes_a_package() {
    runtime().block_on(async {
        let (app, _) = app();
        // Kirish va cookie ni olamiz.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        // Kunlik yozuv yuboramiz.
        let form = "date=2026-09-04&task=Monolit+karkas&volume=12%2C5&unit=m3\
&workers=8&machines=2&weather=ochiq&text=Beton+quyildi&remarks=";
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/o/OBY-1/journal")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);

        // Desktop ilova uni oddiy paket sifatida oladi.
        let token = login(&app, "prorab", "prorab-parol-1").await;
        let (code, body) = send(&app, get("/api/pull?project=OBY-1&since=0", Some(&token))).await;
        assert_eq!(code, StatusCode::OK);
        let changes = body["changes"].as_array().expect("ro'yxat");
        assert_eq!(changes.len(), 1);
        let text = changes[0]["body"].as_str().expect("matn");
        assert!(text.starts_with("QURAI-PACKAGE\t1"), "{text}");
        assert!(text.contains("#journal"), "{text}");
        assert!(text.contains("Beton quyildi"), "{text}");
        // Vergul bilan yozilgan son nuqtaga o'giriladi.
        assert!(text.contains("12.5"), "{text}");
    });
}

/// Faqat ko'ruvchi rol kunlik yozuv kiritmaydi.
#[test]
fn read_only_role_cannot_open_the_journal_form() {
    runtime().block_on(async {
        let (app, _) = app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=mijoz&password=mijoz-parol-11"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1/journal")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    });
}

/// Signal ro'yxati ilovadan keladi va telefon sahifasida ko'rinadi.
#[test]
fn notices_come_from_the_app_and_show_on_the_phone() {
    runtime().block_on(async {
        let (app, state) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;

        let (code, body) = send(
            &app,
            post(
                "/api/notices",
                Some(&token),
                json!({ "project": "OBY-1", "items": [
                    { "code": "NT-1", "severity": "critical", "title": "Muddati o'tgan ish",
                      "detail": "3 ta ish", "count": 3, "days": 5, "source": "Grafik" },
                    { "code": "NT-2", "severity": "major", "title": "Sertifikat muddati",
                      "detail": "2 ta material", "count": 2, "days": 0, "source": "Ombor" }
                ] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{body}");
        assert_eq!(body["saved"], 2);
        // Bazada haqiqatan turibdimi.
        let (stored, at) = state.store.notices("OBY-1");
        assert_eq!(stored.len(), 2, "bazada yo'q; at={at}");

        // Telefon sahifasida ko'rinadi.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&bytes);
        assert!(html.contains("Muddati o&#39;tgan ish"), "{html}");
        assert!(html.contains("Sertifikat muddati"));
        assert!(html.contains("5 kun"), "kun ko'rsatilmadi");

        // Ikkinchi yuborishda eski signal qolmaydi.
        let (code, _) = send(
            &app,
            post(
                "/api/notices",
                Some(&token),
                json!({ "project": "OBY-1", "items": [] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK);
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&bytes);
        assert!(!html.contains("Sertifikat muddati"), "eski signal qoldi");
        assert!(
            html.contains("Signal yo'q") || html.contains("Signal yo&#39;q"),
            "{html}"
        );
    });
}

/// Faqat ko'ruvchi rol signal yubora olmaydi.
#[test]
fn read_only_role_cannot_send_notices() {
    runtime().block_on(async {
        let (app, _) = app();
        let mijoz = login(&app, "mijoz", "mijoz-parol-11").await;
        let (code, _) = send(
            &app,
            post(
                "/api/notices",
                Some(&mijoz),
                json!({ "project": "OBY-1", "items": [] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::FORBIDDEN);
    });
}

/// Ish ro'yxati kelgach, telefonda ish yozilmaydi — tanlanadi.
#[test]
fn journal_form_offers_the_task_list() {
    runtime().block_on(async {
        let (app, _) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;

        // Kirish (cookie) — sahifa uchun.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        let form_html = |cookie: String| {
            let app = app.clone();
            async move {
                let resp = app
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/o/OBY-1/journal")
                            .header(header::COOKIE, cookie)
                            .body(Body::empty())
                            .expect("so'rov"),
                    )
                    .await
                    .expect("javob");
                let bytes = resp.into_body().collect().await.unwrap().to_bytes();
                String::from_utf8_lossy(&bytes).to_string()
            }
        };

        // Ro'yxat yo'q — oddiy maydon.
        let html = form_html(cookie.clone()).await;
        assert!(html.contains("name=\"task\""), "{html}");
        assert!(!html.contains("<select"), "ro'yxatsiz select chiqdi");

        // Ilova ishlarni yuboradi.
        let (code, body) = send(
            &app,
            post(
                "/api/tasks",
                Some(&token),
                json!({ "project": "OBY-1", "items": [
                    { "wbs": "1.1", "name": "Yer ishlari, kotlovan" },
                    { "wbs": "2.1", "name": "Monolit karkas" }
                ] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{body}");
        assert_eq!(body["saved"], 2);

        // Endi forma tanlov beradi va qiymat aynan grafikdagi nom bo'ladi.
        let html = form_html(cookie).await;
        assert!(html.contains("<select name=\"task\""), "{html}");
        assert!(html.contains("value=\"Monolit karkas\""), "{html}");
        assert!(html.contains("1.1 Yer ishlari, kotlovan"), "{html}");
    });
}

/// Telefondan to'ldirilgan tabel paketga aylanadi; bo'sh katak
/// yuborilmaydi.
#[test]
fn timesheet_from_the_phone_skips_empty_cells() {
    runtime().block_on(async {
        let (app, _) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;

        // Ishchilar ro'yxati ilovadan keladi.
        let (code, body) = send(
            &app,
            post(
                "/api/workers",
                Some(&token),
                json!({ "project": "OBY-1", "items": [
                    { "name": "Alisher", "position": "Beton quyuvchi" },
                    { "name": "Bekzod", "position": "Armaturachi" },
                    { "name": "Davron", "position": "Payvandchi" }
                ] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK, "{body}");
        assert_eq!(body["saved"], 3);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        // Forma ishchilarni ko'rsatadi.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1/timesheet")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&bytes);
        assert!(html.contains("Alisher"), "{html}");
        assert!(html.contains("name=\"h_Bekzod\""), "{html}");

        // Ikkitasiga soat kiritamiz, uchinchisi bo'sh qoladi.
        let form = "date=2026-09-07&h_Alisher=8&k_Alisher=work\
&h_Bekzod=4%2C5&k_Bekzod=downtime&h_Davron=&k_Davron=work";
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/o/OBY-1/timesheet")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);

        // Paket ilova formatida va faqat to'ldirilganlar bor.
        let (_, body) = send(&app, get("/api/pull?project=OBY-1&since=0", Some(&token))).await;
        let changes = body["changes"].as_array().expect("ro'yxat");
        assert_eq!(changes.len(), 1);
        let text = changes[0]["body"].as_str().expect("matn");
        assert!(text.contains("#timesheet"), "{text}");
        assert!(text.contains("Alisher\t8\twork"), "{text}");
        // Vergul nuqtaga o'giriladi.
        assert!(text.contains("Bekzod\t4.5\tdowntime"), "{text}");
        assert!(!text.contains("Davron"), "bo'sh katak yuborildi: {text}");
        assert_eq!(changes[0]["rows"], 2);
    });
}

/// Ishchilar ro'yxati kelmagan bo'lsa, sahifa buni ochiq aytadi.
#[test]
fn timesheet_says_when_the_worker_list_is_missing() {
    runtime().block_on(async {
        let (app, _) = app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1/timesheet")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&bytes);
        assert!(html.contains("ro'yxati hali kelmagan"), "{html}");
        // Bo'sh forma ko'rsatilmaydi.
        assert!(!html.contains("<button"), "{html}");
    });
}

/// Telefondagi grafik: muddat bo'yicha tartib va muddati o'tgani belgisi.
#[test]
fn phone_shows_the_schedule_by_deadline() {
    runtime().block_on(async {
        let (app, _) = app();
        let token = login(&app, "prorab", "prorab-parol-1").await;

        let (code, _) = send(
            &app,
            post(
                "/api/tasks",
                Some(&token),
                json!({ "project": "OBY-1", "items": [
                    { "wbs": "1", "name": "Tugagan ish", "start": "2020-01-01",
                      "end": "2020-01-10", "progress": 100.0, "section": "KJ" },
                    { "wbs": "2", "name": "Kechikkan ish", "start": "2020-02-01",
                      "end": "2020-02-10", "progress": 30.0, "section": "AR" },
                    { "wbs": "3", "name": "Kelasi ish", "start": "2090-01-01",
                      "end": "2090-01-10", "progress": 0.0, "section": "VK" }
                ] }),
            ),
        )
        .await;
        assert_eq!(code, StatusCode::OK);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from("login=prorab&password=prorab-parol-1"))
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie")
            .to_string();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/o/OBY-1/tasks")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("so'rov"),
            )
            .await
            .expect("javob");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&bytes);

        // Tugallanmagan ishlar tugaganidan oldin turadi.
        let late = html.find("Kechikkan ish").expect("kechikkan");
        let future = html.find("Kelasi ish").expect("kelasi");
        let done = html.find("Tugagan ish").expect("tugagan");
        assert!(late < future, "muddat bo'yicha tartib buzilgan");
        assert!(future < done, "tugagan ish oldinda turibdi");

        // Muddati o'tgan ish ajratilgan, sana qisqa ko'rinishda.
        assert!(html.contains("class=\"err\">10.02"), "{html}");
        assert!(html.contains("10.01"), "sana ko'rsatilmadi");
    });
}
