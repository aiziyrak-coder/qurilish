//! Baza zaxira nusxasi.
//!
//! Kod git bilan qaytadi, **ma'lumot qaytmaydi** — shuning uchun zaxira nusxa
//! alohida imkoniyat sifatida ajratilgan. SQLite WAL rejimida ishlagani uchun
//! faqat `.db` faylini ko'chirish yetarli emas: yozilmagan tranzaksiyalar
//! `-wal` faylida qolishi mumkin. Shu sababli nusxa `VACUUM INTO` orqali
//! olinadi — bu bazaning izchil, siqilgan nusxasini bitta faylga yozadi.

use std::path::{Path, PathBuf};

/// Zaxira fayli uchun nom: `qurai-YYYY-MM-DD-HHMM.db`.
pub fn suggested_name(now: chrono::NaiveDateTime) -> String {
    format!("qurai-{}.db", now.format("%Y-%m-%d-%H%M"))
}

/// Natija: nusxa yo'li yoki xato matni.
pub type Result = std::result::Result<PathBuf, String>;

/// Bazaning izchil nusxasini `dest` ga yozadi.
///
/// Fayl allaqachon bo'lsa — `VACUUM INTO` xato beradi (SQLite mavjud faylga
/// yozmaydi), shuning uchun avval o'chirib olamiz: foydalanuvchi saqlash
/// oynasida ustiga yozishni allaqachon tasdiqlagan.
pub fn create(conn: &rusqlite::Connection, dest: &Path) -> Result {
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // Yo'lni SQL ichiga qo'ymaymiz — parametr sifatida beramiz.
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])
        .map_err(|e| e.to_string())?;
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_has_date_and_time() {
        let dt = chrono::NaiveDate::from_ymd_opt(2026, 9, 1)
            .unwrap()
            .and_hms_opt(14, 5, 0)
            .unwrap();
        assert_eq!(suggested_name(dt), "qurai-2026-09-01-1405.db");
    }

    /// Nusxa haqiqiy fayl bo'lib chiqadi va undagi ma'lumot o'qiladi.
    #[test]
    fn backup_is_a_readable_copy() {
        let dir = std::env::temp_dir().join(format!("qurai_bk_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.db");
        let dst = dir.join("copy.db");

        let conn = rusqlite::Connection::open(&src).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT);
             INSERT INTO t (name) VALUES ('Navro''z');",
        )
        .unwrap();

        let out = create(&conn, &dst).expect("nusxa");
        assert!(out.exists());

        let copy = rusqlite::Connection::open(&dst).unwrap();
        let name: String = copy
            .query_row("SELECT name FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "Navro'z");
        // Nusxani band qilib turmaymiz: Windows ochiq faylni o'chirtirmaydi.
        drop(copy);

        // Mavjud fayl ustiga qayta yozish ham ishlaydi.
        create(&conn, &dst).expect("qayta nusxa");

        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
