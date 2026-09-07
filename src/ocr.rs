//! Skan qilingan hujjatdan matnni tanish (TZ III.2, IV.11).
//!
//! Skanda matn yo'q — unda rasm bor. Rasmni matnga aylantirish (OCR) alohida
//! ish va uni ilova o'zi qilmaydi: buning uchun tayyor, sinovdan o'tgan
//! dasturlar bor. Shuning uchun bu modul **ko'prik**: kompyuterda o'rnatilgan
//! Tesseract topilsa, undan foydalanadi; topilmasa — jim qolmaydi, nima
//! o'rnatish kerakligini aniq aytadi.
//!
//! Ikkita qat'iy qoida:
//!
//! 1. **Tanilgan matn hech qachon «haqiqat» deb belgilanmaydi.** U qoralama
//!    sifatida keladi va foydalanuvchi tekshiradi. OCR xato o'qiydi — bu
//!    normal holat, uni yashirish esa xavfli.
//! 2. **Hech narsa tashqariga jo'natilmaydi.** Faqat shu kompyuterdagi
//!    dastur chaqiriladi, internetga chiqilmaydi.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Tesseract ishga tushirilishi mumkin bo'lgan joylar.
///
/// Windows da u odatda `PATH` da bo'lmaydi, shuning uchun standart
/// o'rnatish papkalari ham qaraladi.
const CANDIDATES: &[&str] = &[
    "tesseract",
    r"C:\Program Files\Tesseract-OCR\tesseract.exe",
    r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
    "/usr/bin/tesseract",
    "/usr/local/bin/tesseract",
];

/// PDF ni rasmga aylantiradigan dastur (Poppler tarkibida).
const PDF_TO_IMAGE: &[&str] = &[
    "pdftoppm",
    r"C:\Program Files\poppler\bin\pdftoppm.exe",
    "/usr/bin/pdftoppm",
];

/// Tanish uchun tillar: rus, o'zbek (kirill va lotin), ingliz.
///
/// Til paketi o'rnatilmagan bo'lsa Tesseract xato beradi va biz uni
/// foydalanuvchiga aynan ko'rsatamiz — o'zimiz «taxminan» o'qimaymiz.
const LANGS: &str = "rus+uzb+uzb_cyrl+eng";

/// Tesseract topilgan yo'l yoki `None`.
pub fn tesseract() -> Option<PathBuf> {
    CANDIDATES.iter().find_map(|c| runnable(c))
}

/// PDF ni rasmga aylantiruvchi topilgan yo'l yoki `None`.
pub fn pdf_to_image() -> Option<PathBuf> {
    PDF_TO_IMAGE.iter().find_map(|c| runnable(c))
}

/// Dastur ishga tushadimi: `--version` bilan tekshiriladi.
fn runnable(cmd: &str) -> Option<PathBuf> {
    let ok = Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    ok.then(|| PathBuf::from(cmd))
}

/// OCR mavjudligi haqidagi holat — sozlamalarda ko'rsatiladi.
pub fn status() -> &'static str {
    match (tesseract().is_some(), pdf_to_image().is_some()) {
        (true, true) => crate::i18n::t("ocr_ready"),
        (true, false) => crate::i18n::t("ocr_images_only"),
        _ => crate::i18n::t("ocr_missing"),
    }
}

/// Rasmdan matn o'qiydi.
pub fn image_text(path: &Path) -> Result<String, String> {
    let Some(exe) = tesseract() else {
        return Err(crate::i18n::t("ocr_missing").to_string());
    };
    let out = Command::new(exe)
        .arg(path)
        // `stdout` — natijani faylga emas, to'g'ridan-to'g'ri oqimga beradi.
        .arg("stdout")
        .arg("-l")
        .arg(LANGS)
        .output()
        .map_err(|e| format!("{e}"))?;
    if !out.status.success() {
        // Tesseract xatosini o'zgartirmasdan ko'rsatamiz: unda odatda
        // qaysi til paketi yetishmayotgani yozilgan bo'ladi.
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            crate::i18n::t("ocr_failed").to_string()
        } else {
            err
        });
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        return Err(crate::i18n::t("ocr_empty").to_string());
    }
    Ok(text)
}

/// Skan qilingan PDF dan matn o'qiydi.
///
/// PDF avval rasmga aylantiriladi (`pdftoppm`), keyin har bir sahifa
/// tanib olinadi. Oraliq fayllar vaqtinchalik papkada yaratiladi va
/// ishdan keyin o'chiriladi.
pub fn pdf_text(path: &Path) -> Result<String, String> {
    let Some(conv) = pdf_to_image() else {
        return Err(crate::i18n::t("ocr_no_pdftoppm").to_string());
    };
    if tesseract().is_none() {
        return Err(crate::i18n::t("ocr_missing").to_string());
    }

    let dir = std::env::temp_dir().join(format!("qurai_ocr_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let prefix = dir.join("page");

    let out = Command::new(conv)
        .arg("-r")
        // 300 nuqta/dyuym — matnni tanish uchun odatdagi eng kichik sifat.
        .arg("300")
        .arg("-png")
        .arg(path)
        .arg(&prefix)
        .output()
        .map_err(|e| format!("{e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let mut pages: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    pages.sort();

    let mut text = String::new();
    for page in &pages {
        match image_text(page) {
            Ok(t) => {
                text.push_str(&t);
                text.push('\n');
            }
            // Bitta sahifa o'qilmasa ham qolganini beramiz: yarim natija
            // hech narsadan yaxshi, lekin buni yashirmaymiz.
            Err(e) => {
                text.push_str(&format!("[{e}]\n"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    if text.trim().is_empty() {
        return Err(crate::i18n::t("ocr_empty").to_string());
    }
    Ok(text)
}

// ================================================================ Sertifikat

/// Skandan o'qilgan sertifikat maydonlari (TZ IV.11).
///
/// Har maydon **ixtiyoriy**: topilmagani bo'sh qoladi va bu ochiq
/// ko'rsatiladi. Topilmagan maydonni taxmin qilib to'ldirish eng yomon
/// yechim bo'lardi — sertifikat raqami xato bo'lsa, hujjat yaroqsiz.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Certificate {
    /// Sertifikat raqami.
    pub number: String,
    /// Berilgan sana.
    pub issued: Option<chrono::NaiveDate>,
    /// Amal qilish muddati.
    pub until: Option<chrono::NaiveDate>,
    /// Me'yoriy hujjat: GOST, O'z DSt, SNiP.
    pub standard: String,
    /// Ishlab chiqaruvchi.
    pub maker: String,
}

impl Certificate {
    /// Kamida bitta maydon topilganmi.
    pub fn found(&self) -> bool {
        !self.number.is_empty()
            || self.issued.is_some()
            || self.until.is_some()
            || !self.standard.is_empty()
            || !self.maker.is_empty()
    }
}

/// Raqam qidiriladigan so'zlar (o'zbekcha va ruscha).
const NUMBER_WORDS: [&str; 6] = [
    "sertifikat",
    "сертификат",
    "guvohnoma",
    "паспорт",
    "pasport",
    "protokol",
];

/// «Berilgan» ma'nosidagi so'zlar.
const ISSUED_WORDS: [&str; 5] = ["berilgan", "выдан", "выдано", "sana", "дата"];

/// «Amal qiladi» ma'nosidagi so'zlar.
const UNTIL_WORDS: [&str; 6] = [
    "amal qiladi",
    "действителен",
    "действительно",
    "срок",
    "muddat",
    "до",
];

/// Ishlab chiqaruvchi so'zlari.
const MAKER_WORDS: [&str; 5] = [
    "ishlab chiqaruvchi",
    "изготовитель",
    "производитель",
    "завод",
    "zavod",
];

/// Tanilgan matndan sertifikat maydonlarini ajratadi.
///
/// Bu «tushunish» emas, **qidirish**: har maydon o'ziga xos so'z yonidan
/// izlanadi. Shuning uchun natija qoralama sifatida beriladi va uni odam
/// tekshiradi.
pub fn certificate(text: &str) -> Certificate {
    let mut out = Certificate::default();

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let low = line.to_lowercase();

        // ---- Raqam: «Сертификат № 1234-56»
        if out.number.is_empty() && NUMBER_WORDS.iter().any(|w| low.contains(w)) {
            if let Some(num) = after_number_sign(line) {
                out.number = num;
            }
        }

        // ---- Me'yoriy hujjat
        if out.standard.is_empty() {
            if let Some(std) = standard_of(line) {
                out.standard = std;
            }
        }

        // ---- Ishlab chiqaruvchi: «Изготовитель: ...»
        if out.maker.is_empty() && MAKER_WORDS.iter().any(|w| low.contains(w)) {
            if let Some(rest) = line.split_once(':').map(|(_, r)| r.trim()) {
                if !rest.is_empty() {
                    out.maker = rest.to_string();
                }
            }
        }

        // ---- Sanalar: qaysi so'z yonida turgani muhim.
        let dates = dates_in(line);
        if dates.is_empty() {
            continue;
        }
        let has_until = UNTIL_WORDS.iter().any(|w| low.contains(w));
        let has_issued = ISSUED_WORDS.iter().any(|w| low.contains(w));
        if has_until && out.until.is_none() {
            // Ikki sana bir qatorda bo'lsa, keyingisi — muddat.
            out.until = dates.last().copied();
            if has_issued && dates.len() > 1 && out.issued.is_none() {
                out.issued = dates.first().copied();
            }
        } else if has_issued && out.issued.is_none() {
            out.issued = dates.first().copied();
        }
    }

    // Sanalar tartibi teskari bo'lsa, ular almashtiriladi: sertifikat
    // berilishidan oldin tugay olmaydi.
    if let (Some(a), Some(b)) = (out.issued, out.until) {
        if b < a {
            out.issued = Some(b);
            out.until = Some(a);
        }
    }
    out
}

/// `№` yoki `#` dan keyingi raqamni oladi.
///
/// Sertifikat raqami ichida bo'shliq bo'lishi mumkin («ROSS RU.АГ99.H01234»),
/// shuning uchun qator oxirigacha olinadi va faqat ajratgichlarda
/// to'xtatiladi. Uzun matn raqam bo'la olmaydi — chegara qo'yilgan.
fn after_number_sign(line: &str) -> Option<String> {
    let pos = line.find(['№', '#'])?;
    let rest = line[pos..].trim_start_matches(['№', '#', ' ', ':']);
    // Vergul, qavs yoki «от/dan» so'zi raqamning oxiri.
    let mut value = rest;
    for stop in [",", "(", " от ", " ot ", " dan ", " berilgan"] {
        if let Some(i) = value.find(stop) {
            value = &value[..i];
        }
    }
    let value: String = value.trim().chars().take(40).collect();
    let value = value.trim_matches(['.', '-', '/', ' ']).to_string();
    // Raqamsiz matn — bu raqam emas, sarlavhaning davomi.
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    (!value.is_empty() && has_digit).then_some(value)
}

/// Me'yoriy hujjat belgisi: `GOST 12345-89`, `O'z DSt 1234:2020`.
fn standard_of(line: &str) -> Option<String> {
    const MARKS: [&str; 6] = ["ГОСТ", "GOST", "O'z DSt", "Oz DSt", "СНиП", "ШНК"];
    let upper = line.to_uppercase();
    let mark = MARKS.iter().find(|m| upper.contains(&m.to_uppercase()))?;
    let pos = upper.find(&mark.to_uppercase())?;
    let rest = &line[pos..];
    let value: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '.' | ':' | '\''))
        .collect();
    let value = value.trim().to_string();
    (value.len() > mark.len()).then_some(value)
}

/// Qatordagi barcha sanalar: `dd.mm.yyyy`, `dd/mm/yyyy`, `yyyy-mm-dd`.
fn dates_in(line: &str) -> Vec<chrono::NaiveDate> {
    let mut out = Vec::new();
    let bytes: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || matches!(bytes[i], '.' | '/' | '-'))
        {
            i += 1;
        }
        let token: String = bytes[start..i].iter().collect();
        if let Some(d) = parse_date(&token) {
            out.push(d);
        }
    }
    out
}

/// Bitta sana matnini o'qiydi.
fn parse_date(token: &str) -> Option<chrono::NaiveDate> {
    let token = token.trim_matches(['.', '-', '/']);
    for sep in ['.', '/', '-'] {
        let parts: Vec<&str> = token.split(sep).collect();
        if parts.len() != 3 {
            continue;
        }
        let nums: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
        if nums.len() != 3 {
            continue;
        }
        // `yyyy-mm-dd` yoki `dd.mm.yyyy` — qaysi qism yil ekani uzunligidan.
        let (y, m, d) = if parts[0].len() == 4 {
            (nums[0], nums[1], nums[2])
        } else {
            (nums[2], nums[1], nums[0])
        };
        // Ikki raqamli yil: 2000 yillar deb olinadi.
        let y = if y < 100 { 2000 + y } else { y };
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(y as i32, m, d) {
            return Some(date);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sertifikat maydonlari tanilgan matndan ajratiladi.
    #[test]
    fn certificate_fields_are_extracted() {
        let text = "\
СЕРТИФИКАТ СООТВЕТСТВИЯ № ROSS RU.АГ99.H01234\n\
Выдан 12.03.2026\n\
Действителен до 12.03.2028\n\
Продукция: цемент портландский ГОСТ 31108-2020\n\
Изготовитель: ООО \"Ohangaron sement\"\n";
        let c = certificate(text);
        assert!(c.found());
        assert_eq!(c.number, "ROSS RU.АГ99.H01234");
        assert_eq!(c.issued, chrono::NaiveDate::from_ymd_opt(2026, 3, 12));
        assert_eq!(c.until, chrono::NaiveDate::from_ymd_opt(2028, 3, 12));
        assert_eq!(c.standard, "ГОСТ 31108-2020");
        assert!(c.maker.contains("Ohangaron"), "{}", c.maker);
    }

    /// O'zbekcha matn ham o'qiladi.
    #[test]
    fn uzbek_certificate_is_read() {
        let text = "\
Muvofiqlik sertifikati № UZ-123/45\n\
Berilgan sana: 01.02.2026\n\
Amal qiladi: 01.02.2027\n\
O'z DSt 901:2019\n";
        let c = certificate(text);
        assert_eq!(c.number, "UZ-123/45");
        assert_eq!(c.issued, chrono::NaiveDate::from_ymd_opt(2026, 2, 1));
        assert_eq!(c.until, chrono::NaiveDate::from_ymd_opt(2027, 2, 1));
        assert!(c.standard.starts_with("O'z DSt"), "{}", c.standard);
    }

    /// Topilmagan maydon taxmin qilinmaydi.
    #[test]
    fn nothing_is_invented_when_the_text_has_no_fields() {
        let c = certificate("shunchaki matn, hech qanday sertifikat yo'q");
        assert!(!c.found());
        assert!(c.number.is_empty());
        assert!(c.issued.is_none());
        assert!(c.until.is_none());

        // Bo'sh matn ham xavfsiz.
        assert!(!certificate("").found());
    }

    /// Sana tartibi teskari yozilgan bo'lsa to'g'rilanadi: sertifikat
    /// berilishidan oldin tugay olmaydi.
    #[test]
    fn reversed_dates_are_swapped() {
        let text = "Выдан 10.10.2027\nДействителен до 10.10.2026\n";
        let c = certificate(text);
        assert_eq!(c.issued, chrono::NaiveDate::from_ymd_opt(2026, 10, 10));
        assert_eq!(c.until, chrono::NaiveDate::from_ymd_opt(2027, 10, 10));
    }

    /// Sana ko'rinishlari: nuqta, chiziq va ISO.
    #[test]
    fn date_formats_are_understood() {
        assert_eq!(
            parse_date("12.03.2026"),
            chrono::NaiveDate::from_ymd_opt(2026, 3, 12)
        );
        assert_eq!(
            parse_date("2026-03-12"),
            chrono::NaiveDate::from_ymd_opt(2026, 3, 12)
        );
        assert_eq!(
            parse_date("12/03/26"),
            chrono::NaiveDate::from_ymd_opt(2026, 3, 12)
        );
        // Mavjud bo'lmagan sana qabul qilinmaydi.
        assert_eq!(parse_date("32.13.2026"), None);
        assert_eq!(parse_date("shunchaki"), None);
    }

    /// Dastur o'rnatilmagan bo'lsa ham modul yiqilmaydi va sabab aytadi.
    #[test]
    fn missing_tools_are_reported_not_guessed() {
        // Holat qatori har doim bo'ladi.
        assert!(!status().is_empty());

        if tesseract().is_none() {
            let err = image_text(Path::new("yo-q.png")).unwrap_err();
            assert_eq!(err, crate::i18n::t("ocr_missing"));
        }
        if pdf_to_image().is_none() {
            let err = pdf_text(Path::new("yo-q.pdf")).unwrap_err();
            assert!(!err.is_empty());
        }
    }

    /// Yo'q dastur ishga tushmaydi — tekshiruv `false` qaytaradi.
    #[test]
    fn unknown_command_is_not_runnable() {
        assert!(runnable("qurai-yo-q-dastur-12345").is_none());
    }
}
