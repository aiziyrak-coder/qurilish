//! Loyiha varaqlarini AI ga tayyorlash va javobini jadvalga aylantirish.
//!
//! So'rovlarning o'zi `smeta_ai` da; bu yerda varaq matni, bitta varaqli
//! PDF va spetsifikatsiya javobini o'qish turadi.
//!
//! Joylashuv qoidasi (`takeoff::tables`) matn parchalarining
//! koordinatasiga tayanadi va chizma dasturi jadvalni g'alati tergan joyda
//! adashadi. AI esa varaqni **ko'radi**: unga varaqning o'zi PDF holida va
//! dastur ajratib olgan matn beriladi, u spetsifikatsiya jadvallarini
//! qator-qator ko'chirib beradi.
//!
//! Ish taqsimoti qat'iy:
//!
//! - **AI o'qiydi** — jadvalni tuzilmaga soladi. Unga hisoblash
//!   taqiqlangan: sonlar varaqdagidek ko'chiriladi.
//! - **Dastur hisoblaydi** — ko'paytirish va yig'ish `takeoff` da, bitta
//!   joyda. AI o'qigan jadval ham, qoida o'qigani ham o'sha hisobdan
//!   o'tadi.
//! - **Ikkalasi solishtiriladi** — har varaq ikki mustaqil yo'l bilan
//!   o'qiladi; farq qilgan varaq odamga ko'rsatiladi.
//!
//! Bu faqat foydalanuvchi tugmani bosganda ishlaydi va varaqlarni tashqi
//! xizmatga yuboradi — ekranda shunday deb yozilgan. Kalit faqat
//! `Authorization` sarlavhasida ketadi.

use crate::pdfread::Piece;
use crate::takeoff::{SpecRow, SpecTable};

/// Modelga beriladigan varaq matni chegarasi, belgida.
// Varaq matni chegarasi: ortig'i — chizma o'lchamlari, ular jadvalga
// hech narsa bermaydi, token esa ketadi.
const MAX_TEXT: usize = 9_000;
/// Varaq matnini qatorlarga teradi: yuqoridan pastga, kataklar ` | `
/// bilan ajratilgan.
///
/// Model PDF ni o'zi ko'radi; bu matn — sonlarni aniq ko'chirishi uchun
/// tayanch (rasmdan o'qilgan raqam adashishi mumkin, matn qatlami — yo'q).
pub fn page_text(pieces: &[Piece]) -> String {
    let mut out = String::new();
    for dir in 0..4u8 {
        let mut part: Vec<&Piece> = pieces.iter().filter(|p| p.dir == dir).collect();
        if part.is_empty() {
            continue;
        }
        part.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));
        let mut lines: Vec<Vec<&Piece>> = Vec::new();
        for p in part {
            match lines.last_mut() {
                Some(l) if (l[0].y - p.y).abs() <= p.size * 0.5 => l.push(p),
                _ => lines.push(vec![p]),
            }
        }
        for mut l in lines {
            l.sort_by(|a, b| a.x.total_cmp(&b.x));
            let mut end = f64::MIN;
            for p in l {
                let gap = p.x - end;
                if end != f64::MIN {
                    if gap > p.size * 1.5 {
                        out.push_str(" | ");
                    } else if gap > p.size * 0.33 {
                        out.push(' ');
                    }
                }
                out.push_str(p.text.trim());
                end = p.x + p.text.chars().count() as f64 * p.size * 0.5;
            }
            out.push('\n');
            if out.len() > MAX_TEXT {
                // Belgi chegarasida kesiladi — UTF-8 buzilmasin.
                let cut: String = out.chars().take(MAX_TEXT).collect();
                return cut;
            }
        }
    }
    out
}

/// Bitta varaqdan iborat PDF.
pub fn page_pdf(doc: &lopdf::Document, page: u32) -> Option<Vec<u8>> {
    let mut one = doc.clone();
    let others: Vec<u32> = one
        .get_pages()
        .keys()
        .copied()
        .filter(|n| *n != page)
        .collect();
    one.delete_pages(&others);
    one.prune_objects();
    let mut out = Vec::new();
    one.save_to(&mut out).ok()?;
    (!out.is_empty()).then_some(out)
}

/// JSON qiymatini matnga: model sonni son qilib ham, satr qilib ham
/// berishi mumkin.
fn cell(v: Option<&serde_json::Value>) -> String {
    match v {
        Some(serde_json::Value::String(s)) => s.trim().to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Model javobini jadvallarga aylantiradi: `(ega, jadvallar)`.
///
/// Javob JSON bo'lmasa yoki kutilgan shaklda bo'lmasa — xato; taxmin
/// qilib tuzatilmaydi.
pub fn parse(body: &str, page: usize) -> Result<(String, Vec<SpecTable>), String> {
    // Ba'zi modellar JSON ni ``` ichiga o'raydi.
    let text = body.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text).trim();
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("JSON: {e}"))?;
    let tables = v
        .get("tables")
        .and_then(|t| t.as_array())
        .ok_or_else(|| "JSON: tables yo'q".to_string())?;
    // Ega — belgining o'zi: `К3(2К80-6М3-с-а)` → `К3`.
    let owner = cell(v.get("owner"))
        .split(['(', ' ', ','])
        .next()
        .unwrap_or_default()
        .to_string();
    let mut out = Vec::new();
    for t in tables {
        let mut rows = Vec::new();
        // Jadval nomi — sarlavha qatori: unda konstruksiya belgisi
        // bo'lishi mumkin («Спецификация на Фм1»).
        let title = cell(t.get("title"));
        if !title.is_empty() {
            rows.push(SpecRow {
                name: title,
                ..Default::default()
            });
        }
        for r in t
            .get("rows")
            .and_then(|r| r.as_array())
            .into_iter()
            .flatten()
        {
            let total = cell(r.get("total"));
            let unit = cell(r.get("unit"));
            let row = SpecRow {
                pos: cell(r.get("pos")),
                designation: cell(r.get("designation")),
                name: cell(r.get("name"))
                    .replace('∅', "Ø")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
                qty: cell(r.get("qty")),
                mass: cell(r.get("mass")),
                // Hisob «Примеч.» ustunidan jami massani yoki birlikni
                // kutadi: `68.0`, `шт.`, `294 п.м.`.
                note: format!("{total} {unit}").trim().to_string(),
                unit,
            };
            if row != SpecRow::default() {
                rows.push(row);
            }
        }
        // Po'lat sarfi vedomosti — shu varaqdagi spetsifikatsiyaning
        // yig'indisi. Model uni ko'rsatmaga qaramay bersa ham, u hisobga
        // kirmaydi: aks holda armatura ikki marta sanalardi. Belgisi —
        // «metall tanlovi» ichida armatura diametri.
        let summary = rows
            .iter()
            .any(|r| r.designation == crate::takeoff::KMD && r.name.trim_start().starts_with('Ø'));
        if summary {
            continue;
        }
        // Faqat sarlavhadan iborat jadval — bo'sh.
        if rows
            .iter()
            .any(|r| !r.qty.is_empty() || !r.mass.is_empty() || !r.note.is_empty())
        {
            out.push(SpecTable {
                page,
                rows,
                ai: true,
                ..Default::default()
            });
        }
    }
    Ok((owner, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, text: &str) -> Piece {
        Piece {
            x,
            y,
            size: 10.0,
            text: text.to_string(),
            dir: 0,
        }
    }

    #[test]
    fn the_sheet_text_keeps_rows_and_cells() {
        let pieces = vec![
            p(200.0, 100.0, "Наименование"),
            p(100.0, 100.0, "Поз."),
            p(104.0, 80.0, "Фм"),
            p(114.0, 80.0, "1"),
            p(200.0, 80.0, "Фундамент монолитный"),
            p(400.0, 80.0, "20"),
        ];
        assert_eq!(
            page_text(&pieces),
            "Поз. | Наименование\nФм1 | Фундамент монолитный | 20\n"
        );
    }

    /// Model javobi hisob kutgan shaklga keladi va o'sha hisobdan o'tadi.
    #[test]
    fn a_reply_becomes_tables_the_engine_can_count() {
        let body = r#"```json
{"sheet":"Фм3","owner":"Фм3","tables":[
 {"title":"Спецификация","rows":[
   {"pos":"","designation":"","name":"Фундамент Фм3 (на 1 шт.)","qty":"","mass":"","total":"","unit":""},
   {"pos":"1","designation":"ГОСТ 5781-82","name":"∅14 A-III L=2550","qty":22,"mass":3.09,"total":"68.0","unit":""},
   {"pos":"","designation":"","name":"Бетон кл. В20","qty":"3.66","mass":"","total":"","unit":"м3"}
 ]},
 {"title":"Пустая","rows":[]}
]}
```"#;
        let (owner, tables) = parse(body, 13).expect("jadval");
        assert_eq!(owner, "Фм3");
        assert_eq!(tables.len(), 1, "bo'sh jadval tashlanadi");
        let t = &tables[0];
        assert!(t.ai && t.page == 13);
        assert_eq!(t.rows[0].name, "Спецификация");
        assert_eq!(t.rows[2].qty, "22");
        assert_eq!(t.rows[2].note, "68.0");
        let rebar = crate::takeoff::item(&t.rows[2]).expect("armatura");
        assert_eq!(rebar.material, "Armatura Ø14 A-III");
        assert_eq!(rebar.amount, 68.0);
        let beton = crate::takeoff::item(&t.rows[3]).expect("beton");
        assert_eq!((beton.material.as_str(), beton.amount), ("Beton B20", 3.66));
    }

    /// Po'lat sarfi vedomosti ikkinchi marta sanalmaydi; ega — belgi.
    #[test]
    fn a_steel_summary_is_dropped_and_the_owner_is_a_mark() {
        let body = r#"{"owner":"К3(2К80-6М3-c-а)","tables":[
 {"title":"Ведомость расхода стали","rows":[
   {"designation":"КМД","name":"Ø32 A-III ГОСТ 5781-82","mass":"375.6","unit":"кг"}]},
 {"title":"Выборка металла","rows":[
   {"designation":"КМД","name":"□80х80х4 С255","mass":"3856,8","unit":"кг"}]},
 {"title":"","rows":[
   {"pos":"10","name":"Лист 12x390x680\nС235","qty":"1","mass":"25","total":"25.0","unit":"кг"}]}
]}"#;
        let (owner, tables) = parse(body, 19).unwrap();
        assert_eq!(owner, "К3");
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].rows[1].designation, crate::takeoff::KMD);
        assert_eq!(tables[1].rows[0].name, "Лист 12x390x680 С235");
    }

    /// Buzuq javob taxmin qilib tuzatilmaydi.
    #[test]
    fn a_broken_reply_is_an_error_not_a_guess() {
        assert!(parse("jadval topilmadi", 1).is_err());
        assert!(parse(r#"{"sheet":"x"}"#, 1).is_err());
        assert_eq!(parse(r#"{"tables":[]}"#, 1).unwrap().1.len(), 0);
    }
}
