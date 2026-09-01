//! Rang palitrasi / Цветовая палитра.
//!
//! Standart mavzu — yorug'. Ranglar funksiya orqali beriladi, chunki mavzu
//! ish jarayonida almashtiriladi va konstantalar bunga yaramaydi.

use egui::Color32;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub const ALL: [Theme; 2] = [Theme::Light, Theme::Dark];

    pub fn code(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Theme {
        match s {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => crate::i18n::t("theme_light"),
            Theme::Dark => crate::i18n::t("theme_dark"),
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_theme(t: Theme) {
    CURRENT.store(if t == Theme::Dark { 1 } else { 0 }, Ordering::Relaxed);
}

pub fn theme() -> Theme {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Theme::Dark
    } else {
        Theme::Light
    }
}

/// Yorug' va qorong'i qiymatdan joriysini tanlaydi.
#[inline]
fn pick(light: Color32, dark: Color32) -> Color32 {
    match theme() {
        Theme::Light => light,
        Theme::Dark => dark,
    }
}

/// Sahifa foni.
pub fn bg() -> Color32 {
    pick(
        Color32::from_rgb(242, 244, 247),
        Color32::from_rgb(24, 26, 31),
    )
}

/// Yuqori va yon panel.
pub fn panel() -> Color32 {
    pick(
        Color32::from_rgb(255, 255, 255),
        Color32::from_rgb(31, 34, 41),
    )
}

/// Kartochka foni.
pub fn card() -> Color32 {
    pick(
        Color32::from_rgb(255, 255, 255),
        Color32::from_rgb(38, 42, 51),
    )
}

/// Chegara chiziqlari.
pub fn line() -> Color32 {
    pick(
        Color32::from_rgb(219, 224, 231),
        Color32::from_rgb(55, 60, 71),
    )
}

/// Asosiy matn.
pub fn text() -> Color32 {
    pick(
        Color32::from_rgb(27, 32, 39),
        Color32::from_rgb(226, 229, 235),
    )
}

/// Ikkilamchi matn.
pub fn muted() -> Color32 {
    pick(
        Color32::from_rgb(102, 112, 133),
        Color32::from_rgb(146, 154, 168),
    )
}

/// Urg'u rangi.
pub fn accent() -> Color32 {
    pick(
        Color32::from_rgb(37, 99, 235),
        Color32::from_rgb(96, 165, 250),
    )
}

pub fn ok() -> Color32 {
    pick(
        Color32::from_rgb(21, 128, 71),
        Color32::from_rgb(74, 190, 130),
    )
}

pub fn warn() -> Color32 {
    pick(
        Color32::from_rgb(176, 104, 0),
        Color32::from_rgb(226, 168, 62),
    )
}

pub fn danger() -> Color32 {
    pick(
        Color32::from_rgb(198, 40, 40),
        Color32::from_rgb(230, 96, 88),
    )
}

/// Diagramma maydonining foni.
pub fn canvas() -> Color32 {
    pick(
        Color32::from_rgb(255, 255, 255),
        Color32::from_rgb(20, 22, 26),
    )
}

/// Jadvaldagi toq qatorlar.
pub fn row_alt() -> Color32 {
    pick(
        Color32::from_rgb(247, 249, 251),
        Color32::from_rgb(26, 28, 34),
    )
}

/// Qatorlar orasidagi chiziq.
pub fn row_line() -> Color32 {
    pick(
        Color32::from_rgb(233, 237, 242),
        Color32::from_rgb(40, 44, 52),
    )
}

/// Dam olish kunlari foni.
pub fn weekend() -> Color32 {
    pick(
        Color32::from_rgb(243, 245, 249),
        Color32::from_rgb(28, 30, 36),
    )
}

/// Bog'lanish strelkalari.
pub fn arrow() -> Color32 {
    pick(
        Color32::from_rgb(140, 149, 163),
        Color32::from_rgb(105, 115, 132),
    )
}

/// Progress-polosaning bo'sh qismi.
pub fn track() -> Color32 {
    pick(
        Color32::from_rgb(230, 234, 240),
        Color32::from_rgb(45, 49, 58),
    )
}

/// Gant polosasidagi yozuv rangi.
pub fn bar_label() -> Color32 {
    pick(Color32::from_rgb(26, 31, 38), Color32::from_rgb(15, 17, 20))
}

/// Polosaning bajarilmagan qismi uchun shaffoflik.
pub fn bar_track_alpha() -> f32 {
    match theme() {
        Theme::Light => 0.20,
        Theme::Dark => 0.35,
    }
}

/// Polosaning bajarilgan qismi uchun shaffoflik.
pub fn bar_fill_alpha() -> f32 {
    match theme() {
        Theme::Light => 0.65,
        Theme::Dark => 1.0,
    }
}

/// Bo'limlarning ranglari yorug' fonda biroz to'qroq bo'lishi kerak.
pub fn section_color(base: Color32) -> Color32 {
    match theme() {
        Theme::Light => {
            // Yorug' fonda o'qilishi uchun 20% ga to'qlashtiramiz.
            let f = |c: u8| ((c as f32) * 0.8) as u8;
            Color32::from_rgb(f(base.r()), f(base.g()), f(base.b()))
        }
        Theme::Dark => base,
    }
}
