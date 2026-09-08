//! Fotoning ichidagi ma'lumot — EXIF (TZ V.9, X.9, XI.9, XIV.14, XV.12).
//!
//! TZ da «fotolarni AI tahlili» deb yozilgan. Rasmning **mazmunini**
//! tanish (bu ustunmi, bu kaskami) haqiqatan ham tashqi model ishi va u
//! bu yerda yo'q. Lekin foto-nazoratning amaliy foydasi ko'p hollarda
//! mazmunda emas, **hujjatlilikda**: rasm o'sha kuni olinganmi, o'sha
//! joydami, tahrirlanmaganmi, avval boshqa dalilga qo'yilmaganmi.
//!
//! Bu ma'lumot rasmning o'zida turadi — EXIF blokida — va uni o'qish
//! uchun hech qanday tashqi xizmat kerak emas. Shu sababli bu yerda
//! aynan shu o'qiladi, natijasi [`crate::photocheck`] da hukmga aylanadi.
//!
//! O'qiladigan format — JPEG (telefon kamerasi shuni beradi). PNG va
//! boshqalarida EXIF odatda bo'lmaydi: bunda «ma'lumot yo'q» deymiz,
//! taxmin qilmaymiz.

use chrono::{NaiveDate, NaiveDateTime};

/// Fotodan o'qilgan ma'lumot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    /// Rasm olingan payt (EXIF `DateTimeOriginal`).
    pub taken: Option<NaiveDateTime>,
    /// Kamera koordinatasi.
    pub point: Option<crate::geo::Point>,
    /// Qurilma: ishlab chiqaruvchi va model.
    pub camera: String,
    /// Rasmni yaratgan yoki o'zgartirgan dastur (`Software`).
    pub software: String,
}

impl Meta {
    /// Bironta ham foydali maydon bormi.
    pub fn any(&self) -> bool {
        self.taken.is_some()
            || self.point.is_some()
            || !self.camera.is_empty()
            || !self.software.is_empty()
    }
}

/// Fayldan EXIF o'qiydi.
///
/// Fayl butunlay xotiraga olinmaydi: EXIF sarlavhada turadi, shuning
/// uchun boshidan cheklangan miqdorda o'qiladi. Katta rasm bo'lsa ham
/// ish tez bo'lishi kerak — jurnalda o'nlab foto bo'lishi mumkin.
pub fn read(path: &std::path::Path) -> Option<Meta> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    // 512 KB — telefon rasmidagi EXIF va eskiz uchun yetarli.
    let mut head = vec![0u8; 512 * 1024];
    let read = file.read(&mut head).ok()?;
    head.truncate(read);
    from_bytes(&head)
}

/// JPEG baytlaridan EXIF o'qiydi.
pub fn from_bytes(data: &[u8]) -> Option<Meta> {
    let exif = find_exif(data)?;
    parse_tiff(exif)
}

/// JPEG segmentlari orasidan `APP1` (Exif) blokini topadi.
fn find_exif(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None; // JPEG emas
    }
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            // Segmentlar oqimi buzilgan — davom etishdan ma'no yo'q.
            return None;
        }
        let marker = data[i + 1];
        // Boshlanish/tugash va to'ldirgichlar uzunliksiz keladi.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        if marker == 0xDA || marker == 0xD9 {
            return None; // rasm ma'lumoti boshlandi, EXIF endi bo'lmaydi
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if len < 2 || i + 2 + len > data.len() {
            return None;
        }
        let body = &data[i + 4..i + 2 + len];
        if marker == 0xE1 && body.len() > 6 && &body[..6] == b"Exif\0\0" {
            return Some(&body[6..]);
        }
        i += 2 + len;
    }
    None
}

/// Bayt tartibi bilan sonlarni o'qish uchun kichik yordamchi.
struct Tiff<'a> {
    data: &'a [u8],
    le: bool,
}

impl<'a> Tiff<'a> {
    fn u16(&self, at: usize) -> Option<u16> {
        let b = self.data.get(at..at + 2)?;
        Some(if self.le {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        })
    }

    fn u32(&self, at: usize) -> Option<u32> {
        let b = self.data.get(at..at + 4)?;
        Some(if self.le {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        })
    }
}

/// Bitta IFD yozuvi.
#[derive(Clone, Copy)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    /// Qiymat 4 baytga sig'sa — o'zi, aks holda uning joyi.
    value: u32,
    /// Qiymat yozuv ichida turibdimi.
    inline: bool,
}

/// Turning bir birligi necha bayt egallaydi.
fn type_size(kind: u16) -> u32 {
    match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 => 4,
        5 | 10 | 12 => 8,
        _ => 0,
    }
}

/// IFD ni o'qiydi. Yozuvlar soni cheklangan: buzuq fayl bizni
/// cheksiz aylanishga tortmasligi kerak.
fn read_ifd(t: &Tiff, at: usize) -> Vec<Entry> {
    let mut out = Vec::new();
    let Some(count) = t.u16(at) else {
        return out;
    };
    for i in 0..count.min(512) as usize {
        let e = at + 2 + i * 12;
        let (Some(tag), Some(kind), Some(count), Some(value)) =
            (t.u16(e), t.u16(e + 2), t.u32(e + 4), t.u32(e + 8))
        else {
            break;
        };
        let bytes = type_size(kind).saturating_mul(count);
        out.push(Entry {
            tag,
            kind,
            count,
            value: if bytes <= 4 { e as u32 + 8 } else { value },
            inline: bytes <= 4,
        });
    }
    out
}

/// ASCII qiymatni matn qilib oladi.
fn text(t: &Tiff, e: &Entry) -> String {
    if e.kind != 2 {
        return String::new();
    }
    let start = e.value as usize;
    let end = start.saturating_add(e.count as usize).min(t.data.len());
    let raw = t.data.get(start..end).unwrap_or(&[]);
    String::from_utf8_lossy(raw)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

/// RATIONAL (surat/maxraj) qiymatlarini o'qiydi.
fn rationals(t: &Tiff, e: &Entry) -> Vec<f64> {
    if e.kind != 5 && e.kind != 10 {
        return Vec::new();
    }
    (0..e.count as usize)
        .filter_map(|i| {
            let at = e.value as usize + i * 8;
            let num = t.u32(at)? as f64;
            let den = t.u32(at + 4)? as f64;
            (den != 0.0).then(|| num / den)
        })
        .collect()
}

/// EXIF sanasi: `YYYY:MM:DD HH:MM:SS`.
fn date_time(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S")
        .ok()
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
}

/// TIFF blokidan kerakli teglarni yig'adi.
fn parse_tiff(data: &[u8]) -> Option<Meta> {
    let le = match data.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let t = Tiff { data, le };
    if t.u16(2)? != 42 {
        return None;
    }
    let ifd0 = t.u32(4)? as usize;
    if ifd0 >= data.len() {
        return None;
    }

    let mut meta = Meta::default();
    let mut make = String::new();
    let mut model = String::new();
    let mut exif_ifd = 0usize;
    let mut gps_ifd = 0usize;

    for e in read_ifd(&t, ifd0) {
        match e.tag {
            0x010F => make = text(&t, &e),
            0x0110 => model = text(&t, &e),
            0x0131 => meta.software = text(&t, &e),
            // Sana IFD0 da ham uchraydi; asl sana bo'lmasa shu ishlatiladi.
            0x0132 => meta.taken = meta.taken.or_else(|| date_time(&text(&t, &e))),
            0x8769 => exif_ifd = ifd_offset(&t, &e),
            0x8825 => gps_ifd = ifd_offset(&t, &e),
            _ => {}
        }
    }

    if exif_ifd > 0 && exif_ifd < data.len() {
        for e in read_ifd(&t, exif_ifd) {
            // 0x9003 — DateTimeOriginal, rasm haqiqatan olingan payt.
            if e.tag == 0x9003 {
                if let Some(dt) = date_time(&text(&t, &e)) {
                    meta.taken = Some(dt);
                }
            }
        }
    }

    if gps_ifd > 0 && gps_ifd < data.len() {
        meta.point = parse_gps(&t, gps_ifd);
    }

    meta.camera = match (make.is_empty(), model.is_empty()) {
        (true, true) => String::new(),
        (true, false) => model,
        (false, true) => make,
        // Ba'zi qurilmalarda model ichida ishlab chiqaruvchi ham bor.
        (false, false) if model.to_lowercase().starts_with(&make.to_lowercase()) => model,
        _ => format!("{make} {model}"),
    };
    meta.any().then_some(meta)
}

/// Ichki IFD ning joyi — qiymat yozuv ichida bo'lsa ham to'g'ri o'qiladi.
fn ifd_offset(t: &Tiff, e: &Entry) -> usize {
    if e.inline {
        t.u32(e.value as usize).unwrap_or(0) as usize
    } else {
        e.value as usize
    }
}

/// GPS IFD dan koordinatani yig'adi.
fn parse_gps(t: &Tiff, at: usize) -> Option<crate::geo::Point> {
    let mut lat = None;
    let mut lon = None;
    let mut lat_ref = 'N';
    let mut lon_ref = 'E';
    let mut accuracy = 0.0f64;

    for e in read_ifd(t, at) {
        match e.tag {
            0x0001 => lat_ref = text(t, &e).chars().next().unwrap_or('N'),
            0x0002 => lat = dms(&rationals(t, &e)),
            0x0003 => lon_ref = text(t, &e).chars().next().unwrap_or('E'),
            0x0004 => lon = dms(&rationals(t, &e)),
            // GPSHPositioningError — telefon aytgan aniqlik, metrda.
            0x001F => accuracy = rationals(t, &e).first().copied().unwrap_or(0.0),
            _ => {}
        }
    }

    let (lat, lon) = (lat?, lon?);
    let p = crate::geo::Point {
        lat: if lat_ref == 'S' { -lat } else { lat },
        lon: if lon_ref == 'W' { -lon } else { lon },
        accuracy: accuracy.max(0.0),
    };
    p.valid().then_some(p)
}

/// Daraja-daqiqa-soniyani o'nlik darajaga aylantiradi.
fn dms(parts: &[f64]) -> Option<f64> {
    match parts {
        [d] => Some(*d),
        [d, m] => Some(d + m / 60.0),
        [d, m, s, ..] => Some(d + m / 60.0 + s / 3600.0),
        _ => None,
    }
}

/// EXIF sanasidan kun.
pub fn day(meta: &Meta) -> Option<NaiveDate> {
    meta.taken.map(|dt| dt.date())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sinov uchun EXIF li kichik JPEG yasaydi.
    ///
    /// Haqiqiy rasm kerak emas: bizni faqat sarlavha qiziqtiradi, shuning
    /// uchun SOI + APP1 + EOI yetarli. Shu bilan o'quvchi haqiqiy bayt
    /// tartibida sinaladi, «o'zim yozgan tuzilma» da emas.
    fn jpeg_with_exif(date: &str, gps: Option<(f64, f64)>, make: &str) -> Vec<u8> {
        // --- TIFF blokini yig'amiz (little-endian).
        let mut tiff: Vec<u8> = b"II".to_vec();
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD0 shu yerdan

        // Ma'lumotlar blokini keyinroq qo'shamiz; joyini oldindan bilish
        // uchun IFD o'lchamini sanaymiz.
        let entries = 2 + u16::from(gps.is_some()); // Make, ExifIFD, (GPS)
        let ifd0_size = 2 + entries as usize * 12 + 4;
        let mut heap_at = 8 + ifd0_size;

        let make_at = heap_at;
        heap_at += make.len() + 1;
        let exif_ifd_at = heap_at;
        // Exif IFD: 1 ta yozuv (DateTimeOriginal) + ma'lumot
        let exif_ifd_size = 2 + 12 + 4;
        let date_at = exif_ifd_at + exif_ifd_size;
        heap_at = date_at + date.len() + 1;
        let gps_ifd_at = heap_at;
        // GPS IFD: 4 ta yozuv + 2 ta 3-lik rational
        let gps_ifd_size = 2 + 4 * 12 + 4;
        let gps_data_at = gps_ifd_at + gps_ifd_size;

        let mut ifd0: Vec<u8> = Vec::new();
        ifd0.extend_from_slice(&entries.to_le_bytes());
        let entry = |tag: u16, kind: u16, count: u32, value: u32| {
            let mut e = Vec::new();
            e.extend_from_slice(&tag.to_le_bytes());
            e.extend_from_slice(&kind.to_le_bytes());
            e.extend_from_slice(&count.to_le_bytes());
            e.extend_from_slice(&value.to_le_bytes());
            e
        };
        ifd0.extend(entry(0x010F, 2, make.len() as u32 + 1, make_at as u32));
        ifd0.extend(entry(0x8769, 4, 1, exif_ifd_at as u32));
        if gps.is_some() {
            ifd0.extend(entry(0x8825, 4, 1, gps_ifd_at as u32));
        }
        ifd0.extend_from_slice(&0u32.to_le_bytes()); // keyingi IFD yo'q
        assert_eq!(ifd0.len(), ifd0_size);
        tiff.extend(ifd0);

        tiff.extend_from_slice(make.as_bytes());
        tiff.push(0);

        let mut exif_ifd: Vec<u8> = Vec::new();
        exif_ifd.extend_from_slice(&1u16.to_le_bytes());
        exif_ifd.extend(entry(0x9003, 2, date.len() as u32 + 1, date_at as u32));
        exif_ifd.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(exif_ifd.len(), exif_ifd_size);
        tiff.extend(exif_ifd);
        tiff.extend_from_slice(date.as_bytes());
        tiff.push(0);

        if let Some((lat, lon)) = gps {
            let mut gps_ifd: Vec<u8> = Vec::new();
            gps_ifd.extend_from_slice(&4u16.to_le_bytes());
            // Ref lar bitta harf — yozuv ichiga sig'adi.
            let r = |c: u8| {
                let mut v = [0u8; 4];
                v[0] = c;
                u32::from_le_bytes(v)
            };
            gps_ifd.extend(entry(0x0001, 2, 2, r(b'N')));
            gps_ifd.extend(entry(0x0002, 5, 3, gps_data_at as u32));
            gps_ifd.extend(entry(0x0003, 2, 2, r(b'E')));
            gps_ifd.extend(entry(0x0004, 5, 3, gps_data_at as u32 + 24));
            gps_ifd.extend_from_slice(&0u32.to_le_bytes());
            assert_eq!(gps_ifd.len(), gps_ifd_size);
            tiff.extend(gps_ifd);

            // Daraja, daqiqa, soniya — maxraj 1, 1 va 10000.
            let mut push_dms = |v: f64| {
                let deg = v.trunc();
                let min = ((v - deg) * 60.0).trunc();
                let sec = (v - deg - min / 60.0) * 3600.0;
                for (num, den) in [
                    (deg as u32, 1u32),
                    (min as u32, 1u32),
                    ((sec * 10_000.0).round() as u32, 10_000u32),
                ] {
                    tiff.extend_from_slice(&num.to_le_bytes());
                    tiff.extend_from_slice(&den.to_le_bytes());
                }
            };
            push_dms(lat);
            push_dms(lon);
        }

        // --- JPEG qobig'i.
        let mut jpeg: Vec<u8> = vec![0xFF, 0xD8];
        let mut app1: Vec<u8> = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        jpeg.extend_from_slice(&[0xFF, 0xE1]);
        jpeg.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        jpeg.extend_from_slice(&app1);
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        jpeg
    }

    /// Haqiqiy bayt tartibidagi JPEG dan sana, qurilma va koordinata o'qiladi.
    #[test]
    fn photo_tells_when_and_where_it_was_taken() {
        let bytes = jpeg_with_exif("2026:09:08 07:41:12", Some((41.2995, 69.2401)), "Samsung");
        let meta = from_bytes(&bytes).expect("EXIF o'qilishi kerak");

        assert_eq!(
            meta.taken.map(|d| d.to_string()),
            Some("2026-09-08 07:41:12".to_string())
        );
        assert_eq!(meta.camera, "Samsung");
        let p = meta.point.expect("koordinata");
        assert!((p.lat - 41.2995).abs() < 1e-4, "{}", p.lat);
        assert!((p.lon - 69.2401).abs() < 1e-4, "{}", p.lon);
        assert_eq!(day(&meta).map(|d| d.to_string()), Some("2026-09-08".into()));
    }

    /// Koordinatasiz foto ham o'qiladi — shunchaki joy yo'q.
    #[test]
    fn photo_without_gps_still_has_a_date() {
        let bytes = jpeg_with_exif("2026:01:02 10:00:00", None, "Xiaomi");
        let meta = from_bytes(&bytes).expect("EXIF");
        assert!(meta.point.is_none());
        assert!(meta.taken.is_some());
    }

    /// JPEG bo'lmagan yoki buzuq fayl xatoga olib kelmaydi — «yo'q» deydi.
    #[test]
    fn broken_file_is_not_a_crash() {
        assert!(from_bytes(&[]).is_none());
        assert!(from_bytes(b"not a jpeg at all").is_none());
        assert!(from_bytes(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x02]).is_none());
        // To'g'ri boshlangan, lekin yarmida uzilgan fayl.
        let mut bytes = jpeg_with_exif("2026:09:08 07:41:12", Some((41.3, 69.2)), "Samsung");
        bytes.truncate(bytes.len() / 2);
        let _ = from_bytes(&bytes); // panic bo'lmasa yetarli
    }

    /// Daraja-daqiqa-soniya o'nlik darajaga to'g'ri aylanadi.
    #[test]
    fn degrees_minutes_seconds_become_decimal() {
        assert_eq!(
            dms(&[41.0, 17.0, 58.2]).map(|v| (v * 1e4).round()),
            Some(412_995.0)
        );
        assert_eq!(dms(&[10.0]), Some(10.0));
        assert_eq!(dms(&[]), None);
    }
}
