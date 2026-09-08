//! Smeta importi (TZ III.2): XLSX, XLS, ODS va CSV.
//!
//! Sметный dastur eksporti bir xil emas: sarlavha qatori faylning boshida
//! bo'lmasligi, ustunlar tartibi va nomlari har xil bo'lishi mumkin. Shuning
//! uchun parser ustunlarni nom bo'yicha topadi va topolmagan holatda qatorni
//! tashlab yubormay, xatoni ochiq aytadi.

use crate::domain::EstimateItem;
use crate::model::Section;
use std::path::Path;

/// Import natijasi: topilgan pozitsiyalar va foydalanuvchiga aytiladigan izoh.
pub struct Imported {
    pub items: Vec<EstimateItem>,
    /// Hujjatda ko'rsatilgan yakuniy summa, topilgan bo'lsa.
    pub declared_total: f64,
    /// Faylning nomi — smeta nomi sifatida ishlatiladi.
    pub name: String,
    /// O'qib bo'lmagan qatorlar soni.
    pub skipped: usize,
}

/// Bitta katak: matn yoki son.
#[derive(Debug, Clone, Default)]
struct Cell {
    text: String,
    num: Option<f64>,
}

impl Cell {
    fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.num.is_none()
    }

    /// Sonni matndan ham ajratib oladi: «1 250,50», «1 250.50», «1250».
    fn number(&self) -> Option<f64> {
        if let Some(n) = self.num {
            return Some(n);
        }
        parse_number(&self.text)
    }
}

/// Smeta fayllarida sonlar probel, uzilmas probel yoki apostrof bilan
/// ajratiladi, kasr qismi esa vergul bo'lishi mumkin.
fn parse_number(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{00A0}' | '\u{202F}' | '\'' | '`'))
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    // «1,234.56» — vergul razryad ajratkichi; «1234,56» — kasr belgisi.
    let normalized = if cleaned.contains('.') && cleaned.contains(',') {
        cleaned.replace(',', "")
    } else {
        cleaned.replace(',', ".")
    };
    normalized.parse::<f64>().ok()
}

/// Ustun nomlarini solishtirish uchun: kichik harf, faqat harf, raqam va nomer belgisi.
fn key(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '\u{2116}')
        .collect()
}

/// Smeta ustunlari. Har biri ikki tilda va bir necha shaklda kelishi mumkin.
#[derive(Debug, Default, Clone, Copy)]
struct Columns {
    pos: Option<usize>,
    code: Option<usize>,
    name: Option<usize>,
    unit: Option<usize>,
    qty: Option<usize>,
    price: Option<usize>,
    cost: Option<usize>,
    section: Option<usize>,
}

impl Columns {
    /// Import ma'noli bo'lishi uchun kamida nom va ikkita son ustuni kerak.
    fn usable(&self) -> bool {
        self.name.is_some() && self.qty.is_some() && (self.price.is_some() || self.cost.is_some())
    }
}

/// Sarlavha qatoridan ustunlarni aniqlaydi.
fn detect_columns(row: &[Cell]) -> Columns {
    let mut c = Columns::default();
    for (i, cell) in row.iter().enumerate() {
        let k = key(&cell.text);
        if k.is_empty() {
            continue;
        }
        let hit = |variants: &[&str]| variants.iter().any(|v| k == *v || k.starts_with(v));

        // Qisqa "n" varianti bo'lmasligi kerak: u "narx" va "nomi" ga ham tushardi.
        if c.pos.is_none() && hit(&["№", "nomer", "номер", "поз", "pozitsiya", "пп"]) {
            c.pos = Some(i);
        } else if c.code.is_none()
            && hit(&[
                "шифр",
                "код",
                "kod",
                "shifr",
                "расценка",
                "rasenka",
                "обоснование",
            ])
        {
            c.code = Some(i);
        } else if c.name.is_none()
            && hit(&[
                "наименование",
                "название",
                "работа",
                "nomi",
                "nomlanishi",
                "ish",
                "описание",
            ])
        {
            c.name = Some(i);
        } else if c.unit.is_none()
            && hit(&["ед", "единица", "birlik", "olchov", "измерения", "изм"])
        {
            c.unit = Some(i);
        } else if c.qty.is_none()
            && hit(&[
                "количество",
                "колво",
                "кол",
                "miqdor",
                "hajm",
                "объем",
                "obyem",
            ])
        {
            c.qty = Some(i);
        } else if c.price.is_none() && hit(&["цена", "narx", "расценкаед", "стоимостьед"])
        {
            c.price = Some(i);
        } else if c.cost.is_none()
            && hit(&["стоимость", "сумма", "summa", "всего", "итого", "qiymat"])
        {
            c.cost = Some(i);
        } else if c.section.is_none() && hit(&["раздел", "bolim", "bo'lim", "section"]) {
            c.section = Some(i);
        }
    }
    c
}

/// Yakuniy summa qatorini aniqlaydi: «Итого», «Всего по смете», «Jami».
fn is_total_row(row: &[Cell], name_col: Option<usize>) -> bool {
    let text = match name_col.and_then(|i| row.get(i)) {
        Some(c) if !c.text.trim().is_empty() => c.text.clone(),
        // Yakun ba'zan birinchi to'ldirilgan katakda yoziladi.
        _ => row
            .iter()
            .find(|c| !c.text.trim().is_empty())
            .map(|c| c.text.clone())
            .unwrap_or_default(),
    };
    let k = key(&text);
    [
        "итого",
        "всего",
        "jami",
        "yakun",
        "итогопосмете",
        "всегопосмете",
    ]
    .iter()
    .any(|v| k.starts_with(v))
}

/// Jadvalni pozitsiyalarga aylantiradi. `estimate_id` keyin to'ldiriladi.
fn rows_to_items(rows: &[Vec<Cell>], name: String) -> Result<Imported, String> {
    // Sarlavha qatorini qidiramiz: birinchi 40 qatorda ustunlar topilishi kerak.
    let mut header = None;
    for (i, row) in rows.iter().take(40).enumerate() {
        let c = detect_columns(row);
        if c.usable() {
            header = Some((i, c));
            break;
        }
    }
    let Some((header_row, cols)) = header else {
        return Err(crate::i18n::t("import_no_header").to_string());
    };

    let mut items = Vec::new();
    let mut declared_total = 0.0;
    let mut skipped = 0;
    let mut pos = 0;

    for row in rows.iter().skip(header_row + 1) {
        if row.iter().all(|c| c.is_empty()) {
            continue;
        }
        let cell = |idx: Option<usize>| -> Cell {
            idx.and_then(|i| row.get(i)).cloned().unwrap_or_default()
        };

        if is_total_row(row, cols.name) {
            // Yakun qatoridagi eng katta son — smetaning umumiy summasi.
            let best = row.iter().filter_map(|c| c.number()).fold(0.0, f64::max);
            if best > declared_total {
                declared_total = best;
            }
            continue;
        }

        let name_cell = cell(cols.name);
        let title = name_cell.text.trim().to_string();
        if title.is_empty() {
            continue;
        }

        let qty = cell(cols.qty).number();
        let price = cell(cols.price).number();
        let cost = cell(cols.cost).number();

        // Bo'lim sarlavhasi: nomi bor, sonlari yo'q. Bu xato emas, shunchaki
        // pozitsiya emas — sanamaymiz.
        if qty.is_none() && price.is_none() && cost.is_none() {
            continue;
        }
        let Some(qty) = qty else {
            skipped += 1;
            continue;
        };

        let price = price.unwrap_or_else(|| {
            // Narx ustuni yo'q bo'lsa, summadan chiqaramiz.
            match cost {
                Some(c) if qty.abs() > 1e-9 => c / qty,
                _ => 0.0,
            }
        });
        // Summa ustuni yo'q bo'lsa, uni o'ylab topmaymiz — hisoblab qo'yamiz,
        // shunda arifmetika tekshiruvi noto'g'ri xato bermaydi.
        let cost = cost.unwrap_or(qty * price);

        pos += 1;
        items.push(EstimateItem {
            id: 0,
            estimate_id: 0,
            pos: cell(cols.pos)
                .number()
                .map(|v| v as i64)
                .filter(|v| *v > 0)
                .unwrap_or(pos),
            section: Section::parse(cell(cols.section).text.trim()),
            code: cell(cols.code).text.trim().to_string(),
            name: title,
            unit: cell(cols.unit).text.trim().to_string(),
            qty,
            price,
            cost,
            task_id: None,
            note: String::new(),
        });
    }

    if items.is_empty() {
        return Err(crate::i18n::t("import_no_items").to_string());
    }
    Ok(Imported {
        items,
        declared_total,
        name,
        skipped,
    })
}

/// Fayl kengaytmasiga qarab kerakli parserni chaqiradi.
pub fn estimate_from_file(path: &Path) -> Result<Imported, String> {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| crate::i18n::t("estimate_new_name").to_string());

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let rows = match ext.as_str() {
        "csv" | "txt" => read_csv(path)?,
        "xlsx" | "xlsm" | "xls" | "xlsb" | "ods" => read_spreadsheet(path)?,
        "pdf" => read_pdf(path)?,
        _ => return Err(crate::i18n::t("import_bad_format").to_string()),
    };
    rows_to_items(&rows, name)
}

/// Narx ro'yxati importi (TZ III.15).
///
/// Smeta importidan farqi: bu yerda **miqdor kerak emas** — prays-listda
/// faqat nom, birlik va narx bo'ladi. Shu sababli sarlavhani topish
/// sharti ham boshqacha, lekin ustunlarni tanish bir xil kod bilan
/// bajariladi: ikkita mustaqil tanish bir kun kelib ajralib qolardi.
pub fn prices_from_file(path: &Path) -> Result<Vec<crate::prices::PriceRow>, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let rows = match ext.as_str() {
        "csv" | "txt" => read_csv(path)?,
        "xlsx" | "xlsm" | "xls" | "xlsb" | "ods" => read_spreadsheet(path)?,
        _ => return Err(crate::i18n::t("import_bad_format").to_string()),
    };
    let source = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    rows_to_prices(&rows, &source)
}

/// Prays-listda uchraydigan qo'shimcha ustunlar.
#[derive(Default, Clone, Copy)]
struct ExtraColumns {
    source: Option<usize>,
    date: Option<usize>,
    region: Option<usize>,
}

fn detect_extra(row: &[Cell]) -> ExtraColumns {
    let mut c = ExtraColumns::default();
    for (i, cell) in row.iter().enumerate() {
        let k = key(&cell.text);
        if k.is_empty() {
            continue;
        }
        let hit = |variants: &[&str]| variants.iter().any(|v| k == *v || k.starts_with(v));
        if c.source.is_none()
            && hit(&[
                "manba",
                "источник",
                "поставщик",
                "yetkazib",
                "taminotchi",
                "supplier",
            ])
        {
            c.source = Some(i);
        } else if c.date.is_none() && hit(&["sana", "дата", "date"]) {
            c.date = Some(i);
        } else if c.region.is_none() && hit(&["hudud", "регион", "область", "viloyat", "region"])
        {
            c.region = Some(i);
        }
    }
    c
}

/// Qatorlardan narx ro'yxatini yig'adi.
fn rows_to_prices(
    rows: &[Vec<Cell>],
    file_source: &str,
) -> Result<Vec<crate::prices::PriceRow>, String> {
    // Sarlavha: nom va narx ustunlari bo'lishi yetarli.
    let mut header = None;
    for (i, row) in rows.iter().take(40).enumerate() {
        let c = detect_columns(row);
        if c.name.is_some() && c.price.is_some() {
            header = Some((i, c, detect_extra(row)));
            break;
        }
    }
    let Some((header_row, cols, extra)) = header else {
        return Err(crate::i18n::t("import_no_header").to_string());
    };

    let mut out = Vec::new();
    for row in rows.iter().skip(header_row + 1) {
        if row.iter().all(|c| c.is_empty()) {
            continue;
        }
        let cell = |idx: Option<usize>| -> Cell {
            idx.and_then(|i| row.get(i)).cloned().unwrap_or_default()
        };
        let name = cell(cols.name).text.trim().to_string();
        let price = cell(cols.price).number().unwrap_or(0.0);
        // Nomsiz yoki narxsiz qator prays emas: u sarlavha, izoh yoki
        // yakuniy qator bo'lishi mumkin va u jimgina tashlanadi.
        if name.is_empty() || price <= 0.0 {
            continue;
        }
        let source = {
            let s = cell(extra.source).text.trim().to_string();
            if s.is_empty() {
                file_source.to_string()
            } else {
                s
            }
        };
        out.push(crate::prices::PriceRow {
            id: 0,
            code: cell(cols.code).text.trim().to_string(),
            name,
            unit: cell(cols.unit).text.trim().to_string(),
            price,
            source,
            date: parse_any_date(&cell(extra.date).text),
            region: cell(extra.region).text.trim().to_string(),
        });
    }

    if out.is_empty() {
        return Err(crate::i18n::t("import_no_items").to_string());
    }
    Ok(out)
}

/// Prays-listdagi sanani o'qiydi.
///
/// Fayllarda sana turli ko'rinishda yoziladi; tanilmagani **bo'sh**
/// qoladi — noto'g'ri sana qo'yishdan ko'ra sanasiz qator yaxshiroq.
fn parse_any_date(s: &str) -> Option<chrono::NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    for fmt in ["%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y", "%d-%m-%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

/// PDF dan jadval o'qiydi (TZ III.2).
///
/// PDF da jadval tuzilmasi saqlanmaydi — u matn parchalarining
/// joylashuvidan tiklanadi (`pdfread`). Shuning uchun natija taxminiy
/// bo'lishi mumkin: ustunlar noto'g'ri ajralsa, sarlavha topilmaydi va
/// import ochiq xato beradi — jim turib noto'g'ri son qo'ymaydi.
fn read_pdf(path: &Path) -> Result<Vec<Vec<Cell>>, String> {
    let rows: Vec<Vec<Cell>> = match crate::pdfread::table(path) {
        Ok(rows) => rows
            .into_iter()
            .map(|r| r.into_iter().map(|text| Cell { text, num: None }).collect())
            .collect(),
        // Matni yo'q PDF — bu skan. Kompyuterda matnni tanish dasturi
        // bo'lsa, undan foydalanamiz; bo'lmasa xato o'zgarmasdan qaytadi.
        Err(e) => {
            if crate::ocr::tesseract().is_some() && crate::ocr::pdf_to_image().is_some() {
                let text = crate::ocr::pdf_text(path)?;
                text_rows(&text)
            } else {
                return Err(e);
            }
        }
    };
    Ok(rows)
}

/// Tanilgan matnni qatorlarga bo'ladi.
///
/// OCR jadval chegaralarini bilmaydi; u faqat matn beradi. Ustunlar
/// **ikki va undan ortiq bo'shliq** bo'yicha ajratiladi — bu tanilgan
/// jadvalda ustunlar orasidagi odatdagi bo'shliq.
fn text_rows(text: &str) -> Vec<Vec<Cell>> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            line.split("  ")
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .map(|c| Cell {
                    text: c.to_string(),
                    num: None,
                })
                .collect()
        })
        .collect()
}

fn read_spreadsheet(path: &Path) -> Result<Vec<Vec<Cell>>, String> {
    use calamine::{Data, Reader};

    let mut wb = calamine::open_workbook_auto(path).map_err(|e| e.to_string())?;
    let sheets = wb.sheet_names().to_vec();
    if sheets.is_empty() {
        return Err(crate::i18n::t("import_no_sheets").to_string());
    }

    // Bir nechta varaq bo'lsa, ustunlari topiladigan birinchisini olamiz.
    let mut fallback: Option<Vec<Vec<Cell>>> = None;
    for sheet in sheets {
        let Ok(range) = wb.worksheet_range(&sheet) else {
            continue;
        };
        let rows: Vec<Vec<Cell>> = range
            .rows()
            .map(|r| {
                r.iter()
                    .map(|d| match d {
                        Data::Empty => Cell::default(),
                        Data::Int(i) => Cell {
                            text: i.to_string(),
                            num: Some(*i as f64),
                        },
                        Data::Float(f) => Cell {
                            text: f.to_string(),
                            num: Some(*f),
                        },
                        Data::String(s) => Cell {
                            text: s.clone(),
                            num: None,
                        },
                        Data::Bool(b) => Cell {
                            text: b.to_string(),
                            num: None,
                        },
                        other => Cell {
                            text: other.to_string(),
                            num: None,
                        },
                    })
                    .collect()
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        if rows.iter().take(40).any(|r| detect_columns(r).usable()) {
            return Ok(rows);
        }
        if fallback.is_none() {
            fallback = Some(rows);
        }
    }
    fallback.ok_or_else(|| crate::i18n::t("import_no_sheets").to_string())
}

fn read_csv(path: &Path) -> Result<Vec<Vec<Cell>>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Smeta eksporti ko'pincha UTF-8 BOM bilan keladi.
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);

    // Ajratkich: nuqtali vergul (Yevropa Excel) yoki vergul.
    let semi = text.matches(';').count();
    let comma = text.matches(',').count();
    let sep = if semi >= comma { ';' } else { ',' };

    Ok(text
        .lines()
        .map(|line| {
            split_csv(line, sep)
                .into_iter()
                .map(|t| Cell { text: t, num: None })
                .collect()
        })
        .collect())
}

/// Qo'shtirnoq ichidagi ajratkichni hisobga oladigan oddiy CSV bo'luvchi.
fn split_csv(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == sep && !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TZ III.15: prays-list miqdorsiz ham o'qiladi.
    ///
    /// Smeta importidan farqi shu: prays-listda «miqdor» ustuni
    /// bo'lmaydi va uni talab qilish ro'yxatni o'qib bo'lmaydigan qilib
    /// qo'yardi.
    #[test]
    fn a_price_list_is_read_without_a_quantity_column() {
        let rows = vec![
            cells(&["Ta'minotchi prays-listi", "", "", "", ""]),
            cells(&["Kod", "Nomi", "Birlik", "Narx", "Manba"]),
            cells(&["C-400", "Sement M400", "kg", "1 200", "Qizilqum"]),
            cells(&["", "G'isht qizil M150", "dona", "950,50", ""]),
            // Narxsiz qator tashlanadi: u izoh yoki yakuniy qator.
            cells(&["", "Jami", "", "", ""]),
        ];
        let out = rows_to_prices(&rows, "prays-2026").expect("o'qildi");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].code, "C-400");
        assert_eq!(out[0].name, "Sement M400");
        assert_eq!(out[0].price, 1200.0);
        assert_eq!(out[0].source, "Qizilqum");
        // Manba ustuni bo'sh bo'lsa fayl nomi ishlatiladi.
        assert_eq!(out[1].source, "prays-2026");
        assert!((out[1].price - 950.5).abs() < 1e-9);
    }

    /// Sarlavha topilmasa import jim turmaydi.
    #[test]
    fn a_file_without_a_price_column_is_refused() {
        let rows = vec![cells(&["Nomi", "Izoh"]), cells(&["Sement", "yaxshi"])];
        assert!(rows_to_prices(&rows, "x").is_err());
        // Sarlavha bor, lekin birorta ham qator narxsiz.
        let rows = vec![cells(&["Nomi", "Narx"]), cells(&["Sement", ""])];
        assert!(rows_to_prices(&rows, "x").is_err());
    }

    /// Sana turli ko'rinishda yozilishi mumkin; tanilmagani bo'sh qoladi.
    #[test]
    fn a_date_is_read_or_left_empty() {
        assert_eq!(
            parse_any_date("2026-09-08").map(|d| d.to_string()),
            Some("2026-09-08".into())
        );
        assert_eq!(
            parse_any_date("08.09.2026").map(|d| d.to_string()),
            Some("2026-09-08".into())
        );
        assert!(parse_any_date("sentabr").is_none());
        assert!(parse_any_date("").is_none());
    }

    fn cells(row: &[&str]) -> Vec<Cell> {
        row.iter()
            .map(|s| Cell {
                text: s.to_string(),
                num: None,
            })
            .collect()
    }

    /// TZ III.2: PDF dagi smeta jadvali o'qiladi.
    ///
    /// Sinov haqiqiy PDF yozadi va uni qaytadan o'qiydi: ustunlar
    /// joylashuvdan tiklanadi, sonlar son bo'lib qoladi.
    #[test]
    fn estimate_is_read_from_a_pdf() {
        use crate::docgen::{Cell as DocCell, Table};

        let path = std::env::temp_dir().join(format!("qurai_imp_{}.pdf", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let table = Table {
            name: "Smeta".into(),
            headers: vec![
                "Nomi".into(),
                "Birlik".into(),
                "Miqdori".into(),
                "Narxi".into(),
                "Summa".into(),
            ],
            rows: vec![
                vec![
                    DocCell::Text("Beton quyish".into()),
                    DocCell::Text("m3".into()),
                    DocCell::Num(10.0),
                    DocCell::Num(500.0),
                    DocCell::Num(5000.0),
                ],
                vec![
                    DocCell::Text("Armatura montaji".into()),
                    DocCell::Text("t".into()),
                    DocCell::Num(2.0),
                    DocCell::Num(1000.0),
                    DocCell::Num(2000.0),
                ],
            ],
        };
        // Shrift topilmasa PDF yozib bo'lmaydi — bu muhit masalasi.
        if crate::pdf::write_table(&path, &table, "").is_err() {
            return;
        }

        let out = estimate_from_file(&path).expect("import");
        assert_eq!(out.items.len(), 2, "{:?}", out.items);
        let first = &out.items[0];
        assert_eq!(first.name, "Beton quyish");
        assert_eq!(first.unit, "m3");
        assert_eq!(first.qty, 10.0);
        assert_eq!(first.price, 500.0);
        assert_eq!(first.cost, 5000.0);
        let _ = std::fs::remove_file(&path);
    }

    /// Matni yo'q PDF da taxmin qilinmaydi — sabab aytiladi.
    #[test]
    fn pdf_without_text_reports_the_reason() {
        let out = estimate_from_file(std::path::Path::new("yo-q.pdf"));
        assert!(out.is_err());
        assert!(!out.err().unwrap_or_default().is_empty());
    }

    #[test]
    fn numbers_survive_smeta_formatting() {
        assert_eq!(parse_number("1 250,50"), Some(1250.5));
        assert_eq!(parse_number("1\u{00A0}250"), Some(1250.0));
        assert_eq!(parse_number("1,234.56"), Some(1234.56));
        assert_eq!(parse_number("450000"), Some(450000.0));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("м3"), None);
    }

    #[test]
    fn columns_are_found_in_both_languages() {
        let ru = detect_columns(&cells(&[
            "№ п/п",
            "Шифр расценки",
            "Наименование работ",
            "Ед. изм.",
            "Количество",
            "Цена",
            "Стоимость",
        ]));
        assert!(ru.usable());
        assert_eq!(ru.name, Some(2));
        assert_eq!(ru.qty, Some(4));
        assert_eq!(ru.cost, Some(6));

        let uz = detect_columns(&cells(&[
            "№", "Kod", "Ish nomi", "Birlik", "Miqdor", "Narx", "Summa",
        ]));
        assert!(uz.usable());
        assert_eq!(uz.unit, Some(3));
        assert_eq!(uz.price, Some(5));
    }

    /// Sarlavha faylning boshida bo'lmasligi mumkin — yuqoridagi shapka o'tkazib
    /// yuboriladi, «Итого» qatori esa yakuniy summaga tushadi.
    #[test]
    fn header_offset_and_total_row() {
        let rows = vec![
            cells(&["Локальная смета № 2-1", "", "", "", "", "", ""]),
            cells(&["Объект: ЖК Навруз", "", "", "", "", "", ""]),
            cells(&[""]),
            cells(&[
                "№",
                "Шифр",
                "Наименование",
                "Ед. изм.",
                "Кол-во",
                "Цена",
                "Стоимость",
            ]),
            cells(&[
                "1",
                "Е6-1-1",
                "Бетонирование",
                "м3",
                "450",
                "1 250 000",
                "562 500 000",
            ]),
            cells(&[
                "2",
                "Е8-2-1",
                "Кладка стен",
                "м2",
                "1 200",
                "310 000",
                "372 000 000",
            ]),
            cells(&["", "", "Итого по смете", "", "", "", "934 500 000"]),
        ];
        let out = rows_to_items(&rows, "smeta".into()).unwrap();
        assert_eq!(out.items.len(), 2);
        assert_eq!(out.items[0].name, "Бетонирование");
        assert_eq!(out.items[0].qty, 450.0);
        assert_eq!(out.items[0].cost, 562_500_000.0);
        assert_eq!(out.items[1].unit, "м2");
        assert_eq!(out.declared_total, 934_500_000.0);
        assert_eq!(out.skipped, 0);
    }

    /// Bo'lim sarlavhasi pozitsiya emas — sanalmaydi va xato ham bermaydi.
    #[test]
    fn section_heading_is_not_an_item() {
        let rows = vec![
            cells(&[
                "№",
                "Наименование",
                "Ед. изм.",
                "Кол-во",
                "Цена",
                "Стоимость",
            ]),
            cells(&["", "Раздел 1. Земляные работы", "", "", "", ""]),
            cells(&[
                "1",
                "Разработка грунта",
                "м3",
                "800",
                "45 000",
                "36 000 000",
            ]),
        ];
        let out = rows_to_items(&rows, "smeta".into()).unwrap();
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].name, "Разработка грунта");
    }

    /// Summa ustuni yo'q bo'lsa, u miqdor x narxdan hisoblanadi —
    /// arifmetika tekshiruvi soxta xato bermasligi uchun.
    #[test]
    fn missing_cost_column_is_computed() {
        let rows = vec![
            cells(&["№", "Наименование", "Ед. изм.", "Количество", "Цена"]),
            cells(&["1", "Штукатурка", "м2", "100", "50 000"]),
        ];
        let out = rows_to_items(&rows, "smeta".into()).unwrap();
        assert_eq!(out.items[0].cost, 5_000_000.0);
    }

    #[test]
    fn missing_header_is_reported() {
        let rows = vec![cells(&["бла", "бла"]), cells(&["1", "2"])];
        assert!(rows_to_items(&rows, "x".into()).is_err());
    }

    /// Haqiqiy fayl orqali: yozamiz, o'qiymiz, natijani tekshiramiz.
    #[test]
    fn csv_file_round_trip() {
        let path = std::env::temp_dir().join(format!("qurai_import_{}.csv", std::process::id()));
        let csv = "\u{FEFF}Локальная смета № 3\n\
                   \n\
                   № п/п;Шифр;Наименование работ;Ед. изм.;Кол-во;Цена;Стоимость\n\
                   1;Е6-1-1;Бетонирование фундамента;м3;450;1 250 000;562 500 000\n\
                   2;Е8-2-1;\"Кладка стен, кирпич\";м2;1 200;310 000;372 000 000\n\
                   ;;Итого по смете;;;;934 500 000\n";
        std::fs::write(&path, csv).unwrap();

        let out = estimate_from_file(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(out.items.len(), 2);
        assert_eq!(out.items[0].code, "Е6-1-1");
        assert_eq!(out.items[0].unit, "м3");
        assert_eq!(out.items[0].qty, 450.0);
        // Qo'shtirnoq ichidagi vergul ustunni bo'lib yubormasligi kerak.
        assert_eq!(out.items[1].name, "Кладка стен, кирпич");
        assert_eq!(out.items[1].qty, 1200.0);
        assert_eq!(out.declared_total, 934_500_000.0);
        assert_eq!(out.name, format!("qurai_import_{}", std::process::id()));
    }

    /// Buzuq va g'alati fayllar panikaga olib kelmasligi kerak — xato qaytadi.
    #[test]
    fn broken_files_are_refused_not_panicked() {
        let dir = std::env::temp_dir();
        let cases: [(&str, &str); 5] = [
            // Butunlay bo'sh fayl.
            ("empty", ""),
            // Faqat sarlavha, pozitsiyasiz.
            (
                "headeronly",
                "\u{FEFF}№;Наименование;Ед. изм.;Кол-во;Цена\n",
            ),
            // Ustunlar tanilmaydi.
            ("garbage", "aaa;bbb;ccc\n1;2;3\n"),
            // Bitta ustun.
            ("onecol", "Наименование\nБетон\n"),
            // Sarlavha bor, lekin qatorlar kalta — indeks chegaradan chiqmasin.
            (
                "short",
                "№;Шифр;Наименование;Ед. изм.;Кол-во;Цена;Стоимость\n1;E1\n2\n",
            ),
        ];
        for (name, body) in cases {
            let path = dir.join(format!("qurai_bad_{}_{}.csv", std::process::id(), name));
            std::fs::write(&path, body).unwrap();
            // Natija muvaffaqiyat yoki xato bo'lishi mumkin, lekin panika bo'lmasin.
            let r = estimate_from_file(&path);
            std::fs::remove_file(&path).ok();
            if let Ok(v) = r {
                assert!(!v.items.is_empty(), "{name}: bo'sh natija Ok bo'lmasin");
            }
        }
    }

    /// Manfiy va juda katta sonlar hisobni buzmasin.
    #[test]
    fn extreme_numbers_are_handled() {
        assert_eq!(parse_number("-1 500,50"), Some(-1500.5));
        assert_eq!(parse_number("999999999999"), Some(999_999_999_999.0));
        // Son emas — None, panika emas.
        assert_eq!(parse_number("---"), None);
        assert_eq!(parse_number("1.2.3"), None);
    }

    /// Miqdor nol bo'lsa, narx summadan bo'linmaydi (nolga bo'lish).
    #[test]
    fn zero_quantity_does_not_divide_by_zero() {
        let rows = vec![
            cells(&["№", "Наименование", "Ед. изм.", "Количество", "Стоимость"]),
            cells(&["1", "Nol miqdor", "m2", "0", "1000"]),
        ];
        let out = rows_to_items(&rows, "x".into()).unwrap();
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].qty, 0.0);
        // Narx chiqarib bo'lmaydi — nol qoladi, cheksizlik emas.
        assert_eq!(out.items[0].price, 0.0);
        assert!(out.items[0].price.is_finite());
    }

    #[test]
    fn unknown_format_is_refused() {
        let path = std::path::Path::new("smeta.docx");
        assert!(estimate_from_file(path).is_err());
    }

    #[test]
    fn csv_respects_quotes_and_separator() {
        assert_eq!(
            split_csv(r#"1;"Кладка стен, кирпич";м2;1 200"#, ';'),
            vec!["1", "Кладка стен, кирпич", "м2", "1 200"]
        );
    }
}
