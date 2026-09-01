//! Loyiha paketi: bitta obyektni faylga chiqarish va boshqa nusxaga olib kirish.
//!
//! Server yo'q, shuning uchun qurilmalar orasida ma'lumot **fayl orqali**
//! ko'chadi: obyektdagi bir kunlik ish qurilish maydonchasida to'ldiriladi,
//! paket ofisga olib kelinadi va u yerdagi bazaga qo'shiladi.
//!
//! Paket — bu SQLite fayli emas, balki oddiy **matnli jadval to'plami**: har
//! bo'lim `#TABLE` sarlavhasi bilan boshlanadi, keyin ustunlar va qatorlar
//! keladi. Shu sabab paketni odam ham o'qiy oladi va nima ko'chganini
//! ko'zdan kechirishi mumkin — bu ishonch uchun muhim.
//!
//! Import **qo'shadi, o'chirmaydi**: mavjud yozuvlarga tegilmaydi. Shuning
//! uchun ikki tomonlama tahrir qilingan yozuv avtomatik birlashtirilmaydi —
//! bu ataylab: ma'lumotni jimgina qayta yozgandan ko'ra, dublikatni odam
//! ko'rib hal qilgani xavfsizroq.

use std::collections::HashMap;

/// Paket versiyasi — kelajakda format o'zgarsa, eskisini tanish uchun.
pub const VERSION: &str = "1";

/// Bitta jadval: nomi, ustunlari va qatorlari.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// O'qilgan paket.
#[derive(Debug, Clone, Default)]
pub struct Package {
    pub version: String,
    /// Paket qaysi obyektdan olingani — import paytida ko'rsatiladi.
    pub project: String,
    pub created: String,
    pub tables: Vec<Table>,
}

impl Package {
    pub fn table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// Paketdagi jami qatorlar soni — hisobotda ko'rsatiladi.
    pub fn row_count(&self) -> usize {
        self.tables.iter().map(|t| t.rows.len()).sum()
    }
}

// ================================================================ Yozish

/// Maydonni xavfsiz ko'rinishga keltiradi.
///
/// Ajratgich — tabulyatsiya, shuning uchun matndagi tabulyatsiya va qator
/// ko'chirish belgilari qochiriladi. Aks holda bitta izoh butun jadvalni
/// buzib yuborardi.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "")
        .replace('\n', "\\n")
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Paketni matnga aylantiradi.
pub fn write(pkg: &Package) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let _ = writeln!(o, "QURAI-PACKAGE\t{}", pkg.version);
    let _ = writeln!(o, "PROJECT\t{}", escape(&pkg.project));
    let _ = writeln!(o, "CREATED\t{}", escape(&pkg.created));
    for t in &pkg.tables {
        let _ = writeln!(o);
        let _ = writeln!(o, "#{}", t.name);
        let _ = writeln!(o, "{}", t.columns.join("\t"));
        for r in &t.rows {
            let cells: Vec<String> = r.iter().map(|c| escape(c)).collect();
            let _ = writeln!(o, "{}", cells.join("\t"));
        }
    }
    o
}

// ================================================================ O'qish

/// Matndan paketni o'qiydi.
///
/// Buzilgan qator butun faylni yo'qotmaydi: ustunlar soniga to'g'ri kelmagan
/// qator tashlab yuboriladi va qolgani o'qilaveradi.
pub fn read(src: &str) -> Result<Package, String> {
    let mut pkg = Package::default();
    let mut current: Option<Table> = None;
    let mut expect_columns = false;
    let mut first = true;

    for line in src.lines() {
        let line = line.trim_end_matches('\r');
        if first {
            first = false;
            let mut it = line.split('\t');
            if it.next() != Some("QURAI-PACKAGE") {
                return Err("not_a_package".into());
            }
            pkg.version = it.next().unwrap_or("").to_string();
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("PROJECT\t") {
            pkg.project = unescape(rest);
            continue;
        }
        if let Some(rest) = line.strip_prefix("CREATED\t") {
            pkg.created = unescape(rest);
            continue;
        }
        if let Some(name) = line.strip_prefix('#') {
            if let Some(t) = current.take() {
                pkg.tables.push(t);
            }
            current = Some(Table {
                name: name.trim().to_string(),
                columns: Vec::new(),
                rows: Vec::new(),
            });
            expect_columns = true;
            continue;
        }
        let Some(t) = current.as_mut() else { continue };
        if expect_columns {
            t.columns = line.split('\t').map(|c| c.trim().to_string()).collect();
            expect_columns = false;
            continue;
        }
        let cells: Vec<String> = line.split('\t').map(unescape).collect();
        // Ustunlar soni mos kelmasa — qator ishonchsiz, olinmaydi.
        if cells.len() == t.columns.len() {
            t.rows.push(cells);
        }
    }
    // Bo'sh fayl ham paket emas: sarlavha umuman o'qilmagan.
    if first {
        return Err("not_a_package".into());
    }
    if let Some(t) = current.take() {
        pkg.tables.push(t);
    }
    Ok(pkg)
}

/// Qatorni ustun nomlari bo'yicha o'qish uchun qulay ko'rinish.
pub fn row_map<'a>(t: &'a Table, row: &'a [String]) -> HashMap<&'a str, &'a str> {
    t.columns
        .iter()
        .zip(row.iter())
        .map(|(c, v)| (c.as_str(), v.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Package {
        Package {
            version: VERSION.into(),
            project: "«Navro'z» TJM".into(),
            created: "2026-09-01".into(),
            tables: vec![
                Table {
                    name: "journal".into(),
                    columns: vec!["date".into(), "text".into(), "volume".into()],
                    rows: vec![
                        vec!["2026-09-01".into(), "Beton quyildi".into(), "420".into()],
                        // Ichida tabulyatsiya va qator ko'chirish bor izoh.
                        vec![
                            "2026-09-02".into(),
                            "Ikki\tustun\nva ikki qator".into(),
                            "0".into(),
                        ],
                    ],
                },
                Table {
                    name: "timesheet".into(),
                    columns: vec!["worker".into(), "hours".into()],
                    rows: vec![vec!["Karimov A.".into(), "8".into()]],
                },
            ],
        }
    }

    #[test]
    fn write_then_read_gives_the_same_data() {
        let a = sample();
        let text = write(&a);
        let b = read(&text).expect("o'qildi");
        assert_eq!(b.version, a.version);
        assert_eq!(b.project, a.project);
        assert_eq!(b.tables, a.tables);
        assert_eq!(b.row_count(), 3);
    }

    #[test]
    fn tabs_and_newlines_survive() {
        let text = write(&sample());
        let b = read(&text).unwrap();
        let j = b.table("journal").unwrap();
        assert_eq!(j.rows[1][1], "Ikki\tustun\nva ikki qator");
        // Fayl ichida ular qochirilgan bo'lishi kerak.
        assert!(text.contains("Ikki\\tustun\\nva ikki qator"));
    }

    #[test]
    fn foreign_file_is_refused() {
        assert!(read("bu oddiy matn\nikkinchi qator").is_err());
        assert!(read("").is_err());
    }

    /// Buzilgan qator tashlanadi, qolgani o'qiladi.
    #[test]
    fn broken_row_is_skipped() {
        let text = "QURAI-PACKAGE\t1\nPROJECT\tX\n\n#journal\ndate\ttext\n2026-09-01\tYaxshi\nfaqat bitta ustun\n2026-09-02\tYana yaxshi\n";
        let p = read(text).unwrap();
        let j = p.table("journal").unwrap();
        assert_eq!(j.rows.len(), 2);
        assert_eq!(j.rows[1][1], "Yana yaxshi");
    }

    #[test]
    fn row_map_reads_by_column_name() {
        let p = sample();
        let t = p.table("timesheet").unwrap();
        let m = row_map(t, &t.rows[0]);
        assert_eq!(m.get("worker"), Some(&"Karimov A."));
        assert_eq!(m.get("hours"), Some(&"8"));
        assert_eq!(m.get("yo'q"), None);
    }
}
