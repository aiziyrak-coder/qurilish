//! QR yorliqlar (TZ VI.11): kvartira, partiya va hujjat uchun.
//!
//! Yorliq **qog'oz bilan bazani bog'laydi**: omborga kelgan partiyaga
//! yopishtiriladi, kvartira eshigiga osiladi, hujjatga bosiladi. Telefon
//! kamerasi bilan o'qilganda ekranda yozuv ochiladi (server sozlangan
//! bo'lsa) yoki hech bo'lmasa yozuv nomi ko'rinadi.
//!
//! Ikkita qat'iy qoida:
//!
//! 1. **Kod ma'noli bo'lishi kerak.** Ichida shunchaki raqam emas, obyekt
//!    va yozuv turi yoziladi: boshqa dasturda ochilganda ham nimaligi
//!    tushunarli bo'ladi.
//! 2. **Kamera bilan o'qish ilova ichida yo'q.** Buning uchun qurilma va
//!    kutubxona kerak; yorliqni telefondagi har qanday QR o'quvchi
//!    ochadi, chunki ichida oddiy matn yoki havola turadi.

use crate::pdf;

/// Yorliq: QR ichidagi matn va uning ostidagi yozuv.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// QR ichiga yoziladigan matn yoki havola.
    pub code: String,
    /// Yorliqning birinchi qatori — nima ekani.
    pub title: String,
    /// Ikkinchi qator — qo'shimcha ma'lumot.
    pub note: String,
}

/// Yozuv turi — kod ichida ko'rinadi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Unit,
    Batch,
    Document,
    /// Ishchi bejigi: telefonda o'qilganda kirish-chiqish sahifasi ochiladi.
    Worker,
}

impl Kind {
    /// Kod ichidagi belgi.
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Unit => "unit",
            Kind::Batch => "batch",
            Kind::Document => "doc",
            Kind::Worker => "worker",
        }
    }
}

/// QR ichidagi matnni tuzadi.
///
/// Server sozlangan bo'lsa — telefon ochadigan havola; aks holda
/// `QURAI:obyekt:tur:raqam` ko'rinishidagi matn. Ikkinchi holatda ham
/// yorliq foydali: telefon ekranida yozuv nomi ko'rinadi.
pub fn code(base_url: &str, project: &str, kind: Kind, number: &str) -> String {
    let url = base_url.trim().trim_end_matches('/');
    if url.is_empty() {
        return format!("QURAI:{project}:{}:{number}", kind.tag());
    }
    format!(
        "{url}/o/{}?{}={}",
        urlencode(project),
        kind.tag(),
        urlencode(number)
    )
}

/// Manzil uchun matnni xavfsiz ko'rinishga keltiradi.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Bitta yorliqning o'lchami, millimetrda.
const LABEL_W: f32 = 63.5;
const LABEL_H: f32 = 38.1;
/// A4 varaqdagi ustun va qator soni (3 × 7 = 21 ta yorliq).
const COLS: usize = 3;
const ROWS: usize = 7;
/// Varaq chekkasi.
const MARGIN: f32 = 8.0;

/// QR modulining eng kichik o'lchami: undan kichigi bosmada o'qilmaydi.
const MIN_MODULE: f32 = 0.5;

/// Yorliqlarni A4 varaqlarga joylashtirib PDF ga yozadi.
///
/// O'lcham standart yorliq qog'oziga mos (63,5 × 38,1 mm, varaqda 21 ta) —
/// oddiy qog'ozga ham bosib, qirqib ishlatish mumkin.
pub fn write_labels(path: &std::path::Path, title: &str, labels: &[Label]) -> Result<(), String> {
    if labels.is_empty() {
        return Err(crate::i18n::t("qr_empty").to_string());
    }
    pdf::write_pages(
        path,
        title,
        labels.len().div_ceil(COLS * ROWS),
        |page, sheet| {
            for (i, label) in labels
                .iter()
                .skip(sheet * COLS * ROWS)
                .take(COLS * ROWS)
                .enumerate()
            {
                let col = i % COLS;
                let row = i / COLS;
                let x = MARGIN + col as f32 * LABEL_W;
                // PDF da y pastdan o'sadi: birinchi qator yuqorida turishi
                // uchun balandlikdan ayiriladi.
                let y = pdf::PAGE_H_PORTRAIT - MARGIN - (row + 1) as f32 * LABEL_H;
                draw_label(page, x, y, label);
            }
        },
    )
}

/// Bitta yorliqni chizadi: QR kvadrat va yonida ikki qator yozuv.
fn draw_label(page: &pdf::Page, x: f32, y: f32, label: &Label) {
    use qrcode::QrCode;

    // QR chizilmasa yorliq baribir chiqadi: yozuv o'zi ham foydali.
    if let Ok(qr) = QrCode::new(label.code.as_bytes()) {
        let size = qr.width();
        let side = (LABEL_H - 8.0).max(10.0);
        let module = (side / size as f32).max(MIN_MODULE);
        let start_x = x + 3.0;
        let start_y = y + 4.0;
        for (index, dark) in qr.to_colors().iter().enumerate() {
            if *dark != qrcode::Color::Dark {
                continue;
            }
            let mx = index % size;
            // QR ning birinchi qatori yuqorida turishi kerak.
            let my = size - 1 - index / size;
            page.fill_rect(
                start_x + mx as f32 * module,
                start_y + my as f32 * module,
                module,
                module,
            );
        }
    }

    let text_x = x + LABEL_H - 2.0;
    page.text(&label.title, 9.0, text_x, y + LABEL_H - 12.0);
    page.text(&label.note, 7.5, text_x, y + LABEL_H - 20.0);
    // Kod ostida kichkina qilib yoziladi: QR o'qilmasa qo'lda kiritiladi.
    page.text(&short(&label.code), 6.0, text_x, y + 5.0);
    page.frame(x + 1.0, y + 1.0, LABEL_W - 2.0, LABEL_H - 2.0);
}

/// Uzun kodni yorliqqa sig'diradi.
fn short(code: &str) -> String {
    if code.chars().count() <= 42 {
        return code.to_string();
    }
    let tail: String = code.chars().rev().take(39).collect();
    format!("…{}", tail.chars().rev().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Server sozlanmagan bo'lsa ham kod ma'noli bo'ladi.
    #[test]
    fn code_is_readable_without_a_server() {
        let c = code("", "OBY-1", Kind::Batch, "P-12");
        assert_eq!(c, "QURAI:OBY-1:batch:P-12");

        // Server bo'lsa — telefon ochadigan havola.
        let c = code("http://10.0.0.5:8080/", "OBY 1", Kind::Unit, "12/A");
        assert_eq!(c, "http://10.0.0.5:8080/o/OBY%201?unit=12%2FA");
    }

    /// Uzun kod yorliqqa sig'adi va oxiri ko'rinadi.
    #[test]
    fn long_code_keeps_its_tail() {
        let long = format!("http://server.example/o/{}", "x".repeat(80));
        let s = short(&long);
        assert!(s.chars().count() <= 40, "{s}");
        assert!(s.starts_with('…'));
        assert!(s.ends_with("xxx"));
        // Qisqa kod o'zgarmaydi.
        assert_eq!(short("QURAI:OBY-1:doc:AOSR-1"), "QURAI:OBY-1:doc:AOSR-1");
    }

    /// Yorliqlar haqiqiy PDF ga yoziladi va varaqlarga bo'linadi.
    #[test]
    fn labels_are_written_to_a_real_pdf() {
        if crate::pdf::font_bytes().is_none() {
            return;
        }
        let labels: Vec<Label> = (0..25)
            .map(|i| Label {
                code: code("", "OBY-1", Kind::Unit, &format!("{i}")),
                title: format!("Kvartira {i}"),
                note: "3-qavat · 62 m²".into(),
            })
            .collect();

        let path = std::env::temp_dir().join(format!("qurai_qr_{}.pdf", std::process::id()));
        let _ = std::fs::remove_file(&path);
        write_labels(&path, "QR", &labels).expect("yozilmadi");

        let bytes = std::fs::read(&path).expect("fayl");
        assert!(bytes.starts_with(b"%PDF"));
        // 25 yorliq — ikki varaq (21 + 4).
        assert!(bytes.len() > 20_000, "fayl juda kichik: {}", bytes.len());
        let _ = std::fs::remove_file(&path);
    }

    /// Bo'sh ro'yxatda fayl yozilmaydi va sabab aytiladi.
    #[test]
    fn empty_list_is_refused() {
        let path = std::env::temp_dir().join("qurai_qr_empty.pdf");
        assert!(write_labels(&path, "QR", &[]).is_err());
        assert!(!path.exists());
    }
}
