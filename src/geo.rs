//! Geolokatsiya: nuqta, masofa va obyekt geozonasi (TZ IX.9, XI.19, XIII.27,
//! XV.5, XVI.5).
//!
//! Ish stolidagi kompyuterda GPS yo'q — u yerda koordinata **hosil
//! qilinmaydi**. Koordinatani maydonchadagi odamning telefoni beradi:
//! server sahifasi brauzerdan so'raydi va yozuvga qo'shib yuboradi. Bu
//! yerdagi kod esa kelgan koordinata bilan ishlaydi: masofani hisoblaydi,
//! obyekt doirasiga tushdimi — aytadi.
//!
//! Bitta qat'iy qoida: **koordinata bo'lmasa hukm ham yo'q.** «Koordinata
//! yo'q» degani «uzoqda» degani emas — telefon eski bo'lishi, ichkarida
//! signal yo'qolishi, odam ruxsat bermasligi mumkin. Shuning uchun
//! [`Verdict::Unknown`] alohida holat.

/// Yer radiusi, metrda — masofa shu bilan hisoblanadi.
const EARTH_R: f64 = 6_371_008.8;

/// Xaritadagi nuqta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
    /// Telefon aytgan aniqlik, metrda. 0 — noma'lum.
    pub accuracy: f64,
}

impl Point {
    pub fn new(lat: f64, lon: f64) -> Self {
        Point {
            lat,
            lon,
            accuracy: 0.0,
        }
    }

    /// Koordinata umuman mumkinmi.
    ///
    /// Nol-nol — Atlantika okeanidagi nuqta; amalda u «bo'sh qiymat»
    /// degani, shuning uchun qabul qilinmaydi.
    pub fn valid(&self) -> bool {
        (-90.0..=90.0).contains(&self.lat)
            && (-180.0..=180.0).contains(&self.lon)
            && !(self.lat.abs() < 1e-9 && self.lon.abs() < 1e-9)
    }

    /// Yozuvda saqlanadigan ko'rinish: `lat,lon` yoki `lat,lon,aniqlik`.
    pub fn store(&self) -> String {
        if self.accuracy > 0.0 {
            format!("{:.6},{:.6},{:.0}", self.lat, self.lon, self.accuracy)
        } else {
            format!("{:.6},{:.6}", self.lat, self.lon)
        }
    }

    /// Odam o'qiydigan ko'rinish.
    pub fn label(&self) -> String {
        format!("{:.5}, {:.5}", self.lat, self.lon)
    }
}

/// Saqlangan matndan koordinatani o'qiydi.
///
/// Ajratgich sifatida vergul, nuqtali vergul va bo'shliq qabul qilinadi;
/// kasr belgisi nuqta bo'lishi kerak.
pub fn parse(s: &str) -> Option<Point> {
    let parts: Vec<&str> = s
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|p| !p.trim().is_empty())
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let lat: f64 = parts[0].trim().parse().ok()?;
    let lon: f64 = parts[1].trim().parse().ok()?;
    let accuracy: f64 = parts
        .get(2)
        .and_then(|a| a.trim().parse().ok())
        .unwrap_or(0.0);
    let p = Point {
        lat,
        lon,
        accuracy: accuracy.max(0.0),
    };
    p.valid().then_some(p)
}

/// Ikki nuqta orasidagi masofa, metrda (haversine).
///
/// Yer sharsimon deb olinadi: qurilish maydonchasi o'lchamida bu xato
/// bir metrdan kam, ya'ni GPS ning o'z xatosidan ancha kichik.
pub fn distance_m(a: Point, b: Point) -> f64 {
    let (lat1, lat2) = (a.lat.to_radians(), b.lat.to_radians());
    let dlat = (b.lat - a.lat).to_radians();
    let dlon = (b.lon - a.lon).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_R * h.sqrt().clamp(0.0, 1.0).asin()
}

/// Obyekt geozonasi: markaz va radius (TZ XV.5, XVI.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fence {
    pub center: Point,
    /// Radius, metrda.
    pub radius: f64,
}

/// Geozona uchun eng kichik radius: undan kichigi GPS xatosiga tushib
/// qoladi va har yozuvni «tashqarida» deb ko'rsatardi.
pub const MIN_RADIUS: f64 = 50.0;

impl Fence {
    /// Sozlamadagi `lat,lon,radius` matnidan o'qiydi.
    pub fn parse(s: &str) -> Option<Fence> {
        let center = parse(s)?;
        let radius = s
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter(|p| !p.trim().is_empty())
            .nth(2)
            .and_then(|r| r.trim().parse::<f64>().ok())
            .unwrap_or(MIN_RADIUS);
        Some(Fence {
            center: Point::new(center.lat, center.lon),
            radius: radius.max(MIN_RADIUS),
        })
    }

    pub fn store(&self) -> String {
        format!("{},{:.0}", self.center.store(), self.radius)
    }

    /// Nuqta doiraga tushdimi.
    ///
    /// Telefonning o'z aniqligi radiusga **qo'shiladi**: aniqlik 80 metr
    /// bo'lsa, 60 metrlik chetlanish hali xato emas — shu aniqlik ichida.
    pub fn check(&self, p: Point) -> Verdict {
        if !p.valid() {
            return Verdict::Unknown;
        }
        let d = distance_m(self.center, p);
        let allowed = self.radius + p.accuracy;
        if d <= allowed {
            Verdict::Inside { distance: d }
        } else {
            Verdict::Outside {
                distance: d,
                over: d - allowed,
            }
        }
    }
}

/// Geozona hukmi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    /// Koordinata yo'q yoki noto'g'ri — hukm chiqarilmaydi.
    Unknown,
    Inside {
        distance: f64,
    },
    Outside {
        distance: f64,
        /// Doiradan necha metr chiqib ketgani.
        over: f64,
    },
}

impl Verdict {
    pub fn outside(&self) -> bool {
        matches!(self, Verdict::Outside { .. })
    }

    /// Qisqa yozuv: «120 m · ichkarida».
    pub fn label(&self) -> String {
        match self {
            Verdict::Unknown => crate::i18n::t("geo_unknown").to_string(),
            Verdict::Inside { distance } => {
                format!("{} · {}", meters(*distance), crate::i18n::t("geo_inside"))
            }
            Verdict::Outside { distance, .. } => {
                format!("{} · {}", meters(*distance), crate::i18n::t("geo_outside"))
            }
        }
    }
}

/// Masofani odam o'qiydigan ko'rinishga keltiradi.
pub fn meters(d: f64) -> String {
    if d < 1000.0 {
        format!("{d:.0} m")
    } else {
        format!("{:.1} km", d / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Masofa ma'lum ikki nuqtada tekshiriladi.
    #[test]
    fn distance_matches_a_known_pair() {
        // Toshkent — Samarqand: ~270 km.
        let tashkent = Point::new(41.2995, 69.2401);
        let samarkand = Point::new(39.6270, 66.9750);
        let d = distance_m(tashkent, samarkand) / 1000.0;
        assert!((d - 270.0).abs() < 10.0, "{d} km");

        // Bir nuqtadan o'ziga — nol.
        assert!(distance_m(tashkent, tashkent) < 0.001);

        // 0,001 daraja kenglik ≈ 111 metr.
        let near = Point::new(41.2995 + 0.001, 69.2401);
        let d = distance_m(tashkent, near);
        assert!((d - 111.0).abs() < 2.0, "{d} m");
    }

    /// Nol-nol koordinata qabul qilinmaydi: u bo'sh qiymat.
    #[test]
    fn empty_coordinate_is_not_a_place() {
        assert!(!Point::new(0.0, 0.0).valid());
        assert!(parse("0,0").is_none());
        assert!(parse("").is_none());
        assert!(parse("41.3").is_none());
        assert!(parse("91,10").is_none(), "kenglik 90 dan katta");

        let p = parse("41.299500, 69.240100, 25").expect("o'qilishi kerak");
        assert_eq!(p.accuracy, 25.0);
        assert_eq!(p.store(), "41.299500,69.240100,25");
    }

    /// Telefonning aniqligi hisobga olinadi: aniqlik past bo'lsa
    /// chetlanish xato deb belgilanmaydi.
    #[test]
    fn accuracy_widens_the_fence() {
        let fence = Fence {
            center: Point::new(41.2995, 69.2401),
            radius: 100.0,
        };
        // 111 metr — chegaradan tashqarida.
        let mut p = Point::new(41.3005, 69.2401);
        assert!(fence.check(p).outside());

        // Xuddi shu nuqta, lekin telefon «aniqlik 50 m» degan — endi xato emas.
        p.accuracy = 50.0;
        assert!(!fence.check(p).outside());

        // Koordinatasiz hukm yo'q.
        assert_eq!(fence.check(Point::new(0.0, 0.0)), Verdict::Unknown);
    }

    /// Juda kichik radius qabul qilinmaydi — u GPS xatosidan kichik.
    #[test]
    fn fence_has_a_sane_minimum() {
        let f = Fence::parse("41.2995,69.2401,5").expect("o'qilishi kerak");
        assert_eq!(f.radius, MIN_RADIUS);
        // Radius yozilmasa ham standart qiymat bo'ladi.
        let f = Fence::parse("41.2995,69.2401").expect("o'qilishi kerak");
        assert_eq!(f.radius, MIN_RADIUS);
        assert_eq!(f.store(), "41.299500,69.240100,50");
        assert!(Fence::parse("").is_none());
    }
}
