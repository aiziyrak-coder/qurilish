//! Server ombori: foydalanuvchilar, seanslar, o'zgarishlar va imzolar.
//!
//! Server **qurilish mantiqini takrorlamaydi**. Barcha hisob-kitob desktop
//! ilovada qoladi; server uchta ishni bajaradi:
//!
//! 1. **Kim kirgan** — parol va seans; rol bo'yicha huquq.
//! 2. **Nima o'zgargan** — qurilmalardan kelgan paketlarni tartib bilan
//!    saqlaydi va boshqalarga tarqatadi.
//! 3. **Kim imzoladi** — hujjat imzosi: kim, qachon va **nimani** imzolagani
//!    (matn xesh-yig'indisi bilan).
//!
//! Paket formati desktop ilovaniki bilan bir xil — bu ataylab: server yangi
//! format o'ylab topmaydi, shu sababli ikkala tomonda ma'lumot bir xil
//! tushuniladi.

use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

/// Foydalanuvchi.
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub name: String,
    /// Desktop ilovadagi rol kodi (`admin`, `foreman`, `supervisor`, ...).
    pub role: String,
    pub active: bool,
}

/// Qurilmadan kelgan bitta o'zgarishlar paketi.
#[derive(Debug, Clone)]
pub struct Change {
    pub id: i64,
    pub project: String,
    /// Paket matni — desktop ilova formatida.
    pub body: String,
    pub author: String,
    pub at: String,
    pub rows: i64,
}

/// Hujjat imzosi.
#[derive(Debug, Clone)]
pub struct Signature {
    pub id: i64,
    pub project: String,
    /// Hujjat raqami — desktop ilovadagi ijro hujjati bilan bog'lanadi.
    pub document: String,
    pub user: String,
    pub role: String,
    pub at: String,
    /// Imzolangan matnning xesh-yig'indisi.
    pub digest: String,
    /// Rad etilgan bo'lsa sababi; bo'sh bo'lsa — tasdiqlangan.
    pub rejected: String,
}

/// Obyekt bo'yicha bitta signal.
///
/// Signal serverda **hisoblanmaydi**: uni desktop ilova hisoblab yuboradi.
/// Shu sababli telefonda ko'ringan son ofisdagi ekrandagi son bilan bir
/// xil bo'ladi — ikkinchi hisob bo'lsa, ular albatta bir-biridan farq
/// qilib qolardi.
#[derive(Debug, Clone)]
pub struct Notice {
    pub code: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub count: i64,
    pub days: i64,
    pub source: String,
}

/// Ish ro'yxatidagi bitta qator.
///
/// Ish nomini prorab qo'lda yozsa, u grafikdagi nom bilan mos kelmaydi va
/// keyin hajm qaysi ishga tegishli ekani noma'lum bo'lib qoladi. Shuning
/// uchun telefonga tayyor ro'yxat beriladi va u yerdan **tanlanadi**.
#[derive(Debug, Clone)]
pub struct TaskRef {
    pub wbs: String,
    pub name: String,
    /// Reja bo'yicha boshlanish va tugash sanasi (`YYYY-MM-DD`).
    pub start: String,
    pub end: String,
    /// Bajarilish foizi.
    pub progress: f64,
    /// Bo'lim kodi: AR, KJ, VK va h.k.
    pub section: String,
}

/// Tabel uchun ishchi.
///
/// Ish ro'yxati kabi bu ham grafik va tabelning nusxasi: serverda
/// o'zgartirilmaydi, faqat telefonda tanlash uchun ko'rsatiladi.
#[derive(Debug, Clone)]
pub struct WorkerRef {
    pub name: String,
    pub position: String,
}

/// Obyekt yakunining bitta qatori (TZ VIII.35).
///
/// Buyurtmachi kabineti shu qatorlarni ko'rsatadi. Ular serverda
/// hisoblanmaydi — ilova hisoblab yuboradi, shuning uchun kabinetdagi
/// son pudratchi ekranidagi son bilan bir xil bo'ladi.
#[derive(Debug, Clone)]
pub struct SummaryRow {
    pub module: String,
    pub indicator: String,
    pub value: String,
}

/// Baza. Bitta ulanish mutex ostida: server kichik va yozuvlar qisqa.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Bazani ochadi va sxemani yaratadi.
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        // WAL — bir vaqtda o'qish va yozish uchun.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "foreign_keys", "ON");
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// Sinov uchun xotiradagi baza.
    pub fn memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        // Mutex buzilgan bo'lsa ham ishlashda davom etamiz: bu bitta
        // so'rovning xatosi, butun serverni to'xtatish sababi emas.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ---------------------------------------------------------- Foydalanuvchi

    /// Foydalanuvchi qo'shadi. Parol xeshi tayyor holda keladi — bu modul
    /// parolning o'zini hech qachon ko'rmaydi va saqlamaydi.
    pub fn add_user(&self, login: &str, name: &str, role: &str, hash: &str) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO user (login,name,role,hash,active) VALUES (?1,?2,?3,?4,1)",
            params![login.trim().to_lowercase(), name, role, hash],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// Login bo'yicha foydalanuvchi va uning parol xeshi.
    pub fn user_by_login(&self, login: &str) -> Option<(User, String)> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id,login,name,role,active,hash FROM user WHERE login=?1",
            params![login.trim().to_lowercase()],
            |r| {
                Ok((
                    User {
                        id: r.get(0)?,
                        login: r.get(1)?,
                        name: r.get(2)?,
                        role: r.get(3)?,
                        active: r.get::<_, i64>(4)? != 0,
                    },
                    r.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn user(&self, id: i64) -> Option<User> {
        let conn = self.lock();
        conn.query_row(
            "SELECT id,login,name,role,active FROM user WHERE id=?1",
            params![id],
            |r| {
                Ok(User {
                    id: r.get(0)?,
                    login: r.get(1)?,
                    name: r.get(2)?,
                    role: r.get(3)?,
                    active: r.get::<_, i64>(4)? != 0,
                })
            },
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn users(&self) -> Vec<User> {
        let conn = self.lock();
        let mut st = match conn.prepare("SELECT id,login,name,role,active FROM user ORDER BY login")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map([], |r| {
            Ok(User {
                id: r.get(0)?,
                login: r.get(1)?,
                name: r.get(2)?,
                role: r.get(3)?,
                active: r.get::<_, i64>(4)? != 0,
            })
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    /// Parolni almashtiradi (xesh tayyor holda keladi).
    pub fn set_hash(&self, user_id: i64, hash: &str) -> bool {
        self.lock()
            .execute(
                "UPDATE user SET hash=?2 WHERE id=?1",
                params![user_id, hash],
            )
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    /// Foydalanuvchini yoqadi yoki o'chiradi. Yozuv o'chirilmaydi: uning
    /// imzolari va o'zgarishlari tarixda qolishi kerak.
    pub fn set_active(&self, user_id: i64, active: bool) -> bool {
        self.lock()
            .execute(
                "UPDATE user SET active=?2 WHERE id=?1",
                params![user_id, i64::from(active)],
            )
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    // ---------------------------------------------------------------- Seans

    /// Seans ochadi va uning belgisini qaytaradi.
    pub fn open_session(&self, user_id: i64, token: &str, until: &str) -> bool {
        self.lock()
            .execute(
                "INSERT INTO session (token,user_id,until) VALUES (?1,?2,?3)",
                params![token, user_id, until],
            )
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    /// Belgi bo'yicha foydalanuvchi. Muddati o'tgan seans qabul qilinmaydi.
    pub fn session_user(&self, token: &str, now: &str) -> Option<User> {
        let conn = self.lock();
        let id: Option<i64> = conn
            .query_row(
                "SELECT user_id FROM session WHERE token=?1 AND until > ?2",
                params![token, now],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        drop(conn);
        let user = self.user(id?)?;
        user.active.then_some(user)
    }

    pub fn close_session(&self, token: &str) -> bool {
        self.lock()
            .execute("DELETE FROM session WHERE token=?1", params![token])
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    /// Muddati o'tgan seanslarni tozalaydi.
    pub fn purge_sessions(&self, now: &str) -> usize {
        self.lock()
            .execute("DELETE FROM session WHERE until <= ?1", params![now])
            .unwrap_or(0)
    }

    // ----------------------------------------------------------- O'zgarishlar

    /// Paketni saqlaydi va uning tartib raqamini qaytaradi.
    pub fn push_change(
        &self,
        project: &str,
        body: &str,
        author: &str,
        at: &str,
        rows: i64,
    ) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO change (project,body,author,at,rows) VALUES (?1,?2,?3,?4,?5)",
            params![project, body, author, at, rows],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// `since` dan keyingi o'zgarishlar — tartib bo'yicha.
    ///
    /// Tartib muhim: paketlar bir-birining ustiga qo'yiladi, shuning uchun
    /// ular yozilgan tartibda qaytariladi.
    pub fn changes_since(&self, project: &str, since: i64, limit: i64) -> Vec<Change> {
        let conn = self.lock();
        let mut st = match conn.prepare(
            "SELECT id,project,body,author,at,rows FROM change
             WHERE project=?1 AND id>?2 ORDER BY id LIMIT ?3",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map(params![project, since, limit], |r| {
            Ok(Change {
                id: r.get(0)?,
                project: r.get(1)?,
                body: r.get(2)?,
                author: r.get(3)?,
                at: r.get(4)?,
                rows: r.get(5)?,
            })
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    /// Obyektdagi eng oxirgi o'zgarish raqami.
    pub fn last_change(&self, project: &str) -> i64 {
        self.lock()
            .query_row(
                "SELECT COALESCE(MAX(id),0) FROM change WHERE project=?1",
                params![project],
                |r| r.get(0),
            )
            .unwrap_or(0)
    }

    /// Serverdagi obyektlar ro'yxati va ularning oxirgi o'zgarishi.
    pub fn projects(&self) -> Vec<(String, i64, String)> {
        let conn = self.lock();
        let mut st = match conn.prepare(
            "SELECT project, MAX(id), MAX(at) FROM change GROUP BY project ORDER BY project",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)));
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    // ---------------------------------------------------------------- Imzo

    /// Imzo qo'yadi. Bir hujjatni bitta odam ikki marta imzolamaydi.
    pub fn sign(&self, s: &Signature) -> Result<i64, String> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO signature (project,document,user,role,at,digest,rejected)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(project,document,user) DO NOTHING",
            params![s.project, s.document, s.user, s.role, s.at, s.digest, s.rejected],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn signatures(&self, project: &str) -> Vec<Signature> {
        let conn = self.lock();
        let mut st = match conn.prepare(
            "SELECT id,project,document,user,role,at,digest,rejected FROM signature
             WHERE project=?1 ORDER BY at DESC, id DESC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map(params![project], |r| {
            Ok(Signature {
                id: r.get(0)?,
                project: r.get(1)?,
                document: r.get(2)?,
                user: r.get(3)?,
                role: r.get(4)?,
                at: r.get(5)?,
                digest: r.get(6)?,
                rejected: r.get(7)?,
            })
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    // -------------------------------------------------------------- Signal

    /// Obyektning signal ro'yxatini **butunlay almashtiradi**.
    ///
    /// Signal — bu holat surati, tarix emas: eskisi saqlanib qolsa,
    /// bartaraf etilgan muammo telefonda turaverardi. Shuning uchun
    /// ro'yxat har safar to'liq qayta yoziladi.
    pub fn set_notices(&self, project: &str, at: &str, items: &[Notice]) -> Result<usize, String> {
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM notice WHERE project=?1", params![project])
            .map_err(|e| e.to_string())?;
        // Hisob vaqti alohida saqlanadi: bo'sh ro'yxat ham **javob** —
        // «hisoblandi, signal yo'q». Vaqt signal qatorlarida tursa, ro'yxat
        // tozalangach «hech qachon yuborilmagan» bilan chalkashib ketardi.
        tx.execute(
            "INSERT INTO notice_state (project,at) VALUES (?1,?2)
             ON CONFLICT(project) DO UPDATE SET at=excluded.at",
            params![project, at],
        )
        .map_err(|e| e.to_string())?;
        for n in items {
            tx.execute(
                "INSERT INTO notice (project,code,severity,title,detail,count,days,source,at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    project, n.code, n.severity, n.title, n.detail, n.count, n.days, n.source, at
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(items.len())
    }

    /// Obyekt signallari va ular qachon hisoblangani.
    pub fn notices(&self, project: &str) -> (Vec<Notice>, String) {
        let conn = self.lock();
        let at: String = conn
            .query_row(
                "SELECT at FROM notice_state WHERE project=?1",
                params![project],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or_default();
        let mut st = match conn.prepare(
            "SELECT code,severity,title,detail,count,days,source FROM notice
             WHERE project=?1 ORDER BY id",
        ) {
            Ok(s) => s,
            Err(_) => return (Vec::new(), at),
        };
        let rows = st.query_map(params![project], |r| {
            Ok(Notice {
                code: r.get(0)?,
                severity: r.get(1)?,
                title: r.get(2)?,
                detail: r.get(3)?,
                count: r.get(4)?,
                days: r.get(5)?,
                source: r.get(6)?,
            })
        });
        let list = rows
            .map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default();
        (list, at)
    }

    // ------------------------------------------------------------ Ishlar

    /// Obyektning ish ro'yxatini almashtiradi.
    ///
    /// Ro'yxat grafikning nusxasi: u serverda o'zgartirilmaydi va
    /// hisoblanmaydi — faqat telefonda tanlash uchun ko'rsatiladi.
    pub fn set_tasks(&self, project: &str, items: &[TaskRef]) -> Result<usize, String> {
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM task_ref WHERE project=?1", params![project])
            .map_err(|e| e.to_string())?;
        for t in items {
            tx.execute(
                "INSERT INTO task_ref (project,wbs,name,start,finish,progress,section)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![project, t.wbs, t.name, t.start, t.end, t.progress, t.section],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(items.len())
    }

    pub fn tasks(&self, project: &str) -> Vec<TaskRef> {
        let conn = self.lock();
        let mut st = match conn.prepare(
            "SELECT wbs,name,start,finish,progress,section FROM task_ref
             WHERE project=?1 ORDER BY id",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map(params![project], |r| {
            Ok(TaskRef {
                wbs: r.get(0)?,
                name: r.get(1)?,
                start: r.get(2)?,
                end: r.get(3)?,
                progress: r.get(4)?,
                section: r.get(5)?,
            })
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    // ----------------------------------------------------------- Ishchilar

    /// Ishchilar ro'yxatini almashtiradi.
    pub fn set_workers(&self, project: &str, items: &[WorkerRef]) -> Result<usize, String> {
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM worker_ref WHERE project=?1", params![project])
            .map_err(|e| e.to_string())?;
        for w in items {
            tx.execute(
                "INSERT INTO worker_ref (project,name,position) VALUES (?1,?2,?3)",
                params![project, w.name, w.position],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(items.len())
    }

    pub fn workers(&self, project: &str) -> Vec<WorkerRef> {
        let conn = self.lock();
        let mut st = match conn
            .prepare("SELECT name,position FROM worker_ref WHERE project=?1 ORDER BY id")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map(params![project], |r| {
            Ok(WorkerRef {
                name: r.get(0)?,
                position: r.get(1)?,
            })
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }

    // ------------------------------------------------------------- Yakun

    /// Obyekt yakunini almashtiradi (TZ VIII.35).
    pub fn set_summary(
        &self,
        project: &str,
        at: &str,
        rows: &[SummaryRow],
    ) -> Result<usize, String> {
        let mut conn = self.lock();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM summary WHERE project=?1", params![project])
            .map_err(|e| e.to_string())?;
        for r in rows {
            tx.execute(
                "INSERT INTO summary (project,module,indicator,value,at)
                 VALUES (?1,?2,?3,?4,?5)",
                params![project, r.module, r.indicator, r.value, at],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(rows.len())
    }

    /// Obyekt yakuni va u qachon hisoblangani.
    pub fn summary(&self, project: &str) -> (Vec<SummaryRow>, String) {
        let conn = self.lock();
        let at: String = conn
            .query_row(
                "SELECT COALESCE(MAX(at),'') FROM summary WHERE project=?1",
                params![project],
                |r| r.get(0),
            )
            .unwrap_or_default();
        let mut st = match conn
            .prepare("SELECT module,indicator,value FROM summary WHERE project=?1 ORDER BY id")
        {
            Ok(s) => s,
            Err(_) => return (Vec::new(), at),
        };
        let rows = st.query_map(params![project], |r| {
            Ok(SummaryRow {
                module: r.get(0)?,
                indicator: r.get(1)?,
                value: r.get(2)?,
            })
        });
        let list = rows
            .map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default();
        (list, at)
    }

    // --------------------------------------------------------------- Jurnal

    /// Amallar jurnali: kim, qachon, nima qildi.
    ///
    /// Parol, belgi va boshqa maxfiy qiymatlar bu yerga **hech qachon**
    /// yozilmaydi — faqat amalning nomi va obyekt.
    pub fn log(&self, user: &str, action: &str, detail: &str, at: &str) {
        let _ = self.lock().execute(
            "INSERT INTO server_log (user,action,detail,at) VALUES (?1,?2,?3,?4)",
            params![user, action, detail, at],
        );
    }

    pub fn recent_log(&self, limit: i64) -> Vec<(String, String, String, String)> {
        let conn = self.lock();
        let mut st = match conn
            .prepare("SELECT user,action,detail,at FROM server_log ORDER BY id DESC LIMIT ?1")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = st.query_map(params![limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        });
        rows.map(|it| it.filter_map(|x| x.ok()).collect())
            .unwrap_or_default()
    }
}

/// Sxema. Har bir jadval bitta savolga javob beradi.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS user (
    id INTEGER PRIMARY KEY,
    login TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    hash TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS session (
    token TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES user(id),
    until TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS change (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    body TEXT NOT NULL,
    author TEXT NOT NULL,
    at TEXT NOT NULL,
    rows INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS change_project ON change(project, id);
CREATE TABLE IF NOT EXISTS signature (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    document TEXT NOT NULL,
    user TEXT NOT NULL,
    role TEXT NOT NULL,
    at TEXT NOT NULL,
    digest TEXT NOT NULL,
    rejected TEXT NOT NULL DEFAULT '',
    UNIQUE(project, document, user)
);
CREATE TABLE IF NOT EXISTS notice (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    code TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    detail TEXT NOT NULL DEFAULT '',
    count INTEGER NOT NULL DEFAULT 0,
    days INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT '',
    at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS notice_project ON notice(project);
CREATE TABLE IF NOT EXISTS task_ref (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    wbs TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL,
    start TEXT NOT NULL DEFAULT '',
    finish TEXT NOT NULL DEFAULT '',
    progress REAL NOT NULL DEFAULT 0,
    section TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS task_ref_project ON task_ref(project);
CREATE TABLE IF NOT EXISTS worker_ref (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    name TEXT NOT NULL,
    position TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS worker_ref_project ON worker_ref(project);
CREATE TABLE IF NOT EXISTS summary (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL,
    module TEXT NOT NULL,
    indicator TEXT NOT NULL,
    value TEXT NOT NULL,
    at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS summary_project ON summary(project);
CREATE TABLE IF NOT EXISTS notice_state (
    project TEXT PRIMARY KEY,
    at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS server_log (
    id INTEGER PRIMARY KEY,
    user TEXT NOT NULL,
    action TEXT NOT NULL,
    detail TEXT NOT NULL,
    at TEXT NOT NULL
);
";

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::memory().expect("baza")
    }

    #[test]
    fn user_round_trips_and_login_is_case_insensitive() {
        let s = store();
        let id = s.add_user("Prorab", "Alisher", "foreman", "xesh").unwrap();
        let (u, hash) = s.user_by_login("PRORAB").expect("topildi");
        assert_eq!(u.id, id);
        assert_eq!(u.login, "prorab");
        assert_eq!(u.role, "foreman");
        assert_eq!(hash, "xesh");
        assert!(u.active);
        // Bir xil login ikki marta qo'shilmaydi.
        assert!(s.add_user("prorab", "Boshqa", "admin", "x").is_err());
    }

    #[test]
    fn expired_session_is_not_accepted() {
        let s = store();
        let id = s.add_user("a", "A", "admin", "h").unwrap();
        assert!(s.open_session(id, "belgi", "2020-01-01T00:00:00Z"));
        assert!(s.session_user("belgi", "2026-01-01T00:00:00Z").is_none());

        assert!(s.open_session(id, "yangi", "2030-01-01T00:00:00Z"));
        assert!(s.session_user("yangi", "2026-01-01T00:00:00Z").is_some());
        // O'chirilgan foydalanuvchining seansi ham ishlamaydi.
        s.set_active(id, false);
        assert!(s.session_user("yangi", "2026-01-01T00:00:00Z").is_none());
    }

    #[test]
    fn changes_come_back_in_order_after_the_marker() {
        let s = store();
        let a = s.push_change("OBY-1", "birinchi", "a", "t1", 1).unwrap();
        let b = s.push_change("OBY-1", "ikkinchi", "a", "t2", 2).unwrap();
        s.push_change("OBY-2", "boshqa obyekt", "a", "t3", 3)
            .unwrap();

        let all = s.changes_since("OBY-1", 0, 100);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, a);
        assert_eq!(all[1].id, b);

        // Belgidan keyingilari — faqat yangilari.
        let rest = s.changes_since("OBY-1", a, 100);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].body, "ikkinchi");

        assert_eq!(s.last_change("OBY-1"), b);
        assert_eq!(s.last_change("YO-Q"), 0);
        assert_eq!(s.projects().len(), 2);
    }

    #[test]
    fn one_person_signs_a_document_once() {
        let s = store();
        let sig = Signature {
            id: 0,
            project: "OBY-1".into(),
            document: "AOSR-001".into(),
            user: "prorab".into(),
            role: "foreman".into(),
            at: "2026-09-04T10:00:00Z".into(),
            digest: "abc".into(),
            rejected: String::new(),
        };
        s.sign(&sig).unwrap();
        s.sign(&sig).unwrap();
        assert_eq!(s.signatures("OBY-1").len(), 1);

        // Boshqa odam o'sha hujjatni imzolashi mumkin.
        let mut other = sig.clone();
        other.user = "nazorat".into();
        s.sign(&other).unwrap();
        assert_eq!(s.signatures("OBY-1").len(), 2);
    }

    /// Signal ro'yxati to'liq almashtiriladi: eski signal qolib ketmaydi.
    #[test]
    fn notices_are_replaced_not_appended() {
        let s = store();
        let n = |code: &str| Notice {
            code: code.into(),
            severity: "major".into(),
            title: format!("{code} sarlavhasi"),
            detail: String::new(),
            count: 2,
            days: 3,
            source: "schedule".into(),
        };
        s.set_notices("OBY-1", "t1", &[n("A"), n("B")]).unwrap();
        let (list, at) = s.notices("OBY-1");
        assert_eq!(list.len(), 2);
        assert_eq!(at, "t1");

        // Ikkinchi yuborishda faqat yangi ro'yxat qoladi.
        s.set_notices("OBY-1", "t2", &[n("C")]).unwrap();
        let (list, at) = s.notices("OBY-1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].code, "C");
        assert_eq!(at, "t2");

        // Bo'sh ro'yxat — hammasi bartaraf etilgan. Bu «hisoblanmagan»
        // emas: hisob vaqti saqlanib qoladi.
        s.set_notices("OBY-1", "t3", &[]).unwrap();
        let (list, at) = s.notices("OBY-1");
        assert!(list.is_empty());
        assert_eq!(at, "t3", "hisob vaqti yo'qoldi");
        // Umuman yuborilmagan obyektda vaqt ham bo'lmaydi.
        assert_eq!(s.notices("YUBORILMAGAN").1, "");

        // Boshqa obyekt tegilmaydi.
        s.set_notices("OBY-2", "t1", &[n("D")]).unwrap();
        assert_eq!(s.notices("OBY-2").0.len(), 1);
        assert!(s.notices("OBY-1").0.is_empty());
    }

    /// Ish ro'yxati almashtiriladi va tartibi saqlanadi.
    #[test]
    fn task_list_is_replaced_and_keeps_order() {
        let s = store();
        let t = |wbs: &str, name: &str| TaskRef {
            wbs: wbs.into(),
            name: name.into(),
            start: "2026-09-01".into(),
            end: "2026-09-10".into(),
            progress: 40.0,
            section: "AR".into(),
        };
        s.set_tasks("OBY-1", &[t("1", "Yer ishlari"), t("2", "Poydevor")])
            .unwrap();
        let list = s.tasks("OBY-1");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Yer ishlari");
        assert_eq!(list[1].wbs, "2");
        // Muddat va bajarilish ham saqlanadi.
        assert_eq!(list[0].start, "2026-09-01");
        assert_eq!(list[0].progress, 40.0);

        s.set_tasks("OBY-1", &[t("3", "Karkas")]).unwrap();
        let list = s.tasks("OBY-1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Karkas");
        assert!(s.tasks("BOSHQA").is_empty());
    }

    #[test]
    fn log_keeps_the_last_actions() {
        let s = store();
        s.log("a", "kirdi", "", "t1");
        s.log("a", "paket", "OBY-1", "t2");
        let rows = s.recent_log(10);
        assert_eq!(rows.len(), 2);
        // Oxirgisi birinchi turadi.
        assert_eq!(rows[0].1, "paket");
    }
}
