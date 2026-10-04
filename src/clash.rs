//! Geometriya bo'yicha kolliziya (TZ II, VII.31, XIV.14).
//!
//! Qoidalar bo'yicha kolliziya (`checks`) bog'lanishlarni tekshiradi:
//! deraza teshiksiz, quvur rigelni kesib o'tadi degan xulosalar u yerdan
//! chiqadi. Bu yerdagi hisob esa **o'lchamga** tayanadi: ikki elementning
//! qutisi kesishsa, ular bir joyni egallayapti.
//!
//! Uchta qat'iy qoida ishni halol tutadi:
//!
//! 1. **O'lchami yo'q element hisobga kirmaydi.** IFC dan shakli
//!    tanilmagan (BREP, CSG) element uchun quti qo'yilmaydi va u
//!    kolliziya ro'yxatida umuman ko'rinmaydi. «Taxminiy quti» butun
//!    hisobni yolg'on qilardi.
//! 2. **Quti o'qlar bo'yicha tekislangan** — burilish hisobga
//!    olinmaydi. Shuning uchun burilgan devorning qutisi haqiqiydan
//!    kattaroq bo'ladi va bu **ortiqcha topilma** beradi, tushib qolgan
//!    topilma emas. Ekranda shu ochiq aytiladi.
//! 3. **Tegib turish kolliziya emas.** Devor plitaga tegadi, ustun
//!    rigelga tegadi — bu loyihaning o'zi. Shuning uchun kesishish har
//!    uch o'q bo'yicha [`MIN_OVERLAP`] dan katta bo'lishi shart.

// Bu modul hozir ekrandan chaqirilmaydi: loyiha sahifasi kalkulyatsiyaga
// aylantirilgach, IFC/DXF yuklash tugmalari olib tashlandi. Kod va uning
// sinovlari saqlangan — qaytarish yoki o'chirish alohida qaror.
#![allow(dead_code)]

use crate::domain::{Element, ElementLink, Relation};

/// Kesishish shu qalinlikdan oshsagina kolliziya deb hisoblanadi, metrda.
///
/// Ikki santimetr — o'lchov va yaxlitlash xatosidan katta, lekin haqiqiy
/// urilishdan kichik.
pub const MIN_OVERLAP: f64 = 0.02;

/// Ro'yxatda ko'rsatiladigan eng ko'p topilma.
///
/// Katta modelda kesishish minglab bo'lishi mumkin va ularning hammasini
/// ko'rsatish ro'yxatni foydasiz qiladi. Eng kattalari muhim.
pub const MAX_FOUND: usize = 200;

/// Ikki elementning kesishishi.
#[derive(Debug, Clone, PartialEq)]
pub struct Clash {
    pub a: i64,
    pub b: i64,
    /// Har o'q bo'yicha kesishish qalinligi, metrda.
    pub overlap: [f64; 3],
}

impl Clash {
    /// Kesishgan hajm, m³ — topilmalarni saralash uchun.
    pub fn volume(&self) -> f64 {
        self.overlap[0] * self.overlap[1] * self.overlap[2]
    }

    /// Eng katta kesishish o'lchami, metrda — «qanchalik chuqur kirgan».
    pub fn depth(&self) -> f64 {
        self.overlap.iter().cloned().fold(0.0, f64::max)
    }
}

/// Ikki qutining kesishishi.
fn overlap(a: [f64; 6], b: [f64; 6]) -> Option<[f64; 3]> {
    let mut out = [0.0f64; 3];
    for i in 0..3 {
        let lo = a[i].max(b[i]);
        let hi = a[i + 3].min(b[i + 3]);
        let d = hi - lo;
        if d <= MIN_OVERLAP {
            return None;
        }
        out[i] = d;
    }
    Some(out)
}

/// Bog'lanish «shunday bo'lishi kerak» degani.
///
/// Devor ichidagi teshik, ustunga tayangan rigel, xonaga xizmat
/// qiladigan quvur — bularning kesishishi loyihaning o'zi.
///
/// `Crosses` esa **aynan muammo**: quvur rigelni kesib o'tyapti. Uni bu
/// ro'yxatga qo'shish geometrik hisobni ma'nosiz qilardi — u topishi
/// kerak bo'lgan narsani o'zi yashirib qo'yardi.
fn by_design(relation: Relation) -> bool {
    matches!(
        relation,
        Relation::Contains | Relation::Serves | Relation::SupportedBy | Relation::PoweredBy
    )
}

/// Elementlar orasidagi geometrik kolliziyalarni topadi.
///
/// Loyihaning o'zi ko'zda tutgan juftlar tekshirilmaydi (qarang
/// [`by_design`]).
pub fn find(elements: &[Element], links: &[ElementLink]) -> Vec<Clash> {
    // Bog'langan juftlar — ikki tomonlama.
    let mut linked: std::collections::BTreeSet<(i64, i64)> = Default::default();
    for l in links.iter().filter(|l| by_design(l.relation)) {
        linked.insert(pair(l.from_el, l.to_el));
    }

    // Har juftni tekshirish katta modelda ishlamaydi: 50 000 element —
    // milliarddan ortiq juft. Shuning uchun **supurish** ishlatiladi:
    // qutilar X bo'yicha tartiblanadi va faqat X oralig'i ustma-ust
    // tushganlari solishtiriladi. Natija aynan bir xil, ish esa
    // amaliyotda chiziqli.
    let mut boxed: Vec<(&Element, [f64; 6])> = elements
        .iter()
        .filter_map(|e| e.bbox.map(|b| (e, b)))
        .collect();
    boxed.sort_by(|x, y| {
        x.1[0]
            .partial_cmp(&y.1[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = Vec::new();
    // X bo'yicha hali tugamagan qutilar.
    let mut active: Vec<usize> = Vec::new();
    for i in 0..boxed.len() {
        let (a, ba) = boxed[i];
        // Chapda qolganlar endi kesisha olmaydi.
        active.retain(|&j| boxed[j].1[3] > ba[0] + MIN_OVERLAP);
        for &j in &active {
            let (b, bb) = boxed[j];
            if linked.contains(&pair(a.id, b.id)) {
                continue;
            }
            if let Some(o) = overlap(ba, bb) {
                // Tartib doim bir xil bo'lsin: juftlik qaysi tomondan
                // kelganiga qarab o'zgarmasin.
                let (first, second) = pair(a.id, b.id);
                out.push(Clash {
                    a: first,
                    b: second,
                    overlap: o,
                });
            }
        }
        active.push(i);
    }

    // Eng katta kesishish oldinda: ro'yxat qisqarganda muhimi qolsin.
    out.sort_by(|x, y| {
        y.volume()
            .partial_cmp(&x.volume())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(MAX_FOUND);
    out
}

/// Juftlikni tartibga soladi: (3,7) va (7,3) bir xil juft.
fn pair(a: i64, b: i64) -> (i64, i64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Nechta element o'lchamli — hisob qanchasini qamraganini ko'rsatish uchun.
///
/// Bu son **ekranda ko'rsatiladi**: «kolliziya topilmadi» degan xulosa
/// hamma element tekshirilganda va hech biri tekshirilmaganda bir xil
/// ko'rinmasligi kerak.
pub fn measured(elements: &[Element]) -> usize {
    elements.iter().filter(|e| e.bbox.is_some()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ElementKind, Relation};
    use crate::model::Section;

    fn el(id: i64, bbox: Option<[f64; 6]>) -> Element {
        Element {
            id,
            project_id: 1,
            section: Section::Kj,
            kind: ElementKind::Beam,
            mark: format!("E-{id}"),
            room: String::new(),
            axis: String::new(),
            level: "1".into(),
            size: 0.0,
            unit: String::new(),
            value: 0.0,
            value_name: String::new(),
            sheet: "IFC".into(),
            note: String::new(),
            pos: bbox.map(|b| [b[0], b[1], b[2]]),
            bbox,
        }
    }

    fn link(from: i64, to: i64) -> ElementLink {
        ElementLink {
            id: 0,
            from_el: from,
            to_el: to,
            relation: Relation::Contains,
        }
    }

    /// Kesishgan ikki quti topiladi, tegib turgani esa yo'q.
    #[test]
    fn touching_is_not_a_clash_but_overlapping_is() {
        // A: 0..1 kub. B: 0.5 dan boshlanadi — yarmi kirgan.
        let a = el(1, Some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        let b = el(2, Some([0.5, 0.0, 0.0, 1.5, 1.0, 1.0]));
        // C: aynan B ning yonida — tegib turibdi, kesishmaydi.
        let c = el(3, Some([1.5, 0.0, 0.0, 2.5, 1.0, 1.0]));

        let found = find(&[a.clone(), b.clone(), c.clone()], &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!((found[0].a, found[0].b), (1, 2));
        assert!((found[0].overlap[0] - 0.5).abs() < 1e-9);
        assert!((found[0].volume() - 0.5).abs() < 1e-9);
        assert!((found[0].depth() - 1.0).abs() < 1e-9);
    }

    /// O'lchami yo'q element hisobga kirmaydi.
    #[test]
    fn an_element_without_a_box_is_not_judged() {
        let a = el(1, Some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        let b = el(2, None);
        assert!(find(&[a, b], &[]).is_empty());
        assert_eq!(measured(&[el(1, Some([0.0; 6])), el(2, None)]), 1);
    }

    /// «Kesib o'tadi» bog'lanishi kolliziyani yashirmaydi.
    ///
    /// Bu bog'lanish muammoning **nomi**, uni ko'zda tutilgan deb
    /// hisoblash geometrik hisobni o'z vazifasidan mahrum qilardi.
    #[test]
    fn a_crossing_link_does_not_hide_the_clash() {
        let beam = el(1, Some([0.0, 0.0, 3.0, 6.0, 0.4, 3.6]));
        let pipe = el(2, Some([2.0, 0.0, 3.2, 2.2, 0.4, 3.4]));
        let found = find(
            &[beam, pipe],
            &[ElementLink {
                id: 0,
                from_el: 2,
                to_el: 1,
                relation: Relation::Crosses,
            }],
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }

    /// Bog'langan juft kolliziya emas: devor va uning teshigi.
    #[test]
    fn a_linked_pair_is_by_design() {
        let wall = el(1, Some([0.0, 0.0, 0.0, 4.0, 0.2, 3.0]));
        let opening = el(2, Some([1.0, -0.1, 0.0, 2.0, 0.3, 2.1]));
        // Bog'lanishsiz — kolliziya deb ko'rinadi.
        assert_eq!(find(&[wall.clone(), opening.clone()], &[]).len(), 1);
        // Bog'langach — yo'q. Yo'nalish ahamiyatsiz.
        assert!(find(&[wall.clone(), opening.clone()], &[link(2, 1)]).is_empty());
        assert!(find(&[wall, opening], &[link(1, 2)]).is_empty());
    }

    /// Topilmalar eng kattasidan boshlab keladi va soni cheklanadi.
    #[test]
    fn the_biggest_clash_comes_first() {
        let small = el(1, Some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        let tiny = el(2, Some([0.9, 0.0, 0.0, 1.9, 1.0, 1.0]));
        let big = el(3, Some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        let found = find(&[small, tiny, big], &[]);
        // 1↔3 to'liq ustma-ust (hajm 1), 1↔2 va 2↔3 esa 0,1.
        assert!(found[0].volume() > found[1].volume(), "{found:?}");
        assert_eq!((found[0].a, found[0].b), (1, 3));
    }

    /// Supurish usuli har juftni tekshirish bilan bir xil natija beradi.
    ///
    /// Tezlik uchun qilingan soddalashtirish natijani o'zgartirmasligi
    /// kerak — bu sinov aynan shuni tekshiradi va tasodifiy emas,
    /// takrorlanadigan ma'lumotda ishlaydi.
    #[test]
    fn the_sweep_finds_exactly_what_a_full_scan_would() {
        // Takrorlanadigan «tasodif»: oddiy chiziqli generator.
        let mut seed = 12_345u64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 33) % 1000) as f64 / 100.0
        };
        let elements: Vec<Element> = (1..=200)
            .map(|id| {
                let (x, y, z) = (next(), next(), next());
                let (dx, dy, dz) = (next() / 5.0 + 0.1, next() / 5.0 + 0.1, next() / 5.0 + 0.1);
                el(id, Some([x, y, z, x + dx, y + dy, z + dz]))
            })
            .collect();

        // To'g'ridan-to'g'ri hisob: har juft.
        let mut brute: Vec<(i64, i64)> = Vec::new();
        for (i, a) in elements.iter().enumerate() {
            for b in elements.iter().skip(i + 1) {
                if let (Some(ba), Some(bb)) = (a.bbox, b.bbox) {
                    if overlap(ba, bb).is_some() {
                        brute.push(pair(a.id, b.id));
                    }
                }
            }
        }
        brute.sort();

        let mut fast: Vec<(i64, i64)> = find(&elements, &[]).iter().map(|c| (c.a, c.b)).collect();
        fast.sort();

        assert!(!brute.is_empty(), "sinov ma'lumotida kesishish yo'q");
        assert_eq!(fast, brute, "supurish boshqa natija berdi");
    }

    /// Faqat bitta o'q bo'yicha kesishish kolliziya emas.
    #[test]
    fn crossing_in_one_axis_only_is_not_a_clash() {
        // Bir xil X va Y, lekin Z bo'yicha ajratilgan: ustma-ust turgan
        // ikki qavat plitasi.
        let lower = el(1, Some([0.0, 0.0, 0.0, 5.0, 5.0, 0.2]));
        let upper = el(2, Some([0.0, 0.0, 3.0, 5.0, 5.0, 3.2]));
        assert!(find(&[lower, upper], &[]).is_empty());
    }
}
