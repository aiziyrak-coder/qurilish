//! Локальное хранилище (SQLite). Схема писалась с расчетом на будущую
//! синхронизацию с сервером: у каждой записи есть `updated_at`, ключи не переиспользуются.

use crate::model::*;
use chrono::NaiveDate;
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;

pub struct Db {
    conn: Connection,
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

    pub fn open(path: &PathBuf) -> SqlResult<Db> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        // Baza band bo'lsa darhol xato bermay, 5 soniya kutadi.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        let db = Db { conn };
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

        Ok(pid)
    }
}

/// Namoyish ishining tavsifi:
/// VBS, nomi (uz), nomi (ru), bo'lim, davomiyligi, mas'ul, bajarilgan %, hajm, birlik.
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

        assert_eq!(t.db.project_count().unwrap(), 1);
        // Obyekt holati saqlanganidek qaytishi kerak.
        let project = t.db.projects().unwrap().remove(0);
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
        assert_eq!(t.db.estimates(pid).len(), 1);
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
        let saved = t.db.issues(pid);
        assert_eq!(saved.len(), project_issues.len() + estimate_issues.len());
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

        app.apply_journal_to_tasks();

        // Namoyish jurnalida shu ish bo'yicha 420 + 380 + 365 = 1165 birlik.
        let expected = 1165.0 / task.volume * 100.0;
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

        // Boshida natija yo'q.
        assert!(app.issues.is_empty());

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

        assert_eq!(t.db.project_count().unwrap(), 0);
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
        assert_eq!(workers.len(), 6);
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
