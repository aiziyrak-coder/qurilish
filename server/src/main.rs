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

/// UTC yozuvni maydoncha vaqtiga keltiradi (ko'rsatish uchun).
///
/// Bazadagi vaqt UTC bo'lib qoladi — bu yerda faqat **ko'rinishi**
/// o'zgaradi. Siljishi ko'rsatilmagan matn o'zgartirilmaydi: u qayerdan
/// kelganini bilmaymiz va taxmin qilish xato vaqt berardi.
pub fn in_zone(iso: &str, tz_minutes: i32) -> String {
    let t = iso.trim();
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(t) else {
        return t.to_string();
    };
    let Some(off) = chrono::FixedOffset::east_opt(tz_minutes * 60) else {
        return t.to_string();
    };
    dt.with_timezone(&off)
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

/// Maydonchadagi bugungi sana, `YYYY-MM-DD`.
pub fn site_today(tz_minutes: i32) -> String {
    in_zone(&now(), tz_minutes)
        .split('T')
        .next()
        .unwrap_or("")
        .to_string()
}

/// `minutes` daqiqa oldingi vaqt — urinishlar oynasining boshi.
pub fn minus_minutes(minutes: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::minutes(minutes))
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
  QURAI_HTTPS  1 — server HTTPS beruvchi proksi ortida: seans belgisiga
               `Secure` qo'yiladi va u HTTP orqali ketmaydi
  QURAI_TZ     maydoncha vaqt mintaqasi, masalan `+05:00` (sukut: server
               kompyuterining mintaqasi). Telefondagi «bugun» va soat shu
               bo'yicha hisoblanadi; bazada vaqt UTC da qoladi

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

/// Har javobga qo'yiladigan xavfsizlik sarlavhalari.
///
/// Bular brauzerga **nimaga ruxsat yo'q** ekanini aytadi:
///
/// - `nosniff` — brauzer fayl turini o'zi taxmin qilmaydi;
/// - `frame-ancestors 'none'` va `X-Frame-Options` — sahifani boshqa
///   saytning ichiga qo'yib bo'lmaydi, shu sababli bosishni o'g'irlash
///   ishlamaydi;
/// - `default-src 'self'` — rasm, uslub va skript faqat shu serverdan;
/// - `form-action 'self'` — forma boshqa manzilga yuborilmaydi;
/// - `no-referrer` — obyekt nomi boshqa saytga sarlavha bilan ketmaydi.
///
/// Chegara ochiq aytiladi: sahifalardagi kichik skriptlar HTML ichida
/// turadi, shuning uchun `script-src` da `'unsafe-inline'` qolgan. Ya'ni
/// CSP tashqi manbadan skript yuklanishini va sahifani ramkaga olishni
/// to'sadi, lekin sahifaga qo'shib qo'yilgan skriptdan himoya qilmaydi —
/// undan HTML ga tushadigan har bir matnni qochirish himoya qiladi (`esc`).
const CSP: &str = concat!(
    "default-src 'self'; img-src 'self' data:; ",
    "style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; ",
    "connect-src 'self'; form-action 'self'; ",
    "frame-ancestors 'none'; base-uri 'none'"
);

async fn security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::HeaderValue;
    let mut resp = next.run(req).await;
    let h = resp.headers_mut();
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert("content-security-policy", HeaderValue::from_static(CSP));
    resp
}

/// Yo'llar jadvali. Bitta joyda turadi — nima ochiq ekani ko'rinib tursin.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // ---- Mobil ko'rinish
        .route("/", get(web::index))
        .route("/login", post(web::login))
        .route("/logout", post(web::logout))
        // ---- Telefonga o'rnatiladigan ko'rinish (PWA)
        .route("/manifest.webmanifest", get(web::manifest))
        .route("/icon.svg", get(web::icon))
        .route("/sw.js", get(web::service_worker))
        .route("/offline", get(web::offline_page))
        .route("/o/{project}", get(web::object))
        .route("/o/{project}/sign", post(web::sign))
        .route("/o/{project}/journal", get(web::journal_form))
        .route("/o/{project}/journal", post(web::journal_submit))
        .route("/o/{project}/checkin", get(web::checkin_page))
        .route("/o/{project}/checkin", post(web::checkin_submit))
        .route("/o/{project}/request", get(web::request_form))
        .route("/o/{project}/request", post(web::request_submit))
        .route("/o/{project}/machine", get(web::machine_form))
        .route("/o/{project}/machine", post(web::machine_submit))
        .route("/o/{project}/scan", get(web::scan_page))
        .route("/o/{project}/chat", get(web::chat_page))
        .route("/o/{project}/chat", post(web::chat_submit))
        .route("/o/{project}/find", get(web::find_page))
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
        .route("/api/labels", post(api::labels))
        .route("/api/machines", post(api::machines))
        .route("/api/message", post(api::message))
        .route("/api/messages", get(api::messages))
        .route("/api/chain", post(api::chain_mark))
        .route("/api/chain", get(api::chain_list))
        .route("/o/{project}/client", get(web::client_page))
        .route("/o/{project}/tasks", get(web::tasks_page))
        .route("/o/{project}/timesheet", get(web::timesheet_form))
        .route("/o/{project}/timesheet", post(web::timesheet_submit))
        .route("/api/signatures", get(api::signatures))
        // Sarlavhalar hamma javobga qo'yiladi — sinovlarda ham, chunki
        // ular shu `router` ni ishlatadi.
        .layer(axum::middleware::from_fn(security_headers))
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
        // Mintaqa ko'rinib tursin: noto'g'ri siljish bilan «bugun» boshqa
        // kun bo'lib qolardi va buni keyin topish qiyin.
        let tz = state.config.tz_minutes;
        println!(
            "Maydoncha vaqti: UTC{}{:02}:{:02} · bugun {}",
            if tz < 0 { '-' } else { '+' },
            tz.abs() / 60,
            tz.abs() % 60,
            site_today(tz)
        );
        let local = bind.starts_with("127.0.0.1") || bind.starts_with("localhost");
        if !local {
            println!(
                "Diqqat: server tarmoqqa ochiq. HTTPS beruvchi teskari proksi ortiga qo'ying."
            );
            if !state.config.https {
                println!("Diqqat: QURAI_HTTPS qo'yilmagan — seans belgisiga `Secure` qo'yilmaydi.");
                println!("Proksi HTTPS bersa, `QURAI_HTTPS=1` ni ham qo'ying.");
            }
        }

        // Muddati o'tgan seans va eski urinish yozuvlari fonda tozalanadi.
        // Ishga tushishdagi bitta tozalash uzoq ishlab turgan serverda
        // yetmaydi: jadvallar oylar davomida o'sib borardi.
        let keeper = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                keeper.store.purge_sessions(&now());
                keeper
                    .store
                    .purge_login_tries(&minus_minutes(auth::TRY_WINDOW_MINUTES));
            }
        });

        let served = run(listener, state, shutdown()).await;
        if let Err(e) = served {
            eprintln!("server to'xtadi: {e}");
        }
        println!("to'xtadi.");
    });
}

/// So'rovlarni qabul qiladi va `stop` tugaganda shafqatli to'xtaydi.
///
/// To'xtash belgisi **parametr** qilib olingan: shunda sinov uni o'zi
/// berib, haqiqiy portda to'xtash xatti-harakatini tekshira oladi.
/// `Ctrl+C` ni sinovda hosil qilib bo'lmaydi.
pub async fn run(
    listener: tokio::net::TcpListener,
    state: Arc<AppState>,
    stop: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router(state))
        .with_graceful_shutdown(stop)
        .await
}

/// `Ctrl+C` ni kutadi.
///
/// Shuning uchun kerak: belgi kelganda server yangi so'rov qabul qilmaydi,
/// lekin **ketayotgan** so'rovlar tugatiladi. Avval ulanish o'rtasida
/// uzilardi — telefon yozuv yuborilmadi deb ko'rsatib, yozuv esa bazaga
/// tushib qolishi mumkin edi.
async fn shutdown() {
    if tokio::signal::ctrl_c().await.is_err() {
        // Belgini kuzatib bo'lmasa, to'xtatishni kutib o'tirmaymiz.
        return;
    }
    println!("to'xtatilmoqda: ketayotgan so'rovlar tugatiladi...");
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

    /// Server belgi kelganda to'xtaydi va porti bo'shaydi.
    ///
    /// Shu tekshiriladi: to'xtashdan **oldin** yuborilgan so'rov to'liq
    /// javob oladi, to'xtagandan keyin esa port yopiladi. Avval `serve`
    /// umuman to'xtash belgisini kutmasdi — `Ctrl+C` da ketayotgan
    /// so'rovlar uzilib qolardi.
    #[test]
    fn the_server_stops_when_it_is_told_to() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let store = store::Store::memory().expect("baza");
            let state = Arc::new(AppState {
                store,
                config: Config::default(),
            });
            // 0 — tizim bo'sh portni o'zi tanlaydi: sinov band portga
            // bog'liq bo'lmasligi kerak.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("port");
            let addr = listener.local_addr().expect("manzil");
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let server = tokio::spawn(run(listener, state, async {
                let _ = rx.await;
            }));

            // To'xtashdan oldin so'rov o'tadi.
            let before = tcp_get(addr, "/api/health").await;
            assert!(before.starts_with("HTTP/1.1 200"), "{before}");

            // Belgi beriladi va server o'zi tugaydi.
            tx.send(()).expect("belgi");
            let done = tokio::time::timeout(std::time::Duration::from_secs(5), server)
                .await
                .expect("server to'xtamadi")
                .expect("vazifa");
            assert!(done.is_ok(), "{done:?}");

            // Port bo'shadi: yangi ulanish o'rnatilmaydi.
            assert!(
                tokio::net::TcpStream::connect(addr).await.is_err(),
                "port hali ham ochiq"
            );
        });
    }

    /// Oddiy HTTP so'rovi: javobning birinchi qatorini qaytaradi.
    async fn tcp_get(addr: std::net::SocketAddr, path: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock = tokio::net::TcpStream::connect(addr).await.expect("ulanish");
        let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        sock.write_all(req.as_bytes()).await.expect("yuborish");
        let mut out = Vec::new();
        sock.read_to_end(&mut out).await.expect("o'qish");
        String::from_utf8_lossy(&out).to_string()
    }

    /// Maydoncha vaqti UTC dan siljiydi va kun chegarasi ham siljiydi.
    ///
    /// Toshkentda (UTC+5) yarim tundan keyingi belgi UTC da hali
    /// **kechagi** kunga tegishli edi: tunda kirgan ishchi ertalab yana
    /// «Kirdim» tugmasini ko'rardi va kunlik yozuv formasi kechagi sanani
    /// taklif qilardi.
    #[test]
    fn site_time_moves_the_day_boundary() {
        // UTC yarim tundan oldin — Toshkentda allaqachon ertangi kun.
        assert_eq!(in_zone("2026-10-03T21:30:00Z", 300), "2026-10-04T02:30:00");
        assert_eq!(in_zone("2026-10-03T03:00:00Z", 300), "2026-10-03T08:00:00");
        // Manfiy siljish ham ishlaydi.
        assert_eq!(in_zone("2026-10-03T02:00:00Z", -210), "2026-10-02T22:30:00");
        // Siljish nol — vaqt o'zgarmaydi.
        assert_eq!(in_zone("2026-10-03T03:00:00Z", 0), "2026-10-03T03:00:00");
        // Siljishi ko'rsatilmagan matn o'zgartirilmaydi.
        assert_eq!(in_zone("2026-10-03", 300), "2026-10-03");
        assert_eq!(in_zone("", 300), "");
        // Bugungi sana — o'n belgi.
        assert_eq!(site_today(300).len(), 10);
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
