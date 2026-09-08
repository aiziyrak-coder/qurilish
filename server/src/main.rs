//! QURAi server: sinxronizatsiya, kirish nazorati va mobil ko'rinish.
//!
//! Server desktop ilovaning **davomi**, o'rnini bosuvchisi emas. Butun
//! hisob-kitob va tekshiruv ilovada qoladi; server uchta ishni qiladi:
//!
//! 1. **Kim kirgani** — parol va rol serverda tekshiriladi. Desktop
//!    ilovada rol ish taqsimoti edi (baza fayli ochiq), bu yerda esa u
//!    haqiqiy chegara: bazaga faqat server tegadi.
//! 2. **Qurilmalar orasida almashish** — maydonchadagi kunlik ijro
//!    paket bo'lib serverga chiqadi, ofis undan o'qiydi. Paket formati
//!    desktop ilovaniki bilan bir xil.
//! 3. **Masofadan imzolash** — texnik nazorat hujjatni telefonidan
//!    tasdiqlaydi; imzo hujjat matniga bog'lanadi.
//!
//! Ishga tushirish:
//!
//! ```text
//! QURAI_BIND=0.0.0.0:8080 QURAI_DB=/var/qurai/server.db qurai-server
//! qurai-server --add-user prorab "Alisher" foreman   # parol so'raladi
//! ```
//!
//! **Xavfsizlik chegarasi ochiq aytiladi:** server HTTP beradi. Internetga
//! chiqarilganda uni TLS (HTTPS) beruvchi teskari proksi ortiga qo'yish
//! shart — aks holda parol va seans belgisi tarmoqda ochiq ketadi.

mod api;
mod auth;
#[cfg(test)]
mod http_tests;
mod state;
mod store;
mod web;

use axum::routing::{get, post};
use axum::Router;
use state::{AppState, Config};
use std::sync::Arc;

/// Hozirgi vaqt — ISO 8601, UTC.
///
/// Vaqt bir joyda hosil qilinadi: turli modullarda turli format bo'lsa,
/// solishtirish va saralash buzilardi.
pub fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Bugundan `days` kun keyingi vaqt.
pub fn plus_days(days: i64) -> String {
    (chrono::Utc::now() + chrono::Duration::days(days))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = Config::from_env();

    let store = match store::Store::open(&config.db) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("baza ochilmadi ({}): {e}", config.db.display());
            std::process::exit(1);
        }
    };

    // ---- Buyruqlar: foydalanuvchi qo'shish va parol almashtirish.
    match args.first().map(|s| s.as_str()) {
        Some("--add-user") => {
            let (Some(login), Some(name), Some(role)) = (args.get(1), args.get(2), args.get(3))
            else {
                eprintln!("qurai-server --add-user <login> <ism> <rol>");
                std::process::exit(2);
            };
            add_user(&store, login, name, role);
            return;
        }
        Some("--set-password") => {
            let Some(login) = args.get(1) else {
                eprintln!("qurai-server --set-password <login>");
                std::process::exit(2);
            };
            set_password(&store, login);
            return;
        }
        Some("--list-users") => {
            for u in store.users() {
                println!(
                    "{:<16} {:<12} {:<22} {}",
                    u.login,
                    u.role,
                    u.name,
                    if u.active { "faol" } else { "o'chirilgan" }
                );
            }
            return;
        }
        Some("--help") | Some("-h") => {
            println!("{HELP}");
            return;
        }
        Some(other) if other.starts_with("--") => {
            eprintln!("noma'lum buyruq: {other}\n\n{HELP}");
            std::process::exit(2);
        }
        _ => {}
    }

    // Birinchi ishga tushirishda foydalanuvchi bo'lmasa, buni aytamiz:
    // administratorni parolsiz o'zimiz yaratmaymiz.
    if store.users().is_empty() {
        println!(
            "Foydalanuvchi yo'q. Avval administrator qo'shing:\n  \
             qurai-server --add-user {} \"Ism Familiya\" admin",
            config.admin_login
        );
    }

    let removed = store.purge_sessions(&now());
    if removed > 0 {
        println!("muddati o'tgan seans tozalandi: {removed}");
    }

    let state = Arc::new(AppState { store, config });
    serve(state);
}

const HELP: &str = "\
QURAi server

Buyruqlar:
  (buyruqsiz)                       serverni ishga tushiradi
  --add-user <login> <ism> <rol>    foydalanuvchi qo'shadi (parol so'raladi)
  --set-password <login>            parolni almashtiradi
  --list-users                      foydalanuvchilar ro'yxati

Rollar: admin, director, pm, foreman, brigadier, supervisor, designer,
        estimator, supply, storekeeper, client, accountant

Muhit o'zgaruvchilari:
  QURAI_BIND   tinglash manzili (sukut: 127.0.0.1:8080)
  QURAI_DB     baza fayli (sukut: qurai-server.db)

Internetga chiqarishda HTTPS beruvchi teskari proksi ortiga qo'ying.";

/// Terminaldan parol so'raydi.
///
/// Parol ekranda ko'rinadi — bu holat ochiq aytiladi. Yashirin kiritish
/// uchun terminal rejimini o'zgartirish kerak; bu esa har tizimda har xil
/// va xatolik ehtimolini oshiradi. Parol qo'yish bir martalik ish,
/// shuning uchun oddiy yo'l tanlangan.
fn ask_password() -> Option<String> {
    println!("Parol kiriting (ekranda ko'rinadi), keyin Enter:");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok()?;
    let pass = line.trim().to_string();
    (!pass.is_empty()).then_some(pass)
}

fn add_user(store: &store::Store, login: &str, name: &str, role: &str) {
    if !ROLES.contains(&role) {
        eprintln!("noma'lum rol: {role}\nRollar: {}", ROLES.join(", "));
        std::process::exit(2);
    }
    let Some(pass) = ask_password() else {
        eprintln!("parol kiritilmadi");
        std::process::exit(2);
    };
    let hash = match auth::hash_password(&pass) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("parol qabul qilinmadi: {e}");
            std::process::exit(2);
        }
    };
    match store.add_user(login, name, role, &hash) {
        Ok(id) => println!("qo'shildi: {login} (#{id}, {role})"),
        Err(e) => {
            eprintln!("qo'shilmadi: {e}");
            std::process::exit(1);
        }
    }
}

fn set_password(store: &store::Store, login: &str) {
    let Some((user, _)) = store.user_by_login(login) else {
        eprintln!("foydalanuvchi topilmadi: {login}");
        std::process::exit(2);
    };
    let Some(pass) = ask_password() else {
        eprintln!("parol kiritilmadi");
        std::process::exit(2);
    };
    match auth::hash_password(&pass) {
        Ok(h) if store.set_hash(user.id, &h) => println!("parol almashtirildi: {login}"),
        Ok(_) => {
            eprintln!("parol saqlanmadi");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("parol qabul qilinmadi: {e}");
            std::process::exit(2);
        }
    }
}

/// Ruxsat etilgan rol kodlari — desktop ilovadagi bilan bir xil.
const ROLES: [&str; 12] = [
    "admin",
    "director",
    "pm",
    "foreman",
    "brigadier",
    "supervisor",
    "designer",
    "estimator",
    "supply",
    "storekeeper",
    "client",
    "accountant",
];

/// Yo'llar jadvali. Bitta joyda turadi — nima ochiq ekani ko'rinib tursin.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // ---- Mobil ko'rinish
        .route("/", get(web::index))
        .route("/login", post(web::login))
        .route("/logout", post(web::logout))
        .route("/o/{project}", get(web::object))
        .route("/o/{project}/sign", post(web::sign))
        .route("/o/{project}/journal", get(web::journal_form))
        .route("/o/{project}/journal", post(web::journal_submit))
        // ---- Desktop ilova bilan aloqa
        .route("/api/health", get(api::health))
        .route("/api/login", post(api::login))
        .route("/api/logout", post(api::logout))
        .route("/api/projects", get(api::projects))
        .route("/api/push", post(api::push))
        .route("/api/pull", get(api::pull))
        .route("/api/sign", post(api::sign))
        .route("/api/notices", post(api::notices))
        .route("/api/tasks", post(api::tasks))
        .route("/api/workers", post(api::workers))
        .route("/api/summary", post(api::summary))
        .route("/o/{project}/client", get(web::client_page))
        .route("/o/{project}/tasks", get(web::tasks_page))
        .route("/o/{project}/timesheet", get(web::timesheet_form))
        .route("/o/{project}/timesheet", post(web::timesheet_submit))
        .route("/api/signatures", get(api::signatures))
        .with_state(state)
}

/// Serverni ishga tushiradi.
fn serve(state: Arc<AppState>) {
    let bind = state.config.bind.clone();
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("ishga tushmadi: {e}");
            std::process::exit(1);
        }
    };
    rt.block_on(async move {
        let listener = match tokio::net::TcpListener::bind(&bind).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{bind} tinglanmadi: {e}");
                std::process::exit(1);
            }
        };
        println!("QURAi server: http://{bind}");
        if !bind.starts_with("127.0.0.1") && !bind.starts_with("localhost") {
            println!(
                "Diqqat: server tarmoqqa ochiq. HTTPS beruvchi teskari proksi ortiga qo'ying."
            );
        }
        if let Err(e) = axum::serve(listener, router(state)).await {
            eprintln!("server to'xtadi: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vaqt formati saralanadigan bo'lishi kerak: matn sifatida
    /// solishtirilganda ham tartib to'g'ri chiqadi.
    #[test]
    fn time_format_sorts_correctly() {
        let a = "2026-09-04T09:00:00Z".to_string();
        let b = "2026-09-04T10:00:00Z".to_string();
        assert!(a < b);
        let n = now();
        assert_eq!(n.len(), 20, "{n}");
        assert!(n.ends_with('Z'));
        assert!(plus_days(1) > n);
    }

    /// Rollar ro'yxati va huquqlar bir-biriga mos.
    #[test]
    fn every_role_has_rights() {
        for role in ROLES {
            let rights = auth::access(role);
            assert!(!rights.is_empty(), "{role}");
            assert!(rights.contains(&auth::Access::Read), "{role}");
        }
        // Faqat administrator boshqaradi.
        let admins: Vec<&str> = ROLES
            .iter()
            .copied()
            .filter(|r| auth::can(r, auth::Access::Admin))
            .collect();
        assert_eq!(admins, vec!["admin"]);
    }
}
