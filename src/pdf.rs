//! Jadvalni PDF ga chiqarish (umumiy talab: Excel yonida PDF).
//!
//! Excel — ish uchun: son son bo'lib chiqadi, saralanadi, yig'iladi. PDF —
//! **topshirish uchun**: uni o'zgartirib bo'lmaydi, har sahifada sarlavha,
//! sana va sahifa raqami turadi. Shuning uchun ikkalasi ham kerak va ikkalasi
//! ham bitta jadvaldan chiqadi — ikki xil raqam bo'lishi mumkin emas.
//!
//! Shrift tizimdan olinadi (interfeys shrifti bilan bir xil): faqat shundagina
//! kirill va lotin harflari, shu jumladan `o'` va `g'`, PDF da to'g'ri
//! ko'rinadi. Shrift topilmasa PDF yozilmaydi va buning sababi ochiq
//! aytiladi — o'qib bo'lmaydigan fayl berish yomonroq.

use crate::docgen::{Cell, Table};
use printpdf::{IndirectFontRef, Line, Mm, PdfDocument, PdfLayerReference, Point};
use std::io::BufWriter;
use std::path::Path;

/// A4 albom: kengroq jadval sig'adi.
const PAGE_W: f32 = 297.0;
const PAGE_H: f32 = 210.0;
/// Chekka bo'shliqlar.
const MARGIN: f32 = 12.0;
/// Qator balandligi va shrift o'lchamlari.
const ROW_H: f32 = 6.0;
const FONT_BODY: f32 = 8.5;
const FONT_HEAD: f32 = 9.0;
const FONT_TITLE: f32 = 13.0;

/// Interfeysdagi bilan bir xil shrift nomzodlari.
const FONTS: [&str; 4] = [
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\tahoma.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

/// Tizim shriftini o'qiydi.
fn font_bytes() -> Option<Vec<u8>> {
    FONTS.iter().find_map(|p| std::fs::read(p).ok())
}

/// Katakning PDF dagi matni.
fn text(c: &Cell) -> String {
    match c {
        Cell::Text(s) => s.clone(),
        Cell::Num(v) => crate::ui::materials::trim_num(*v),
        Cell::Money(v) => crate::ui::money(*v),
        Cell::Date(d) => d.format("%d.%m.%Y").to_string(),
        Cell::Empty => String::new(),
    }
}

/// Katak o'ngga tekislanadimi — son va pul o'ngda turadi.
fn right(c: &Cell) -> bool {
    matches!(c, Cell::Num(_) | Cell::Money(_))
}

/// Matnning taxminiy kengligi, millimetrda.
///
/// Aniq kenglik shrift metrikasidan chiqadi, lekin bu yerda u kerak emas:
/// ustun kengligini taqsimlash va matnni qisqartirish uchun taxmin yetarli.
/// Koeffitsiyent Segoe UI o'rtacha belgisiga qarab olingan.
fn width_of(s: &str, size: f32) -> f32 {
    s.chars().count() as f32 * size * 0.19
}

/// Ustun kengligiga sig'maydigan matnni qisqartiradi.
fn fit(s: &str, size: f32, max: f32) -> String {
    if width_of(s, size) <= max {
        return s.to_string();
    }
    let per = size * 0.19;
    let n = ((max / per) as usize).saturating_sub(1);
    if n == 0 {
        return String::new();
    }
    let cut: String = s.chars().take(n).collect();
    format!("{cut}…")
}

/// Ustun kengliklari: sarlavha va qiymatlarga qarab, jami sahifaga sig'adi.
fn columns(table: &Table) -> Vec<f32> {
    let n = table.headers.len();
    if n == 0 {
        return Vec::new();
    }
    // Har ustun uchun kerakli kenglik — eng uzun qiymat bo'yicha.
    let mut want: Vec<f32> = table
        .headers
        .iter()
        .map(|h| width_of(h, FONT_HEAD) + 4.0)
        .collect();
    for row in &table.rows {
        for (i, c) in row.iter().enumerate().take(n) {
            want[i] = want[i].max(width_of(&text(c), FONT_BODY) + 4.0);
        }
    }
    // Sahifaga sig'dirish: ortiqchasi eng keng ustunlardan proporsional olinadi.
    let avail = PAGE_W - 2.0 * MARGIN;
    let total: f32 = want.iter().sum();
    if total > avail {
        let k = avail / total;
        for w in want.iter_mut() {
            *w *= k;
        }
    }
    want
}

/// Bir sahifaga sig'adigan qator soni.
fn rows_per_page() -> usize {
    // Yuqorida sarlavha va jadval boshi, pastda kolontitul joyi.
    (((PAGE_H - 2.0 * MARGIN - 26.0) / ROW_H).floor() as usize).max(1)
}

/// Jadvalni PDF ga yozadi.
///
/// `subtitle` — obyekt nomi va sana kabi qo'shimcha qator; bo'sh bo'lsa
/// chiqmaydi.
pub fn write_table(path: &Path, table: &Table, subtitle: &str) -> Result<(), String> {
    let Some(bytes) = font_bytes() else {
        return Err(crate::i18n::t("pdf_no_font").to_string());
    };

    let (doc, first_page, first_layer) = PdfDocument::new(&table.name, Mm(PAGE_W), Mm(PAGE_H), "1");
    let font = doc
        .add_external_font(&bytes[..])
        .map_err(|e| format!("{e}"))?;

    let widths = columns(table);
    let per_page = rows_per_page();
    let pages = table.rows.len().div_ceil(per_page).max(1);

    for page in 0..pages {
        let layer = if page == 0 {
            doc.get_page(first_page).get_layer(first_layer)
        } else {
            let (p, l) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), format!("{}", page + 1));
            doc.get_page(p).get_layer(l)
        };
        let from = page * per_page;
        let to = (from + per_page).min(table.rows.len());
        draw_page(
            &layer,
            &font,
            table,
            &widths,
            &table.rows[from..to],
            subtitle,
            page + 1,
            pages,
        );
    }

    let file = std::fs::File::create(path).map_err(|e| format!("{e}"))?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| format!("{e}"))
}

/// Bitta sahifa: sarlavha, jadval boshi, qatorlar va kolontitul.
#[allow(clippy::too_many_arguments)]
fn draw_page(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    table: &Table,
    widths: &[f32],
    rows: &[Vec<Cell>],
    subtitle: &str,
    page: usize,
    pages: usize,
) {
    let mut y = PAGE_H - MARGIN - 4.0;
    layer.use_text(&table.name, FONT_TITLE, Mm(MARGIN), Mm(y), font);
    if !subtitle.is_empty() {
        y -= 5.0;
        layer.use_text(subtitle, FONT_BODY, Mm(MARGIN), Mm(y), font);
    }
    y -= 8.0;

    // Jadval boshi.
    let mut x = MARGIN;
    for (i, h) in table.headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(20.0);
        layer.use_text(fit(h, FONT_HEAD, w), FONT_HEAD, Mm(x + 1.0), Mm(y), font);
        x += w;
    }
    y -= 1.5;
    rule(layer, y);
    y -= ROW_H;

    for row in rows {
        let mut x = MARGIN;
        for (i, c) in row.iter().enumerate() {
            let w = widths.get(i).copied().unwrap_or(20.0);
            let s = fit(&text(c), FONT_BODY, w - 2.0);
            // Son o'ngda turadi: ustunda raqamlar bir chiziqqa keladi.
            let tx = if right(c) {
                x + w - 1.0 - width_of(&s, FONT_BODY)
            } else {
                x + 1.0
            };
            layer.use_text(s, FONT_BODY, Mm(tx), Mm(y), font);
            x += w;
        }
        y -= ROW_H;
    }

    // Kolontitul: sahifa raqami va jami qator soni.
    let foot = format!(
        "{} {page} / {pages} · {} {}",
        crate::i18n::t("pdf_page"),
        crate::i18n::t("pdf_rows"),
        table.rows.len()
    );
    layer.use_text(foot, 7.5, Mm(MARGIN), Mm(MARGIN - 4.0), font);
}

/// Jadval boshi ostidagi chiziq.
fn rule(layer: &PdfLayerReference, y: f32) {
    layer.set_outline_thickness(0.4);
    layer.add_line(Line {
        points: vec![
            (Point::new(Mm(MARGIN), Mm(y)), false),
            (Point::new(Mm(PAGE_W - MARGIN), Mm(y)), false),
        ],
        is_closed: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docgen::Cell;

    fn sample(rows: usize) -> Table {
        Table {
            name: "Sinov jadvali".into(),
            headers: vec!["Nomi".into(), "Hajm".into(), "Sana".into()],
            rows: (0..rows)
                .map(|i| {
                    vec![
                        Cell::Text(format!("Ish {i} — beton quyish, o'lchov · Бетон М300")),
                        Cell::Num(i as f64 * 1.5),
                        Cell::Date(chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()),
                    ]
                })
                .collect(),
        }
    }

    /// Ustunlar yig'indisi sahifa kengligidan oshmaydi — aks holda
    /// o'ng tomondagi ustun qog'ozdan tashqarida qolardi.
    #[test]
    fn columns_fit_the_page() {
        let w = columns(&sample(50));
        let total: f32 = w.iter().sum();
        assert!(total <= PAGE_W - 2.0 * MARGIN + 0.01, "jami {total}");
        assert_eq!(w.len(), 3);
        assert!(w.iter().all(|x| *x > 0.0));
    }

    /// Uzun matn ustunga sig'adigan holga keltiriladi.
    #[test]
    fn long_text_is_trimmed() {
        let s = fit("Juda uzun matn, ustunga sig'maydi", FONT_BODY, 10.0);
        assert!(width_of(&s, FONT_BODY) <= 10.5, "{s}");
        assert!(s.ends_with('…'));
        // Sig'adigan matn o'zgarmaydi.
        assert_eq!(fit("qisqa", FONT_BODY, 50.0), "qisqa");
    }

    /// Ko'p qatorli jadval bir necha sahifaga bo'linadi.
    #[test]
    fn many_rows_are_split_into_pages() {
        let per = rows_per_page();
        assert!(per > 10, "sahifaga {per} qator sig'yapti");
        let table = sample(per * 2 + 3);
        assert_eq!(table.rows.len().div_ceil(per), 3);
    }

    /// Fayl haqiqatda yoziladi va PDF sarlavhasi bilan boshlanadi.
    ///
    /// Shrift topilmasa sinov o'tkazib yuboriladi: bu muhit masalasi,
    /// kod xatosi emas.
    #[test]
    fn writes_a_real_pdf_file() {
        if font_bytes().is_none() {
            return;
        }
        let path = std::env::temp_dir().join(format!("qurai_pdf_{}.pdf", std::process::id()));
        let _ = std::fs::remove_file(&path);
        write_table(&path, &sample(120), "Obyekt · 10.04.2026").expect("PDF yozilmadi");

        let bytes = std::fs::read(&path).expect("fayl yo'q");
        assert!(bytes.starts_with(b"%PDF"), "PDF sarlavhasi yo'q");
        // Bo'sh fayl bo'lmasin: shrift ham ichida.
        assert!(bytes.len() > 20_000, "fayl juda kichik: {}", bytes.len());
        let _ = std::fs::remove_file(&path);
    }
}
