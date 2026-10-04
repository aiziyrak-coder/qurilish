//! Loyihadan material hisobi — kalkulyatsiya (TZ II.12, II.13).
//!
//! Savol oddiy: **nimadan qancha ketadi.** Loyihada har konstruksiya bitta
//! dona uchun yozilgan: «Fundament Fm3» ning spetsifikatsiyasi bitta
//! poydevorga ketadigan armatura va betonni beradi, joylashuv sxemasidagi
//! jadval esa bunday poydevor to'rtta ekanini aytadi. Haqiqiy ehtiyoj —
//! birinchisini ikkinchisiga ko'paytirib, hammasini material bo'yicha
//! yig'ish.
//!
//! Hisob uch qadamda bajariladi va har biri alohida ko'rinadi:
//!
//! 1. **Jadval** — spetsifikatsiya varaqdagi joylashuvi bo'yicha o'qiladi:
//!    ustun sarlavha ostidagi matn sifatida aniqlanadi.
//! 2. **Qator** — armatura, prokat, beton yoki boshqa buyum deb taniladi.
//! 3. **Ko'paytirish** — yig'ma birlik soni va konstruksiya soni.
//!
//! Qoida qat'iy: **son o'ylab topilmaydi.** Konstruksiya soni topilmasa,
//! qator birga ko'paytiriladi va shunday deb belgilanadi — odam uni ko'radi
//! va sonni o'zi kiritadi. Jimgina taxmin qilingan ko'paytuvchi butun
//! kalkulyatsiyani ishonchsiz qilardi.

use crate::pdfread::Piece;
use serde::{Deserialize, Serialize};

/// Spetsifikatsiyaning bitta qatori — varaqda qanday yozilgan bo'lsa.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecRow {
    pub pos: String,
    pub designation: String,
    pub name: String,
    pub qty: String,
    pub mass: String,
    pub note: String,
}

/// Bitta spetsifikatsiya jadvali.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecTable {
    /// Sahifa raqami, 1 dan.
    pub page: usize,
    pub rows: Vec<SpecRow>,
}

// ============================================================ 1. Jadval

/// Ustun sarlavhasi turlari — tartibi varaqdagidek.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Col {
    Pos,
    Designation,
    Name,
    Qty,
    Mass,
    Note,
}

fn header_kind(text: &str) -> Option<Col> {
    let t = text.trim().to_lowercase();
    // `Macca` ba'zan lotin harflari bilan terilgan — ikkalasi ham olinadi.
    if t.starts_with("поз") {
        Some(Col::Pos)
    } else if t.starts_with("обознач") {
        Some(Col::Designation)
    } else if t.starts_with("наимен") {
        Some(Col::Name)
    } else if t.starts_with("кол") {
        Some(Col::Qty)
    } else if t.starts_with("масса") || t.starts_with("macca") || t.starts_with("площ") {
        Some(Col::Mass)
    } else if t.starts_with("примеч") {
        Some(Col::Note)
    } else {
        None
    }
}

/// Parchaning taxminiy kengligi.
fn width(p: &Piece) -> f64 {
    p.text.chars().count() as f64 * p.size * 0.5
}

/// Sahifadagi barcha spetsifikatsiya jadvallarini o'qiydi.
pub fn tables(pieces: &[Piece], page: usize) -> Vec<SpecTable> {
    let mut out = Vec::new();
    // Har «Поз.» parchasi — jadval boshlanishiga nomzod.
    let anchors: Vec<&Piece> = pieces
        .iter()
        .filter(|p| header_kind(&p.text) == Some(Col::Pos))
        .collect();

    for a in &anchors {
        let line = a.size;
        // Sarlavha qatori: «Поз.» bilan bir balandlikdagi sarlavha so'zlari.
        let mut cols: Vec<(Col, f64, f64)> = Vec::new();
        for p in pieces {
            if (p.y - a.y).abs() > line * 1.3 || p.x < a.x - line {
                continue;
            }
            let Some(kind) = header_kind(&p.text) else {
                continue;
            };
            // Juda uzoqdagi so'z boshqa jadvalniki.
            if p.x - a.x > line * 70.0 {
                continue;
            }
            if !cols.iter().any(|c| c.0 == kind) {
                cols.push((kind, p.x, p.x + width(p)));
            }
        }
        cols.sort_by(|x, y| x.1.total_cmp(&y.1));
        let has = |k: Col| cols.iter().any(|c| c.0 == k);
        if !has(Col::Name) || !has(Col::Qty) {
            continue;
        }

        let left = a.x - line * 1.5;
        let last = cols.last().map(|c| c.2).unwrap_or(a.x);
        let right = last + line * 7.0;
        let top = a.y - line * 1.4;

        // Sarlavha ostidagi parchalar. Pastki chegara — navbatdagi
        // jadval sarlavhasi (shu ustunda) yoki katta bo'shliq.
        let next_header = anchors
            .iter()
            .filter(|o| o.y < a.y - line && (o.x - a.x).abs() < line * 30.0)
            .map(|o| o.y)
            .fold(f64::MIN, f64::max);
        let mut body: Vec<&Piece> = pieces
            .iter()
            .filter(|p| p.y < top && p.x >= left && p.x <= right)
            .filter(|p| next_header == f64::MIN || p.y > next_header + line)
            .collect();
        body.sort_by(|x, y| y.y.total_cmp(&x.y).then(x.x.total_cmp(&y.x)));

        // Qatorlar: bir balandlikdagi parchalar. Katta bo'shliqdan keyin
        // jadval tugagan.
        let mut lines: Vec<Vec<&Piece>> = Vec::new();
        let mut prev_y = top;
        for p in body {
            let same = lines
                .last()
                .is_some_and(|l| (l[0].y - p.y).abs() <= line * 0.5);
            if same {
                if let Some(l) = lines.last_mut() {
                    l.push(p);
                }
                continue;
            }
            if prev_y - p.y > line * 7.0 {
                break;
            }
            prev_y = p.y;
            lines.push(vec![p]);
        }
        if lines.is_empty() {
            continue;
        }
        let _ = &lines;

        // Ustun chegaralari **ma'lumotdan** olinadi: sarlavha so'zi
        // ustunning o'rtasida turadi, ma'lumot esa undan chaproqda
        // boshlanishi mumkin. Shuning uchun ikki sarlavha orasidagi eng
        // keng bo'sh oraliq chegara deb olinadi.
        let spans: Vec<(f64, f64)> = lines
            .iter()
            .flatten()
            .map(|p| (p.x, p.x + width(p)))
            .collect();
        let mut bounds: Vec<f64> = Vec::new();
        for w in cols.windows(2) {
            let (lo, hi) = (w[0].1, w[1].1 + line * 0.5);
            bounds.push(widest_gap(&spans, lo, hi).unwrap_or((w[0].2 + w[1].1) / 2.0));
        }

        let mut rows = Vec::new();
        for l in &lines {
            let mut cells: Vec<String> = vec![String::new(); cols.len()];
            // Har katakdagi oxirgi parchaning o'ng cheti.
            let mut ends: Vec<f64> = vec![f64::MIN; cols.len()];
            let mut sorted: Vec<&&Piece> = l.iter().collect();
            sorted.sort_by(|x, y| x.x.total_cmp(&y.x));
            for p in sorted {
                let idx = bounds
                    .iter()
                    .filter(|b| p.x >= **b)
                    .count()
                    .min(cols.len() - 1);
                let cell = &mut cells[idx];
                let text = p.text.trim();
                // Chizma dasturi bitta so'zni bo'lib yozadi: `Фм` va `1`
                // alohida parcha. Orasi tor bo'lsa ular bitta so'z —
                // bo'sh joy qo'yilsa `Фм 1` belgi sifatida tanilmay qolardi.
                let tight = p.x - ends[idx] < p.size * 0.33;
                let spaced = cell.ends_with(' ') || p.text.starts_with(' ');
                let glue =
                    cell.is_empty() || text.starts_with(['.', ',', ')']) || (tight && !spaced);
                if !glue && !cell.ends_with(' ') {
                    cell.push(' ');
                }
                cell.push_str(text);
                if p.text.ends_with(' ') {
                    cell.push(' ');
                }
                ends[idx] = p.x + width(p);
            }
            let get = |k: Col| {
                cols.iter()
                    .position(|c| c.0 == k)
                    .map(|i| cells[i].trim().to_string())
                    .unwrap_or_default()
            };
            let row = SpecRow {
                pos: get(Col::Pos),
                designation: get(Col::Designation),
                name: get(Col::Name),
                qty: get(Col::Qty),
                mass: get(Col::Mass),
                note: get(Col::Note),
            };
            if row != SpecRow::default() {
                rows.push(row);
            }
        }
        // Jadval tagidagi umumiy izohlar va chizmadagi o'lcham yozuvlari
        // jadvalga tegishli emas. Belgisi: ketma-ket uchta qatorda na
        // pozitsiya, na miqdor, na massa bor. Sarlavha qatorlari
        // (`Сборочные единицы`, `Материалы`) ikkitadan ortiq ketma-ket
        // kelmaydi.
        let bare = |r: &SpecRow| {
            // Uzun matn pozitsiya ustuniga ham tushib qolishi mumkin —
            // u ham izoh.
            (r.pos.is_empty() || r.pos.chars().count() > 20)
                && r.qty.is_empty()
                && r.mass.is_empty()
                && number(&r.note).is_none()
        };
        let mut run = 0;
        let mut cut = rows.len();
        for (i, r) in rows.iter().enumerate() {
            if bare(r) {
                run += 1;
                if run == 3 {
                    cut = i + 1 - 3;
                    break;
                }
            } else {
                run = 0;
            }
        }
        rows.truncate(cut);
        while rows.last().is_some_and(bare) {
            rows.pop();
        }
        if !rows.is_empty() {
            out.push(SpecTable { page, rows });
        }
    }
    out
}

/// `[lo, hi]` oralig'ida hech bir parcha egallamagan eng keng bo'shliqning
/// o'rtasi.
fn widest_gap(spans: &[(f64, f64)], lo: f64, hi: f64) -> Option<f64> {
    let mut inside: Vec<(f64, f64)> = spans
        .iter()
        .filter(|s| s.1 > lo && s.0 < hi)
        .map(|s| (s.0.max(lo), s.1.min(hi)))
        .collect();
    inside.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<(f64, f64)> = None;
    let mut edge = lo;
    for (a, b) in inside {
        if a > edge {
            let gap = a - edge;
            if best.is_none_or(|g| gap > g.0) {
                best = Some((gap, (edge + a) / 2.0));
            }
        }
        edge = edge.max(b);
    }
    if hi > edge {
        let gap = hi - edge;
        if best.is_none_or(|g| gap > g.0) {
            best = Some((gap, (edge + hi) / 2.0));
        }
    }
    best.map(|g| g.1)
}

// ============================================================= 2. Qator

/// Material turi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Concrete,
    Rebar,
    Steel,
    Other,
}

/// Qatordan tanilgan material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub kind: Kind,
    /// Yig'ish kaliti: `Beton B20`, `Armatura ∅12 A-III`.
    pub material: String,
    pub unit: String,
    /// Bitta yig'ma birlikka to'g'ri keladigan miqdor.
    pub amount: f64,
}

/// Matndan sonni o'qiydi: `3.09`, `29,47`, `159.9кг.` → son.
pub fn number(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    let head: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let head = head.trim_end_matches('.');
    if head.is_empty() {
        return None;
    }
    head.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Armatura: `∅14 A-III L=2550`, `∅12A400, L= 1700`, `∅12 A-III м.п.`.
fn rebar(name: &str) -> Option<(String, String)> {
    let at = name.find(['∅', 'Ø', 'ø'])?;
    let rest = &name[at..];
    let mut chars = rest.chars();
    chars.next();
    let tail: String = chars.collect();
    let tail = tail.trim_start();
    let dia: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if dia.is_empty() {
        return None;
    }
    let after = tail[dia.len()..].trim_start();
    // Sinf: birinchi so'z — `A-III`, `A400`, `Bp-I`, `B-I`.
    let class: String = after
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if class.is_empty() || !class.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return None;
    }
    Some((dia, class))
}

/// Beton sinfi: `Бетон кл. В20(М250)W8` → `B20`.
fn concrete(name: &str) -> Option<String> {
    let low = name.to_lowercase();
    if !low.starts_with("бетон") {
        return None;
    }
    // `В` kirill ham, lotin ham bo'lishi mumkin.
    let at = name.char_indices().find(|(i, c)| {
        (*c == 'В' || *c == 'B')
            && name[*i + c.len_utf8()..]
                .trim_start()
                .chars()
                .next()
                .is_some_and(|n| n.is_ascii_digit())
    })?;
    let tail = name[at.0 + at.1.len_utf8()..].trim_start();
    let class: String = tail
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let class = class.replace(',', ".");
    let class = class.trim_end_matches('.');
    (!class.is_empty()).then(|| format!("B{class}"))
}

/// Qatorni materialga aylantiradi. Tanilmasa — `None`.
pub fn item(row: &SpecRow) -> Option<Item> {
    let name = row.name.trim();
    if name.is_empty() {
        return None;
    }
    let qty = number(&row.qty);
    let mass = number(&row.mass);
    let note = number(&row.note);

    if let Some(class) = concrete(name) {
        // Hajm qaysi ustunda yozilgani loyihachiga qarab farq qiladi.
        let amount = qty.or(mass).or(note)?;
        return Some(Item {
            kind: Kind::Concrete,
            material: format!("Beton {class}"),
            unit: "m3".into(),
            amount,
        });
    }

    // Massa: «Примеч.» ustunida jami yoziladi; bo'lmasa — dona × birlik.
    let total = match (note, qty, mass) {
        (Some(n), _, Some(_)) => Some(n),
        (_, Some(q), Some(m)) => Some(q * m),
        _ => None,
    };

    let both = format!("{} {}", row.designation, name);
    if let Some((dia, class)) = rebar(&both) {
        return Some(Item {
            kind: Kind::Rebar,
            material: format!("Armatura ∅{dia} {class}"),
            unit: "kg".into(),
            amount: total?,
        });
    }

    // Prokat **nomidan** taniladi. «Обозначение» ustunida ko'pincha
    // «данный лист» («shu varaq») deb yoziladi — uni po'lat list deb olish
    // har bir yig'ma birlikni prokatga aylantirib yuborardi.
    let low = name.to_lowercase();
    if low.starts_with("лист") || low.contains("уголок") || low.contains("швеллер")
    {
        let label = if low.contains("уголок") {
            "Burchak (ugolok)"
        } else if low.contains("швеллер") {
            "Shveller"
        } else {
            "Po'lat list"
        };
        return Some(Item {
            kind: Kind::Steel,
            material: label.into(),
            unit: "kg".into(),
            amount: total?,
        });
    }

    // Boshqa buyum: miqdori va o'lchov birligi aniq yozilgan bo'lsa.
    let unit = unit_of(&row.note)?;
    Some(Item {
        kind: Kind::Other,
        material: name.to_string(),
        unit,
        amount: qty.or(number(&row.note))?,
    })
}

/// «Примеч.» ustunidagi o'lchov birligi: `шт.`, `п.м.`, `1050 м3`.
fn unit_of(note: &str) -> Option<String> {
    let low = note.to_lowercase().replace(' ', "");
    if low.contains("п.м") {
        Some("p.m".into())
    } else if low.contains("м3") || low.contains("м³") {
        Some("m3".into())
    } else if low.contains("м2") {
        Some("m2".into())
    } else if low.contains("шт") || low.contains("ш.т") {
        Some("dona".into())
    } else {
        None
    }
}

// ======================================================= 3. Ko'paytirish

/// Konstruksiya — joylashuv sxemasidagi qator: `Фм3 — 4 dona`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Construct {
    pub mark: String,
    pub name: String,
    pub count: f64,
    pub unit: String,
    pub page: usize,
}

/// Kalkulyatsiyaning bitta qatori: material qaysi konstruksiyadan va
/// qanday ko'paytuvchi bilan kelgani.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub item: Item,
    /// Konstruksiya yoki yig'ma birlik nomi — sarlavha qatoridan.
    pub group: String,
    /// Yig'ma birlik soni (`Арматурная сетка С1 — 2 шт`).
    pub sub: f64,
    /// Konstruksiya soni. `None` — topilmadi va birga ko'paytirildi.
    pub count: Option<f64>,
    pub page: usize,
}

impl Line {
    /// Jami miqdor: birlikka × yig'ma birlik × konstruksiya soni.
    pub fn total(&self) -> f64 {
        self.item.amount * self.sub * self.count.unwrap_or(1.0)
    }
}

/// Belgini solishtirish uchun soddalashtiradi: `К-1` va `К1` — bitta.
pub fn mark_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Jadvallardan konstruksiyalar ro'yxatini yig'adi.
///
/// Konstruksiya qatori: pozitsiyasida **belgi** bor (`Фм1`, `К2`), o'zi
/// material emas, miqdori esa dona yoki pog'on metrda.
pub fn constructs(tables: &[SpecTable]) -> Vec<Construct> {
    let mut out: Vec<Construct> = Vec::new();
    for t in tables {
        for r in &t.rows {
            let mark = r.pos.trim();
            let has_letter = mark.chars().any(|c| c.is_alphabetic());
            let has_digit = mark.chars().any(|c| c.is_ascii_digit());
            if !has_letter || !has_digit || mark.contains(' ') {
                continue;
            }
            // Armatura yoki prokat pozitsiyasi (`Ос-1`, `Хм-2`) — bu
            // konstruksiya emas, material.
            if item(r).is_some_and(|i| i.kind != Kind::Other) {
                continue;
            }
            let Some(count) = number(&r.qty) else {
                continue;
            };
            if count <= 0.0 {
                continue;
            }
            // Massasi yozilgan qator — yig'ma birlikka havola (`С4 — 4
            // dona, 2.15 kg`), konstruksiya emas.
            if number(&r.mass).is_some() {
                continue;
            }
            let key = mark_key(mark);
            if out.iter().any(|c| mark_key(&c.mark) == key) {
                continue;
            }
            out.push(Construct {
                mark: mark.to_string(),
                name: r.name.clone(),
                count,
                unit: unit_of(&r.note).unwrap_or_else(|| "dona".into()),
                page: t.page,
            });
        }
    }
    out
}

/// «(на 64 п.м)», «(за 2 шт.)» — jadval nechta birlik uchun yozilgani.
fn basis(name: &str) -> f64 {
    let low = name.to_lowercase();
    for key in ["(на ", "(за "] {
        if let Some(at) = low.find(key) {
            if let Some(n) = number(&low[at + key.len()..]) {
                if n > 0.0 {
                    return n;
                }
            }
        }
    }
    1.0
}

/// Sarlavha qatori qaysi konstruksiyaga tegishli.
fn construct_of<'a>(name: &str, list: &'a [Construct]) -> Option<&'a Construct> {
    let words: Vec<String> = name.split_whitespace().map(mark_key).collect();
    // Uzunroq belgi ustun: `Фм10` ni `Фм1` deb olmaslik uchun.
    let mut best: Option<&Construct> = None;
    for c in list {
        let key = mark_key(&c.mark);
        let hit =
            words.contains(&key) || words.windows(2).any(|w| format!("{}{}", w[0], w[1]) == key);
        if hit && best.is_none_or(|b| mark_key(&b.mark).len() < key.len()) {
            best = Some(c);
        }
    }
    best
}

/// Sarlavhaning belgisi: `Арматурная сетка С 1` → `с1`.
fn heading_keys(name: &str) -> Vec<String> {
    let words: Vec<String> = name.split_whitespace().map(mark_key).collect();
    let mut out = Vec::new();
    if let Some(last) = words.last() {
        out.push(last.clone());
    }
    if words.len() >= 2 {
        out.push(format!(
            "{}{}",
            words[words.len() - 2],
            words[words.len() - 1]
        ));
    }
    out
}

/// Yig'ma birlikka havola: konstruksiya ichida `С1 — 2 dona`.
struct Reference {
    page: usize,
    key: String,
    /// Jami soni: havoladagi son × konstruksiya soni. `None` — konstruksiya
    /// soni noma'lum.
    count: Option<f64>,
}

/// Qator yig'ma birlikka havolami: pozitsiyasi, soni va massasi bor,
/// lekin o'zi material emas.
fn is_reference(r: &SpecRow) -> bool {
    !r.pos.trim().is_empty()
        && number(&r.qty).is_some()
        && number(&r.mass).is_some()
        && item(r).is_none()
}

/// Barcha jadvallardan kalkulyatsiya qatorlarini tuzadi.
///
/// Loyihada tarkib ikki xil yoziladi va ikkalasi ham o'qiladi:
///
/// - **ichma-ich** — konstruksiya jadvalining o'zida: `Арматурная сетка
///   С1 — 2 шт`, keyin uning pozitsiyalari;
/// - **havola bilan** — konstruksiya jadvalida `С1 — 1 dona`, to'rning
///   tarkibi esa shu varaqdagi alohida jadvalda.
///
/// Ikkinchi holatda to'r tarkibi havoladagi songa va konstruksiya soniga
/// ko'paytiriladi. Havola topilmasa — birga ko'paytiriladi va belgilanadi.
pub fn lines(tables: &[SpecTable], list: &[Construct]) -> Vec<Line> {
    // ---- 1-o'tish: yig'ma birliklarga havolalar.
    let mut refs: Vec<Reference> = Vec::new();
    for t in tables {
        let mut count: Option<f64> = None;
        for r in &t.rows {
            if r.pos.trim().is_empty() {
                if let Some(c) = construct_of(&r.name, list) {
                    count = Some(c.count / basis(&r.name));
                }
                continue;
            }
            if is_reference(r) {
                let q = number(&r.qty).unwrap_or(0.0);
                refs.push(Reference {
                    page: t.page,
                    key: mark_key(&r.pos),
                    count: count.map(|c| c * q),
                });
            }
        }
    }
    let referenced = |page: usize, name: &str| -> Option<Option<f64>> {
        let keys = heading_keys(name);
        let hits: Vec<&Reference> = refs
            .iter()
            .filter(|r| r.page == page && keys.contains(&r.key))
            .collect();
        if hits.is_empty() {
            return None;
        }
        // Bittasida ham son noma'lum bo'lsa — jami ham noma'lum.
        let mut sum = 0.0;
        for h in &hits {
            match h.count {
                Some(c) => sum += c,
                None => return Some(None),
            }
        }
        Some(Some(sum))
    };

    // ---- 2-o'tish: material qatorlari.
    let mut out = Vec::new();
    for t in tables {
        let mut group = String::new();
        let mut count: Option<f64> = None;
        let mut per = 1.0;
        let mut sub = 1.0;
        for r in &t.rows {
            let it = item(r);
            let heading = r.pos.trim().is_empty()
                && !r.name.trim().is_empty()
                && it.as_ref().is_none_or(|i| i.kind == Kind::Other);
            if heading {
                if let Some(c) = construct_of(&r.name, list) {
                    // Yangi konstruksiya: ko'paytuvchilar qaytadan.
                    group = r.name.trim().to_string();
                    count = Some(c.count);
                    per = basis(&r.name);
                    sub = 1.0;
                } else if let Some(total) = referenced(t.page, &r.name) {
                    // Alohida jadvaldagi yig'ma birlik: soni havoladan.
                    group = r.name.trim().to_string();
                    count = total;
                    per = 1.0;
                    sub = 1.0;
                } else {
                    // Ichma-ich yig'ma birlik: `Арматурная сетка С1 — 2 шт`.
                    if group.is_empty() {
                        group = r.name.trim().to_string();
                    }
                    sub = number(&r.qty).filter(|q| *q > 0.0).unwrap_or(1.0);
                }
                continue;
            }
            // Konstruksiyaning o'zi haqidagi qator (`Фм1 — 20 шт`) va
            // yig'ma birlikka havola — material emas.
            let own = !r.pos.trim().is_empty()
                && list.iter().any(|c| mark_key(&c.mark) == mark_key(&r.pos))
                && it.as_ref().is_none_or(|i| i.kind == Kind::Other);
            if own || is_reference(r) {
                continue;
            }
            let Some(mut it) = it else { continue };
            // Jadval bir necha birlik uchun yozilgan bo'lsa — bittaga
            // keltiriladi.
            it.amount /= per;
            // Yig'ma birlik soni faqat uning **raqamlangan pozitsiyalariga**
            // tegishli. Beton va pozitsiyasiz qator konstruksiyaning o'ziga
            // yoziladi — uni to'r soniga ko'paytirish hajmni oshirib
            // yuborardi.
            let sub = if it.kind == Kind::Concrete || r.pos.trim().is_empty() {
                1.0
            } else {
                sub
            };
            out.push(Line {
                item: it,
                group: group.clone(),
                sub,
                count,
                page: t.page,
            });
        }
    }
    out
}

/// Material bo'yicha yig'ma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Total {
    pub kind: Kind,
    pub material: String,
    pub unit: String,
    pub amount: f64,
    /// Nechta qatordan yig'ilgan.
    pub lines: usize,
    /// Konstruksiya soni topilmagan qatorlar — ular birga ko'paytirilgan.
    pub unsure: usize,
}

pub fn totals(lines: &[Line]) -> Vec<Total> {
    let mut out: Vec<Total> = Vec::new();
    for l in lines {
        let unsure = usize::from(l.count.is_none());
        match out
            .iter_mut()
            .find(|t| t.material == l.item.material && t.unit == l.item.unit)
        {
            Some(t) => {
                t.amount += l.total();
                t.lines += 1;
                t.unsure += unsure;
            }
            None => out.push(Total {
                kind: l.item.kind,
                material: l.item.material.clone(),
                unit: l.item.unit.clone(),
                amount: l.total(),
                lines: 1,
                unsure,
            }),
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.material.cmp(&b.material)));
    out
}

/// Loyihadan olingan hisob — bazada saqlanadi, qayta o'qish shart emas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Takeoff {
    pub file: String,
    pub pages: usize,
    pub tables: Vec<SpecTable>,
    pub constructs: Vec<Construct>,
}

impl Takeoff {
    /// Sahifalar bo'yicha parchalardan hisobni tuzadi.
    pub fn read(file: &str, pages: &[Vec<Piece>]) -> Takeoff {
        let mut all = Vec::new();
        for (i, p) in pages.iter().enumerate() {
            all.extend(tables(p, i + 1));
        }
        let list = constructs(&all);
        Takeoff {
            file: file.to_string(),
            pages: pages.len(),
            tables: all,
            constructs: list,
        }
    }

    pub fn lines(&self) -> Vec<Line> {
        lines(&self.tables, &self.constructs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, text: &str) -> Piece {
        Piece {
            x,
            y,
            size: 10.0,
            text: text.to_string(),
        }
    }

    /// Haqiqiy loyihadagi joylashuv: sarlavha va uning ostidagi qatorlar.
    fn sample() -> Vec<Piece> {
        vec![
            p(0.0, 500.0, "Поз"),
            p(60.0, 500.0, "Обозначение"),
            p(180.0, 500.0, "Наименование"),
            p(275.0, 500.0, "Кол"),
            p(300.0, 500.0, "Масса"),
            p(330.0, 500.0, "Примеч"),
            // Sarlavha qatori: konstruksiya.
            p(180.0, 478.0, "Фундамент Фм3"),
            p(280.0, 478.0, "1"),
            p(340.0, 478.0, "шт"),
            // Yig'ma birlik — ikki dona.
            p(165.0, 462.0, "Арматурная сетка С 1"),
            p(280.0, 462.0, "2"),
            p(340.0, 462.0, "шт"),
            p(5.0, 446.0, "1"),
            p(58.0, 446.0, "ГОСТ 5781-82"),
            p(148.0, 446.0, "∅14 A-III"),
            p(244.0, 446.0, "L=2550"),
            p(278.0, 446.0, "22"),
            p(302.0, 446.0, "3.09"),
            p(337.0, 446.0, "68.0"),
            p(170.0, 430.0, "Бетон кл. В20(М250)W8"),
            p(337.0, 430.0, "3.66"),
        ]
    }

    #[test]
    fn a_table_is_read_by_position() {
        let t = tables(&sample(), 13);
        assert_eq!(t.len(), 1);
        let rows = &t[0].rows;
        assert_eq!(rows.len(), 4, "{rows:#?}");
        assert_eq!(rows[0].name, "Фундамент Фм3");
        assert_eq!(rows[0].qty, "1");
        // Nom ustuni sarlavhadan chaproqda boshlansa ham nomda qoladi,
        // uzunlik esa miqdor ustuniga o'tib ketmaydi.
        assert_eq!(rows[2].pos, "1");
        assert_eq!(rows[2].designation, "ГОСТ 5781-82");
        assert_eq!(rows[2].name, "∅14 A-III L=2550");
        assert_eq!(rows[2].qty, "22");
        assert_eq!(rows[2].mass, "3.09");
        assert_eq!(rows[2].note, "68.0");
    }

    #[test]
    fn rows_become_materials() {
        let rebar = SpecRow {
            pos: "1".into(),
            designation: "ГОСТ 5781-82".into(),
            name: "∅14 A-III L=2550".into(),
            qty: "22".into(),
            mass: "3.09".into(),
            note: "68.0".into(),
        };
        let it = item(&rebar).expect("armatura");
        assert_eq!(it.kind, Kind::Rebar);
        assert_eq!(it.material, "Armatura ∅14 A-III");
        assert_eq!(it.amount, 68.0);

        let beton = SpecRow {
            name: "Бетон кл. В20(М250)W8".into(),
            note: "3.66".into(),
            ..Default::default()
        };
        let it = item(&beton).expect("beton");
        assert_eq!(it.material, "Beton B20");
        assert_eq!(it.unit, "m3");
        assert_eq!(it.amount, 3.66);

        let b75 = SpecRow {
            name: "Бетон В7.5 м3".into(),
            note: "0.48".into(),
            ..Default::default()
        };
        assert_eq!(item(&b75).unwrap().material, "Beton B7.5");

        // Sarlavha qatori material emas.
        let head = SpecRow {
            name: "Анкер Ан-1".into(),
            ..Default::default()
        };
        assert!(item(&head).is_none());
    }

    /// Kalkulyatsiya: birlikka × yig'ma birlik × konstruksiya soni.
    #[test]
    fn amounts_are_multiplied_by_the_construct_count() {
        let mut all = tables(&sample(), 13);
        // Joylashuv sxemasidagi jadval: Фм3 — 4 dona.
        all.push(SpecTable {
            page: 10,
            rows: vec![SpecRow {
                pos: "Фм3".into(),
                name: "Фундамент монолитный Фм3".into(),
                qty: "4".into(),
                note: "шт.".into(),
                ..Default::default()
            }],
        });
        let list = constructs(&all);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].mark, "Фм3");
        assert_eq!(list[0].count, 4.0);

        let ls = lines(&all, &list);
        assert_eq!(ls.len(), 2, "{ls:#?}");
        // Armatura: 68 kg × 2 to'r × 4 poydevor = 544 kg.
        assert_eq!(ls[0].item.material, "Armatura ∅14 A-III");
        assert_eq!(ls[0].sub, 2.0);
        assert_eq!(ls[0].count, Some(4.0));
        assert_eq!(ls[0].total(), 544.0);

        let sums = totals(&ls);
        let beton = sums.iter().find(|t| t.material == "Beton B20").unwrap();
        // Beton to'rdan keyin yozilgan, lekin to'r soniga ko'paymaydi:
        // 3.66 m3 × 4 poydevor.
        assert_eq!(beton.unsure, 0);
        assert!((beton.amount - 14.64).abs() < 1e-9, "{}", beton.amount);
    }

    /// Konstruksiya soni topilmasa — birga ko'paytiriladi va belgilanadi.
    #[test]
    fn an_unknown_count_is_flagged_not_guessed() {
        let all = tables(&sample(), 13);
        let ls = lines(&all, &[]);
        assert!(!ls.is_empty());
        for l in &ls {
            assert_eq!(l.count, None);
        }
        let sums = totals(&ls);
        assert!(sums.iter().all(|t| t.unsure == t.lines));
    }

    /// Tarkib alohida jadvalda bo'lsa, soni havoladan olinadi.
    ///
    /// Haqiqiy loyihadagi holat: `Фм1` jadvalida `С2 — 2 dona`, to'rning
    /// armaturasi esa shu varaqdagi boshqa jadvalda. Poydevor 20 ta
    /// bo'lgani uchun to'r 40 ta.
    #[test]
    fn a_separate_assembly_table_is_multiplied_through_the_reference() {
        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            designation: String::new(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
        };
        let all = vec![
            SpecTable {
                page: 10,
                rows: vec![row("Фм1", "Фундамент монолитный Фм1", "20", "", "шт.")],
            },
            // To'rning tarkibi — alohida jadval.
            SpecTable {
                page: 11,
                rows: vec![
                    row("", "Арматурная сетка С 2", "", "", "13.7"),
                    row("3", "∅12A400, L= 1700", "5", "1.51", "7.5"),
                ],
            },
            // Konstruksiya jadvali: to'rga havola.
            SpecTable {
                page: 11,
                rows: vec![
                    row("", "Фм 1", "", "", ""),
                    row("С2", "Арматурная сетка С2", "2", "13.7", "27.4"),
                    row("", "Бетон кл. В 20(М250)W8", "7.2", "", "м³"),
                ],
            },
        ];
        let list = constructs(&all);
        assert_eq!(list.len(), 1, "{list:?}");
        let ls = lines(&all, &list);
        let rebar = ls.iter().find(|l| l.item.kind == Kind::Rebar).unwrap();
        // 7.5 kg × (2 to'r × 20 poydevor).
        assert_eq!(rebar.count, Some(40.0));
        assert_eq!(rebar.total(), 300.0);
        let beton = ls.iter().find(|l| l.item.kind == Kind::Concrete).unwrap();
        assert_eq!(beton.item.material, "Beton B20");
        assert_eq!(beton.total(), 144.0);
        // Havola qatorining o'zi ikkinchi marta sanalmaydi.
        assert_eq!(ls.len(), 2, "{ls:#?}");
    }

    #[test]
    fn marks_match_despite_dashes() {
        assert_eq!(mark_key("К-1"), mark_key("К1"));
        assert_eq!(mark_key("Фл-1"), "фл1");
        assert_eq!(basis("Фундамент Фл-1 (на 64 п.м)"), 64.0);
        assert_eq!(basis("Км-1 (за 2 шт.)"), 2.0);
        assert_eq!(basis("Фундамент Фм3"), 1.0);
        assert_eq!(number("159.9кг."), Some(159.9));
        assert_eq!(number("29,47"), Some(29.47));
        assert_eq!(number("шт."), None);
    }
}
