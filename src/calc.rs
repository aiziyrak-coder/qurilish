//! Kalkulyatsiya varag'i: bo'limlar, sinflar va jamilar.
//!
//! Smeta sahifasi olti bosqichli yo'lga o'tgach bu varaq ekranga
//! ulanmagan; sinovlari bilan saqlanadi — material bo'yicha yig'ma
//! (Excel/PDF) kerak bo'lsa qayta ulanadi.
#![allow(dead_code)]
//!
//! Ekran, Excel va PDF **bitta ro'yxatdan** chiqadi — shu yerda tuziladi.
//! Aks holda ekranda bir jami, faylda boshqasi bo'lib qolishi mumkin edi.
//!
//! Tartib: tur (beton, armatura, prokat, boshqa) → sinf (masalan, silliq
//! armatura, davriy profilli armatura, sim) → material. Har sinf va har
//! tur o'z jami bilan tugaydi, oxirida umumiy jami turadi.
//!
//! Jami haqida qoida: narxsiz qator jamiga **nol bo'lib kirmaydi** — u
//! sanaladi va jami yonida «N qator narxsiz» deb yoziladi. Miqdor esa
//! faqat birlik bir xil bo'lganda qo'shiladi: kilogrammni donaga qo'shib
//! bo'lmaydi.

use crate::app::CostRow;
use crate::docgen::{Cell, Table};
use crate::i18n::t;
use crate::takeoff::Kind;

/// Turlar — varaqdagi tartibda.
pub const KINDS: [Kind; 4] = [Kind::Concrete, Kind::Rebar, Kind::Steel, Kind::Other];

/// Har tur ichidagi sinflar — varaqdagi tartibda. Qiymat — tarjima kaliti.
fn classes(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Concrete => &["cls_concrete"],
        Kind::Rebar => &["cls_rebar_smooth", "cls_rebar_ribbed", "cls_rebar_wire"],
        Kind::Steel => &[
            "cls_steel_sheet",
            "cls_steel_shape",
            "cls_steel_pipe",
            "cls_steel_parts",
        ],
        Kind::Other => &[
            "cls_other_masonry",
            "cls_other_roof",
            "cls_other_pipes",
            "cls_other_misc",
        ],
    }
}

/// Material qaysi sinfga tegishli.
pub fn class_of(kind: Kind, material: &str) -> &'static str {
    let low = material.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| low.contains(w));
    match kind {
        Kind::Concrete => "cls_concrete",
        Kind::Rebar => {
            // `Armatura Ø12 A-III` — sinf uchinchi so'z.
            let class = material.split_whitespace().nth(2).unwrap_or("");
            if class.starts_with(['B', 'В']) {
                "cls_rebar_wire"
            } else if class == "A-I" || class == "А-I" || class.ends_with("240") {
                "cls_rebar_smooth"
            } else {
                "cls_rebar_ribbed"
            }
        }
        Kind::Steel => {
            if has(&["quvur", "труба"]) {
                "cls_steel_pipe"
            } else if has(&["burchak", "shveller", "∠"]) {
                "cls_steel_shape"
            } else if has(&["po'lat list", "prokat зд-"]) {
                "cls_steel_sheet"
            } else {
                "cls_steel_parts"
            }
        }
        Kind::Other => {
            if has(&["кирпич", "засыпк", "шлакоблок"]) {
                "cls_other_masonry"
            } else if low.starts_with("сп-") || has(&["сэндвич", "водосборный", "кровел"])
            {
                "cls_other_roof"
            } else if has(&[
                "труб",
                "отвод",
                "тройник",
                "воронк",
                "ревизи",
                "патрубок",
                "лоток",
            ]) {
                "cls_other_pipes"
            } else {
                "cls_other_misc"
            }
        }
    }
}

/// Saralash kaliti: nomdagi birinchi son, keyin nomning o'zi.
///
/// Satr bo'yicha saralashda `B7.5` `B25` dan keyin, `Ø8` esa `Ø32` dan
/// keyin chiqardi.
fn order(material: &str) -> (f64, String) {
    let digits: String = material
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    (digits.parse().unwrap_or(f64::MAX), material.to_lowercase())
}

/// Bir guruh qatorning jami.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sum {
    /// Miqdor va birlik — faqat hamma qator bir xil birlikda bo'lsa.
    pub qty: Option<(f64, String)>,
    /// Summa — kamida bitta qator narxlangan bo'lsa.
    pub cost: Option<f64>,
    /// Narxsiz qatorlar soni: ular `cost` ga kirmagan.
    pub missing: usize,
}

fn sum_of(rows: &[&CostRow]) -> Sum {
    let unit = rows
        .first()
        .map(|r| r.total.unit.clone())
        .unwrap_or_default();
    let same = !rows.is_empty() && rows.iter().all(|r| r.total.unit == unit);
    let costs: Vec<f64> = rows.iter().filter_map(|r| r.sum()).collect();
    Sum {
        qty: same.then(|| (rows.iter().map(|r| r.total.amount).sum(), unit)),
        cost: (!costs.is_empty()).then(|| costs.iter().sum()),
        missing: rows.len() - costs.len(),
    }
}

/// Varaqning bitta qatori.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    /// Tur sarlavhasi: `BETON`.
    Section(Kind),
    /// Sinf sarlavhasi (tarjima kaliti).
    Class(&'static str),
    /// Material: tartib raqami va `cost_rows` dagi o'rni.
    Item(usize, usize),
    ClassTotal(&'static str, Sum),
    SectionTotal(Kind, Sum),
    Grand(Sum),
}

/// Varaqni tuzadi.
pub fn sheet(rows: &[CostRow]) -> Vec<Row> {
    let mut out = Vec::new();
    let mut no = 0;
    for kind in KINDS {
        let mut part: Vec<usize> = (0..rows.len())
            .filter(|i| rows[*i].total.kind == kind)
            .collect();
        if part.is_empty() {
            continue;
        }
        part.sort_by(|a, b| {
            let (ka, kb) = (
                order(&rows[*a].total.material),
                order(&rows[*b].total.material),
            );
            ka.0.total_cmp(&kb.0).then(ka.1.cmp(&kb.1))
        });
        out.push(Row::Section(kind));
        let used: Vec<&'static str> = classes(kind)
            .iter()
            .copied()
            .filter(|c| {
                part.iter()
                    .any(|i| class_of(kind, &rows[*i].total.material) == *c)
            })
            .collect();
        for class in &used {
            let own: Vec<usize> = part
                .iter()
                .copied()
                .filter(|i| class_of(kind, &rows[*i].total.material) == *class)
                .collect();
            // Bitta sinfli turda sinf sarlavhasi ortiqcha — tur nomining
            // o'zi yetadi.
            if used.len() > 1 {
                out.push(Row::Class(class));
            }
            for i in &own {
                no += 1;
                out.push(Row::Item(no, *i));
            }
            if used.len() > 1 {
                let refs: Vec<&CostRow> = own.iter().map(|i| &rows[*i]).collect();
                out.push(Row::ClassTotal(class, sum_of(&refs)));
            }
        }
        let refs: Vec<&CostRow> = part.iter().map(|i| &rows[*i]).collect();
        out.push(Row::SectionTotal(kind, sum_of(&refs)));
    }
    // Umumiy jamida miqdor yo'q: beton kubi bilan armatura tonnasi
    // qo'shilmaydi.
    let all: Vec<&CostRow> = rows.iter().collect();
    let mut grand = sum_of(&all);
    grand.qty = None;
    out.push(Row::Grand(grand));
    out
}

pub fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Concrete => t("tk_concrete"),
        Kind::Rebar => t("tk_rebar"),
        Kind::Steel => t("tk_steel"),
        Kind::Other => t("tk_other"),
    }
}

/// Narx qayerdan kelgani — so'z bilan.
pub fn source(r: &CostRow) -> String {
    if r.manual {
        t("tk_src_manual").to_string()
    } else if let Some(s) = r.orientir {
        format!("{} · {s}", t("tk_src_orientir"))
    } else if r.price.is_some() {
        t("tk_src_book").to_string()
    } else {
        t("tk_src_none").to_string()
    }
}

/// Narxsiz qatorlar haqida izoh — jami yonida.
fn missing_note(s: &Sum) -> Cell {
    if s.missing > 0 {
        Cell::Text(format!("{} {}", s.missing, t("tk_no_price")))
    } else {
        Cell::Empty
    }
}

/// Varaqni eksport jadvaliga aylantiradi (Excel va PDF uchun).
///
/// Miqdor loyihadagi birlikda va **son** bo'lib chiqadi: Excel'da darrov
/// qayta hisoblash mumkin.
pub fn table(rows: &[CostRow], name: &str) -> Table {
    let money = |v: Option<f64>| v.map(Cell::Money).unwrap_or(Cell::Empty);
    let total = |label: String, s: &Sum| -> Vec<Cell> {
        let (qty, unit) = match &s.qty {
            Some((q, u)) => (Cell::Num(*q), Cell::Text(u.clone())),
            None => (Cell::Empty, Cell::Empty),
        };
        vec![
            Cell::Empty,
            Cell::Text(label),
            unit,
            qty,
            Cell::Empty,
            money(s.cost),
            missing_note(s),
        ]
    };
    let title = |label: String| -> Vec<Cell> {
        let mut v = vec![Cell::Empty, Cell::Text(label)];
        v.resize(7, Cell::Empty);
        v
    };
    let body = sheet(rows)
        .into_iter()
        .map(|row| match row {
            Row::Section(k) => title(kind_name(k).to_uppercase()),
            Row::Class(c) => title(format!("— {}", t(c))),
            Row::Item(no, i) => {
                let r = &rows[i];
                vec![
                    Cell::Num(no as f64),
                    Cell::Text(r.total.material.clone()),
                    Cell::Text(r.total.unit.clone()),
                    Cell::Num(r.total.amount),
                    money(r.price),
                    money(r.sum()),
                    Cell::Text(source(r)),
                ]
            }
            Row::ClassTotal(c, s) => total(format!("{}: {}", t("tk_subtotal"), t(c)), &s),
            Row::SectionTotal(k, s) => total(
                format!("{} — {}", t("tk_subtotal"), kind_name(k)).to_uppercase(),
                &s,
            ),
            Row::Grand(s) => total(t("tk_grand_total").to_string(), &s),
        })
        .collect();
    Table {
        name: name.to_string(),
        headers: [
            "tk_col_no",
            "tk_col_material",
            "tk_col_unit",
            "tk_col_amount",
            "tk_col_price",
            "tk_col_sum",
            "tk_col_source",
        ]
        .iter()
        .map(|k| t(k).to_string())
        .collect(),
        rows: body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::takeoff::Total;

    fn row(kind: Kind, material: &str, unit: &str, amount: f64, price: Option<f64>) -> CostRow {
        CostRow {
            total: Total {
                kind,
                material: material.into(),
                unit: unit.into(),
                amount,
                lines: 1,
                unsure: 0,
            },
            price,
            manual: price.is_some(),
            orientir: None,
        }
    }

    fn sample() -> Vec<CostRow> {
        vec![
            row(Kind::Concrete, "Beton B25", "m3", 10.0, Some(700.0)),
            row(Kind::Concrete, "Beton B7.5", "m3", 5.0, Some(400.0)),
            row(Kind::Rebar, "Armatura Ø32 A-III", "kg", 100.0, Some(8.0)),
            row(Kind::Rebar, "Armatura Ø8 A-I", "kg", 50.0, None),
            row(Kind::Rebar, "Armatura Ø8 A-III", "kg", 20.0, Some(8.0)),
            row(Kind::Other, "Сп-1, L=11450x1000", "dona", 4.0, None),
            row(Kind::Other, "Объем кирпича 380мм", "m3", 3.0, Some(100.0)),
        ]
    }

    /// Sinflar tartibda, sonlar son bo'yicha saralanadi.
    #[test]
    fn rows_are_grouped_and_sorted_by_number() {
        let rows = sample();
        let names: Vec<String> = sheet(&rows)
            .iter()
            .filter_map(|r| match r {
                Row::Item(_, i) => Some(rows[*i].total.material.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            names,
            [
                "Beton B7.5",
                "Beton B25",
                "Armatura Ø8 A-I",
                "Armatura Ø8 A-III",
                "Armatura Ø32 A-III",
                "Объем кирпича 380мм",
                "Сп-1, L=11450x1000",
            ]
        );
        // Beton bitta sinf — sinf sarlavhasi chiqmaydi; armaturada chiqadi.
        let s = sheet(&rows);
        assert_eq!(s[0], Row::Section(Kind::Concrete));
        assert!(matches!(s[1], Row::Item(1, _)));
        assert!(s.contains(&Row::Class("cls_rebar_smooth")));
        assert!(s.contains(&Row::Class("cls_rebar_ribbed")));
    }

    /// Narxsiz qator jamiga nol bo'lib kirmaydi — u sanaladi.
    #[test]
    fn totals_count_unpriced_rows_instead_of_hiding_them() {
        let rows = sample();
        let s = sheet(&rows);
        let rebar = s
            .iter()
            .find_map(|r| match r {
                Row::SectionTotal(Kind::Rebar, s) => Some(s.clone()),
                _ => None,
            })
            .expect("armatura jami");
        assert_eq!(rebar.qty, Some((170.0, "kg".to_string())));
        assert_eq!(rebar.cost, Some(960.0));
        assert_eq!(rebar.missing, 1);
        // Boshqa bo'limda birliklar har xil — miqdor qo'shilmaydi.
        let other = s
            .iter()
            .find_map(|r| match r {
                Row::SectionTotal(Kind::Other, s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(other.qty, None);
        let Some(Row::Grand(g)) = s.last() else {
            panic!("jami yo'q")
        };
        assert_eq!(g.cost, Some(7000.0 + 2000.0 + 960.0 + 300.0));
        assert_eq!(g.missing, 2);
        assert_eq!(g.qty, None);
    }

    /// Ekrandagi varaq Excel va PDF ga bir xil chiqadi.
    #[test]
    fn the_sheet_is_written_to_excel_and_pdf() {
        let rows = sample();
        let table = table(&rows, "Kalkulyatsiya");
        assert_eq!(table.rows.len(), sheet(&rows).len());
        assert!(table.rows.iter().all(|r| r.len() == table.headers.len()));
        let dir = std::env::temp_dir();
        let xlsx = dir.join(format!("qurai_calc_{}.xlsx", std::process::id()));
        crate::docgen::write_table(&xlsx, &table).expect("xlsx");
        assert!(std::fs::metadata(&xlsx).unwrap().len() > 1000);
        let _ = std::fs::remove_file(&xlsx);
        // PDF tizim shriftini talab qiladi; shrift yo'q joyda u ochiq
        // xato beradi va bu sinovning ishi emas.
        if crate::pdf::font_bytes().is_some() {
            let pdf = dir.join(format!("qurai_calc_{}.pdf", std::process::id()));
            crate::pdf::write_table(&pdf, &table, "sinov").expect("pdf");
            assert!(std::fs::metadata(&pdf).unwrap().len() > 1000);
            let _ = std::fs::remove_file(&pdf);
        }
    }
}
