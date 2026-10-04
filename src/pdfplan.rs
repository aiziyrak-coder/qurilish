//! PDF loyihadan element spetsifikatsiyasini o'qish (TZ II.1).
//!
//! Haqiqiy ishda loyiha ko'pincha **PDF** bo'lib keladi: IFC va DXF
//! loyihachida bo'ladi, buyurtmachiga esa chop etishga tayyor PDF
//! beriladi. Shuning uchun PDF — chetdagi holat emas, asosiy yo'l.
//!
//! PDF da chizmaning o'zi vektor yoki rasm bo'lib turadi va undan element
//! chiqarib bo'lmaydi. Lekin loyihada deyarli har doim **spetsifikatsiya
//! jadvali** bo'ladi va element o'sha jadvaldan olinadi.
//!
//! Qoida qat'iy: **avval jadval sarlavhasi topiladi**, keyin faqat uning
//! ostidagi qatorlar o'qiladi va qiymat sarlavha ko'rsatgan ustundan
//! olinadi. Sarlavha topilmasa — hech narsa o'qilmaydi.
//!
//! Buning sababi tajribadan: avval qator shakliga qarab marka tanilardi
//! («harf ham, raqam ham bor»). Haqiqiy loyihada bu `159.9кг`, `5270шт`,
//! `-16x400x400` kabi **o'lcham va miqdorlarni** marka deb olib, bitta
//! hujjatdan sakkiz yuzdan ortiq soxta element yasadi. Shakl bo'yicha
//! markani miqdordan ajratib bo'lmaydi — buni faqat ustun sarlavhasi
//! aytadi.
//!
//! Chegara ochiq: **skan qilingan PDF o'qilmaydi** — unda matn yo'q, rasm
//! bor. Bunday faylda dastur taxmin qilmaydi, shunday deb aytadi.

use crate::domain::{Element, ElementKind};
use crate::model::Section;

/// Spetsifikatsiya jadvalining ustunlari.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Columns {
    /// Marka (yoki belgi) ustuni — elementni shu aniqlaydi.
    pub mark: usize,
    pub name: Option<usize>,
    pub qty: Option<usize>,
}

/// Ustun turi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `Марка`, `Обозначение` — elementning o'z belgisi.
    Mark,
    /// `Поз.` — jadvaldagi tartib raqami. Marka topilmasa ishlatiladi.
    Position,
    Name,
    Qty,
}

/// Ustun nomini taniydi.
///
/// Ikki tilda ham, qisqartma ko'rinishda ham yoziladi. Ro'yxat yopiq:
/// tanilmagan sarlavha ustun deb olinmaydi, chunki noto'g'ri tanilgan
/// ustun butun jadvalni buzardi.
fn column_kind(cell: &str) -> Option<Kind> {
    let c = cell.trim().to_lowercase();
    if c.is_empty() || c.chars().count() > 40 {
        return None;
    }
    const MARK: [&str; 6] = ["марка", "обознач", "шифр", "marka", "belgi", "shifr"];
    const POSITION: [&str; 4] = ["поз", "pozitsiya", "poz.", "тартиб"];
    const NAME: [&str; 5] = ["наименован", "название", "nomlanish", "nomi", "nom"];
    const QTY: [&str; 6] = ["кол-во", "кол.", "количест", "miqdor", "soni", "son"];

    if MARK.iter().any(|k| c.contains(k)) {
        return Some(Kind::Mark);
    }
    if POSITION.iter().any(|k| c.contains(k)) {
        return Some(Kind::Position);
    }
    if NAME.iter().any(|k| c.contains(k)) {
        return Some(Kind::Name);
    }
    if QTY.iter().any(|k| c.contains(k)) {
        return Some(Kind::Qty);
    }
    None
}

/// Qator spetsifikatsiya sarlavhasimi.
///
/// Sarlavha deb tan olinishi uchun **marka (yoki pozitsiya) va nom**
/// ustunlari birga bo'lishi shart. Faqat bittasi bo'lsa bu sarlavha emas,
/// balki matn ichidagi tasodifiy so'z bo'lishi mumkin.
pub fn header(row: &[String]) -> Option<Columns> {
    let mut mark: Option<usize> = None;
    let mut position: Option<usize> = None;
    let mut name: Option<usize> = None;
    let mut qty: Option<usize> = None;
    for (i, cell) in row.iter().enumerate() {
        match column_kind(cell) {
            // Birinchi uchragani olinadi: jadval sarlavhasi bir marta
            // yoziladi, takrori esa odatda keyingi jadvalniki.
            Some(Kind::Mark) if mark.is_none() => mark = Some(i),
            Some(Kind::Position) if position.is_none() => position = Some(i),
            Some(Kind::Name) if name.is_none() => name = Some(i),
            Some(Kind::Qty) if qty.is_none() => qty = Some(i),
            _ => {}
        }
    }
    let name = name?;
    // Marka ustuni ustun turadi: `Поз.` da tartib raqami bo'ladi va u
    // elementni aniqlamaydi.
    let mark = mark.or(position)?;
    Some(Columns {
        mark,
        name: Some(name),
        qty,
    })
}

/// Katak **butunlay** sonmi.
fn number(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Katak o'lcham yoki miqdormi — marka emas.
///
/// Ikkinchi himoya qatlami: PDF da jadval qayta tiklanadi va ustunlar
/// ba'zan siljiydi, shuning uchun marka ustuniga miqdor tushib qolishi
/// mumkin.
pub fn is_measure(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    // `400x400`, `16х400х400`, `100*50` — o'lcham.
    let dimension = t.split(['x', 'х', '*']).filter(|p| !p.is_empty()).count() > 1
        && t.chars().any(|c| c.is_ascii_digit());
    if dimension {
        return true;
    }
    // Son bilan boshlanib o'lchov birligi bilan tugaydi: `159.9кг`, `5270шт`.
    //
    // Oxiridagi nuqta va vergul olib tashlanadi: jadvalda `159.9кг.` deb
    // yoziladi va u ham o'lcham.
    const UNITS: [&str; 16] = [
        "кг", "шт", "т", "мм", "см", "м2", "м3", "м", "kg", "dona", "sht", "mm", "cm", "m2", "m3",
        "л",
    ];
    let tail = t.trim_end_matches(['.', ',', ' ']);
    let starts_with_digit = tail.chars().next().is_some_and(|c| c.is_ascii_digit());
    starts_with_digit && UNITS.iter().any(|u| tail.ends_with(u))
}

/// O'qish natijasi.
#[derive(Debug, Default, Clone)]
pub struct Found {
    /// Jami ko'rilgan qatorlar.
    pub rows: usize,
    /// Topilgan spetsifikatsiya jadvallari soni.
    pub tables: usize,
    /// Jadval ostidagi, lekin markasi yo'q qatorlar.
    pub skipped: usize,
    pub elements: Vec<Element>,
}

/// Jadval qatorlaridan elementlarni ajratadi.
///
/// `sheet` — varaq nomi (odatda fayl nomi): element qaysi chizmadan
/// kelgani keyin ham ko'rinib tursin.
pub fn from_rows(rows: &[Vec<String>], project_id: i64, sheet: &str) -> Found {
    let mut out = Found {
        rows: rows.len(),
        ..Default::default()
    };
    let mut cols: Option<Columns> = None;
    for row in rows {
        if let Some(c) = header(row) {
            cols = Some(c);
            out.tables += 1;
            continue;
        }
        // Sarlavha topilmaguncha hech narsa o'qilmaydi.
        let Some(c) = cols else { continue };
        let Some(cell) = row.get(c.mark) else {
            out.skipped += 1;
            continue;
        };
        let mark = cell.trim().to_string();
        if mark.is_empty() || number(&mark).is_some() || is_measure(&mark) {
            out.skipped += 1;
            continue;
        }
        let name = c
            .name
            .and_then(|i| row.get(i))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let value = c
            .qty
            .and_then(|i| row.get(i))
            .and_then(|s| number(s))
            .unwrap_or(0.0);

        out.elements.push(Element {
            id: 0,
            project_id,
            // Bo'lim PDF dan aniqlanmaydi: uni odam qo'yadi. Taxmin
            // qilingan bo'lim butun tekshiruvni chalg'itardi.
            section: Section::None,
            kind: ElementKind::Other,
            mark,
            room: String::new(),
            axis: String::new(),
            level: String::new(),
            size: 0.0,
            unit: String::new(),
            value,
            value_name: String::new(),
            sheet: sheet.to_string(),
            note: name,
            pos: None,
            bbox: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| (*s).to_string()).collect()
    }

    /// Vaqtinchalik: haqiqiy fayl ustida o'lchov.
    #[test]
    fn real_file_probe() {
        let path = std::path::Path::new(
            r"C:\Users\alocomputers\Desktop\AL_QUDRA_МЧЖга_АЖРАТИЛГАН_5га_ЕР_МАЙДОНИДА_ГРАНУЛАНИ_КАЙТА_ИШЛАШ.pdf",
        );
        if !path.exists() {
            println!("PROBE: fayl yo'q");
            return;
        }
        let t0 = std::time::Instant::now();
        let rows = crate::pdfread::table(path).expect("matn");
        let read_ms = t0.elapsed().as_millis();
        let t1 = std::time::Instant::now();
        let found = from_rows(&rows, 1, "probe");
        let parse_ms = t1.elapsed().as_millis();
        println!(
            "PROBE: o'qish {read_ms} ms · tahlil {parse_ms} ms · qator {} · jadval {} · element {} · tashlangan {}",
            found.rows, found.tables, found.elements.len(), found.skipped
        );
        let mut sample = String::new();
        for e in found.elements.iter().take(25) {
            sample.push_str(&format!("{} | {} | {}\n", e.mark, e.note, e.value));
        }
        let _ = std::fs::write(
            r"C:\Users\alocomputers\AppData\Local\Temp\claude\E--QURAi\fc91bc95-1682-491a-8dd0-299985dfabdf\scratchpad\probe.txt",
            sample,
        );
    }

    #[test]
    fn a_header_needs_a_name_column_and_a_mark_or_position() {
        let h = header(&row(&["Поз.", "Обозначение", "Наименование", "Кол-во"])).expect("sarlavha");
        // Marka ustuni pozitsiyadan ustun turadi.
        assert_eq!(h.mark, 1);
        assert_eq!(h.name, Some(2));
        assert_eq!(h.qty, Some(3));

        // Markasiz, faqat pozitsiya — baribir sarlavha.
        let h = header(&row(&["Поз.", "Наименование", "Кол."])).expect("sarlavha");
        assert_eq!(h.mark, 0);

        // Nom ustuni yo'q — bu sarlavha emas.
        assert!(header(&row(&["Марка", "Кол-во"])).is_none());
        // Oddiy matn qatori — sarlavha emas.
        assert!(header(&row(&["Rigel", "300x600", "4"])).is_none());
        assert!(header(&[]).is_none());
    }

    /// O'lcham va miqdor marka bo'lib tushmaydi.
    ///
    /// Haqiqiy loyihadan olingan qiymatlar: ular avval marka deb
    /// o'qilgan va sakkiz yuzdan ortiq soxta element yasagan.
    #[test]
    fn measurements_are_not_marks() {
        for m in [
            "-16x400x400",
            "-8x56x110",
            "159.9кг.",
            "3.04кг",
            "5270шт",
            "80шт",
            "89.6кг.",
            "400х400",
            "100*50",
        ] {
            assert!(is_measure(m), "{m}");
        }
        for ok in [
            "B-1",
            "K1-2",
            "OK-12",
            "C-1",
            "R-1",
            "2К80-6М3-с-а",
            "Rigel",
        ] {
            assert!(!is_measure(ok), "{ok}");
        }
    }

    /// Sarlavhasiz qator umuman o'qilmaydi.
    #[test]
    fn nothing_is_read_without_a_header() {
        let rows = vec![
            row(&["Rigel 300x600", "4", "шт"]),
            row(&["B-1", "Ustun", "8"]),
        ];
        let found = from_rows(&rows, 1, "chizma");
        assert_eq!(found.rows, 2);
        assert_eq!(found.tables, 0);
        assert!(found.elements.is_empty(), "{:?}", found.elements);
    }

    #[test]
    fn rows_under_a_header_become_elements() {
        let rows = vec![
            row(&["Lист 1"]),
            row(&["Поз.", "Марка", "Наименование", "Кол-во"]),
            row(&["1", "B-1", "Ригель 300x600", "4"]),
            row(&["2", "K1-2", "Колонна", "8"]),
            // Markasi bo'sh — tashlanadi.
            row(&["3", "", "Без марки", "1"]),
            // Marka ustuniga miqdor tushib qolgan — tashlanadi.
            row(&["4", "159.9кг.", "Шов", "2"]),
        ];
        let found = from_rows(&rows, 7, "KJ-01");
        assert_eq!(found.tables, 1);
        assert_eq!(found.elements.len(), 2, "{:?}", found.elements);
        assert_eq!(found.skipped, 2);

        let a = &found.elements[0];
        assert_eq!(a.mark, "B-1");
        assert_eq!(a.note, "Ригель 300x600");
        assert_eq!(a.value, 4.0);
        assert_eq!(a.sheet, "KJ-01");
        assert_eq!(a.project_id, 7);
        // Bo'lim va joy taxmin qilinmaydi.
        assert_eq!(a.section, Section::None);
        assert!(a.pos.is_none());
        assert!(a.bbox.is_none());
    }

    /// Ikkinchi jadval sarlavhasi ustunlarni qaytadan belgilaydi.
    #[test]
    fn a_second_table_uses_its_own_columns() {
        let rows = vec![
            row(&["Поз.", "Марка", "Наименование", "Кол-во"]),
            row(&["1", "B-1", "Ригель", "4"]),
            // Boshqa jadval: ustunlar boshqa tartibda.
            row(&["Наименование", "Обозначение"]),
            row(&["Плита", "P-9"]),
        ];
        let found = from_rows(&rows, 1, "x");
        assert_eq!(found.tables, 2);
        assert_eq!(found.elements.len(), 2);
        assert_eq!(found.elements[0].mark, "B-1");
        assert_eq!(found.elements[1].mark, "P-9");
        assert_eq!(found.elements[1].note, "Плита");
    }
}
