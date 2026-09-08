//! Локальное хранилище (SQLite). Схема писалась с расчетом на будущую
//! синхронизацию с сервером: у каждой записи есть `updated_at`, ключи не переиспользуются.

use crate::domain::{
    Contract, ContractKind, ContractStatus, Estimate, EstimateItem, Inspection, InspectionKind,
    InspectionResult, PaymentStage,
};
use crate::model::*;
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;

pub struct Db {
    conn: Connection,
    /// Amallar tarixiga yoziladigan foydalanuvchi nomi (umumiy talab).
    ///
    /// `Db` foydalanuvchini o'zi bilmaydi — uni ilova o'rnatadi. Bo'sh bo'lsa
    /// jurnalda «tanlanmagan» deb qoladi: kimdir deb o'ylab topmaymiz.
    user: std::sync::Mutex<String>,
    /// Har bir yozuv o'zgarishida oshadigan hisoblagich.
    ///
    /// Interfeys shu son bo'yicha «bazada nimadir o'zgardimi» degan savolga
    /// arzon javob oladi — aks holda hisoblar har kadrda qayta bajarilardi.
    revision: std::sync::atomic::AtomicU64,
}

/// Путь к файлу базы: рядом с исполняемым файлом в подпапке `data`,
/// а если туда писать нельзя — в профиле пользователя.
pub fn default_db_path() -> PathBuf {
    // `QURAI_DB` — sinov va portativ ishlatish uchun aniq yo'l.
    if let Ok(p) = std::env::var("QURAI_DB") {
        let p = PathBuf::from(p);
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        return p;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let data = dir.join("data");
            if std::fs::create_dir_all(&data).is_ok() {
                let probe = data.join(".w");
                if std::fs::write(&probe, b"").is_ok() {
                    let _ = std::fs::remove_file(&probe);
                    return data.join("qurai.db");
                }
            }
        }
    }
    let base = directories::ProjectDirs::from("uz", "QURAi", "QURAi")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&base);
    base.join("qurai.db")
}

impl Db {
    /// Modullar qatlami () uchun ulanishga kirish.
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Amallar tarixi uchun joriy foydalanuvchini o'rnatadi.
    pub fn set_audit_user(&self, name: &str) {
        if let Ok(mut u) = self.user.lock() {
            *u = name.to_string();
        }
    }

    /// Joriy foydalanuvchi nomi. O'rnatilmagan bo'lsa bo'sh satr.
    /// Yozuv o'zgarishlari hisoblagichi.
    pub fn revision(&self) -> u64 {
        self.revision.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Hisoblagichni oshiradi — yozuv chokepointlaridan chaqiriladi.
    pub(crate) fn bump_revision(&self) {
        self.revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(crate) fn audit_user(&self) -> String {
        self.user.lock().map(|u| u.clone()).unwrap_or_default()
    }

    pub fn open(path: &PathBuf) -> SqlResult<Db> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Baza band bo'lsa darhol xato bermay, 5 soniya kutadi.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let db = Db {
            conn,
            user: std::sync::Mutex::new(String::new()),
            revision: std::sync::atomic::AtomicU64::new(0),
        };
        db.migrate()?;
        db.migrate_modules()?;
        db.migrate_norms()?;
        Ok(db)
    }

    fn migrate(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS project (
                id             INTEGER PRIMARY KEY,
                name           TEXT NOT NULL,
                code           TEXT NOT NULL DEFAULT '',
                address        TEXT NOT NULL DEFAULT '',
                status         TEXT NOT NULL DEFAULT 'in_progress',
                start_date     TEXT NOT NULL,
                planned_end    TEXT NOT NULL,
                contract_sum   REAL NOT NULL DEFAULT 0,
                currency       TEXT NOT NULL DEFAULT 'UZS',
                funding_source TEXT NOT NULL DEFAULT '',
                notes          TEXT NOT NULL DEFAULT '',
                updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS party (
                id         INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                role       TEXT NOT NULL,
                name       TEXT NOT NULL DEFAULT '',
                person     TEXT NOT NULL DEFAULT '',
                phone      TEXT NOT NULL DEFAULT '',
                email      TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_party_project ON party(project_id);

            CREATE TABLE IF NOT EXISTS task (
                id          INTEGER PRIMARY KEY,
                project_id  INTEGER NOT NULL REFERENCES project(id) ON DELETE CASCADE,
                wbs         TEXT NOT NULL DEFAULT '',
                name        TEXT NOT NULL,
                section     TEXT NOT NULL DEFAULT '—',
                responsible TEXT NOT NULL DEFAULT '',
                duration    INTEGER NOT NULL DEFAULT 1,
                plan_start  TEXT NOT NULL,
                fact_start  TEXT,
                fact_end    TEXT,
                progress    REAL NOT NULL DEFAULT 0,
                pinned      INTEGER NOT NULL DEFAULT 0,
                volume      REAL NOT NULL DEFAULT 0,
                unit        TEXT NOT NULL DEFAULT '',
                sort_order  INTEGER NOT NULL DEFAULT 0,
                updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_task_project ON task(project_id);

            CREATE TABLE IF NOT EXISTS link (
                id       INTEGER PRIMARY KEY,
                pred     INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
                succ     INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
                kind     TEXT NOT NULL DEFAULT 'FS',
                lag      INTEGER NOT NULL DEFAULT 0,
                UNIQUE(pred, succ)
            );
            CREATE INDEX IF NOT EXISTS idx_link_succ ON link(succ);

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        // Pasport maydonlari keyin qo'shilgan: eski bazada ustun bo'lmasa
        // yaratamiz, bo'lsa ALTER xatosini indamay o'tkazamiz.
        for sql in [
            "ALTER TABLE project ADD COLUMN object_type TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE project ADD COLUMN floors INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE project ADD COLUMN area_total REAL NOT NULL DEFAULT 0",
            "ALTER TABLE project ADD COLUMN paid_total REAL NOT NULL DEFAULT 0",
        ] {
            let _ = self.conn.execute(sql, []);
        }
        Ok(())
    }

    // ---------- Sozlamalar / Настройки ----------

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- Проекты ----------

    pub fn projects(&self) -> SqlResult<Vec<Project>> {
        let mut st = self.conn.prepare(
            "SELECT id, name, code, address, status, start_date, planned_end,
                    contract_sum, currency, funding_source, notes,
                    object_type, floors, area_total, paid_total
             FROM project ORDER BY name",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                code: r.get(2)?,
                address: r.get(3)?,
                status: ObjectStatus::parse(&r.get::<_, String>(4)?),
                start_date: parse_date(&r.get::<_, String>(5)?),
                planned_end: parse_date(&r.get::<_, String>(6)?),
                contract_sum: r.get(7)?,
                currency: r.get(8)?,
                funding_source: r.get(9)?,
                notes: r.get(10)?,
                object_type: r.get(11)?,
                floors: r.get(12)?,
                area_total: r.get(13)?,
                paid_total: r.get(14)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_project(&self, p: &Project) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO project (name, code, address, status, start_date, planned_end,
                                  contract_sum, currency, funding_source, notes,
                                  object_type, floors, area_total, paid_total)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                p.name,
                p.code,
                p.address,
                p.status.as_str(),
                p.start_date.to_string(),
                p.planned_end.to_string(),
                p.contract_sum,
                p.currency,
                p.funding_source,
                p.notes,
                p.object_type,
                p.floors,
                p.area_total,
                p.paid_total
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_project(&self, p: &Project) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE project SET name=?2, code=?3, address=?4, status=?5, start_date=?6,
                    planned_end=?7, contract_sum=?8, currency=?9, funding_source=?10,
                    notes=?11, object_type=?12, floors=?13, area_total=?14, paid_total=?15,
                    updated_at=datetime('now')
             WHERE id=?1",
            params![
                p.id,
                p.name,
                p.code,
                p.address,
                p.status.as_str(),
                p.start_date.to_string(),
                p.planned_end.to_string(),
                p.contract_sum,
                p.currency,
                p.funding_source,
                p.notes,
                p.object_type,
                p.floors,
                p.area_total,
                p.paid_total
            ],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM project WHERE id=?1", [id])?;
        Ok(())
    }

    // ---------- Участники ----------

    pub fn parties(&self, project_id: i64) -> SqlResult<Vec<Party>> {
        let mut st = self.conn.prepare(
            "SELECT id, project_id, role, name, person, phone, email
             FROM party WHERE project_id=?1 ORDER BY id",
        )?;
        let rows = st.query_map([project_id], |r| {
            Ok(Party {
                id: r.get(0)?,
                project_id: r.get(1)?,
                role: PartyRole::parse(&r.get::<_, String>(2)?),
                name: r.get(3)?,
                person: r.get(4)?,
                phone: r.get(5)?,
                email: r.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_party(&self, p: &Party) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO party (project_id, role, name, person, phone, email)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                p.project_id,
                p.role.as_str(),
                p.name,
                p.person,
                p.phone,
                p.email
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_party(&self, p: &Party) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE party SET role=?2, name=?3, person=?4, phone=?5, email=?6,
                    updated_at=datetime('now') WHERE id=?1",
            params![p.id, p.role.as_str(), p.name, p.person, p.phone, p.email],
        )?;
        Ok(())
    }

    pub fn delete_party(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM party WHERE id=?1", [id])?;
        Ok(())
    }

    // ---------- Работы ГПР ----------

    pub fn tasks(&self, project_id: i64) -> SqlResult<Vec<Task>> {
        let mut st = self.conn.prepare(
            "SELECT id, project_id, wbs, name, section, responsible, duration, plan_start,
                    fact_start, fact_end, progress, pinned, volume, unit
             FROM task WHERE project_id=?1 ORDER BY sort_order, id",
        )?;
        let rows = st.query_map([project_id], |r| {
            Ok(Task {
                id: r.get(0)?,
                project_id: r.get(1)?,
                wbs: r.get(2)?,
                name: r.get(3)?,
                section: Section::parse(&r.get::<_, String>(4)?),
                responsible: r.get(5)?,
                duration: r.get(6)?,
                plan_start: parse_date(&r.get::<_, String>(7)?),
                fact_start: r.get::<_, Option<String>>(8)?.map(|s| parse_date(&s)),
                fact_end: r.get::<_, Option<String>>(9)?.map(|s| parse_date(&s)),
                progress: r.get(10)?,
                pinned: r.get::<_, i64>(11)? != 0,
                volume: r.get(12)?,
                unit: r.get(13)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_task(&self, t: &Task) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO task (project_id, wbs, name, section, responsible, duration,
                               plan_start, fact_start, fact_end, progress, pinned, volume, unit,
                               sort_order)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,
                     COALESCE((SELECT MAX(sort_order)+1 FROM task WHERE project_id=?1), 0))",
            params![
                t.project_id,
                t.wbs,
                t.name,
                t.section.code(),
                t.responsible,
                t.duration,
                t.plan_start.to_string(),
                t.fact_start.map(|d| d.to_string()),
                t.fact_end.map(|d| d.to_string()),
                t.progress,
                t.pinned as i64,
                t.volume,
                t.unit
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_task(&self, t: &Task) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE task SET wbs=?2, name=?3, section=?4, responsible=?5, duration=?6,
                    plan_start=?7, fact_start=?8, fact_end=?9, progress=?10, pinned=?11,
                    volume=?12, unit=?13, updated_at=datetime('now')
             WHERE id=?1",
            params![
                t.id,
                t.wbs,
                t.name,
                t.section.code(),
                t.responsible,
                t.duration,
                t.plan_start.to_string(),
                t.fact_start.map(|d| d.to_string()),
                t.fact_end.map(|d| d.to_string()),
                t.progress,
                t.pinned as i64,
                t.volume,
                t.unit
            ],
        )?;
        Ok(())
    }

    /// Ishlar tartibini butunlay qayta yozadi: ro'yxatdagi o'rin — sort_order.
    /// Eski yozuvlarda sort_order teng bo'lib qolgan bo'lishi mumkin, shuning
    /// uchun almashtirish o'rniga to'liq raqamlab chiqamiz.
    pub fn set_task_orders(&self, ids: &[i64]) -> SqlResult<()> {
        self.conn.execute_batch("BEGIN")?;
        for (i, id) in ids.iter().enumerate() {
            self.conn.execute(
                "UPDATE task SET sort_order=?2 WHERE id=?1",
                params![id, i as i64],
            )?;
        }
        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    pub fn delete_task(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM task WHERE id=?1", [id])?;
        Ok(())
    }

    // ---------- Связи ----------

    pub fn links(&self, project_id: i64) -> SqlResult<Vec<Link>> {
        let mut st = self.conn.prepare(
            "SELECT l.id, l.pred, l.succ, l.kind, l.lag FROM link l
             JOIN task t ON t.id = l.succ WHERE t.project_id=?1",
        )?;
        let rows = st.query_map([project_id], |r| {
            Ok(Link {
                id: r.get(0)?,
                pred: r.get(1)?,
                succ: r.get(2)?,
                kind: LinkType::parse(&r.get::<_, String>(3)?),
                lag: r.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_link(&self, l: &Link) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT OR REPLACE INTO link (pred, succ, kind, lag) VALUES (?1,?2,?3,?4)",
            params![l.pred, l.succ, l.kind.short(), l.lag],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_link(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM link WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn project_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM project", [], |r| r.get(0))
    }

    /// Namunaviy obyektlarning shifrlari.
    ///
    /// Tozalash ularni shu ro'yxat bo'yicha topadi: foydalanuvchi o'zi
    /// yaratgan obyektga tegib ketmasligi kerak.
    pub const DEMO_CODES: [&'static str; 2] = ["NVZ-15", "BB-240"];

    /// Namunaviy obyektlarni o'chiradi va ularni qayta yaratishni to'xtatadi.
    ///
    /// Qaytaradi: nechta obyekt o'chirildi. Ilova bo'sh bazada namunani
    /// o'zi yaratadi, shuning uchun sozlamaga bayroq qo'yiladi — aks holda
    /// tozalangan namuna keyingi ochilishda qaytib kelardi.
    pub fn clear_demo(&self) -> SqlResult<usize> {
        let mut n = 0;
        for p in self.projects()? {
            if Self::DEMO_CODES.contains(&p.code.as_str()) {
                self.delete_project(p.id)?;
                n += 1;
            }
        }
        let _ = self.set_setting("demo_cleared", "1");
        Ok(n)
    }

    /// Namuna tozalanganmi — bo'sh bazada uni qayta yaratmaslik uchun.
    pub fn demo_cleared(&self) -> bool {
        self.get_setting("demo_cleared").as_deref() == Some("1")
    }

    /// Namoyish obyekti: bog'lanishlari va kritik yo'li bor GPR ko'rsatadi.
    /// Nomlar joriy tilda yoziladi — bu foydalanuvchi ma'lumoti, tarjima qilinmaydi.
    pub fn seed_demo(&self) -> SqlResult<i64> {
        let ru = crate::i18n::lang() == crate::i18n::Lang::Ru;
        let today = chrono::Local::now().date_naive();
        // Obyekt qurilishning o'rtasida: reja bo'yicha ~74 %, haqiqatda ~69 %.
        // Ilova qanday ishlashini ko'rsatish uchun bir nechta ish kechikkan,
        // lekin loyiha «buzilgan» ko'rinmaydi.
        let start = today - chrono::Duration::days(125);

        let pid = self.insert_project(&Project {
            id: 0,
            name: if ru {
                "ЖК «Навруз», блок №15"
            } else {
                "«Navro'z» TJM, 15-blok"
            }
            .into(),
            code: "NVZ-15".into(),
            address: if ru {
                "г. Ташкент, Яшнабадский район"
            } else {
                "Toshkent sh., Yashnobod tumani"
            }
            .into(),
            object_type: if ru {
                "Жилой дом"
            } else {
                "Turar-joy binosi"
            }
            .into(),
            floors: 9,
            area_total: 18_650.0,
            status: ObjectStatus::InProgress,
            start_date: start,
            planned_end: start + chrono::Duration::days(400),
            contract_sum: 48_500_000_000.0,
            paid_total: 27_300_000_000.0,
            currency: "UZS".into(),
            funding_source: if ru {
                "Собственные средства заказчика + банковский кредит"
            } else {
                "Buyurtmachining o'z mablag'lari + bank krediti"
            }
            .into(),
            notes: if ru {
                "Демонстрационный объект. 9 этажей, монолитный каркас."
            } else {
                "Namoyish obyekti. 9 qavat, monolit karkas."
            }
            .into(),
        })?;

        let parties: [(PartyRole, &str, &str, &str); 6] = [
            (
                PartyRole::Client,
                "MChJ «Navro'z Development»",
                "Karimov A.T.",
                "+998 90 000-00-01",
            ),
            (
                PartyRole::Contractor,
                "MChJ «Mega Qurilish»",
                "Yusupov B.R.",
                "+998 90 000-00-02",
            ),
            (
                PartyRole::Subcontractor,
                "XK «Elektromontaj-S»",
                "Niyozov D.X.",
                "+998 90 000-00-03",
            ),
            (
                PartyRole::Designer,
                "LI «Toshkentboshplanloyiha»",
                "Saidova M.I.",
                "+998 90 000-00-04",
            ),
            (
                PartyRole::TechSupervision,
                "MChJ «StroyKontrol»",
                "Rahimov Sh.A.",
                "+998 90 000-00-05",
            ),
            (
                PartyRole::AuthorSupervision,
                "LI «Toshkentboshplanloyiha»",
                "Saidova M.I.",
                "+998 90 000-00-04",
            ),
        ];
        for (role, name, person, phone) in parties {
            self.insert_party(&Party {
                id: 0,
                project_id: pid,
                role,
                name: name.into(),
                person: person.into(),
                phone: phone.into(),
                email: String::new(),
            })?;
        }

        let contractor = "Mega Qurilish";
        let electro = "Elektromontaj-S";

        // Hajm va birlik smeta tekshiruviga (TZ III.5-III.6) kerak: qoidalar
        // smetadagi miqdorni loyihadagi hajm bilan solishtiradi.
        let defs: Vec<TaskDef> = vec![
            (
                "1",
                "Maydonni tayyorlash",
                "Подготовка площадки",
                Section::None,
                10,
                contractor,
                100.0,
                0.0,
                "",
            ),
            (
                "2",
                "Yer ishlari, kotlovan",
                "Земляные работы, котлован",
                Section::Kj,
                14,
                contractor,
                100.0,
                12500.0,
                "m3",
            ),
            (
                "3",
                "Poydevor plitasini qurish",
                "Устройство фундаментной плиты",
                Section::Kj,
                21,
                contractor,
                100.0,
                1850.0,
                "m3",
            ),
            (
                "4",
                "Poydevor gidroizolyatsiyasi",
                "Гидроизоляция фундамента",
                Section::Ar,
                7,
                contractor,
                100.0,
                2400.0,
                "m2",
            ),
            (
                "5",
                "Monolit karkas, 1-3 qavat",
                "Монолитный каркас, этажи 1-3",
                Section::Kj,
                30,
                contractor,
                100.0,
                2100.0,
                "m3",
            ),
            (
                "6",
                "Monolit karkas, 4-6 qavat",
                "Монолитный каркас, этажи 4-6",
                Section::Kj,
                30,
                contractor,
                100.0,
                2100.0,
                "m3",
            ),
            (
                "7",
                "Monolit karkas, 7-9 qavat",
                "Монолитный каркас, этажи 7-9",
                Section::Kj,
                30,
                contractor,
                45.0,
                2100.0,
                "m3",
            ),
            (
                "8",
                "Tom metall konstruksiyalari",
                "Металлоконструкции кровли",
                Section::Km,
                18,
                contractor,
                15.0,
                46.0,
                "t",
            ),
            (
                "9",
                "Tashqi devorlar g'ishtligi",
                "Кладка наружных стен",
                Section::Ar,
                35,
                contractor,
                100.0,
                3800.0,
                "m2",
            ),
            (
                "10",
                "Ichki devorlar",
                "Внутренние перегородки",
                Section::Ar,
                25,
                contractor,
                100.0,
                5200.0,
                "m2",
            ),
            (
                "11",
                "Suv va kanalizatsiya tarmog'i",
                "Разводка ВК, стояки",
                Section::Vk,
                22,
                electro,
                95.0,
                4200.0,
                "m",
            ),
            (
                "12",
                "Isitish tizimini montaj qilish",
                "Монтаж системы отопления",
                Section::Ov,
                20,
                electro,
                90.0,
                3600.0,
                "m",
            ),
            (
                "13",
                "Ventilyatsiya montaji",
                "Монтаж вентиляции",
                Section::Ov,
                18,
                electro,
                85.0,
                1800.0,
                "m",
            ),
            (
                "14",
                "Elektromontaj ishlari",
                "Электромонтажные работы",
                Section::Eom,
                28,
                electro,
                100.0,
                18500.0,
                "m",
            ),
            (
                "15",
                "Kuchsiz tok tizimlari, SKS",
                "Слаботочные системы, СКС",
                Section::Ss,
                15,
                electro,
                80.0,
                6400.0,
                "m",
            ),
            (
                "16",
                "Yong'in signalizatsiyasi va SOUE",
                "Пожарная сигнализация и СОУЭ",
                Section::Pb,
                14,
                electro,
                65.0,
                5200.0,
                "m",
            ),
            (
                "17",
                "Deraza va vitrajlar",
                "Окна и витражи",
                Section::Ar,
                20,
                contractor,
                60.0,
                1450.0,
                "m2",
            ),
            (
                "18",
                "Pardozlash ishlari",
                "Отделочные работы",
                Section::Ar,
                45,
                contractor,
                0.0,
                24500.0,
                "m2",
            ),
            (
                "19",
                "Hududni obodonlashtirish",
                "Благоустройство территории",
                Section::None,
                20,
                contractor,
                0.0,
                6800.0,
                "m2",
            ),
            (
                "20",
                "Ishga tushirish va topshirish",
                "Пусконаладка и сдача объекта",
                Section::None,
                12,
                contractor,
                0.0,
                0.0,
                "",
            ),
        ];

        let mut ids = Vec::new();
        for (wbs, name_uz, name_ru, section, dur, resp, prog, volume, unit) in defs {
            let id = self.insert_task(&Task {
                id: 0,
                project_id: pid,
                wbs: wbs.into(),
                name: if ru { name_ru } else { name_uz }.into(),
                section,
                responsible: resp.into(),
                duration: dur,
                plan_start: start,
                // Haqiqiy sanalar jurnaldan keladi (TZ V), namoyishda ularni
                // o'ylab topmaymiz — tugallanganini bajarilish foizi ko'rsatadi.
                fact_start: None,
                fact_end: None,
                progress: prog,
                pinned: false,
                volume,
                unit: unit.into(),
            })?;
            ids.push(id);
        }

        // Ro'yxatdagi tartib raqamlari 1 dan boshlanadi.
        let n = |i: usize| ids[i - 1];
        let chain: Vec<(usize, usize, LinkType, i64)> = vec![
            (1, 2, LinkType::Fs, 0),
            (2, 3, LinkType::Fs, 0),
            (3, 4, LinkType::Fs, 0),
            (4, 5, LinkType::Fs, 0),
            (5, 6, LinkType::Fs, 0),
            (6, 7, LinkType::Fs, 0),
            (7, 8, LinkType::Fs, 0),
            (5, 9, LinkType::Ss, 20),
            (9, 10, LinkType::Ss, 15),
            (10, 11, LinkType::Ss, 5),
            (10, 12, LinkType::Ss, 8),
            (10, 13, LinkType::Ss, 10),
            (10, 14, LinkType::Ss, 6),
            (14, 15, LinkType::Ss, 10),
            (14, 16, LinkType::Ss, 12),
            (9, 17, LinkType::Fs, 0),
            (11, 18, LinkType::Fs, 0),
            (12, 18, LinkType::Fs, 0),
            (14, 18, LinkType::Fs, 0),
            (17, 18, LinkType::Fs, 0),
            (8, 18, LinkType::Fs, 0),
            (18, 19, LinkType::Ss, 30),
            (18, 20, LinkType::Fs, 0),
            (16, 20, LinkType::Fs, 0),
            (19, 20, LinkType::Ff, 0),
        ];
        for (p, s, kind, lag) in chain {
            self.insert_link(&Link {
                id: 0,
                pred: n(p),
                succ: n(s),
                kind,
                lag,
            })?;
        }

        // II-III modullar uchun namoyish ma'lumoti: elementlar grafi va smeta.
        self.seed_demo_modules(pid);

        // «Obyektlar» ekrani bitta obyektda o'z vazifasini ko'rsatmaydi, shuning
        // uchun namunada ikkinchi obyekt ham bor: kichikroq, boshqa holatda va
        // ataylab orqada qolgan — konsolidatsiya nima uchun kerakligi ko'rinsin.
        self.seed_second_object(ru, today)?;

        Ok(pid)
    }

    /// Ikkinchi namoyish obyekti: faqat pasport va GPR.
    ///
    /// Bu yerda boshqa modullar to'ldirilmaydi — atayin: portfelda «ma'lumoti
    /// to'liq emas» obyekt qanday ko'rinishini ham ko'rsatish kerak.
    /// Ikkinchi obyektning smetasi, shartnomasi va tekshiruvlari.
    ///
    /// Bu yerda alohida matnlar: umumiy namuna 9 qavatli turar-joy uchun
    /// yozilgan, bu esa ikki qavatli ijtimoiy obyekt. «3-qavat plitasi»
    /// degan yozuv unda xatoday ko'rinardi.
    fn seed_second_details(&self, pid: i64, ru: bool, today: NaiveDate) -> SqlResult<()> {
        let d = |back: i64| today - chrono::Duration::days(back);
        let tasks = self.tasks(pid).unwrap_or_default();
        let by_wbs = |w: &str| tasks.iter().find(|t| t.wbs == w).map(|t| t.id);

        // ---------- Smeta ----------
        let eid = self.insert_estimate(&Estimate {
            id: 0,
            project_id: pid,
            name: if ru {
                "Смета — детский сад на 240 мест".into()
            } else {
                "Smeta — 240 o'rinli bolalar bog'chasi".into()
            },
            currency: "UZS".into(),
            declared_total: 0.0,
            overhead_pct: 14.0,
            profit_pct: 8.0,
            vat_pct: 12.0,
            added_at: String::new(),
        });
        if eid != 0 {
            // (bo'lim, kod, nomi uz/ru, birlik, miqdor, narx, VBS)
            let items: [SecondItemDef; 7] = [
                (
                    Section::None,
                    "E1-1-1",
                    "Maydonni tayyorlash",
                    "Подготовка площадки",
                    "m2",
                    4_180.0,
                    28_000.0,
                    "1",
                ),
                (
                    Section::Kj,
                    "E6-1-1",
                    "Poydevor plitasi, beton B25",
                    "Фундаментная плита, бетон B25",
                    "m3",
                    1_240.0,
                    780_000.0,
                    "2",
                ),
                (
                    Section::Kj,
                    "E6-1-22",
                    "Karkas va devorlar, beton B25",
                    "Каркас и стены, бетон B25",
                    "m3",
                    2_950.0,
                    860_000.0,
                    "3",
                ),
                (
                    Section::Ar,
                    "E12-1-4",
                    "Tom yopish, rulonli",
                    "Кровля рулонная",
                    "m2",
                    2_180.0,
                    195_000.0,
                    "4",
                ),
                (
                    Section::Ov,
                    "E20-2-1",
                    "Muhandislik tarmoqlari",
                    "Инженерные сети",
                    "kompleks",
                    1.0,
                    1_050_000_000.0,
                    "5",
                ),
                (
                    Section::Ar,
                    "E11-1-9",
                    "Ichki pardozlash",
                    "Внутренняя отделка",
                    "m2",
                    6_400.0,
                    168_000.0,
                    "6",
                ),
                (
                    Section::None,
                    "E2-1-8",
                    "Obodonlashtirish",
                    "Благоустройство",
                    "m2",
                    3_100.0,
                    74_000.0,
                    "7",
                ),
            ];
            for (i, (section, code, uz, rux, unit, qty, price, wbs)) in items.iter().enumerate() {
                self.insert_estimate_item(&EstimateItem {
                    id: 0,
                    estimate_id: eid,
                    pos: (i + 1) as i64,
                    section: *section,
                    code: (*code).into(),
                    name: if ru { *rux } else { *uz }.into(),
                    unit: (*unit).into(),
                    qty: *qty,
                    price: *price,
                    cost: qty * price,
                    task_id: by_wbs(wbs),
                    note: String::new(),
                });
            }
        }

        // ---------- Shartnoma va to'lov jadvali ----------
        let cid = self.insert_contract(&Contract {
            id: 0,
            project_id: pid,
            number: "P-2026/22".into(),
            name: if ru {
                "Генподряд: детский сад на 240 мест"
            } else {
                "Bosh pudrat: 240 o'rinli bolalar bog'chasi"
            }
            .into(),
            kind: ContractKind::General,
            party_id: None,
            signed: d(220),
            start: d(210),
            end: d(210) + chrono::Duration::days(300),
            sum: 9_200_000_000.0,
            advance_pct: 15.0,
            retention_pct: 5.0,
            currency: "UZS".into(),
            status: ContractStatus::Active,
            note: String::new(),
        });

        // (raqam, asos uz/ru, muddat kuni, summa, to'langan, to'langan kun)
        let stages: [SecondStageDef; 4] = [
            (
                "T-1",
                "Avans",
                "Аванс",
                205,
                1_380_000_000.0,
                1_380_000_000.0,
                Some(203),
            ),
            (
                "T-2",
                "1-bosqich bajarilgan ish",
                "Работы 1 этапа",
                140,
                900_000_000.0,
                900_000_000.0,
                Some(132),
            ),
            (
                "T-3",
                "2-bosqich bajarilgan ish",
                "Работы 2 этапа",
                62,
                820_000_000.0,
                820_000_000.0,
                Some(44),
            ),
            // Uchinchi to'lov kechikkan — portfelda bu obyekt e'tibor
            // talab qilishining yana bir sababi.
            (
                "T-4",
                "3-bosqich bajarilgan ish",
                "Работы 3 этапа",
                18,
                760_000_000.0,
                0.0,
                None,
            ),
        ];
        for (number, uz, rux, back, amount, paid, paid_back) in stages {
            self.insert_payment_stage(&PaymentStage {
                id: 0,
                project_id: pid,
                contract_id: Some(cid),
                number: number.into(),
                basis: if ru { rux } else { uz }.into(),
                due: d(back),
                amount,
                paid,
                paid_at: paid_back.map(d),
                note: String::new(),
            });
        }

        // ---------- Tekshiruvlar ----------
        // (tur, raqam, reja kuni, o'tkazilgan kuni, joy uz/ru, natija, VBS)
        let checks: [SecondCheckDef; 5] = [
            (
                InspectionKind::Hidden,
                "TN-101",
                168,
                Some(168),
                "Poydevor armaturasi",
                "Арматура фундамента",
                InspectionResult::Pass,
                "2",
            ),
            (
                InspectionKind::Concrete,
                "TN-102",
                160,
                Some(160),
                "Poydevor plitasi, beton quyish",
                "Фундаментная плита, бетонирование",
                InspectionResult::Pass,
                "2",
            ),
            (
                InspectionKind::Hidden,
                "TN-103",
                96,
                Some(95),
                "1-qavat devorlari armaturasi",
                "Арматура стен 1 этажа",
                InspectionResult::Conditional,
                "3",
            ),
            (
                InspectionKind::Physical,
                "TN-104",
                24,
                Some(24),
                "Tom qoplamasi germetikligi",
                "Герметичность кровельного покрытия",
                InspectionResult::Fail,
                "4",
            ),
            (
                InspectionKind::Volume,
                "TN-105",
                -4,
                None,
                "Bajarilgan ish hajmi, 2-qavat",
                "Объём выполненных работ, 2 этаж",
                InspectionResult::Waiting,
                "3",
            ),
        ];
        for (kind, number, plan, done, uz, rux, result, wbs) in checks {
            self.insert_inspection(&Inspection {
                id: 0,
                project_id: pid,
                task_id: by_wbs(wbs),
                kind,
                number: number.into(),
                planned: d(plan),
                done: done.map(d),
                requested_by: if ru {
                    "Рахимов О.С."
                } else {
                    "Rahimov O.S."
                }
                .into(),
                inspector: if done.is_some() {
                    if ru {
                        "Собиров Р.Х.".into()
                    } else {
                        "Sobirov R.X.".to_string()
                    }
                } else {
                    String::new()
                },
                place: if ru { rux } else { uz }.into(),
                result,
                // Salbiy natijaga muddat qo'yiladi, ijobiysiga kerak emas.
                deadline: (result == InspectionResult::Fail).then(|| d(18)),
                fixed_at: (result == InspectionResult::Conditional).then(|| d(88)),
                note: String::new(),
            });
        }

        Ok(())
    }

    fn seed_second_object(&self, ru: bool, today: NaiveDate) -> SqlResult<i64> {
        let start = today - chrono::Duration::days(210);
        let pid = self.insert_project(&Project {
            id: 0,
            name: if ru {
                "Детский сад на 240 мест"
            } else {
                "240 o'rinli bolalar bog'chasi"
            }
            .into(),
            code: "BB-240".into(),
            address: if ru {
                "Ташкентская обл., Зангиатинский р-н"
            } else {
                "Toshkent vil., Zangiota tumani"
            }
            .into(),
            object_type: if ru {
                "Социальный объект"
            } else {
                "Ijtimoiy obyekt"
            }
            .into(),
            floors: 2,
            area_total: 4_180.0,
            status: ObjectStatus::InProgress,
            start_date: start,
            planned_end: start + chrono::Duration::days(300),
            contract_sum: 9_200_000_000.0,
            paid_total: 3_100_000_000.0,
            currency: "UZS".into(),
            funding_source: if ru {
                "Государственный бюджет"
            } else {
                "Davlat byudjeti"
            }
            .into(),
            notes: String::new(),
        })?;

        let plan: [SecondTaskDef; 8] = [
            (
                "1",
                "Tayyorgarlik ishlari",
                "Подготовительные работы",
                Section::None,
                20,
                100.0,
                1.0,
                "kompleks",
            ),
            (
                "2",
                "Kotlovan va poydevor",
                "Котлован и фундамент",
                Section::Kj,
                45,
                100.0,
                1_240.0,
                "m3",
            ),
            (
                "3",
                "Karkas va devorlar",
                "Каркас и стены",
                Section::Kj,
                90,
                82.0,
                2_950.0,
                "m3",
            ),
            (
                "4",
                "Tom yopish",
                "Кровля",
                Section::Ar,
                35,
                40.0,
                2_180.0,
                "m2",
            ),
            (
                "5",
                "Muhandislik tarmoqlari",
                "Инженерные сети",
                Section::Ov,
                60,
                25.0,
                1.0,
                "kompleks",
            ),
            (
                "6",
                "Ichki pardozlash",
                "Внутренняя отделка",
                Section::Ar,
                70,
                8.0,
                6_400.0,
                "m2",
            ),
            (
                "7",
                "Obodonlashtirish",
                "Благоустройство",
                Section::None,
                30,
                0.0,
                3_100.0,
                "m2",
            ),
            (
                "8",
                "Topshirish",
                "Сдача объекта",
                Section::None,
                15,
                0.0,
                1.0,
                "kompleks",
            ),
        ];
        let boss = if ru {
            "Рахимов О.С."
        } else {
            "Rahimov O.S."
        };
        let mut ids: Vec<i64> = Vec::new();
        let mut cursor = start;
        for (wbs, uz, rux, section, days, progress, volume, unit) in plan {
            let id = self.insert_task(&Task {
                id: 0,
                project_id: pid,
                wbs: wbs.into(),
                name: if ru { rux } else { uz }.into(),
                section,
                responsible: boss.into(),
                duration: days,
                plan_start: cursor,
                // Boshlangan ish sanasi bor, tugagani yopilgan.
                fact_start: (progress > 0.0).then_some(cursor),
                fact_end: (progress >= 100.0).then_some(cursor + chrono::Duration::days(days)),
                progress,
                pinned: false,
                volume,
                unit: unit.into(),
            })?;
            cursor += chrono::Duration::days(days);
            ids.push(id);
        }
        // Zanjir: har bir ish oldingisidan keyin.
        for w in ids.windows(2) {
            self.insert_link(&Link {
                id: 0,
                pred: w[0],
                succ: w[1],
                kind: LinkType::Fs,
                lag: 0,
            })?;
        }

        // Ikkinchi obyektda ham ombor, ishchilar va ta'minot bo'lishi kerak:
        // mijoz uni ochganda bo'sh ekranlarni ko'rmasin. Hajmi kichikroq —
        // bu obyekt kichikroq va portfelda «ma'lumoti to'liq emas» emas,
        // «kichikroq» bo'lib turishi kerak.
        self.seed_demo_stock(pid, ru);
        self.seed_demo_resources(pid, ru);
        self.seed_demo_supply(pid, ru);
        self.seed_demo_history(pid, ru);
        self.seed_second_details(pid, ru, today)?;

        Ok(pid)
    }
}

/// Namoyish ishining tavsifi:
/// VBS, nomi (uz), nomi (ru), bo'lim, davomiyligi, mas'ul, bajarilgan %, hajm, birlik.
/// Ikkinchi obyekt smetasining pozitsiyasi: bo'lim, kod, nomi (uz/ru),
/// birlik, miqdor, narx, VBS.
type SecondItemDef = (
    Section,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    f64,
    f64,
    &'static str,
);

/// Ikkinchi obyektning to'lov bosqichi: raqam, asos (uz/ru), muddat (orqaga),
/// summa, to'langan, to'langan kun.
type SecondStageDef = (
    &'static str,
    &'static str,
    &'static str,
    i64,
    f64,
    f64,
    Option<i64>,
);

/// Ikkinchi obyektning tekshiruvi: tur, raqam, reja kuni, o'tkazilgan kuni,
/// joy (uz/ru), natija, VBS.
type SecondCheckDef = (
    InspectionKind,
    &'static str,
    i64,
    Option<i64>,
    &'static str,
    &'static str,
    InspectionResult,
    &'static str,
);

/// Namunaning ikkinchi obyekti uchun ish tavsifi:
/// VBS, nomi (uz), nomi (ru), bo'lim, davomiyligi, bajarilgan %, hajm, birlik.
type SecondTaskDef = (
    &'static str,
    &'static str,
    &'static str,
    Section,
    i64,
    f64,
    f64,
    &'static str,
);

type TaskDef = (
    &'static str,
    &'static str,
    &'static str,
    Section,
    i64,
    &'static str,
    f64,
    f64,
    &'static str,
);

fn parse_date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap_or_else(|_| chrono::Local::now().date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks;
    use crate::domain::{IssueModule, QualityResult, Severity};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    struct TempDb {
        path: PathBuf,
        db: Db,
    }

    impl TempDb {
        fn new() -> TempDb {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("qurai_test_{}_{}.db", std::process::id(), n));
            let _ = std::fs::remove_file(&path);
            let db = Db::open(&path).expect("bazani ochib bo'lmadi");
            TempDb { path, db }
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("db-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("db-shm"));
        }
    }

    /// Sxema yaratiladi, namoyish obyekti to'liq yoziladi va qayta o'qiladi.
    #[test]
    fn seed_demo_round_trips() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().expect("namoyish obyekti");

        // Namunada ikkita obyekt: asosiysi va portfel uchun ikkinchisi.
        assert_eq!(t.db.project_count().unwrap(), 2);
        // Obyekt holati saqlanganidek qaytishi kerak.
        let project =
            t.db.projects()
                .unwrap()
                .into_iter()
                .find(|p| p.id == pid)
                .expect("asosiy obyekt");
        assert_eq!(project.status, ObjectStatus::InProgress);
        assert_eq!(project.floors, 9);
        assert!(project.paid_total > 0.0);
        let tasks = t.db.tasks(pid).unwrap();
        assert_eq!(tasks.len(), 20);
        assert_eq!(t.db.links(pid).unwrap().len(), 25);
        assert_eq!(t.db.parties(pid).unwrap().len(), 6);
        // Hajm va birlik smeta tekshiruvi uchun to'ldirilgan bo'lishi kerak.
        assert!(tasks.iter().filter(|x| x.volume > 0.0).count() >= 15);
        assert!(!t.db.elements(pid).is_empty());
        assert!(!t.db.element_links(pid).is_empty());
        // Namunada ikki smeta varianti bor — ularni solishtirish uchun.
        assert_eq!(t.db.estimates(pid).len(), 2);
    }

    /// Namoyish ma'lumotida ikkala tekshiruv ham ishlaydi va natija saqlanadi.
    #[test]
    fn checks_run_over_demo_data() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let tasks = t.db.tasks(pid).unwrap();
        let elements = t.db.elements(pid);
        let el_links = t.db.element_links(pid);
        let estimate = t.db.estimates(pid).remove(0);
        let items = t.db.estimate_items(estimate.id);
        let norms = t.db.norms();
        assert!(!items.is_empty());

        let ctx = checks::Ctx {
            project_id: pid,
            tasks: &tasks,
            elements: &elements,
            links: &el_links,
            items: &items,
            declared_total: estimate.declared_total,
            norms: &norms,
            prices: &[],
        };

        let project_issues = checks::check_project(&ctx);
        let estimate_issues = checks::check_estimate(&ctx);
        assert!(
            project_issues
                .iter()
                .any(|i| i.severity == Severity::Critical),
            "namoyish loyihasida kritik nomuvofiqlik ko'zda tutilgan"
        );
        assert!(
            estimate_issues
                .iter()
                .any(|i| i.severity == Severity::Critical),
            "namoyish smetasida kritik nomuvofiqlik ko'zda tutilgan"
        );

        t.db.replace_auto_issues(pid, IssueModule::Project, &project_issues);
        t.db.replace_auto_issues(pid, IssueModule::Estimate, &estimate_issues);
        // Namunada qo'lda kiritilgan nomuvofiqliklar ham bor — avtomatik
        // tekshiruv faqat o'z yozuvlarini almashtiradi.
        let auto: Vec<_> = t.db.issues(pid).into_iter().filter(|i| i.auto).collect();
        assert_eq!(auto.len(), project_issues.len() + estimate_issues.len());
    }

    /// Foydalanuvchi yopgan nomuvofiqlik qayta tekshiruvda tirilib qolmasin.
    #[test]
    fn closed_issue_is_not_resurrected() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let tasks = t.db.tasks(pid).unwrap();
        let elements = t.db.elements(pid);
        let el_links = t.db.element_links(pid);
        let norms = t.db.norms();
        let ctx = checks::Ctx {
            project_id: pid,
            tasks: &tasks,
            elements: &elements,
            links: &el_links,
            items: &[],
            declared_total: 0.0,
            norms: &norms,
            prices: &[],
        };
        let found = checks::check_project(&ctx);
        t.db.replace_auto_issues(pid, IssueModule::Project, &found);

        let mut first = t.db.issues(pid).remove(0);
        let closed_code = first.code.clone();
        first.status = crate::domain::IssueStatus::Fixed;
        assert!(t.db.update_issue(&first));

        t.db.replace_auto_issues(pid, IssueModule::Project, &found);
        let after = t.db.issues(pid);
        let same: Vec<_> = after.iter().filter(|i| i.code == closed_code).collect();
        assert_eq!(same.len(), 1, "yopilgan yozuv takrorlanmasligi kerak");
        assert_eq!(same[0].status, crate::domain::IssueStatus::Fixed);
    }

    /// TZ V: jurnaldagi haqiqiy hajm GPR dagi bajarilish foiziga aylanadi.
    #[test]
    fn journal_volume_becomes_task_progress() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        // App o'z ulanishini egallaydi, shuning uchun ikkinchisini ochamiz.
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let task = app.tasks.iter().find(|x| x.wbs == "7").cloned().unwrap();
        assert!(task.volume > 0.0, "namoyish ishida hajm bo'lishi kerak");

        // Kutilgan foiz jurnalning o'zidan hisoblanadi: namuna tarixi
        // o'zgarsa ham sinov o'z ma'nosini yo'qotmasin.
        let logged: f64 = app
            .journal
            .iter()
            .filter(|j| j.task_id == Some(task.id))
            .map(|j| j.volume)
            .sum();
        assert!(logged > 0.0, "jurnalda shu ish bo'yicha yozuv yo'q");

        app.apply_journal_to_tasks();

        let expected = (logged / task.volume * 100.0).min(100.0);
        let after = app.tasks.iter().find(|x| x.id == task.id).unwrap();
        assert!(
            (after.progress - expected).abs() < 0.01,
            "{} != {expected}",
            after.progress
        );
        assert!(
            after.fact_start.is_some(),
            "boshlanish sanasi jurnaldan olinadi"
        );

        // Qayta o'qilganda ham saqlanib qolishi kerak.
        let reread = t.db.tasks(pid).unwrap();
        let saved = reread.iter().find(|x| x.id == task.id).unwrap();
        assert!((saved.progress - expected).abs() < 0.01);
    }

    /// PPR tekshiruvi namoyish ma'lumotida ishlaydi va natijani saqlaydi.
    #[test]
    fn ppr_check_runs_over_demo_data() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        assert!(!app.ppr_docs.is_empty());
        app.run_ppr_check();

        let issues = t.db.issues(pid);
        // Namoyishda tasdiqlanmagan karta bo'yicha ish ketmoqda.
        assert!(issues
            .iter()
            .any(|i| i.module == IssueModule::Ppr && i.severity == Severity::Critical));
        // Ishga bog'lanmagan xavfsizlik kartasi ham bor.
        assert!(issues
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_ppr_orphan_title")));
    }

    /// Ilova qayta ochilganda kritik yo'l o'zgarib ketmasligi kerak.
    /// `recompute` CPM natijasini `plan_start` ga yozadi — agar u keyingi
    /// hisobga ta'sir qilsa, grafik har ochilishda «suzib» ketardi.
    #[test]
    fn schedule_is_stable_across_restarts() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let mut first = crate::app::App::new(Db::open(&t.path).unwrap());
        first.select_project(pid);
        let crit1 = first
            .tasks
            .iter()
            .filter(|x| first.schedule.is_critical(x.id))
            .count();
        let days1 = first.schedule.project_days;
        drop(first);

        let mut second = crate::app::App::new(Db::open(&t.path).unwrap());
        second.select_project(pid);
        let crit2 = second
            .tasks
            .iter()
            .filter(|x| second.schedule.is_critical(x.id))
            .count();
        let days2 = second.schedule.project_days;

        assert_eq!(days1, days2, "loyiha davomiyligi o'zgarmasligi kerak");
        assert_eq!(
            crit1, crit2,
            "kritik yo'ldagi ishlar soni o'zgarmasligi kerak"
        );
        assert!(
            crit1 > 1,
            "namoyish grafigida kritik yo'l bir nechta ishdan iborat"
        );
    }

    /// Bo'sh obyekt — foydalanuvchi «+ Yangi obyekt» bosgandagi holat.
    /// Barcha hisoblar nolga bo'lish yoki panikaga olib kelmasligi kerak.
    #[test]
    fn empty_project_survives_every_calculation() {
        let t = TempDb::new();
        let today = chrono::Local::now().date_naive();
        let pid =
            t.db.insert_project(&Project {
                id: 0,
                name: "Bo'sh".into(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: ObjectStatus::Design,
                start_date: today,
                // Ataylab: tugash sanasi boshlanish bilan bir xil — 0 kunlik oraliq.
                planned_end: today,
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: String::new(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();

        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Hisoblar nol ma'lumotda ham xavfsiz ishlashi kerak.
        assert!(app.tasks.is_empty());
        assert_eq!(app.progress.plan_pct, 0.0);
        assert_eq!(app.progress.fact_pct, 0.0);
        assert!(app.progress.overdue.is_empty());

        // Tekshiruvlar ma'lumotsiz ishga tushirilса — xabar berib to'xtaydi.
        app.run_project_check();
        app.run_estimate_check();
        app.run_ppr_check();
        assert!(
            app.issues.is_empty(),
            "ma'lumotsiz nomuvofiqlik yaratilmasin"
        );

        // Pul xulosasi ham nolga bo'linmasin.
        let cost = app.cost_summary();
        assert_eq!(cost.total, 0.0);
        assert_eq!(cost.duplicate_count, 0);

        // Resurs talabi bo'sh grafikda ham qaytishi kerak.
        let d = crate::checks::resource_demand(&app.tasks, &app.schedule, &app.ppr_docs, false);
        assert!(d.iter().all(|v| *v == 0));

        // Jurnalni GPR ga o'tkazish ham xavfsiz.
        app.apply_journal_to_tasks();

        // Ijro hujjatlari talabi bo'sh.
        assert!(crate::checks::required_docs(&app.tasks, &app.exec_docs, true).is_empty());
    }

    /// Bitta ishли loyiha: eng kichik haqiqий holat.
    #[test]
    fn single_task_project_computes() {
        let t = TempDb::new();
        let today = chrono::Local::now().date_naive();
        let pid =
            t.db.insert_project(&Project {
                id: 0,
                name: "Bitta ish".into(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: ObjectStatus::InProgress,
                start_date: today - chrono::Duration::days(5),
                planned_end: today + chrono::Duration::days(5),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: "UZS".into(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();
        t.db.insert_task(&Task {
            id: 0,
            project_id: pid,
            wbs: "1".into(),
            name: "Yagona ish".into(),
            section: Section::None,
            responsible: String::new(),
            duration: 10,
            plan_start: today - chrono::Duration::days(5),
            fact_start: None,
            fact_end: None,
            progress: 50.0,
            pinned: false,
            volume: 0.0,
            unit: String::new(),
        })
        .unwrap();

        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.schedule.project_days, 10);
        // 10 kunlik ishning 6-kuni: reja 60 %, fakt 50 %.
        assert!((app.progress.plan_pct - 60.0).abs() < 0.01);
        assert!((app.progress.fact_pct - 50.0).abs() < 0.01);
        // Yagona ish kritik yo'lda bo'lishi kerak.
        assert!(app.schedule.is_critical(app.tasks[0].id));
    }

    /// Eski versiyada yaratilgan baza yangi kod bilan ochilishi kerak.
    /// Ustunlar keyin qo'shilgan — ular ALTER bilan yaratiladi va mavjud
    /// yozuvlar yo'qolmaydi.
    #[test]
    fn old_database_migrates_without_data_loss() {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("qurai_old_{}_{}.db", std::process::id(), n));
        let _ = std::fs::remove_file(&path);

        // Eski sxema: yangi ustunlarsiz.
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            c.execute_batch(
                r#"
                CREATE TABLE project (
                    id INTEGER PRIMARY KEY, name TEXT NOT NULL, code TEXT NOT NULL DEFAULT '',
                    address TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'in_progress',
                    start_date TEXT NOT NULL, planned_end TEXT NOT NULL,
                    contract_sum REAL NOT NULL DEFAULT 0, currency TEXT NOT NULL DEFAULT 'UZS',
                    funding_source TEXT NOT NULL DEFAULT '', notes TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                INSERT INTO project (name, code, start_date, planned_end, contract_sum)
                VALUES ('Eski obyekt', 'OLD-1', '2026-01-01', '2026-12-31', 1000000);
                "#,
            )
            .unwrap();
        }

        // Yangi kod bilan ochamiz — migratsiya o'tishi kerak.
        let db = Db::open(&path).expect("eski baza ochilmadi");
        let projects = db.projects().expect("loyihalarni o'qib bo'lmadi");
        assert_eq!(projects.len(), 1, "eski yozuv yo'qolmasligi kerak");
        assert_eq!(projects[0].name, "Eski obyekt");
        assert_eq!(projects[0].contract_sum, 1_000_000.0);
        // Yangi ustunlar standart qiymat bilan to'ldiriladi.
        assert_eq!(projects[0].floors, 0);
        assert_eq!(projects[0].paid_total, 0.0);
        assert_eq!(projects[0].object_type, "");

        // Yozish ham ishlashi kerak.
        let mut p = projects[0].clone();
        p.floors = 12;
        p.paid_total = 500_000.0;
        db.update_project(&p).expect("yangilash");
        assert_eq!(db.projects().unwrap()[0].floors, 12);

        // Ikkinchi marta ochilganda ALTER qayta urinmaydi va xato bermaydi.
        drop(db);
        let db2 = Db::open(&path).expect("qayta ochilmadi");
        assert_eq!(db2.projects().unwrap()[0].floors, 12);
        drop(db2);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    /// Tekshiruv ekrani ochilganda natija avtomatik hisoblanadi, lekin
    /// foydalanuvchi holatini buzmaydi va qayta-qayta ishlamaydi.
    #[test]
    fn auto_check_runs_once_and_respects_saved_state() {
        use crate::domain::{IssueModule, IssueStatus};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Boshida avtomatik natija yo'q: namunadagi yozuvlar qo'lda kiritilgan.
        assert!(!app.issues.iter().any(|i| i.auto));

        // Birinchi ochilish — natija hisoblanadi.
        app.auto_check(IssueModule::Project);
        let first = app
            .issues
            .iter()
            .filter(|i| i.module == IssueModule::Project)
            .count();
        assert!(first > 0, "avtomatik tekshiruv natija bermadi");

        // Foydalanuvchi bitta yozuvni yopadi.
        let mut one = app
            .issues
            .iter()
            .find(|i| i.module == IssueModule::Project)
            .cloned()
            .unwrap();
        one.status = IssueStatus::Fixed;
        let code = one.code.clone();
        app.save_issue(one);

        // Ekran qayta ochilganda tekshiruv qaytadan ishlamaydi —
        // yopilgan yozuv o'z holida qoladi.
        app.auto_check(IssueModule::Project);
        let still = app
            .issues
            .iter()
            .find(|i| i.code == code)
            .map(|i| i.status)
            .unwrap();
        assert_eq!(still, IssueStatus::Fixed, "yopilgan yozuv qayta ochilmasin");

        // Boshqa modul o'z navbatida ishlaydi.
        app.auto_check(IssueModule::Estimate);
        assert!(app.issues.iter().any(|i| i.module == IssueModule::Estimate));
    }

    /// Tekshiradigan ma'lumot bo'lmasa, avtomatik ishga tushirish tegmaydi.
    #[test]
    fn auto_check_skips_when_there_is_nothing_to_check() {
        use crate::domain::IssueModule;

        let t = TempDb::new();
        let today = chrono::Local::now().date_naive();
        let pid =
            t.db.insert_project(&Project {
                id: 0,
                name: "Bo'sh".into(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: ObjectStatus::Design,
                start_date: today,
                planned_end: today + chrono::Duration::days(30),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: String::new(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        app.auto_check(IssueModule::Project);
        app.auto_check(IssueModule::Estimate);
        app.auto_check(IssueModule::Ppr);
        assert!(app.issues.is_empty());
    }

    /// Tezlikni o'lchash: `cargo test -- --ignored --nocapture perf_typing`.
    ///
    /// Matn maydoniga yozish har bosilgan harfda `update_project` + `recompute`
    /// chaqiradi. Katta loyihada bu qanchaga tushishini o'lchaymiz.
    #[test]
    #[ignore = "diagnostika: tezlikni o'lchaydi"]
    fn perf_typing() {
        for task_count in [20usize, 200, 500] {
            let t = TempDb::new();
            let today = chrono::Local::now().date_naive();
            let pid =
                t.db.insert_project(&Project {
                    id: 0,
                    name: "Tezlik".into(),
                    code: String::new(),
                    address: String::new(),
                    object_type: String::new(),
                    floors: 0,
                    area_total: 0.0,
                    status: ObjectStatus::InProgress,
                    start_date: today,
                    planned_end: today + chrono::Duration::days(900),
                    contract_sum: 0.0,
                    paid_total: 0.0,
                    currency: "UZS".into(),
                    funding_source: String::new(),
                    notes: String::new(),
                })
                .unwrap();

            // Zanjir ko'rinishidagi ishlar — CPM uchun eng og'ir holat.
            let mut prev = 0i64;
            for i in 0..task_count {
                let id =
                    t.db.insert_task(&Task {
                        id: 0,
                        project_id: pid,
                        wbs: (i + 1).to_string(),
                        name: format!("Ish {i}"),
                        section: Section::Kj,
                        responsible: String::new(),
                        duration: 5,
                        plan_start: today,
                        fact_start: None,
                        fact_end: None,
                        progress: 0.0,
                        pinned: false,
                        volume: 0.0,
                        unit: String::new(),
                    })
                    .unwrap();
                if prev > 0 {
                    t.db.insert_link(&Link {
                        id: 0,
                        pred: prev,
                        succ: id,
                        kind: LinkType::Fs,
                        lag: 0,
                    })
                    .unwrap();
                }
                prev = id;
            }

            let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
            app.select_project(pid);

            // 1. Bitta harf yozish: pasport ekranidagi yo'l.
            let start = std::time::Instant::now();
            for i in 0..20 {
                let mut p = app.project().cloned().unwrap();
                p.name = format!("Tezlik {i}");
                let _ = app.db.update_project(&p);
                if let Some(slot) = app.projects.iter_mut().find(|x| x.id == p.id) {
                    *slot = p;
                }
                app.recompute();
            }
            let per_key = start.elapsed().as_secs_f64() * 1000.0 / 20.0;

            // 2. Faqat CPM hisobi (bazasiz).
            let start = std::time::Instant::now();
            for _ in 0..20 {
                let _ = crate::cpm::compute(&app.tasks, &app.links, app.origin());
            }
            let cpm_ms = start.elapsed().as_secs_f64() * 1000.0 / 20.0;

            // 3. Faqat bazaga bitta yozuv.
            let p = app.project().cloned().unwrap();
            let start = std::time::Instant::now();
            for _ in 0..20 {
                let _ = app.db.update_project(&p);
            }
            let write_ms = start.elapsed().as_secs_f64() * 1000.0 / 20.0;

            println!(
                "{task_count:>4} ta ish | bitta harf: {per_key:>7.2} ms | CPM: {cpm_ms:>6.2} ms | baza yozuv: {write_ms:>6.2} ms"
            );
        }
    }

    /// Namoyish ma'lumotidagi tekshiruv natijasini ko'rish uchun diagnostika.
    /// `cargo test -- --ignored --nocapture demo_report` bilan ishga tushiriladi.
    #[test]
    #[ignore = "diagnostika: natijani ekranga chiqaradi"]
    fn demo_report() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let tasks = t.db.tasks(pid).unwrap();
        let elements = t.db.elements(pid);
        let el_links = t.db.element_links(pid);
        let estimate = t.db.estimates(pid).remove(0);
        let items = t.db.estimate_items(estimate.id);
        let norms = t.db.norms();
        let ctx = checks::Ctx {
            project_id: pid,
            tasks: &tasks,
            elements: &elements,
            links: &el_links,
            items: &items,
            declared_total: estimate.declared_total,
            norms: &norms,
            prices: &[],
        };
        for (title, list) in [
            ("LOYIHA", checks::check_project(&ctx)),
            ("SMETA", checks::check_estimate(&ctx)),
        ] {
            println!(
                "
=== {title}: {} ta ===",
                list.len()
            );
            for i in &list {
                println!(
                    "{:<10} {:<10} {:<4} {:<40} {}",
                    i.code,
                    i.severity.code(),
                    i.section.code(),
                    i.title,
                    i.element
                );
            }
        }
    }

    /// Obyekt o'chirilganda bog'liq ma'lumot ham ketadi (FOREIGN KEY CASCADE).
    #[test]
    fn delete_project_cascades() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        t.db.delete_project(pid).unwrap();

        // Namunadagi ikkinchi obyekt joyida qoladi — o'chirish faqat bittasiga
        // tegishi kerak.
        assert_eq!(t.db.project_count().unwrap(), 1);
        assert!(t.db.tasks(pid).unwrap().is_empty());
        assert!(t.db.elements(pid).is_empty());
        assert!(t.db.estimates(pid).is_empty());
        assert!(t.db.issues(pid).is_empty());
    }

    /// Qurilmalar orasida almashish: paket chiqarilib, boshqa bazaga tushadi.
    #[test]
    fn package_moves_field_work_between_databases() {
        // Maydondagi baza: namuna ma'lumot bilan.
        let field = TempDb::new();
        let pid = field.db.seed_demo().unwrap();
        let mut site = crate::app::App::new(Db::open(&field.path).unwrap());
        site.select_project(pid);

        let pkg = site.export_package();
        assert!(pkg.row_count() > 0, "paket bo'sh");
        assert!(pkg.table("journal").is_some());
        assert!(pkg.table("timesheet").is_some());
        // Grafik va smeta ataylab chiqmaydi.
        assert!(pkg.table("task").is_none());
        assert!(pkg.table("estimate").is_none());

        // Fayl orqali ko'chiramiz.
        let text = crate::package::write(&pkg);
        let back = crate::package::read(&text).expect("o'qildi");

        // Ofisdagi baza: o'sha obyekt bor, lekin kunlik ijro yo'q.
        let office = TempDb::new();
        let opid = office
            .db
            .insert_project(&crate::model::Project {
                id: 0,
                name: "Ofis nusxasi".into(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: crate::model::ObjectStatus::InProgress,
                start_date: chrono::Local::now().date_naive(),
                planned_end: chrono::Local::now().date_naive(),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: "UZS".into(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();
        let mut hq = crate::app::App::new(Db::open(&office.path).unwrap());
        hq.select_project(opid);
        assert!(hq.journal.is_empty());

        let (added, existing) = hq.import_package(&back);
        assert!(added > 0, "hech narsa qo'shilmadi");
        assert_eq!(existing, 0);
        assert!(!hq.journal.is_empty(), "jurnal ko'chmadi");
        assert!(!hq.timesheet.is_empty(), "tabel ko'chmadi");
        // Ishchilar paketda nom bilan kelgan — ular yaratildi.
        assert!(!hq.workers.is_empty());

        // Ikkinchi import dublikat yaratmaydi.
        let before = hq.journal.len();
        let (added2, existing2) = hq.import_package(&back);
        assert_eq!(added2, 0, "takroriy import dublikat berdi");
        assert!(existing2 > 0);
        assert_eq!(hq.journal.len(), before);
    }

    /// TZ XIII.4: telefondan kelgan kirish-chiqish belgisi ilovaga tushadi
    /// va tabel bilan solishtiriladi.
    #[test]
    fn a_mark_from_the_phone_lands_in_the_app() {
        let field = TempDb::new();
        let pid = field.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&field.path).unwrap());
        app.select_project(pid);
        let who = app.workers.first().expect("ishchi").name.clone();
        assert!(app.attendance.is_empty());

        // Server yuboradigan paketning aynan o'zi.
        let text = format!(
            "QURAI-PACKAGE\t1\nPROJECT\tOBY\n\n#attendance\n\
worker\tat\tkind\tgps\tsource\n\
{who}\t2026-09-08T08:00:00\tin\t41.299500,69.240100,12\tqr\n\
{who}\t2026-09-08T17:00:00\tout\t41.299500,69.240100,12\tlist\n"
        );
        let pkg = crate::package::read(&text).expect("o'qildi");
        let (added, _) = app.import_package(&pkg);
        assert_eq!(added, 2, "belgilar qo'shilmadi");
        app.reload_modules();
        assert_eq!(app.attendance.len(), 2);

        // Kun juftlanadi: 9 soat.
        let days = app.attendance_days();
        assert_eq!(days.len(), 1);
        assert!((days[0].hours - 9.0).abs() < 1e-9, "{}", days[0].hours);
        assert!(days[0].by_qr, "QR bilan qo'yilgani ko'rinishi kerak");
        assert!(!days[0].open);

        // Ikkinchi import dublikat bermaydi.
        let (added2, existing2) = app.import_package(&pkg);
        assert_eq!(added2, 0);
        assert!(existing2 > 0);
        app.reload_modules();
        assert_eq!(app.attendance.len(), 2);
    }

    /// TZ XIII.5: geozona kiritilmaguncha joy bo'yicha hukm chiqmaydi.
    #[test]
    fn without_a_fence_there_is_no_verdict_about_place() {
        let field = TempDb::new();
        let pid = field.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&field.path).unwrap());
        app.select_project(pid);
        let who = app.workers.first().expect("ishchi").name.clone();

        app.db.insert_attendance(&crate::domain::Attendance {
            id: 0,
            project_id: pid,
            worker: who,
            at: chrono::NaiveDate::from_ymd_opt(2026, 9, 8)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap(),
            kind: crate::domain::InOut::In,
            // Obyektdan uzoqda.
            gps: "41.330000,69.240100,10".into(),
            source: "list".into(),
        });
        app.reload_modules();

        assert!(app.fence().is_none());
        assert_eq!(
            app.attendance_days()[0].verdict,
            crate::geo::Verdict::Unknown,
            "geozonasiz hukm chiqmasligi kerak"
        );

        // Geozona kiritilgach — chetlanish ko'rinadi va saqlanadi.
        app.set_fence(Some(crate::geo::Fence {
            center: crate::geo::Point::new(41.2995, 69.2401),
            radius: 100.0,
        }));
        let fence = app.fence().expect("saqlanishi kerak");
        assert!((fence.radius - 100.0).abs() < 1e-9);
        assert!(app.attendance_days()[0].verdict.outside());
        assert!(app
            .attendance_mismatch()
            .iter()
            .any(|m| matches!(m, crate::attend::Mismatch::Outside { .. })));

        // O'chirilsa yana hukm yo'q.
        app.set_fence(None);
        assert!(app.fence().is_none());
    }

    /// TZ V.13: telefondan kelgan koordinata jurnal yozuvida saqlanadi.
    #[test]
    fn a_journal_entry_from_the_phone_keeps_its_place() {
        let field = TempDb::new();
        let pid = field.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&field.path).unwrap());
        app.select_project(pid);

        let text = "QURAI-PACKAGE\t1\nPROJECT\tOBY\n\n#journal\n\
date\tauthor\ttext\tgps\n\
2026-09-08\tAlisher\tBeton quyildi\t41.299500,69.240100,12\n";
        let pkg = crate::package::read(text).expect("o'qildi");
        let (added, _) = app.import_package(&pkg);
        assert_eq!(added, 1);
        app.reload_modules();

        let entry = app
            .journal
            .iter()
            .find(|j| j.text == "Beton quyildi")
            .expect("yozuv");
        let point = crate::geo::parse(&entry.gps).expect("koordinata");
        assert!((point.lat - 41.2995).abs() < 1e-6);
        assert_eq!(point.accuracy, 12.0);

        // Ilovaning o'z paketiga ham koordinata bilan chiqadi.
        let out = app.export_package();
        let table = out.table("journal").expect("jurnal");
        assert!(
            table.columns.iter().any(|c| c == "gps"),
            "paketda gps ustuni yo'q"
        );
    }

    /// TZ IV.18, V.28: imzo daftariga yozuv tushadi va hujjat keyin
    /// o'zgartirilsa bu ko'rinadi.
    #[test]
    fn signing_records_the_document_and_notices_later_edits() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        assert!(app.sign_log.is_empty());

        let doc = app.exec_docs.first().cloned().expect("hujjat");
        let text = app.document_text(&doc);
        app.sign_document(&doc.number, &doc.name, &text, "");
        assert_eq!(app.sign_log.len(), 1);
        assert!(app.sign_breaks().is_empty(), "toza daftarda e'tiroz yo'q");

        // Ikkinchi imzo zanjirga ulanadi.
        app.sign_document(&doc.number, &doc.name, &text, "rad: hajm mos emas");
        assert_eq!(app.sign_log.len(), 2);
        assert!(app.sign_breaks().is_empty());
        assert_ne!(app.sign_log[0].chain, app.sign_log[1].chain);

        // Hujjat imzodan keyin o'zgartirildi — bu ko'rinadi.
        let mut edited = doc.clone();
        edited.name = format!("{} (tuzatilgan)", doc.name);
        assert!(app.db.update_exec_doc(&edited));
        app.reload_modules();
        let breaks = app.sign_breaks();
        assert!(
            breaks
                .iter()
                .any(|b| matches!(b, crate::signlog::Break::Text { .. })),
            "{breaks:?}"
        );
    }

    /// Serverdagi belgi bilan solishtirish: yozuv o'chirilsa aytiladi.
    #[test]
    fn a_server_mark_catches_a_deleted_record() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        let doc = app.exec_docs.first().cloned().expect("hujjat");
        let text = app.document_text(&doc);
        for _ in 0..3 {
            app.sign_document(&doc.number, &doc.name, &text, "");
        }
        let (count, head) = crate::signlog::head(&app.sign_log);
        assert_eq!(count, 3);

        // Server o'sha paytdagi uchni eslab qolgan.
        app.chain_marks = vec![crate::sync::ChainMark {
            count: 3,
            head: head.clone(),
            at: String::new(),
        }];
        assert!(app.chain_alerts().is_empty());

        // Kimdir bazadan bitta yozuvni o'chirdi.
        let id = app.sign_log[1].id;
        app.db
            .conn()
            .execute("DELETE FROM sign_log WHERE id=?1", [id])
            .expect("o'chirish");
        app.reload_modules();
        assert_eq!(app.sign_log.len(), 2);
        assert!(
            !app.chain_alerts().is_empty(),
            "serverdagi belgi bilan farq sezilmadi"
        );
    }

    /// TZ VI-VIII: rol yozuvchi amallarni to'sadi, ko'rishga xalaqit bermaydi.
    #[test]
    fn role_blocks_writes_but_not_reads() {
        use crate::app::Screen;
        use crate::roles::{Role, User};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let client = t.db.insert_user(&User {
            id: 0,
            name: "Buyurtmachi".into(),
            role: Role::Client,
            note: String::new(),
        });
        let foreman = t.db.insert_user(&User {
            id: 0,
            name: "Prorab".into(),
            role: Role::Foreman,
            note: String::new(),
        });

        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        let task = app.tasks.first().cloned().expect("ish");

        // Buyurtmachi: ko'radi, lekin o'zgartira olmaydi.
        app.set_user(Some(client));
        assert_eq!(app.role(), Role::Client);
        assert!(!app.tasks.is_empty(), "ma'lumot ko'rinishda qolishi kerak");
        app.screen = Screen::Gantt;
        let mut edited = task.clone();
        edited.progress = 5.0;
        app.save_task(edited);
        let after = t.db.tasks(pid).unwrap();
        let saved = after.iter().find(|x| x.id == task.id).unwrap();
        assert_eq!(
            saved.progress, task.progress,
            "buyurtmachi ishni o'zgartirdi"
        );

        // Prorab: grafikda ishlay oladi.
        app.set_user(Some(foreman));
        app.screen = Screen::Gantt;
        let mut edited = task.clone();
        edited.progress = 42.0;
        app.save_task(edited);
        let after = t.db.tasks(pid).unwrap();
        let saved = after.iter().find(|x| x.id == task.id).unwrap();
        assert_eq!(saved.progress, 42.0, "prorab ishni o'zgartira olmadi");

        // Ammo smetani emas.
        assert!(!app.can_edit(Screen::Estimate));

        // Rol tanlanmagan bo'lsa — cheklov yo'q.
        app.set_user(None);
        assert_eq!(app.role(), Role::Admin);
        assert!(app.can_edit(Screen::Estimate));
    }

    /// TZ II.1-2: IFC fayli o'qilib, elementlar va bog'lanishlar bazaga tushadi.
    #[test]
    fn ifc_import_fills_the_graph() {
        use crate::domain::Relation;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let ifc = "ISO-10303-21;\nHEADER;\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
#5= IFCBUILDINGSTOREY('s1',$,'3-qavat',$,$,$,$,$,.ELEMENT.,0.);\n\
#10= IFCWALLSTANDARDCASE('w1',$,'Devor',$,$,$,$,'IFC-D1');\n\
#11= IFCOPENINGELEMENT('o1',$,'Teshik',$,$,$,$,'IFC-PR1');\n\
#12= IFCWINDOW('n1',$,'Deraza',$,$,$,$,'IFC-OK1');\n\
#13= IFCDUCTSEGMENT('d1',$,'Vozduxovod',$,$,$,$,'IFC-V1');\n\
#20= IFCRELCONTAINEDINSPATIALSTRUCTURE('r1',$,$,$,(#10,#13),#5);\n\
#21= IFCRELVOIDSELEMENT('r2',$,$,$,#10,#11);\n\
#22= IFCRELFILLSELEMENT('r3',$,$,$,#11,#12);\n\
ENDSEC;\nEND-ISO-10303-21;\n";

        let dir = std::env::temp_dir().join(format!("qurai_ifc_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("demo.ifc");
        std::fs::write(&path, ifc).unwrap();

        // Ilova o'z ulanishi bilan ishlaydi — xuddi haqiqiy ishga tushirishdagidek.
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        let before = app.elements.len();
        let links_before = app.element_links.len();

        app.import_ifc(&path);

        let added: Vec<&crate::domain::Element> =
            app.elements.iter().filter(|e| e.sheet == "IFC").collect();
        assert_eq!(added.len(), 4, "IFC dan 4 element kutilgan");
        assert!(app.elements.len() > before);

        let by_mark = |m: &str| {
            app.elements
                .iter()
                .find(|e| e.mark == m)
                .unwrap_or_else(|| panic!("{m} topilmadi"))
        };
        let wall = by_mark("IFC-D1");
        assert_eq!(wall.kind, crate::domain::ElementKind::Wall);
        assert_eq!(wall.section, crate::model::Section::Ar);
        assert_eq!(wall.level, "3-qavat");
        assert_eq!(by_mark("IFC-V1").section, crate::model::Section::Ov);

        // Devor -> teshik -> deraza zanjiri bazaga tushdi.
        let links = t.db.element_links(pid);
        let has = |from: i64, to: i64| {
            links
                .iter()
                .any(|l| l.from_el == from && l.to_el == to && l.relation == Relation::Contains)
        };
        assert!(has(wall.id, by_mark("IFC-PR1").id));
        assert!(has(by_mark("IFC-PR1").id, by_mark("IFC-OK1").id));
        assert!(app.element_links.len() > links_before);

        // Ikkinchi import dublikat yaratmaydi.
        app.import_ifc(&path);
        let again = app.elements.iter().filter(|e| e.sheet == "IFC").count();
        assert_eq!(again, 4, "qayta import dublikat berdi");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TZ XIII-XVI: resurs modullarining namuna ma'lumoti to'liq va izchil.
    #[test]
    fn demo_resources_are_consistent() {
        use crate::domain::{IssueStatus, MachineStatus, SafetyKind};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();

        let workers = t.db.workers(pid);
        let sheet = t.db.timesheet(pid);
        assert!(workers.len() >= 6, "namunada ishchilar yetarli emas");
        assert!(!sheet.is_empty(), "tabel bo'sh");
        // Tabelda faqat mavjud ishchilar va real soatlar. Yo'qlik kunida
        // soat nol bo'lishi mumkin — ishchi kelmagan.
        for e in &sheet {
            assert!(workers.iter().any(|w| w.id == e.worker_id));
            assert!(e.hours >= 0.0 && e.hours <= 24.0, "soat: {}", e.hours);
            assert!(
                e.hours > 0.0 || e.kind != crate::domain::DayKind::Work,
                "ish kunida soat nol"
            );
            assert!(e.date <= today);
        }
        // Namunada brigada, yo'qlik va bo'sh turish ko'rinadi (TZ XIII.8, 16–22).
        assert_eq!(t.db.brigades(pid).len(), 2);
        assert!(workers.iter().filter(|w| w.brigade_id.is_some()).count() >= 4);
        assert!(sheet
            .iter()
            .any(|e| e.kind == crate::domain::DayKind::Downtime));
        assert!(sheet.iter().any(|e| e.kind == crate::domain::DayKind::Sick));
        assert!(sheet
            .iter()
            .any(|e| e.shift == crate::domain::Shift::Evening));

        // Sifat: mos emas va shartli holatlar namunada bor.
        let quality = t.db.quality_checks(pid);
        assert!(quality.len() >= 5);
        assert!(quality.iter().any(|q| q.result == QualityResult::Fail));
        assert!(quality
            .iter()
            .any(|q| q.result == QualityResult::Conditional));

        // Xavfsizlik: muddati o'tgan yopilmagan yozuv bor.
        let safety = t.db.safety_events(pid);
        assert!(safety.iter().any(|s| s.kind == SafetyKind::Training));
        assert!(
            safety.iter().any(|s| {
                matches!(s.status, IssueStatus::Open | IssueStatus::InWork)
                    && s.deadline.is_some_and(|d| d < today)
            }),
            "muddati o'tgan xavfsizlik yozuvi yo'q"
        );

        // Texnika: smenalar mavjud texnikaga bog'langan, ta'mirdagisi ishlamaydi.
        let machines = t.db.machines(pid);
        let logs = t.db.machine_logs(pid);
        assert_eq!(machines.len(), 4);
        assert!(!logs.is_empty());
        for l in &logs {
            assert!(machines.iter().any(|m| m.id == l.machine_id));
            assert!(l.hours > 0.0);
        }
        let repair = machines
            .iter()
            .find(|m| m.status == MachineStatus::Repair)
            .expect("ta'mirdagi texnika");
        assert!(
            !logs.iter().any(|l| l.machine_id == repair.id),
            "ta'mirdagi texnikada smena bo'lmasligi kerak"
        );
        // Texnik ko'rik muddati o'tgan texnika ataylab qoldirilgan.
        assert!(machines
            .iter()
            .any(|m| m.inspection_until.is_some_and(|d| d < today)));
    }

    /// Namuna ikki marta chaqirilsa nusxalanmaydi.
    #[test]
    fn demo_resources_are_seeded_once() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let n = t.db.workers(pid).len();
        let m = t.db.machines(pid).len();
        t.db.seed_demo_resources(pid, false);
        assert_eq!(t.db.workers(pid).len(), n);
        assert_eq!(t.db.machines(pid).len(), m);
    }

    /// TZ XIX: namuna sotuv ma'lumoti to'liq quriladi va kvartira holati
    /// shartnomaga mos keladi.
    #[test]
    fn demo_sales_is_consistent() {
        use crate::domain::{DealStatus, UnitStatus};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let blocks = t.db.blocks(pid);
        let units = t.db.units(pid);
        let deals = t.db.deals(pid);
        let payments = t.db.payments(pid);

        assert_eq!(blocks.len(), 2);
        // Ikki blok x 9 qavat x 4 kvartira.
        assert_eq!(units.len(), 72, "kvartiralar soni");
        assert_eq!(deals.len(), 8);
        assert!(!payments.is_empty());

        for d in &deals {
            let u = units.iter().find(|u| u.id == d.unit_id).expect("kvartira");
            let want = crate::sales::status_for(Some(d)).unwrap();
            assert_eq!(u.status, want, "{} holati mos emas", d.number);

            // Har bir shartnomaning grafigi shartnoma summasiga teng.
            let st = crate::sales::deal_state(d, &payments, chrono::Local::now().date_naive());
            assert!(
                !st.schedule_mismatch,
                "{}: grafik {} != {}",
                d.number,
                st.planned,
                d.total()
            );
        }

        // Sotilgan va band qilingan kvartiralar bor, bo'shlari ham qolgan.
        let free = units
            .iter()
            .filter(|u| u.status == UnitStatus::Free)
            .count();
        assert!(free > 0 && free < units.len());
        assert!(deals.iter().any(|d| d.status == DealStatus::Completed));
        assert!(deals.iter().any(|d| d.status == DealStatus::Reserved));
    }

    /// Xulosa qarz va tushumni shartnomalardan yig'adi.
    #[test]
    fn sales_summary_adds_up() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let units = t.db.units(pid);
        let deals = t.db.deals(pid);
        let payments = t.db.payments(pid);

        let s = crate::sales::sales_summary(&units, &deals, &payments, today);
        assert_eq!(s.units, units.len());
        assert!(s.contracted > 0.0);
        assert!(s.received > 0.0, "namunada to'langan shartnomalar bor");
        // Tushum shartnomalar summasidan oshib ketmaydi.
        assert!(s.received <= s.contracted + 1.0);
        // Qarz = shartnoma summasi - tushum.
        assert!(
            (s.debt - (s.contracted - s.received)).abs() < 1.0,
            "qarz: {}",
            s.debt
        );
        assert!(s.avg_price_m2 > 0.0);
    }

    /// Namuna ikki marta chaqirilsa nusxalanmaydi.
    #[test]
    fn demo_sales_is_seeded_once() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let n = t.db.units(pid).len();
        t.db.seed_demo_sales(pid, false);
        assert_eq!(t.db.units(pid).len(), n);
        assert_eq!(t.db.blocks(pid).len(), 2);
    }

    /// TZ IX-X: qoplanish buyurtma miqdoriga, kechikish esa ehtiyoj sanasiga
    /// qarab aniqlanadi; yopilgan va rad etilgan ariza kechikkan hisoblanmaydi.
    #[test]
    fn supply_status_reflects_requests_and_purchases() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let reqs = t.db.requests(pid);
        let purs = t.db.purchases(pid);
        assert!(reqs.len() >= 4, "arizalar yetarli emas: {}", reqs.len());

        let lines = crate::checks::supply_status(&reqs, &purs, today);
        let by_number = |n: &str| {
            let q = reqs.iter().find(|q| q.number == n).expect("ariza");
            (
                q.clone(),
                lines.iter().find(|l| l.request_id == q.id).expect("qator"),
            )
        };

        // Z-001 — yopilgan zanjir: to'liq qoplangan, kechikish yo'q.
        let (_, l1) = by_number("Z-001");
        assert!(l1.covered);
        assert!(!l1.late);
        assert!(l1.delivered > 0.0);

        // Z-003 — muddati o'tgan va xarid ehtiyojni qoplamaydi.
        let (_, l3) = by_number("Z-003");
        assert!(l3.late);
        assert!(!l3.covered);

        // Z-004 — xarid ochilmagan.
        let (_, l4) = by_number("Z-004");
        assert_eq!(l4.purchases, 0);
        assert_eq!(l4.ordered, 0.0);
    }

    /// Yordamchi: sinov uchun bitta material.
    #[cfg(test)]
    fn test_material(pid: i64, code: &str, min_stock: f64) -> crate::domain::Material {
        crate::domain::Material {
            id: 0,
            project_id: pid,
            code: code.into(),
            name: code.into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock,
            price: 1_000.0,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        }
    }

    /// TZ XI.14–15: normativ sarf bajarilgan hajmga qarab hisoblanadi va
    /// ruxsat etilgan foizdan oshgani ortiqcha sarf deb belgilanadi.
    #[test]
    fn consumption_compares_norm_with_fact() {
        use crate::domain::{MaterialNorm, MoveKind, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "N-1", 0.0));
        let today = chrono::Local::now().date_naive();

        // Ish: hajmi 100, yarmi bajarilgan → normativ hajm 50.
        let mut tasks = t.db.tasks(pid).unwrap_or_default();
        let task = tasks.first_mut().expect("ish");
        task.volume = 100.0;
        task.unit = "m3".into();
        task.progress = 50.0;
        t.db.update_task(task).unwrap();
        let tid = task.id;

        // Norma: bir birlikka 2.0 → normativ sarf 100.0, ruxsat 5% → 105.0.
        t.db.insert_material_norm(&MaterialNorm {
            id: 0,
            project_id: pid,
            task_id: tid,
            material_id: mid,
            per_unit: 2.0,
            tolerance: 5.0,
            note: String::new(),
        });

        let out = |kind: MoveKind, qty: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: today,
                kind,
                qty,
                price: 0.0,
                document: String::new(),
                counterparty: String::new(),
                task_id: Some(tid),
                note: String::new(),
                warehouse_id: None,
                batch_id: None,
            });
        };
        out(MoveKind::Out, 110.0);
        // Qaytarilgan material sarf hisobidan chiqadi.
        out(MoveKind::Return, 4.0);

        let calc = |db: &crate::db::Db| {
            crate::checks::consumption(
                &db.material_norms(pid),
                &db.tasks(pid).unwrap_or_default(),
                &db.materials(pid),
                &db.stock_moves(pid),
            )
            .into_iter()
            .find(|l| l.material_id == mid)
            .expect("qator")
        };

        let l = calc(&t.db);
        assert_eq!(l.done_volume, 50.0);
        assert_eq!(l.norm, 100.0);
        assert_eq!(l.fact, 106.0, "qaytarilgani ayriladi");
        assert_eq!(l.diff, 6.0);
        assert!(l.over, "105.0 ruxsatdan oshgan");

        // Ruxsatni kengaytirsak — endi ortiqcha sarf emas.
        // Namunada ham normalar bor — o'zimiznikini material bo'yicha topamiz.
        let mut n =
            t.db.material_norms(pid)
                .into_iter()
                .find(|n| n.material_id == mid)
                .expect("norma");
        n.tolerance = 10.0;
        t.db.update_material_norm(&n);
        let l = calc(&t.db);
        assert!(!l.over);
        assert_eq!(l.over_cost, 0.0);
    }

    /// TZ XIII.13–14, 22: ish haqi smena va kun turini hisobga oladi.
    #[test]
    fn wages_count_shift_overtime_and_unpaid_days() {
        use crate::domain::{DayKind, Shift, Worker};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let wid = t.db.insert_worker(&Worker {
            id: 0,
            project_id: pid,
            name: "Sinov ishchisi".into(),
            position: String::new(),
            org: String::new(),
            hourly_rate: 10_000.0,
            active: true,
            brigade_id: None,
        });
        let day = chrono::NaiveDate::from_ymd_opt(2026, 3, 2).expect("sana");
        let d = |n: i64| day + chrono::Duration::days(n);

        // Oddiy kun: 8 soat → 80 000.
        t.db.set_timesheet(pid, wid, d(0), 8.0);
        // Ortiqcha ish: 10 soat → (8 + 2×1.5) × 10 000 = 110 000.
        t.db.set_timesheet(pid, wid, d(1), 10.0);
        // Tungi smena: 8 soat × 1.5 → 120 000.
        t.db.set_timesheet(pid, wid, d(2), 8.0);
        t.db.set_timesheet_shift(pid, wid, d(2), Shift::Night);
        // Bo'sh turish to'lanadi: 4 soat → 40 000.
        t.db.set_timesheet(pid, wid, d(3), 4.0);
        t.db.set_timesheet_kind(pid, wid, d(3), DayKind::Downtime);
        // Sababsiz yo'qlik to'lanmaydi.
        t.db.set_timesheet_kind(pid, wid, d(4), DayKind::Absent);

        let lines = crate::checks::wages(&t.db.workers(pid), &t.db.timesheet(pid), day, d(6));
        let l = lines.iter().find(|l| l.worker_id == wid).expect("qator");

        assert_eq!(l.hours, 30.0);
        assert_eq!(l.worked_hours, 26.0, "bo'sh turish ishlangan soat emas");
        assert_eq!(l.overtime_hours, 2.0);
        assert_eq!(l.night_hours, 8.0);
        assert_eq!(l.downtime_hours, 4.0);
        assert_eq!(l.absence_days, 1);
        assert_eq!(l.absent_days, 1);
        assert_eq!(l.wage, 80_000.0 + 110_000.0 + 120_000.0 + 40_000.0);
    }

    /// TZ XIII.24–25: brigadalar bo'sh turish ulushi bilan solishtiriladi.
    #[test]
    fn brigade_lines_compare_groups() {
        use crate::domain::{Brigade, DayKind, Worker};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let day = chrono::NaiveDate::from_ymd_opt(2026, 4, 6).expect("sana");
        let bid = t.db.insert_brigade(&Brigade {
            id: 0,
            project_id: pid,
            name: "Sinov brigadasi".into(),
            foreman: String::new(),
            task_id: None,
            note: String::new(),
        });
        let mut ids = Vec::new();
        for i in 0..2 {
            ids.push(t.db.insert_worker(&Worker {
                id: 0,
                project_id: pid,
                name: format!("Ishchi {i}"),
                position: String::new(),
                org: String::new(),
                hourly_rate: 10_000.0,
                active: true,
                brigade_id: Some(bid),
            }));
        }
        // Ikkovi 8 soatdan; birining yarim kuni bo'sh turish.
        t.db.set_timesheet(pid, ids[0], day, 8.0);
        t.db.set_timesheet(pid, ids[1], day, 8.0);
        t.db.set_timesheet(pid, ids[1], day + chrono::Duration::days(1), 4.0);
        t.db.set_timesheet_kind(
            pid,
            ids[1],
            day + chrono::Duration::days(1),
            DayKind::Downtime,
        );

        let lines = crate::checks::brigade_lines(
            &t.db.brigades(pid),
            &t.db.workers(pid),
            &t.db.timesheet(pid),
            day,
            day + chrono::Duration::days(6),
        );
        let l = lines.iter().find(|l| l.brigade_id == bid).expect("qator");
        assert_eq!(l.workers, 2);
        assert_eq!(l.hours, 20.0);
        assert_eq!(l.worked_hours, 16.0);
        assert_eq!(l.downtime_hours, 4.0);
        assert_eq!(l.downtime_pct, 20.0);
        assert_eq!(l.wage, 200_000.0);
        assert_eq!(
            l.cost_per_hour, 12_500.0,
            "bo'sh turish soatning tannarxini oshiradi"
        );
    }

    /// TZ XIII.30–31: ish tannarxi tabel va ombordan yig'iladi.
    #[test]
    fn task_cost_sums_labour_and_material() {
        use crate::domain::{MoveKind, StockMove, Worker};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut tasks = t.db.tasks(pid).unwrap_or_default();
        let task = tasks.first_mut().expect("ish");
        task.volume = 100.0;
        t.db.update_task(task).unwrap();
        let tid = task.id;

        let wid = t.db.insert_worker(&Worker {
            id: 0,
            project_id: pid,
            name: "Tannarx ishchisi".into(),
            position: String::new(),
            org: String::new(),
            hourly_rate: 10_000.0,
            active: true,
            brigade_id: None,
        });
        let day = chrono::NaiveDate::from_ymd_opt(2026, 5, 4).expect("sana");
        t.db.set_timesheet(pid, wid, day, 8.0);
        t.db.set_timesheet_task(pid, wid, day, Some(tid));

        let mid = t.db.insert_material(&test_material(pid, "C-1", 0.0));
        t.db.insert_stock_move(&StockMove {
            id: 0,
            project_id: pid,
            material_id: mid,
            date: day,
            kind: MoveKind::Out,
            qty: 5.0,
            price: 20_000.0,
            document: String::new(),
            counterparty: String::new(),
            task_id: Some(tid),
            note: String::new(),
            warehouse_id: None,
            batch_id: None,
        });

        let costs = crate::checks::task_costs(
            &t.db.tasks(pid).unwrap_or_default(),
            &t.db.workers(pid),
            &t.db.timesheet(pid),
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.machines(pid),
            &t.db.machine_logs(pid),
        );
        let c = costs.iter().find(|c| c.task_id == tid).expect("qator");
        assert_eq!(c.labour, 80_000.0);
        assert_eq!(c.material, 100_000.0);
        assert_eq!(c.machine, 0.0, "bu ishda texnika ishlamagan");
        assert_eq!(c.total, 180_000.0);
        assert_eq!(c.per_unit, 1_800.0);
    }

    /// TZ X.30: qisman yetkazish qoldiqni ochiq qoldiradi.
    #[test]
    fn partial_delivery_leaves_a_remainder() {
        use crate::domain::{Purchase, PurchaseStatus};

        let p = |qty: f64, delivered: f64| Purchase {
            id: 0,
            project_id: 1,
            request_id: None,
            number: "X-1".into(),
            date: chrono::Local::now().date_naive(),
            supplier: String::new(),
            title: String::new(),
            qty,
            unit: String::new(),
            price: 1000.0,
            currency: "UZS".into(),
            delivery_date: chrono::Local::now().date_naive(),
            status: PurchaseStatus::Ordered,
            delivered_qty: delivered,
            section: crate::model::Section::None,
            task_id: None,
            contract_id: None,
            urgent: false,
            buyer: String::new(),
            material_id: None,
            substitute_for: None,
            paid: 0.0,
            pay_due: None,
            tech_ok: true,
            tech_by: "Test".into(),
            note: String::new(),
        };
        let a = p(180.0, 120.0);
        assert!(a.partial());
        assert!(!a.fully_delivered());
        assert_eq!(a.remaining(), 60.0);

        let b = p(180.0, 180.0);
        assert!(b.fully_delivered());
        assert!(!b.partial());
        assert_eq!(b.remaining(), 0.0);

        // Hech narsa kelmagan buyurtma «qisman» emas.
        assert!(!p(180.0, 0.0).partial());
    }

    /// TZ X.11–12: takliflar ariza kesimida solishtiriladi.
    #[test]
    fn quotes_mark_the_cheapest_and_the_fastest() {
        use crate::domain::Quote;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let rid = t.db.requests(pid).first().map(|r| r.id);

        let mk = |price: f64, days: i64, valid: Option<i64>| {
            t.db.insert_quote(&Quote {
                id: 0,
                project_id: pid,
                request_id: rid,
                supplier: format!("S{price}"),
                title: "Sinov".into(),
                qty: 10.0,
                unit: "t".into(),
                price,
                currency: "UZS".into(),
                delivery_days: days,
                valid_until: valid.map(|d| today + chrono::Duration::days(d)),
                chosen: false,
                date: today,
                note: String::new(),
            })
        };
        let cheap = mk(100.0, 30, Some(10));
        let quick = mk(120.0, 3, Some(10));
        // Eng arzoni, lekin muddati o'tgan — hisobga olinmasligi kerak.
        let stale = mk(80.0, 2, Some(-1));

        let quotes: Vec<Quote> =
            t.db.quotes(pid)
                .into_iter()
                .filter(|q| q.request_id == rid)
                .collect();
        let lines = crate::checks::quote_lines(&quotes, today);
        let get = |id: i64| lines.iter().find(|l| l.quote_id == id).expect("qator");

        assert!(
            get(cheap).cheapest,
            "muddati o'tgani eng arzon bo'la olmaydi"
        );
        assert!(!get(cheap).fastest);
        assert!(get(quick).fastest);
        assert_eq!(get(quick).over_best_pct, 20.0);
        assert!(get(stale).expired);
        assert!(!get(stale).cheapest);

        // Tanlash: bitta arizada faqat bitta taklif tanlanadi.
        t.db.choose_quote(cheap);
        t.db.choose_quote(quick);
        let chosen: Vec<i64> =
            t.db.quotes(pid)
                .into_iter()
                .filter(|q| q.chosen && q.request_id == rid)
                .map(|q| q.id)
                .collect();
        assert_eq!(chosen, vec![quick]);
    }

    /// TZ X.8, 40: yetkazib beruvchi tarixi xaridlardan hisoblanadi.
    #[test]
    fn supplier_history_comes_from_purchases() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let lines = crate::checks::supplier_lines(&t.db.purchases(pid), today);

        assert!(!lines.is_empty(), "namunada xarid bor");
        for l in &lines {
            assert!(!l.supplier.is_empty());
            assert!(l.orders > 0);
            assert!((0.0..=100.0).contains(&l.on_time_pct));
        }
        // Namunada muddati o'tgan, to'liq yetkazilmagan buyurtma bor.
        assert!(
            lines.iter().any(|l| l.avg_delay > 0.0),
            "kechikish ko'rinmadi"
        );
        assert!(lines.iter().any(|l| l.open_orders > 0));
    }

    /// TZ X.13: narx katalogdagidan keskin farq qilsa belgilanadi.
    #[test]
    fn price_anomaly_triggers_only_on_a_big_gap() {
        use crate::checks::price_anomaly;
        assert_eq!(price_anomaly(120.0, 100.0), Some(20.0));
        assert_eq!(price_anomaly(70.0, 100.0), Some(-30.0));
        assert_eq!(price_anomaly(110.0, 100.0), None, "10% — normal tebranish");
        // Narx yoki katalog nol bo'lsa — solishtiradigan narsa yo'q.
        assert_eq!(price_anomaly(0.0, 100.0), None);
        assert_eq!(price_anomaly(100.0, 0.0), None);
    }

    /// TZ X.34–35: byudjet bo'lim kesimida nazorat qilinadi.
    #[test]
    fn budget_tracks_sections_and_flags_overspend() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let lines = crate::checks::budget_lines(&t.db.purchase_budgets(pid), &t.db.purchases(pid));

        let vk = lines
            .iter()
            .find(|l| l.section == crate::model::Section::Vk)
            .expect("VK bo'limi");
        assert!(vk.over, "byudjeti kam bo'lim oshib ketgan bo'lishi kerak");
        assert!(vk.left < 0.0);
        assert!(
            vk.delivered > 0.0 && vk.delivered < vk.ordered,
            "qisman yetkazilgan"
        );

        let kj = lines
            .iter()
            .find(|l| l.section == crate::model::Section::Kj)
            .expect("KJ bo'limi");
        assert!(!kj.over);
        assert!(kj.used_pct > 0.0 && kj.used_pct < 100.0);
    }

    /// TZ IX.9: marshrut ariza summasiga qarab uzayadi.
    #[test]
    fn approval_route_grows_with_the_amount() {
        use crate::checks::{approval_route, APPROVAL_LIMIT_1, APPROVAL_LIMIT_2};
        use crate::roles::Role;

        assert_eq!(approval_route(0.0), vec![Role::Foreman]);
        assert_eq!(approval_route(APPROVAL_LIMIT_1), vec![Role::Foreman]);
        assert_eq!(
            approval_route(APPROVAL_LIMIT_1 + 1.0),
            vec![Role::Foreman, Role::ProjectManager]
        );
        assert_eq!(
            approval_route(APPROVAL_LIMIT_2 + 1.0),
            vec![Role::Foreman, Role::ProjectManager, Role::Director]
        );
        // Har bir marshrutda bosqichlar takrorlanmaydi.
        for amount in [1.0, 50_000_000.0, 500_000_000.0] {
            let r = approval_route(amount);
            let mut sorted = r.clone();
            sorted.sort_by_key(|x| x.code());
            sorted.dedup();
            assert_eq!(sorted.len(), r.len(), "takroriy bosqich: {amount}");
        }
    }

    /// TZ IX.8: bosqichlar tartib bilan o'tiladi, bitta rad hammasini to'xtatadi.
    #[test]
    fn route_state_follows_the_steps_in_order() {
        use crate::checks::{route_state, RouteState};
        use crate::domain::{Approval, ApprovalDecision};
        use crate::roles::Role;

        let today = chrono::Local::now().date_naive();
        let mk = |step: i64, role: Role, decision: ApprovalDecision| Approval {
            id: step,
            project_id: 1,
            request_id: 7,
            step,
            role: role.code().into(),
            approver: String::new(),
            decision,
            decided_at: None,
            comment: String::new(),
        };
        let _ = today;

        assert_eq!(route_state(7, &[]), RouteState::None);

        let mut list = vec![
            mk(1, Role::Foreman, ApprovalDecision::Pending),
            mk(2, Role::ProjectManager, ApprovalDecision::Pending),
        ];
        assert_eq!(
            route_state(7, &list),
            RouteState::Waiting {
                step: 1,
                role: Role::Foreman
            }
        );

        list[0].decision = ApprovalDecision::Approved;
        assert_eq!(
            route_state(7, &list),
            RouteState::Waiting {
                step: 2,
                role: Role::ProjectManager
            }
        );

        list[1].decision = ApprovalDecision::Approved;
        assert_eq!(route_state(7, &list), RouteState::Approved);

        // Rad etish keyingi bosqichlarga qaramay marshrutni to'xtatadi.
        list[0].decision = ApprovalDecision::Rejected;
        assert_eq!(
            route_state(7, &list),
            RouteState::Rejected {
                step: 1,
                role: Role::Foreman
            }
        );

        // Boshqa arizaning bosqichlari aralashmaydi.
        let mut other = mk(1, Role::Director, ApprovalDecision::Pending);
        other.request_id = 8;
        list.push(other);
        assert_eq!(
            route_state(8, &list),
            RouteState::Waiting {
                step: 1,
                role: Role::Director
            }
        );
    }

    /// TZ IX.10: ariza bo'lim byudjetiga solishtiriladi.
    #[test]
    fn request_is_checked_against_the_section_budget() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let materials = t.db.materials(pid);
        let budgets = t.db.purchase_budgets(pid);
        let purchases = t.db.purchases(pid);

        // Namunada VK byudjeti ataylab kam — u yerdagi ariza oshib ketadi.
        let vk = materials
            .iter()
            .find(|m| m.section == crate::model::Section::Vk)
            .expect("VK materiali");
        let mut q = t.db.requests(pid).into_iter().next().expect("ariza");
        q.material_id = Some(vk.id);
        q.qty = 1.0;
        let left = crate::checks::request_budget_left(&q, &materials, &budgets, &purchases)
            .expect("byudjet bor");
        assert!(
            left < 0.0,
            "byudjeti kam bo'limda ariza oshib ketishi kerak"
        );

        // Materiali yo'q arizada tekshiradigan narsa yo'q.
        q.material_id = None;
        assert!(crate::checks::request_budget_left(&q, &materials, &budgets, &purchases).is_none());
    }

    /// Namunada kelishuv marshrutlari ikki xil holatda ko'rinadi.
    #[test]
    fn demo_shows_a_finished_and_a_waiting_route() {
        use crate::checks::{route_state, RouteState};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let approvals = t.db.approvals(pid);
        assert!(!approvals.is_empty(), "namunada marshrut yo'q");

        let states: Vec<RouteState> =
            t.db.requests(pid)
                .iter()
                .map(|q| route_state(q.id, &approvals))
                .collect();
        assert!(states.contains(&RouteState::Approved));
        assert!(states
            .iter()
            .any(|s| matches!(s, RouteState::Waiting { .. })));
    }

    /// Yordamchi: sinov uchun GPR ishi.
    #[cfg(test)]
    fn test_task(id: i64, volume: f64, progress: f64) -> crate::model::Task {
        crate::model::Task {
            id,
            project_id: 1,
            wbs: "1".into(),
            name: "sinov ishi".into(),
            section: crate::model::Section::Kj,
            responsible: String::new(),
            duration: 10,
            plan_start: chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            fact_start: None,
            fact_end: None,
            progress,
            pinned: false,
            volume,
            unit: "m3".into(),
        }
    }

    /// Yordamchi: sinov uchun sifat tekshiruvi.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn test_check(
        pid: i64,
        kind: crate::domain::QualityKind,
        task_id: Option<i64>,
        result: crate::domain::QualityResult,
        defect: &str,
        deadline: Option<chrono::NaiveDate>,
        fixed_at: Option<chrono::NaiveDate>,
    ) -> crate::domain::QualityCheck {
        crate::domain::QualityCheck {
            id: 0,
            project_id: pid,
            kind,
            date: chrono::Local::now().date_naive(),
            task_id,
            material_id: None,
            subject: "Sinov".into(),
            inspector: String::new(),
            result,
            defect: defect.into(),
            deadline,
            checklist_id: None,
            fixed_at,
            note: String::new(),
        }
    }

    /// TZ XIV.33: sifat balli ochiq nuqsonlar uchun pasayadi.
    #[test]
    fn quality_score_drops_with_open_defects() {
        use crate::checks::quality_score;
        use crate::domain::QualityKind;

        let today = chrono::Local::now().date_naive();
        let mk = |result: crate::domain::QualityResult, fixed: bool, deadline: Option<i64>| {
            test_check(
                1,
                QualityKind::Operational,
                None,
                result,
                "n",
                deadline.map(|d| today + chrono::Duration::days(d)),
                fixed.then_some(today),
            )
        };

        // Tekshiruv yo'q — ball qo'yilmaydi, 100 emas.
        let empty = quality_score(&[], today);
        assert_eq!(empty.checks, 0);
        assert_eq!(empty.score, 0.0);

        // Hammasi o'tgan.
        let all_pass: Vec<_> = (0..4)
            .map(|_| mk(QualityResult::Pass, false, None))
            .collect();
        assert_eq!(quality_score(&all_pass, today).score, 100.0);

        // Bittasi shartli, bartaraf etilgan: 3.5/4 = 87.5, jarima yo'q.
        let mut list = all_pass.clone();
        list[0] = mk(QualityResult::Conditional, true, None);
        let s = quality_score(&list, today);
        assert_eq!(s.conditional, 1);
        assert_eq!(s.open, 0);
        assert_eq!(s.score, 87.5);

        // O'sha nuqson ochiq bo'lsa — 2 ball jarima.
        list[0] = mk(QualityResult::Conditional, false, None);
        assert_eq!(quality_score(&list, today).score, 85.5);

        // Muddati o'tgan bo'lsa — yana 3 ball.
        list[0] = mk(QualityResult::Conditional, false, Some(-1));
        let s = quality_score(&list, today);
        assert_eq!(s.overdue, 1);
        assert_eq!(s.score, 82.5);

        // Ball 0 dan pastga tushmaydi.
        let bad: Vec<_> = (0..30)
            .map(|_| mk(QualityResult::Fail, false, Some(-5)))
            .collect();
        assert_eq!(quality_score(&bad, today).score, 0.0);
    }

    /// TZ XIV.10, 35: yopilishga yaqin ish sifat bo'yicha bloklanadi.
    #[test]
    fn task_blocks_catch_unfinished_quality() {
        use crate::checks::{task_blocks, CLOSING_PROGRESS};
        use crate::domain::{CheckPoint, PointResult, QualityKind};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();

        let mut tasks = t.db.tasks(pid).unwrap_or_default();
        let task = tasks.first_mut().expect("ish");
        task.progress = CLOSING_PROGRESS;
        t.db.update_task(task).unwrap();
        let tid = task.id;
        let tasks = t.db.tasks(pid).unwrap_or_default();

        // Tekshiruv umuman yo'q — qabul nazorati yetishmaydi.
        let blocks = task_blocks(&tasks, &[], &[], today);
        let b = blocks.iter().find(|b| b.task_id == tid).expect("blok");
        assert!(b.no_acceptance);
        assert!(b.blocked());

        // Qabul nazorati bor va nuqsonsiz — to'siq yo'q.
        let ok = vec![test_check(
            pid,
            QualityKind::Acceptance,
            Some(tid),
            QualityResult::Pass,
            "",
            None,
            None,
        )];
        assert!(!task_blocks(&tasks, &ok, &[], today)
            .iter()
            .any(|b| b.task_id == tid));

        // Bartaraf etilmagan nuqson — to'siq.
        let mut bad = ok.clone();
        bad[0].result = QualityResult::Fail;
        bad[0].deadline = Some(today - chrono::Duration::days(2));
        bad[0].id = 5;
        let blocks = task_blocks(&tasks, &bad, &[], today);
        let b = blocks.iter().find(|b| b.task_id == tid).expect("blok");
        assert_eq!(b.open_defects, 1);
        assert_eq!(b.overdue, 1);

        // Bartaraf etilgan bo'lsa to'siq qolmaydi.
        let mut fixed = bad.clone();
        fixed[0].fixed_at = Some(today);
        assert!(!task_blocks(&tasks, &fixed, &[], today)
            .iter()
            .any(|b| b.task_id == tid));

        // To'ldirilmagan nazorat nuqtasi ham to'sadi.
        let point = CheckPoint {
            id: 1,
            check_id: 5,
            pos: 1,
            text: "Nuqta".into(),
            norm_doc: String::new(),
            norm_clause: String::new(),
            result: PointResult::Pending,
            note: String::new(),
        };
        let blocks = task_blocks(&tasks, &fixed, std::slice::from_ref(&point), today);
        let b = blocks.iter().find(|b| b.task_id == tid).expect("blok");
        assert_eq!(b.pending_points, 1);
    }

    /// TZ XIV.30–31: bir xil nuqson guruhlanadi.
    #[test]
    fn defect_groups_find_repeats() {
        use crate::checks::defect_groups;
        use crate::domain::QualityKind;

        let today = chrono::Local::now().date_naive();
        let mk = |defect: &str, task: Option<i64>, fixed: bool| {
            test_check(
                1,
                QualityKind::Operational,
                task,
                QualityResult::Fail,
                defect,
                None,
                fixed.then_some(today),
            )
        };
        let list = vec![
            mk("Beton kavakligi", Some(1), false),
            // Bir xil sabab, boshqacha yozilgan — bitta guruhga tushishi kerak.
            mk("beton  KAVAKLIGI", Some(2), true),
            mk("Chok qalinligi", Some(1), false),
            test_check(
                1,
                QualityKind::Operational,
                None,
                QualityResult::Pass,
                "Beton kavakligi",
                None,
                None,
            ),
        ];

        let groups = defect_groups(&list);
        assert_eq!(groups.len(), 2, "o'tgan tekshiruv brak emas");
        let first = &groups[0];
        assert_eq!(first.count, 2, "yozilishi farq qilsa ham bitta sabab");
        assert_eq!(first.open, 1);
        assert_eq!(first.tasks, vec![1, 2]);
    }

    /// Namunada chek-listlar va to'ldirilgan nazorat nuqtalari bor.
    #[test]
    fn demo_has_checklists_with_points() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let lists = t.db.checklists(pid);
        assert_eq!(lists.len(), 2);
        let items = t.db.checklist_items(pid);
        assert!(items.len() >= 8);
        for i in &items {
            assert!(!i.text.is_empty());
            assert!(!i.norm_doc.is_empty(), "normativ havolasi yo'q");
        }

        let points = t.db.check_points(pid);
        assert!(!points.is_empty(), "nazorat nuqtalari ko'chirilmagan");
        assert!(points
            .iter()
            .any(|p| p.result == crate::domain::PointResult::Fail));
    }

    /// TZ XV.4–9: ruxsat va SIZ holati ishchi bo'yicha yig'iladi.
    #[test]
    fn worker_safety_tracks_permits_and_ppe() {
        use crate::checks::worker_safety;
        use crate::domain::{PermitKind, PpeItem};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let safety = worker_safety(
            &t.db.workers(pid),
            &t.db.worker_permits(pid),
            &t.db.ppe_issues(pid),
            today,
        );
        assert_eq!(safety.len(), t.db.workers(pid).len());

        // Namunada ataylab qoldirilgan uch holat bor.
        assert!(
            safety
                .iter()
                .any(|s| s.expired.contains(&PermitKind::Induction)),
            "muddati o'tgan instruktaj yo'q"
        );
        assert!(
            safety
                .iter()
                .any(|s| s.ppe_missing.contains(&PpeItem::Gloves)),
            "SIZ yetishmasligi yo'q"
        );
        assert!(
            safety.iter().any(|s| !s.expiring.is_empty()),
            "muddati tugayotgan ruxsat yo'q"
        );
        // Ogohlantirish chegarasi hurmat qilinadi.
        for s in &safety {
            for k in &s.expiring {
                assert!(s.valid.contains(k), "tugayotgani hali amal qiladi");
            }
        }

        // Instruktaji o'tib ketgan yoki SIZ yo'q ishchini ishga qo'yib bo'lmaydi.
        assert!(safety.iter().any(|s| s.blocked()));
    }

    /// TZ XV.12: naryad kamchiliklari topiladi.
    #[test]
    fn permit_issues_catch_a_bad_naryad() {
        use crate::checks::{permit_issues, worker_safety, PermitIssue};
        use crate::domain::PermitStatus;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let safety = worker_safety(
            &t.db.workers(pid),
            &t.db.worker_permits(pid),
            &t.db.ppe_issues(pid),
            today,
        );
        let permits = t.db.work_permits(pid);
        assert_eq!(permits.len(), 2);

        // Birinchisi to'g'ri to'ldirilgan.
        let good = permits
            .iter()
            .find(|p| p.number.ends_with("001"))
            .expect("ND-001");
        assert!(
            permit_issues(good, &safety, today).is_empty(),
            "to'g'ri naryadda kamchilik topildi"
        );

        // Ikkinchisida chora-tadbir ham, nazoratchi ham yo'q.
        let bad = permits
            .iter()
            .find(|p| p.number.ends_with("002"))
            .expect("ND-002");
        let issues = permit_issues(bad, &safety, today);
        assert!(issues.contains(&PermitIssue::NoMeasures));
        assert!(issues.contains(&PermitIssue::NoIssuer));
        // O't ishlariga ruxsati yo'q ishchilar kiritilgan.
        assert!(issues
            .iter()
            .any(|i| matches!(i, PermitIssue::WorkerNotAllowed(_))));

        // Muddati o'tgan ochiq naryad ham belgilanadi.
        let mut late = bad.clone();
        late.status = PermitStatus::Open;
        late.date_to = today - chrono::Duration::days(1);
        assert!(permit_issues(&late, &safety, today).contains(&PermitIssue::Overdue));

        // Yopilgan naryadda muddat kamchilik emas.
        late.status = PermitStatus::Closed;
        assert!(!permit_issues(&late, &safety, today).contains(&PermitIssue::Overdue));
    }

    /// TZ XV.33: xavfsizlik balli kamchiliklardan pasayadi.
    #[test]
    fn safety_score_falls_with_findings() {
        use crate::checks::{safety_score, worker_safety};
        use crate::domain::{IssueStatus, SafetyEvent, SafetyKind, Severity};

        let today = chrono::Local::now().date_naive();
        let event = |kind: SafetyKind, days_ago: i64| SafetyEvent {
            id: 0,
            project_id: 1,
            date: today - chrono::Duration::days(days_ago),
            kind,
            severity: Severity::Warning,
            place: String::new(),
            description: String::new(),
            responsible: String::new(),
            measure: String::new(),
            deadline: None,
            status: IssueStatus::Fixed,
            root_cause: crate::domain::RootCause::Unknown,
        };

        // Hech narsa yo'q — to'liq ball.
        assert_eq!(safety_score(&[], &[], &[], today).score, 100.0);

        // Bitta buzilish — 5 ball.
        let one = vec![event(SafetyKind::Violation, 3)];
        assert_eq!(safety_score(&one, &[], &[], today).score, 95.0);

        // Hodisa eng og'ir jarima.
        let incident = vec![event(SafetyKind::Incident, 3)];
        assert_eq!(safety_score(&incident, &[], &[], today).score, 75.0);

        // 90 kundan eski hodisa hisobga olinmaydi.
        let old = vec![event(SafetyKind::Incident, 120)];
        assert_eq!(safety_score(&old, &[], &[], today).score, 100.0);

        // Namunadagi holat: ball 100 dan past, lekin nolga tushmagan.
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let safety = worker_safety(
            &t.db.workers(pid),
            &t.db.worker_permits(pid),
            &t.db.ppe_issues(pid),
            today,
        );
        let s = safety_score(
            &t.db.safety_events(pid),
            &safety,
            &t.db.work_permits(pid),
            today,
        );
        assert!(s.score < 100.0, "namunada kamchilik bor");
        assert!(s.expired_permits > 0);
        assert!(s.without_ppe > 0);
        assert!(s.bad_permits > 0);
    }

    /// TZ XVI.36–38: foydalanish koeffitsiyenti ish kunlariga nisbatan.
    #[test]
    fn machine_utilization_counts_work_days_only() {
        use crate::checks::{machine_lines, SHIFT_HOURS};
        use crate::domain::{Machine, MachineKind, MachineLog, MachineStatus};

        // Dushanbadan yakshanbagacha bir hafta: olti ish kuni.
        let from = chrono::NaiveDate::from_ymd_opt(2026, 3, 2).expect("sana");
        let to = from + chrono::Duration::days(6);

        let machine = Machine {
            id: 1,
            project_id: 1,
            name: "Sinov".into(),
            kind: MachineKind::Excavator,
            reg_no: String::new(),
            owner: String::new(),
            status: MachineStatus::Working,
            hour_rate: 100_000.0,
            operator: String::new(),
            inspection_until: None,
            fuel_norm: 10.0,
            service_hours: 100.0,
            service_done: 0.0,
            rented: false,
            price: 0.0,
        };
        let log = |days: i64, hours: f64, fuel: f64, odo: (f64, f64)| MachineLog {
            id: 0,
            project_id: 1,
            machine_id: 1,
            date: from + chrono::Duration::days(days),
            hours,
            fuel,
            task_id: None,
            number: "YV-1".into(),
            driver: String::new(),
            route: String::new(),
            odo_start: odo.0,
            odo_end: odo.1,
            trips: 0,
            cargo: 0.0,
            note: String::new(),
            gps: String::new(),
        };
        // Uch kun ishlagan, kuniga 8 soat.
        let logs = vec![
            log(0, 8.0, 100.0, (1000.0, 1040.0)),
            log(1, 8.0, 100.0, (1040.0, 1075.0)),
            log(2, 8.0, 100.0, (1075.0, 1075.0)),
        ];

        let lines = machine_lines(std::slice::from_ref(&machine), &logs, from, to, from);
        let l = &lines[0];
        assert_eq!(l.hours, 24.0);
        assert_eq!(l.work_days, 3);
        assert_eq!(l.idle_days, 3, "yakshanba bo'sh kun hisoblanmaydi");
        // 24 soat / (6 kun × 8 soat) = 50%.
        assert_eq!(l.utilization, 24.0 / (6.0 * SHIFT_HOURS) * 100.0);
        assert_eq!(l.distance, 75.0, "spidometrsiz smena masofaga qo'shilmaydi");
        assert_eq!(l.cost, 2_400_000.0);

        // Yoqilg'i: norma 10 l/soat → 240 litr, fakt 300 → ortiqcha.
        assert_eq!(l.fuel_norm, 240.0);
        assert_eq!(l.fuel_diff, 60.0);
        assert!(l.fuel_over);

        // TX: oralig'i 100 soat, 24 soat ishlangan → 76 qoldi.
        assert_eq!(l.service_left, Some(76.0));
        assert!(!l.blocked());
    }

    /// TZ XVI.27: texnik ko'rik va TX muddati ishlatishga to'sqinlik qiladi.
    #[test]
    fn machine_blocks_stop_operation() {
        use crate::checks::{machine_lines, MachineBlock};
        use crate::domain::{Machine, MachineKind, MachineStatus};

        let today = chrono::Local::now().date_naive();
        let base = Machine {
            id: 1,
            project_id: 1,
            name: "Sinov".into(),
            kind: MachineKind::Crane,
            reg_no: String::new(),
            owner: String::new(),
            status: MachineStatus::Working,
            hour_rate: 0.0,
            operator: String::new(),
            inspection_until: Some(today + chrono::Duration::days(30)),
            fuel_norm: 0.0,
            service_hours: 0.0,
            service_done: 0.0,
            rented: false,
            price: 0.0,
        };
        let at = |m: &Machine| {
            machine_lines(std::slice::from_ref(m), &[], today, today, today)[0]
                .blocks
                .clone()
        };
        assert!(at(&base).is_empty());

        let mut expired = base.clone();
        expired.inspection_until = Some(today - chrono::Duration::days(1));
        assert_eq!(at(&expired), vec![MachineBlock::Inspection]);

        // TX oralig'i tugagan: 100 soat reja, 100 soat ishlangan.
        let mut due = base.clone();
        due.service_hours = 100.0;
        due.service_done = 0.0;
        let lines = machine_lines(
            std::slice::from_ref(&due),
            &[crate::domain::MachineLog {
                id: 0,
                project_id: 1,
                machine_id: 1,
                date: today,
                hours: 100.0,
                fuel: 0.0,
                task_id: None,
                number: String::new(),
                driver: String::new(),
                route: String::new(),
                odo_start: 0.0,
                odo_end: 0.0,
                trips: 0,
                cargo: 0.0,
                note: String::new(),
                gps: String::new(),
            }],
            today,
            today,
            today,
        );
        assert_eq!(lines[0].blocks, vec![MachineBlock::Service]);

        let mut repair = base.clone();
        repair.status = MachineStatus::Repair;
        assert_eq!(at(&repair), vec![MachineBlock::Repair]);
    }

    /// TZ XVI.35: texnika xarajati ish tannarxiga kiradi.
    #[test]
    fn machine_cost_reaches_the_task() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let costs = crate::checks::task_costs(
            &t.db.tasks(pid).unwrap_or_default(),
            &t.db.workers(pid),
            &t.db.timesheet(pid),
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.machines(pid),
            &t.db.machine_logs(pid),
        );
        let with_machine: Vec<_> = costs.iter().filter(|c| c.machine > 0.0).collect();
        assert!(
            !with_machine.is_empty(),
            "texnika hech qaysi ishga tushmadi"
        );
        for c in with_machine {
            assert!(c.total >= c.machine);
            assert_eq!(c.total, c.labour + c.material + c.machine);
        }
    }

    /// Namunada yo'l varaqalari to'ldirilgan va ortiqcha sarf ko'rinadi.
    #[test]
    fn demo_waybills_show_fuel_overuse() {
        use crate::checks::machine_lines;
        use crate::domain::WaybillState;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let logs = t.db.machine_logs(pid);
        assert!(!logs.is_empty());
        for l in &logs {
            assert_eq!(l.state(), WaybillState::Closed, "yo'l varaqasi yopilmagan");
            assert!(!l.driver.is_empty(), "haydovchi ko'rsatilmagan");
            assert!(!l.route.is_empty(), "marshrut ko'rsatilmagan");
        }
        // Spidometr bo'lgan smenalarda masofa hisoblanadi.
        assert!(logs.iter().any(|l| l.distance() > 0.0));

        let lines = machine_lines(
            &t.db.machines(pid),
            &logs,
            today - chrono::Duration::days(30),
            today,
            today,
        );
        assert!(
            lines.iter().any(|l| l.fuel_over),
            "namunada ortiqcha yoqilg'i sarfi yo'q"
        );
        assert!(lines.iter().any(|l| l.blocked()), "to'siq ko'rinmadi");
    }

    /// TZ XVII.30: pul oqimi oylar kesimida yig'iladi.
    #[test]
    fn cash_flow_splits_income_and_expense_by_month() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let supply = app.supply();
        let stock = app.stock();
        let cost = app.cost_summary();
        let sales = app.sales();
        let inp = app.analytics_input(&supply, &stock, &cost, &sales);

        let flow = crate::analytics::cash_flow(&inp, 3, 3);
        assert_eq!(flow.len(), 7, "3 o'tgan + joriy + 3 kelasi oy");

        // Oylar ketma-ket va takrorlanmaydi.
        for w in flow.windows(2) {
            assert!(w[0].month < w[1].month);
        }
        // Joriy oy ro'yxatda bor.
        let now = crate::analytics::month_of(app.today);
        assert!(flow.iter().any(|m| m.month == now));

        for m in &flow {
            // Chiqim tarkibi yig'indiga teng bo'lishi kerak.
            assert!(
                (m.expense - (m.purchases + m.payroll + m.machines)).abs() < 0.01,
                "chiqim tarkibi mos emas"
            );
            assert!((m.net - (m.income - m.expense)).abs() < 0.01);
            assert!(m.income >= 0.0 && m.expense >= 0.0);
        }
        // Qoldiq — o'tgan oylarning yig'indisi.
        let mut sum = 0.0;
        for m in &flow {
            sum += m.net;
            assert!((m.balance - sum).abs() < 0.01);
        }
        // Namunada xarid ham, ish haqi ham bor.
        assert!(flow.iter().any(|m| m.purchases > 0.0));
        assert!(flow.iter().any(|m| m.payroll > 0.0));
    }

    /// TZ XVII.31: kassa uzilishi faqat kelajakdan izlanadi.
    #[test]
    fn cash_gap_looks_only_forward() {
        use crate::analytics::{cash_gap, month_of, CashMonth};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).expect("sana");
        let m = |month: u32, balance: f64| CashMonth {
            month: chrono::NaiveDate::from_ymd_opt(2026, month, 1).expect("oy"),
            past: month <= 6,
            income: 0.0,
            expense: 0.0,
            purchases: 0.0,
            payroll: 0.0,
            machines: 0.0,
            net: 0.0,
            balance,
        };

        // O'tgan oydagi minus hisobga olinmaydi.
        let past_only = vec![m(5, -100.0), m(6, -50.0), m(7, 10.0)];
        assert!(cash_gap(&past_only, today).is_none());

        // Kelasi oydagi minus topiladi.
        let ahead = vec![m(6, 100.0), m(7, 50.0), m(8, -30.0), m(9, -80.0)];
        let g = cash_gap(&ahead, today).expect("uzilish");
        assert_eq!(
            g.month,
            chrono::NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()
        );
        assert_eq!(g.amount, 30.0, "birinchi manfiy oy olinadi");
        assert_eq!(g.months_ahead, 2);

        assert_eq!(month_of(today), m(6, 0.0).month);
    }

    /// TZ XVII.37: kunlik xulosa faqat bugungi narsalarni ko'rsatadi.
    #[test]
    fn briefing_lists_only_todays_items() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Bugunga aniq bir narsa qo'shamiz: bugun bartaraf etilishi kerak nuqson.
        let mut q = app
            .quality
            .iter()
            .find(|q| q.open_defect())
            .cloned()
            .expect("ochiq nuqson");
        q.deadline = Some(app.today);
        app.db.update_quality(&q);
        app.reload_modules();

        let supply = app.supply();
        let stock = app.stock();
        let cost = app.cost_summary();
        let sales = app.sales();
        let inp = app.analytics_input(&supply, &stock, &cost, &sales);

        let lines = crate::analytics::briefing(&inp);
        assert!(
            lines
                .iter()
                .any(|l| l.area == crate::analytics::Area::Quality),
            "bugungi nuqson xulosada yo'q"
        );
        // Har bir qatorda matn va o'tish ekrani bor.
        for l in &lines {
            assert!(!l.text.is_empty());
            assert_eq!(l.screen, l.area.screen());
        }
        // Muhimlik bo'yicha tartiblangan.
        let ranks: Vec<u8> = lines
            .iter()
            .map(|l| match l.severity {
                crate::domain::Severity::Critical => 0,
                crate::domain::Severity::Major => 1,
                crate::domain::Severity::Warning => 2,
                crate::domain::Severity::Info => 3,
                crate::domain::Severity::Ok => 4,
            })
            .collect();
        assert!(ranks.windows(2).all(|w| w[0] <= w[1]));
    }

    /// Umumiy talab: har bir jadvalli ekran eksportga tayyor bo'lishi kerak.
    ///
    /// Namuna bazasida hamma modul to'ldirilgan, shuning uchun har bir
    /// ro'yxatdagi ekran bo'sh bo'lmagan jadval qaytarishi kerak. Yangi ekran
    /// qo'shilib, eksporti unutilsa — shu sinov aytadi.
    #[test]
    fn every_table_screen_can_be_exported() {
        use crate::app::Screen;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        // Nomuvofiqliklar ekran ochilganda hisoblanadi — eksport ham shu
        // ma'lumotdan chiqadi, shuning uchun tekshiruvni ishga tushiramiz.
        app.auto_check(crate::domain::IssueModule::Project);

        let screens = [
            Screen::Gantt,
            Screen::Ppr,
            Screen::AiCheck,
            Screen::Estimate,
            Screen::ExecDocs,
            Screen::Journal,
            Screen::Materials,
            Screen::Warehouse,
            Screen::Requests,
            Screen::Purchases,
            Screen::Timesheet,
            Screen::Quality,
            Screen::Safety,
            Screen::Machines,
            Screen::Deals,
            Screen::Sales,
            Screen::Analytics,
            Screen::Portfolio,
            Screen::Inspections,
            Screen::Contracts,
            Screen::Notices,
            Screen::Director,
        ];
        for s in screens {
            let table = crate::ui::export::table_of(&app, s)
                .unwrap_or_else(|| panic!("{s:?} uchun jadval yo'q"));
            assert!(!table.rows.is_empty(), "{s:?}: qator yo'q");
            assert!(!table.headers.is_empty(), "{s:?}: sarlavha yo'q");
            // Har bir qatorda sarlavha bilan bir xil ustun bo'lishi kerak —
            // aks holda Excel da ustunlar surilib ketadi.
            for (i, row) in table.rows.iter().enumerate() {
                assert_eq!(
                    row.len(),
                    table.headers.len(),
                    "{s:?}: {i}-qatorda ustun soni mos emas"
                );
            }
            // Nom yon paneldagi ekran nomi bilan bir xil.
            assert_eq!(table.name, s.label());
            // Fayl nomi xavfsiz va sanani o'z ichiga oladi.
            let file = crate::ui::export::file_name(&app, s);
            assert!(file.ends_with(".xlsx"));
            assert!(!file.contains(' '));
        }

        // Jadvali yo'q ekranda eksport ham yo'q.
        assert!(crate::ui::export::table_of(&app, Screen::Settings).is_none());
        assert!(crate::ui::export::table_of(&app, Screen::Dashboard).is_none());
    }

    /// Bo'sh bazada eksport hech narsa qaytarmaydi — bo'sh fayl yozilmaydi.
    #[test]
    fn empty_project_exports_nothing() {
        use crate::app::Screen;

        let t = TempDb::new();
        let pid =
            t.db.insert_project(&crate::model::Project {
                id: 0,
                name: "Bo'sh".into(),
                code: String::new(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: crate::model::ObjectStatus::Design,
                start_date: chrono::Local::now().date_naive(),
                planned_end: chrono::Local::now().date_naive(),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: "UZS".into(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        for s in [Screen::Warehouse, Screen::Quality, Screen::Deals] {
            assert!(
                crate::ui::export::table_of(&app, s).is_none(),
                "{s:?}: bo'sh bazada jadval qaytdi"
            );
        }
    }

    /// Umumiy talab: har bir o'zgarish jurnalga tushadi.
    #[test]
    fn every_change_reaches_the_audit_log() {
        use crate::domain::AuditAction;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        t.db.set_audit_user("Yusupov B.R. · Prorab");
        let before = t.db.audit_count();

        // Qo'shish.
        let mid = t.db.insert_material(&test_material(pid, "AU-1", 0.0));
        assert!(mid > 0);
        // O'zgartirish.
        let mut m =
            t.db.materials(pid)
                .into_iter()
                .find(|m| m.id == mid)
                .expect("material");
        m.name = "Yangi nom".into();
        assert!(t.db.update_material(&m));
        // O'chirish.
        assert!(t.db.del("material", mid));

        let log = t.db.audit_log(10);
        assert!(t.db.audit_count() >= before + 3);

        // Oxirgi uchta amal — teskari tartibda.
        assert_eq!(log[0].action, AuditAction::Delete);
        assert_eq!(log[0].table_name, "material");
        assert_eq!(log[0].row_id, mid);
        assert_eq!(log[1].action, AuditAction::Update);
        assert_eq!(log[1].table_name, "material");
        assert_eq!(log[2].action, AuditAction::Insert);
        assert_eq!(log[2].row_id, mid);

        // Kim o'zgartirgani yozilgan.
        for e in log.iter().take(3) {
            assert_eq!(e.user, "Yusupov B.R. · Prorab");
            assert!(e.at.len() >= 19, "vaqt yozilmagan: {}", e.at);
        }
    }

    /// Foydalanuvchi tanlanmagan bo'lsa jurnalda bo'sh qoladi — kim ekani
    /// o'ylab topilmaydi.
    #[test]
    fn audit_user_is_empty_when_nobody_is_chosen() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        t.db.set_audit_user("");
        t.db.insert_material(&test_material(pid, "AU-2", 0.0));
        let log = t.db.audit_log(1);
        assert_eq!(log[0].user, "");
    }

    /// Jurnalning o'zi jurnalga tushmaydi — aks holda cheksiz o'sardi.
    #[test]
    fn audit_log_does_not_log_itself() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        t.db.insert_material(&test_material(pid, "AU-3", 0.0));
        assert!(
            t.db.audit_log(500)
                .iter()
                .all(|e| e.table_name != "audit_log"),
            "jurnal o'zini yozib qo'ygan"
        );
    }

    /// Jurnal belgilangan hajmdan oshmaydi: eng eskilari olib tashlanadi.
    #[test]
    fn audit_log_is_trimmed_to_the_limit() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        for i in 0..30 {
            t.db.insert_material(&test_material(pid, &format!("AU-T{i}"), 0.0));
        }
        let before = t.db.audit_count();
        assert!(before > 20);

        let removed = t.db.trim_audit_log(20);
        assert!(removed > 0);
        assert_eq!(t.db.audit_count(), 20);
        // Eng yangilari qoladi.
        let log = t.db.audit_log(20);
        assert_eq!(log.len(), 20);
        assert!(log[0].id > log[19].id);

        // Chegaradan kam bo'lsa hech narsa o'chirilmaydi.
        assert_eq!(t.db.trim_audit_log(100), 0);
        assert_eq!(t.db.audit_count(), 20);
    }

    /// TZ XII.19: narx tarixi kirimlardan chiqadi va o'zgarishni ko'rsatadi.
    #[test]
    fn price_history_shows_the_change_between_deliveries() {
        use crate::checks::price_history;
        use crate::domain::{MoveKind, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "PH-1", 0.0));
        let day = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).expect("sana");
        let mv = |days: i64, kind: MoveKind, qty: f64, price: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: day + chrono::Duration::days(days),
                kind,
                qty,
                price,
                document: format!("TTN-{days}"),
                counterparty: "Yetkazuvchi".into(),
                task_id: None,
                note: String::new(),
                warehouse_id: None,
                batch_id: None,
            });
        };
        // Tartib ataylab aralash kiritiladi — hisob sanaga qarab saralaydi.
        mv(10, MoveKind::In, 5.0, 1_200.0);
        mv(0, MoveKind::In, 10.0, 1_000.0);
        // Narxsiz kirim va chiqim tarixga kirmaydi.
        mv(5, MoveKind::In, 3.0, 0.0);
        mv(7, MoveKind::Out, 2.0, 900.0);

        let hist = price_history(mid, &t.db.stock_moves(pid));
        assert_eq!(hist.len(), 2, "faqat narxi bor kirimlar");
        assert_eq!(hist[0].price, 1_000.0);
        assert_eq!(
            hist[0].change_pct, None,
            "birinchi kirimda solishtiruvchi yo'q"
        );
        assert_eq!(hist[1].price, 1_200.0);
        assert_eq!(hist[1].change_pct, Some(20.0));
        assert_eq!(hist[1].document, "TTN-10");
    }

    /// TZ XII.29–30: material zanjiri hujjatgacha boradi.
    #[test]
    fn material_trace_reaches_the_exec_doc() {
        use crate::checks::material_trace;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let moves = t.db.stock_moves(pid);
        let docs = t.db.exec_docs(pid);
        let quality = t.db.quality_checks(pid);

        // Namunada ishga berilgan material bor.
        let mid = moves
            .iter()
            .find(|m| m.kind == crate::domain::MoveKind::Out && m.task_id.is_some())
            .map(|m| m.material_id)
            .expect("ishga berilgan material");

        let tr = material_trace(mid, &moves, &docs, &quality);
        assert!(tr.received > 0.0, "kirim yo'q");
        assert!(tr.issued > 0.0, "chiqim yo'q");
        assert!(!tr.tasks.is_empty(), "ish bog'lanmagan");
        assert!(!tr.suppliers.is_empty(), "yetkazib beruvchi yo'q");

        // Zanjir bo'lmagan materialda hammasi bo'sh, lekin xato emas.
        let free = t.db.insert_material(&test_material(pid, "TR-0", 0.0));
        let empty = material_trace(free, &moves, &docs, &quality);
        assert_eq!(empty.received, 0.0);
        assert!(empty.tasks.is_empty());
        assert!(empty.docs.is_empty());
    }

    /// TZ XII.36: brak ombordan chiqmagan bo'lsa ochiq masala bo'lib qoladi.
    #[test]
    fn defect_stays_open_until_it_leaves_the_warehouse() {
        use crate::checks::defect_lines;
        use crate::domain::{MoveKind, QualityKind, QualityResult, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "BR-1", 0.0));
        let today = chrono::Local::now().date_naive();

        t.db.insert_quality(&test_check(
            pid,
            QualityKind::Input,
            None,
            QualityResult::Fail,
            "Sertifikat mos emas",
            None,
            None,
        ));
        // Tekshiruvni materialga bog'laymiz.
        let mut q =
            t.db.quality_checks(pid)
                .into_iter()
                .next()
                .expect("tekshiruv");
        q.material_id = Some(mid);
        q.result = QualityResult::Fail;
        t.db.update_quality(&q);

        let lines = defect_lines(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.quality_checks(pid),
        );
        let d = lines.iter().find(|d| d.material_id == mid).expect("brak");
        assert_eq!(d.rejected, 1);
        assert!(d.unresolved, "ombordan chiqmagan brak ochiq bo'lishi kerak");

        // Yetkazib beruvchiga qaytargach — masala yopiladi.
        t.db.insert_stock_move(&StockMove {
            id: 0,
            project_id: pid,
            material_id: mid,
            date: today,
            kind: MoveKind::ToSupplier,
            qty: 5.0,
            price: 0.0,
            document: "V-01".into(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
            warehouse_id: None,
            batch_id: None,
        });
        let lines = defect_lines(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.quality_checks(pid),
        );
        let d = lines.iter().find(|d| d.material_id == mid).expect("brak");
        assert_eq!(d.returned, 5.0);
        assert!(!d.unresolved);
    }

    /// TZ XII.28: yaqinda boshlanadigan ishga material yetmasa ogohlantiriladi.
    #[test]
    fn readiness_warns_before_the_work_starts() {
        use crate::checks::{readiness, stock_balances};
        use crate::domain::MaterialNorm;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let mid = t.db.insert_material(&test_material(pid, "RD-1", 0.0));

        // Ish besh kundan keyin boshlanadi, hajmi 100.
        let mut tasks = t.db.tasks(pid).unwrap_or_default();
        let task = tasks.first_mut().expect("ish");
        task.plan_start = today + chrono::Duration::days(5);
        task.fact_start = None;
        task.progress = 0.0;
        task.volume = 100.0;
        t.db.update_task(task).unwrap();
        let tid = task.id;

        // Norma: bir birlikka 2 → 200 kerak, omborda esa hech narsa yo'q.
        t.db.insert_material_norm(&MaterialNorm {
            id: 0,
            project_id: pid,
            task_id: tid,
            material_id: mid,
            per_unit: 2.0,
            tolerance: 0.0,
            note: String::new(),
        });

        let stock = stock_balances(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.reservations(pid),
            today,
        );
        let lines = readiness(
            &t.db.material_norms(pid),
            &t.db.tasks(pid).unwrap_or_default(),
            &stock,
            today,
            14,
        );
        let l = lines
            .iter()
            .find(|l| l.task_id == tid && l.material_id == mid)
            .expect("ogohlantirish");
        assert_eq!(l.needed, 200.0, "butun hajmga qaraladi");
        assert_eq!(l.available, 0.0);
        assert_eq!(l.short, 200.0);
        assert_eq!(l.days_left, 5);

        // Uzoqdagi ish hozircha tekshirilmaydi.
        let far = readiness(
            &t.db.material_norms(pid),
            &t.db.tasks(pid).unwrap_or_default(),
            &stock,
            today,
            2,
        );
        assert!(!far.iter().any(|l| l.task_id == tid && l.material_id == mid));
    }

    /// Namunada analoglar bor: bittasi tasdiqlangan, bittasi yo'q.
    #[test]
    fn demo_has_approved_and_pending_alternatives() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let alts = t.db.material_alts(pid);
        assert_eq!(alts.len(), 2);
        assert!(alts.iter().any(|a| a.approved()));
        assert!(alts.iter().any(|a| !a.approved()));

        // Smeta va spetsifikatsiya havolalari to'ldirilgan.
        let materials = t.db.materials(pid);
        assert!(materials.iter().any(|m| !m.estimate_code.is_empty()));
        assert!(materials.iter().any(|m| !m.spec_ref.is_empty()));
        assert!(materials.iter().any(|m| !m.special.is_empty()));
    }

    /// TZ III.17–20: ustama, foyda va QQS ketma-ket qo'yiladi.
    #[test]
    fn estimate_totals_follow_the_order() {
        use crate::domain::Estimate;

        let e = Estimate {
            id: 1,
            project_id: 1,
            name: String::new(),
            currency: "UZS".into(),
            declared_total: 0.0,
            overhead_pct: 10.0,
            profit_pct: 5.0,
            vat_pct: 12.0,
            added_at: String::new(),
        };
        let x = e.totals(1_000.0);
        assert_eq!(x.direct, 1_000.0);
        assert_eq!(x.overhead, 100.0, "ustama to'g'ridan-to'g'ri xarajatdan");
        assert_eq!(x.profit, 55.0, "foyda ustama bilan birga summadan");
        assert_eq!(x.before_vat, 1_155.0);
        assert_eq!(x.vat, 138.6);
        assert!((x.total - 1_293.6).abs() < 0.001);

        // Koeffitsiyentsiz smetada yakuniy summa to'g'ridan-to'g'ri xarajatga teng.
        let zero = Estimate {
            overhead_pct: 0.0,
            profit_pct: 0.0,
            vat_pct: 0.0,
            ..e
        };
        assert_eq!(zero.totals(1_000.0).total, 1_000.0);
    }

    /// Koeffitsiyent ko'rsatilmagani va haddan tashqari kattasi topiladi.
    #[test]
    fn estimate_issues_catch_missing_and_extreme_coefficients() {
        use crate::checks::{estimate_issues, EstimateIssue, OVERHEAD_LIMIT};
        use crate::domain::Estimate;

        let base = Estimate {
            id: 1,
            project_id: 1,
            name: String::new(),
            currency: "UZS".into(),
            declared_total: 0.0,
            overhead_pct: 0.0,
            profit_pct: 0.0,
            vat_pct: 0.0,
            added_at: String::new(),
        };
        let issues = estimate_issues(&base, &[]);
        assert!(issues.contains(&EstimateIssue::NoOverhead));
        assert!(issues.contains(&EstimateIssue::NoProfit));
        assert!(issues.contains(&EstimateIssue::NoVat));

        // Normal koeffitsiyentlarda kamchilik yo'q.
        let ok = Estimate {
            overhead_pct: 14.0,
            profit_pct: 8.0,
            vat_pct: 12.0,
            ..base.clone()
        };
        assert!(estimate_issues(&ok, &[]).is_empty());

        // Haddan tashqari ustama belgilanadi.
        let big = Estimate {
            overhead_pct: OVERHEAD_LIMIT + 1.0,
            ..ok.clone()
        };
        assert!(estimate_issues(&big, &[]).iter().any(|i| matches!(
            i,
            EstimateIssue::Suspicious {
                name: "overhead",
                ..
            }
        )));
    }

    /// Hujjatdagi summa hisoblangandan farq qilsa aytiladi.
    #[test]
    fn estimate_total_mismatch_is_reported() {
        use crate::checks::{estimate_issues, EstimateIssue};
        use crate::domain::{Estimate, EstimateItem};

        let e = Estimate {
            id: 7,
            project_id: 1,
            name: String::new(),
            currency: "UZS".into(),
            // Hisob bo'yicha 1000, hujjatda esa 1500.
            declared_total: 1_500.0,
            overhead_pct: 0.0,
            profit_pct: 0.0,
            vat_pct: 0.0,
            added_at: String::new(),
        };
        let items = vec![EstimateItem {
            id: 1,
            estimate_id: 7,
            pos: 1,
            section: crate::model::Section::Kj,
            code: String::new(),
            name: String::new(),
            unit: String::new(),
            qty: 10.0,
            price: 100.0,
            cost: 1_000.0,
            task_id: None,
            note: String::new(),
        }];
        assert!(estimate_issues(&e, &items)
            .iter()
            .any(|i| matches!(i, EstimateIssue::TotalMismatch { .. })));

        // Yaxlitlash farqi kamchilik emas.
        let close = Estimate {
            declared_total: 1_000.4,
            ..e
        };
        assert!(!estimate_issues(&close, &items)
            .iter()
            .any(|i| matches!(i, EstimateIssue::TotalMismatch { .. })));
    }

    /// TZ III.26: smeta va GPR bog'lanishi ikki tomondan tekshiriladi.
    #[test]
    fn estimate_coverage_looks_both_ways() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let tasks = t.db.tasks(pid).unwrap_or_default();
        let est = t.db.estimates(pid).into_iter().next().expect("smeta");
        let items = t.db.estimate_items(est.id);

        let c = crate::checks::estimate_coverage(&items, &tasks);
        assert!(c.linked_pct > 0.0, "namunada bog'lanish yo'q");
        assert!(
            c.linked_pct < 100.0,
            "namunada bog'lanmagan pozitsiya bo'lishi kerak"
        );
        assert!(c.free_cost > 0.0);
        assert!(!c.free_tasks.is_empty(), "smetasiz ish yo'q");

        // Bo'sh smetada bo'linish ham bo'lmaydi.
        let empty = crate::checks::estimate_coverage(&[], &tasks);
        assert_eq!(empty.linked_pct, 0.0);
        assert!(empty.free_items.is_empty());
    }

    /// TZ III.23: smeta va byudjet bo'lim kesimida solishtiriladi.
    #[test]
    fn estimate_is_compared_with_the_budget() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let est = t.db.estimates(pid).into_iter().next().expect("smeta");
        let items = t.db.estimate_items(est.id);
        let lines = crate::checks::estimate_vs_budget(&items, &t.db.purchase_budgets(pid));

        assert!(!lines.is_empty());
        for l in &lines {
            assert!((l.gap - (l.budget - l.estimate)).abs() < 0.01);
        }
        // Byudjeti yo'q bo'limlar ham chiqadi.
        assert!(lines.iter().any(|l| l.budget == 0.0 && l.estimate > 0.0));
    }

    /// TZ III.21: ikki variant bo'lim kesimida solishtiriladi.
    #[test]
    fn two_estimates_are_compared_by_section() {
        use crate::domain::{Estimate, EstimateItem};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let first = t.db.estimates(pid).into_iter().next().expect("smeta");

        // Ikkinchi variant: bir xil bo'lim, boshqa narx.
        let second = t.db.insert_estimate(&Estimate {
            id: 0,
            project_id: pid,
            name: "2-variant".into(),
            currency: "UZS".into(),
            declared_total: 0.0,
            overhead_pct: 14.0,
            profit_pct: 8.0,
            vat_pct: 12.0,
            added_at: String::new(),
        });
        t.db.insert_estimate_item(&EstimateItem {
            id: 0,
            estimate_id: second,
            pos: 1,
            section: crate::model::Section::Kj,
            code: "E6-1-1".into(),
            name: "Sinov".into(),
            unit: "m3".into(),
            qty: 10.0,
            price: 100.0,
            cost: 1_000.0,
            task_id: None,
            note: String::new(),
        });

        let mut all = t.db.estimate_items(first.id);
        all.extend(t.db.estimate_items(second));
        let diff = crate::checks::compare_estimates(&all, first.id, second);
        let kj = diff
            .iter()
            .find(|l| l.section == crate::model::Section::Kj)
            .expect("KJ bo'limi");
        assert!(kj.left > 0.0);
        assert_eq!(kj.right, 1_000.0);
        assert_eq!(kj.diff, kj.right - kj.left);
        assert!(kj.diff < 0.0, "ikkinchi variant arzonroq");
    }

    /// TZ VII.3: kalendar faqat o'tkazilmagan tekshiruvlarni ko'rsatadi va
    /// kunlarga guruhlaydi.
    #[test]
    fn inspection_calendar_groups_open_checks_by_day() {
        use crate::checks::inspection_calendar;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let list = t.db.inspections(pid);
        assert!(!list.is_empty(), "namunada tekshiruv yo'q");

        let days = inspection_calendar(&list, today);
        assert!(!days.is_empty());
        // Kunlar o'sish tartibida.
        for w in days.windows(2) {
            assert!(w[0].date < w[1].date, "kunlar tartibi buzilgan");
        }
        // Kalendardagi har bir yozuv haqiqatan ochiq.
        let open: std::collections::HashSet<i64> =
            list.iter().filter(|x| x.open()).map(|x| x.id).collect();
        let in_calendar: usize = days.iter().map(|d| d.ids.len()).sum();
        assert_eq!(in_calendar, open.len());
        for d in &days {
            for id in &d.ids {
                assert!(open.contains(id), "yopilgan tekshiruv kalendarda");
            }
            assert_eq!(d.overdue, d.date < today);
        }
    }

    /// Muddati o'tgan tekshiruv yakunda alohida sanaladi.
    #[test]
    fn overdue_inspection_is_counted_separately() {
        use crate::checks::inspection_summary;
        use crate::domain::{Inspection, InspectionKind, InspectionResult};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let mk = |planned: NaiveDate, done: Option<NaiveDate>, result| Inspection {
            id: 0,
            project_id: 1,
            task_id: None,
            kind: InspectionKind::Hidden,
            number: String::new(),
            planned,
            done,
            requested_by: String::new(),
            inspector: String::new(),
            place: String::new(),
            result,
            deadline: None,
            fixed_at: None,
            note: String::new(),
        };
        let day = |d: i64| today + chrono::Duration::days(d);
        let list = vec![
            mk(day(-3), None, InspectionResult::Waiting), // muddati o'tgan
            mk(day(0), None, InspectionResult::Waiting),  // bugun
            mk(day(4), None, InspectionResult::Waiting),  // shu hafta
            mk(day(20), None, InspectionResult::Waiting), // uzoq
            mk(day(-5), Some(day(-5)), InspectionResult::Pass),
            mk(day(-6), Some(day(-6)), InspectionResult::Fail),
        ];
        let s = inspection_summary(&list, today);
        assert_eq!(s.overdue, 1);
        assert_eq!(s.today, 1);
        assert_eq!(s.week, 1, "yigirma kundan keyingisi haftaga kirmaydi");
        assert_eq!(s.passed, 1);
        assert_eq!(s.open_defects, 1);
        assert_eq!(s.total, 6);
    }

    /// TZ VII.12: beton mustahkamligi hisoblanmaydi — laboratoriya natijasi
    /// talab bilan solishtiriladi.
    #[test]
    fn concrete_strength_is_compared_not_invented() {
        use crate::domain::ConcreteTest;

        let poured = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let base = ConcreteTest {
            id: 0,
            project_id: 1,
            inspection_id: None,
            task_id: None,
            sample: String::new(),
            grade: "B25".into(),
            structure: String::new(),
            poured,
            age_days: 28,
            required: 25.0,
            actual: None,
            lab: String::new(),
            note: String::new(),
        };
        // Natija yo'q — hukm ham yo'q.
        assert_eq!(base.passed(), None);
        assert_eq!(base.pct(), None);
        assert_eq!(base.test_date(), poured + chrono::Duration::days(28));

        let ok = ConcreteTest {
            actual: Some(27.5),
            ..base.clone()
        };
        assert_eq!(ok.passed(), Some(true));
        assert!((ok.pct().unwrap() - 110.0).abs() < 0.001);

        let bad = ConcreteTest {
            actual: Some(24.9),
            ..base.clone()
        };
        assert_eq!(bad.passed(), Some(false));

        // Aynan talab darajasi — o'tgan hisoblanadi.
        let exact = ConcreteTest {
            actual: Some(25.0),
            ..base
        };
        assert_eq!(exact.passed(), Some(true));
    }

    /// Beton yakunida kutilayotgan natijalar alohida sanaladi.
    #[test]
    fn concrete_summary_separates_pending_results() {
        use crate::checks::concrete_summary;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let list = t.db.concrete_tests(pid);
        let s = concrete_summary(&list, today);

        assert_eq!(s.total, list.len());
        assert_eq!(s.tested, list.iter().filter(|x| x.actual.is_some()).count());
        assert_eq!(s.tested, s.passed + s.failed);
        assert!(
            s.failed > 0,
            "namunada talabga yetmagan sinov bo'lishi kerak"
        );
        assert!(s.avg_pct > 0.0);
        assert!(s.worst_pct < 100.0, "eng past natija talabdan past");
    }

    /// TZ VII.14: dopuskdan chiqqan nuqtalar eng katta chetlanishdan boshlanadi.
    #[test]
    fn geodesy_issues_are_sorted_by_excess() {
        use crate::checks::geodesy_issues;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let points = t.db.geodesy_points(pid);
        assert!(!points.is_empty());

        let bad = geodesy_issues(&points);
        assert!(!bad.is_empty(), "namunada dopuskdan chiqqan nuqta yo'q");
        for p in &bad {
            assert!(!p.within(), "dopusk ichidagi nuqta ro'yxatga tushdi");
        }
        // Ortiqcha chetlanish kamayish tartibida.
        for w in bad.windows(2) {
            let ex = |p: &crate::domain::GeodesyPoint| p.deviation().abs() - p.tolerance;
            assert!(ex(w[0]) >= ex(w[1]), "tartib buzilgan");
        }
        // Dopusk chegarasidagi nuqta muammo emas.
        let edge = crate::domain::GeodesyPoint {
            tolerance: 8.0,
            design: 0.0,
            fact: 8.0,
            ..points[0].clone()
        };
        assert!(edge.within());
    }

    /// TZ VII.34: kunlik hisobot bazadagi yozuvlardan yig'iladi.
    #[test]
    fn supervision_day_report_is_built_from_records() {
        use crate::checks::supervision_day;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let inspections = t.db.inspections(pid);
        let day = inspections
            .iter()
            .find_map(|x| x.done)
            .expect("o'tkazilgan tekshiruv");

        let r = supervision_day(
            day,
            &inspections,
            &t.db.issues(pid),
            &t.db.quality_checks(pid),
            &t.db.exec_docs(pid),
            &t.db.concrete_tests(pid),
            &t.db.geodesy_points(pid),
        );
        assert_eq!(r.date, day);
        assert!(r.inspections > 0);
        assert!(!r.is_empty());
        assert_eq!(
            r.inspections,
            inspections.iter().filter(|x| x.done == Some(day)).count()
        );

        // Hech narsa bo'lmagan kun bo'sh hisobot beradi.
        let quiet = day - chrono::Duration::days(3650);
        assert!(supervision_day(
            quiet,
            &inspections,
            &t.db.issues(pid),
            &t.db.quality_checks(pid),
            &t.db.exec_docs(pid),
            &t.db.concrete_tests(pid),
            &t.db.geodesy_points(pid),
        )
        .is_empty());
    }

    /// Tekshiruv yozuvi bazadan o'zgarishsiz qaytadi.
    #[test]
    fn inspection_round_trips() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut x = t.db.inspections(pid).remove(0);

        x.result = crate::domain::InspectionResult::Conditional;
        x.inspector = "Sinov".into();
        x.fixed_at = Some(chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert!(t.db.update_inspection(&x));

        let back =
            t.db.inspections(pid)
                .into_iter()
                .find(|y| y.id == x.id)
                .expect("yozuv yo'qoldi");
        assert_eq!(back.result, crate::domain::InspectionResult::Conditional);
        assert_eq!(back.inspector, "Sinov");
        assert_eq!(back.fixed_at, x.fixed_at);

        assert!(t.db.delete_inspection(x.id));
        assert!(!t.db.inspections(pid).iter().any(|y| y.id == x.id));
    }

    /// Oxirgi tanlangan obyekt keyingi ochilishda tiklanadi.
    #[test]
    fn last_project_is_remembered() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let other =
            t.db.projects()
                .unwrap()
                .into_iter()
                .find(|p| p.id != pid)
                .expect("ikkinchi obyekt");

        // Birinchi ochilish: sozlama yo'q, alifbo bo'yicha birinchisi.
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        let first = app.current.expect("obyekt tanlanmadi");

        // Boshqasiga o'tamiz va ilovani qaytadan ochamiz.
        app.select_project(if first == other.id { pid } else { other.id });
        let chosen = app.current;
        drop(app);

        let again = crate::app::App::new(Db::open(&t.path).unwrap());
        assert_eq!(again.current, chosen, "oxirgi obyekt tiklanmadi");
        assert_ne!(again.current, Some(first), "eski tanlov qaytdi");
    }

    /// TZ VIII.11-12: faqat tasdiqlangan o'zgarish shartnoma summasini o'zgartiradi.
    #[test]
    fn only_approved_changes_move_the_contract_sum() {
        use crate::checks::contract_state;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let contracts = t.db.contracts(pid);
        let changes = t.db.contract_changes(pid);
        let stages = t.db.payment_stages(pid);

        let general = contracts
            .iter()
            .find(|c| c.kind == crate::domain::ContractKind::General)
            .expect("bosh pudrat");
        let st = contract_state(general, &changes, &stages, today);

        assert_eq!(st.base, general.sum);
        assert!(st.approved_changes > 0.0, "tasdiqlangan o'zgarish yo'q");
        assert!(st.pending_changes > 0.0, "kutayotgan o'zgarish yo'q");
        // Amaldagi summa faqat tasdiqlanganini o'z ichiga oladi.
        assert!((st.current - (st.base + st.approved_changes)).abs() < 0.01);
        assert!(
            st.current < st.base + st.approved_changes + st.pending_changes,
            "kutayotgan o'zgarish summaga qo'shilib ketgan"
        );
        assert!(st.approved_days > 0, "tasdiqlangan muddat surilishi yo'q");
        assert!(st.change_pct() > 0.0);
    }

    /// Kamaytiruvchi o'zgarish summani pasaytiradi.
    #[test]
    fn a_reduction_lowers_the_contract_sum() {
        use crate::checks::contract_state;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let contracts = t.db.contracts(pid);
        let sub = contracts
            .iter()
            .find(|c| c.kind == crate::domain::ContractKind::Sub)
            .expect("subpudrat");

        let st = contract_state(
            sub,
            &t.db.contract_changes(pid),
            &t.db.payment_stages(pid),
            today,
        );
        assert!(st.approved_changes < 0.0, "kamaytirish yo'q");
        assert!(st.current < st.base);
        assert!(st.change_pct() < 0.0);
    }

    /// TZ VIII.30: to'lov intizomi qarz va kechikishni ajratadi.
    #[test]
    fn payment_discipline_separates_debt_from_delay() {
        use crate::checks::payment_discipline;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let stages = t.db.payment_stages(pid);
        let d = payment_discipline(&stages, today);

        assert_eq!(d.stages, stages.len());
        assert!(d.closed > 0);
        assert!(d.paid > 0.0);
        assert!(
            d.paid < d.planned,
            "hammasi to'langan bo'lsa sinov ma'nosiz"
        );
        // Muddati o'tgan qarz bor va u to'lanmagan qoldiqdan oshmaydi.
        assert!(d.debt > 0.0, "muddati o'tgan to'lov yo'q");
        assert!(d.debt <= d.planned - d.paid + 0.01);
        assert!(d.max_delay > 0);
        // O'rtacha kechikish faqat to'langan bosqichlardan — namunada erta
        // to'langanlar bor, shuning uchun u nolga teng bo'lishi mumkin.
        assert!(d.avg_delay >= 0.0);
        // Kelgusi muddatlar alohida sanaladi.
        assert!(d.due_soon >= 0.0);
    }

    /// To'lov bosqichining chetki holatlari.
    #[test]
    fn payment_stage_edges_are_handled() {
        use crate::domain::PaymentStage;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let base = PaymentStage {
            id: 0,
            project_id: 1,
            contract_id: None,
            number: String::new(),
            basis: String::new(),
            due: today - chrono::Duration::days(10),
            amount: 1_000.0,
            paid: 0.0,
            paid_at: None,
            note: String::new(),
        };
        assert!(base.overdue(today));
        assert_eq!(base.delay_days(today), 10);
        assert_eq!(base.left(), 1_000.0);

        // To'liq to'langan bosqich muddati o'tgan hisoblanmaydi.
        let closed = PaymentStage {
            paid: 1_000.0,
            paid_at: Some(today),
            ..base.clone()
        };
        assert!(closed.closed());
        assert!(!closed.overdue(today));
        assert_eq!(closed.delay_days(today), 0);
        assert_eq!(closed.left(), 0.0);

        // Ortiqcha to'lov qoldiqni manfiy qilmaydi.
        let over = PaymentStage {
            paid: 1_200.0,
            ..base
        };
        assert_eq!(over.left(), 0.0);
    }

    /// TZ VIII.33: haftalik hisobot yozuvlardan yig'iladi.
    #[test]
    fn week_report_is_built_from_records() {
        use crate::checks::week_report;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let r = week_report(
            today,
            &t.db.tasks(pid).unwrap_or_default(),
            &t.db.journal(pid),
            &t.db.inspections(pid),
            &t.db.issues(pid),
            &t.db.exec_docs(pid),
            &t.db.payment_stages(pid),
            &t.db.contract_changes(pid),
            &t.db.work_acceptances(pid),
        );

        assert_eq!(r.to, today);
        assert_eq!((r.to - r.from).num_days(), 7);
        // Bajarilish hafta davomida kamaymaydi.
        assert!(r.progress_end >= r.progress_start - 0.001);
        assert!(r.progress_end > 0.0 && r.progress_end <= 100.0);
        assert!(r.changes_pending > 0, "namunada kutayotgan o'zgarish yo'q");
        assert!(
            r.acceptances_pending > 0,
            "namunada kutayotgan qabul hujjati yo'q"
        );
    }

    /// TZ VIII.21: qabul hujjati qaror kutadi, keyin yopiladi.
    #[test]
    fn work_acceptance_round_trips() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let list = t.db.work_acceptances(pid);
        assert!(!list.is_empty());
        assert!(list.iter().any(|x| x.pending()), "kutayotgani yo'q");
        assert!(
            list.iter()
                .any(|x| x.state == crate::domain::AcceptState::Rejected),
            "rad etilgani yo'q"
        );
        // Rad etilgan hujjatda sabab yozilgan bo'lishi kerak.
        for x in list
            .iter()
            .filter(|x| x.state == crate::domain::AcceptState::Rejected)
        {
            assert!(!x.comment.is_empty(), "rad etish sababi yo'q");
        }

        let mut x = list
            .into_iter()
            .find(|x| x.pending())
            .expect("kutayotgan hujjat");
        x.state = crate::domain::AcceptState::Accepted;
        x.decided_at = Some(t.db.projects().unwrap()[0].start_date);
        x.decided_by = "Sinov".into();
        assert!(t.db.update_work_acceptance(&x));

        let back =
            t.db.work_acceptances(pid)
                .into_iter()
                .find(|y| y.id == x.id)
                .expect("yozuv yo'qoldi");
        assert_eq!(back.state, crate::domain::AcceptState::Accepted);
        assert_eq!(back.decided_by, "Sinov");
        assert!(!back.pending());
    }

    /// Shartnoma avansi va kafolat ushlanmasi foizdan hisoblanadi.
    #[test]
    fn contract_advance_and_retention_follow_the_percent() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let c =
            t.db.contracts(pid)
                .into_iter()
                .find(|c| c.kind == crate::domain::ContractKind::General)
                .expect("bosh pudrat");

        assert!((c.advance() - c.sum * c.advance_pct / 100.0).abs() < 0.01);
        assert!((c.retention() - c.sum * c.retention_pct / 100.0).abs() < 0.01);
        assert!(c.advance() > 0.0);
        assert!(c.retention() > 0.0);
    }

    /// Yozuv o'zgarishi baza revizyasini oshiradi — interfeys shu son
    /// bo'yicha hisobni qayta bajarish kerakligini biladi.
    #[test]
    fn writes_bump_the_revision() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let start = t.db.revision();
        assert!(start > 0, "namuna yozuvlari hisoblanmadi");

        // O'qish revizyani o'zgartirmaydi.
        let _ = t.db.inspections(pid);
        let _ = t.db.contracts(pid);
        assert_eq!(t.db.revision(), start, "o'qish revizyani oshirdi");

        // Yozuv oshiradi.
        let mut x = t.db.inspections(pid).remove(0);
        x.note = "sinov".into();
        assert!(t.db.update_inspection(&x));
        let after_update = t.db.revision();
        assert!(after_update > start);

        // O'chirish ham oshiradi.
        assert!(t.db.delete_inspection(x.id));
        assert!(t.db.revision() > after_update);
    }

    /// Namunani tozalash: obyektlar ham, ularning yozuvlari ham ketadi va
    /// keyingi ochilishda qaytib kelmaydi.
    #[test]
    fn demo_can_be_cleared_and_does_not_return() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        assert_eq!(t.db.project_count().unwrap(), 2);
        assert!(!t.db.tasks(pid).unwrap().is_empty());

        // Foydalanuvchining o'z obyekti tegilmasligi kerak.
        let mine =
            t.db.insert_project(&crate::model::Project {
                id: 0,
                name: "Mening obyektim".into(),
                code: "MY-1".into(),
                address: String::new(),
                object_type: String::new(),
                floors: 0,
                area_total: 0.0,
                status: crate::model::ObjectStatus::Design,
                start_date: chrono::Local::now().date_naive(),
                planned_end: chrono::Local::now().date_naive(),
                contract_sum: 0.0,
                paid_total: 0.0,
                currency: "UZS".into(),
                funding_source: String::new(),
                notes: String::new(),
            })
            .unwrap();

        assert_eq!(t.db.clear_demo().unwrap(), 2);
        let left = t.db.projects().unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].id, mine, "foydalanuvchi obyekti o'chib ketdi");

        // Namunaning yozuvlari ham qoldiqsiz ketadi.
        assert!(t.db.tasks(pid).unwrap().is_empty());
        assert!(t.db.inspections(pid).is_empty());
        assert!(t.db.contracts(pid).is_empty());
        assert!(t.db.journal(pid).is_empty());

        // Bayroq qo'yilgan: bo'sh bazada ham namuna qayta yaratilmaydi.
        assert!(t.db.demo_cleared());
    }

    /// TZ X.4-6: reja qoldiq, yo'ldagi buyurtma va normativ ehtiyojdan
    /// hisoblanadi — qo'lda tuzilmaydi.
    #[test]
    fn purchase_plan_subtracts_stock_and_orders() {
        use crate::checks::purchase_plan;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let materials = t.db.materials(pid);
        let moves = t.db.stock_moves(pid);
        let stock =
            crate::checks::stock_balances(&materials, &moves, &t.db.reservations(pid), today);
        let plan = purchase_plan(
            &materials,
            &stock,
            &t.db.purchases(pid),
            &t.db.requests(pid),
            &t.db.material_norms(pid),
            &t.db.tasks(pid).unwrap_or_default(),
            today,
            45,
        );

        // Rejada faqat haqiqatan yetishmayotgani qoladi.
        for l in &plan {
            assert!(l.to_buy > 0.0, "nol miqdor rejaga tushdi");
            let target = l.needed_for_tasks.max(l.min_stock);
            assert!(
                (l.to_buy - (target - l.available - l.ordered)).abs() < 0.001,
                "hisob mos emas"
            );
            assert!(l.cost >= 0.0);
        }
        // Muddati yaqinlari oldinda turadi.
        let dates: Vec<_> = plan.iter().filter_map(|l| l.need_by).collect();
        for w in dates.windows(2) {
            assert!(w[0] <= w[1], "muddat tartibi buzilgan");
        }
    }

    /// Qoldiq yetarli bo'lsa material rejaga tushmaydi.
    #[test]
    fn material_with_enough_stock_stays_out_of_the_plan() {
        use crate::checks::{purchase_plan, StockLine};
        use crate::domain::Material;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let m = Material {
            id: 7,
            project_id: 1,
            code: "M-1".into(),
            name: "Sinov".into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock: 10.0,
            price: 1_000.0,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let line = |available: f64| StockLine {
            material_id: 7,
            balance: available,
            incoming: 0.0,
            outgoing: 0.0,
            written_off: 0.0,
            returned: 0.0,
            reserved: 0.0,
            available,
            ..Default::default()
        };

        // Qoldiq minimal zaxiradan katta — reja bo'sh.
        let plan = purchase_plan(
            std::slice::from_ref(&m),
            &[line(12.0)],
            &[],
            &[],
            &[],
            &[],
            today,
            45,
        );
        assert!(plan.is_empty());

        // Qoldiq kam — rejada aynan yetishmaydigan miqdor.
        let plan = purchase_plan(&[m], &[line(4.0)], &[], &[], &[], &[], today, 45);
        assert_eq!(plan.len(), 1);
        assert!((plan[0].to_buy - 6.0).abs() < 0.001);
        assert!((plan[0].cost - 6_000.0).abs() < 0.001);
    }

    /// TZ X.41: risk belgilari dalil bilan chiqadi.
    #[test]
    fn procurement_risks_are_found_with_evidence() {
        use crate::checks::{procurement_risks, ProcurementRisk};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let purchases = t.db.purchases(pid);
        let risks = procurement_risks(&purchases, &t.db.quotes(pid), &t.db.materials(pid));

        assert!(!risks.is_empty(), "namunada risk belgisi topilmadi");
        // Taklifsiz xarid — namunada bor.
        assert!(
            risks
                .iter()
                .any(|r| matches!(r, ProcurementRisk::NoQuotes { .. })),
            "taklifsiz xarid belgilanmadi"
        );
        // Har bir belgida son bor: bo'sh ogohlantirish foydasiz.
        for r in &risks {
            match r {
                ProcurementRisk::SupplierShare { supplier, pct } => {
                    assert!(!supplier.is_empty());
                    assert!(*pct > crate::checks::SUPPLIER_SHARE_LIMIT);
                }
                ProcurementRisk::NoQuotes { number, amount } => {
                    assert!(!number.is_empty());
                    assert!(*amount > 0.0);
                }
                ProcurementRisk::HighPrice { over_pct, .. } => {
                    assert!(*over_pct > crate::checks::PRICE_OVER_LIMIT);
                }
                ProcurementRisk::TooManyUrgent { count, pct } => {
                    assert!(*count > 0);
                    assert!(*pct > crate::checks::URGENT_SHARE_LIMIT);
                }
            }
        }

        // Xarid bo'lmasa belgi ham bo'lmaydi.
        assert!(procurement_risks(&[], &[], &[]).is_empty());
    }

    /// TZ X.46: xaridchi kesimidagi yakun.
    #[test]
    fn buyer_stats_are_grouped_by_person() {
        use crate::checks::buyer_stats;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let purchases = t.db.purchases(pid);
        let stats = buyer_stats(&purchases, &t.db.quotes(pid), today);

        assert!(!stats.is_empty(), "namunada xaridchi ko'rsatilmagan");
        // Jami summa xaridlar summasidan oshmaydi.
        let total: f64 = purchases.iter().map(|p| p.amount()).sum();
        let counted: f64 = stats.iter().map(|s| s.amount).sum();
        assert!(counted <= total + 0.01);
        for s in &stats {
            assert!(!s.buyer.is_empty(), "nomsiz xaridchi ro'yxatga tushdi");
            assert!(s.purchases > 0);
            assert!(s.on_time_pct >= 0.0 && s.on_time_pct <= 100.0);
            assert!(s.with_quotes_pct >= 0.0 && s.with_quotes_pct <= 100.0);
        }
    }

    /// Xaridning ishga va shartnomaga bog'lanishi bazadan qaytadi.
    #[test]
    fn purchase_keeps_its_task_and_buyer() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut p = t.db.purchases(pid).remove(0);

        let task = t.db.tasks(pid).unwrap().remove(0).id;
        p.task_id = Some(task);
        p.buyer = "Sinov".into();
        p.urgent = true;
        assert!(t.db.update_purchase(&p));

        let back =
            t.db.purchases(pid)
                .into_iter()
                .find(|x| x.id == p.id)
                .expect("xarid yo'qoldi");
        assert_eq!(back.task_id, Some(task));
        assert_eq!(back.buyer, "Sinov");
        assert!(back.urgent);
    }

    /// TZ XVII.14: yakuniy tannarx bugungi tannarxdan bajarilish ulushiga
    /// bo'linadi — boshqa taxmin yo'q.
    #[test]
    fn finance_forecast_scales_cost_by_progress() {
        use crate::checks::finance_forecast;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let f = finance_forecast(1_000.0, &[], &[], 200.0, 40.0, today);
        assert_eq!(f.contract, 1_000.0);
        assert!((f.cost_forecast - 500.0).abs() < 0.001, "200 / 0.4 = 500");
        assert!((f.profit_forecast - 500.0).abs() < 0.001);
        assert!((f.margin_pct - 50.0).abs() < 0.001);
        assert!((f.earned - 400.0).abs() < 0.001);

        // Bajarilish juda kichik — prognoz berilmaydi.
        let early = finance_forecast(1_000.0, &[], &[], 10.0, 2.0, today);
        assert_eq!(early.cost_forecast, 0.0);
        assert_eq!(early.profit_forecast, 0.0);
    }

    /// Tasdiqlangan o'zgarish shartnoma summasini oshiradi, debitorlik esa
    /// to'lov jadvalidan olinadi.
    #[test]
    fn forecast_uses_approved_changes_and_stages() {
        use crate::checks::finance_forecast;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let project =
            t.db.projects()
                .unwrap()
                .into_iter()
                .find(|p| p.id == pid)
                .unwrap();
        let changes = t.db.contract_changes(pid);
        let stages = t.db.payment_stages(pid);

        let f = finance_forecast(
            project.contract_sum,
            &changes,
            &stages,
            5_000_000_000.0,
            50.0,
            today,
        );
        let approved: f64 = changes
            .iter()
            .filter(|c| c.counts())
            .map(|c| c.amount)
            .sum();
        assert!((f.contract - (project.contract_sum + approved)).abs() < 0.01);
        assert!(f.receivable > 0.0, "to'lanmagan qoldiq yo'q");
        assert!(f.overdue > 0.0, "muddati o'tgan qarz yo'q");
        assert!(f.receivable >= f.overdue);
    }

    /// TZ XVII.36: ssenariy — bashorat emas, arifmetika.
    #[test]
    fn scenario_applies_assumptions_to_todays_numbers() {
        use crate::checks::{finance_forecast, scenario, Scenario, TaskCost};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let base = finance_forecast(1_000.0, &[], &[], 200.0, 40.0, today);
        // Tannarx: yarmi material, yarmi ish haqi.
        let costs = vec![TaskCost {
            task_id: 1,
            hours: 0.0,
            labour: 100.0,
            material: 100.0,
            machine: 0.0,
            total: 200.0,
            per_unit: 0.0,
        }];
        let end = today + chrono::Duration::days(100);

        // O'zgarishsiz ssenariy hech narsani siljitmaydi.
        let zero = scenario(&base, &costs, Some(end), end, &Scenario::default());
        assert!((zero.cost_after - zero.cost).abs() < 0.001);
        assert_eq!(zero.finish_after, zero.finish);
        assert!(!zero.over_deadline);

        // Material 10 % qimmatlashsa, yarmi material bo'lgani uchun
        // yakuniy tannarx 5 % ga oshadi.
        let s = Scenario {
            delay_days: 20,
            price_pct: 10.0,
            wage_pct: 0.0,
        };
        let r = scenario(&base, &costs, Some(end), end, &s);
        assert!((r.cost_after - base.cost_forecast * 1.05).abs() < 0.001);
        assert!(r.profit_after < r.profit);
        assert_eq!(r.finish_after, Some(end + chrono::Duration::days(20)));
        assert!(
            r.over_deadline,
            "muddat surildi, lekin chegara belgilanmadi"
        );
    }

    /// TZ XVII.41, 44: yo'qotishlar pulda o'lchanadi.
    #[test]
    fn opportunities_are_measured_in_money() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let list = app.opportunities();
        assert!(!list.is_empty(), "namunada topilma yo'q");
        for o in &list {
            assert!(o.amount != 0.0, "{}: summa nol", o.code);
            assert!(!o.title.is_empty());
            assert!(!o.detail.is_empty());
        }
        // Kodlar takrorlanmaydi.
        let mut codes: Vec<&str> = list.iter().map(|o| o.code).collect();
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), before);
    }

    /// TZ XVII.26: unumdorlik faqat hajmi va soati bor ishlarda hisoblanadi.
    #[test]
    fn productivity_needs_volume_and_hours() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.productivity();
        assert!(!rows.is_empty(), "namunada unumdorlik hisoblanmadi");
        for r in &rows {
            assert!(r.done_volume > 0.0);
            assert!(r.hours > 0.0);
            assert!((r.hours_per_unit - r.hours / r.done_volume).abs() < 0.001);
            assert!(r.cost_per_unit >= 0.0);
        }
        // Eng qimmati yuqorida.
        for w in rows.windows(2) {
            assert!(w[0].hours_per_unit >= w[1].hours_per_unit);
        }
    }

    /// TZ XI.34: asbob qaytariladi, shuning uchun asosiy savol — kimda.
    #[test]
    fn tool_status_shows_who_holds_it() {
        use crate::checks::{tool_status, tool_summary};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let tools = t.db.tools(pid);
        let issues = t.db.tool_issues(pid);
        assert!(!tools.is_empty(), "namunada asbob yo'q");

        let st = tool_status(&tools, &issues, today);
        assert_eq!(st.len(), tools.len());

        // Ishchida turgan asbobda ega ham, sana ham bor.
        for s in st.iter().filter(|s| s.holder.is_some()) {
            assert!(s.issued.is_some(), "berilgan sana yo'q");
            assert!(s.days >= 0);
        }
        // Qaytarilgan asbob egasiz qoladi.
        let returned: Vec<i64> = issues
            .iter()
            .filter(|x| x.returned.is_some())
            .map(|x| x.tool_id)
            .collect();
        for id in returned {
            let open = issues.iter().any(|x| x.tool_id == id && x.open());
            if !open {
                let s = st.iter().find(|s| s.tool_id == id).unwrap();
                assert!(s.holder.is_none(), "qaytarilgan asbob hali ishchida");
            }
        }

        let sum = tool_summary(&tools, &st, today);
        assert_eq!(sum.total, tools.len());
        assert_eq!(sum.in_store + sum.issued, sum.total);
        assert!(sum.overdue > 0, "namunada muddati o'tgan asbob yo'q");
        assert!(sum.out_of_service > 0, "ta'mirdagi asbob yo'q");
    }

    /// Muddati o'tgan berish aniq belgilanadi.
    #[test]
    fn overdue_tool_issue_is_flagged() {
        use crate::domain::ToolIssue;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap();
        let mk = |due: Option<i64>, returned: Option<i64>| ToolIssue {
            id: 0,
            project_id: 1,
            tool_id: 1,
            worker_id: 1,
            issued: today - chrono::Duration::days(20),
            due: due.map(|d| today + chrono::Duration::days(d)),
            returned: returned.map(|d| today - chrono::Duration::days(d)),
            note: String::new(),
        };
        // Muddati o'tgan va qaytarilmagan.
        assert!(mk(Some(-3), None).overdue(today));
        // Muddati kelmagan.
        assert!(!mk(Some(5), None).overdue(today));
        // Qaytarilgan — muddat o'tgan bo'lsa ham savol yo'q.
        assert!(!mk(Some(-3), Some(1)).overdue(today));
        // Muddatsiz berilgan.
        assert!(!mk(None, None).overdue(today));
        // Ishchida turgan kun soni.
        assert_eq!(mk(None, None).days(today), 20);
        assert_eq!(mk(None, Some(5)).days(today), 15);
    }

    /// TZ XI.26: takrorlangan kamomad tizimli sabab degani.
    #[test]
    fn shortages_count_repeated_differences() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.shortages();
        assert!(!rows.is_empty(), "namunada kamomad yo'q");
        for r in &rows {
            assert!(r.times > 0);
            assert!(r.shortage >= 0.0);
            assert!(r.cost >= 0.0);
        }
        // Puldagi zarari kattalari oldinda.
        for w in rows.windows(2) {
            assert!(w[0].cost >= w[1].cost);
        }
        // Ochiq inventarizatsiya hisobga kirmaydi: u hali to'ldirilmoqda.
        let open_ids: Vec<i64> = app
            .inventories
            .iter()
            .filter(|i| !i.closed)
            .map(|i| i.id)
            .collect();
        assert!(!open_ids.is_empty(), "namunada ochiq inventarizatsiya yo'q");
    }

    /// TZ XI.42: ortiqcha material boshqa obyektga taklif qilinadi.
    #[test]
    fn redistribution_moves_surplus_to_where_it_is_needed() {
        use crate::checks::{redistribution, PurchasePlanLine, StockLine};
        use crate::domain::Material;

        let mat = |id: i64, min_stock: f64| Material {
            id,
            project_id: 1,
            code: String::new(),
            name: "Sement M400".into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock,
            price: 1_000_000.0,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let stock = |id: i64, available: f64| StockLine {
            material_id: id,
            available,
            ..Default::default()
        };
        let plan = |id: i64, to_buy: f64| PurchasePlanLine {
            material_id: id,
            available: 0.0,
            ordered: 0.0,
            min_stock: 0.0,
            needed_for_tasks: to_buy,
            to_buy,
            cost: 0.0,
            need_by: None,
            has_request: false,
        };

        // 1-obyektda 30 t, minimal zaxira 10 t — 20 t erkin.
        // 2-obyektga 8 t kerak.
        let surplus = vec![(1, vec![stock(11, 30.0)], vec![mat(11, 10.0)])];
        let need = vec![(2, vec![plan(21, 8.0)], vec![mat(21, 0.0)])];
        let moves = redistribution(&surplus, &need);
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].from_project, 1);
        assert_eq!(moves[0].to_project, 2);
        assert!(
            (moves[0].qty - 8.0).abs() < 0.001,
            "kerakli miqdor ko'chadi"
        );
        assert!((moves[0].saving - 8_000_000.0).abs() < 0.01);

        // Minimal zaxiraga tegilmaydi: 12 t bo'lsa, faqat 2 t erkin.
        let tight = vec![(1, vec![stock(11, 12.0)], vec![mat(11, 10.0)])];
        let moves = redistribution(&tight, &need);
        assert_eq!(moves.len(), 1);
        assert!((moves[0].qty - 2.0).abs() < 0.001);

        // Zaxira chegarada — ko'chirish taklif qilinmaydi.
        let none = vec![(1, vec![stock(11, 10.0)], vec![mat(11, 10.0)])];
        assert!(redistribution(&none, &need).is_empty());
    }

    /// TZ IX.11-14: ariza tasdiqlashdan oldin tekshiriladi, lekin
    /// tekshiruv taqiq emas — u savol qo'yadi.
    #[test]
    fn request_issues_ask_questions_not_forbid() {
        use crate::checks::{request_issues, RequestIssue};
        use crate::domain::{Material, Request, RequestKind, RequestStatus};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let mat = |id: i64, name: &str, price: f64, banned: bool| Material {
            id,
            project_id: 1,
            code: String::new(),
            name: name.into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock: 0.0,
            price,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned,
            ban_reason: if banned {
                "Sinovdan o'tmagan".into()
            } else {
                String::new()
            },
            note: String::new(),
        };
        let req = |id: i64, material: i64, qty: f64, status: RequestStatus| Request {
            id,
            project_id: 1,
            number: format!("A-{id:04}"),
            date: today,
            kind: RequestKind::Material,
            title: String::new(),
            material_id: Some(material),
            qty,
            unit: "t".into(),
            requester: String::new(),
            need_date: today,
            priority: crate::domain::Priority::Normal,
            status,
            task_id: None,
            reject_reason: String::new(),
            note: String::new(),
        };

        let materials = vec![mat(1, "Sement M400", 1_000.0, false)];
        let r = req(10, 1, 5.0, RequestStatus::New);

        // Ish ko'rsatilmagan — savol bor.
        let out = request_issues(
            &r,
            std::slice::from_ref(&r),
            &materials,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            None,
        );
        assert!(out.contains(&RequestIssue::NoTask));

        // Ochiq dublikat topiladi, yopilgani esa yo'q.
        let open_dup = req(11, 1, 2.0, RequestStatus::Approved);
        let closed_dup = req(12, 1, 2.0, RequestStatus::Closed);
        let out = request_issues(
            &r,
            &[r.clone(), open_dup, closed_dup],
            &materials,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            None,
        );
        assert_eq!(
            out.iter()
                .filter(|i| matches!(i, RequestIssue::Duplicate { .. }))
                .count(),
            1,
            "yopilgan ariza dublikat sifatida sanaldi"
        );

        // Taqiqlangan material sababi bilan aytiladi.
        let banned = vec![mat(1, "Sement M400", 1_000.0, true)];
        let out = request_issues(
            &r,
            std::slice::from_ref(&r),
            &banned,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            None,
        );
        assert!(out
            .iter()
            .any(|i| matches!(i, RequestIssue::Banned { reason } if !reason.is_empty())));
    }

    /// Arzonroq analog topilsa tejash summasi bilan ko'rsatiladi.
    #[test]
    fn cheaper_alternative_is_offered_with_saving() {
        use crate::checks::{request_issues, RequestIssue};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let materials = t.db.materials(pid);
        let alts = t.db.material_alts(pid);
        assert!(!alts.is_empty(), "namunada analog yo'q");

        // Analogi bor va undan qimmatroq materialga ariza tuzamiz.
        let pair = alts.iter().find_map(|a| {
            let m = materials.iter().find(|m| m.id == a.material_id)?;
            let alt = materials.iter().find(|m| m.id == a.alt_id)?;
            (alt.price > 0.0 && alt.price < m.price).then_some((m, alt))
        });
        let Some((m, alt)) = pair else {
            // Namunada arzonroq analog bo'lmasa sinov ma'nosini yo'qotadi.
            return;
        };

        let r = crate::domain::Request {
            id: 0,
            project_id: pid,
            number: "A-9999".into(),
            date: t.db.projects().unwrap()[0].start_date,
            kind: crate::domain::RequestKind::Material,
            title: String::new(),
            material_id: Some(m.id),
            qty: 10.0,
            unit: m.unit.clone(),
            requester: String::new(),
            need_date: t.db.projects().unwrap()[0].start_date,
            priority: crate::domain::Priority::Normal,
            status: crate::domain::RequestStatus::New,
            task_id: None,
            reject_reason: String::new(),
            note: String::new(),
        };
        let out = request_issues(&r, &[], &materials, &alts, &[], &[], &[], &[], &[], None);
        let found = out.iter().find_map(|i| match i {
            RequestIssue::Cheaper { name, saving } => Some((name.clone(), *saving)),
            _ => None,
        });
        let (name, saving) = found.expect("arzonroq analog topilmadi");
        assert_eq!(name, alt.name);
        assert!((saving - (m.price - alt.price) * 10.0).abs() < 0.01);
    }

    /// TZ XIII.20: tabeldagi g'ayrioddiy holatlar topiladi.
    #[test]
    fn timesheet_anomalies_are_found() {
        use crate::checks::{timesheet_anomalies, TimesheetAnomaly, MAX_DAY_HOURS};
        use crate::domain::{DayKind, Shift, TimesheetEntry};

        let start = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let entry = |day: i64, hours: f64| TimesheetEntry {
            id: 0,
            project_id: 1,
            worker_id: 7,
            date: start + chrono::Duration::days(day),
            hours,
            kind: DayKind::Work,
            shift: Shift::Day,
            task_id: None,
            note: String::new(),
        };

        // Kunlik chegaradan ko'p soat.
        let long = vec![entry(0, MAX_DAY_HOURS + 2.0)];
        assert!(timesheet_anomalies(&long)
            .iter()
            .any(|a| matches!(a, TimesheetAnomaly::TooManyHours { .. })));

        // Odatdagi kun — savol yo'q.
        let normal = vec![entry(0, 8.0)];
        assert!(timesheet_anomalies(&normal).is_empty());

        // Dam olishsiz sakkiz kun ketma-ket.
        let streak: Vec<TimesheetEntry> = (0..8).map(|d| entry(d, 8.0)).collect();
        let found = timesheet_anomalies(&streak);
        assert!(found
            .iter()
            .any(|a| matches!(a, TimesheetAnomaly::NoRest { days, .. } if *days >= 8)));

        // Yakshanba ishi belgilanadi: 2026-06-07 — yakshanba.
        let sunday = vec![entry(6, 8.0)];
        assert!(timesheet_anomalies(&sunday)
            .iter()
            .any(|a| matches!(a, TimesheetAnomaly::WeekendWork { .. })));
    }

    /// TZ XIII.27: xodim ehtiyoji bugungi unumdorlikdan hisoblanadi.
    #[test]
    fn staff_forecast_uses_todays_productivity() {
        use crate::checks::{staff_forecast, Productivity};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let task = crate::model::Task {
            id: 5,
            project_id: 1,
            wbs: "1".into(),
            name: "Sinov".into(),
            section: crate::model::Section::Kj,
            responsible: String::new(),
            duration: 30,
            plan_start: today,
            fact_start: None,
            fact_end: None,
            // Yarmi bajarilgan: qolgani 500 birlik.
            progress: 50.0,
            pinned: false,
            volume: 1_000.0,
            unit: "m3".into(),
        };
        // Har birlik 2 soat: 500 birlik = 1000 soat.
        let prod = vec![Productivity {
            task_id: 5,
            done_volume: 500.0,
            unit: "m3".into(),
            hours: 1_000.0,
            hours_per_unit: 2.0,
            cost_per_unit: 0.0,
        }];
        let f = staff_forecast(std::slice::from_ref(&task), &prod, &[], today, 30);
        assert!((f.needed_hours - 1_000.0).abs() < 0.001);
        assert!(f.needed_workers > 0);
        assert_eq!(f.tasks, 1);
        // Ishchi yo'q — hammasi yetishmaydi.
        assert_eq!(f.gap, f.needed_workers as i64);

        // Unumdorligi noma'lum ish hisobga kirmaydi: taxmin qilib bo'lmaydi.
        let empty = staff_forecast(std::slice::from_ref(&task), &[], &[], today, 30);
        assert_eq!(empty.needed_hours, 0.0);
        assert_eq!(empty.tasks, 0);
    }

    /// TZ XIII.35: yopilgan davr bazadan o'zgarishsiz qaytadi.
    #[test]
    fn timesheet_period_round_trips() {
        use crate::domain::TimesheetPeriod;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let month = chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();

        let id = t.db.insert_timesheet_period(&TimesheetPeriod {
            id: 0,
            project_id: pid,
            month,
            closed: true,
            closed_at: Some(month + chrono::Duration::days(35)),
            closed_by: "Sinov".into(),
            reopen_reason: String::new(),
            note: String::new(),
        });
        assert!(id > 0);

        let back =
            t.db.timesheet_periods(pid)
                .into_iter()
                .find(|p| p.id == id)
                .expect("davr yo'qoldi");
        assert_eq!(back.month, month);
        assert!(back.closed);
        assert_eq!(back.closed_by, "Sinov");

        // Qayta ochilganda sabab saqlanadi.
        let mut p = back;
        p.closed = false;
        p.closed_at = None;
        p.reopen_reason = "Xato topildi".into();
        assert!(t.db.update_timesheet_period(&p));
        let again =
            t.db.timesheet_periods(pid)
                .into_iter()
                .find(|x| x.id == id)
                .unwrap();
        assert!(!again.closed);
        assert_eq!(again.reopen_reason, "Xato topildi");
    }

    /// TZ XIV.22: sinovda hukm laboratoriyaniki — ilova faqat solishtiradi.
    #[test]
    fn lab_test_keeps_result_and_value() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let tests = t.db.lab_tests(pid);
        assert!(!tests.is_empty(), "namunada sinov yo'q");

        // Natijasi kelmagan sinovda hukm ham yo'q.
        for x in tests.iter().filter(|x| x.pending()) {
            assert_eq!(x.result, crate::domain::LabTestResult::Waiting);
        }
        // Salbiy natijada qayta sinov sanasi qo'yiladi.
        for x in tests
            .iter()
            .filter(|x| x.result == crate::domain::LabTestResult::Fail)
        {
            assert!(x.retest.is_some(), "qayta sinov tayinlanmagan");
        }
        // Foiz hisobi: qiymat va talab bo'lsa.
        let with_value = tests
            .iter()
            .find(|x| x.value.is_some() && x.required.is_some())
            .expect("qiymatli sinov yo'q");
        let pct = with_value.pct().expect("foiz hisoblanmadi");
        assert!(
            (pct - with_value.value.unwrap() / with_value.required.unwrap() * 100.0).abs() < 0.001
        );
    }

    /// TZ XIV.32: xavf ro'yxatiga faqat ikki va undan ortiq sababli ish tushadi.
    #[test]
    fn quality_risks_need_at_least_two_reasons() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let risks = app.quality_risks();
        for r in &risks {
            assert!(
                r.reasons.len() >= 2,
                "bitta sabab bilan xavf ro'yxatiga tushdi"
            );
            assert_eq!(r.level, r.reasons.len());
            // Tugagan ish xavf ro'yxatiga tushmasligi kerak.
            let task = app.task(r.task_id).expect("ish topilmadi");
            assert!(task.fact_end.is_none() && task.progress < 99.999);
        }
        // Ko'proq sababli ishlar oldinda.
        for w in risks.windows(2) {
            assert!(w[0].level >= w[1].level);
        }
    }

    /// TZ XIV.28: mas'ullar reytingi umumiy ball bilan bir xil qoidada.
    #[test]
    fn contractor_quality_uses_the_same_score_rule() {
        use crate::checks::{contractor_quality, quality_score};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let quality = t.db.quality_checks(pid);
        let rating = contractor_quality(&quality, today);
        assert!(!rating.is_empty(), "namunada mas'ul ko'rsatilmagan");

        for c in &rating {
            let mine: Vec<crate::domain::QualityCheck> = quality
                .iter()
                .filter(|q| q.inspector.trim() == c.name)
                .cloned()
                .collect();
            let s = quality_score(&mine, today);
            assert!((c.score - s.score).abs() < 0.001, "ball boshqa qoidada");
            assert_eq!(c.checks, s.checks);
        }
        // Yuqori ball oldinda.
        for w in rating.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    /// TZ XIV.6: taqiqlangan material chiqarilgani ko'rsatiladi.
    #[test]
    fn banned_material_usage_is_visible() {
        use crate::checks::banned_usage;
        use crate::domain::{MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let mut m = crate::domain::Material {
            id: 3,
            project_id: 1,
            code: String::new(),
            name: "Sinov".into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock: 0.0,
            price: 100.0,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let mv = |kind: MoveKind, qty: f64| StockMove {
            id: 0,
            project_id: 1,
            material_id: 3,
            date: today,
            kind,
            qty,
            price: 0.0,
            document: String::new(),
            counterparty: String::new(),
            task_id: None,
            warehouse_id: None,
            batch_id: None,
            note: String::new(),
        };

        // Taqiqlanmagan material — savol yo'q.
        let moves = vec![mv(MoveKind::Out, 5.0)];
        assert!(banned_usage(std::slice::from_ref(&m), &moves).is_empty());

        // Taqiqlangan va chiqarilgan — ko'rsatiladi.
        m.banned = true;
        m.ban_reason = "Sertifikat yo'q".into();
        let out = banned_usage(std::slice::from_ref(&m), &moves);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].qty, 5.0);
        assert_eq!(out[0].moves, 1);
        assert_eq!(out[0].reason, "Sertifikat yo'q");

        // Faqat kirim bo'lsa — ishlatilmagan.
        let only_in = vec![mv(MoveKind::In, 5.0)];
        assert!(banned_usage(std::slice::from_ref(&m), &only_in).is_empty());
    }

    /// TZ XVI.12: bir texnika bir vaqtda ikki ishga band bo'lsa aytiladi.
    #[test]
    fn booking_conflicts_are_found() {
        use crate::checks::booking_conflicts;
        use crate::domain::MachineBooking;

        let day = |n: i64| {
            chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap() + chrono::Duration::days(n)
        };
        let bk = |id: i64, machine: i64, from: i64, to: i64| MachineBooking {
            id,
            project_id: 1,
            machine_id: machine,
            task_id: None,
            from: day(from),
            to: day(to),
            shifts: 1.0,
            note: String::new(),
        };

        // Kesishmaydigan bandliklar — savol yo'q.
        let ok = vec![bk(1, 7, 0, 5), bk(2, 7, 6, 10)];
        assert!(booking_conflicts(&ok).is_empty());

        // Kesishadi: 3-5 kunlar ikkalasida ham band.
        let bad = vec![bk(1, 7, 0, 5), bk(2, 7, 3, 9)];
        let found = booking_conflicts(&bad);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].machine_id, 7);
        assert_eq!(found[0].days, 3, "3, 4 va 5-kunlar kesishadi");

        // Boshqa texnika — to'qnashuv emas.
        let other = vec![bk(1, 7, 0, 5), bk(2, 8, 3, 9)];
        assert!(booking_conflicts(&other).is_empty());

        // Bandlik davomiyligi kunlar bilan.
        assert_eq!(bk(1, 7, 0, 6).days(), 7);
    }

    /// TZ XVI.43: «ta'mirlash yoki almashtirish» qarori ta'mir qiymatining
    /// balans qiymatiga nisbatidan chiqadi.
    #[test]
    fn repair_summary_flags_expensive_machines() {
        use crate::checks::{repair_summary, REPLACE_LIMIT_PCT};
        use crate::domain::{MachineRepair, RepairKind};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let machine = |id: i64, price: f64| crate::domain::Machine {
            id,
            project_id: 1,
            name: format!("M-{id}"),
            kind: crate::domain::MachineKind::Excavator,
            reg_no: String::new(),
            owner: String::new(),
            status: crate::domain::MachineStatus::Working,
            hour_rate: 0.0,
            operator: String::new(),
            inspection_until: None,
            fuel_norm: 0.0,
            service_hours: 0.0,
            service_done: 0.0,
            rented: false,
            price,
        };
        let repair = |machine_id: i64, cost: f64, open: bool| MachineRepair {
            id: 0,
            project_id: 1,
            machine_id,
            kind: RepairKind::Fault,
            started: today - chrono::Duration::days(4),
            finished: (!open).then(|| today - chrono::Duration::days(2)),
            reason: String::new(),
            cost,
            hours_at: 0.0,
            note: String::new(),
        };

        let machines = vec![machine(1, 100_000_000.0), machine(2, 100_000_000.0)];
        let repairs = vec![
            // Birinchisiga chegaradan ko'p sarflangan.
            repair(1, 45_000_000.0, false),
            // Ikkinchisiga kam, lekin hozir ta'mirda.
            repair(2, 10_000_000.0, true),
        ];
        let out = repair_summary(&machines, &repairs, today);
        assert_eq!(out.len(), 2);

        let first = out.iter().find(|s| s.machine_id == 1).unwrap();
        assert!(first.cost_pct > REPLACE_LIMIT_PCT);
        assert!(first.consider_replacing);
        assert!(!first.in_repair);
        assert_eq!(first.faults, 1);
        assert_eq!(first.downtime, 3, "boshlangan kun ham hisobga kiradi");

        let second = out.iter().find(|s| s.machine_id == 2).unwrap();
        assert!(!second.consider_replacing);
        assert!(second.in_repair);

        // Qiymati noma'lum texnikada savol qo'yib bo'lmaydi.
        let unknown = repair_summary(&[machine(3, 0.0)], &[repair(3, 99.0, false)], today);
        assert_eq!(unknown[0].cost_pct, 0.0);
        assert!(!unknown[0].consider_replacing);
    }

    /// Namunada bandlik to'qnashuvi va ochiq ta'mir bor.
    #[test]
    fn demo_has_booking_conflict_and_open_repair() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();

        let bookings = t.db.machine_bookings(pid);
        assert!(!bookings.is_empty(), "namunada bandlik yo'q");
        assert!(
            !crate::checks::booking_conflicts(&bookings).is_empty(),
            "namunada to'qnashuv ko'rsatilmagan"
        );

        let repairs = t.db.machine_repairs(pid);
        assert!(!repairs.is_empty(), "namunada ta'mir yo'q");
        assert!(
            repairs.iter().any(|r| r.open()),
            "namunada ochiq ta'mir yo'q"
        );
        let sum = crate::checks::repair_summary(&t.db.machines(pid), &repairs, today);
        assert!(sum.iter().any(|s| s.in_repair));
        assert!(sum.iter().map(|s| s.cost).sum::<f64>() > 0.0);
    }

    /// TZ XV.28: sabab tahlili faqat haqiqiy hodisalarni sanaydi.
    #[test]
    fn root_causes_count_only_real_events() {
        use crate::checks::root_causes;
        use crate::domain::{RootCause, SafetyEvent, SafetyKind};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ev = |kind: SafetyKind, severity: Severity, cause: RootCause| SafetyEvent {
            id: 0,
            project_id: 1,
            date: today,
            kind,
            severity,
            place: String::new(),
            description: String::new(),
            responsible: "A".into(),
            measure: String::new(),
            deadline: None,
            status: crate::domain::IssueStatus::Fixed,
            root_cause: cause,
        };

        let events = vec![
            // Instruktaj va tekshiruv sabab talab qilmaydi — sanalmaydi.
            ev(SafetyKind::Training, Severity::Info, RootCause::Unknown),
            ev(SafetyKind::Inspection, Severity::Info, RootCause::Unknown),
            // Haqiqiy hodisalar.
            ev(SafetyKind::Violation, Severity::Major, RootCause::NoPpe),
            ev(SafetyKind::Violation, Severity::Warning, RootCause::NoPpe),
            ev(
                SafetyKind::NearMiss,
                Severity::Warning,
                RootCause::NoBarrier,
            ),
        ];
        let out = root_causes(&events);
        let total: usize = out.iter().map(|c| c.count).sum();
        assert_eq!(total, 3, "instruktaj va tekshiruv sanalib ketdi");

        let ppe = out.iter().find(|c| c.cause == RootCause::NoPpe).unwrap();
        assert_eq!(ppe.count, 2);
        assert_eq!(ppe.serious, 1, "faqat jiddiy va kritiklari");
        assert!((ppe.pct - 66.666).abs() < 0.01);

        // Ko'p uchraydigan sabab oldinda.
        assert_eq!(out[0].cause, RootCause::NoPpe);

        // Hodisasiz ro'yxat bo'sh.
        assert!(root_causes(&[]).is_empty());
    }

    /// TZ XV.36: bir xil sabab uch marta takrorlansa — tizim nuqsoni.
    #[test]
    fn repeated_cause_becomes_a_risk() {
        use crate::checks::{safety_risks, SafetyRisk, CAUSE_REPEAT_LIMIT};
        use crate::domain::{RootCause, SafetyEvent, SafetyKind};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let ev = |cause: RootCause| SafetyEvent {
            id: 0,
            project_id: 1,
            date: today,
            kind: SafetyKind::Violation,
            severity: Severity::Warning,
            place: String::new(),
            description: String::new(),
            responsible: String::new(),
            measure: String::new(),
            deadline: None,
            status: crate::domain::IssueStatus::Fixed,
            root_cause: cause,
        };

        // Ikkitasi hali tizim nuqsoni emas.
        let two = vec![ev(RootCause::Rush), ev(RootCause::Rush)];
        assert!(!safety_risks(&[], &two, &[], &[], today)
            .iter()
            .any(|r| matches!(r, SafetyRisk::RepeatedCause { .. })));

        // Uchtasi — belgilanadi.
        let three: Vec<_> = (0..CAUSE_REPEAT_LIMIT)
            .map(|_| ev(RootCause::Rush))
            .collect();
        let found = safety_risks(&[], &three, &[], &[], today);
        assert!(found
            .iter()
            .any(|r| matches!(r, SafetyRisk::RepeatedCause { cause, count }
                if *cause == RootCause::Rush && *count == CAUSE_REPEAT_LIMIT)));

        // Aniqlanmagan sabab takrorlansa ham belgilanmaydi: u sabab emas.
        let unknown: Vec<_> = (0..5).map(|_| ev(RootCause::Unknown)).collect();
        assert!(!safety_risks(&[], &unknown, &[], &[], today)
            .iter()
            .any(|r| matches!(r, SafetyRisk::RepeatedCause { .. })));
    }

    /// Zonada chora ko'rilmagani va tekshiruv muddati alohida belgilanadi.
    #[test]
    fn zone_states_are_separated() {
        use crate::checks::{safety_risks, SafetyRisk};
        use crate::domain::{SafetyZone, ZoneKind};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let zone = |id: i64, ready: bool, due: i64| SafetyZone {
            id,
            project_id: 1,
            kind: ZoneKind::Danger,
            name: format!("Z-{id}"),
            place: String::new(),
            measure: String::new(),
            responsible: String::new(),
            check_due: Some(today + chrono::Duration::days(due)),
            checked_at: None,
            ready,
            note: String::new(),
        };

        // Chora ko'rilmagan — muddatdan qat'i nazar birinchi savol shu.
        let not_ready = zone(1, false, 10);
        assert!(not_ready.needs_action(today));
        let out = safety_risks(std::slice::from_ref(&not_ready), &[], &[], &[], today);
        assert!(out
            .iter()
            .any(|r| matches!(r, SafetyRisk::ZoneNotReady { zone_id } if *zone_id == 1)));

        // Chora ko'rilgan, lekin tekshiruv muddati o'tgan.
        let overdue = zone(2, true, -5);
        assert!(overdue.overdue(today));
        let out = safety_risks(std::slice::from_ref(&overdue), &[], &[], &[], today);
        assert!(out.iter().any(
            |r| matches!(r, SafetyRisk::ZoneOverdue { zone_id, days } if *zone_id == 2 && *days == 5)
        ));

        // Hammasi joyida — savol yo'q.
        let fine = zone(3, true, 20);
        assert!(!fine.needs_action(today));
        assert!(safety_risks(std::slice::from_ref(&fine), &[], &[], &[], today).is_empty());
    }

    /// Namunada zonalar va sabablar to'ldirilgan.
    #[test]
    fn demo_has_zones_and_causes() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();

        let zones = t.db.safety_zones(pid);
        assert!(!zones.is_empty(), "namunada zona yo'q");
        assert!(
            zones.iter().any(|z| z.needs_action(today)),
            "e'tibor talab qiladigan zona yo'q"
        );
        // Yong'in inventari va evakuatsiya ham bor.
        assert!(zones
            .iter()
            .any(|z| z.kind == crate::domain::ZoneKind::Fire));
        assert!(zones
            .iter()
            .any(|z| z.kind == crate::domain::ZoneKind::Evacuation));

        let causes = crate::checks::root_causes(&t.db.safety_events(pid));
        assert!(!causes.is_empty(), "sabab tahlili bo'sh");
        assert!(
            causes
                .iter()
                .any(|c| c.cause != crate::domain::RootCause::Unknown),
            "namunada aniq sabab ko'rsatilmagan"
        );
    }

    /// TZ VIII.28: har bir ogohlantirishda son bor — foiz, kun yoki summa.
    #[test]
    fn contract_alerts_carry_numbers() {
        use crate::checks::{contract_alerts, ContractAlert, DECISION_LIMIT_DAYS};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let alerts = contract_alerts(
            &t.db.contracts(pid),
            &t.db.contract_changes(pid),
            &t.db.payment_stages(pid),
            &t.db.work_acceptances(pid),
            today,
        );
        assert!(!alerts.is_empty(), "namunada ogohlantirish yo'q");

        for a in &alerts {
            match a {
                ContractAlert::SumDeviation { pct, .. } => {
                    assert!(pct.abs() > crate::checks::SUM_DEVIATION_LIMIT);
                }
                ContractAlert::ChangePending { number, days } => {
                    assert!(!number.is_empty());
                    assert!(*days > DECISION_LIMIT_DAYS);
                }
                ContractAlert::PaymentOverdue {
                    number,
                    days,
                    amount,
                } => {
                    assert!(!number.is_empty());
                    assert!(*days > 0);
                    assert!(*amount > 0.0);
                }
                ContractAlert::ContractOverdue { days, .. } => assert!(*days > 0),
                ContractAlert::ScheduleGap { gap, .. } => assert!(*gap > 0.0),
                ContractAlert::AcceptancePending { number, days } => {
                    assert!(!number.is_empty());
                    assert!(*days > DECISION_LIMIT_DAYS);
                }
            }
        }
        // Muddati o'tgan to'lov namunada bor.
        assert!(alerts
            .iter()
            .any(|a| matches!(a, ContractAlert::PaymentOverdue { .. })));
    }

    /// Yangi qaror kutayotgan o'zgarish darhol ogohlantirishga aylanmaydi.
    #[test]
    fn fresh_pending_change_is_not_an_alert_yet() {
        use crate::checks::{contract_alerts, ContractAlert, DECISION_LIMIT_DAYS};
        use crate::domain::{ChangeKind, ChangeStatus, ContractChange};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let change = |days_ago: i64| ContractChange {
            id: 0,
            project_id: 1,
            contract_id: None,
            number: "DS-9".into(),
            kind: ChangeKind::Extra,
            date: today - chrono::Duration::days(days_ago),
            description: String::new(),
            amount: 100.0,
            days: 0,
            reason: String::new(),
            status: ChangeStatus::Sent,
            decided_at: None,
            decided_by: String::new(),
            note: String::new(),
        };

        // Kecha yuborilgan — hali savol yo'q.
        let fresh = contract_alerts(&[], &[change(1)], &[], &[], today);
        assert!(fresh.is_empty());

        // Chegaradan oshgan — ogohlantirish.
        let old = contract_alerts(&[], &[change(DECISION_LIMIT_DAYS + 2)], &[], &[], today);
        assert!(old
            .iter()
            .any(|a| matches!(a, ContractAlert::ChangePending { .. })));
    }

    /// TZ III.33: zanjir smetadan faktgacha bo'lgan yo'lni bir qatorga yig'adi.
    #[test]
    fn estimate_chain_links_plan_to_fact() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let lines = app.estimate_chain();
        assert!(!lines.is_empty(), "zanjir bo'sh");

        for l in &lines {
            // Bo'sh qator zanjirga tushmasligi kerak.
            assert!(
                l.planned != 0.0 || l.requested != 0.0 || l.purchased != 0.0 || l.actual != 0.0,
                "bo'sh qator zanjirda"
            );
            assert!(l.progress >= 0.0 && l.progress <= 100.0);
            // Bajarilgan reja: reja × bajarilish ulushi.
            assert!((l.earned - l.planned * l.progress / 100.0).abs() < 0.01);
            // Farq: bajarilgan reja minus fakt.
            assert!((l.diff - (l.earned - l.actual)).abs() < 0.01);
        }

        // Ortiqcha sarf kattalari oldinda.
        for w in lines.windows(2) {
            assert!(w[0].diff <= w[1].diff);
        }

        let totals = crate::checks::chain_totals(&lines);
        assert!((totals.planned - lines.iter().map(|l| l.planned).sum::<f64>()).abs() < 0.01);
        assert!((totals.diff - lines.iter().map(|l| l.diff).sum::<f64>()).abs() < 0.01);
        assert_eq!(totals.gaps, lines.iter().filter(|l| l.has_gap()).count());
    }

    /// Zanjirdagi uzilish: xarid arizadan katta yoki chiqim kirimdan katta.
    #[test]
    fn chain_gap_is_detected() {
        use crate::checks::ChainLine;

        let line = |requested: f64, purchased: f64, received: f64, issued: f64| ChainLine {
            task_id: Some(1),
            planned: 100.0,
            requested,
            purchased,
            received,
            issued,
            actual: 0.0,
            progress: 0.0,
            earned: 0.0,
            diff: 0.0,
        };

        // Hammasi joyida.
        assert!(!line(100.0, 90.0, 90.0, 80.0).has_gap());
        // Xarid arizadan katta — ariza bosqichi hujjatsiz o'tgan.
        assert!(line(50.0, 90.0, 90.0, 80.0).has_gap());
        // Chiqim kirimdan katta — omborda yo'q material berilgan.
        assert!(line(100.0, 90.0, 40.0, 80.0).has_gap());
        // Teng qiymatlar uzilish emas.
        assert!(!line(90.0, 90.0, 80.0, 80.0).has_gap());
    }

    /// TZ IV.19: rad etilgan hujjat o'chirilmaydi — yangi versiya paydo
    /// bo'ladi, eskisi arxivda qoladi va «o'rniga chiqilgan» deb belgilanadi.
    #[test]
    fn document_versions_are_kept() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let docs = t.db.exec_docs(pid);

        let v2 = docs
            .iter()
            .find(|d| d.version == 2)
            .expect("namunada ikkinchi versiya yo'q");
        let old_id = v2.replaces.expect("yangi versiya eskisiga bog'lanmagan");
        let v1 = docs
            .iter()
            .find(|d| d.id == old_id)
            .expect("eski versiya o'chib ketgan");

        assert_eq!(v1.version, 1);
        assert_eq!(v1.status, crate::domain::ExecDocStatus::Rejected);
        assert!(v1.superseded(&docs), "eskisi o'rniga chiqilgan emas");
        assert!(!v2.superseded(&docs), "oxirgi versiya o'rniga chiqilgan");
        // Ikkalasi ham bitta ishga tegishli.
        assert_eq!(v1.task_id, v2.task_id);
    }

    /// TZ IV.20: imzolashdan oldingi tekshiruv boshqa modullardagi yozuvga
    /// tayanadi — o'zi yangi hisob-kitob qilmaydi.
    #[test]
    fn doc_readiness_reads_other_modules() {
        use crate::checks::DocProblem as P;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let checks = app.doc_readiness();
        assert!(!checks.is_empty(), "namunada e'tiroz yo'q");

        for c in &checks {
            let doc = app
                .exec_docs
                .iter()
                .find(|d| d.id == c.doc_id)
                .expect("e'tiroz mavjud bo'lmagan hujjatga");
            // Qaror chiqib bo'lgan hujjat tekshirilmaydi.
            assert!(!matches!(
                doc.status,
                crate::domain::ExecDocStatus::Signed | crate::domain::ExecDocStatus::Rejected
            ));
            assert!(!c.problems.is_empty());

            for p in &c.problems {
                match p {
                    // Ish tugallanmagani — GPR dagi bajarilishdan olinadi.
                    P::WorkUnfinished { progress } => {
                        let task = app.task(doc.task_id.unwrap()).unwrap();
                        assert!((task.progress - progress).abs() < 0.001);
                        assert!(task.fact_end.is_none());
                    }
                    // Salbiy tekshiruv — texnik nazorat yozuvidan.
                    P::InspectionFailed { number } => {
                        assert!(app.inspections.iter().any(|i| &i.number == number
                            && i.result == crate::domain::InspectionResult::Fail));
                    }
                    // Salbiy sinov — laboratoriya yozuvidan.
                    P::LabFailed { number } => {
                        assert!(app.lab_tests.iter().any(|l| &l.number == number
                            && l.result == crate::domain::LabTestResult::Fail));
                    }
                    _ => {}
                }
            }
        }

        // To'sadiganlar oldinda turadi.
        let mut seen_ready = false;
        for c in &checks {
            if c.ready() {
                seen_ready = true;
            } else {
                assert!(!seen_ready, "to'sadigan e'tiroz tayyorlaridan keyin qolgan");
            }
        }
    }

    /// Yashirin ish dalolatnomasi imzolangach — keyingi ish to'siqdan chiqadi.
    #[test]
    fn signed_hidden_act_lifts_the_block() {
        use crate::domain::ExecDocStatus;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let before = app.hidden_blocks();
        assert!(!before.is_empty(), "namunada yashirin ish to'sig'i yo'q");
        let block = before[0].clone();

        // O'sha ishning dalolatnomasini imzolaymiz.
        let mut doc = app
            .exec_docs
            .iter()
            .filter(|d| {
                d.task_id == Some(block.pred_id) && d.kind == crate::domain::ExecDocKind::Hidden
            })
            .max_by_key(|d| d.version)
            .cloned()
            .unwrap_or_else(|| {
                let mut fresh = app.exec_docs[0].clone();
                fresh.id = 0;
                fresh.kind = crate::domain::ExecDocKind::Hidden;
                fresh.task_id = Some(block.pred_id);
                fresh.version = 1;
                fresh.replaces = None;
                let id = app.db.insert_exec_doc(&fresh);
                fresh.id = id;
                fresh
            });
        doc.status = ExecDocStatus::Signed;
        assert!(app.db.update_exec_doc(&doc));
        app.reload_modules();

        let after = app.hidden_blocks();
        assert!(
            !after.iter().any(|b| b.pred_id == block.pred_id),
            "imzolangandan keyin ham to'siq qolyapti"
        );
        assert!(after.len() < before.len());
    }

    /// TZ XII.8, 15, 32: moslik e'tirozlari kartochkadagi yozuvdan chiqadi
    /// va ishlatilgan material oldinda turadi.
    #[test]
    fn material_fit_reads_the_card() {
        use crate::checks::{FitProblem as P, CERT_WARN_DAYS};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let fits = app.material_fit();
        for f in &fits {
            let m = app
                .materials
                .iter()
                .find(|m| m.id == f.material_id)
                .expect("e'tiroz mavjud bo'lmagan materialga");
            assert!(!f.problems.is_empty());
            for p in &f.problems {
                match p {
                    P::NoSpec => assert!(m.spec.trim().is_empty()),
                    P::NoSpecRef => assert!(m.spec_ref.trim().is_empty()),
                    P::NoEstimateCode => assert!(m.estimate_code.trim().is_empty()),
                    P::NoCertificate => assert!(m.cert_no.trim().is_empty()),
                    P::CertExpired { days } => {
                        let until = m.cert_until.expect("muddat yo'q");
                        assert_eq!((app.today - until).num_days(), *days);
                    }
                    P::CertExpiring { days } => {
                        let until = m.cert_until.expect("muddat yo'q");
                        assert_eq!((until - app.today).num_days(), *days);
                        assert!(*days <= CERT_WARN_DAYS);
                    }
                    P::BanWithoutReason => {
                        assert!(m.banned && m.ban_reason.trim().is_empty());
                    }
                    _ => {}
                }
            }
        }

        // Tartib: jiddiylari, keyin ishlatilganlari.
        let mut seen_non_critical = false;
        for f in &fits {
            if f.critical() {
                assert!(!seen_non_critical, "jiddiy e'tiroz pastga tushib qolgan");
            } else {
                seen_non_critical = true;
            }
        }
    }

    /// Sertifikatsiz va tavsifsiz material — ishlatilgan bo'lsa jiddiy.
    #[test]
    fn missing_certificate_is_severe() {
        use crate::checks::{material_fit, FitProblem};
        use crate::domain::{Material, MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let mut m = Material {
            id: 7,
            project_id: 1,
            code: "M-7".into(),
            name: "Sement".into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: "PC 400 GOST 10178".into(),
            cert_no: String::new(),
            cert_until: None,
            min_stock: 0.0,
            price: 100.0,
            estimate_code: "E-1".into(),
            spec_ref: "S-1".into(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };

        // Ishlatilmagan: e'tiroz bor, lekin kritik emas.
        let quiet = material_fit(std::slice::from_ref(&m), &[], today);
        assert_eq!(quiet.len(), 1);
        assert!(quiet[0].problems.contains(&FitProblem::NoCertificate));
        assert!(!quiet[0].used);
        assert!(!quiet[0].critical());

        // Ishga berilgan: endi bu jiddiy.
        let mv = StockMove {
            id: 1,
            project_id: 1,
            material_id: 7,
            kind: MoveKind::Out,
            qty: 3.0,
            price: 100.0,
            date: today,
            task_id: None,
            warehouse_id: None,
            batch_id: None,
            document: String::new(),
            counterparty: String::new(),
            note: String::new(),
        };
        let loud = material_fit(std::slice::from_ref(&m), std::slice::from_ref(&mv), today);
        assert!(loud[0].used);
        assert!(loud[0].critical());

        // Sertifikat berilsa — e'tiroz yo'qoladi.
        m.cert_no = "SS-2211".into();
        let fixed = material_fit(std::slice::from_ref(&m), std::slice::from_ref(&mv), today);
        assert!(fixed.is_empty() || !fixed[0].problems.contains(&FitProblem::NoCertificate));
    }

    /// TZ XII.33: komplekt tayyorligi «Tayyorlik» jadvalidagi yetishmovchilik
    /// bilan bir xil manbadan chiqadi — ikki ekran ziddiyatga tushmaydi.
    #[test]
    fn material_kit_matches_readiness() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let ready = app.readiness();
        let kits = app.material_kits();
        assert!(!kits.is_empty(), "komplekt topilmadi");

        for k in &kits {
            let missing = ready.iter().filter(|r| r.task_id == k.task_id).count();
            assert_eq!(k.missing, missing);
            assert!(k.total >= 1);
            let expected = (k.total - k.missing.min(k.total)) as f64 * 100.0 / k.total as f64;
            assert!((k.ready_pct - expected).abs() < 0.001);
            assert_eq!(k.complete(), k.missing == 0);
            if k.missing > 0 {
                assert!(k.worst.is_some());
            }
        }

        // Eng kam tayyor komplekt oldinda.
        for w in kits.windows(2) {
            assert!(w[0].ready_pct <= w[1].ready_pct + 0.001);
        }
    }

    /// TZ XII.34: solishtiruv faqat ikki va undan ortiq taklifi bor
    /// materiallar bo'yicha ko'rsatiladi; eng arzoni birinchi qatorda.
    #[test]
    fn maker_comparison_needs_two_offers() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        for c in app.maker_comparison() {
            assert!(c.offers.len() >= 2, "bitta taklif solishtiruv emas");
            // Narx bo'yicha o'sish tartibida.
            for w in c.offers.windows(2) {
                assert!(w[0].price <= w[1].price);
            }
            // Birinchisi — eng arzoni.
            assert!(c.offers[0].over_pct.abs() < 0.001);
            let max = c.offers.last().unwrap().price;
            let min = c.offers[0].price;
            assert!((c.spread_pct - (max - min) * 100.0 / min).abs() < 0.001);
        }
    }

    /// TZ VI.33-34: kun yakuni tekshiruvi bugungi yozuvlarga qaraydi va
    /// to'sadigan kamchiliklar ro'yxat boshida turadi.
    #[test]
    fn day_close_reads_todays_records() {
        use crate::checks::DayIssue as D;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let issues = app.day_close();
        let today = app.today;

        for i in &issues {
            match i {
                D::NoJournal => assert!(!app.journal.iter().any(|j| j.date == today)),
                D::NoTimesheet => {
                    assert!(!app
                        .timesheet
                        .iter()
                        .any(|e| e.date == today && e.hours > 0.0));
                }
                D::CrewMismatch { journal, timesheet } => {
                    let counted = app
                        .timesheet
                        .iter()
                        .filter(|e| e.date == today && e.hours > 0.0)
                        .count() as i64;
                    assert_eq!(*timesheet, counted);
                    assert!((journal - timesheet).abs() > 1);
                }
                // Hajm faqat bugun ketayotgan ish bo'yicha so'raladi.
                D::NoVolume { task_id } => {
                    assert!(app.running_today().contains(task_id));
                    assert!(!app
                        .journal
                        .iter()
                        .any(|j| j.date == today && j.task_id == Some(*task_id) && j.volume > 0.0));
                }
                _ => {}
            }
        }

        let mut seen_soft = false;
        for i in &issues {
            if i.blocking() {
                assert!(!seen_soft, "to'sadigan kamchilik pastga tushib qolgan");
            } else {
                seen_soft = true;
            }
        }
    }

    /// Jurnal va tabel to'ldirilgach, kun yakuni tekshiruvidan bu ikki
    /// to'siq yo'qoladi.
    #[test]
    fn filling_the_day_clears_the_blocks() {
        use crate::checks::DayIssue as D;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        let today = app.today;

        // Bugungi jurnal yozuvi va tabel. Namunada bugungi yozuv bo'lishi
        // mumkin — shunda uni to'ldiramiz, yangisini yaratmaymiz.
        if let Some(mut j) = app.journal.iter().find(|j| j.date == today).cloned() {
            j.weather = "ochiq".into();
            j.photos = "foto.jpg".into();
            assert!(app.db.update_journal(&j));
        } else {
            app.db.insert_journal(&crate::domain::JournalEntry {
                id: 0,
                project_id: pid,
                date: today,
                author: "Test".into(),
                weather: "ochiq".into(),
                temperature: 24.0,
                workers: 0,
                machines: 0,
                task_id: None,
                volume: 0.0,
                unit: String::new(),
                text: "kun yakuni".into(),
                remarks: String::new(),
                photos: "foto.jpg".into(),
                gps: String::new(),
            });
        }
        for w in app.workers.iter().filter(|w| w.active) {
            app.db.set_timesheet(pid, w.id, today, 8.0);
        }
        app.reload_modules();

        let issues = app.day_close();
        assert!(!issues.contains(&D::NoJournal));
        assert!(!issues.contains(&D::NoTimesheet));
        assert!(!issues.contains(&D::NoWeather));
        assert!(!issues.contains(&D::NoPhoto));
    }

    /// TZ VI.25: yopish ogohlantirishi sifat va hujjat modullaridan o'qiydi —
    /// o'zi qayta hisoblamaydi.
    #[test]
    fn close_warnings_come_from_other_modules() {
        use crate::checks::CloseWarning as W;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let blocks = app.task_blocks();
        let required = crate::checks::required_docs(&app.tasks, &app.exec_docs, false);

        let mut checked = 0;
        for task in app.tasks.clone() {
            let warns = app.close_warnings(task.id);
            for w in &warns {
                checked += 1;
                match w {
                    W::Quality { defects, points } => {
                        let b = blocks
                            .iter()
                            .find(|b| b.task_id == task.id)
                            .expect("sifat to'sig'i modulda yo'q");
                        assert_eq!(b.open_defects, *defects);
                        assert_eq!(b.pending_points, *points);
                    }
                    W::Docs { missing } => {
                        let n = required
                            .iter()
                            .filter(|r| r.task_id == task.id && !r.signed)
                            .count();
                        assert_eq!(n, *missing);
                    }
                    W::NoLabour => {
                        assert!(!app
                            .timesheet
                            .iter()
                            .any(|e| e.task_id == Some(task.id) && e.hours > 0.0));
                    }
                    _ => {}
                }
            }
            // Jiddiylari oldinda.
            let mut seen_soft = false;
            for w in &warns {
                if w.severe() {
                    assert!(!seen_soft, "jiddiy ogohlantirish pastga tushgan");
                } else {
                    seen_soft = true;
                }
            }
        }
        assert!(checked > 0, "namunada ogohlantirish umuman yo'q");
    }

    /// TZ V.10-11: kunlik hajm normaga ko'paytiriladi va o'sha kuni
    /// berilgan material bilan solishtiriladi.
    #[test]
    fn day_material_compares_norm_with_issue() {
        use crate::checks::day_material;
        use crate::domain::{JournalEntry, MaterialNorm, MoveKind, StockMove};

        let day = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let entry = JournalEntry {
            id: 1,
            project_id: 1,
            date: day,
            author: String::new(),
            weather: String::new(),
            temperature: 0.0,
            workers: 4,
            machines: 0,
            task_id: Some(10),
            volume: 20.0,
            unit: "m3".into(),
            text: String::new(),
            remarks: String::new(),
            photos: String::new(),
            gps: String::new(),
        };
        let norm = MaterialNorm {
            id: 1,
            project_id: 1,
            task_id: 10,
            material_id: 5,
            per_unit: 0.1,
            tolerance: 5.0,
            note: String::new(),
        };
        let out = |qty: f64| StockMove {
            id: 1,
            project_id: 1,
            material_id: 5,
            kind: MoveKind::Out,
            qty,
            price: 0.0,
            date: day,
            task_id: Some(10),
            warehouse_id: None,
            batch_id: None,
            document: String::new(),
            counterparty: String::new(),
            note: String::new(),
        };

        // 20 m3 × 0.1 = 2.0 kerak. 2.05 — chegara ichida (5%).
        let ok = day_material(
            std::slice::from_ref(&entry),
            std::slice::from_ref(&norm),
            &[out(2.05)],
            day,
        );
        assert_eq!(ok.len(), 1);
        assert!((ok[0].by_norm - 2.0).abs() < 1e-9);
        assert!((ok[0].diff - 0.05).abs() < 1e-9);
        assert!(!ok[0].over());

        // 2.3 — chegaradan chiqadi.
        let over = day_material(
            std::slice::from_ref(&entry),
            std::slice::from_ref(&norm),
            &[out(2.3)],
            day,
        );
        assert!(over[0].over());

        // Boshqa kunning chiqimi bugungi hisobga kirmaydi.
        let mut other = out(2.3);
        other.date = day - chrono::Duration::days(1);
        let clean = day_material(&[entry], &[norm], &[other], day);
        assert_eq!(clean[0].issued, 0.0);
    }

    /// TZ V.32: dastur ichki ziddiyatni ko'rsatadi — rejadan katta hajm,
    /// tabelsiz brigada, kelajak sana.
    #[test]
    fn journal_doubts_find_internal_conflicts() {
        use crate::checks::{journal_doubts, JournalDoubt as D};
        use crate::domain::JournalEntry;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let task = test_task(10, 100.0, 90.0);
        let entry = |volume: f64, workers: i64, date: chrono::NaiveDate| JournalEntry {
            id: 1,
            project_id: 1,
            date,
            author: String::new(),
            weather: String::new(),
            temperature: 0.0,
            workers,
            machines: 0,
            task_id: Some(10),
            volume,
            unit: "m3".into(),
            text: String::new(),
            remarks: String::new(),
            photos: String::new(),
            gps: String::new(),
        };

        // Qolgan hajm 10; 25 yozilgan — ziddiyat.
        let over = journal_doubts(
            &[entry(25.0, 0, today)],
            std::slice::from_ref(&task),
            &[],
            today,
        );
        assert!(over[0]
            .doubts
            .iter()
            .any(|d| matches!(d, D::VolumeOverPlan { .. })));

        // Brigada bor, tabel bo'sh.
        let crew = journal_doubts(
            &[entry(5.0, 6, today)],
            std::slice::from_ref(&task),
            &[],
            today,
        );
        assert!(crew[0]
            .doubts
            .iter()
            .any(|d| matches!(d, D::CrewWithoutTimesheet { .. })));

        // Kelajak sana.
        let future = journal_doubts(
            &[entry(5.0, 0, today + chrono::Duration::days(2))],
            std::slice::from_ref(&task),
            &[],
            today,
        );
        assert!(future[0].doubts.contains(&D::FutureDate));

        // Toza yozuv — ziddiyat yo'q.
        let clean = journal_doubts(&[entry(5.0, 0, today)], &[task], &[], today);
        assert!(clean.is_empty());
    }

    /// Bir xil hajm ketma-ket kunlarda takrorlansa — savol tug'iladi.
    #[test]
    fn repeated_volume_raises_a_question() {
        use crate::checks::{journal_doubts, JournalDoubt as D, REPEAT_DAYS};
        use crate::domain::JournalEntry;

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let task = test_task(10, 1000.0, 10.0);
        let rows: Vec<JournalEntry> = (0..REPEAT_DAYS as i64)
            .map(|i| JournalEntry {
                id: i + 1,
                project_id: 1,
                date: today - chrono::Duration::days(i),
                author: String::new(),
                weather: String::new(),
                temperature: 0.0,
                workers: 0,
                machines: 0,
                task_id: Some(10),
                volume: 12.0,
                unit: "m3".into(),
                text: String::new(),
                remarks: String::new(),
                photos: String::new(),
                gps: String::new(),
            })
            .collect();

        let found = journal_doubts(&rows, std::slice::from_ref(&task), &[], today);
        let newest = found.first().expect("shubha topilmadi");
        assert!(newest
            .doubts
            .iter()
            .any(|d| matches!(d, D::RepeatedVolume { days, .. } if *days >= REPEAT_DAYS)));

        // Bitta kun boshqacha bo'lsa — takror emas.
        let mut mixed = rows.clone();
        mixed[1].volume = 9.0;
        let mild = journal_doubts(&mixed, &[task], &[], today);
        assert!(!mild.iter().any(|c| c
            .doubts
            .iter()
            .any(|d| matches!(d, D::RepeatedVolume { .. }))));
    }

    /// TZ V.16: ertangi reja komplekt tayyorligini «Komplekt» jadvalidan
    /// oladi va to'siqli ishlarni oldinga chiqaradi.
    #[test]
    fn tomorrow_plan_uses_the_kit_readiness() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let kits = app.material_kits();
        let plan = app.tomorrow_plan();
        let tomorrow = app.today + chrono::Duration::days(1);
        let running = app.running_on(tomorrow);

        assert_eq!(plan.len(), running.len());
        for p in &plan {
            assert!(running.contains(&p.task_id));
            if let Some(k) = kits.iter().find(|k| k.task_id == p.task_id) {
                assert!((p.kit_ready - k.ready_pct).abs() < 0.001);
                assert_eq!(p.missing, k.missing);
            } else {
                // Norma kiritilmagan ish — to'siq sifatida ko'rsatilmaydi.
                assert_eq!(p.missing, 0);
            }
            assert_eq!(p.starts, !app.running_today().contains(&p.task_id));
        }

        // To'siqlilar oldinda.
        let mut seen_ready = false;
        for p in &plan {
            if p.ready() {
                seen_ready = true;
            } else {
                assert!(!seen_ready, "to'siqli ish pastga tushib qolgan");
            }
        }
    }

    /// Yordamchi: sinov uchun loyiha elementi.
    #[cfg(test)]
    fn test_element(
        id: i64,
        section: crate::model::Section,
        kind: crate::domain::ElementKind,
        mark: &str,
        size: f64,
    ) -> crate::domain::Element {
        crate::domain::Element {
            id,
            project_id: 1,
            section,
            kind,
            mark: mark.into(),
            room: String::new(),
            axis: String::new(),
            level: "1".into(),
            size,
            unit: "m2".into(),
            value: 0.0,
            value_name: String::new(),
            sheet: "L-1".into(),
            note: String::new(),
            pos: None,
        }
    }

    /// Yordamchi: elementlar orasidagi bog'lanish.
    #[cfg(test)]
    fn test_link(
        from_el: i64,
        to_el: i64,
        relation: crate::domain::Relation,
    ) -> crate::domain::ElementLink {
        crate::domain::ElementLink {
            id: 0,
            from_el,
            to_el,
            relation,
        }
    }

    /// TZ II.10: katta xona yong'in qurilmasisiz va eshiksiz qolsa —
    /// ikkala kamchilik ham kritik.
    #[test]
    fn fire_rules_ask_for_protection_and_exit() {
        use crate::checks::{check_project, Ctx};
        use crate::domain::{ElementKind, Relation, Severity};
        use crate::model::Section;
        use std::collections::HashMap;

        let norms = HashMap::new();
        let run = |elements: &[crate::domain::Element], links: &[crate::domain::ElementLink]| {
            check_project(&Ctx {
                project_id: 1,
                tasks: &[],
                elements,
                links,
                items: &[],
                declared_total: 0.0,
                norms: &norms,
                prices: &[],
            })
        };

        // Chegaradan katta xona, hech narsa bog'lanmagan.
        let room = test_element(1, Section::Ar, ElementKind::Room, "X-1", 48.0);
        let bare = run(std::slice::from_ref(&room), &[]);
        let titles: Vec<&str> = bare.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&crate::i18n::t("chk_pb_room_title")));
        // Eshiklar umuman modellanmagan: chiqish haqida savol berilmaydi —
        // bu chizmaning emas, modelning to'liqsizligi.
        assert!(!titles.contains(&crate::i18n::t("chk_pb_exit_title")));

        // Loyihada eshik bog'lanishi ishlatilgan, lekin bu xonada yo'q —
        // endi savol o'rinli.
        let other = test_element(5, Section::Ar, ElementKind::Room, "X-9", 30.0);
        let some_door = test_element(6, Section::Ar, ElementKind::Door, "D-9", 0.9);
        let modelled = run(
            &[room.clone(), other, some_door],
            &[test_link(5, 6, Relation::Contains)],
        );
        assert!(modelled
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_pb_exit_title") && i.element == "X-1"));
        for i in bare
            .iter()
            .filter(|i| i.title == crate::i18n::t("chk_pb_room_title"))
        {
            assert_eq!(i.section, Section::Pb);
            assert_eq!(i.severity, Severity::Critical);
            assert!(!i.recommendation.is_empty());
        }

        // Kichik xona tekshiruvdan chetda qoladi.
        let small = test_element(1, Section::Ar, ElementKind::Room, "X-2", 6.0);
        let quiet = run(std::slice::from_ref(&small), &[]);
        assert!(!quiet
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_pb_room_title")));

        // Datchik va eshik qo'shilsa — ikkala e'tiroz ham yopiladi.
        let device = test_element(2, Section::Pb, ElementKind::Device, "IP-1", 1.0);
        let door = test_element(3, Section::Ar, ElementKind::Door, "D-1", 0.9);
        let pipe = test_element(4, Section::Vk, ElementKind::Pipe, "T-1", 50.0);
        let full = run(
            &[room, device, door, pipe],
            &[
                test_link(2, 1, Relation::Serves),
                test_link(1, 3, Relation::Contains),
                test_link(2, 4, Relation::Related),
            ],
        );
        assert!(!full
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_pb_room_title")));
        assert!(!full
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_pb_exit_title")));
        // Suv ta'minoti bog'langani uchun bu e'tiroz ham yo'q.
        assert!(!full
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_pb_water_title")));
    }

    /// TZ II.9: kuchsiz tok qurilmasi kabelsiz va joysiz qolmasligi kerak.
    #[test]
    fn lowvoltage_device_needs_cable_and_place() {
        use crate::checks::{check_project, Ctx};
        use crate::domain::{ElementKind, Relation};
        use crate::model::Section;
        use std::collections::HashMap;

        let norms = HashMap::new();
        let run = |elements: &[crate::domain::Element], links: &[crate::domain::ElementLink]| {
            check_project(&Ctx {
                project_id: 1,
                tasks: &[],
                elements,
                links,
                items: &[],
                declared_total: 0.0,
                norms: &norms,
                prices: &[],
            })
        };

        let device = test_element(1, Section::Ss, ElementKind::Device, "SS-1", 1.0);
        let bare = run(std::slice::from_ref(&device), &[]);
        assert!(bare
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_ss_cable_title")));
        assert!(bare
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_ss_place_title")));
        for i in &bare {
            if i.title == crate::i18n::t("chk_ss_cable_title") {
                assert_eq!(i.section, Section::Ss);
            }
        }

        // Kabel bog'lansa va xona ko'rsatilsa — e'tiroz qolmaydi.
        let mut placed = device.clone();
        placed.room = "Server xonasi".into();
        let cable = test_element(2, Section::Ss, ElementKind::Cable, "K-1", 4.0);
        let wired = run(&[placed, cable], &[test_link(2, 1, Relation::Serves)]);
        assert!(!wired
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_ss_cable_title")));
        assert!(!wired
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_ss_place_title")));
    }

    /// TZ VII.26: chek-list tekshiruv turi va ish bo'limidan chiqadi,
    /// har bandi tarjimaga ega va ro'yxat qisqa qoladi.
    #[test]
    fn checklist_follows_kind_and_section() {
        use crate::checks::inspection_checklist;
        use crate::domain::InspectionKind;
        use crate::model::Section;

        for kind in InspectionKind::ALL {
            for section in Section::ALL {
                let items = inspection_checklist(*kind, section);
                assert!(items.len() >= 3, "chek-list juda qisqa");
                assert!(items.len() <= 10, "uzun ro'yxat o'qilmaydi");
                for i in &items {
                    // Har band tarjimaga ega bo'lishi shart.
                    assert_ne!(crate::i18n::t(i.key), i.key, "tarjima yo'q: {}", i.key);
                }
                // Bandlar takrorlanmaydi.
                let mut keys: Vec<&str> = items.iter().map(|i| i.key).collect();
                keys.sort_unstable();
                let before = keys.len();
                keys.dedup();
                assert_eq!(before, keys.len(), "chek-listda takror band bor");
            }
        }

        // Beton tekshiruvida namuna bandi bor, geodeziyada yo'q.
        let concrete = inspection_checklist(InspectionKind::Concrete, Section::Kj);
        assert!(concrete.iter().any(|i| i.key == "cl_concrete_sample"));
        let geodesy = inspection_checklist(InspectionKind::Geodesy, Section::Kj);
        assert!(!geodesy.iter().any(|i| i.key == "cl_concrete_sample"));

        // Payvand bandi faqat metall konstruksiyada majburiy.
        let km = inspection_checklist(InspectionKind::Hidden, Section::Km);
        assert!(km.iter().any(|i| i.key == "cl_sec_weld" && i.required));
        let kj = inspection_checklist(InspectionKind::Hidden, Section::Kj);
        assert!(kj.iter().any(|i| i.key == "cl_sec_weld" && !i.required));
    }

    /// TZ VII.35-36: yakuniy qabul tayyorligi mavjud modullardagi yozuvlardan
    /// yig'iladi — alohida hisob-kitob emas.
    #[test]
    fn final_readiness_sums_up_the_modules() {
        use crate::checks::FinalBlock as B;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let r = app.final_readiness();
        assert_eq!(r.total_checks, 7);
        assert!(
            !r.ready(),
            "namunadagi obyekt hali topshirishga tayyor emas"
        );
        assert!(
            (r.ready_pct
                - (r.total_checks - r.blocks.len()) as f64 * 100.0 / r.total_checks as f64)
                .abs()
                < 0.001
        );

        for b in &r.blocks {
            assert!(b.count() > 0);
            match b {
                B::TasksOpen { count } => assert_eq!(
                    *count,
                    app.tasks
                        .iter()
                        .filter(|t| t.progress < 99.999 && t.fact_end.is_none())
                        .count()
                ),
                B::SafetyOpen { count } => assert_eq!(
                    *count,
                    app.safety
                        .iter()
                        .filter(|s| matches!(
                            s.status,
                            crate::domain::IssueStatus::Open | crate::domain::IssueStatus::InWork
                        ))
                        .count()
                ),
                B::LabFailed { count } => assert_eq!(
                    *count,
                    app.lab_tests
                        .iter()
                        .filter(|l| l.result == crate::domain::LabTestResult::Fail)
                        .count()
                ),
                _ => {}
            }
        }

        // Eng ko'p yozuvli to'siq oldinda.
        for w in r.blocks.windows(2) {
            assert!(w[0].count() >= w[1].count());
        }
    }

    /// TZ XVIII: modelga jo'natiladigan kontekst ilova hisoblab bergan
    /// sonlardan iborat — model son o'ylab topmasligi kerak.
    #[test]
    fn llm_context_is_built_from_the_app_numbers() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let ctx = app.llm_context();
        assert!(!ctx.is_empty(), "kontekst bo'sh");
        // Obyekt nomi va sanasi bosh qismda.
        let project = app.project().expect("obyekt").clone();
        assert!(ctx.contains(&project.name));
        assert!(ctx.contains(&app.today.format("%d.%m.%Y").to_string()));

        // Yordamchining har bo'limi kontekstga tushadi va qatorlar aynan
        // ekranda ko'rinadigan sonlar bo'ladi.
        let supply = app.supply();
        let stock = app.stock();
        let cost = app.cost_summary();
        let sales = app.sales();
        let inp = app.analytics_input(&supply, &stock, &cost, &sales);
        let overview = crate::copilot::answer(crate::copilot::Intent::Overview, &inp);
        for l in &overview.lines {
            assert!(
                ctx.contains(&format!("{}: {}", l.label, l.value)),
                "kontekstda yo'q: {}",
                l.label
            );
        }

        // Kontekst chegaradan oshmaydi.
        assert!(ctx.chars().count() <= crate::llm::MAX_CONTEXT + 2);
    }

    /// Sozlama o'chiq bo'lsa savol jo'natilmaydi va sabab aytiladi.
    #[test]
    fn asking_with_the_model_off_reports_why() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        assert!(!app.llm.is_ready(), "namunada model yoqilgan turibdi");
        app.ask_llm("Nima kechikkan?".into());

        assert!(app.llm_pending.is_none(), "o'chiq sozlamada so'rov ketdi");
        assert!(app.llm_chat.is_empty(), "javobsiz savol tarixga yozildi");
        let (key, _, retry) = app.llm_error.clone().expect("xato ko'rsatilmadi");
        assert_eq!(key, "llm_err_not_configured");
        assert!(!retry, "sozlamasiz qayta urinishdan foyda yo'q");
    }

    /// Yoqilgan sozlamada savol tarixga tushadi, so'rov fonda ketadi va
    /// javob kelgach suhbat to'ldiriladi.
    #[test]
    fn chat_keeps_the_question_and_collects_the_answer() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Manzil ataylab mavjud emas: haqiqiy tarmoqqa chiqmaymiz, lekin
        // butun yo'l — savolni yozish, fon oqimi, xatoni ko'rsatish —
        // ishlashini tekshiramiz.
        app.llm = crate::llm::Config {
            enabled: true,
            endpoint: "http://127.0.0.1:1/v1/chat/completions".into(),
            model: "test-model".into(),
            api_key: "sk-test".into(),
            timeout_secs: 5,
        };

        app.ask_llm("Nima kechikkan?".into());
        assert_eq!(app.llm_chat.len(), 1);
        assert!(app.llm_chat[0].from_user);
        assert_eq!(app.llm_chat[0].text, "Nima kechikkan?");

        // Ikkinchi savol birinchisi tugamaguncha jo'natilmaydi.
        app.ask_llm("Yana bir savol".into());
        assert_eq!(app.llm_chat.len(), 1, "ikkita so'rov birga ketdi");

        // Javobni kutamiz: `llm` xususiyatisiz yig'ilishda darhol
        // «sozlanmagan» xatosi keladi, aks holda tarmoq xatosi.
        let mut waited = 0;
        while !app.poll_llm() && waited < 200 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            waited += 1;
        }
        assert!(app.llm_pending.is_none(), "so'rov tugamadi");
        assert!(app.llm_error.is_some(), "xato ko'rsatilmadi");

        // Tozalash suhbatni ham, xatoni ham olib tashlaydi.
        app.clear_llm_chat();
        assert!(app.llm_chat.is_empty());
        assert!(app.llm_error.is_none());
        assert_eq!(app.llm_tokens, 0);
    }

    /// Kalit bazada saqlanadi, lekin so'rov tanasiga hech qachon tushmaydi.
    #[test]
    fn api_key_never_leaves_the_header() {
        let t = TempDb::new();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.llm = crate::llm::Config {
            enabled: true,
            endpoint: crate::llm::DEFAULT_ENDPOINT.into(),
            model: crate::llm::DEFAULT_MODEL.into(),
            api_key: "sk-secret-value-9911".into(),
            timeout_secs: 30,
        };
        app.save_llm();

        let body = crate::llm::build_request(&app.llm, &[], "savol", "kontekst").unwrap();
        assert!(!body.contains("sk-secret-value-9911"));
        assert!(!app.llm.masked_key().contains("secret"));

        // Sozlama qayta o'qilganda tiklanadi.
        let again = crate::app::App::new(Db::open(&t.path).unwrap());
        assert_eq!(again.llm.api_key, "sk-secret-value-9911");
        assert_eq!(again.llm.timeout_secs, 30);
        assert_eq!(again.llm.model, crate::llm::DEFAULT_MODEL);
    }

    /// Umumiy izoh mexanizmi: bitta jadval har xil turdagi yozuvga xizmat
    /// qiladi va yozuvlar bir-biriga aralashib ketmaydi.
    #[test]
    fn notes_stay_with_their_record() {
        use crate::domain::{Note, NoteTarget};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let add = |target: NoteTarget, target_id: i64, text: &str| {
            t.db.insert_note(&Note {
                id: 0,
                project_id: pid,
                target,
                target_id,
                author: "Test".into(),
                at: "2026-06-20 10:00".into(),
                text: text.into(),
                parent: None,
                resolved: false,
            })
        };

        add(NoteTarget::Quality, 7, "sifat izohi");
        add(NoteTarget::Purchase, 7, "xarid izohi");
        let root = add(NoteTarget::Quality, 7, "ikkinchi sifat izohi");

        let all = t.db.notes(pid);
        let quality: Vec<_> = all
            .iter()
            .filter(|n| n.target == NoteTarget::Quality && n.target_id == 7)
            .collect();
        // Bir xil raqam, boshqa tur — aralashmaydi.
        assert_eq!(quality.len(), 2);
        assert!(all
            .iter()
            .any(|n| n.target == NoteTarget::Purchase && n.target_id == 7));

        // Javob ildizga bog'lanadi.
        t.db.insert_note(&Note {
            id: 0,
            project_id: pid,
            target: NoteTarget::Quality,
            target_id: 7,
            author: "Test".into(),
            at: "2026-06-20 11:00".into(),
            text: "javob".into(),
            parent: Some(root),
            resolved: false,
        });
        let all = t.db.notes(pid);
        let parent = all.iter().find(|n| n.id == root).expect("ildiz izoh");
        assert!(parent.has_replies(&all));

        // Izoh matni o'zgarmaydi — faqat holati.
        assert!(t.db.set_note_resolved(root, true));
        let all = t.db.notes(pid);
        let parent = all.iter().find(|n| n.id == root).unwrap();
        assert!(parent.resolved);
        assert_eq!(parent.text, "ikkinchi sifat izohi");
    }

    /// «Oldin» va «keyin» fotolari biriktirma sifatida saqlanadi va fayl
    /// joyida yo'qligi ochiq ko'rinadi (TZ VII.20, XIV.21).
    #[test]
    fn attachments_mark_before_and_after() {
        use crate::domain::{Attachment, NoteTarget, PhotoStage};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        let add = |stage: PhotoStage, path: &str| {
            t.db.insert_attachment(&Attachment {
                id: 0,
                project_id: pid,
                target: NoteTarget::Quality,
                target_id: 3,
                path: path.into(),
                stage,
                caption: String::new(),
                author: "Test".into(),
                at: "2026-06-20 10:00".into(),
            })
        };

        add(PhotoStage::Before, "C:/foto/oldin.jpg");
        let after = add(PhotoStage::After, "C:/foto/keyin.png");
        add(PhotoStage::Plain, "C:/hujjat/akt.pdf");

        let rows = t.db.attachments(pid);
        assert_eq!(rows.len(), 3);

        let before = rows
            .iter()
            .find(|a| a.stage == PhotoStage::Before)
            .expect("«oldin» fotosi");
        assert_eq!(before.file_name(), "oldin.jpg");
        assert!(before.is_photo());
        // Mavjud bo'lmagan fayl ochiq ko'rsatiladi — soxta ishonch bermaymiz.
        assert!(!before.exists());

        let doc = rows
            .iter()
            .find(|a| a.path.ends_with(".pdf"))
            .expect("hujjat");
        assert!(!doc.is_photo());

        // Bosqichni o'zgartirish mumkin.
        let mut a = rows.into_iter().find(|a| a.id == after).unwrap();
        a.stage = PhotoStage::Plain;
        a.caption = "umumiy ko'rinish".into();
        assert!(t.db.update_attachment(&a));
        let back =
            t.db.attachments(pid)
                .into_iter()
                .find(|x| x.id == after)
                .unwrap();
        assert_eq!(back.stage, PhotoStage::Plain);
        assert_eq!(back.caption, "umumiy ko'rinish");

        // Ro'yxatdan olib tashlash ishlaydi.
        assert!(t.db.delete_attachment(after));
        assert_eq!(t.db.attachments(pid).len(), 2);
    }

    /// Namunada muhokama bor: ochiq savol ham, hal qilingani ham.
    #[test]
    fn demo_has_a_discussion() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let notes = t.db.notes(pid);

        assert!(notes.len() >= 4, "namunada izoh yetarli emas");
        assert!(notes.iter().any(|n| !n.resolved), "ochiq izoh yo'q");
        assert!(notes.iter().any(|n| n.resolved), "hal qilingan izoh yo'q");
        assert!(notes.iter().any(|n| n.parent.is_some()), "javob yo'q");

        // Har izoh mavjud yozuvga tegishli.
        for n in &notes {
            let exists = match n.target {
                crate::domain::NoteTarget::Quality => {
                    t.db.quality_checks(pid).iter().any(|q| q.id == n.target_id)
                }
                crate::domain::NoteTarget::Inspection => {
                    t.db.inspections(pid).iter().any(|i| i.id == n.target_id)
                }
                crate::domain::NoteTarget::Purchase => {
                    t.db.purchases(pid).iter().any(|p| p.id == n.target_id)
                }
                _ => true,
            };
            assert!(exists, "izoh mavjud bo'lmagan yozuvga: {:?}", n.target);
        }

        // Obyekt o'chirilsa izohlar ham ketadi.
        t.db.delete_project(pid).expect("obyekt o'chirilmadi");
        assert!(t.db.notes(pid).is_empty());
    }

    /// TZ X.33, 37-38: obyektlar bo'yicha xaridlar nom bo'yicha guruhlanadi,
    /// tejash esa eng arzon narxga qarab hisoblanadi.
    #[test]
    fn central_purchases_group_by_name() {
        use crate::checks::{central_purchases, CENTRAL_SPREAD};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let buy = |project_id: i64, title: &str, qty: f64, price: f64| {
            let mut p = t.db.purchases(pid).into_iter().next().expect("xarid");
            p.id = 0;
            p.project_id = project_id;
            p.title = title.into();
            p.qty = qty;
            p.price = price;
            p
        };

        // Ikki obyekt, bitta material, har xil narx.
        let rows = vec![
            (1_i64, vec![buy(1, "Sement M400", 10.0, 100.0)]),
            (2_i64, vec![buy(2, "sement m400  ", 30.0, 130.0)]),
        ];
        let lines = central_purchases(&rows);
        assert_eq!(lines.len(), 1, "nom bo'yicha guruhlanmadi");
        let l = &lines[0];
        assert_eq!(l.objects.len(), 2);
        assert!((l.total_qty - 40.0).abs() < 1e-9);
        // Eng arzoni birinchi.
        assert!((l.best_price - 100.0).abs() < 1e-9);
        assert!((l.spread_pct - 30.0).abs() < 1e-9);
        // Tejash: 40 dona × 100 o'rniga hozir 10×100 + 30×130 = 4900.
        assert!((l.saving - 900.0).abs() < 1e-6);
        assert!(l.spread_pct > CENTRAL_SPREAD && l.worth_central());

        // Bitta obyektda olingani markazlashtirishga tushmaydi.
        let single = central_purchases(&[(1, vec![buy(1, "Gips", 5.0, 50.0)])]);
        assert!(!single[0].worth_central());
    }

    /// TZ X.42: bo'laklar yig'indisi har doim jamiga teng bo'ladi.
    #[test]
    fn split_order_keeps_the_total() {
        use crate::checks::split_order;

        // Uchga bo'linmaydigan hajm: yaxlitlash qoldig'i yo'qolmasligi kerak.
        let need = vec![(1_i64, 1.0), (2, 1.0), (3, 1.0)];
        let out = split_order(&need, 10.0, 25.0);
        assert_eq!(out.len(), 3);
        let sum: f64 = out.iter().map(|s| s.qty).sum();
        assert!((sum - 10.0).abs() < 1e-9, "yig'indi jamiga teng emas");
        for s in &out {
            assert!((s.amount - s.qty * 25.0).abs() < 1e-9);
            assert!(s.share_pct > 0.0);
        }

        // Ehtiyoj ulushiga qarab bo'linadi.
        let uneven = split_order(&[(1, 30.0), (2, 10.0)], 40.0, 1.0);
        assert!((uneven[0].qty - 30.0).abs() < 1e-6);
        assert!((uneven[1].qty - 10.0).abs() < 1e-6);
        assert!((uneven[0].share_pct - 75.0).abs() < 1e-6);

        // Ehtiyoj yo'q bo'lsa taklif ham yo'q.
        assert!(split_order(&[], 10.0, 1.0).is_empty());
        assert!(split_order(&[(1, 0.0)], 10.0, 1.0).is_empty());
    }

    /// TZ X.18-19: texnik kelishuv va tasdiqlanmagan almashtirish
    /// nazoratga tushadi; qoralama esa tekshirilmaydi.
    #[test]
    fn supply_control_watches_approval_and_substitution() {
        use crate::checks::{supply_control, SupplyIssue as S};
        use crate::domain::PurchaseStatus;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let base = t.db.purchases(pid).into_iter().next().expect("xarid");

        // Texnik kelishuvsiz buyurtma.
        let mut plain = base.clone();
        plain.status = PurchaseStatus::Ordered;
        plain.tech_ok = false;
        plain.substitute_for = None;
        plain.contract_id = None;
        plain.qty = 1.0;
        plain.price = 1000.0;
        let out = supply_control(std::slice::from_ref(&plain), &[], &[], &[], today);
        assert!(out.iter().any(|i| matches!(i, S::NoTechApproval { .. })));

        // Qoralamadan talab qilinmaydi.
        let mut draft = plain.clone();
        draft.status = PurchaseStatus::Draft;
        assert!(supply_control(std::slice::from_ref(&draft), &[], &[], &[], today).is_empty());

        // Tasdiqlanmagan almashtirish.
        let mut sub = plain.clone();
        sub.substitute_for = Some(11);
        sub.material_id = Some(22);
        let out = supply_control(std::slice::from_ref(&sub), &[], &[], &[], today);
        assert!(out
            .iter()
            .any(|i| matches!(i, S::UnapprovedSubstitute { .. })));
        assert!(out[0].severe(), "jiddiy e'tiroz oldinda emas");

        // Tasdiqlangan analog — endi faqat texnik kelishuv so'raladi.
        let alt = crate::domain::MaterialAlt {
            id: 1,
            project_id: pid,
            material_id: 11,
            alt_id: 22,
            approved_by: "Sobirov".into(),
            approved_at: Some(today),
            note: String::new(),
        };
        let out = supply_control(
            std::slice::from_ref(&sub),
            &[],
            std::slice::from_ref(&alt),
            &[],
            today,
        );
        assert!(out
            .iter()
            .any(|i| matches!(i, S::SubstituteWithoutTech { .. })));
        assert!(!out
            .iter()
            .any(|i| matches!(i, S::UnapprovedSubstitute { .. })));

        // Kelishuv olingach e'tiroz qolmaydi.
        let mut ok = sub.clone();
        ok.tech_ok = true;
        assert!(supply_control(
            std::slice::from_ref(&ok),
            &[],
            std::slice::from_ref(&alt),
            &[],
            today
        )
        .is_empty());
    }

    /// TZ X.22: ta'minot shartnomasi bo'yicha summa va muddat tekshiriladi.
    #[test]
    fn supply_contract_limits_are_checked() {
        use crate::checks::{supply_control, SupplyIssue as S};
        use crate::domain::{ContractKind, ContractStatus, PurchaseStatus};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let base = t.db.purchases(pid).into_iter().next().expect("xarid");

        let contract = crate::domain::Contract {
            id: 5,
            project_id: pid,
            number: "TA-11".into(),
            name: "Ta'minot".into(),
            kind: ContractKind::Supply,
            party_id: None,
            signed: today - chrono::Duration::days(120),
            start: today - chrono::Duration::days(120),
            end: today - chrono::Duration::days(10),
            sum: 1_000.0,
            advance_pct: 0.0,
            retention_pct: 0.0,
            currency: "UZS".into(),
            status: ContractStatus::Active,
            note: String::new(),
        };

        let mut p = base.clone();
        p.status = PurchaseStatus::Ordered;
        p.tech_ok = true;
        p.substitute_for = None;
        p.contract_id = Some(5);
        p.qty = 1.0;
        p.price = 1_500.0;
        p.date = today; // shartnoma muddati tugagach

        let out = supply_control(
            std::slice::from_ref(&p),
            std::slice::from_ref(&contract),
            &[],
            &[],
            today,
        );
        assert!(out
            .iter()
            .any(|i| matches!(i, S::ContractOverrun { over, .. } if *over > 0.0)));
        assert!(out
            .iter()
            .any(|i| matches!(i, S::ContractExpired { days, .. } if *days == 10)));

        // Shartnoma chegarasi ichida va muddatida — e'tiroz yo'q.
        let mut good = p.clone();
        good.price = 500.0;
        good.date = today - chrono::Duration::days(30);
        assert!(supply_control(
            std::slice::from_ref(&good),
            std::slice::from_ref(&contract),
            &[],
            &[],
            today
        )
        .is_empty());
    }

    /// Namunada texnik kelishuvsiz shoshilinch xarid bor — nazorat ekrani
    /// bo'sh ko'rinmasin.
    #[test]
    fn demo_shows_a_purchase_without_approval() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        assert!(
            app.purchases.iter().any(|p| p.tech_ok),
            "kelishuvdan o'tgan xarid yo'q"
        );
        assert!(
            app.purchases.iter().any(|p| !p.tech_ok),
            "kelishuvsiz xarid yo'q"
        );
        let control = app.supply_control();
        assert!(!control.is_empty(), "nazorat ekrani bo'sh");
    }

    /// TZ XVII.49: umumiy ball o'z modullaridagi ballardan yig'iladi va
    /// vaznlar yig'indisi 100 ni beradi.
    #[test]
    fn executive_score_is_a_weighted_sum() {
        use crate::checks::{executive_score, ExecCtx, RequiredDoc};

        let ctx = |quality: f64, safety: f64| ExecCtx {
            delay_days: 0,
            overdue: 0,
            tasks: 10,
            earned: 100.0,
            actual: 100.0,
            quality_score: quality,
            safety_score: safety,
            supply: &[],
            required_docs: &[],
        };

        // Hammasi ideal — ball 100.
        let best = executive_score(&ctx(100.0, 100.0));
        assert!((best.total - 100.0).abs() < 0.001);
        assert!((best.schedule - 100.0).abs() < 0.001);
        assert!((best.docs - 100.0).abs() < 0.001);

        // Sifat va xavfsizlik ballari o'z modulidan olinadi — qayta
        // hisoblanmaydi.
        let mid = executive_score(&ctx(60.0, 40.0));
        assert!((mid.quality - 60.0).abs() < 0.001);
        assert!((mid.safety - 40.0).abs() < 0.001);
        // 40×0.25 + 60×0.25 + 100×0.20 + 100×0.15 + 100×0.10 + 100×0.05
        //  = 10 + 15 + 20 + 15 + 10 + 5 = 75.
        assert!(
            (mid.total - 75.0).abs() < 0.001,
            "vazn buzilgan: {}",
            mid.total
        );

        // Kechikish muddat ballini pasaytiradi.
        let mut late = ctx(100.0, 100.0);
        late.delay_days = 30;
        late.overdue = 4;
        let r = executive_score(&late);
        assert!(
            (r.schedule - 50.0).abs() < 0.001,
            "muddat balli: {}",
            r.schedule
        );

        // Ortiqcha sarf pul ballini pasaytiradi.
        let mut over = ctx(100.0, 100.0);
        over.actual = 130.0;
        assert!((executive_score(&over).money - 70.0).abs() < 0.001);

        // Hujjatlar ulushi imzolanganlariga qarab.
        let docs = vec![
            RequiredDoc {
                task_id: 1,
                task_name: String::new(),
                section: crate::model::Section::Kj,
                kind: crate::domain::ExecDocKind::Hidden,
                task_done: true,
                exists: true,
                signed: true,
            },
            RequiredDoc {
                task_id: 2,
                task_name: String::new(),
                section: crate::model::Section::Kj,
                kind: crate::domain::ExecDocKind::Hidden,
                task_done: true,
                exists: false,
                signed: false,
            },
        ];
        let mut with_docs = ctx(100.0, 100.0);
        with_docs.required_docs = &docs;
        assert!((executive_score(&with_docs).docs - 50.0).abs() < 0.001);
    }

    /// TZ XVII.23: mas'ul bo'yicha sifat balli sifat modulidagi bilan
    /// bir xil bo'ladi.
    #[test]
    fn contractor_report_reuses_the_quality_score() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.contractor_report();
        assert!(!rows.is_empty(), "mas'ullar topilmadi");
        let by_quality = crate::checks::contractor_quality(&app.quality, app.today);

        for c in &rows {
            assert!(c.tasks > 0);
            assert!(c.done <= c.tasks);
            assert!(c.on_time_pct >= 0.0 && c.on_time_pct <= 100.0);
            if let Some(q) = by_quality
                .iter()
                .find(|x| x.name.trim().to_lowercase() == c.name.trim().to_lowercase())
            {
                assert!((c.quality - q.score).abs() < 0.001, "ball mos kelmadi");
                assert_eq!(c.defects, q.open_defects);
            } else {
                // Sifat yozuvi yo'q mas'ul — ball tushirilmaydi.
                assert!((c.quality - 100.0).abs() < 0.001);
            }
        }

        // E'tibor talab qiladiganlar oldinda.
        let mut seen_calm = false;
        for c in &rows {
            if c.attention() {
                assert!(!seen_calm, "e'tibor talab qiladigan mas'ul pastga tushgan");
            } else {
                seen_calm = true;
            }
        }
    }

    /// TZ XVII.24: ta'minotchi balliga narx kirmaydi — arzon, lekin
    /// kechikadigan ta'minotchi yaxshi ko'rinib qolmasligi kerak.
    #[test]
    fn supplier_score_ignores_price() {
        use crate::checks::SupplierReport;

        let base = SupplierReport {
            name: "A".into(),
            deals: 4,
            amount: 100.0,
            complete_pct: 100.0,
            on_time_pct: 100.0,
            avg_delay: 0.0,
            price_over_pct: 0.0,
            rejected: 0,
        };
        assert!((base.score() - 100.0).abs() < 0.001);

        // Narx ikki barobar qimmat bo'lsa ham ball o'zgarmaydi.
        let mut pricey = base.clone();
        pricey.price_over_pct = 100.0;
        assert!((pricey.score() - base.score()).abs() < 0.001);

        // Kechikish esa ballni tushiradi.
        let mut late = base.clone();
        late.on_time_pct = 40.0;
        assert!(late.score() < base.score());

        // Rad etilgan partiya ham.
        let mut bad = base.clone();
        bad.rejected = 2;
        assert!((bad.score() - (100.0 + 100.0 + 50.0) / 3.0).abs() < 0.001);
    }

    /// TZ XVII.25: bog'liqlik faqat yetarli juftlik va yetarli kuchda
    /// e'tiborga olinadi.
    #[test]
    fn correlation_needs_strength_and_points() {
        use crate::checks::{pearson, Correlation, CORR_LIMIT, CORR_MIN_POINTS};

        // To'g'ri chiziqli bog'liqlik.
        let xs: Vec<f64> = (1..=12).map(|i| i as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|x| x * 3.0 + 1.0).collect();
        assert!((pearson(&xs, &ys) - 1.0).abs() < 1e-9);

        // Teskari bog'liqlik.
        let inv: Vec<f64> = xs.iter().map(|x| -x).collect();
        assert!((pearson(&xs, &inv) + 1.0).abs() < 1e-9);

        // Dispersiya nol — bog'liqlik aniqlanmaydi.
        let flat = vec![5.0; 12];
        assert_eq!(pearson(&xs, &flat), 0.0);
        // Ikki nuqtadan kam — hisoblanmaydi.
        assert_eq!(pearson(&[1.0], &[2.0]), 0.0);

        // Kuchli, lekin juftlik kam — e'tiborga olinmaydi.
        let weak_points = Correlation {
            key: "corr_crew_volume",
            r: 0.99,
            points: CORR_MIN_POINTS - 1,
        };
        assert!(!weak_points.meaningful());
        // Juftlik ko'p, lekin kuchsiz — ham olinmaydi.
        let weak_r = Correlation {
            key: "corr_crew_volume",
            r: CORR_LIMIT - 0.01,
            points: 50,
        };
        assert!(!weak_r.meaningful());
        // Ikkalasi ham yetarli.
        let good = Correlation {
            key: "corr_crew_volume",
            r: -0.8,
            points: 20,
        };
        assert!(good.meaningful());
    }

    /// TZ XVII.35: tasdiqlangan va qaror kutayotgan summalar alohida
    /// turadi — qaror kutayotgani hali pul emas.
    #[test]
    fn change_report_separates_approved_from_pending() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let r = app.change_report();
        assert_eq!(r.total, app.contract_changes.len());
        assert_eq!(r.approved + r.pending + r.rejected, r.total);

        let expected_approved: f64 = app
            .contract_changes
            .iter()
            .filter(|c| c.status == crate::domain::ChangeStatus::Approved)
            .map(|c| c.amount)
            .sum();
        assert!((r.approved_sum - expected_approved).abs() < 0.01);
        // Qaror kutayotgani tasdiqlangan summaga qo'shilmaydi.
        assert!(r.pending_sum >= 0.0);
        assert!((r.approved_sum + r.pending_sum) >= r.approved_sum);

        // Turlar bo'yicha yig'indi jamiga teng.
        let by_kind: usize = r.by_kind.iter().map(|(_, n, _)| n).sum();
        assert_eq!(by_kind, r.total);
        // Eng katta summali tur oldinda.
        for w in r.by_kind.windows(2) {
            assert!(w[0].2 >= w[1].2);
        }
    }

    /// Haftalik hisobot bitta chaqiruv nuqtasidan chiqadi: buyurtmachi
    /// kabineti va analitika bir xil sonni ko'rsatadi.
    #[test]
    fn week_report_has_one_source() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let a = app.week_report();
        let b = app.week_report();
        assert_eq!(a.from, b.from);
        assert_eq!(a.tasks_done, b.tasks_done);
        assert_eq!(a.docs_signed, b.docs_signed);
        assert_eq!(a.to, app.today);
        assert_eq!((a.to - a.from).num_days(), 7);
    }

    /// TZ XI.8: kirim hujjatsiz yoki partiyasiz bo'lsa e'tiroz beriladi,
    /// chiqim esa bu qoidaga tushmaydi.
    #[test]
    fn intake_needs_document_and_batch() {
        use crate::checks::{stock_control, StockCtx, StockIssue as S};
        use crate::domain::{Material, MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let material = Material {
            id: 4,
            project_id: 1,
            code: "M-4".into(),
            name: "Sement".into(),
            unit: "t".into(),
            section: crate::model::Section::Kj,
            spec: "PC 400".into(),
            cert_no: "SS-1".into(),
            cert_until: Some(today + chrono::Duration::days(200)),
            min_stock: 0.0,
            price: 100.0,
            estimate_code: "E-1".into(),
            spec_ref: "S-1".into(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let mv = |kind: MoveKind, doc: &str, batch: Option<i64>| StockMove {
            id: 1,
            project_id: 1,
            material_id: 4,
            warehouse_id: None,
            batch_id: batch,
            date: today,
            kind,
            qty: 5.0,
            price: 100.0,
            document: doc.into(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
        };
        let run = |moves: &[StockMove]| {
            stock_control(&StockCtx {
                moves,
                materials: std::slice::from_ref(&material),
                purchases: &[],
                warehouses: &[],
                machine_logs: &[],
                today,
            })
        };

        // Hujjatsiz va partiyasiz kirim — ikkita e'tiroz.
        let bad = run(&[mv(MoveKind::In, "", None)]);
        assert!(bad.iter().any(|i| matches!(i, S::IntakeNoDocument { .. })));
        assert!(bad.iter().any(|i| matches!(i, S::IntakeNoBatch { .. })));

        // To'g'ri kirim — e'tiroz yo'q.
        assert!(run(&[mv(MoveKind::In, "TTN-15", Some(2))]).is_empty());

        // Chiqim hujjatsiz bo'lsa ham kirim qoidasi qo'llanmaydi.
        let out = run(&[mv(MoveKind::Out, "", None)]);
        assert!(!out.iter().any(|i| matches!(i, S::IntakeNoDocument { .. })));
    }

    /// TZ XI.13: smeta rasenkasiga bog'lanmagan material chiqimi ko'rinadi.
    #[test]
    fn issued_material_without_estimate_code_is_flagged() {
        use crate::checks::{stock_control, StockCtx, StockIssue as S};
        use crate::domain::{Material, MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let mut material = Material {
            id: 9,
            project_id: 1,
            code: "M-9".into(),
            name: "Qum".into(),
            unit: "m3".into(),
            section: crate::model::Section::Kj,
            spec: "yirik".into(),
            cert_no: "SS-9".into(),
            cert_until: None,
            min_stock: 0.0,
            price: 50.0,
            estimate_code: String::new(),
            spec_ref: "S-9".into(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let out = StockMove {
            id: 1,
            project_id: 1,
            material_id: 9,
            warehouse_id: None,
            batch_id: None,
            date: today,
            kind: MoveKind::Out,
            qty: 4.0,
            price: 50.0,
            document: "TN-1".into(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
        };
        let run = |m: &Material| {
            stock_control(&StockCtx {
                moves: std::slice::from_ref(&out),
                materials: std::slice::from_ref(m),
                purchases: &[],
                warehouses: &[],
                machine_logs: &[],
                today,
            })
        };

        let flagged = run(&material);
        // Summa chiqim qiymatidan olinadi: 4 × 50 = 200.
        assert!(flagged.iter().any(
            |i| matches!(i, S::NoEstimateLink { amount, .. } if (*amount - 200.0).abs() < 1e-9)
        ));

        // Rasenka to'ldirilgach e'tiroz yo'qoladi.
        material.estimate_code = "E-9".into();
        assert!(!run(&material)
            .iter()
            .any(|i| matches!(i, S::NoEstimateLink { .. })));
    }

    /// TZ XI.31: omborda berilgan va texnikaga yozilgan yoqilg'i farqi
    /// chegaradan oshsa ko'rsatiladi.
    #[test]
    fn fuel_gap_is_reported_beyond_the_limit() {
        use crate::checks::{stock_control, StockCtx, StockIssue as S, FUEL_GAP_PCT};
        use crate::domain::{MachineLog, Material, MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let fuel = Material {
            id: 2,
            project_id: 1,
            code: "F-1".into(),
            name: "Dizel yoqilg'isi".into(),
            unit: "l".into(),
            section: crate::model::Section::None,
            spec: "DT".into(),
            cert_no: "SS-2".into(),
            cert_until: None,
            min_stock: 0.0,
            price: 10.0,
            estimate_code: "E-2".into(),
            spec_ref: "S-2".into(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let issue = StockMove {
            id: 1,
            project_id: 1,
            material_id: 2,
            warehouse_id: None,
            batch_id: None,
            date: today,
            kind: MoveKind::Out,
            qty: 1000.0,
            price: 10.0,
            document: "TN-2".into(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
        };
        let log = |fuel: f64| MachineLog {
            id: 1,
            project_id: 1,
            machine_id: 1,
            date: today,
            hours: 8.0,
            fuel,
            task_id: None,
            number: String::new(),
            driver: String::new(),
            route: String::new(),
            odo_start: 0.0,
            odo_end: 0.0,
            trips: 0,
            cargo: 0.0,
            note: String::new(),
            gps: String::new(),
        };
        let run = |logs: &[MachineLog]| {
            stock_control(&StockCtx {
                moves: std::slice::from_ref(&issue),
                materials: std::slice::from_ref(&fuel),
                purchases: &[],
                warehouses: &[],
                machine_logs: logs,
                today,
            })
        };

        // Chegara ichidagi farq — e'tiroz yo'q.
        let ok = run(&[log(1000.0 * (1.0 - FUEL_GAP_PCT / 100.0) + 1.0)]);
        assert!(!ok.iter().any(|i| matches!(i, S::FuelGap { .. })));

        // Yarmi yozilmagan — e'tiroz.
        let gap = run(&[log(500.0)]);
        let found = gap
            .iter()
            .find(|i| matches!(i, S::FuelGap { .. }))
            .expect("yoqilg'i farqi topilmadi");
        assert!(found.severe());
        if let S::FuelGap { issued, used, diff } = found {
            assert!((issued - 1000.0).abs() < 1e-9);
            assert!((used - 500.0).abs() < 1e-9);
            assert!((diff - 500.0).abs() < 1e-9);
        }
    }

    /// TZ XI.29: harorat talabi faqat yozgi oylarda va ochiq omborda
    /// tekshiriladi — qishda bu e'tiroz o'rinsiz.
    #[test]
    fn temperature_check_only_in_hot_months() {
        use crate::checks::{stock_control, StockCtx, StockIssue as S};
        use crate::domain::{Material, MoveKind, StockMove, Warehouse, WarehouseKind};

        let material = Material {
            id: 3,
            project_id: 1,
            code: "M-3".into(),
            name: "Gidroizolyatsiya".into(),
            unit: "m2".into(),
            section: crate::model::Section::Kj,
            spec: "rulon".into(),
            cert_no: "SS-3".into(),
            cert_until: None,
            min_stock: 0.0,
            price: 20.0,
            estimate_code: "E-3".into(),
            spec_ref: "S-3".into(),
            special: "saqlash harorati +5..+30".into(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        };
        let open = Warehouse {
            id: 7,
            project_id: 1,
            name: "Ochiq maydon".into(),
            kind: WarehouseKind::Open,
            responsible: String::new(),
            note: String::new(),
        };
        let mv = |date: chrono::NaiveDate| StockMove {
            id: 1,
            project_id: 1,
            material_id: 3,
            warehouse_id: Some(7),
            batch_id: None,
            date,
            kind: MoveKind::In,
            qty: 100.0,
            price: 20.0,
            document: "TTN-3".into(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
        };
        let run = |today: chrono::NaiveDate, wh: &[Warehouse]| {
            let m = mv(today);
            stock_control(&StockCtx {
                moves: std::slice::from_ref(&m),
                materials: std::slice::from_ref(&material),
                purchases: &[],
                warehouses: wh,
                machine_logs: &[],
                today,
            })
        };

        // Iyul — ochiq omborda xavf bor.
        let july = chrono::NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        assert!(run(july, std::slice::from_ref(&open))
            .iter()
            .any(|i| matches!(i, S::TemperatureRisk { .. })));

        // Yanvar — bu e'tiroz o'rinsiz.
        let january = chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        assert!(!run(january, std::slice::from_ref(&open))
            .iter()
            .any(|i| matches!(i, S::TemperatureRisk { .. })));

        // Yopiq ombor — xavf yo'q.
        let mut closed = open.clone();
        closed.kind = WarehouseKind::Central;
        assert!(!run(july, std::slice::from_ref(&closed))
            .iter()
            .any(|i| matches!(i, S::TemperatureRisk { .. })));
    }

    /// TZ XVI.28: bugun ishlagan, lekin ko'rikdan o'tmagan texnika
    /// ko'rsatiladi; ishlamagani esa so'ralmaydi.
    #[test]
    fn daily_check_is_asked_only_for_working_machines() {
        use crate::checks::{mech_issues, MechCtx, MechIssue as M};
        use crate::domain::{MachineCheck, MachineLog};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let machines = t.db.machines(pid);
        let m = machines.first().expect("texnika").clone();

        let log = MachineLog {
            id: 1,
            project_id: pid,
            machine_id: m.id,
            date: today,
            hours: 8.0,
            fuel: 0.0,
            task_id: None,
            number: String::new(),
            driver: String::new(),
            route: String::new(),
            odo_start: 0.0,
            odo_end: 0.0,
            trips: 0,
            cargo: 0.0,
            note: String::new(),
            gps: String::new(),
        };
        let mut clean = m.clone();
        clean.inspection_until = Some(today + chrono::Duration::days(90));
        clean.service_hours = 0.0;
        clean.operator = String::new();
        clean.kind = crate::domain::MachineKind::Truck;

        let run = |logs: &[MachineLog], checks: &[MachineCheck]| {
            mech_issues(&MechCtx {
                machines: std::slice::from_ref(&clean),
                logs,
                checks,
                workers: &[],
                permits: &[],
                today,
            })
        };

        // Ishlagan, ko'rik yo'q — e'tiroz (va operator ko'rsatilmagani).
        let out = run(std::slice::from_ref(&log), &[]);
        assert!(out.iter().any(|i| matches!(i, M::NoDailyCheck { .. })));
        assert!(out.iter().any(|i| matches!(i, M::NoOperator { .. })));

        // Ishlamagan — ko'rik ham so'ralmaydi.
        let idle = run(&[], &[]);
        assert!(!idle.iter().any(|i| matches!(i, M::NoDailyCheck { .. })));
        assert!(!idle.iter().any(|i| matches!(i, M::NoOperator { .. })));

        // Ko'rik yozilgan — e'tiroz yo'q.
        let check = MachineCheck {
            id: 1,
            project_id: pid,
            machine_id: clean.id,
            date: today,
            by: "Test".into(),
            items_ok: 6,
            items_total: 6,
            fault: String::new(),
            allowed: true,
            note: String::new(),
        };
        let ok = run(std::slice::from_ref(&log), std::slice::from_ref(&check));
        assert!(!ok.iter().any(|i| matches!(i, M::NoDailyCheck { .. })));

        // Nosozlik topilgan, texnika esa ishlashda — to'xtatish kerak.
        let mut faulty = check.clone();
        faulty.fault = "tormoz".into();
        let bad = run(std::slice::from_ref(&log), std::slice::from_ref(&faulty));
        let found = bad
            .iter()
            .find(|i| matches!(i, M::FaultButWorking { .. }))
            .expect("nosozlik e'tirozi yo'q");
        assert!(found.stop());

        // Ruxsat berilmagan, lekin ishlatilgan — bu ham to'xtatish.
        let mut blocked = check.clone();
        blocked.allowed = false;
        let used = run(std::slice::from_ref(&log), std::slice::from_ref(&blocked));
        assert!(used
            .iter()
            .any(|i| matches!(i, M::NotAllowedButUsed { .. })));
    }

    /// TZ XVI.31: ko'targich texnikasi operatoriga amaldagi ruxsat kerak.
    #[test]
    fn crane_operator_needs_a_valid_permit() {
        use crate::checks::{mech_issues, MechCtx, MechIssue as M};
        use crate::domain::{MachineKind, PermitKind, WorkerPermit};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let worker = t.db.workers(pid).into_iter().next().expect("ishchi");

        let mut crane = t.db.machines(pid).into_iter().next().expect("texnika");
        crane.kind = MachineKind::Crane;
        crane.operator = worker.name.clone();
        crane.inspection_until = Some(today + chrono::Duration::days(60));
        crane.service_hours = 0.0;

        let permit = |until: chrono::NaiveDate| WorkerPermit {
            id: 1,
            project_id: pid,
            worker_id: worker.id,
            kind: PermitKind::Lifting,
            number: "L-1".into(),
            issued: today - chrono::Duration::days(300),
            valid_until: until,
            note: String::new(),
        };
        let run = |permits: &[WorkerPermit]| {
            mech_issues(&MechCtx {
                machines: std::slice::from_ref(&crane),
                logs: &[],
                checks: &[],
                workers: &t.db.workers(pid),
                permits,
                today,
            })
        };

        // Ruxsat yo'q — e'tiroz va u to'xtatish darajasida.
        let none = run(&[]);
        let found = none
            .iter()
            .find(|i| matches!(i, M::OperatorNoPermit { .. }))
            .expect("ruxsat e'tirozi yo'q");
        assert!(found.stop());

        // Muddati o'tgan ruxsat ham hisoblanmaydi.
        let expired = permit(today - chrono::Duration::days(1));
        assert!(run(std::slice::from_ref(&expired))
            .iter()
            .any(|i| matches!(i, M::OperatorNoPermit { .. })));

        // Amaldagi ruxsat — e'tiroz yo'q.
        let valid = permit(today + chrono::Duration::days(100));
        assert!(!run(std::slice::from_ref(&valid))
            .iter()
            .any(|i| matches!(i, M::OperatorNoPermit { .. })));
    }

    /// TZ XVI.39: park tavsiyalari foydalanish koeffitsiyentiga qarab
    /// beriladi va o'z texnikasi bilan ijara texnikasi bir xil emas.
    #[test]
    fn park_advice_depends_on_usage_and_ownership() {
        use crate::checks::{park_review, ParkAdvice as A, PARK_BUSY_PCT, PARK_IDLE_PCT};
        use crate::domain::MachineLog;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let mut own = t.db.machines(pid).into_iter().next().expect("texnika");
        own.rented = false;
        own.hour_rate = 100.0;
        let mut rented = own.clone();
        rented.id = own.id + 1000;
        rented.rented = true;

        let logs = |machine_id: i64, hours_per_day: f64, days: i64| -> Vec<MachineLog> {
            (0..days)
                .map(|i| MachineLog {
                    id: i + 1,
                    project_id: pid,
                    machine_id,
                    date: today - chrono::Duration::days(i),
                    hours: hours_per_day,
                    fuel: 0.0,
                    task_id: None,
                    number: String::new(),
                    driver: String::new(),
                    route: String::new(),
                    odo_start: 0.0,
                    odo_end: 0.0,
                    trips: 0,
                    cargo: 0.0,
                    note: String::new(),
                    gps: String::new(),
                })
                .collect()
        };

        // O'z texnikasi kam ishlatilgan.
        let idle = logs(own.id, 1.0, 30);
        let (lines, advice) = park_review(std::slice::from_ref(&own), &idle, today, 30);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].usage_pct < PARK_IDLE_PCT);
        // Xarajat: 30 kun × 1 soat × 100.
        assert!((lines[0].period_cost - 3_000.0).abs() < 1e-6);
        assert!(advice.iter().any(|a| matches!(a, A::OwnIdle { .. })));

        // Ijara texnikasi doim ishda.
        let busy = logs(rented.id, 8.0, 30);
        let (_, advice) = park_review(std::slice::from_ref(&rented), &busy, today, 30);
        assert!(advice.iter().any(|a| matches!(a, A::RentedBusy { .. })));

        // Ijara texnikasi kam ishlatilgan — tavsiya boshqacha.
        let rented_idle = logs(rented.id, 1.0, 30);
        let (_, advice) = park_review(std::slice::from_ref(&rented), &rented_idle, today, 30);
        assert!(advice.iter().any(|a| matches!(a, A::RentedIdle { .. })));

        // O'rtacha foydalanish — tavsiya berilmaydi.
        let normal = logs(own.id, 5.0, 30);
        let (lines, advice) = park_review(std::slice::from_ref(&own), &normal, today, 30);
        assert!(lines[0].usage_pct > PARK_IDLE_PCT && lines[0].usage_pct < PARK_BUSY_PCT);
        assert!(advice.is_empty());
    }

    /// Namunada bitta texnika ataylab ko'riksiz qoladi — mexanik kabineti
    /// bo'sh ko'rinmasin.
    #[test]
    fn demo_leaves_one_machine_unchecked() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let checks = t.db.machine_checks(pid);
        let machines = t.db.machines(pid);

        assert!(!checks.is_empty(), "namunada ko'rik yozuvi yo'q");
        assert!(
            checks.len() < machines.len(),
            "hamma texnika ko'rikdan o'tgan — e'tiroz ko'rinmaydi"
        );
        // Nosozlik topilgan, lekin bartaraf etilgan yozuv ham bor.
        assert!(checks.iter().any(|c| !c.fault.trim().is_empty()));
        assert!(checks.iter().any(|c| c.complete()));
    }

    /// TZ III.9: bir xil rasenka kodi turli narxda bo'lsa ko'rsatiladi.
    #[test]
    fn same_code_must_have_one_price() {
        use crate::checks::{estimate_deep, DeepIssue as D};

        let item = |pos: i64, code: &str, name: &str, price: f64| crate::domain::EstimateItem {
            id: pos,
            estimate_id: 1,
            pos,
            section: crate::model::Section::Kj,
            code: code.into(),
            name: name.into(),
            unit: "m3".into(),
            qty: 10.0,
            price,
            cost: 10.0 * price,
            task_id: None,
            note: String::new(),
        };

        // Bir kod, ikki narx.
        let out = estimate_deep(
            &[
                item(1, "E-11-1", "B25 beton quyish plita", 100.0),
                item(2, "E-11-1", "B25 beton quyish ustun", 130.0),
            ],
            &[],
            &[],
            &[],
        );
        let found = out
            .iter()
            .find(|i| matches!(i, D::SamePriceCode { .. }))
            .expect("kod e'tirozi yo'q");
        assert!(found.money());
        if let D::SamePriceCode { low, high, .. } = found {
            assert!((low - 100.0).abs() < 1e-9);
            assert!((high - 130.0).abs() < 1e-9);
        }

        // Bir kod, bir narx — e'tiroz yo'q.
        let same = estimate_deep(
            &[
                item(1, "E-11-1", "B25 beton quyish plita", 100.0),
                item(2, "E-11-1", "B25 beton quyish ustun", 100.0),
            ],
            &[],
            &[],
            &[],
        );
        assert!(!same.iter().any(|i| matches!(i, D::SamePriceCode { .. })));
    }

    /// TZ III.13: konstruksiya ishida marka yoki standart bo'lishi shart;
    /// arxitektura pozitsiyasidan bu talab qilinmaydi.
    #[test]
    fn structural_items_need_a_mark() {
        use crate::checks::{estimate_deep, DeepIssue as D};

        let item = |section: crate::model::Section, name: &str| crate::domain::EstimateItem {
            id: 1,
            estimate_id: 1,
            pos: 1,
            section,
            code: "E-1".into(),
            name: name.into(),
            unit: "m3".into(),
            qty: 1.0,
            price: 1.0,
            cost: 1.0,
            task_id: None,
            note: String::new(),
        };
        let has_mark = |it: crate::domain::EstimateItem| {
            !estimate_deep(std::slice::from_ref(&it), &[], &[], &[])
                .iter()
                .any(|i| matches!(i, D::NoMark { .. }))
        };

        // Markasiz konstruksiya ishi — e'tiroz.
        assert!(!has_mark(item(crate::model::Section::Kj, "Beton quyish")));
        // Marka bor — e'tiroz yo'q.
        assert!(has_mark(item(
            crate::model::Section::Kj,
            "B25 beton quyish"
        )));
        // Standart havolasi ham yetadi.
        assert!(has_mark(item(
            crate::model::Section::Km,
            "Metall konstruksiya GOST bo'yicha"
        )));
        // Arxitektura pozitsiyasidan marka talab qilinmaydi.
        assert!(has_mark(item(crate::model::Section::Ar, "Bo'yash")));
    }

    /// TZ III.16: smeta narxi tijorat taklifidan chegaradan ko'p farq
    /// qilsa ko'rsatiladi.
    #[test]
    fn estimate_price_is_compared_with_quotes() {
        use crate::checks::{estimate_deep, DeepIssue as D, QUOTE_GAP_PCT};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let quote = t.db.quotes(pid).into_iter().next().expect("taklif");

        let item = |price: f64| crate::domain::EstimateItem {
            id: 1,
            estimate_id: 1,
            pos: 5,
            section: crate::model::Section::Kj,
            code: "E-1".into(),
            // Nom taklif nomiga mos bo'lishi kerak.
            name: quote.title.clone(),
            unit: quote.unit.clone(),
            qty: 1.0,
            price,
            cost: price,
            task_id: None,
            note: String::new(),
        };

        // Chegara ichida — e'tiroz yo'q.
        let near = item(quote.price * (1.0 + QUOTE_GAP_PCT / 200.0));
        assert!(!estimate_deep(
            std::slice::from_ref(&near),
            &[],
            &[],
            std::slice::from_ref(&quote)
        )
        .iter()
        .any(|i| matches!(i, D::QuoteGap { .. })));

        // Ikki barobar qimmat — e'tiroz.
        let pricey = item(quote.price * 2.0);
        let out = estimate_deep(
            std::slice::from_ref(&pricey),
            &[],
            &[],
            std::slice::from_ref(&quote),
        );
        let found = out
            .iter()
            .find(|i| matches!(i, D::QuoteGap { .. }))
            .expect("narx farqi topilmadi");
        if let D::QuoteGap { pct, .. } = found {
            assert!((pct - 100.0).abs() < 0.001);
        }
    }

    /// TZ IX.13, 27: ariza tekshiruvi loyihaga muvofiqlikni va xodim
    /// ehtiyojini ham ko'radi.
    #[test]
    fn request_check_covers_spec_and_staff() {
        use crate::checks::{request_issues, RequestIssue as I, StaffForecast};
        use crate::domain::{RequestKind, RequestStatus};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Spetsifikatsiya havolasi bo'sh material.
        let mut material = app.materials[0].clone();
        material.spec_ref = String::new();
        let mut r = app.requests[0].clone();
        r.material_id = Some(material.id);
        r.kind = RequestKind::Material;
        r.status = RequestStatus::New;

        let out = request_issues(
            &r,
            &[],
            std::slice::from_ref(&material),
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            None,
        );
        assert!(out.contains(&I::NoSpecRef));

        // Havola to'ldirilgach — bu e'tiroz yo'qoladi.
        material.spec_ref = "AR-04, poz. 12".into();
        let out = request_issues(
            &r,
            &[],
            std::slice::from_ref(&material),
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            None,
        );
        assert!(!out.contains(&I::NoSpecRef));

        // Xodimga ariza: kasb ko'rsatilmagan va brigada yetarli.
        let mut labor = r.clone();
        labor.kind = RequestKind::Labor;
        labor.title = "—".into();
        labor.material_id = None;
        let enough = StaffForecast {
            have: 12,
            needed_hours: 100.0,
            needed_workers: 8,
            gap: -4,
            tasks: 3,
        };
        let out = request_issues(
            &labor,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            Some(&enough),
        );
        assert!(out.contains(&I::NoProfession));
        assert!(out
            .iter()
            .any(|i| matches!(i, I::StaffEnough { have: 12, need: 8 })));

        // Yetishmovchilik bo'lsa — bu savol berilmaydi.
        let short = StaffForecast {
            gap: 5,
            ..enough.clone()
        };
        let out = request_issues(&labor, &[], &[], &[], &[], &[], &[], &[], &[], Some(&short));
        assert!(!out.iter().any(|i| matches!(i, I::StaffEnough { .. })));
    }

    /// Xodim ehtiyoji bitta chaqiruv nuqtasidan chiqadi.
    #[test]
    fn staff_forecast_has_one_source() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let a = app.staff_forecast();
        let b = app.staff_forecast();
        assert_eq!(a.have, b.have);
        assert_eq!(a.needed_workers, b.needed_workers);
        assert_eq!(a.gap, b.gap);
        // Ishchi soni faol ishchilar soniga teng.
        assert_eq!(a.have, app.workers.iter().filter(|w| w.active).count());
    }

    /// TZ VII.30: yangi versiya kelganda eskisi arxivda qoladi va
    /// undan hech narsa talab qilinmaydi.
    #[test]
    fn document_versions_are_archived_not_deleted() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let docs = t.db.documents(pid);

        let new = docs
            .iter()
            .find(|d| d.version == 2)
            .expect("namunada ikkinchi versiya yo'q");
        let old_id = new.replaces.expect("yangi versiya eskisiga bog'lanmagan");
        let old = docs
            .iter()
            .find(|d| d.id == old_id)
            .expect("eski versiya o'chib ketgan");

        assert!(old.superseded(&docs));
        assert!(!new.superseded(&docs));
        assert_eq!(old.section, new.section);
        assert!(!new.revision.trim().is_empty());
        assert!(new.label().contains("v2"));
        // Eski versiya ham topshirilgan bo'lgan — u ishlatilgan.
        assert!(old.issued.is_some());
    }

    /// TZ VII.32: yangi chizma topshirilgach, undan oldin tugatilgan
    /// ishlar ogohlantirish sifatida ko'rsatiladi.
    #[test]
    fn work_finished_before_a_new_drawing_is_flagged() {
        use crate::checks::VersionIssue as V;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let issues = app.version_issues();
        for i in &issues {
            match i {
                V::WorkDoneBefore { tasks, .. } => assert!(*tasks > 0),
                V::NotIssued { revision, .. } => assert!(!revision.is_empty()),
                _ => {}
            }
        }

        // Xavflilari oldinda.
        let mut seen_soft = false;
        for i in &issues {
            if i.risky() {
                assert!(!seen_soft, "xavfli e'tiroz pastga tushib qolgan");
            } else {
                seen_soft = true;
            }
        }

        // Topshirilmagan versiya — xavf.
        let mut doc = app
            .documents
            .iter()
            .find(|d| d.version == 2)
            .cloned()
            .expect("ikkinchi versiya");
        doc.issued = None;
        assert!(app.db.update_document(&doc));
        app.reload_modules();
        let after = app.version_issues();
        let found = after
            .iter()
            .find(|i| matches!(i, V::NotIssued { .. }))
            .expect("topshirilmagan versiya ko'rsatilmadi");
        assert!(found.risky());
    }

    /// TZ VIII.24: solishtirish kartochkadagi ma'lumotga tayanadi —
    /// varaq soni, belgi va loyihachi izohi.
    #[test]
    fn version_diff_uses_the_card_data() {
        use crate::checks::version_diff;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let docs = t.db.documents(pid);
        let tasks = t.db.tasks(pid).unwrap_or_default();

        let new = docs.iter().find(|d| d.version == 2).expect("versiya");
        let old = docs
            .iter()
            .find(|d| Some(d.id) == new.replaces)
            .expect("eski versiya");

        let d = version_diff(old, new, &tasks);
        assert_eq!(d.from_label, old.label());
        assert_eq!(d.to_label, new.label());
        assert_eq!(d.sheets_from, old.sheets);
        assert_eq!(d.sheets_to, new.sheets);
        assert!(d.sheets_to > d.sheets_from, "namunada varaq qo'shilgan");
        assert!(!d.note.trim().is_empty(), "o'zgartirish izohi yo'q");
        assert_eq!(d.issued, new.issued);

        // Oldin tugatilgan ishlar aynan shu bo'limdan olinadi.
        for id in &d.tasks_before {
            let task = tasks.iter().find(|x| x.id == *id).expect("ish");
            assert_eq!(task.section, new.section);
            assert!(task.fact_end.is_some_and(|e| e < new.issued.unwrap()));
        }
    }

    /// TZ XIV.37: haftalik hisobotdagi ball sifat modulidagi bilan
    /// bir xil bo'ladi va sonlar hafta oynasidan chiqadi.
    #[test]
    fn quality_week_matches_the_module() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let w = app.quality_week();
        let module = crate::checks::quality_score(&app.quality, app.today);
        assert!((w.score - module.score).abs() < 0.001, "ball mos kelmadi");
        assert_eq!((w.to - w.from).num_days(), 7);
        assert_eq!(w.to, app.today);

        // Hafta ichidagi tekshiruvlar aynan shu oynadan.
        let in_week = app
            .quality
            .iter()
            .filter(|q| q.date > w.from && q.date <= w.to)
            .count();
        assert_eq!(w.checks, in_week);
        assert!(w.passed + w.failed <= w.checks);

        // Ochiq nuqsonlar butun loyiha bo'yicha sanaladi.
        let open = app
            .quality
            .iter()
            .filter(|q| !q.defect.trim().is_empty() && q.fixed_at.is_none())
            .count();
        assert_eq!(w.defects_open, open);
        assert!(w.overdue <= w.defects_open);
        // Eng ko'p takrorlanganlar uchtadan oshmaydi.
        assert!(w.top_defects.len() <= 3);
        for pair in w.top_defects.windows(2) {
            assert!(pair[0].1 >= pair[1].1);
        }
    }

    /// TZ V.24: kunlik xulosaning har qatori bugungi yozuvlardan sanaladi
    /// va to'siqlar soni prorab ekranidagi bilan bir xil.
    #[test]
    fn day_report_counts_todays_records() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let d = app.day_report();
        let today = app.today;
        assert_eq!(d.day, today);
        assert_eq!(d.running, app.running_today().len());
        assert_eq!(d.blockers, app.day_close().len());
        assert_eq!(
            d.workers,
            app.timesheet
                .iter()
                .filter(|e| e.date == today && e.hours > 0.0)
                .count()
        );
        assert_eq!(
            d.machines,
            app.machine_logs
                .iter()
                .filter(|l| l.date == today && l.hours > 0.0)
                .count()
        );
        assert_eq!(
            d.safety_new,
            app.safety.iter().filter(|s| s.date == today).count()
        );
        assert!(d.material_cost >= 0.0);
        assert!(d.hours >= 0.0);
    }

    /// TZ V.17: taklif faqat ombordagi qoldiq yetmaganda beriladi va
    /// so'raladigan miqdor aynan yetishmaydigan qism bo'ladi.
    #[test]
    fn journal_request_only_when_stock_is_short() {
        use crate::checks::{journal_requests, StockLine};
        use crate::domain::{JournalEntry, MaterialNorm};

        let day = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let task = test_task(10, 100.0, 40.0); // qolgan hajm: 60
        let entry = JournalEntry {
            id: 1,
            project_id: 1,
            date: day,
            author: String::new(),
            weather: String::new(),
            temperature: 0.0,
            workers: 5,
            machines: 0,
            task_id: Some(10),
            volume: 8.0,
            unit: "m3".into(),
            text: String::new(),
            remarks: String::new(),
            photos: String::new(),
            gps: String::new(),
        };
        let norm = MaterialNorm {
            id: 1,
            project_id: 1,
            task_id: 10,
            material_id: 3,
            per_unit: 0.5,
            tolerance: 0.0,
            note: String::new(),
        };
        let line = |available: f64| StockLine {
            material_id: 3,
            available,
            ..Default::default()
        };

        // Qolgan 60 × 0.5 = 30 kerak. Omborda 10 — 20 yetishmaydi.
        let out = journal_requests(
            std::slice::from_ref(&entry),
            std::slice::from_ref(&norm),
            std::slice::from_ref(&task),
            &[line(10.0)],
            day,
        );
        assert_eq!(out.len(), 1);
        assert!((out[0].need - 30.0).abs() < 1e-9);
        assert!((out[0].qty - 20.0).abs() < 1e-9);
        assert_eq!(out[0].task_id, 10);

        // Qoldiq yetarli — taklif berilmaydi.
        let enough = journal_requests(
            std::slice::from_ref(&entry),
            std::slice::from_ref(&norm),
            std::slice::from_ref(&task),
            &[line(40.0)],
            day,
        );
        assert!(enough.is_empty());

        // Tugallangan ishga material so'ralmaydi.
        let done = test_task(10, 100.0, 100.0);
        assert!(journal_requests(
            std::slice::from_ref(&entry),
            std::slice::from_ref(&norm),
            std::slice::from_ref(&done),
            &[line(0.0)],
            day,
        )
        .is_empty());

        // Boshqa kunning yozuvi hisobga olinmaydi.
        let mut other = entry.clone();
        other.date = day - chrono::Duration::days(1);
        assert!(journal_requests(
            std::slice::from_ref(&other),
            std::slice::from_ref(&norm),
            std::slice::from_ref(&task),
            &[line(0.0)],
            day,
        )
        .is_empty());
    }

    /// TZ XIII.12: dam olish kunidagi ish taqiq emas, lekin ko'rinadi;
    /// ish kunida hech kim yo'qligi ham ko'rinadi.
    #[test]
    fn schedule_sees_rest_days_and_empty_days() {
        use crate::checks::{schedule_issues, ScheduleIssue as S, WorkSchedule};
        use crate::domain::{DayKind, Shift, TimesheetEntry};

        // 2026-06-15 — dushanba, 2026-06-21 — yakshanba.
        let monday = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let sunday = chrono::NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let sch = WorkSchedule::default();
        assert!(sch.is_work_day(monday));
        assert!(!sch.is_work_day(sunday));
        // Olti kunlik hafta: dushanbadan yakshanbagacha 6 ish kuni.
        assert_eq!(sch.work_days_in(monday, sunday), 6);

        let entry = |date: chrono::NaiveDate, hours: f64| TimesheetEntry {
            id: 1,
            project_id: 1,
            worker_id: 1,
            date,
            hours,
            task_id: None,
            kind: DayKind::Work,
            shift: Shift::Day,
            note: String::new(),
        };

        // Yakshanbadagi ish — ko'rinadi.
        let out = schedule_issues(&[entry(sunday, 8.0)], &sch, sunday, sunday);
        assert!(out
            .iter()
            .any(|i| matches!(i, S::WorkOnRestDay { workers: 1, .. })));

        // Dushanba bo'sh — ko'rinadi.
        let out = schedule_issues(&[], &sch, monday, monday);
        assert!(out.iter().any(|i| matches!(i, S::EmptyWorkDay { .. })));

        // Uzun smena — ko'rinadi.
        let out = schedule_issues(&[entry(monday, 12.0)], &sch, monday, monday);
        assert!(out.iter().any(|i| matches!(i, S::OverShift { .. })));

        // Odatiy kun — e'tiroz yo'q.
        let out = schedule_issues(&[entry(monday, 8.0)], &sch, monday, monday);
        assert!(out.is_empty());
    }

    /// TZ XIII.3: obyektlar kesimidagi son har obyektning o'z
    /// yozuvlaridan chiqadi.
    #[test]
    fn object_staff_counts_each_object_separately() {
        let t = TempDb::new();
        t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let from = today - chrono::Duration::days(30);

        let rows = crate::portfolio::object_staff(&t.db, from, today);
        assert!(!rows.is_empty(), "obyektlar topilmadi");

        for o in &rows {
            let workers = t.db.workers(o.project_id);
            assert_eq!(o.workers, workers.iter().filter(|w| w.active).count());
            let hours: f64 =
                t.db.timesheet(o.project_id)
                    .iter()
                    .filter(|e| {
                        e.date >= from && e.date <= today && e.kind == crate::domain::DayKind::Work
                    })
                    .map(|e| e.hours)
                    .sum();
            assert!((o.hours - hours).abs() < 0.001);
            if o.workers > 0 {
                assert!((o.hours_per_worker - o.hours / o.workers as f64).abs() < 0.001);
            }
        }

        // Eng ko'p odam turgan obyekt oldinda.
        for w in rows.windows(2) {
            assert!(w[0].workers >= w[1].workers);
        }
    }

    /// TZ XIII.28: ko'chirishda tabel yozuvlari o'z joyida qoladi.
    #[test]
    fn moving_a_worker_keeps_the_timesheet() {
        let t = TempDb::new();
        let first = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        let second = app
            .projects
            .iter()
            .find(|p| p.id != first)
            .map(|p| p.id)
            .expect("ikkinchi obyekt");
        app.select_project(first);

        let worker = app
            .workers
            .iter()
            .find(|w| w.active)
            .cloned()
            .expect("ishchi");
        let before = app
            .timesheet
            .iter()
            .filter(|e| e.worker_id == worker.id)
            .count();

        assert!(app.move_worker(worker.id, second));
        // Xodim endi birinchi obyektda yo'q.
        assert!(!app.workers.iter().any(|w| w.id == worker.id));
        // Tabel yozuvlari o'z joyida qoldi.
        let after = app
            .timesheet
            .iter()
            .filter(|e| e.worker_id == worker.id)
            .count();
        assert_eq!(before, after, "tabel yozuvlari ko'chib ketdi");

        // Ikkinchi obyektda paydo bo'ldi va brigadasiz.
        app.select_project(second);
        let moved = app
            .workers
            .iter()
            .find(|w| w.id == worker.id)
            .expect("ko'chirilgan xodim");
        assert_eq!(moved.project_id, second);
        assert!(moved.brigade_id.is_none());

        // O'sha obyektga qayta ko'chirish ma'nosiz.
        assert!(!app.move_worker(worker.id, second));
    }

    /// TZ IV.7: o'lchovsiz sxema imzoga tayyor emas; dopusk ichidagi
    /// o'lchov esa tayyorlikni beradi.
    #[test]
    fn scheme_needs_measured_points() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.scheme_status();
        for s in &rows {
            assert!(s.out_of_tolerance <= s.points);
            // O'lchovsiz sxema hech qachon tayyor emas.
            if s.points == 0 {
                assert!(!s.ready());
                assert_eq!(s.out_of_tolerance, 0);
            }
            // Tayyor sxemada dopuskdan chiqqan nuqta bo'lmaydi.
            if s.ready() {
                assert!(s.points > 0 && s.out_of_tolerance == 0);
            }
        }

        // Tayyor bo'lmaganlari oldinda.
        let mut seen_ready = false;
        for s in &rows {
            if s.ready() {
                seen_ready = true;
            } else {
                assert!(!seen_ready, "tayyor bo'lmagan sxema pastga tushgan");
            }
        }
    }

    /// TZ IV.24: kabinet yangi ma'lumot yaratmaydi — har qator boshqa
    /// moduldagi yozuvdan keladi.
    #[test]
    fn author_supervision_only_collects() {
        use crate::checks::AuthorTask as A;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.author_supervision();
        for a in &rows {
            match a {
                A::Issue { code, .. } => {
                    assert!(app.issues.iter().any(|i| &i.code == code));
                }
                A::Change { number, days } => {
                    let c = app
                        .contract_changes
                        .iter()
                        .find(|c| &c.number == number)
                        .expect("o'zgarish modulda yo'q");
                    assert!(matches!(
                        c.status,
                        crate::domain::ChangeStatus::Draft | crate::domain::ChangeStatus::Sent
                    ));
                    assert_eq!(*days, (app.today - c.date).num_days().max(0));
                }
                A::Version { name, .. } => {
                    assert!(app
                        .documents
                        .iter()
                        .any(|d| &d.name == name && d.version > 1 && d.issued.is_none()));
                }
                A::Inspection { number, .. } => {
                    assert!(app.inspections.iter().any(|i| &i.number == number
                        && i.result == crate::domain::InspectionResult::Fail));
                }
            }
        }

        // Muddati o'tganlari oldinda.
        let mut seen_ok = false;
        for a in &rows {
            if a.late() {
                assert!(!seen_ok, "kechikkan ish pastga tushib qolgan");
            } else {
                seen_ok = true;
            }
        }
    }

    /// TZ II.12, 14: spetsifikatsiya va qurilish imkoniyati qoidalari
    /// o'z bo'limida qoladi va tuzatish yo'lini ko'rsatadi.
    #[test]
    fn spec_and_constructability_rules_work() {
        use crate::checks::{check_project, Ctx};
        use crate::domain::{ElementKind, Relation, Severity};
        use crate::model::Section;
        use std::collections::HashMap;

        let norms = HashMap::new();
        let run = |elements: &[crate::domain::Element], links: &[crate::domain::ElementLink]| {
            check_project(&Ctx {
                project_id: 1,
                tasks: &[],
                elements,
                links,
                items: &[],
                declared_total: 0.0,
                norms: &norms,
                prices: &[],
            })
        };

        // Miqdorsiz eshik — spetsifikatsiya e'tirozi.
        let door = test_element(1, Section::Ar, ElementKind::Door, "D-1", 0.0);
        let out = run(std::slice::from_ref(&door), &[]);
        assert!(out
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_spec_qty_title")));

        // Miqdor bor, birlik bor — e'tiroz yo'q.
        let mut sized = door.clone();
        sized.size = 12.0;
        sized.unit = "dona".into();
        assert!(!run(std::slice::from_ref(&sized), &[])
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_spec_qty_title")));

        // Birliksiz son — alohida e'tiroz.
        let mut no_unit = sized.clone();
        no_unit.unit = String::new();
        assert!(run(std::slice::from_ref(&no_unit), &[])
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_spec_unit_title")));

        // Teshik rigeldan katta — qurib bo'lmaydi.
        let beam = test_element(2, Section::Kj, ElementKind::Beam, "R-1", 600.0);
        let hole = test_element(3, Section::Kj, ElementKind::Opening, "O-1", 700.0);
        let out = run(
            &[beam.clone(), hole.clone()],
            &[test_link(2, 3, Relation::Contains)],
        );
        let found = out
            .iter()
            .find(|i| i.title == crate::i18n::t("chk_build_size_title"))
            .expect("teshik o'lchami e'tirozi yo'q");
        assert_eq!(found.severity, Severity::Critical);
        assert!(!found.recommendation.is_empty());

        // Teshik kichik — e'tiroz yo'q.
        let mut small = hole.clone();
        small.size = 200.0;
        assert!(!run(&[beam, small], &[test_link(2, 3, Relation::Contains)])
            .iter()
            .any(|i| i.title == crate::i18n::t("chk_build_size_title")));
    }

    /// Modul yordamchisi: har modul ekrani o'z mavzusiga tushadi va
    /// mavzu javobi bo'sh bo'lmaydi.
    #[test]
    fn module_assistant_maps_every_screen() {
        use crate::app::Screen;
        use crate::copilot::Intent;

        // Modul ekranlari mavzuga bog'langan.
        let pairs = [
            (Screen::Gantt, Intent::Delays),
            (Screen::Estimate, Intent::Money),
            (Screen::ExecDocs, Intent::Docs),
            (Screen::Requests, Intent::Supply),
            (Screen::Warehouse, Intent::Stock),
            (Screen::Materials, Intent::Stock),
            (Screen::Timesheet, Intent::Crew),
            (Screen::Machines, Intent::Machines),
            (Screen::Quality, Intent::Quality),
            (Screen::Safety, Intent::Safety),
            (Screen::Director, Intent::Overview),
        ];
        for (screen, want) in pairs {
            assert_eq!(Intent::for_screen(screen), Some(want), "{screen:?}");
        }

        // Sozlamalar moduli emas — yordamchi tugmasi chiqmaydi.
        assert_eq!(Intent::for_screen(Screen::Settings), None);

        // Har mavzuning savoli va javobi bor.
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        let supply = app.supply();
        let stock = app.stock();
        let cost = app.cost_summary();
        let sales = app.sales();
        let inp = app.analytics_input(&supply, &stock, &cost, &sales);
        for i in Intent::ALL {
            assert!(!i.question().is_empty());
            let a = crate::copilot::answer(*i, &inp);
            assert!(!a.lines.is_empty(), "bo'sh javob: {i:?}");
        }
    }

    /// TZ IX.34, X.45, XI.40 va h.k.: rahbar ekranidagi sonlar o'z
    /// modulidagi funksiyalardan olinadi — qayta hisoblanmaydi.
    #[test]
    fn director_numbers_come_from_the_modules() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Ekran chizmasdan, u ishlatadigan funksiyalar bir xil natija
        // berishini tekshiramiz: rahbar ekranidagi son modul ekranidagi
        // bilan farq qilmasligi kerak.
        let a = app.executive_score();
        let b = app.executive_score();
        assert!((a.total - b.total).abs() < 0.001);

        let week = app.quality_week();
        let module = crate::checks::quality_score(&app.quality, app.today);
        assert!((week.score - module.score).abs() < 0.001);

        let staff = app.staff_forecast();
        assert_eq!(staff.have, app.workers.iter().filter(|w| w.active).count());

        let kits = app.material_kits();
        let ready = app.readiness();
        for k in &kits {
            assert_eq!(
                k.missing,
                ready.iter().filter(|r| r.task_id == k.task_id).count()
            );
        }

        // Rahbar ekrani TZ moduli emas — raqami bo'sh.
        assert_eq!(crate::app::Screen::Director.numeral(), "");
    }

    /// TZ II.6-8: tezlik va kesim hisoblari formulaga mos keladi va
    /// ma'lumot yetishmasa tekshiruv o'tkazilmaydi.
    #[test]
    fn engineering_rules_use_real_formulas() {
        use crate::checks::{check_project, Ctx};
        use crate::domain::{ElementKind, Relation};
        use crate::model::Section;
        use std::collections::HashMap;

        let norms = HashMap::new();
        let run = |elements: &[crate::domain::Element], links: &[crate::domain::ElementLink]| {
            check_project(&Ctx {
                project_id: 1,
                tasks: &[],
                elements,
                links,
                items: &[],
                declared_total: 0.0,
                norms: &norms,
                prices: &[],
            })
        };
        let has = |out: &[crate::domain::Issue], key: &str| {
            out.iter().any(|i| i.title == crate::i18n::t(key))
        };

        // --- VK: d = 50 mm, Q = 10 l/s -> v ≈ 5.1 m/s > 3 ---
        let mut pipe = test_element(1, Section::Vk, ElementKind::Pipe, "T-1", 50.0);
        pipe.value = 10.0;
        pipe.value_name = "sarf".into();
        assert!(has(
            &run(std::slice::from_ref(&pipe), &[]),
            "chk_vk_velocity_title"
        ));

        // Sarf kam bo'lsa — tezlik chegarada, e'tiroz yo'q.
        let mut slow = pipe.clone();
        slow.value = 3.0; // v ≈ 1.5 m/s
        assert!(!has(
            &run(std::slice::from_ref(&slow), &[]),
            "chk_vk_velocity_title"
        ));

        // Sarf ko'rsatilmagan — tekshiruv o'tkazilmaydi.
        let mut unknown = pipe.clone();
        unknown.value = 0.0;
        assert!(!has(
            &run(std::slice::from_ref(&unknown), &[]),
            "chk_vk_velocity_title"
        ));

        // --- OV: d = 200 mm, L = 3000 m3/h -> v ≈ 26.5 m/s > 6 ---
        let mut duct = test_element(2, Section::Ov, ElementKind::Duct, "V-1", 200.0);
        duct.value = 3000.0;
        duct.value_name = "havo sarfi".into();
        assert!(has(
            &run(std::slice::from_ref(&duct), &[]),
            "chk_ov_velocity_title"
        ));

        // --- EOM: P = 30 kW -> I ≈ 53.6 A -> S ≈ 10.7 mm2; kabel 6 mm2 ---
        let mut device = test_element(3, Section::Eom, ElementKind::Device, "Q-1", 0.0);
        device.value = 30.0;
        device.value_name = "quvvat".into();
        let cable = test_element(4, Section::Eom, ElementKind::Cable, "K-1", 6.0);
        let out = run(
            &[device.clone(), cable.clone()],
            &[test_link(4, 3, Relation::Serves)],
        );
        assert!(has(&out, "chk_eom_title"));

        // Kesim yetarli bo'lsa — e'tiroz yo'q.
        let mut thick = cable.clone();
        thick.size = 25.0;
        assert!(!has(
            &run(
                &[device.clone(), thick],
                &[test_link(4, 3, Relation::Serves)]
            ),
            "chk_eom_title"
        ));

        // Kabel bog'lanmagan bo'lsa — hisob qilinmaydi.
        assert!(!has(
            &run(std::slice::from_ref(&device), &[]),
            "chk_eom_title"
        ));
    }

    /// TZ II.3-5: eshik, yoritish, beton sinfi va po'lat markasi.
    #[test]
    fn section_rules_ask_for_the_missing_data() {
        use crate::checks::{check_project, Ctx};
        use crate::domain::{ElementKind, Relation};
        use crate::model::Section;
        use std::collections::HashMap;

        let norms = HashMap::new();
        let run = |elements: &[crate::domain::Element], links: &[crate::domain::ElementLink]| {
            check_project(&Ctx {
                project_id: 1,
                tasks: &[],
                elements,
                links,
                items: &[],
                declared_total: 0.0,
                norms: &norms,
                prices: &[],
            })
        };
        let has = |out: &[crate::domain::Issue], key: &str| {
            out.iter().any(|i| i.title == crate::i18n::t(key))
        };

        // Tor eshik.
        let narrow = test_element(1, Section::Ar, ElementKind::Door, "D-1", 600.0);
        assert!(has(
            &run(std::slice::from_ref(&narrow), &[]),
            "chk_ar_door_title"
        ));
        // Kengi — e'tiroz yo'q. Metrda berilgani ham tushuniladi.
        let mut wide = narrow.clone();
        wide.size = 0.9;
        assert!(!has(
            &run(std::slice::from_ref(&wide), &[]),
            "chk_ar_door_title"
        ));

        // Yoritish: 20 m2 xona, 1 m2 deraza -> 1/20 < 1/8.
        let mut room = test_element(2, Section::Ar, ElementKind::Room, "X-1", 20.0);
        room.unit = "m2".into();
        let mut window = test_element(3, Section::Ar, ElementKind::Window, "OK-1", 1.0);
        window.unit = "m2".into();
        let out = run(
            &[room.clone(), window.clone()],
            &[test_link(2, 3, Relation::Contains)],
        );
        assert!(has(&out, "chk_ar_light_title"));

        // Deraza kattaroq bo'lsa — talab bajariladi.
        let mut big = window.clone();
        big.size = 4.0;
        assert!(!has(
            &run(&[room, big], &[test_link(2, 3, Relation::Contains)]),
            "chk_ar_light_title"
        ));

        // Beton sinfisiz ustun.
        let column = test_element(4, Section::Kj, ElementKind::Column, "K-1", 400.0);
        assert!(has(
            &run(std::slice::from_ref(&column), &[]),
            "chk_kj_class_title"
        ));
        // Sinfi ko'rsatilgan — e'tiroz yo'q.
        let mut classed = column.clone();
        classed.mark = "K-1 B25".into();
        assert!(!has(
            &run(std::slice::from_ref(&classed), &[]),
            "chk_kj_class_title"
        ));
        // Kesimi juda kichik.
        let mut thin = classed.clone();
        thin.size = 120.0;
        assert!(has(
            &run(std::slice::from_ref(&thin), &[]),
            "chk_kj_section_title"
        ));

        // Po'lat markasisiz metall rigel.
        let beam = test_element(5, Section::Km, ElementKind::Beam, "B-1", 300.0);
        assert!(has(
            &run(std::slice::from_ref(&beam), &[]),
            "chk_km_steel_title"
        ));
        let mut graded = beam.clone();
        graded.note = "S245 po'lat".into();
        assert!(!has(
            &run(std::slice::from_ref(&graded), &[]),
            "chk_km_steel_title"
        ));
    }

    /// Namunadagi loyiha tekshiruvi haddan tashqari ko'p e'tiroz
    /// chiqarmasligi kerak: 30 tadan oshsa ekran o'qilmay qoladi.
    #[test]
    fn demo_project_check_stays_readable() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);
        app.run_project_check();
        let found: Vec<_> = app
            .issues
            .iter()
            .filter(|i| i.module == crate::domain::IssueModule::Project && i.auto)
            .collect();
        assert!(
            found.len() <= 30,
            "namunada juda ko'p e'tiroz: {}",
            found.len()
        );
        assert!(!found.is_empty(), "namunada e'tiroz umuman yo'q");
    }

    /// TZ XVII.7: sabab faqat yozuvdan chiqadi va bir ishga bitta sabab
    /// yoziladi — beshta sabab javob emas.
    #[test]
    fn delay_cause_comes_from_records() {
        use crate::checks::DelayCause as C;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let delays = app.delay_causes();
        assert_eq!(delays.len(), app.progress.overdue.len());

        for d in &delays {
            assert!(app.progress.overdue.contains(&d.task_id));
            assert!(d.days >= 0);
            assert!((d.cost() - d.daily_cost * d.days as f64).abs() < 0.01);

            match &d.cause {
                C::MaterialShort { short, .. } => {
                    assert!(*short > 0.0);
                    assert!(app.readiness().iter().any(|r| r.task_id == d.task_id));
                }
                C::QualityBlock { defects } => {
                    assert!(*defects > 0);
                    assert!(app
                        .task_blocks()
                        .iter()
                        .any(|b| b.task_id == d.task_id && b.open_defects == *defects));
                }
                C::WaitingDocs { missing } => assert!(*missing > 0),
                C::NoCrew => {
                    assert!(!app
                        .timesheet
                        .iter()
                        .any(|e| e.task_id == Some(d.task_id) && e.hours > 0.0));
                }
                C::PredecessorLate { days, .. } => assert!(*days >= 0),
                C::MachineDown { machine } => assert!(!machine.is_empty()),
                // Noma'lum sabab ham halol javob: o'ylab topilmaydi.
                C::Unknown => assert!(!d.cause.actionable()),
            }
        }

        // Eng qimmat kechikish oldinda.
        for w in delays.windows(2) {
            assert!(w[0].cost() >= w[1].cost() - 0.01);
        }
    }

    /// TZ XVII.10: prognoz og'irligi qoidaga mos va og'irlari oldinda.
    #[test]
    fn risk_forecast_is_weighted() {
        use crate::checks::{risk_forecast, QualityWeek, RiskKind as K};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let quiet = QualityWeek {
            from: today - chrono::Duration::days(7),
            to: today,
            ..Default::default()
        };

        // Hech narsa yo'q — prognoz bo'sh.
        assert!(risk_forecast(0, &[], &[], &[], &quiet, 0, today).is_empty());

        // 20 kun kechikish — 60 ball, ya'ni yuqori daraja.
        let out = risk_forecast(20, &[], &[], &[], &quiet, 0, today);
        let slip = out
            .iter()
            .find(|r| matches!(r.kind, K::ScheduleSlip { .. }))
            .expect("muddat riski yo'q");
        assert!((slip.weight - 60.0).abs() < 0.001);
        assert!(slip.high());

        // Nuqsonlar: muddati o'tgani og'irroq.
        let bad = QualityWeek {
            defects_open: 4,
            overdue: 3,
            ..quiet.clone()
        };
        let out = risk_forecast(0, &[], &[], &[], &bad, 0, today);
        let q = out
            .iter()
            .find(|r| matches!(r.kind, K::QualityDrop { .. }))
            .expect("sifat riski yo'q");
        // 3 × 20 + 4 × 5 = 80.
        assert!((q.weight - 80.0).abs() < 0.001);

        // Og'irlari oldinda.
        let mixed = risk_forecast(5, &[], &[], &[], &bad, 2, today);
        for w in mixed.windows(2) {
            assert!(w[0].weight >= w[1].weight);
        }
        // Har risk manzilga ega.
        for r in &mixed {
            let _ = r.screen();
        }
    }

    /// Prognozda muddat ko'rsatilmagan material qatori bo'lmaydi:
    /// «qachon» degan savolga javob bo'lmasa, bu prognoz emas.
    #[test]
    fn stock_risk_needs_a_date() {
        use crate::checks::{risk_forecast, PurchasePlanLine, QualityWeek, RiskKind as K};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let quiet = QualityWeek::default();
        let line = |need_by: Option<chrono::NaiveDate>| PurchasePlanLine {
            material_id: 1,
            available: 0.0,
            ordered: 0.0,
            min_stock: 0.0,
            needed_for_tasks: 10.0,
            to_buy: 10.0,
            cost: 100.0,
            need_by,
            has_request: false,
        };

        // Muddatsiz qator prognozga tushmaydi.
        let out = risk_forecast(0, &[line(None)], &[], &[], &quiet, 0, today);
        assert!(!out.iter().any(|r| matches!(r.kind, K::StockOut { .. })));

        // Muddat bor — tushadi va yaqinroq bo'lsa og'irroq.
        let soon = risk_forecast(0, &[line(Some(today))], &[], &[], &quiet, 0, today);
        let near = soon
            .iter()
            .find(|r| matches!(r.kind, K::StockOut { .. }))
            .expect("material riski yo'q");
        assert!((near.weight - 100.0).abs() < 0.001);
        assert_eq!(near.when, Some(today));

        let far = risk_forecast(
            0,
            &[line(Some(today + chrono::Duration::days(30)))],
            &[],
            &[],
            &quiet,
            0,
            today,
        );
        let late = far
            .iter()
            .find(|r| matches!(r.kind, K::StockOut { .. }))
            .expect("material riski yo'q");
        assert!(late.weight < near.weight);
    }

    /// TZ X.47: zanjirning har bosqichida qiymat kamaymasligi kerak;
    /// keyingi bosqich oldingisidan katta bo'lsa — uzilish.
    #[test]
    fn supply_chain_finds_the_gaps() {
        use crate::checks::{ChainGap as G, SupplyChain};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let line = |requested: f64,
                    quotes: usize,
                    ordered: f64,
                    delivered: f64,
                    checks: usize,
                    stocked: f64,
                    issued: f64,
                    paid: f64| SupplyChain {
            request_id: None,
            material_id: None,
            title: "Sement".into(),
            unit: "t".into(),
            requested,
            quotes,
            ordered,
            delivered,
            checks,
            stocked,
            issued,
            amount: 1000.0,
            paid,
        };

        // Butun zanjir: uzilish yo'q.
        let clean = line(10.0, 2, 10.0, 10.0, 1, 10.0, 8.0, 1000.0);
        assert!(clean.gaps(today, Some(today)).is_empty());

        // Buyurtma arizadan ko'p.
        let over = line(5.0, 2, 10.0, 10.0, 1, 10.0, 0.0, 1000.0);
        assert!(over
            .gaps(today, None)
            .iter()
            .any(|g| matches!(g, G::OrderOverRequest { .. })));

        // Yetkazilgan buyurtmadan ko'p — jiddiy.
        let much = line(10.0, 2, 10.0, 12.0, 1, 12.0, 0.0, 1000.0);
        let gaps = much.gaps(today, None);
        let found = gaps
            .iter()
            .find(|g| matches!(g, G::DeliveryOverOrder { .. }))
            .expect("yetkazish uzilishi yo'q");
        assert!(found.severe());
        // Jiddiylari oldinda.
        assert!(gaps[0].severe());

        // Omborda yo'q material berilgan.
        let ghost = line(10.0, 2, 10.0, 10.0, 1, 4.0, 9.0, 1000.0);
        assert!(ghost
            .gaps(today, None)
            .iter()
            .any(|g| matches!(g, G::IssuedOverStock { .. })));

        // Kirish nazoratisiz yetkazish va taklifsiz xarid.
        let raw = line(10.0, 0, 10.0, 10.0, 0, 10.0, 0.0, 1000.0);
        let gaps = raw.gaps(today, None);
        assert!(gaps.contains(&G::NoInputCheck));
        assert!(gaps.contains(&G::NoQuotes));

        // To'lov: ortiqcha va muddati o'tgan.
        let money = line(10.0, 2, 10.0, 10.0, 1, 10.0, 0.0, 1500.0);
        assert!(money
            .gaps(today, None)
            .iter()
            .any(|g| matches!(g, G::Overpaid { .. })));
        let debt = line(10.0, 2, 10.0, 10.0, 1, 10.0, 0.0, 400.0);
        let late = debt.gaps(today, Some(today - chrono::Duration::days(1)));
        assert!(late
            .iter()
            .any(|g| matches!(g, G::PaymentOverdue { unpaid } if (*unpaid - 600.0).abs() < 0.01)));
        // Muddat kelmagan bo'lsa — e'tiroz yo'q.
        assert!(!debt
            .gaps(today, Some(today + chrono::Duration::days(5)))
            .iter()
            .any(|g| matches!(g, G::PaymentOverdue { .. })));
    }

    /// TZ X.23: to'lov holati namunada har xil bo'ladi — intizom ekrani
    /// bo'sh ko'rinmasin.
    #[test]
    fn demo_has_mixed_payment_state() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        assert!(
            app.purchases.iter().any(|p| p.unpaid() > 0.0),
            "to'lanmagan xarid yo'q"
        );
        assert!(
            app.purchases.iter().any(|p| p.paid > 0.0),
            "to'langan xarid yo'q"
        );

        let (rows, sum) = app.supply_chain();
        assert_eq!(sum.lines, rows.len());
        assert!(sum.amount > 0.0);
        assert!((sum.paid - rows.iter().map(|(l, _)| l.paid).sum::<f64>()).abs() < 0.01);
        // Uzilishi ko'p qatorlar oldinda.
        for w in rows.windows(2) {
            assert!(w[0].1.len() >= w[1].1.len());
        }
        // Qoralama xaridlar zanjirga tushmaydi.
        assert!(rows.len() <= app.purchases.len());
    }

    /// TZ XVI.26: prognoz ikkitadan kam ta'mirga berilmaydi va o'rtacha
    /// oraliq tarixdan hisoblanadi.
    #[test]
    fn repair_forecast_needs_history() {
        use crate::checks::repair_forecast;
        use crate::domain::{MachineLog, MachineRepair, RepairKind};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let m = t.db.machines(pid).into_iter().next().expect("texnika");

        let repair = |days_ago: i64| MachineRepair {
            id: 0,
            project_id: pid,
            machine_id: m.id,
            kind: RepairKind::Fault,
            started: today - chrono::Duration::days(days_ago),
            finished: Some(today - chrono::Duration::days(days_ago - 1)),
            reason: String::new(),
            cost: 100.0,
            hours_at: 0.0,
            note: String::new(),
        };
        // Har kuni 10 soat ishlagan.
        let logs: Vec<MachineLog> = (0..60)
            .map(|i| MachineLog {
                id: i + 1,
                project_id: pid,
                machine_id: m.id,
                date: today - chrono::Duration::days(i),
                hours: 10.0,
                fuel: 0.0,
                task_id: None,
                number: String::new(),
                driver: String::new(),
                route: String::new(),
                odo_start: 0.0,
                odo_end: 0.0,
                trips: 0,
                cargo: 0.0,
                note: String::new(),
                gps: String::new(),
            })
            .collect();

        // Bitta ta'mir — prognoz yo'q.
        let one = repair_forecast(std::slice::from_ref(&m), &logs, &[repair(40)], today, 30);
        assert!(one.is_empty(), "bitta hodisadan prognoz chiqdi");

        // Ikkita ta'mir: oraliq 20 kun × 10 soat = 200 soat.
        let two = repair_forecast(
            std::slice::from_ref(&m),
            &logs,
            &[repair(40), repair(20)],
            today,
            30,
        );
        let f = two.first().expect("prognoz yo'q");
        assert_eq!(f.based_on, 2);
        assert!(
            (f.mtbf_hours - 200.0).abs() < 0.001,
            "mtbf: {}",
            f.mtbf_hours
        );
        // Oxirgi ta'mirdan beri 20 kun × 10 = 200 soat -> qolgan 0.
        assert!((f.since_last - 200.0).abs() < 0.001);
        assert!(f.hours_left.abs() < 0.001);
        assert!(f.soon());
        assert!(f.when.is_some());
    }

    /// TZ XVI.42: samaradorlik balliga narx kirmaydi va norma yo'q
    /// bo'lsa yoqilg'i qismi ballni tushirmaydi.
    #[test]
    fn machine_score_ignores_price() {
        use crate::checks::MachineChain;

        let base = MachineChain {
            machine_id: 1,
            name: "Kran".into(),
            rented: false,
            requests: 0,
            hours: 240.0, // 30 kun × 8 soat = to'liq foydalanish
            idle_days: 0,
            fuel_used: 0.0,
            fuel_norm: 0.0,
            repairs: 0,
            repair_cost: 0.0,
            work_cost: 1000.0,
        };
        assert!((base.score(30) - 100.0).abs() < 0.001);
        // Norma yo'q — yoqilg'i qismi to'liq ball.
        assert!(base.fuel_deviation().is_none());

        // Narx o'n barobar oshsa ham ball o'zgarmaydi.
        let mut pricey = base.clone();
        pricey.work_cost = 10_000.0;
        assert!((pricey.score(30) - base.score(30)).abs() < 0.001);
        // Lekin soatning qiymati o'zgaradi.
        assert!(pricey.cost_per_hour().unwrap() > base.cost_per_hour().unwrap());

        // Ta'mir ballni tushiradi.
        let mut broken = base.clone();
        broken.repairs = 2;
        assert!((broken.score(30) - (100.0 + 60.0 + 100.0) / 3.0).abs() < 0.001);

        // Yoqilg'i normadan chetlanishi ham.
        let mut thirsty = base.clone();
        thirsty.fuel_norm = 100.0;
        thirsty.fuel_used = 130.0;
        assert!((thirsty.fuel_deviation().unwrap() - 30.0).abs() < 0.001);
        assert!(thirsty.score(30) < base.score(30));
    }

    /// TZ XIV.15: bo'lim balli umumiy ball bilan bir xil qoidada.
    #[test]
    fn section_quality_uses_the_same_rule() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.section_quality();
        for s in &rows {
            // Shu bo'limga tegishli tekshiruvlarni ajratamiz.
            let mine: Vec<crate::domain::QualityCheck> = app
                .quality
                .iter()
                .filter(|q| {
                    q.task_id.and_then(|id| app.task(id)).map(|x| x.section) == Some(s.section)
                })
                .cloned()
                .collect();
            assert_eq!(s.checks, mine.len());
            let module = crate::checks::quality_score(&mine, app.today);
            assert!((s.score - module.score).abs() < 0.001, "ball mos kelmadi");
            assert!(s.overdue <= s.defects_open);
        }
        // Eng past ball oldinda.
        for w in rows.windows(2) {
            assert!(w[0].score <= w[1].score);
        }
    }

    /// TZ XIV.19: ustuvorlik balli sabablardan yig'iladi va har sabab
    /// ko'rsatiladi.
    #[test]
    fn defect_priority_shows_its_reasons() {
        use crate::checks::{defect_priority, CLOSING_PROGRESS};
        use crate::domain::{QualityCheck, QualityKind, QualityResult};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let task = test_task(10, 100.0, CLOSING_PROGRESS);
        let check = |result: QualityResult, deadline: Option<chrono::NaiveDate>| QualityCheck {
            id: 1,
            project_id: 1,
            kind: QualityKind::Input,
            date: today,
            task_id: Some(10),
            material_id: None,
            subject: String::new(),
            inspector: String::new(),
            result,
            defect: "yoriq".into(),
            deadline,
            checklist_id: None,
            fixed_at: None,
            note: String::new(),
        };

        // Ochiq nuqson: 20 + yopilishga yaqin 20 + yashirin bo'lim 10 = 50.
        let plain = defect_priority(
            &[check(QualityResult::Pass, None)],
            std::slice::from_ref(&task),
            today,
        );
        assert_eq!(plain.len(), 1);
        assert!(
            (plain[0].weight - 50.0).abs() < 0.001,
            "{}",
            plain[0].weight
        );
        assert!(plain[0].reasons.contains(&"dp_open"));
        assert!(plain[0].reasons.contains(&"dp_closing"));
        assert!(!plain[0].urgent());

        // Muddati o'tgan va salbiy: 50 + 40 + 20 = 110 -> 100.
        let worst = defect_priority(
            &[check(
                QualityResult::Fail,
                Some(today - chrono::Duration::days(3)),
            )],
            std::slice::from_ref(&task),
            today,
        );
        assert!((worst[0].weight - 100.0).abs() < 0.001);
        assert!(worst[0].urgent());
        assert!(worst[0].reasons.contains(&"dp_overdue"));
        assert!(worst[0].reasons.contains(&"dp_failed"));

        // Bartaraf etilgan nuqson ro'yxatga tushmaydi.
        let mut fixed = check(QualityResult::Fail, None);
        fixed.fixed_at = Some(today);
        assert!(defect_priority(&[fixed], std::slice::from_ref(&task), today).is_empty());
    }

    /// TZ XIV.27, VII.28: oldingi ish tugamasdan boshlangan ish
    /// ko'rsatiladi va tekshirilmaganlari oldinda turadi.
    #[test]
    fn sequence_breaks_put_unchecked_first() {
        use crate::checks::sequence_breaks;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let breaks = sequence_breaks(&app.tasks, &app.links, &app.quality);
        for b in &breaks {
            let succ = app.task(b.task_id).expect("keyingi ish");
            let pred = app.task(b.pred_id).expect("oldingi ish");
            // Keyingisi boshlangan, oldingisi tugamagan.
            assert!(succ.progress > 0.0 || succ.fact_start.is_some());
            assert!(pred.progress < 99.999 && pred.fact_end.is_none());
            assert!((b.pred_progress - pred.progress).abs() < 0.001);
            assert_eq!(
                b.pred_checked,
                app.quality.iter().any(|q| q.task_id == Some(b.pred_id))
            );
        }

        // Tekshirilmaganlari oldinda.
        let mut seen_checked = false;
        for b in &breaks {
            if b.pred_checked {
                seen_checked = true;
            } else {
                assert!(!seen_checked, "tekshirilmagan buzilish pastga tushgan");
            }
        }
    }

    /// TZ XVII.42: qarorlar markazi yangi hisob qilmaydi — har qator
    /// boshqa moduldagi holatdan keladi, tartib esa kutish vaqtiga qarab.
    #[test]
    fn decision_centre_only_collects() {
        use crate::checks::Decision as D;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.decisions();
        for d in &rows {
            match d {
                D::Request { number, days } => {
                    let r = app
                        .requests
                        .iter()
                        .find(|r| &r.number == number)
                        .expect("ariza modulda yo'q");
                    assert_eq!(r.status, crate::domain::RequestStatus::New);
                    assert_eq!(*days, (app.today - r.date).num_days().max(0));
                }
                D::Change { number, .. } => {
                    assert!(app.contract_changes.iter().any(|c| &c.number == number));
                }
                D::Acceptance { number, .. } => {
                    assert!(app.acceptances.iter().any(|a| &a.number == number));
                }
                D::TechApproval { number, .. } => {
                    assert!(app
                        .purchases
                        .iter()
                        .any(|p| &p.number == number && !p.tech_ok));
                }
                D::DefectDeadline { count } => assert!(*count > 0),
                D::DocSign { count } => assert!(*count > 0),
                D::MachineStop { count } => assert!(*count > 0),
            }
            let _ = d.screen();
        }

        // Og'irlari oldinda.
        for w in rows.windows(2) {
            assert!(w[0].weight() >= w[1].weight());
        }

        // Uzoq kutgan ariza yaqinda kelganidan og'irroq.
        let fresh = D::Request {
            number: "A".into(),
            days: 1,
        };
        let old = D::Request {
            number: "B".into(),
            days: 15,
        };
        assert!(old.weight() > fresh.weight());
    }

    /// TZ XVII.27-28: solishtirish birlik hajmga keltiriladi va hajm
    /// noma'lum bo'lsa ustun bo'sh qoladi.
    #[test]
    fn benchmark_is_per_unit_volume() {
        use crate::checks::{benchmark, BenchmarkInput};

        let rows = vec![
            BenchmarkInput {
                project_id: 1,
                fact_pct: 40.0,
                plan_pct: 50.0,
                volume: 100.0,
                cost: 500.0,
                hours: 200.0,
                quality: 80.0,
                safety: 90.0,
            },
            // Hajm yuritilmagan obyekt.
            BenchmarkInput {
                project_id: 2,
                fact_pct: 60.0,
                plan_pct: 50.0,
                volume: 0.0,
                cost: 900.0,
                hours: 300.0,
                quality: 70.0,
                safety: 60.0,
            },
        ];
        let out = benchmark(&rows);
        assert_eq!(out.len(), 2);
        // Rejadan orqada qolgani birinchi.
        assert_eq!(out[0].project_id, 1);
        assert!((out[0].gap() + 10.0).abs() < 0.001);
        assert!((out[0].cost_per_volume.unwrap() - 5.0).abs() < 0.001);
        assert!((out[0].hours_per_volume.unwrap() - 2.0).abs() < 0.001);
        // Hajmsiz obyektda ustun bo'sh — nol yozilmaydi.
        assert!(out[1].cost_per_volume.is_none());
        assert!(out[1].hours_per_volume.is_none());
        assert!(out[1].gap() > 0.0);
    }

    /// TZ XIII.32: fond kelib chiqishi bo'yicha bo'linadi va bo'sh
    /// turish alohida turadi — u ish emas, lekin pul to'lanadi.
    #[test]
    fn payroll_splits_work_from_idle() {
        use crate::checks::{payroll_summary, IDLE_LIMIT_PCT};
        use crate::domain::{DayKind, Shift, TimesheetEntry, Worker};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let from = today - chrono::Duration::days(30);
        let worker = Worker {
            id: 1,
            project_id: 1,
            name: "Ali".into(),
            position: "Payvandchi".into(),
            org: String::new(),
            hourly_rate: 100.0,
            active: true,
            brigade_id: Some(7),
        };
        let entry = |kind: DayKind, shift: Shift, hours: f64| TimesheetEntry {
            id: 1,
            project_id: 1,
            worker_id: 1,
            date: today,
            hours,
            task_id: None,
            kind,
            shift,
            note: String::new(),
        };

        // 8 soat ish (800) + 2 soat bo'sh turish (200) = 1000.
        let rows = vec![
            entry(DayKind::Work, Shift::Day, 8.0),
            entry(DayKind::Downtime, Shift::Day, 2.0),
        ];
        let p = payroll_summary(&rows, std::slice::from_ref(&worker), from, today);
        assert!((p.total - 1000.0).abs() < 0.001);
        assert!((p.work - 800.0).abs() < 0.001);
        assert!((p.idle - 200.0).abs() < 0.001);
        assert!((p.idle_pct() - 20.0).abs() < 0.001);
        assert!(p.idle_pct() > IDLE_LIMIT_PCT && p.idle_high());
        assert!((p.shift_extra).abs() < 0.001);

        // Tungi smena qo'shimchasi alohida ko'rinadi: 8 × 100 × 1.5 = 1200.
        let night = payroll_summary(
            &[entry(DayKind::Work, Shift::Night, 8.0)],
            std::slice::from_ref(&worker),
            from,
            today,
        );
        assert!((night.total - 1200.0).abs() < 0.001);
        assert!((night.shift_extra - 400.0).abs() < 0.001);

        // Brigada kesimi.
        let idle =
            crate::checks::idle_by_brigade(&rows, std::slice::from_ref(&worker), from, today);
        assert_eq!(idle.len(), 1);
        assert_eq!(idle[0].brigade_id, Some(7));
        assert!((idle[0].idle_pct - 20.0).abs() < 0.001);
        assert!((idle[0].cost - 200.0).abs() < 0.001);
    }

    /// TZ XV.32: kunlik hisobot yangi hisob qilmaydi — ishchi holati va
    /// texnika ko'rigi o'z modullaridan olinadi.
    #[test]
    fn safety_day_reads_other_modules() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let d = app.safety_day();
        assert_eq!(d.day, app.today);
        assert_eq!(
            d.blocked_workers,
            app.worker_safety().iter().filter(|w| w.blocked()).count()
        );
        assert_eq!(
            d.open_total,
            app.safety
                .iter()
                .filter(|s| matches!(
                    s.status,
                    crate::domain::IssueStatus::Open | crate::domain::IssueStatus::InWork
                ))
                .count()
        );
        assert!(d.overdue <= d.open_total);
        // Ko'riksiz texnika mexanik ekranidagi bilan bir xil mantiqda.
        let unchecked = app
            .machines
            .iter()
            .filter(|m| {
                app.machine_logs
                    .iter()
                    .any(|l| l.machine_id == m.id && l.date == app.today && l.hours > 0.0)
                    && !app
                        .machine_checks
                        .iter()
                        .any(|c| c.machine_id == m.id && c.date == app.today)
            })
            .count();
        assert_eq!(d.machines_unchecked, unchecked);
        assert_eq!(
            d.clean(),
            d.opened == 0 && d.overdue == 0 && d.blocked_workers == 0 && d.machines_unchecked == 0
        );
    }

    /// TZ XII: kartochka yangi hisob qilmaydi — har bo'lim o'z
    /// funksiyasidan olinadi.
    #[test]
    fn material_card_collects_from_modules() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let stock = app.stock();
        let readiness = app.readiness();
        let consumption = app.consumption();
        let mut checked = 0;

        for m in app.materials.clone() {
            let Some(c) = app.material_card(m.id) else {
                continue;
            };
            checked += 1;
            assert_eq!(c.material_id, m.id);
            assert!((c.catalog_price - m.price).abs() < 0.001);

            // Qoldiq ombordan.
            if let Some(l) = stock.iter().find(|l| l.material_id == m.id) {
                assert!((c.available - l.available).abs() < 0.001);
                assert!((c.balance - l.balance).abs() < 0.001);
                assert_eq!(c.below_min, l.below_min);
            }

            // Yaqin ehtiyoj tayyorlikdan — bir manbadan.
            let need: f64 = readiness
                .iter()
                .filter(|r| r.material_id == m.id)
                .map(|r| r.needed)
                .sum();
            assert!((c.needed_soon - need).abs() < 0.001);
            assert_eq!(
                c.waiting_tasks.len(),
                readiness.iter().filter(|r| r.material_id == m.id).count()
            );

            // Sarf normativ hisobidan.
            let fact: f64 = consumption
                .iter()
                .filter(|x| x.material_id == m.id)
                .map(|x| x.fact)
                .sum();
            assert!((c.fact_total - fact).abs() < 0.001);
            assert!(c.overuse() >= 0.0);
            assert!(c.short() >= 0.0);

            // Almashtiruvchilar faqat tasdiqlanganlari.
            for (id, _, _) in &c.alternatives {
                assert!(app
                    .material_alts
                    .iter()
                    .any(|a| a.material_id == m.id && a.alt_id == *id && a.approved()));
            }
        }
        assert!(checked > 0, "kartochka umuman ochilmadi");
    }

    /// TZ IV.4: matritsa yangi talab o'ylab topmaydi — required_docs ga
    /// tayanadi va tugallangan, lekin hujjatsiz ishlarni oldinga chiqaradi.
    #[test]
    fn document_matrix_follows_requirements() {
        use crate::checks::MatrixCell;
        use crate::domain::ExecDocKind;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = app.document_matrix();
        let required = crate::checks::required_docs(&app.tasks, &app.exec_docs, false);
        assert!(!rows.is_empty(), "matritsa bo'sh");

        for r in &rows {
            assert_eq!(r.cells.len(), ExecDocKind::ALL.len());
            for (i, kind) in ExecDocKind::ALL.iter().enumerate() {
                let req = required
                    .iter()
                    .find(|x| x.task_id == r.task_id && x.kind == *kind);
                match (req, r.cells[i]) {
                    (None, c) => assert_eq!(c, MatrixCell::NotRequired),
                    (Some(x), MatrixCell::Signed) => assert!(x.signed),
                    (Some(x), MatrixCell::Draft) => assert!(x.exists && !x.signed),
                    (Some(x), MatrixCell::Missing) => assert!(!x.exists),
                    (Some(_), MatrixCell::NotRequired) => panic!("talab bor, katak bo'sh"),
                }
            }
        }

        // Tugallangan, lekin hujjatsiz ishlar oldinda.
        let mut seen_other = false;
        for r in &rows {
            if r.done && r.gaps() > 0 {
                assert!(!seen_other, "kechikkan ish pastga tushib qolgan");
            } else {
                seen_other = true;
            }
        }
    }

    /// TZ XI.23: uch shartdan biri yetishmasa chiqim nazoratsiz deb
    /// belgilanadi; uchalasi bo'lsa ro'yxatga tushmaydi.
    #[test]
    fn write_off_needs_reason_task_and_document() {
        use crate::checks::write_offs;
        use crate::domain::{MoveKind, StockMove};

        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        let mv = |note: &str, task: Option<i64>, doc: &str| StockMove {
            id: 1,
            project_id: 1,
            material_id: 3,
            warehouse_id: None,
            batch_id: None,
            date: today,
            kind: MoveKind::Out,
            qty: 5.0,
            price: 10.0,
            document: doc.into(),
            counterparty: String::new(),
            task_id: task,
            note: note.into(),
        };

        // Uchalasi bor — ro'yxatda yo'q.
        assert!(write_offs(&[mv("sarf", Some(1), "TN-1")], today, 30).is_empty());

        // Har bir yetishmovchilik alohida ko'rinadi.
        let no_reason = write_offs(&[mv("", Some(1), "TN-1")], today, 30);
        assert_eq!(no_reason.len(), 1);
        assert!(!no_reason[0].has_reason && no_reason[0].has_task);
        assert!(!no_reason[0].controlled());

        let no_task = write_offs(&[mv("sarf", None, "TN-1")], today, 30);
        assert!(!no_task[0].has_task);

        let no_doc = write_offs(&[mv("sarf", Some(1), "")], today, 30);
        assert!(!no_doc[0].has_document);

        // Kirim bu qoidaga tushmaydi.
        let mut incoming = mv("", None, "");
        incoming.kind = MoveKind::In;
        assert!(write_offs(&[incoming], today, 30).is_empty());

        // Davrdan tashqaridagi chiqim ham.
        let mut old = mv("", None, "");
        old.date = today - chrono::Duration::days(60);
        assert!(write_offs(&[old], today, 30).is_empty());
    }

    /// TZ III.10: GPR da bor, smetada yo'q ishlar; boshlanganlari
    /// oldinda turadi.
    #[test]
    fn missing_works_put_started_first() {
        use crate::checks::missing_works;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = missing_works(&app.tasks, &app.estimate_items);
        for m in &rows {
            let task = app.task(m.task_id).expect("ish");
            // Smetada bu ish yo'q — na bog'lanish, na nom bo'yicha.
            assert!(!app
                .estimate_items
                .iter()
                .any(|i| i.task_id == Some(m.task_id)));
            assert!(!app
                .estimate_items
                .iter()
                .any(|i| i.name.trim().to_lowercase() == task.name.trim().to_lowercase()));
            assert_eq!(m.started, task.progress > 0.0 || task.fact_start.is_some());
        }
        let mut seen_idle = false;
        for m in &rows {
            if m.started {
                assert!(!seen_idle, "boshlangan ish pastga tushib qolgan");
            } else {
                seen_idle = true;
            }
        }
    }

    /// TZ III.7: hajm solishtiruvi qo'pol — ikkala tomonda ham son
    /// bo'lgandagina farq hisoblanadi.
    #[test]
    fn volume_diff_needs_both_sides() {
        use crate::checks::{project_volumes, VOLUME_DIFF_PCT};
        use crate::domain::{ElementKind, EstimateItem};
        use crate::model::Section;

        let element = test_element(1, Section::Kj, ElementKind::Column, "K-1", 100.0);
        let item = |qty: f64| EstimateItem {
            id: 1,
            estimate_id: 1,
            pos: 1,
            section: Section::Kj,
            code: "E-1".into(),
            name: "Ustun".into(),
            unit: "m3".into(),
            qty,
            price: 1.0,
            cost: qty,
            task_id: None,
            note: String::new(),
        };

        // Faqat loyihada bor — farq hisoblanmaydi.
        let one = project_volumes(std::slice::from_ref(&element), &[]);
        assert_eq!(one.len(), 1);
        assert!(one[0].diff_pct.is_none());
        assert!(!one[0].off());

        // Ikkalasi bor va farq chegara ichida.
        let near = project_volumes(std::slice::from_ref(&element), &[item(105.0)]);
        assert!((near[0].diff_pct.unwrap() - 5.0).abs() < 0.001);
        assert!(!near[0].off());

        // Ikki barobar ko'p — e'tiroz.
        let big = project_volumes(std::slice::from_ref(&element), &[item(200.0)]);
        assert!(big[0].diff_pct.unwrap() > VOLUME_DIFF_PCT);
        assert!(big[0].off());
        assert_eq!(big[0].elements, 1);
        assert_eq!(big[0].items, 1);
    }

    /// TZ VII.27, XIV.26: PPR ish boshlanishidan oldin tasdiqlanishi
    /// kerak; keyin tasdiqlangani alohida ko'rsatiladi.
    #[test]
    fn ppr_must_be_approved_before_start() {
        use crate::checks::{ppr_control, PprIssue as P};
        use crate::domain::{PprDoc, PprKind};

        let start = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let mut task = test_task(10, 100.0, 30.0);
        task.fact_start = Some(start);
        let doc = |approved_at: Option<chrono::NaiveDate>| PprDoc {
            id: 1,
            project_id: 1,
            kind: PprKind::Ppr,
            number: "PPR-1".into(),
            name: "Monolit ishlari".into(),
            section: crate::model::Section::Kj,
            task_id: Some(10),
            workers: 5,
            machines: 1,
            path: String::new(),
            approved: approved_at.is_some(),
            author: String::new(),
            approved_at,
            note: String::new(),
        };

        // PPR yo'q — jiddiy.
        let none = ppr_control(std::slice::from_ref(&task), &[]);
        assert!(none.iter().any(|i| matches!(i, P::Missing { .. })));
        assert!(none[0].severe());

        // PPR bor, tasdiqlanmagan — ham jiddiy.
        let draft = ppr_control(std::slice::from_ref(&task), &[doc(None)]);
        assert!(draft.iter().any(|i| matches!(i, P::NotApproved { .. })));

        // Ish boshlanishidan oldin tasdiqlangan — e'tiroz yo'q.
        let early = ppr_control(
            std::slice::from_ref(&task),
            &[doc(Some(start - chrono::Duration::days(5)))],
        );
        assert!(early.is_empty());

        // Keyin tasdiqlangan — alohida e'tiroz, lekin to'xtatish emas.
        let late = ppr_control(
            std::slice::from_ref(&task),
            &[doc(Some(start + chrono::Duration::days(7)))],
        );
        let found = late
            .iter()
            .find(|i| matches!(i, P::ApprovedLate { days: 7, .. }))
            .expect("kech tasdiq e'tirozi yo'q");
        assert!(!found.severe());

        // Boshlanmagan ishdan PPR so'ralmaydi.
        let idle = test_task(11, 100.0, 0.0);
        assert!(ppr_control(std::slice::from_ref(&idle), &[]).is_empty());
    }

    /// TZ IV.17: marshrut bosqichi hujjat holatidan aniqlanadi —
    /// alohida yozuv yuritilmaydi.
    #[test]
    fn document_route_follows_the_status() {
        use crate::checks::doc_route;
        use crate::domain::ExecDocStatus;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut doc = t.db.exec_docs(pid).into_iter().next().expect("hujjat");

        doc.status = ExecDocStatus::Draft;
        let r = doc_route(&doc);
        assert_eq!(r.len(), 3);
        assert!(r[0].current && !r[0].done);
        assert!(!r[1].done && !r[2].done);

        doc.status = ExecDocStatus::OnReview;
        let r = doc_route(&doc);
        assert!(r[0].done);
        assert!(r[1].current);

        doc.status = ExecDocStatus::Signed;
        let r = doc_route(&doc);
        assert!(r.iter().all(|s| s.done));
        assert!(!r.iter().any(|s| s.current));

        // Rad etilgan hujjat boshiga qaytadi.
        doc.status = ExecDocStatus::Rejected;
        let r = doc_route(&doc);
        assert!(r[0].current && !r[0].done);
    }

    /// TZ X.16: tekshiruv hujjatga qaraydi — STIR, taqiq, aloqa,
    /// ulush va yetkazish tarixi.
    #[test]
    fn supplier_check_reads_the_card() {
        use crate::checks::{supplier_cards, SupplierIssue as S, INN_LEN, SUPPLIER_SHARE_LIMIT};
        use crate::domain::{PurchaseStatus, Supplier};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let today = chrono::Local::now().date_naive();
        let base = t.db.purchases(pid).into_iter().next().expect("xarid");

        let supplier = |inn: &str, blocked: bool, contact: &str| Supplier {
            id: 1,
            project_id: pid,
            name: base.supplier.clone(),
            inn: inn.into(),
            contact: contact.into(),
            phone: String::new(),
            blocked,
            note: String::new(),
        };
        let mut buy = base.clone();
        buy.status = PurchaseStatus::Ordered;
        buy.qty = 10.0;
        buy.price = 100.0;
        buy.delivered_qty = 10.0;
        buy.delivery_date = today + chrono::Duration::days(5);

        // To'g'ri kartochka: STIR to'g'ri, taqiq yo'q, aloqa bor.
        let good = "123456789";
        assert_eq!(good.len(), INN_LEN);
        let cards = supplier_cards(
            &[supplier(good, false, "Ali")],
            std::slice::from_ref(&buy),
            today,
        );
        // Yagona ta'minotchi — ulushi 100%, bu e'tiroz.
        assert!((cards[0].share_pct - 100.0).abs() < 0.001);
        assert!(cards[0].share_pct > SUPPLIER_SHARE_LIMIT);
        assert!(cards[0]
            .issues
            .iter()
            .any(|i| matches!(i, S::TooBigShare { .. })));

        // STIR yo'q va aloqa yo'q.
        let empty = supplier_cards(
            &[supplier("", false, "")],
            std::slice::from_ref(&buy),
            today,
        );
        assert!(empty[0].issues.contains(&S::NoInn));
        assert!(empty[0].issues.contains(&S::NoContact));

        // STIR uzunligi noto'g'ri — jiddiy.
        let bad = supplier_cards(
            &[supplier("12345", false, "Ali")],
            std::slice::from_ref(&buy),
            today,
        );
        let found = bad[0]
            .issues
            .iter()
            .find(|i| matches!(i, S::BadInn { .. }))
            .expect("STIR e'tirozi yo'q");
        assert!(found.severe());
        // Jiddiylari oldinda.
        assert!(bad[0].issues[0].severe());

        // Taqiqlangan, lekin xarid bor — jiddiy.
        let blocked = supplier_cards(
            &[supplier(good, true, "Ali")],
            std::slice::from_ref(&buy),
            today,
        );
        assert!(blocked[0]
            .issues
            .iter()
            .any(|i| matches!(i, S::BlockedButUsed { purchases: 1 })));

        // Kechikkan yetkazish tarixi.
        let mut late = buy.clone();
        late.delivery_date = today - chrono::Duration::days(3);
        late.delivered_qty = 0.0;
        let history = supplier_cards(
            &[supplier(good, false, "Ali")],
            std::slice::from_ref(&late),
            today,
        );
        assert!(history[0]
            .issues
            .iter()
            .any(|i| matches!(i, S::LateHistory { late: 1, total: 1 })));
    }

    /// TZ IV.14: maxsus jurnal alohida jadval emas — mavjud
    /// yozuvlarning ko'rinishi, yangi yozuv paydo bo'lmaydi.
    #[test]
    fn special_journals_are_views() {
        use crate::checks::SpecialJournal;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Beton jurnali namunalar sonidan oshmaydi.
        let concrete = app.special_journal(SpecialJournal::Concrete);
        assert_eq!(concrete.len(), app.concrete_tests.len());

        // Yashirin ishlar jurnali — aynan shu turdagi hujjatlar.
        let hidden = app.special_journal(SpecialJournal::Hidden);
        assert_eq!(
            hidden.len(),
            app.exec_docs
                .iter()
                .filter(|d| d.kind == crate::domain::ExecDocKind::Hidden)
                .count()
        );

        // Geodeziya jurnali — o'lchov nuqtalari, chetlanishi bilan.
        let geo = app.special_journal(SpecialJournal::Geodesy);
        assert_eq!(geo.len(), app.geodesy_points.len());
        for (line, point) in geo.iter().zip(
            app.geodesy_points
                .iter()
                .collect::<Vec<_>>()
                .into_iter()
                .rev(),
        ) {
            let _ = point;
            // Yangi yozuvlar oldinda.
            assert!(!line.number.is_empty() || line.subject.is_empty());
        }
        for w in geo.windows(2) {
            assert!(w[0].date >= w[1].date);
        }

        // Payvand jurnali faqat KM bo'limi tekshiruvlaridan.
        let weld = app.special_journal(SpecialJournal::Welding);
        for line in &weld {
            assert!(app.quality.iter().any(|q| q.date == line.date));
        }
    }

    /// TZ IV.22: imzolangan hujjat ortida kamida bitta qurilish yozuvi
    /// bo'lishi kerak; tasdiqsizlari ro'yxat boshida turadi.
    #[test]
    fn signed_documents_need_site_records() {
        use crate::checks::{doc_evidence, EVIDENCE_WINDOW};
        use crate::domain::ExecDocStatus;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let rows = doc_evidence(
            &app.exec_docs,
            &app.journal,
            &app.timesheet,
            &app.stock_moves,
        );
        for e in &rows {
            let doc = app
                .exec_docs
                .iter()
                .find(|d| d.id == e.doc_id)
                .expect("hujjat");
            // Faqat imzolangan va ishga bog'langan hujjatlar tekshiriladi.
            assert_eq!(doc.status, ExecDocStatus::Signed);
            assert!(doc.task_id.is_some());
            assert_eq!(
                e.sources(),
                [e.in_journal, e.in_timesheet, e.has_material]
                    .iter()
                    .filter(|x| **x)
                    .count()
            );
            assert_eq!(e.unsupported(), e.sources() == 0);

            // Jurnal tasdig'i oyna ichidagi yozuvdan chiqadi.
            if e.in_journal {
                let from = doc.date - chrono::Duration::days(EVIDENCE_WINDOW);
                let to = doc.date + chrono::Duration::days(EVIDENCE_WINDOW);
                assert!(app.journal.iter().any(|j| {
                    j.task_id == doc.task_id && j.date >= from && j.date <= to && j.volume > 0.0
                }));
            }
        }

        // Tasdiqsizlari oldinda.
        for w in rows.windows(2) {
            assert!(w[0].sources() <= w[1].sources());
        }
    }

    /// Umumiy talab: izoh mexanizmi asosiy yozuv turlarining hammasida
    /// bir xil ishlaydi — bir joyda izoh bor, boshqasida yo'q bo'lmasin.
    #[test]
    fn notes_work_for_every_wired_target() {
        use crate::domain::{Note, NoteTarget};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();

        // Ekranlarga ulangan yozuv turlari.
        let wired = [
            NoteTarget::Task,
            NoteTarget::Issue,
            NoteTarget::Request,
            NoteTarget::Purchase,
            NoteTarget::ExecDoc,
            NoteTarget::Quality,
            NoteTarget::Safety,
            NoteTarget::Inspection,
            NoteTarget::Machine,
            NoteTarget::Material,
            NoteTarget::Document,
        ];
        for (i, target) in wired.iter().enumerate() {
            let id = i as i64 + 1;
            t.db.insert_note(&Note {
                id: 0,
                project_id: pid,
                target: *target,
                target_id: id,
                author: "Sinov".into(),
                at: "2026-04-10 09:00".into(),
                text: format!("{} izohi", target.code()),
                parent: None,
                resolved: false,
            });
            let mine: Vec<Note> =
                t.db.notes(pid)
                    .into_iter()
                    .filter(|n| n.target == *target && n.target_id == id)
                    .collect();
            assert_eq!(mine.len(), 1, "{target:?}: izoh saqlanmadi");
            assert!(!mine[0].resolved);
        }
    }

    /// TZ II.1-2: DXF chizmasidagi matnlar elementga aylanadi va
    /// takroriy import yangi nusxa yaratmaydi.
    #[test]
    fn dxf_import_adds_elements_once() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let dxf =
            "0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\nAR-Markalar\n10\n10.0\n20\n20.0\n1\nOK-77\n\
                   0\nTEXT\n8\nKJ-Kolonnalar\n10\n30.0\n20\n40.0\n1\nK-9\n\
                   0\nLINE\n8\nAR-Devor\n10\n0.0\n20\n0.0\n0\nEOF\n";
        let path = std::env::temp_dir().join(format!("qurai_t_{}.dxf", std::process::id()));
        std::fs::write(&path, dxf).unwrap();

        let before = app.elements.len();
        app.import_dxf(&path);
        assert_eq!(app.elements.len(), before + 2, "elementlar qo'shilmadi");

        let ok = app
            .elements
            .iter()
            .find(|e| e.mark == "OK-77")
            .expect("marka");
        assert_eq!(ok.section, crate::model::Section::Ar);
        assert_eq!(ok.kind, crate::domain::ElementKind::Window);
        // Qaysi varaqdan kelgani va qatlam nomi saqlanadi.
        assert!(!ok.sheet.is_empty());
        assert!(ok.note.contains("AR-Markalar"));

        let k = app
            .elements
            .iter()
            .find(|e| e.mark == "K-9")
            .expect("marka");
        assert_eq!(k.section, crate::model::Section::Kj);

        // Ikkinchi marta import qilinganda nusxa paydo bo'lmaydi.
        app.import_dxf(&path);
        assert_eq!(app.elements.len(), before + 2, "takroriy nusxa qo'shildi");

        let _ = std::fs::remove_file(&path);
    }

    /// Telefondan (server orqali) kelgan kunlik yozuv ilovaga tushadi.
    ///
    /// Paket matni serverdagi forma qanday yozsa, aynan shunday yoziladi:
    /// ikkala tomon bir xil formatdan foydalanishi shu yerda tekshiriladi.
    #[test]
    fn journal_from_the_phone_reaches_the_desktop() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        let before = app.journal.len();
        let text = "QURAI-PACKAGE\t1\nPROJECT\tOBY-1\nCREATED\t2026-09-04T10:00:00Z\n\n\
                    #journal\n\
                    date\tauthor\tweather\ttemperature\tworkers\tmachines\ttask\tvolume\tunit\ttext\tremarks\n\
                    2026-09-04\tAlisher\tochiq\t0\t8\t2\t\t12.5\tm3\tBeton quyildi\t\n";

        let pkg = crate::package::read(text).expect("paket o'qildi");
        let (added, _) = app.import_package(&pkg);
        assert_eq!(added, 1, "yozuv qo'shilmadi");
        app.reload_modules();
        assert_eq!(app.journal.len(), before + 1);

        let entry = app
            .journal
            .iter()
            .find(|j| j.text == "Beton quyildi")
            .expect("yozuv");
        assert_eq!(entry.author, "Alisher");
        assert_eq!(entry.volume, 12.5);
        assert_eq!(entry.unit, "m3");
        assert_eq!(entry.workers, 8);
        assert_eq!(entry.machines, 2);

        // Ikkinchi marta qo'llansa nusxa paydo bo'lmaydi.
        let (again, skipped) = app.import_package(&pkg);
        assert_eq!(again, 0, "nusxa qo'shildi");
        assert!(skipped >= 1);
    }

    /// Telefondan kelgan tabel ilovaga tushadi va soat to'g'ri o'qiladi.
    #[test]
    fn timesheet_from_the_phone_reaches_the_desktop() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mut app = crate::app::App::new(Db::open(&t.path).unwrap());
        app.select_project(pid);

        // Namunadagi haqiqiy ishchi nomi olinadi: paket ism bo'yicha
        // ulanadi, shuning uchun mavjud ishchi bo'lishi kerak.
        let worker = app.workers.first().cloned().expect("ishchi");
        let day = app.today;
        let text = format!(
            "QURAI-PACKAGE\t1\nPROJECT\tOBY-1\nCREATED\t2026-09-07T10:00:00Z\n\n\
             #timesheet\ndate\tworker\thours\tkind\tshift\n\
             {}\t{}\t7.5\twork\tday\n",
            day, worker.name
        );

        let pkg = crate::package::read(&text).expect("paket");
        let (added, _) = app.import_package(&pkg);
        assert!(added >= 1, "tabel qatori qo'shilmadi");
        app.reload_modules();

        let entry = app
            .timesheet
            .iter()
            .find(|e| e.worker_id == worker.id && e.date == day)
            .expect("tabel katagi");
        assert_eq!(entry.hours, 7.5);
        assert_eq!(entry.kind, crate::domain::DayKind::Work);
    }

    /// TZ XI.21: qaytarish qoldiqni oshiradi.
    #[test]
    fn return_increases_the_balance() {
        use crate::domain::{MoveKind, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "R-1", 0.0));
        let today = chrono::Local::now().date_naive();
        let mv = |kind: MoveKind, qty: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: today,
                kind,
                qty,
                price: 0.0,
                document: String::new(),
                counterparty: String::new(),
                task_id: None,
                note: String::new(),
                warehouse_id: None,
                batch_id: None,
            });
        };
        mv(MoveKind::In, 100.0);
        mv(MoveKind::Out, 40.0);
        mv(MoveKind::Return, 10.0);

        let lines =
            crate::checks::stock_balances(&t.db.materials(pid), &t.db.stock_moves(pid), &[], today);
        let l = lines.iter().find(|l| l.material_id == mid).unwrap();
        assert_eq!(l.balance, 70.0);
        assert_eq!(l.returned, 10.0);
    }

    /// TZ XI.17: rezerv erkin qoldiqni kamaytiradi, lekin qoldiqni emas.
    #[test]
    fn reservation_reduces_available_not_balance() {
        use crate::domain::{MoveKind, Reservation, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        // Minimal zaxira 30: rezervdan keyin erkin qoldiq undan pastga tushadi.
        let mid = t.db.insert_material(&test_material(pid, "R-2", 30.0));
        let today = chrono::Local::now().date_naive();
        t.db.insert_stock_move(&StockMove {
            id: 0,
            project_id: pid,
            material_id: mid,
            date: today,
            kind: MoveKind::In,
            qty: 50.0,
            price: 0.0,
            document: String::new(),
            counterparty: String::new(),
            task_id: None,
            note: String::new(),
            warehouse_id: None,
            batch_id: None,
        });
        t.db.insert_reservation(&Reservation {
            id: 0,
            project_id: pid,
            material_id: mid,
            task_id: None,
            qty: 30.0,
            date: today,
            until: None,
            note: String::new(),
        });

        let lines = crate::checks::stock_balances(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.reservations(pid),
            today,
        );
        let l = lines.iter().find(|l| l.material_id == mid).unwrap();
        assert_eq!(l.balance, 50.0, "qoldiq o'zgarmasligi kerak");
        assert_eq!(l.reserved, 30.0);
        assert_eq!(l.available, 20.0);
        // Ogohlantirish erkin qoldiqqa qaraydi.
        assert!(l.below_min);

        // Muddati o'tgan rezerv hisobga olinmaydi.
        // Namuna ma'lumotida ham rezerv bor — o'zimiznikini material bo'yicha topamiz.
        let mut r =
            t.db.reservations(pid)
                .into_iter()
                .find(|r| r.material_id == mid)
                .expect("rezerv");
        r.until = Some(today - chrono::Duration::days(1));
        t.db.update_reservation(&r);
        let lines = crate::checks::stock_balances(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.reservations(pid),
            today,
        );
        let l = lines.iter().find(|l| l.material_id == mid).unwrap();
        assert_eq!(l.reserved, 0.0);
        assert_eq!(l.available, 50.0);
        assert!(!l.below_min);
    }

    /// TZ XI.3: har bir ombor o'z qoldig'ini yuritadi.
    #[test]
    fn balances_are_split_by_warehouse() {
        use crate::domain::{MoveKind, StockMove, Warehouse, WarehouseKind};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "R-3", 0.0));
        let today = chrono::Local::now().date_naive();
        let wh = |name: &str, kind: WarehouseKind| {
            t.db.insert_warehouse(&Warehouse {
                id: 0,
                project_id: pid,
                name: name.into(),
                kind,
                responsible: String::new(),
                note: String::new(),
            })
        };
        let central = wh("Markaziy", WarehouseKind::Central);
        let site = wh("Obyekt", WarehouseKind::Object);

        let mv = |w: i64, kind: MoveKind, qty: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: today,
                kind,
                qty,
                price: 0.0,
                document: String::new(),
                counterparty: String::new(),
                task_id: None,
                note: String::new(),
                warehouse_id: Some(w),
                batch_id: None,
            });
        };
        mv(central, MoveKind::In, 100.0);
        mv(site, MoveKind::In, 40.0);
        mv(site, MoveKind::Out, 15.0);

        let materials = t.db.materials(pid);
        let moves = t.db.stock_moves(pid);
        let at = |w: Option<i64>| {
            crate::checks::stock_balances_in(&materials, &moves, &[], today, w)
                .into_iter()
                .find(|l| l.material_id == mid)
                .unwrap()
                .balance
        };
        assert_eq!(at(Some(central)), 100.0);
        assert_eq!(at(Some(site)), 25.0);
        assert_eq!(
            at(None),
            125.0,
            "umumiy qoldiq — barcha omborlar yig'indisi"
        );
    }

    /// TZ XI.9 va XI.28: partiya qoldig'i va FEFO navbati.
    #[test]
    fn batches_track_balance_and_fefo_order() {
        use crate::domain::{Batch, MoveKind, StockMove};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&test_material(pid, "R-4", 0.0));
        let today = chrono::Local::now().date_naive();

        let mk = |number: &str, days_ago: i64, expires_in: Option<i64>| {
            t.db.insert_batch(&Batch {
                id: 0,
                project_id: pid,
                material_id: mid,
                number: number.into(),
                received: today - chrono::Duration::days(days_ago),
                supplier: String::new(),
                cert_no: String::new(),
                cert_until: None,
                expires: expires_in.map(|d| today + chrono::Duration::days(d)),
                note: String::new(),
            })
        };
        // Birinchi kelgan, lekin kech tugaydi.
        let old = mk("P-001", 30, Some(90));
        // Keyin kelgan, ammo tezroq tugaydi — FEFO shuni birinchi beradi.
        let soon = mk("P-002", 5, Some(10));

        let mv = |b: i64, kind: MoveKind, qty: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: today,
                kind,
                qty,
                price: 0.0,
                document: String::new(),
                counterparty: String::new(),
                task_id: None,
                note: String::new(),
                warehouse_id: None,
                batch_id: Some(b),
            });
        };
        mv(old, MoveKind::In, 20.0);
        mv(soon, MoveKind::In, 30.0);
        mv(old, MoveKind::Out, 5.0);

        let lines =
            crate::checks::batch_balances(&t.db.batches(pid), &t.db.stock_moves(pid), today);
        let get = |id: i64| lines.iter().find(|l| l.batch_id == id).unwrap();
        assert_eq!(get(old).balance, 15.0);
        assert_eq!(get(soon).balance, 30.0);
        assert!(
            get(soon).next_to_use,
            "FEFO: muddati yaqin partiya birinchi"
        );
        assert!(!get(old).next_to_use);
        assert!(!get(soon).expired);
    }

    /// TZ XI.25: inventarizatsiya farqlari.
    #[test]
    fn inventory_reports_only_real_differences() {
        use crate::domain::{Inventory, InventoryLine};

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let a = t.db.insert_material(&test_material(pid, "R-5", 0.0));
        let b = t.db.insert_material(&test_material(pid, "R-6", 0.0));
        let today = chrono::Local::now().date_naive();

        let inv = t.db.insert_inventory(&Inventory {
            id: 0,
            project_id: pid,
            warehouse_id: None,
            date: today,
            responsible: "Omborchi".into(),
            closed: false,
            note: String::new(),
        });
        let line = |mid: i64, book: f64, fact: f64| {
            t.db.insert_inventory_line(&InventoryLine {
                id: 0,
                inventory_id: inv,
                material_id: mid,
                book,
                fact,
                note: String::new(),
            });
        };
        line(a, 100.0, 96.0);
        line(b, 50.0, 50.0);

        let diffs = crate::checks::inventory_diffs(&t.db.inventory_lines(pid), inv);
        assert_eq!(diffs.len(), 1, "faqat haqiqiy farq chiqishi kerak");
        assert_eq!(diffs[0].material_id, a);
        assert_eq!(diffs[0].diff, -4.0, "kamomad manfiy bo'ladi");
    }

    /// TZ XI: qoldiq = kirim - chiqim - hisobdan chiqarish; qiymati kirim
    /// narxlarining vaznlangan o'rtachasi bo'yicha hisoblanadi.
    #[test]
    fn stock_balance_matches_moves() {
        use crate::domain::{Material, MoveKind, StockMove};
        use crate::model::Section;

        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let mid = t.db.insert_material(&Material {
            id: 0,
            project_id: pid,
            code: "T-1".into(),
            name: "Test".into(),
            unit: "m3".into(),
            section: Section::Kj,
            spec: String::new(),
            cert_no: String::new(),
            cert_until: None,
            min_stock: 50.0,
            price: 1_000.0,
            estimate_code: String::new(),
            spec_ref: String::new(),
            special: String::new(),
            banned: false,
            ban_reason: String::new(),
            note: String::new(),
        });
        let today = chrono::Local::now().date_naive();
        let mv = |kind: MoveKind, qty: f64, price: f64| {
            t.db.insert_stock_move(&StockMove {
                id: 0,
                project_id: pid,
                material_id: mid,
                date: today,
                kind,
                qty,
                price,
                document: String::new(),
                counterparty: String::new(),
                task_id: None,
                note: String::new(),
                warehouse_id: None,
                batch_id: None,
            });
        };
        mv(MoveKind::In, 100.0, 800.0);
        mv(MoveKind::In, 100.0, 1_200.0);
        mv(MoveKind::Out, 130.0, 0.0);
        mv(MoveKind::WriteOff, 10.0, 0.0);

        let lines = crate::checks::stock_balances(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.reservations(pid),
            today,
        );
        let l = lines.iter().find(|l| l.material_id == mid).expect("qator");
        assert_eq!(l.balance, 60.0);
        // Kirimlar o'rtachasi: (100*800 + 100*1200) / 200 = 1000.
        assert_eq!(l.unit_price, 1_000.0);
        assert_eq!(l.value, 60_000.0);
        assert!(!l.negative);
        // 60 > 50 — minimal zaxiradan yuqori.
        assert!(!l.below_min);

        // Ortiqcha chiqim manfiy qoldiq beradi va shu holat belgilanadi.
        mv(MoveKind::Out, 100.0, 0.0);
        let lines = crate::checks::stock_balances(
            &t.db.materials(pid),
            &t.db.stock_moves(pid),
            &t.db.reservations(pid),
            today,
        );
        let l = lines.iter().find(|l| l.material_id == mid).unwrap();
        assert_eq!(l.balance, -40.0);
        assert!(l.negative);
        assert!(l.below_min);
        // Manfiy qoldiq qiymat bermaydi.
        assert_eq!(l.value, 0.0);
    }

    /// Namuna ma'lumoti katalog va harakatlarni ham to'ldiradi, qayta
    /// chaqirilganda nusxalamaydi.
    #[test]
    fn demo_stock_is_seeded_once() {
        let t = TempDb::new();
        let pid = t.db.seed_demo().unwrap();
        let n = t.db.materials(pid).len();
        assert!(n >= 5, "katalog to'lmagan: {n}");
        assert!(!t.db.stock_moves(pid).is_empty());

        t.db.seed_demo_stock(pid, false);
        assert_eq!(t.db.materials(pid).len(), n);
    }
}
