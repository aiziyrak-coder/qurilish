//! PDF loyihadan element spetsifikatsiyasini o'qish (TZ II.1).
//!
//! Haqiqiy ishda loyiha ko'pincha **PDF** bo'lib keladi: IFC va DXF
//! loyihachida bo'ladi, buyurtmachiga esa chop etishga tayyor PDF
//! beriladi. Shuning uchun PDF — chetdagi holat emas, asosiy yo'l.
//!
//! PDF da chizmaning o'zi vektor yoki rasm bo'lib turadi va undan element
//! chiqarib bo'lmaydi. Lekin loyihada deyarli har doim **spetsifikatsiya
//! jadvali** bo'ladi: marka, nomi, miqdori. Shu jadval o'qiladi.
//!
//! Qoida qat'iy: **marka bo'lmasa qator tashlanadi.** Marka — chizmaning
//! o'zida yozilgan belgi (`B-1`, `K1-2`, `OK-12`), uni o'ylab topib
//! bo'lmaydi. Nomi yoki o'lchami tanilmasa — bo'sh qoladi, taxmin
//! qilinmaydi.
//!
//! Chegara ochiq: **skan qilingan PDF o'qilmaydi** — unda matn yo'q, rasm
//! bor. Bunday faylda dastur taxmin qilmaydi, shunday deb aytadi.

use crate::domain::{Element, ElementKind};
use crate::model::Section;

/// O'qish natijasi.
#[derive(Debug, Default, Clone)]
pub struct Found {
    /// Jami ko'rilgan qatorlar.
    pub rows: usize,
    /// Marka topilgan va elementga aylangan qatorlar.
    pub elements: Vec<Element>,
}

/// Matn markaga o'xshaydimi.
///
/// Marka — harf va raqamdan iborat qisqa belgi: `B-1`, `K1-2`, `OK-12`,
/// `КЖ-3`. Shuning uchun uchta shart: qisqa, ichida ham harf ham raqam
/// bor, va bo'sh joy yo'q.
///
/// Bu qoida ataylab tor: xato marka yaratgandan ko'ra, qatorni tashlab
/// ketgan yaxshi — tashlangani ekranda sanaladi va ko'rinib turadi.
pub fn looks_like_mark(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.chars().count() > 16 || t.contains(' ') {
        return false;
    }
    let has_digit = t.chars().any(|c| c.is_ascii_digit());
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    if !has_digit || letters == 0 {
        return false;
    }
    // Faqat harf, raqam va ajratgich.
    if !t
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '/')
    {
        return false;
    }
    // Ajratgich yoki kamida ikkita harf. Shartsiz `m3`, `m2` kabi **o'lchov
    // birliklari** marka bo'lib tushardi va har jadvaldan soxta element
    // chiqarardi.
    let separator = t.contains('-') || t.contains('.') || t.contains('/');
    separator || letters >= 2
}

/// Katak **butunlay** sonmi.
///
/// Qat'iy: `Rigel 300x600` son emas, u nom. Avval matndan raqamlar sug'urib
/// olinardi va shunday nom `300600` degan songa aylanib ketardi.
fn number(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
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
    for row in rows {
        // Markasi bor birinchi ustun — qolganlari tavsif deb olinadi.
        let Some(at) = row.iter().position(|c| looks_like_mark(c)) else {
            continue;
        };
        let mark = row[at].trim().to_string();
        // Nomi: markadan keyingi, sondan iborat bo'lmagan birinchi
        // ma'noli matn.
        let name = row
            .iter()
            .skip(at + 1)
            .map(|c| c.trim())
            .find(|c| c.chars().count() > 2 && number(c).is_none())
            .unwrap_or("")
            .to_string();
        // Miqdor: markadan keyingi birinchi son.
        let value = row
            .iter()
            .skip(at + 1)
            .find_map(|c| number(c))
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

    #[test]
    fn a_mark_is_short_and_mixes_letters_with_digits() {
        for ok in ["B-1", "K1-2", "OK-12", "KJ-3", "1.4/B", "VK12"] {
            assert!(looks_like_mark(ok), "{ok}");
        }
        for bad in [
            "",
            "   ",
            "Beton",
            "12",
            "0.75",
            "Monolit karkas",
            "juda-uzun-belgi-bo-lib-ketdi",
        ] {
            assert!(!looks_like_mark(bad), "{bad}");
        }
    }

    /// Markasi yo'q qator tashlanadi — element o'ylab topilmaydi.
    #[test]
    fn a_row_without_a_mark_is_skipped() {
        let rows = vec![
            vec!["Nomi".into(), "Miqdor".into(), "Birlik".into()],
            vec!["Beton".into(), "12".into(), "m3".into()],
        ];
        let found = from_rows(&rows, 1, "chizma.pdf");
        assert_eq!(found.rows, 2);
        assert!(found.elements.is_empty(), "{:?}", found.elements);
    }

    #[test]
    fn a_specification_row_becomes_an_element() {
        let rows = vec![
            vec!["1".into(), "B-1".into(), "Rigel 300x600".into(), "4".into()],
            vec!["2".into(), "K1-2".into(), "Ustun".into(), "8".into()],
        ];
        let found = from_rows(&rows, 7, "KJ-01.pdf");
        assert_eq!(found.elements.len(), 2);

        let a = &found.elements[0];
        assert_eq!(a.mark, "B-1");
        assert_eq!(a.note, "Rigel 300x600");
        assert_eq!(a.value, 4.0);
        assert_eq!(a.sheet, "KJ-01.pdf");
        assert_eq!(a.project_id, 7);
        // Bo'lim taxmin qilinmaydi.
        assert_eq!(a.section, Section::None);
        // Joyi noma'lum — reja chizmasiga tushmaydi.
        assert!(a.pos.is_none());
        assert!(a.bbox.is_none());

        assert_eq!(found.elements[1].mark, "K1-2");
        assert_eq!(found.elements[1].value, 8.0);
    }

    /// Nomi topilmasa bo'sh qoladi — son nom sifatida olinmaydi.
    #[test]
    fn a_number_is_not_taken_as_a_name() {
        let rows = vec![vec!["B-7".into(), "12".into(), "25".into()]];
        let found = from_rows(&rows, 1, "x.pdf");
        assert_eq!(found.elements.len(), 1);
        assert_eq!(found.elements[0].note, "");
        assert_eq!(found.elements[0].value, 12.0);
    }
}
