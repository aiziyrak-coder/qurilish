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

#[cfg(test)]
mod tests {
    use super::*;

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
