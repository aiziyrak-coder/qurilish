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
