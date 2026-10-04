//! PDF dan jadval o'qish (TZ III.2).
//!
//! Smeta ko'pincha PDF ko'rinishida keladi. PDF da jadval yo'q — unda faqat
//! **matn parchalari va ularning qog'ozdagi koordinatalari** bor. Shuning
//! uchun jadval qaytadan tiklanadi: bir xil balandlikdagi parchalar bitta
//! qator, chapdan o'ngga qarab turgan parchalar esa ustunlar bo'ladi.
//!
//! Chegara ochiq: **skan qilingan PDF o'qilmaydi.** Skanda matn emas, rasm
//! bo'ladi; uni o'qish uchun matnni tanish (OCR) kerak va bu ilovaning ichida
//! qilinmaydi. Shuning uchun matni yo'q PDF uchun dastur taxmin qilmaydi,
//! ochiq aytadi.

use lopdf::content::Content;
use lopdf::{Document, Object};
use std::path::Path;

/// Bitta matn parchasi va uning qog'ozdagi o'rni.
#[derive(Debug, Clone)]
pub struct Piece {
    /// Chapdan masofa (PDF birligi — punkt).
    pub x: f64,
    /// Pastdan masofa. PDF da y yuqoriga qarab o'sadi.
    pub y: f64,
    /// Shrift o'lchami — parcha kengligini baholash uchun.
    pub size: f64,
    pub text: String,
}

/// PDF ni sahifalar bo'yicha, **joylashuvi bilan** o'qiydi.
///
/// `table` matnni qatorlarga yig'ib beradi va joylashuvni tashlab
/// yuboradi. Chizma varag'ida bu yetmaydi: bitta balandlikda ham jadval
/// katagi, ham chizmadagi o'lcham yozuvi turadi va ular bitta qatorga
/// aralashib ketadi. Spetsifikatsiyani aniq o'qish uchun har parchaning
/// o'rni kerak — ustun sarlavhaning tagidagi matn sifatida aniqlanadi.
pub fn pages(path: &Path) -> Result<Vec<Vec<Piece>>, String> {
    let doc = Document::load(path).map_err(|e| format!("{e}"))?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(crate::i18n::t("pdf_no_pages").to_string());
    }
    let out: Vec<Vec<Piece>> = pages
        .into_values()
        .map(|id| page_pieces(&doc, id))
        .collect();
    if out.iter().all(|p| p.is_empty()) {
        return Err(crate::i18n::t("pdf_no_text").to_string());
    }
    Ok(out)
}

/// Bir qatorga tegishli deb hisoblanadigan balandlik farqi, punktda.
///
/// Bitta qatordagi harflar aynan bir balandlikda turmaydi: indekslar va
/// turli o'lchamdagi shriftlar biroz suriladi. 3 punkt — odatdagi
/// 9-11 punktli matn uchun xavfsiz oraliq.
const LINE_TOLERANCE: f64 = 3.0;

/// Ustunlar chegarasi: shundan katta bo'shliq yangi ustun deb qaraladi.
///
/// Chegara shrift o'lchamiga bog'liq: kichik shriftda ustunlar ham yaqin
/// turadi. Bir belgi kengligi taxminan o'lchamning yarmi, shuning uchun
/// ikki belgilik bo'shliq ustun chegarasi deb olinadi.
fn column_gap(size: f64) -> f64 {
    (size * 1.0).clamp(6.0, 24.0)
}

/// Belgining taxminiy kengligi: shrift o'lchamining yarmi.
///
/// Aniq kenglik shrift metrikasidan chiqadi. Bu yerda u kerak emas:
/// taxmin faqat parchalar orasidagi bo'shliqni o'lchash uchun, qiymatga
/// ta'sir qilmaydi.
fn char_width(size: f64) -> f64 {
    size * 0.5
}

/// PDF ni jadval ko'rinishida o'qiydi.
///
/// Natija — qatorlar; har qator ustunlar bo'yicha ajratilgan matn.
/// Matn topilmasa `Err` qaytadi va sabab ochiq aytiladi.
pub fn table(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let doc = Document::load(path).map_err(|e| format!("{e}"))?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(crate::i18n::t("pdf_no_pages").to_string());
    }

    for (_, page_id) in pages {
        let pieces = page_pieces(&doc, page_id);
        rows.extend(lines_to_rows(pieces));
    }

    if rows.is_empty() {
        return Err(crate::i18n::t("pdf_no_text").to_string());
    }
    Ok(rows)
}

/// Shriftning dekodlash usuli.
///
/// Zamonaviy PDF larda matn baytlari harf kodlari emas — ular shrift
/// ichidagi **glif raqamlari**. Ularni o'qish uchun PDF ning o'zida
/// `ToUnicode` jadvali beriladi. Shu jadvalsiz matn «6PHWD» kabi
/// ma'nosiz ko'rinadi, shuning uchun avval u qaraladi.
enum FontDecode {
    /// Oddiy kodlash (WinAnsi, MacRoman va h.k.).
    Simple(String),
    /// `ToUnicode` jadvali: kod → matn. `width` — bitta kodning bayt soni.
    Cmap {
        map: std::collections::HashMap<u32, String>,
        width: usize,
    },
}

impl FontDecode {
    fn decode(&self, bytes: &[u8]) -> String {
        match self {
            FontDecode::Simple(enc) => Document::decode_text(Some(enc.as_str()), bytes),
            FontDecode::Cmap { map, width } => {
                let mut out = String::new();
                for chunk in bytes.chunks(*width) {
                    let mut code = 0u32;
                    for b in chunk {
                        code = (code << 8) | *b as u32;
                    }
                    if let Some(s) = map.get(&code) {
                        out.push_str(s);
                    }
                }
                out
            }
        }
    }
}

/// Sahifadagi shriftlar lug'ati.
///
/// Kutubxonaning tayyor funksiyasi ba'zi PDF larda bo'sh qaytaradi (resurs
/// sahifada emas, ota-tugunda bo'lsa yoki havola orqali berilsa). Shuning
/// uchun resurslar o'zimiz qidiriladi: avval sahifada, keyin ota-tugunda.
fn page_fonts(doc: &Document, page_id: lopdf::ObjectId) -> Vec<(Vec<u8>, lopdf::Dictionary)> {
    let mut out = Vec::new();
    let resolve = |o: &Object| -> Option<Object> {
        match o {
            Object::Reference(id) => doc.get_object(*id).ok().cloned(),
            other => Some(other.clone()),
        }
    };

    // Sahifa lug'ati, keyin ota-tugunlar zanjiri.
    let mut current = doc.get_object(page_id).ok().cloned();
    let mut depth = 0;
    while let Some(obj) = current.take() {
        let Ok(dict) = obj.as_dict() else { break };
        if let Some(res) = dict.get(b"Resources").ok().and_then(resolve) {
            if let Ok(res) = res.as_dict() {
                if let Some(fonts) = res.get(b"Font").ok().and_then(resolve) {
                    if let Ok(fonts) = fonts.as_dict() {
                        for (name, value) in fonts.iter() {
                            if let Some(f) = resolve(value).and_then(|o| o.as_dict().ok().cloned())
                            {
                                out.push((name.to_vec(), f));
                            }
                        }
                    }
                }
            }
        }
        if !out.is_empty() || depth > 8 {
            break;
        }
        current = dict.get(b"Parent").ok().and_then(resolve);
        depth += 1;
    }
    out
}

/// Sahifadagi shriftlar uchun dekodlash usulini yig'adi.
fn font_decoders(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> std::collections::BTreeMap<Vec<u8>, FontDecode> {
    let mut out = std::collections::BTreeMap::new();
    for (name, font) in page_fonts(doc, page_id) {
        let dec = font
            .get(b"ToUnicode")
            .ok()
            .and_then(|o| match o {
                Object::Reference(id) => doc.get_object(*id).ok(),
                other => Some(other),
            })
            .and_then(|o| o.as_stream().ok())
            // Oqim siqilgan bo'lishi ham, bo'lmasligi ham mumkin.
            .map(|st| {
                st.decompressed_content()
                    .unwrap_or_else(|_| st.content.clone())
            })
            .and_then(|data| parse_cmap(&String::from_utf8_lossy(&data)))
            .unwrap_or_else(|| FontDecode::Simple(font.get_font_encoding().to_string()));
        out.insert(name, dec);
    }
    out
}

/// `ToUnicode` jadvalini o'qiydi.
///
/// Jadval ikki ko'rinishda bo'ladi: `bfchar` — bitta kod uchun bitta matn,
/// `bfrange` — kodlar oralig'i uchun ketma-ket matnlar. Ikkalasi ham
/// o'n oltilik sonlar bilan yoziladi.
fn parse_cmap(text: &str) -> Option<FontDecode> {
    let hex = |s: &str| u32::from_str_radix(s.trim(), 16).ok();
    // Kod kengligi kodlar fazosidan olinadi: `<0000> <ffff>` — ikki bayt.
    let width = text
        .split("begincodespacerange")
        .nth(1)
        .and_then(|s| s.split('<').nth(1))
        .and_then(|s| s.split('>').next())
        .map(|s| s.trim().len().div_ceil(2))
        .unwrap_or(2)
        .clamp(1, 4);

    let mut map = std::collections::HashMap::new();

    // ---- bfchar: <kod> <matn>
    for block in text.split("beginbfchar").skip(1) {
        let block = block.split("endbfchar").next().unwrap_or("");
        let items: Vec<&str> = hex_items(block);
        for pair in items.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            if let (Some(code), Some(s)) = (hex(pair[0]), utf16(pair[1])) {
                map.insert(code, s);
            }
        }
    }

    // ---- bfrange: <boshi> <oxiri> <matn> yoki [<matn> <matn> ...]
    for block in text.split("beginbfrange").skip(1) {
        let block = block.split("endbfrange").next().unwrap_or("");
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(open) = line.find('[') {
                // Ro'yxat ko'rinishi: har kodga o'z matni.
                let head = hex_items(&line[..open]);
                let list = hex_items(&line[open..]);
                if head.len() < 2 {
                    continue;
                }
                let Some(lo) = hex(head[0]) else { continue };
                for (i, item) in list.iter().enumerate() {
                    if let Some(s) = utf16(item) {
                        map.insert(lo + i as u32, s);
                    }
                }
                continue;
            }
            let items = hex_items(line);
            if items.len() < 3 {
                continue;
            }
            let (Some(lo), Some(hi), Some(start)) = (hex(items[0]), hex(items[1]), hex(items[2]))
            else {
                continue;
            };
            // Juda katta oraliq — buzilgan jadval, o'tkazib yuboriladi.
            if hi < lo || hi - lo > 0xFFFF {
                continue;
            }
            for code in lo..=hi {
                if let Some(ch) = char::from_u32(start + (code - lo)) {
                    map.insert(code, ch.to_string());
                }
            }
        }
    }

    (!map.is_empty()).then_some(FontDecode::Cmap { map, width })
}

/// `<...>` ichidagi o'n oltilik bo'laklar.
fn hex_items(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(a) = rest.find('<') {
        let Some(b) = rest[a + 1..].find('>') else {
            break;
        };
        out.push(&rest[a + 1..a + 1 + b]);
        rest = &rest[a + 1 + b + 1..];
    }
    out
}

/// UTF-16BE o'n oltilik matnni satrga o'giradi.
fn utf16(hex: &str) -> Option<String> {
    let hex = hex.trim();
    if !hex.len().is_multiple_of(4) || hex.is_empty() {
        // Bir baytli qiymat ham uchraydi.
        if hex.len() == 2 {
            return u8::from_str_radix(hex, 16)
                .ok()
                .map(|b| (b as char).to_string());
        }
        return None;
    }
    let mut units = Vec::new();
    for chunk in hex.as_bytes().chunks(4) {
        let s = std::str::from_utf8(chunk).ok()?;
        units.push(u16::from_str_radix(s, 16).ok()?);
    }
    String::from_utf16(&units).ok()
}

/// Bitta sahifadagi matn parchalari va ularning o'rni.
/// 2D o'zgartirish matritsasi: `[a b c d e f]`.
type Mat = [f64; 6];

const IDENT: Mat = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m1` dan keyin `m2` qo'llanadi.
fn mul(m1: &Mat, m2: &Mat) -> Mat {
    [
        m1[0] * m2[0] + m1[1] * m2[2],
        m1[0] * m2[1] + m1[1] * m2[3],
        m1[2] * m2[0] + m1[3] * m2[2],
        m1[2] * m2[1] + m1[3] * m2[3],
        m1[4] * m2[0] + m1[5] * m2[2] + m2[4],
        m1[4] * m2[1] + m1[5] * m2[3] + m2[5],
    ]
}

fn page_pieces(doc: &Document, page_id: lopdf::ObjectId) -> Vec<Piece> {
    let mut out = Vec::new();
    let Ok(data) = doc.get_page_content(page_id) else {
        return out;
    };
    let Ok(content) = Content::decode(&data) else {
        return out;
    };

    // Shrift kodlashi: matn baytlari shu bo'yicha o'giriladi.
    let decoders = font_decoders(doc, page_id);
    let mut decoder: Option<&FontDecode> = None;

    // Grafik holat. Chizma PDF larida matn o'rni ikki matritsadan chiqadi:
    // sahifa matritsasi (`cm`) va matn matritsasi (`Tm`). Avval faqat
    // `Tm` ning siljishi olinardi — masshtab va burilish tashlab
    // yuborilardi. Shu sababli burilgan varaqda jadval ustunlari qatorga,
    // qatorlari ustunga aylanib ketardi.
    let mut ctm: Mat = IDENT;
    let mut stack: Vec<Mat> = Vec::new();
    let mut tm: Mat = IDENT;
    let mut tlm: Mat = IDENT;
    let mut leading = 0.0_f64;
    let mut size = 10.0_f64;

    // Har parcha yo'nalishi bilan yig'iladi: (parcha, dx, dy).
    let mut raw: Vec<(Piece, f64, f64)> = Vec::new();

    for op in &content.operations {
        let nums = |i: usize| -> f64 {
            op.operands
                .get(i)
                .and_then(|o| match o {
                    Object::Integer(v) => Some(*v as f64),
                    Object::Real(v) => Some(*v as f64),
                    _ => None,
                })
                .unwrap_or(0.0)
        };
        match op.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => {
                if let Some(m) = stack.pop() {
                    ctm = m;
                }
            }
            "cm" => {
                let m: Mat = [nums(0), nums(1), nums(2), nums(3), nums(4), nums(5)];
                ctm = mul(&m, &ctm);
            }
            "BT" => {
                tm = IDENT;
                tlm = IDENT;
            }
            "Tf" => {
                if let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) {
                    decoder = decoders.get(name);
                }
                let s = nums(1);
                if s > 0.0 {
                    size = s;
                }
            }
            "TL" => leading = nums(0),
            "Td" | "TD" => {
                if op.operator == "TD" {
                    leading = -nums(1);
                }
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, nums(0), nums(1)], &tlm);
                tm = tlm;
            }
            "Tm" => {
                tlm = [nums(0), nums(1), nums(2), nums(3), nums(4), nums(5)];
                tm = tlm;
            }
            "T*" => {
                tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &tlm);
                tm = tlm;
            }
            "Tj" | "'" | "\"" | "TJ" => {
                let text = decode(&op.operands, decoder);
                let trm = mul(&tm, &ctm);
                let scale = (trm[0] * trm[0] + trm[1] * trm[1]).sqrt();
                if !text.trim().is_empty() && scale > 0.0 {
                    raw.push((
                        Piece {
                            x: trm[4],
                            y: trm[5],
                            size: size * scale,
                            text: text.clone(),
                        },
                        trm[0] / scale,
                        trm[1] / scale,
                    ));
                }
                // Keyingi parcha shu qatorda davom etsa, ustiga tushmasin:
                // matn kengligi taxminan suriladi.
                let advance = text.chars().count() as f64 * size * 0.5;
                tm = mul(&[1.0, 0.0, 0.0, 1.0, advance, 0.0], &tm);
            }
            _ => {}
        }
    }

    // Varaq burilgan bo'lishi mumkin: ko'pchilik matn qaysi tomonga
    // yozilgan bo'lsa, o'sha «gorizontal» deb olinadi va koordinatalar
    // shunga moslab aylantiriladi. Boshqa yo'nalishdagi yozuvlar (odatda
    // chizmadagi vertikal o'lchamlar) tashlanadi — ular jadvalga
    // tegishli emas va qatorlarni buzardi.
    let mut votes = [0usize; 4];
    let side = |dx: f64, dy: f64| -> usize {
        if dx.abs() >= dy.abs() {
            if dx >= 0.0 {
                0
            } else {
                2
            }
        } else if dy > 0.0 {
            1
        } else {
            3
        }
    };
    for (_, dx, dy) in &raw {
        votes[side(*dx, *dy)] += 1;
    }
    let main = (0..4).max_by_key(|i| votes[*i]).unwrap_or(0);
    for (mut p, dx, dy) in raw {
        if side(dx, dy) != main {
            continue;
        }
        let (x, y) = (p.x, p.y);
        (p.x, p.y) = match main {
            0 => (x, y),
            1 => (y, -x),
            2 => (-x, -y),
            _ => (-y, x),
        };
        out.push(p);
    }
    out
}

/// Operand matnini o'qiydi. `TJ` massivi ichidagi sonlar — belgilar orasidagi
/// siljish; kattasi so'zlarni ajratadi.
fn decode(operands: &[Object], decoder: Option<&FontDecode>) -> String {
    let plain = FontDecode::Simple("StandardEncoding".to_string());
    let dec = decoder.unwrap_or(&plain);
    let mut out = String::new();
    for o in operands {
        match o {
            Object::String(bytes, _) => out.push_str(&dec.decode(bytes)),
            Object::Array(items) => {
                for it in items {
                    match it {
                        Object::String(bytes, _) => out.push_str(&dec.decode(bytes)),
                        // Manfiy siljish — bo'shliq (qiymati katta bo'lsa).
                        Object::Integer(v) if *v < -120 => out.push(' '),
                        Object::Real(v) if *v < -120.0 => out.push(' '),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Parchalarni qator va ustunlarga ajratadi.
fn lines_to_rows(mut pieces: Vec<Piece>) -> Vec<Vec<String>> {
    if pieces.is_empty() {
        return Vec::new();
    }
    // Yuqoridan pastga: PDF da y pastdan o'sadi.
    pieces.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut line: Vec<Piece> = Vec::new();
    let mut current = pieces[0].y;

    for p in pieces {
        if (current - p.y).abs() > LINE_TOLERANCE {
            rows.push(split_columns(&mut line));
            current = p.y;
        }
        line.push(p);
    }
    rows.push(split_columns(&mut line));
    rows.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    rows
}

/// Bitta qatordagi parchalarni ustunlarga bo'ladi.
///
/// Ustun chegarasi — parchalar orasidagi bo'shliq. Aniq ustun kengligini
/// bilish uchun shrift metrikasi kerak bo'lardi; bu yerda oxirgi parchaning
/// uzunligi belgilar soniga qarab taxmin qilinadi. Taxmin faqat **ajratish**
/// uchun ishlatiladi, qiymatga ta'sir qilmaydi.
fn split_columns(line: &mut Vec<Piece>) -> Vec<String> {
    line.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut cols: Vec<String> = Vec::new();
    let mut end = f64::MIN;

    for p in line.drain(..) {
        let width = p.text.chars().count() as f64 * char_width(p.size);
        if end == f64::MIN || p.x - end > column_gap(p.size) {
            cols.push(p.text.trim().to_string());
        } else if let Some(last) = cols.last_mut() {
            // Yaqin parchalar — bitta katakning davomi.
            if !last.is_empty() && !last.ends_with(' ') {
                last.push(' ');
            }
            last.push_str(p.text.trim());
        }
        end = p.x + width;
    }
    cols
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(x: f64, y: f64, text: &str) -> Piece {
        Piece {
            x,
            y,
            size: 10.0,
            text: text.into(),
        }
    }

    /// Bir balandlikdagi parchalar bitta qatorga tushadi, uzoqdagilari
    /// alohida ustun bo'ladi.
    #[test]
    fn pieces_become_rows_and_columns() {
        let rows = lines_to_rows(vec![
            piece(50.0, 700.0, "1"),
            piece(90.0, 700.0, "Beton quyish"),
            piece(300.0, 700.0, "m3"),
            piece(50.0, 686.0, "2"),
            piece(90.0, 686.0, "Armatura"),
            piece(300.0, 686.0, "t"),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["1", "Beton quyish", "m3"]);
        assert_eq!(rows[1], vec!["2", "Armatura", "t"]);
    }

    /// Yonma-yon turgan parchalar bitta katakka birlashadi.
    #[test]
    fn near_pieces_join_into_one_cell() {
        let rows = lines_to_rows(vec![
            piece(50.0, 700.0, "Beton"),
            // Oldingi parcha taxminan 50+25=75 da tugaydi; 78 — yaqin.
            piece(78.0, 700.0, "quyish"),
            piece(300.0, 700.0, "m3"),
        ]);
        assert_eq!(rows[0], vec!["Beton quyish", "m3"]);
    }

    /// Balandligi biroz farq qilgan parchalar ham bitta qatorda qoladi.
    #[test]
    fn small_height_difference_stays_on_one_line() {
        let rows = lines_to_rows(vec![
            piece(50.0, 700.0, "A"),
            piece(200.0, 698.5, "B"),
            piece(50.0, 680.0, "C"),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["A", "B"]);
    }

    /// Bo'sh parchalar qator hosil qilmaydi.
    #[test]
    fn empty_pieces_are_dropped() {
        let rows = lines_to_rows(vec![piece(10.0, 10.0, "   ")]);
        assert!(rows.is_empty());
    }

    /// Matritsalar ketma-ket qo'llanadi: avval birinchisi, keyin ikkinchisi.
    #[test]
    fn matrices_compose_in_order() {
        // Ikki barobar kattalashtirib, keyin (10, 5) ga surish.
        let scale: Mat = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        let shift: Mat = [1.0, 0.0, 0.0, 1.0, 10.0, 5.0];
        let m = mul(&scale, &shift);
        // (3, 4) nuqta: (6, 8) bo'ladi, so'ng (16, 13).
        assert_eq!(3.0 * m[0] + 4.0 * m[2] + m[4], 16.0);
        assert_eq!(3.0 * m[1] + 4.0 * m[3] + m[5], 13.0);
        assert_eq!(mul(&IDENT, &shift), shift);
    }

    /// Matni yo'q faylda taxmin qilinmaydi — sabab aytiladi.
    #[test]
    fn missing_file_reports_the_reason() {
        let err = table(std::path::Path::new("yo-q-fayl.pdf")).unwrap_err();
        assert!(!err.is_empty());
    }

    /// O'zimiz yozgan PDF qaytadan o'qiladi: yozish va o'qish bir-biriga mos.
    #[test]
    fn table_written_by_us_reads_back() {
        use crate::docgen::{Cell, Table};

        // Shrift topilmasa PDF yozib bo'lmaydi — bu muhit masalasi.
        let path = std::env::temp_dir().join(format!("qurai_rt_{}.pdf", std::process::id()));
        let t = Table {
            name: "Smeta".into(),
            headers: vec!["Poz".into(), "Nomi".into(), "Birlik".into()],
            rows: vec![
                vec![
                    Cell::Num(1.0),
                    Cell::Text("Beton quyish".into()),
                    Cell::Text("m3".into()),
                ],
                vec![
                    Cell::Num(2.0),
                    Cell::Text("Armatura montaji".into()),
                    Cell::Text("t".into()),
                ],
            ],
        };
        if crate::pdf::write_table(&path, &t, "").is_err() {
            return;
        }

        let rows = table(&path).expect("o'qildi");
        let flat: Vec<String> = rows.iter().map(|r| r.join(" | ")).collect();
        let all = flat.join("\n");
        assert!(all.contains("Beton quyish"), "topilmadi: {all}");
        assert!(all.contains("Armatura montaji"), "topilmadi: {all}");
        // Ustunlar ajralgan bo'lishi kerak, hammasi bitta katakda emas.
        assert!(
            rows.iter().any(|r| r.len() >= 3),
            "ustunlar ajralmadi: {rows:?}"
        );
        let _ = std::fs::remove_file(&path);
    }
}
