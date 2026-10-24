//! PDF sahifa oqimidan faqat **matn** buyruqlarini tez ajratib olish.
//!
//! `lopdf::Content::decode` oqimdagi har bir buyruqni (chiziq, egri, to'ldirish)
//! obyektga aylantiradi. Chizma varaqlarida bu yuz minglab buyruq: haqiqiy
//! 77 varaqli loyihada (135 MB oqim) bu 114 soniya oldi — o'qish vaqtining
//! 99 %. Bizga faqat matn va uning joylashuvi kerak, shuning uchun bu
//! tahlilchi oqimni bir marta o'tadi va faqat quyidagilarni qaytaradi:
//!
//! `q Q cm BT ET Tf TL Td TD Tm T* Tj ' " TJ`
//!
//! Boshqa buyruqlarning operandlari obyektga aylantirilmaydi — tashlanadi.
//! Natija `lopdf::content::Operation` bilan bir xil ko'rinishda, shuning
//! uchun matn yig'ish kodi o'zgarmaydi.

use lopdf::content::Operation;
use lopdf::{Object, StringFormat};

/// Bizga kerak bo'lgan buyruqlar.
const KEEP: [&[u8]; 15] = [
    b"q", b"Q", b"cm", b"BT", b"ET", b"Tf", b"TL", b"Td", b"TD", b"Tm", b"T*", b"Tj", b"'", b"\"",
    b"TJ",
];

fn is_white(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0c | 0)
}

fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// Oqimni o'qiydi va matn buyruqlarini qaytaradi.
pub fn text_ops(data: &[u8]) -> Vec<Operation> {
    let mut ops = Vec::new();
    let mut stack: Vec<Object> = Vec::new();
    let mut i = 0;
    let n = data.len();
    while i < n {
        let b = data[i];
        if is_white(b) {
            i += 1;
            continue;
        }
        match b {
            b'%' => {
                while i < n && data[i] != b'\n' && data[i] != b'\r' {
                    i += 1;
                }
            }
            b'(' => {
                let (s, next) = literal(data, i + 1);
                stack.push(Object::String(s, StringFormat::Literal));
                i = next;
            }
            b'<' if i + 1 < n && data[i + 1] == b'<' => {
                // Lug'at (inline rasm parametrlari va h.k.) — o'tkazib yuboriladi.
                i = skip_dict(data, i);
            }
            b'<' => {
                let (s, next) = hex(data, i + 1);
                stack.push(Object::String(s, StringFormat::Hexadecimal));
                i = next;
            }
            b'[' => {
                let (arr, next) = array(data, i + 1);
                stack.push(Object::Array(arr));
                i = next;
            }
            b'/' => {
                let start = i + 1;
                let mut j = start;
                while j < n && !is_white(data[j]) && !is_delim(data[j]) {
                    j += 1;
                }
                stack.push(Object::Name(data[start..j].to_vec()));
                i = j;
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let (num, next) = number(data, i);
                stack.push(num);
                i = next;
            }
            b']' | b'>' | b')' | b'{' | b'}' => {
                i += 1;
            }
            _ => {
                // Buyruq (operator) so'zi.
                let start = i;
                let mut j = i;
                while j < n && !is_white(data[j]) && !is_delim(data[j]) {
                    j += 1;
                }
                if j == start {
                    j += 1;
                }
                let word = &data[start..j];
                // Inline rasm: `BI ... ID <ikkilik> EI` — ma'lumot o'tkazib yuboriladi.
                if word == b"ID" {
                    j = skip_inline_image(data, j);
                } else if KEEP.contains(&word) {
                    ops.push(Operation::new(
                        std::str::from_utf8(word).unwrap_or(""),
                        std::mem::take(&mut stack),
                    ));
                }
                stack.clear();
                i = j;
            }
        }
    }
    ops
}

fn number(data: &[u8], mut i: usize) -> (Object, usize) {
    let start = i;
    let n = data.len();
    while i < n && matches!(data[i], b'+' | b'-' | b'.' | b'0'..=b'9') {
        i += 1;
    }
    let s = std::str::from_utf8(&data[start..i]).unwrap_or("0");
    if s.contains('.') {
        (Object::Real(s.parse::<f32>().unwrap_or(0.0)), i)
    } else {
        (Object::Integer(s.parse::<i64>().unwrap_or(0)), i)
    }
}

/// `( ... )` satri: qochish belgilari va ichki qavslar bilan.
fn literal(data: &[u8], mut i: usize) -> (Vec<u8>, usize) {
    let n = data.len();
    let mut out = Vec::new();
    let mut depth = 1;
    while i < n {
        let b = data[i];
        match b {
            b'\\' if i + 1 < n => {
                let c = data[i + 1];
                i += 2;
                match c {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'\r' => {
                        if i < n && data[i] == b'\n' {
                            i += 1;
                        }
                    }
                    b'\n' => {}
                    b'0'..=b'7' => {
                        let mut v = (c - b'0') as u32;
                        for _ in 0..2 {
                            if i < n && (b'0'..=b'7').contains(&data[i]) {
                                v = v * 8 + (data[i] - b'0') as u32;
                                i += 1;
                            }
                        }
                        out.push(v as u8);
                    }
                    other => out.push(other),
                }
            }
            b'(' => {
                depth += 1;
                out.push(b);
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    break;
                }
                out.push(b);
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    (out, i)
}

/// `< ... >` o'n oltilik satri.
fn hex(data: &[u8], mut i: usize) -> (Vec<u8>, usize) {
    let n = data.len();
    let mut digits = Vec::new();
    while i < n && data[i] != b'>' {
        let b = data[i];
        if b.is_ascii_hexdigit() {
            digits.push(b);
        }
        i += 1;
    }
    if digits.len() % 2 == 1 {
        digits.push(b'0');
    }
    let val = |c: u8| -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => c - b'A' + 10,
        }
    };
    let out = digits
        .chunks(2)
        .map(|p| val(p[0]) * 16 + val(p[1]))
        .collect();
    (out, (i + 1).min(n))
}

/// `[ ... ]` massiv — `TJ` uchun: satrlar va sonlar.
fn array(data: &[u8], mut i: usize) -> (Vec<Object>, usize) {
    let n = data.len();
    let mut out = Vec::new();
    while i < n {
        let b = data[i];
        if is_white(b) {
            i += 1;
            continue;
        }
        match b {
            b']' => return (out, i + 1),
            b'(' => {
                let (s, next) = literal(data, i + 1);
                out.push(Object::String(s, StringFormat::Literal));
                i = next;
            }
            b'<' => {
                let (s, next) = hex(data, i + 1);
                out.push(Object::String(s, StringFormat::Hexadecimal));
                i = next;
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let (num, next) = number(data, i);
                out.push(num);
                i = next;
            }
            b'[' => {
                let (inner, next) = array(data, i + 1);
                out.push(Object::Array(inner));
                i = next;
            }
            _ => i += 1,
        }
    }
    (out, i)
}

fn skip_dict(data: &[u8], mut i: usize) -> usize {
    let n = data.len();
    let mut depth = 0;
    while i < n {
        if data[i] == b'<' && i + 1 < n && data[i + 1] == b'<' {
            depth += 1;
            i += 2;
        } else if data[i] == b'>' && i + 1 < n && data[i + 1] == b'>' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else if data[i] == b'(' {
            i = literal(data, i + 1).1;
        } else {
            i += 1;
        }
    }
    i
}

/// Inline rasm ma'lumoti: `ID` dan keyin `EI` gacha.
fn skip_inline_image(data: &[u8], mut i: usize) -> usize {
    let n = data.len();
    while i + 2 < n {
        if is_white(data[i])
            && data[i + 1] == b'E'
            && data[i + 2] == b'I'
            && (i + 3 >= n || is_white(data[i + 3]) || is_delim(data[i + 3]))
        {
            return i + 3;
        }
        i += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;

    /// Tez tahlilchi matn buyruqlarida lopdf bilan aynan bir xil natija beradi.
    #[test]
    fn same_text_ops_as_lopdf() {
        let stream = b"q 1 0 0 1 10 20 cm 0.5 w 0 0 m 100 100 l S \
            BT /F1 12 Tf 14 TL 72 712 Td (Salom \\(dunyo\\)) Tj T* \
            [(A) -250 (B) 120 <0041>] TJ 1 0 0 1 5 5 Tm (\\101\\102) ' ET Q \
            BI /W 2 /H 2 /BPC 8 /CS /G ID \x00\xff\x10 EI 50 50 m";
        let ours = text_ops(stream);
        let theirs: Vec<Operation> = Content::decode(stream)
            .unwrap()
            .operations
            .into_iter()
            .filter(|o| KEEP.contains(&o.operator.as_bytes()))
            .collect();
        assert_eq!(ours.len(), theirs.len());
        for (a, b) in ours.iter().zip(theirs.iter()) {
            assert_eq!(a.operator, b.operator);
            assert_eq!(format!("{:?}", a.operands), format!("{:?}", b.operands));
        }
    }
}
