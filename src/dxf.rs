//! DXF chizmasini o'qish va bilimlar grafiga aylantirish (TZ II.1–2).
//!
//! DWG va RVT — yopiq formatlar; ularni ochish uchun litsenziyali kutubxona
//! kerak. **DXF esa o'sha CAD dasturlarining ochiq almashinuv formati**:
//! AutoCAD, nanoCAD, ZWCAD, BricsCAD — hammasi «Save As → DXF» beradi.
//! Shuning uchun chizmani o'qish shu format orqali qilinadi.
//!
//! DXF ning tuzilishi oddiy: fayl juft-juft qatorlardan iborat — avval
//! **kod**, keyin **qiymat**. Kod nimani anglatishi standartda belgilangan:
//! 0 — obyekt turi, 8 — qatlam nomi, 1 — matn, 10/20 — koordinata.
//!
//! Chizmadan nima olinadi va nima olinmaydi, ochiq aytilgan:
//!
//! - **Olinadi**: matnlar (`TEXT`, `MTEXT`) — bular chizmadagi markalar,
//!   xona nomlari va izohlar; qatlam nomi — bo'limni aniqlaydi; blok
//!   nusxalari (`INSERT`) — jihoz va uskunalar.
//! - **Olinmaydi**: geometriya (chiziq, yoy, shtrix). Ulardan hajm hisoblash
//!   uchun 3D model kerak; chizmadagi chiziq esa faqat tasvir. Dastur
//!   ularni o'ylab topmaydi.

use crate::domain::{Element, ElementKind};
use crate::model::Section;

/// DXF dan olingan bitta yozuv.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Obyekt turi: `TEXT`, `MTEXT`, `INSERT`.
    pub kind: String,
    /// Qatlam nomi — bo'lim shundan aniqlanadi.
    pub layer: String,
    /// Matn yoki blok nomi.
    pub text: String,
    /// Chizmadagi o'rni.
    pub x: f64,
    pub y: f64,
}

/// O'qish natijasi.
#[derive(Debug, Default)]
pub struct Drawing {
    pub entries: Vec<Entry>,
    /// Faylda uchragan, lekin olinmagan obyektlar soni (geometriya).
    pub skipped: usize,
    /// Chizmadagi qatlamlar.
    pub layers: Vec<String>,
}

/// Matn olinadigan obyektlar.
const TEXT_KINDS: [&str; 3] = ["TEXT", "MTEXT", "ATTRIB"];

/// DXF faylini o'qiydi.
///
/// Parser ASCII DXF bilan ishlaydi. Ikkilik DXF (`AutoCAD Binary DXF`)
/// va DWG o'qilmaydi — ular uchun CAD dan oddiy DXF eksport qilish kerak.
pub fn parse(src: &str) -> Drawing {
    let mut out = Drawing::default();
    let mut lines = src.lines().map(|l| l.trim());

    // Joriy obyekt: turi va yig'ilayotgan maydonlari.
    let mut kind = String::new();
    let mut layer = String::new();
    let mut text = String::new();
    let mut x = 0.0_f64;
    let mut y = 0.0_f64;

    // Obyekt tugaganda uni ro'yxatga qo'shamiz.
    macro_rules! flush {
        () => {
            if !kind.is_empty() {
                if TEXT_KINDS.contains(&kind.as_str()) || kind == "INSERT" {
                    let value = clean(&text);
                    if !value.is_empty() {
                        out.entries.push(Entry {
                            kind: kind.clone(),
                            layer: layer.clone(),
                            text: value,
                            x,
                            y,
                        });
                    } else {
                        out.skipped += 1;
                    }
                } else {
                    out.skipped += 1;
                }
            }
            layer.clear();
            text.clear();
            // Koordinatalar keyingi obyektda qayta o'qiladi; bu yerda
            // ularni tozalash kerak emas.
        };
    }

    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        let Ok(code) = code.parse::<i32>() else {
            // Kod raqam bo'lmasa — bu ASCII DXF emas.
            continue;
        };
        match code {
            0 => {
                flush!();
                kind = value.to_uppercase();
                if kind == "EOF" {
                    break;
                }
            }
            // 8 — qatlam nomi.
            8 => {
                layer = value.to_string();
                if !out.layers.iter().any(|l| l == value) {
                    out.layers.push(value.to_string());
                }
            }
            // 1 — asosiy matn, 2 — blok nomi (INSERT uchun).
            1 => text = value.to_string(),
            2 if kind == "INSERT" => text = value.to_string(),
            // 3 — MTEXT ning davomi (uzun matn bo'laklarga bo'linadi).
            3 => text.push_str(value),
            10 => x = value.parse().unwrap_or(0.0),
            20 => y = value.parse().unwrap_or(0.0),
            _ => {}
        }
    }
    flush!();
    out
}

/// MTEXT formatlash belgilarini olib tashlaydi.
///
/// MTEXT ichida `\P` — yangi qator, `{\fArial|b0;matn}` — shrift belgisi.
/// Bular chizmada ko'rinmaydi, shuning uchun matnda ham qolmasligi kerak.
fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('P') | Some('p') => out.push(' '),
                // `\A1;`, `\fArial;` — nuqta-vergulgacha tashlanadi.
                Some(_) => {
                    for n in chars.by_ref() {
                        if n == ';' {
                            break;
                        }
                    }
                }
                None => {}
            },
            '{' | '}' => {}
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Qatlam nomidan bo'limni aniqlaydi.
///
/// Qatlam nomlari standart emas, lekin amalda ular bo'lim kodini o'z ichiga
/// oladi: `AR_Devorlar`, `КЖ-Колонны`, `EOM-Cable`. Kod topilmasa bo'lim
/// `None` bo'lib qoladi — dastur taxmin qilmaydi.
pub fn section_of(layer: &str) -> Section {
    let l = layer.to_uppercase().replace(['-', '_', ' ', '.'], "");
    // Uzunroq kodlar oldin tekshiriladi: «EOM» «OV» dan oldin.
    const MAP: &[(&[&str], Section)] = &[
        (&["EOM", "ЭОМ", "ELEKTR", "ЭЛЕКТР"], Section::Eom),
        (&["SS", "СС", "SKS", "СКС"], Section::Ss),
        (&["PB", "ПБ", "POJ", "ПОЖ"], Section::Pb),
        (&["OV", "ОВ", "VENT", "ВЕНТ"], Section::Ov),
        (&["VK", "ВК", "SUV", "ВОДО"], Section::Vk),
        (&["KJ", "КЖ", "BETON", "БЕТОН"], Section::Kj),
        (&["KM", "КМ", "METALL", "МЕТАЛЛ"], Section::Km),
        (&["AR", "АР", "ARX", "АРХ"], Section::Ar),
    ];
    for (keys, section) in MAP {
        if keys.iter().any(|k| l.starts_with(k) || l.contains(k)) {
            return *section;
        }
    }
    Section::None
}

/// Matn ko'rinishidan element turini aniqlaydi.
///
/// Chizmadagi marka o'z turini aytadi: `ОК-12` — deraza, `Д-3` — eshik,
/// `К1` — kolonna. Tanilmagan matn izoh bo'lib qoladi.
fn kind_of(text: &str) -> ElementKind {
    let t = text.to_uppercase();
    let starts = |p: &[&str]| p.iter().any(|x| t.starts_with(x));
    if starts(&["ОК", "OK", "W-", "ОКНО"]) {
        return ElementKind::Window;
    }
    if starts(&["Д-", "D-", "ДВ", "ESHIK", "ДВЕРЬ"]) {
        return ElementKind::Door;
    }
    if starts(&["К-", "K-", "К1", "K1", "КОЛ", "KOL"]) {
        return ElementKind::Column;
    }
    if starts(&["Б-", "B-", "БАЛ", "BAL", "РИГ"]) {
        return ElementKind::Beam;
    }
    if starts(&["ПЛ", "PL", "ПЕРЕКР"]) {
        return ElementKind::Slab;
    }
    ElementKind::Other
}

/// Xona nomi ko'rinishidagi matn.
///
/// Xona nomi odatda raqam bilan boshlanadi va nomi bor: «101 Yotoqxona»,
/// «205 Кухня». Faqat raqam bo'lsa — bu o'lchov, xona emas.
fn is_room(text: &str) -> bool {
    let mut parts = text.split_whitespace();
    let Some(first) = parts.next() else {
        return false;
    };
    let numeric = first
        .trim_end_matches(['.', ','])
        .chars()
        .all(|c| c.is_ascii_digit());
    numeric && first.len() >= 2 && parts.next().is_some()
}

/// Chizmadan bilimlar grafi tuzadi.
///
/// Har bir matn alohida element bo'lmaydi: o'lchov raqamlari, sarlavhalar va
/// bo'sh yozuvlar tashlanadi. Qolganlari marka yoki xona sifatida yoziladi,
/// qaysi varaqdan kelgani esa `sheet` da saqlanadi.
pub fn to_elements(dw: &Drawing, project_id: i64, sheet: &str) -> Vec<Element> {
    let mut out = Vec::new();
    for e in &dw.entries {
        let text = e.text.trim();
        // Faqat sondan iborat yozuv — o'lchov, element emas.
        if text.is_empty()
            || text
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | ' ' | '-' | '+'))
        {
            continue;
        }
        // Juda uzun matn — izoh yoki eslatma, marka emas.
        if text.chars().count() > 64 {
            continue;
        }

        let room = is_room(text);
        out.push(Element {
            id: 0,
            project_id,
            section: section_of(&e.layer),
            kind: if room {
                ElementKind::Room
            } else {
                kind_of(text)
            },
            mark: if room {
                String::new()
            } else {
                text.to_string()
            },
            room: if room {
                text.to_string()
            } else {
                String::new()
            },
            axis: String::new(),
            level: String::new(),
            size: 0.0,
            unit: String::new(),
            value: 0.0,
            value_name: String::new(),
            sheet: sheet.to_string(),
            // Qatlam nomi saqlanadi: bo'lim noto'g'ri aniqlansa, muhandis
            // qayerdan kelganini ko'radi.
            note: format!("{}: {}", crate::i18n::t("dxf_layer"), e.layer),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Oddiy DXF: ikkita matn va bitta chiziq.
    const SAMPLE: &str = "0\nSECTION\n2\nENTITIES\n0\nTEXT\n8\nAR-Markalar\n10\n100.0\n20\n50.0\n1\nОК-12\n0\nTEXT\n8\nAR-Xonalar\n10\n200.0\n20\n80.0\n1\n101 Yotoqxona\n0\nLINE\n8\nAR-Devor\n10\n0.0\n20\n0.0\n0\nENDSEC\n0\nEOF\n";

    /// Matnlar o'qiladi, geometriya sanaladi lekin olinmaydi.
    #[test]
    fn text_is_read_geometry_is_counted() {
        let dw = parse(SAMPLE);
        assert_eq!(dw.entries.len(), 2, "{:?}", dw.entries);
        assert_eq!(dw.entries[0].text, "ОК-12");
        assert_eq!(dw.entries[0].layer, "AR-Markalar");
        assert_eq!(dw.entries[0].x, 100.0);
        assert_eq!(dw.entries[1].text, "101 Yotoqxona");
        // LINE va bo'limlar olinmadi, lekin yo'qolmadi — sanaldi.
        assert!(dw.skipped >= 1);
        assert!(dw.layers.contains(&"AR-Devor".to_string()));
    }

    /// Qatlam nomidan bo'lim aniqlanadi, tanilmasa taxmin qilinmaydi.
    #[test]
    fn layer_name_gives_the_section() {
        assert_eq!(section_of("AR-Devorlar"), Section::Ar);
        assert_eq!(section_of("КЖ_Колонны"), Section::Kj);
        assert_eq!(section_of("EOM Cable"), Section::Eom);
        assert_eq!(section_of("ЭОМ-Розетки"), Section::Eom);
        assert_eq!(section_of("Layer1"), Section::None);
        assert_eq!(section_of(""), Section::None);
    }

    /// MTEXT formatlash belgilari matnda qolmaydi.
    #[test]
    fn mtext_formatting_is_removed() {
        assert_eq!(clean(r"{\fArial|b0;ОК-12}"), "ОК-12");
        assert_eq!(clean(r"Birinchi\Pikkinchi"), "Birinchi ikkinchi");
        assert_eq!(clean("  ko'p    bo'shliq "), "ko'p bo'shliq");
    }

    /// O'lchov raqamlari element bo'lmaydi.
    #[test]
    fn dimension_numbers_are_not_elements() {
        let dw = Drawing {
            entries: vec![
                Entry {
                    kind: "TEXT".into(),
                    layer: "AR".into(),
                    text: "3600".into(),
                    x: 0.0,
                    y: 0.0,
                },
                Entry {
                    kind: "TEXT".into(),
                    layer: "AR".into(),
                    text: "ОК-12".into(),
                    x: 0.0,
                    y: 0.0,
                },
            ],
            skipped: 0,
            layers: Vec::new(),
        };
        let els = to_elements(&dw, 1, "L-1");
        assert_eq!(els.len(), 1);
        assert_eq!(els[0].mark, "ОК-12");
        assert_eq!(els[0].kind, ElementKind::Window);
        assert_eq!(els[0].sheet, "L-1");
    }

    /// Xona nomi marka emas, xona bo'lib yoziladi.
    #[test]
    fn room_label_becomes_a_room() {
        let dw = parse(SAMPLE);
        let els = to_elements(&dw, 1, "L-1");
        let room = els
            .iter()
            .find(|e| e.kind == ElementKind::Room)
            .expect("xona");
        assert_eq!(room.room, "101 Yotoqxona");
        assert!(room.mark.is_empty());
        // Faqat raqamli yozuv xona emas.
        assert!(!is_room("3600"));
        assert!(!is_room("101"));
        assert!(is_room("205 Кухня"));
    }

    /// Bo'sh yoki ikkilik fayl yiqilmaydi.
    #[test]
    fn broken_input_does_not_panic() {
        assert!(parse("").entries.is_empty());
        assert!(parse("shunchaki matn\nyana matn").entries.is_empty());
        // Toq sonli qatorlar — oxirgi juftlik to'liq emas.
        assert!(parse("0\nTEXT\n8").entries.is_empty());
    }
}
