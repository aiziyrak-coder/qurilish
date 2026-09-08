//! Reja chizmasi: nuqtalarni tekislikda ko'rsatish (TZ VI.13, VIII.6,
//! XIII.5, XVI.40).
//!
//! Bu **3D ko'rinish emas va xarita ham emas.** Bu — nuqtalar rejasi:
//! nima qayerda turgani ko'rinadi, o'lchov chizig'i bilan. Shunday
//! qilinganining ikki sababi bor:
//!
//! 1. **Xarita tayllari internetdan keladi**, ilova esa butunlay lokal
//!    ishlashi kerak. Tayl yo'q joyda bo'sh kvadrat ko'rsatishdan ko'ra
//!    nuqtalarni o'lchov bilan chizgan foydaliroq.
//! 2. **3D geometriya alohida yadro talab qiladi.** IFC dan olinadigan
//!    narsa — elementning joylashuvi; shakli emas. Joylashuvni ko'rsatish
//!    halol, «model ko'rinishi» deb atash esa yo'q.
//!
//! Chizma **avtomatik moslashadi**: nuqtalar chegarasi topiladi va
//! maydonga sig'diriladi. Nuqta bitta bo'lsa masshtab ma'nosiz — bunda
//! shartli doira chiziladi.

use super::*;
use egui::{vec2, Stroke};

/// Chiziladigan bitta nuqta.
pub struct Dot {
    /// Metrdagi koordinata: o'ngga va yuqoriga.
    pub x: f64,
    pub y: f64,
    pub color: egui::Color32,
    pub label: String,
    /// Sichqoncha ustiga kelganda ko'rinadigan matn.
    pub hint: String,
}

/// Nuqtalar sig'ishi uchun chekkadan qoldiriladigan joy.
const PAD: f32 = 28.0;

/// Nuqta radiusi.
const DOT_R: f32 = 3.5;

/// Rejani chizadi va ustiga kelingan nuqtaning izohini ko'rsatadi.
///
/// `circle` — chizmaga qo'shimcha doira (geozona radiusi kabi), metrda.
pub fn draw(ui: &mut egui::Ui, dots: &[Dot], height: f32, circle: Option<f64>) {
    if dots.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(t("plan_empty"))
                    .color(theme::muted())
                    .size(13.0),
            );
        });
        return;
    }

    let width = ui.available_width().max(120.0);
    let height = height.max(120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::canvas());

    // --- Chegaralar: nuqtalar va (bo'lsa) doira sig'ishi kerak.
    let mut min_x = dots.iter().map(|d| d.x).fold(f64::INFINITY, f64::min);
    let mut max_x = dots.iter().map(|d| d.x).fold(f64::NEG_INFINITY, f64::max);
    let mut min_y = dots.iter().map(|d| d.y).fold(f64::INFINITY, f64::min);
    let mut max_y = dots.iter().map(|d| d.y).fold(f64::NEG_INFINITY, f64::max);
    if let Some(r) = circle {
        min_x = min_x.min(-r);
        max_x = max_x.max(r);
        min_y = min_y.min(-r);
        max_y = max_y.max(r);
    }
    // Bitta nuqta yoki bir chiziqdagi nuqtalar: masshtab bo'lmaydi,
    // shuning uchun atrofiga shartli joy qo'yiladi.
    if (max_x - min_x) < 1e-6 {
        min_x -= 5.0;
        max_x += 5.0;
    }
    if (max_y - min_y) < 1e-6 {
        min_y -= 5.0;
        max_y += 5.0;
    }

    let inner =
        egui::Rect::from_min_max(rect.min + vec2(PAD, PAD), rect.max - vec2(PAD, PAD + 14.0));
    let sx = (inner.width() as f64 / (max_x - min_x)).max(1e-9);
    let sy = (inner.height() as f64 / (max_y - min_y)).max(1e-9);
    // Bir xil masshtab: aks holda shakl cho'zilib, masofa yolg'on
    // ko'rinardi.
    let scale = sx.min(sy);
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    let to_screen = |x: f64, y: f64| -> egui::Pos2 {
        egui::pos2(
            inner.center().x + ((x - cx) * scale) as f32,
            // Ekranda y pastga o'sadi, rejada esa yuqoriga.
            inner.center().y - ((y - cy) * scale) as f32,
        )
    };

    // --- Doira (geozona).
    if let Some(r) = circle {
        painter.circle_stroke(
            to_screen(0.0, 0.0),
            (r * scale) as f32,
            Stroke::new(1.0_f32, theme::accent().gamma_multiply(0.6)),
        );
    }

    // --- Nuqtalar.
    let mut hovered: Option<&Dot> = None;
    for d in dots {
        let p = to_screen(d.x, d.y);
        painter.circle_filled(p, DOT_R, d.color);
        if let Some(pos) = response.hover_pos() {
            if pos.distance(p) < 8.0 {
                hovered = Some(d);
            }
        }
        if !d.label.is_empty() {
            painter.text(
                p + vec2(6.0, -2.0),
                egui::Align2::LEFT_CENTER,
                &d.label,
                egui::FontId::proportional(10.0),
                theme::muted(),
            );
        }
    }

    // --- O'lchov chizig'i: masshtabsiz reja aldamchi bo'lardi.
    let bar_m = nice_step((inner.width() as f64 / scale) / 4.0);
    let bar_px = (bar_m * scale) as f32;
    if bar_px > 8.0 && bar_px < inner.width() {
        let y = rect.max.y - 12.0;
        let x0 = rect.min.x + PAD;
        painter.line_segment(
            [egui::pos2(x0, y), egui::pos2(x0 + bar_px, y)],
            Stroke::new(1.5_f32, theme::muted()),
        );
        painter.text(
            egui::pos2(x0 + bar_px + 6.0, y),
            egui::Align2::LEFT_CENTER,
            crate::geo::meters(bar_m),
            egui::FontId::proportional(10.0),
            theme::muted(),
        );
    }

    if let Some(d) = hovered {
        response.on_hover_text(&d.hint);
    }
}

/// O'lchov chizig'i uchun qulay qadam: 1, 2, 5 va ularning o'nliklari.
fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let pow = 10f64.powf(raw.log10().floor());
    let n = raw / pow;
    let step = if n < 1.5 {
        1.0
    } else if n < 3.5 {
        2.0
    } else if n < 7.5 {
        5.0
    } else {
        10.0
    };
    step * pow
}

/// Koordinatani markazga nisbatan metrga o'giradi (TZ XIII.5, XVI.40).
///
/// Kichik maydonda (qurilish obyekti o'lchamida) yer sirtini tekis deb
/// olish mumkin: xato santimetrlarda qoladi.
pub fn to_meters(center: crate::geo::Point, p: crate::geo::Point) -> (f64, f64) {
    let east = crate::geo::distance_m(center, crate::geo::Point::new(center.lat, p.lon));
    let north = crate::geo::distance_m(center, crate::geo::Point::new(p.lat, center.lon));
    (
        if p.lon < center.lon { -east } else { east },
        if p.lat < center.lat { -north } else { north },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O'lchov chizig'i qulay songa yaxlitlanadi.
    #[test]
    fn the_scale_bar_uses_round_numbers() {
        assert_eq!(nice_step(1.2), 1.0);
        assert_eq!(nice_step(2.6), 2.0);
        assert_eq!(nice_step(4.0), 5.0);
        assert_eq!(nice_step(9.0), 10.0);
        assert_eq!(nice_step(120.0), 100.0);
        assert_eq!(nice_step(0.4), 0.5);
        // Buzuq qiymat chizmani yiqitmaydi.
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(nice_step(f64::NAN), 1.0);
        assert_eq!(nice_step(-5.0), 1.0);
    }

    /// Koordinata markazga nisbatan to'g'ri tomonga o'giriladi.
    #[test]
    fn coordinates_become_metres_in_the_right_direction() {
        let c = crate::geo::Point::new(41.2995, 69.2401);
        // Shimolga 0,001 daraja ≈ 111 m.
        let (e, n) = to_meters(c, crate::geo::Point::new(41.3005, 69.2401));
        assert!(e.abs() < 0.001, "{e}");
        assert!((n - 111.0).abs() < 2.0, "{n}");
        // Janubga — manfiy.
        let (_, n) = to_meters(c, crate::geo::Point::new(41.2985, 69.2401));
        assert!(n < 0.0, "{n}");
        // G'arbga — manfiy.
        let (e, _) = to_meters(c, crate::geo::Point::new(41.2995, 69.2391));
        assert!(e < 0.0, "{e}");
        // Markazning o'zi — nol.
        let (e, n) = to_meters(c, c);
        assert!(e.abs() < 1e-6 && n.abs() < 1e-6);
    }
}
