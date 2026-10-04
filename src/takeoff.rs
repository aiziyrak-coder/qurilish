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
    /// «Единица измерения» ustuni — uskunalar spetsifikatsiyasida bo'ladi.
    #[serde(default)]
    pub unit: String,
}

/// Bitta spetsifikatsiya jadvali.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecTable {
    /// Sahifa raqami, 1 dan.
    pub page: usize,
    pub rows: Vec<SpecRow>,
    /// Jadval hisobdan chiqarilganmi.
    ///
    /// Loyihada bir necha blok bo'lsa yoki jadval boshqa obyektga
    /// tegishli bo'lsa, odam uni o'chirib qo'yadi. Dastur buni o'zi hal
    /// qilmaydi: qaysi varaq qaysi blokka tegishli ekani jadvalda
    /// yozilmagan.
    #[serde(default)]
    pub off: bool,
    /// Varaq shtampida nomi yozilgan konstruksiya belgisi (`К3`).
    ///
    /// Ba'zi varaqlar butunlay bitta konstruksiyaga bag'ishlangan va
    /// jadval ichida uning nomi takrorlanmaydi. Bunday jadval shtampdagi
    /// belgi orqali konstruksiya soniga ko'paytiriladi. Bo'sh — egasi
    /// aniqlanmagan.
    #[serde(default)]
    pub owner: String,
    /// Jadvalni AI o'qiganmi (aks holda — joylashuv qoidasi bo'yicha).
    #[serde(default)]
    pub ai: bool,
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
    Unit,
}

fn header_kind(text: &str) -> Option<Col> {
    let t = text.trim().to_lowercase();
    // `Macca` ba'zan lotin harflari bilan terilgan — ikkalasi ham olinadi.
    // Aniq shakl: chizmadagi izoh ham «поз.1 - Строповочные петли» deb
    // boshlanadi va uni sarlavha deb olish jadvalni o'rtasidan kesardi.
    if matches!(t.as_str(), "поз" | "поз." | "позиция" | "поз.,") {
        Some(Col::Pos)
    } else if t.starts_with("обознач") {
        Some(Col::Designation)
    } else if t.starts_with("наимен") {
        Some(Col::Name)
    } else if (t == "кол"
        || t.starts_with("кол.")
        || t.starts_with("кол-во")
        || t.starts_with("колич")
        || t == "коли-")
        && !t.contains("уч")
    {
        // Aniq shakllar: «Колонна» ham «кол» bilan boshlanadi va uni
        // ustun sarlavhasi deb olish butun jadvalni siljitib yuborardi.
        // «Кол.уч.» esa shtampdagi yozuv.
        Some(Col::Qty)
    } else if t.starts_with("масса") || t.starts_with("macca") || t.starts_with("площ") {
        Some(Col::Mass)
    } else if t.starts_with("примеч") {
        Some(Col::Note)
    } else if t.starts_with("единица") || t.starts_with("ед. изм") || t.starts_with("ед.изм")
    {
        // Faqat to'liq yozilgani: `Масса ед.` dagi «ед» alohida parcha
        // bo'lib keladi va uni ustun deb olish jadvalni buzardi.
        Some(Col::Unit)
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
    // Har yo'nalishdagi matn alohida ko'riladi: jadval chizmadan boshqa
    // tomonga yozilgan bo'lishi mumkin.
    let mut out = Vec::new();
    for dir in 0..4u8 {
        let part: Vec<Piece> = pieces.iter().filter(|p| p.dir == dir).cloned().collect();
        if !part.is_empty() {
            out.extend(tables_in(&part, page));
            out.extend(steel_tables(&part, page));
        }
    }
    out
}

fn tables_in(pieces: &[Piece], page: usize) -> Vec<SpecTable> {
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
        let mut masses: Vec<(f64, f64)> = Vec::new();
        for p in pieces {
            // Sarlavha katagi ikki-uch qatorli bo'lishi mumkin («Коли-
            // чество», «Масса единицы кг»), shuning uchun so'zlar bir
            // necha qator balandligida qidiriladi.
            if (p.y - a.y).abs() > line * 2.6 || p.x < a.x - line {
                continue;
            }
            let Some(kind) = header_kind(&p.text) else {
                continue;
            };
            // Juda uzoqdagi so'z boshqa jadvalniki. Uskunalar
            // spetsifikatsiyasi butun varaq kengligida bo'ladi, shuning
            // uchun chegara keng.
            if p.x - a.x > line * 110.0 {
                continue;
            }
            if !cols.iter().any(|c| c.0 == kind) {
                cols.push((kind, p.x, p.x + width(p)));
            } else if kind == Col::Mass {
                masses.push((p.x, p.x + width(p)));
            }
        }
        // Ba'zi jadvallarda «Примеч.» o'rnida ikkinchi «Масса» turadi:
        // birinchisi — birlik massasi, ikkinchisi — jami. Jami massa
        // boshqa jadvallarda «Примеч.» ustunida yoziladi, shuning uchun
        // u shu ustun sifatida o'qiladi.
        if !cols.iter().any(|c| c.0 == Col::Note) {
            let first = cols.iter().find(|c| c.0 == Col::Mass).map(|c| c.1);
            if let Some(m) = first.and_then(|f| masses.iter().find(|m| m.0 > f + line * 2.0)) {
                cols.push((Col::Note, m.0, m.1));
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
            // Sarlavhalar orasidagi o'rta nuqta — taxminiy chegara.
            let guess = (w[0].2 + w[1].1) / 2.0;
            bounds.push(nearest_gap(&spans, lo, hi, guess, line * 0.8).unwrap_or(guess));
        }

        let mut rows = Vec::new();
        for l in &lines {
            let mut cells: Vec<String> = vec![String::new(); cols.len()];
            // Har katakdagi oxirgi parchaning o'ng cheti.
            let mut ends: Vec<f64> = vec![f64::MIN; cols.len()];
            let mut sorted: Vec<&&Piece> = l.iter().collect();
            sorted.sort_by(|x, y| x.x.total_cmp(&y.x));
            // Pozitsiyasiz qator — sarlavha yoki izoh. U bir necha ustun
            // ustidan yozilishi mumkin (`Монолитная перемычка Пм-1 (на 1
            // п.м.)`): parchalar uzluksiz davom etsa, ular chegaradan
            // o'tsa ham bitta katakda qoladi. Ikki son yonma-yon kelsa —
            // bu ikki ustun, ular birlashtirilmaydi.
            let has_pos = bounds
                .first()
                .is_some_and(|b| sorted.first().is_some_and(|p| p.x < *b));
            let mut prev: Option<(usize, f64, f64, bool)> = None;
            let mut carried = false;
            for p in sorted {
                let mut idx = bounds
                    .iter()
                    .filter(|b| p.x >= **b)
                    .count()
                    .min(cols.len() - 1);
                if let (false, Some((was, start, end, digit))) = (has_pos, prev) {
                    let gap = p.x - end;
                    let slack = (end - start) * 0.3 + p.size;
                    let numeric = p
                        .text
                        .trim_start()
                        .starts_with(|c: char| c.is_ascii_digit());
                    if gap < p.size * 0.33 && gap > -slack && !(digit && numeric) && idx != was {
                        idx = was;
                        carried = true;
                    }
                }
                prev = Some((
                    idx,
                    p.x,
                    p.x + width(p),
                    p.text.trim_end().ends_with(|c: char| c.is_ascii_digit()),
                ));
                let cell = &mut cells[idx];
                // `∅` (U+2205) ko'p shriftlarda yo'q va katakcha bo'lib
                // chiqadi; `Ø` hamma joyda bor va ma'nosi bir xil.
                let shown = p.text.replace('∅', "Ø");
                let text = shown.trim();
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
            let mut row = SpecRow {
                pos: get(Col::Pos),
                designation: get(Col::Designation),
                name: get(Col::Name),
                qty: get(Col::Qty),
                mass: get(Col::Mass),
                note: get(Col::Note),
                unit: get(Col::Unit),
            };
            // Yoyilgan matn «Обозначение» ustunidan boshlangan bo'lsa ham
            // u nom: belgi ustuniga bunday uzun yozuv sig'maydi.
            if carried && row.name.is_empty() && !row.designation.is_empty() {
                row.name = std::mem::take(&mut row.designation);
            }
            // Sarlavha ostidagi ustun raqamlari qatori: `1 2 3 4 5`.
            let filled: Vec<&String> = [
                &row.pos,
                &row.designation,
                &row.name,
                &row.qty,
                &row.mass,
                &row.note,
                &row.unit,
            ]
            .into_iter()
            .filter(|c| !c.is_empty())
            .collect();
            let numbering = filled.len() >= 3
                && filled
                    .iter()
                    .all(|c| c.chars().all(|ch| ch.is_ascii_digit() || ch == ' '));
            if row != SpecRow::default() && !numbering {
                rows.push(row);
            }
        }
        // Katakdagi matn raqamdan sal yuqoriroq yozilgan bo'lsa, u alohida
        // qator bo'lib chiqadi: avval nom, keyin pozitsiya va miqdor.
        // Bunday juftlik bitta qator.
        let mut i = 0;
        while i + 1 < rows.len() {
            let (a, b) = (&rows[i], &rows[i + 1]);
            let name_only = a.pos.is_empty()
                && !a.name.is_empty()
                && a.qty.is_empty()
                && a.mass.is_empty()
                && a.note.is_empty();
            if name_only && !b.pos.is_empty() && b.name.is_empty() && !b.qty.is_empty() {
                let a = rows.remove(i);
                let b = &mut rows[i];
                b.name = a.name;
                if b.designation.is_empty() {
                    b.designation = a.designation;
                }
            }
            i += 1;
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
                // Miqdori nomning ichida yozilgan qator — izoh emas.
                && item(r).is_none()
        };
        // Jadval boshida bir necha sarlavha ketma-ket kelishi mumkin
        // (`Документация`, `Технические требования…`, `Сборочные
        // единицы`) — sanash birinchi ma'lumotli qatordan boshlanadi.
        let mut run = 0;
        let mut seen = false;
        let mut cut = rows.len();
        for (i, r) in rows.iter().enumerate() {
            if bare(r) {
                run += 1;
                if run == 3 && seen {
                    cut = i + 1 - 3;
                    break;
                }
            } else {
                run = 0;
                seen = true;
            }
        }
        rows.truncate(cut);
        while rows.last().is_some_and(bare) {
            rows.pop();
        }
        if !rows.is_empty() {
            out.push(SpecTable {
                page,
                rows,
                off: false,
                owner: String::new(),
                ai: false,
            });
        }
    }
    out
}

/// Varaq shtampida nomi yozilgan konstruksiyani topadi.
///
/// Shtampning chap qismi — `Изм. | Кол.уч. | Лист | № док. | Подп. | Дата`,
/// undan o'ngda varaq nomi yoziladi: `К3(2К80-6М3-с-а)`. Nomda ro'yxatdagi
/// konstruksiyalardan **aynan bittasi** tilga olingan bo'lsa — varaq
/// o'shaniki. Ikkita yoki undan ko'p bo'lsa (`Колонны К1, К2`) jadval
/// qaysi biriga tegishli ekanini bilib bo'lmaydi va ega belgilanmaydi.
pub fn sheet_owner(pieces: &[Piece], list: &[Construct]) -> Option<String> {
    let izm = pieces.iter().find(|p| p.text.trim() == "Изм.")?;
    let date = pieces
        .iter()
        .filter(|p| p.dir == izm.dir && (p.y - izm.y).abs() < izm.size * 0.5 && p.x > izm.x)
        .find(|p| p.text.trim() == "Дата")?;
    let left = date.x + width(date);
    let mut zone: Vec<&Piece> = pieces
        .iter()
        .filter(|p| {
            p.dir == izm.dir
                && p.x > left
                && p.y < izm.y + izm.size
                && p.y > izm.y - izm.size * 10.0
        })
        .collect();
    zone.sort_by(|a, b| b.y.total_cmp(&a.y).then(a.x.total_cmp(&b.x)));

    // Qatorlar: zich turgan parchalar bitta so'z (`К` + `3(2` + `К80…`).
    let mut text = String::new();
    let mut prev: Option<&Piece> = None;
    for p in zone {
        match prev {
            Some(q) if (q.y - p.y).abs() <= p.size * 0.5 => {
                if p.x - (q.x + width(q)) >= p.size * 0.33 {
                    text.push(' ');
                }
            }
            Some(_) => text.push('\n'),
            None => {}
        }
        text.push_str(&p.text);
        prev = Some(p);
    }
    let tokens: Vec<String> = text
        .split(|c: char| c.is_whitespace() || "(),;:".contains(c))
        .filter(|t| !t.is_empty())
        .map(mark_key)
        .collect();
    let mut owners = list
        .iter()
        .filter(|c| c.register && tokens.contains(&mark_key(&c.mark)));
    let first = owners.next()?;
    owners.next().is_none().then(|| first.mark.clone())
}

/// KMD varag'idagi «Выборка металла» jadvali belgisi.
pub const KMD: &str = "КМД";

/// Metall tanlovi jadvalini o'qiydi (KMD varaqlari).
///
/// Bu jadval spetsifikatsiyadan boshqacha tuzilgan: `Профиль | Материал |
/// Масса общ. | с 1,032%`. Har qator — bitta prokat profili va uning shu
/// varaqdagi **jami** massasi, ya'ni marka soniga allaqachon
/// ko'paytirilgan. Shuning uchun bu qatorlar qayta ko'paytirilmaydi.
///
/// «общ.» ustuni olinadi — sof massa. Payvand va chiqindiga qo'shimcha
/// (`1,032 %`) loyihachining o'z hisobi, u alohida ustunda va bu yerda
/// qo'shilmaydi.
fn steel_tables(pieces: &[Piece], page: usize) -> Vec<SpecTable> {
    let mut out = Vec::new();
    for a in pieces
        .iter()
        .filter(|p| p.text.trim().to_lowercase().starts_with("профиль"))
    {
        let line = a.size;
        let same = |p: &&Piece| (p.y - a.y).abs() <= line * 1.2 && p.x > a.x;
        let Some(grade) = pieces
            .iter()
            .filter(same)
            .find(|p| p.text.trim().to_lowercase().starts_with("материал"))
        else {
            continue;
        };
        // «общ.» — massa ustunining boshi.
        let total_x = pieces
            .iter()
            .filter(|p| (p.y - a.y).abs() <= line * 1.2 && p.x > grade.x)
            .find(|p| p.text.trim().to_lowercase().starts_with("общ"))
            .map(|p| p.x)
            .unwrap_or(grade.x + line * 5.0);
        let left = a.x - line * 1.5;
        let b1 = grade.x - line * 0.6;
        let b2 = total_x - line * 1.2;
        let b3 = total_x + line * 3.2;

        let mut body: Vec<&Piece> = pieces
            .iter()
            .filter(|p| p.y < a.y - line * 1.2 && p.x >= left && p.x < b3)
            .collect();
        body.sort_by(|x, y| y.y.total_cmp(&x.y).then(x.x.total_cmp(&y.x)));

        let mut rows = Vec::new();
        let mut prev = a.y;
        let mut i = 0;
        while i < body.len() {
            let y = body[i].y;
            if prev - y > line * 5.0 {
                break;
            }
            prev = y;
            let mut cells = [String::new(), String::new(), String::new()];
            while i < body.len() && (body[i].y - y).abs() <= line * 0.5 {
                let p = body[i];
                let idx = if p.x < b1 {
                    0
                } else if p.x < b2 {
                    1
                } else {
                    2
                };
                cells[idx].push_str(p.text.trim());
                i += 1;
            }
            let low = cells[0].to_lowercase();
            // Po'lat sinfi: `С245`, `С255` — harf va uch raqam. Bu shart
            // varaqdagi boshqa jadvallarning (uzunlik, massa ro'yxati)
            // qatorlarini chiqarib tashlaydi.
            let grade_ok = {
                let g = cells[1].trim();
                let mut ch = g.chars();
                matches!(ch.next(), Some('С' | 'C'))
                    && g.chars().skip(1).all(|c| c.is_ascii_digit())
                    && g.chars().count() >= 3
            };
            if cells[0].is_empty()
                || !grade_ok
                || low.contains("итого")
                || low.contains("всего")
                || number(&cells[2]).is_none()
            {
                continue;
            }
            // `□` ko'p shriftda yo'q — so'z bilan yoziladi.
            let profile = cells[0].replace('□', "Kvadrat quvur ");
            rows.push(SpecRow {
                designation: KMD.to_string(),
                name: format!("{} {}", profile.trim(), cells[1])
                    .trim()
                    .to_string(),
                mass: cells[2].clone(),
                note: "кг".into(),
                ..Default::default()
            });
        }
        if !rows.is_empty() {
            out.push(SpecTable {
                page,
                rows,
                off: false,
                owner: String::new(),
                ai: false,
            });
        }
    }
    out
}

/// `[lo, hi]` oralig'idagi bo'sh oraliqlardan `guess` ga eng yaqinining
/// o'rtasi.
///
/// Ustun chegarasi ma'lumotdagi bo'shliqdan olinadi, lekin bo'shliq bir
/// nechta bo'lishi mumkin: tor pozitsiya ustunida sarlavhadan chapda ham,
/// o'ngda ham joy qoladi. Eng kengini olish xato berardi (uskunalar
/// spetsifikatsiyasida pozitsiya raqami nomga yopishib ketgan edi),
/// shuning uchun sarlavhalar o'rtasiga eng yaqini tanlanadi. `min` dan tor
/// oraliq — harflar orasidagi joy, ustun chegarasi emas.
fn nearest_gap(spans: &[(f64, f64)], lo: f64, hi: f64, guess: f64, min: f64) -> Option<f64> {
    let mut inside: Vec<(f64, f64)> = spans
        .iter()
        .filter(|s| s.1 > lo && s.0 < hi)
        .map(|s| (s.0.max(lo), s.1.min(hi)))
        .collect();
    inside.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut gaps: Vec<(f64, f64)> = Vec::new();
    let mut edge = lo;
    for (a, b) in inside {
        if a - edge >= min {
            gaps.push((edge, a));
        }
        edge = edge.max(b);
    }
    if hi - edge >= min {
        gaps.push((edge, hi));
    }
    gaps.into_iter()
        .map(|(a, b)| {
            // Taxmin oraliq ichida bo'lsa — masofa nol.
            let dist = if guess < a {
                a - guess
            } else if guess > b {
                guess - b
            } else {
                0.0
            };
            (dist, (a + b) / 2.0)
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map(|g| g.1)
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
    // Sinf `A` yoki `B` bilan boshlanadi (lotin yoki kirill): `A-III`,
    // `A400`, `Bp-I`. Aks holda bu armatura emas — `Ø 100мм` quvur
    // diametri ham shu belgi bilan yoziladi.
    if !class
        .chars()
        .next()
        .is_some_and(|c| matches!(c, 'A' | 'А' | 'B' | 'В'))
    {
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

    if row.designation == KMD {
        return Some(Item {
            kind: Kind::Steel,
            material: format!("Prokat {name}"),
            unit: "kg".into(),
            amount: mass?,
        });
    }

    if let Some(class) = concrete(name) {
        // Hajm qaysi ustunda yozilgani loyihachiga qarab farq qiladi.
        // Ba'zan hajm nomning o'ziga yopishib yoziladi:
        // `Бетон кл.В7.5(подготовка)1.12`.
        let glued = || name.rsplit_once(')').and_then(|(_, tail)| number(tail));
        let amount = qty.or(mass).or(note).or_else(glued)?;
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
            material: format!("Armatura Ø{dia} {class}"),
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
        // Massasi yozilmagan bo'lsa (`Лоток из швеллера — 8 шт`) u dona
        // bilan sanaladigan buyum — pastda shunday o'qiladi.
        if let Some(amount) = total {
            return Some(Item {
                kind: Kind::Steel,
                material: label.into(),
                unit: "kg".into(),
                amount,
            });
        }
    }

    // Boshqa buyum: miqdori va o'lchov birligi aniq yozilgan bo'lsa.
    let unit = unit_of(&row.unit).or_else(|| unit_of(&row.note));
    if let (Some(unit), Some(amount)) = (unit, qty.or(number(&row.note))) {
        return Some(Item {
            kind: Kind::Other,
            material: name.to_string(),
            unit,
            amount,
        });
    }
    let (material, amount, unit) = inline(name)?;
    Some(Item {
        kind: Kind::Other,
        material,
        unit,
        amount,
    })
}

/// Miqdori matnning o'zida yozilgan qator:
/// `Водосборный лоток сталь листовая б=2 мм - 14.67 тонн`.
///
/// Faqat aniq shakl olinadi: oxirida ` - son birlik`. Boshqa har qanday
/// yozuv tanilmaydi — taxmin qilinmaydi.
fn inline(name: &str) -> Option<(String, f64, String)> {
    let (head, tail) = name.rsplit_once(" - ")?;
    let mut words = tail.split_whitespace();
    let figure = words.next()?;
    if !figure
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
    {
        return None;
    }
    let amount = number(figure)?;
    let unit = match words.next()?.to_lowercase().trim_end_matches('.') {
        "тонн" | "т" => "t",
        "кг" => "kg",
        "м3" => "m3",
        "м2" => "m2",
        "м" => "m",
        "шт" => "dona",
        _ => return None,
    };
    if words.next().is_some() || head.trim().is_empty() {
        return None;
    }
    Some((head.trim().to_string(), amount, unit.to_string()))
}

/// «Примеч.» ustunidagi o'lchov birligi: `шт.`, `п.м.`, `1050 м3`.
fn unit_of(note: &str) -> Option<String> {
    let low = note.to_lowercase().replace(' ', "");
    // Ustunda birlikning o'zi turadi: `м`, `т`, `кг`.
    match low.trim_end_matches('.') {
        "м" => return Some("m".into()),
        "т" => return Some("t".into()),
        "кг" => return Some("kg".into()),
        "компл" | "к-т" => return Some("kompl".into()),
        _ => {}
    }
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
    /// Joylashuv sxemasining ro'yxatidan olinganmi.
    ///
    /// Ro'yxatdagi konstruksiya (`Фм1`, `К1`) nomi bo'yicha boshqa
    /// varaqdagi spetsifikatsiya bilan bog'lanadi. Ro'yxatdan tashqarida
    /// topilgan belgi (`Ан-1`) esa faqat o'z jadvalida ishlaydi: aks holda
    /// poydevor ichidagi «Анкер Ан-1» sarlavhasi butun poydevorni anker
    /// soniga ko'paytirib yuborardi.
    #[serde(default)]
    pub register: bool,
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
        // Jadval ro'yxatmi: unda material qatori yo'q, faqat konstruksiyalar.
        let materials = t
            .rows
            .iter()
            .filter(|r| item(r).is_some_and(|i| i.kind != Kind::Other))
            .count();
        let register = materials == 0;
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
            // Soni «Кол.» ustunida, ba'zan esa izohda birligi bilan
            // yoziladi: `294 п.м.`.
            let in_note = unit_of(&r.note).and_then(|_| number(&r.note));
            let Some(count) = number(&r.qty).or(in_note) else {
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
                register,
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
    for c in list.iter().filter(|c| c.register) {
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

/// Havola qatorining kalitlari: pozitsiyasi va nomining belgisi.
fn ref_keys(r: &SpecRow) -> Vec<String> {
    let mut keys = vec![mark_key(&r.pos)];
    // Bir-ikki belgili pozitsiya (`2`, `а`) — tartib raqami, belgi emas;
    // u holda havola nomdagi belgi bo'yicha topiladi.
    keys.extend(heading_keys(&r.name));
    keys.retain(|k| !k.is_empty());
    keys
}

/// Yig'ma birlikka havola: konstruksiya ichida `С1 — 2 dona`.
struct Reference {
    page: usize,
    /// Havola pozitsiyasi (`С1`) yoki nomining oxiri (`… МН-1`) bo'yicha
    /// topiladi.
    keys: Vec<String>,
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
    build(tables, list).0
}

/// Sanalmagan takror spetsifikatsiyalar: `(sarlavha, sahifa)`.
///
/// Bitta konstruksiya loyihada bir necha marta yozilishi mumkin: bir
/// pog'on metrga, keyin 64 metrga, yoki boshqa varaqda qayta. Hammasini
/// qo'shish ehtiyojni ikki-uch barobar oshirardi. Shuning uchun birinchisi
/// olinadi, qolganlari sanalmaydi va shu ro'yxatda ko'rsatiladi — odam
/// qaysi biri to'g'ri ekanini o'zi tekshiradi.
pub fn repeats(tables: &[SpecTable], list: &[Construct]) -> Vec<(String, usize)> {
    build(tables, list).1
}

/// Jadval egasi bo'lgan konstruksiya.
fn owner_of<'a>(t: &SpecTable, list: &'a [Construct]) -> Option<&'a Construct> {
    if t.owner.is_empty() {
        return None;
    }
    let key = mark_key(&t.owner);
    list.iter().find(|c| mark_key(&c.mark) == key)
}

fn build(tables: &[SpecTable], list: &[Construct]) -> (Vec<Line>, Vec<(String, usize)>) {
    // ---- 1-o'tish: yig'ma birliklarga havolalar.
    let mut refs: Vec<Reference> = Vec::new();
    for t in tables {
        let mut count: Option<f64> = owner_of(t, list).map(|c| c.count);
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
                    keys: ref_keys(r),
                    count: count.map(|c| c * q),
                });
            }
        }
    }
    // Sahifadagi sarlavhalar — havola o'z jadvalini topdimi, shundan
    // bilinadi.
    let mut heads: Vec<(usize, Vec<String>)> = Vec::new();
    for t in tables {
        for r in &t.rows {
            if r.pos.trim().is_empty() && !r.name.trim().is_empty() && item(r).is_none() {
                heads.push((t.page, heading_keys(&r.name)));
            }
        }
    }
    let referenced = |page: usize, name: &str| -> Option<Option<f64>> {
        let keys = heading_keys(name);
        let hits: Vec<&Reference> = refs
            .iter()
            .filter(|r| r.page == page && r.keys.iter().any(|k| keys.contains(k)))
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
    let mut skipped: Vec<(String, usize)> = Vec::new();
    // Spetsifikatsiyasi allaqachon o'qilgan konstruksiyalar: belgi -> jadval.
    let mut done: Vec<(String, usize)> = Vec::new();
    for (ti, t) in tables.iter().enumerate() {
        // Varaq bitta konstruksiyaga bag'ishlangan bo'lsa — hisob o'sha
        // konstruksiya sonidan boshlanadi.
        let owner = owner_of(t, list);
        let mut group = owner
            .map(|c| format!("{} {}", c.mark, c.name).trim().to_string())
            .unwrap_or_default();
        let mut count: Option<f64> = owner.map(|c| c.count);
        let mut per = 1.0;
        let mut sub = 1.0;
        // Takror spetsifikatsiya ichidamiz — qatorlar sanalmaydi.
        let mut skip = false;
        // Aynan bir xil jadval boshqa varaqda qayta berilgan (masalan,
        // tom panellari ham AR, ham KM albomida): ikkinchisi sanalmaydi.
        // Egasi boshqa bo'lsa (`К3` va `К4` varaqlari) — bu takror emas.
        // Metall tanlovi bundan mustasno: bir xil fermalar varaqlari
        // haqiqatan bir xil bo'lishi mumkin.
        let kmd = t.rows.iter().any(|r| r.designation == KMD);
        let twin = tables[..ti]
            .iter()
            .any(|o| o.page != t.page && o.owner == t.owner && o.rows == t.rows);
        if twin && !kmd {
            let title = t
                .rows
                .iter()
                .map(|r| r.name.trim())
                .find(|n| !n.is_empty())
                .unwrap_or_default();
            skipped.push((title.to_string(), t.page));
            continue;
        }
        for (i, r) in t.rows.iter().enumerate() {
            let it = item(r);
            // Pozitsiyasiz qator ikki xil bo'ladi: yig'ma birlik sarlavhasi
            // (`Арматурная сетка С1 — 2 шт`, ortidan uning pozitsiyalari
            // keladi) yoki o'zi material (`Объем кирпича — 1050 м3`).
            // Farqi — keyingi qatorda pozitsiya bor-yo'qligi.
            let followed = t.rows.get(i + 1).is_some_and(|n| !n.pos.trim().is_empty());
            let heading = r.pos.trim().is_empty()
                && !r.name.trim().is_empty()
                && match &it {
                    None => true,
                    Some(x) if x.kind == Kind::Other => {
                        followed || construct_of(&r.name, list).is_some()
                    }
                    Some(_) => false,
                };
            if heading {
                if let Some(c) = construct_of(&r.name, list) {
                    // Yangi konstruksiya: ko'paytuvchilar qaytadan.
                    group = r.name.trim().to_string();
                    count = Some(c.count);
                    per = basis(&r.name);
                    sub = 1.0;
                    let key = mark_key(&c.mark);
                    skip = done.iter().any(|d| d.0 == key && d.1 != ti);
                    if skip {
                        skipped.push((group.clone(), t.page));
                    } else if !done.iter().any(|d| d.0 == key) {
                        done.push((key, ti));
                    }
                } else if skip {
                    // Takror ichidagi yig'ma birlik ham sanalmaydi.
                } else if let Some(total) = referenced(t.page, &r.name) {
                    // Alohida jadvaldagi yig'ma birlik: soni havoladan.
                    group = r.name.trim().to_string();
                    count = total;
                    per = 1.0;
                    sub = 1.0;
                } else {
                    // Ichma-ich yig'ma birlik: `Арматурная сетка С1 — 2 шт`.
                    // Konstruksiyasiz jadvalda sarlavha guruh nomi bo'ladi.
                    if count.is_none() {
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
            if own {
                // Konstruksiya qatoridan keyin shu jadvalning o'zida uning
                // pozitsiyalari kelishi mumkin (`Зд-1 — 60 dona`, so'ng
                // list va armatura). U holda bu qator — sarlavha.
                if let Some(c) = list.iter().find(|c| mark_key(&c.mark) == mark_key(&r.pos)) {
                    if c.register {
                        // Ro'yxat jadvalida tarkib yozilmaydi: undan
                        // keyingi qator (`Объем кирпича — 1050 м3`) shu
                        // konstruksiyaga tegishli emas va uning soniga
                        // ko'paytirilmaydi.
                        group.clear();
                        count = None;
                        per = 1.0;
                    } else {
                        group = format!("{} {}", c.mark, r.name.trim());
                        count = Some(c.count);
                        per = basis(&r.name);
                    }
                    sub = 1.0;
                    skip = false;
                }
                continue;
            }
            if skip {
                continue;
            }
            if is_reference(r) {
                // Tarkibi shu varaqda yozilgan bo'lsa — material o'sha
                // jadvaldan olinadi. Yozilmagan bo'lsa (seriya bo'yicha
                // tayyor buyum: `Изделие закладное УП2-8 — 4 шт`) uning
                // massasi loyihada yozilganicha olinadi, aks holda u
                // hisobdan butunlay tushib qolardi.
                let keys = ref_keys(r);
                let resolved = heads
                    .iter()
                    .any(|h| h.0 == t.page && h.1.iter().any(|k| keys.contains(k)));
                let mass = match (number(&r.note), number(&r.qty), number(&r.mass)) {
                    (Some(n), _, _) => Some(n),
                    (_, Some(q), Some(m)) => Some(q * m),
                    _ => None,
                };
                if let (false, Some(mass)) = (resolved, mass) {
                    out.push(Line {
                        item: Item {
                            kind: Kind::Steel,
                            material: r.name.trim().to_string(),
                            unit: "kg".into(),
                            amount: mass / per,
                        },
                        group: group.clone(),
                        sub: 1.0,
                        count,
                        page: t.page,
                    });
                }
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
    (out, skipped)
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

/// Beton narxlari manbasi.
pub const ORIENTIR_CONCRETE: &str = "prom.uz e'lonlari, 2026-10";
/// Metall narxlari manbasi. Prays eski — bu ochiq aytiladi.
pub const ORIENTIR_METAL: &str = "uzmetallsavdo.uz praysi, 2023-09";
/// Praysda yo'q ingichka armatura — eng yaqin diametr narxi bilan.
pub const ORIENTIR_THIN: &str = "uzmetallsavdo.uz, Ø10–12 narxi bo'yicha (ingichkasi praysda yo'q)";
pub const ORIENTIR_WIRE: &str = "prom.uz, Вр-1 sim e'loni, 2026-10";
/// KMD dagi `зд-8` — qalinligi bo'yicha list deb olindi.
pub const ORIENTIR_PLATE: &str = "uzmetallsavdo.uz, list narxi (зд — list deb olindi)";
/// Tayyor metall buyum — faqat metallining narxi.
pub const ORIENTIR_PARTS: &str = "uzmetallsavdo.uz, list narxi bo'yicha — tayyorlash haqisiz";
pub const ORIENTIR_BRICK: &str = "e'lonlar: 1 000 so'm/dona × 400 dona/m³ terim";
pub const ORIENTIR_PANEL: &str = "met-trans.uz, PIR 50 mm: 150 522 so'm/m² × panel yuzasi";
pub const ORIENTIR_PIPE: &str = "uzmetallsavdo.uz: 10 500 so'm/kg × nazariy massa";
pub const ORIENTIR_BEND: &str = "prom.uz, otvod DN108 e'loni";

/// Sendvich panel yuzasi, m²: `Сп-1, L=11450x1000` → 11.45.
fn panel_area(material: &str) -> Option<f64> {
    let tail = material.split_once("L=")?.1;
    let (a, b) = tail.split_once(['x', 'х'])?;
    let (a, b) = (number(a)?, number(b)?);
    (a > 0.0 && b > 0.0).then_some(a * b / 1e6)
}

/// Po'lat quvurning nazariy massasi, kg/m: `Ø159*4,5` → 17.15.
fn pipe_mass(material: &str) -> Option<f64> {
    let tail = material.split_once('Ø')?.1;
    let (d, t) = tail.split_once(['*', 'x', 'х'])?;
    let (d, t) = (number(d)?, number(t)?);
    (d > t && t > 0.0).then_some((d - t) * t * 0.02466)
}

/// Orientir narx: `(so'm, manba)`.
///
/// Bu **ta'minotchi narxi emas**. Raqamlar Toshkentdagi ochiq e'lon va
/// prayslardan olingan va faqat tartibni ko'rsatadi: beton — bir necha
/// sotuvchi e'lonining o'rtachasi, metall — QQS bilan prays narxi. Narx
/// bazasida yoki qo'lda kiritilgan narx bo'lsa, u ustun turadi.
///
/// Ba'zi narxlar manbadagi raqamdan **hisoblab** chiqariladi (quvur —
/// kilogramm narxi × nazariy massa, panel — kvadrat metr narxi × yuza,
/// g'isht — dona narxi × terimdagi soni) yoki eng yaqin o'xshash narx
/// bilan olinadi. Har biri manba satrida shunday deb yozilgan — odam
/// qaysi raqamga qancha ishonishni o'zi ko'radi.
///
/// Manbasi ham, hisoblash yo'li ham yo'q material narxsiz qoladi.
pub fn orientir(kind: Kind, material: &str, unit: &str) -> Option<(f64, &'static str)> {
    let low = material.to_lowercase();
    match kind {
        Kind::Concrete if unit == "m3" => {
            let price = match material {
                "Beton B7.5" => 451_000.0,
                "Beton B15" => 523_000.0,
                "Beton B20" => 675_000.0,
                "Beton B22.5" => 684_000.0,
                "Beton B25" => 750_000.0,
                _ => return None,
            };
            Some((price, ORIENTIR_CONCRETE))
        }
        Kind::Rebar if unit == "kg" => {
            // `Armatura Ø12 A-III`: sinf — uchinchi so'z.
            let class = material.split_whitespace().nth(2).unwrap_or("");
            if class.starts_with(['B', 'В']) {
                return Some((10_400.0, ORIENTIR_WIRE));
            }
            let at = material.find('Ø')?;
            let dia: String = material[at + 'Ø'.len_utf8()..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            match dia.parse::<u32>().ok()? {
                1..=9 => Some((8_350.0, ORIENTIR_THIN)),
                10..=12 => Some((8_350.0, ORIENTIR_METAL)),
                14..=40 => Some((8_200.0, ORIENTIR_METAL)),
                _ => None,
            }
        }
        Kind::Steel if unit == "kg" => Some(if low.contains("kvadrat quvur") {
            (10_700.0, ORIENTIR_METAL)
        } else if low.contains("труба") {
            (10_500.0, ORIENTIR_METAL)
        } else if low.contains("shveller") {
            (13_500.0, ORIENTIR_METAL)
        } else if low.contains("burchak") || material.contains('∠') || low.contains("po'lat list")
        {
            // Praysda list va burchak narxi yaqin: 11,8–13,0 mln/t.
            (12_400.0, ORIENTIR_METAL)
        } else if low.starts_with("prokat зд-") {
            (12_400.0, ORIENTIR_PLATE)
        } else {
            (12_400.0, ORIENTIR_PARTS)
        }),
        Kind::Other => match unit {
            "m3" if low.contains("кирпич") => Some((400_000.0, ORIENTIR_BRICK)),
            "dona" if low.starts_with("сп-") => {
                panel_area(material).map(|a| ((150_522.0 * a).round(), ORIENTIR_PANEL))
            }
            "dona" if low.contains("отвод") && low.contains("ø100") => {
                Some((95_700.0, ORIENTIR_BEND))
            }
            "m" if low.contains("труб") => {
                pipe_mass(material).map(|m| ((10_500.0 * m).round(), ORIENTIR_PIPE))
            }
            "t" if low.contains("сталь") || low.contains("стальн") || low.contains("металл") => {
                Some((12_400_000.0, ORIENTIR_PARTS))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Loyihadan olingan hisob — bazada saqlanadi, qayta o'qish shart emas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Takeoff {
    pub file: String,
    pub pages: usize,
    pub tables: Vec<SpecTable>,
    pub constructs: Vec<Construct>,
    /// AI o'qigan varaqlarning **qoida bo'yicha** o'qilgan nusxasi.
    ///
    /// Hisobga kirmaydi. Ikki mustaqil o'qishni solishtirish va AI
    /// o'qishidan voz kechish uchun saqlanadi.
    #[serde(default)]
    pub rule_tables: Vec<SpecTable>,
    /// AI o'qishi haqida ma'lumot; `None` — AI ishlatilmagan.
    #[serde(default)]
    pub ai: Option<AiInfo>,
}

/// AI o'qishining qaydi: nima o'qildi, nima o'qilmadi.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AiInfo {
    pub model: String,
    /// AI o'qigan varaqlar.
    pub pages: Vec<usize>,
    /// O'qilmagan varaqlar va sababi — ular qoida bo'yicha qolgan.
    pub failed: Vec<(usize, String)>,
    /// Sarflangan tokenlar.
    pub tokens: u32,
}

/// Bitta varaqda ikki o'qishning farqi: metall (kg) va beton (m³).
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    pub page: usize,
    pub rule_kg: f64,
    pub ai_kg: f64,
    pub rule_m3: f64,
    pub ai_m3: f64,
}

impl Diff {
    /// Ikki o'qish mos keldimi: farq 1 % dan oshmasa.
    pub fn agrees(&self) -> bool {
        let near = |a: f64, b: f64| (a - b).abs() <= 0.01 * a.abs().max(b.abs()).max(1e-9);
        near(self.rule_kg, self.ai_kg) && near(self.rule_m3, self.ai_m3)
    }
}

/// Jadvallardagi xom miqdor: ko'paytirilmagan metall va beton.
fn raw_sums(tables: &[&SpecTable]) -> (f64, f64) {
    let (mut kg, mut m3) = (0.0, 0.0);
    for r in tables.iter().flat_map(|t| &t.rows) {
        // Yig'ma birlikka havola (`С1 — 123.4 kg`) tarkibi bilan birga
        // ikki marta sanalmasin.
        if is_reference(r) {
            continue;
        }
        match item(r) {
            Some(i) if i.kind == Kind::Concrete => m3 += i.amount,
            Some(i) if i.unit == "kg" => kg += i.amount,
            _ => {}
        }
    }
    (kg, m3)
}

impl Takeoff {
    /// Sahifalar bo'yicha parchalardan hisobni tuzadi.
    pub fn read(file: &str, pages: &[Vec<Piece>]) -> Takeoff {
        let mut all = Vec::new();
        for (i, p) in pages.iter().enumerate() {
            all.extend(tables(p, i + 1));
        }
        let list = constructs(&all);
        for (i, p) in pages.iter().enumerate() {
            let Some(owner) = sheet_owner(p, &list) else {
                continue;
            };
            for t in all.iter_mut().filter(|t| t.page == i + 1) {
                // Metall tanlovi — varaq bo'yicha jami, u ko'paytirilmaydi.
                if !t.rows.iter().any(|r| r.designation == KMD) {
                    t.owner = owner.clone();
                }
            }
        }
        Takeoff {
            file: file.to_string(),
            pages: pages.len(),
            tables: all,
            constructs: list,
            ..Default::default()
        }
    }

    /// AI o'qigan varaqlarni hisobga kiritadi.
    ///
    /// Har varaqning qoida bo'yicha o'qilgan jadvallari `rule_tables` ga
    /// ko'chadi, o'rniga AI jadvallari qo'yiladi. AI hech narsa topmagan
    /// varaq o'zgarmaydi — bo'sh javob jadvalni o'chirib yubormaydi.
    /// Konstruksiyalar ro'yxati yangi jadvallardan qayta yig'iladi.
    pub fn apply_ai(&mut self, pages: Vec<(usize, String, Vec<SpecTable>)>, info: AiInfo) {
        let mut read = Vec::new();
        for (page, owner, mut tables) in pages {
            tables.retain(|t| !t.rows.is_empty());
            if tables.is_empty() {
                continue;
            }
            // Qayta o'qishda eski AI jadvallari tashlanadi, qoida nusxasi
            // esa birinchi o'qishdagidek qoladi.
            let (old_ai, old_rule): (Vec<SpecTable>, Vec<SpecTable>) = self
                .tables
                .iter()
                .filter(|t| t.page == page)
                .cloned()
                .partition(|t| t.ai);
            let _ = old_ai;
            if !old_rule.is_empty() {
                self.rule_tables.retain(|t| t.page != page);
                self.rule_tables.extend(old_rule);
            }
            // Shtampdan topilgan ega saqlanadi, AI aytgani — zaxira.
            let known = self
                .rule_tables
                .iter()
                .find(|t| t.page == page && !t.owner.is_empty())
                .map(|t| t.owner.clone());
            self.tables.retain(|t| t.page != page);
            for mut t in tables {
                t.page = page;
                t.ai = true;
                t.off = false;
                let kmd = t.rows.iter().any(|r| r.designation == KMD);
                t.owner = if kmd {
                    String::new()
                } else {
                    known.clone().unwrap_or_else(|| owner.clone())
                };
                self.tables.push(t);
            }
            read.push(page);
        }
        self.tables.sort_by_key(|t| t.page);
        self.constructs = constructs(&self.tables);
        // AI aytgan ega ro'yxatdagi konstruksiya bo'lmasa — ega emas.
        let list = self.constructs.clone();
        for t in self.tables.iter_mut().filter(|t| t.ai) {
            let key = mark_key(&t.owner);
            if !list.iter().any(|c| c.register && mark_key(&c.mark) == key) {
                t.owner.clear();
            }
        }
        let mut all = self.ai.take().map(|a| a.pages).unwrap_or_default();
        all.extend(read);
        all.sort_unstable();
        all.dedup();
        self.ai = Some(AiInfo { pages: all, ..info });
    }

    /// AI o'qishidan voz kechadi: qoida bo'yicha o'qilgan jadvallar
    /// qaytariladi.
    pub fn revert_ai(&mut self) {
        let pages: Vec<usize> = self
            .tables
            .iter()
            .filter(|t| t.ai)
            .map(|t| t.page)
            .collect();
        self.tables.retain(|t| !t.ai);
        let back: Vec<SpecTable> = self
            .rule_tables
            .drain(..)
            .filter(|t| pages.contains(&t.page))
            .collect();
        self.tables.extend(back);
        self.tables.sort_by_key(|t| t.page);
        self.constructs = constructs(&self.tables);
        self.ai = None;
    }

    /// Ikki mustaqil o'qishni varaqma-varaq solishtiradi.
    ///
    /// Bu — aniqlikning o'lchovi: qoida ham, AI ham bir xil kilogramm va
    /// kub chiqargan varaqqa ishonish mumkin; farq qilganini odam ko'rishi
    /// kerak. Qaysi biri to'g'ri ekanini dastur hal qilmaydi.
    pub fn diffs(&self) -> Vec<Diff> {
        let mut pages: Vec<usize> = self
            .tables
            .iter()
            .filter(|t| t.ai)
            .map(|t| t.page)
            .collect();
        pages.dedup();
        pages
            .into_iter()
            .map(|page| {
                let rule: Vec<&SpecTable> =
                    self.rule_tables.iter().filter(|t| t.page == page).collect();
                let ai: Vec<&SpecTable> = self
                    .tables
                    .iter()
                    .filter(|t| t.page == page && t.ai)
                    .collect();
                let (rule_kg, rule_m3) = raw_sums(&rule);
                let (ai_kg, ai_m3) = raw_sums(&ai);
                Diff {
                    page,
                    rule_kg,
                    ai_kg,
                    rule_m3,
                    ai_m3,
                }
            })
            .collect()
    }

    pub fn repeats(&self) -> Vec<(String, usize)> {
        repeats(&self.active(), &self.constructs)
    }

    /// Hisobga olinadigan jadvallar.
    fn active(&self) -> Vec<SpecTable> {
        self.tables.iter().filter(|t| !t.off).cloned().collect()
    }

    pub fn lines(&self) -> Vec<Line> {
        let mut out = lines(&self.active(), &self.constructs);
        // Tarkibi loyihada yozilmagan konstruksiya (tayyor buyum: yig'ma
        // ustun, sendvich panel) o'zi material sifatida sanaladi — aks
        // holda u kalkulyatsiyadan tushib qolardi.
        for c in &self.constructs {
            let key = mark_key(&c.mark);
            if out.iter().any(|l| mark_key(&l.group).contains(&key)) {
                continue;
            }
            let name = if c.name.trim().is_empty() {
                c.mark.clone()
            } else {
                c.name.trim().to_string()
            };
            out.push(Line {
                item: Item {
                    kind: Kind::Other,
                    material: name,
                    unit: c.unit.clone(),
                    amount: 1.0,
                },
                group: c.mark.clone(),
                sub: 1.0,
                count: Some(c.count),
                page: c.page,
            });
        }
        out
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
            dir: 0,
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
        assert_eq!(rows[2].name, "Ø14 A-III L=2550");
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
            unit: String::new(),
        };
        let it = item(&rebar).expect("armatura");
        assert_eq!(it.kind, Kind::Rebar);
        assert_eq!(it.material, "Armatura Ø14 A-III");
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
            off: false,
            owner: String::new(),
            ai: false,
        });
        let list = constructs(&all);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].mark, "Фм3");
        assert_eq!(list[0].count, 4.0);

        let ls = lines(&all, &list);
        assert_eq!(ls.len(), 2, "{ls:#?}");
        // Armatura: 68 kg × 2 to'r × 4 poydevor = 544 kg.
        assert_eq!(ls[0].item.material, "Armatura Ø14 A-III");
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
            unit: String::new(),
        };
        let all = vec![
            SpecTable {
                page: 10,
                rows: vec![row("Фм1", "Фундамент монолитный Фм1", "20", "", "шт.")],
                off: false,
                owner: String::new(),
                ai: false,
            },
            // To'rning tarkibi — alohida jadval.
            SpecTable {
                page: 11,
                rows: vec![
                    row("", "Арматурная сетка С 2", "", "", "13.7"),
                    row("3", "∅12A400, L= 1700", "5", "1.51", "7.5"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            // Konstruksiya jadvali: to'rga havola.
            SpecTable {
                page: 11,
                rows: vec![
                    row("", "Фм 1", "", "", ""),
                    row("С2", "Арматурная сетка С2", "2", "13.7", "27.4"),
                    row("", "Бетон кл. В 20(М250)W8", "7.2", "", "м³"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
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

    /// Uskunalar spetsifikatsiyasi: birlik o'z ustunida.
    #[test]
    fn an_equipment_row_takes_its_unit_from_the_unit_column() {
        let r = SpecRow {
            pos: "3".into(),
            name: "Трубы стальные электросварные".into(),
            qty: "850".into(),
            unit: "м".into(),
            ..Default::default()
        };
        let it = item(&r).expect("quvur");
        assert_eq!(it.kind, Kind::Other);
        assert_eq!(it.unit, "m");
        assert_eq!(it.amount, 850.0);
        // Birligi yozilmagan qator material emas — taxmin qilinmaydi.
        let bare = SpecRow {
            pos: "4".into(),
            name: "Nimadir".into(),
            qty: "5".into(),
            ..Default::default()
        };
        assert!(item(&bare).is_none());
    }

    /// Pozitsiyasiz qator: tarkibi ortidan kelsa sarlavha, aks holda
    /// o'zi material.
    #[test]
    fn a_bare_row_is_a_heading_only_when_parts_follow() {
        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
            ..Default::default()
        };
        let all = vec![SpecTable {
            page: 35,
            rows: vec![
                // 140 p.m uchun, tarkibi bir pog'on metrga yozilgan.
                row("", "МН1 (на 1-п.м.)", "140", "", "п.м"),
                row("1", "30х30х4 Уголок L=1000", "2", "1.78", "3.56"),
                // O'zi material: ortidan pozitsiya kelmaydi.
                row("", "Объем кирпича 380мм", "", "", "1050 м3"),
            ],
            off: false,
            owner: String::new(),
            ai: false,
        }];
        let ls = lines(&all, &[]);
        assert_eq!(ls.len(), 2, "{ls:#?}");
        // Burchak: 3.56 kg × 140 p.m.
        assert_eq!(ls[0].item.kind, Kind::Steel);
        assert!((ls[0].total() - 498.4).abs() < 1e-9, "{}", ls[0].total());
        assert_eq!(ls[1].item.material, "Объем кирпича 380мм");
        assert_eq!(ls[1].total(), 1050.0);
    }

    /// Tarkibi yozilmagan konstruksiya kalkulyatsiyadan tushib qolmaydi.
    #[test]
    fn a_construct_without_a_spec_is_still_counted() {
        let tk = Takeoff {
            file: "x.pdf".into(),
            pages: 1,
            tables: vec![SpecTable {
                page: 16,
                rows: vec![SpecRow {
                    pos: "К3".into(),
                    name: "2К138-6М3-c-а".into(),
                    qty: "12".into(),
                    ..Default::default()
                }],
                off: false,
                owner: String::new(),
                ai: false,
            }],
            constructs: vec![Construct {
                mark: "К3".into(),
                name: "2К138-6М3-c-а".into(),
                count: 12.0,
                unit: "dona".into(),
                page: 16,
                register: true,
            }],
            ..Default::default()
        };
        let ls = tk.lines();
        assert_eq!(ls.len(), 1);
        assert_eq!(ls[0].total(), 12.0);
        assert_eq!(ls[0].item.unit, "dona");
    }

    /// Bitta konstruksiya ikki marta yozilgan bo'lsa, faqat birinchisi
    /// sanaladi — ikkinchisi ko'rsatiladi, lekin qo'shilmaydi.
    #[test]
    fn a_repeated_spec_is_not_counted_twice() {
        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
            ..Default::default()
        };
        let all = vec![
            SpecTable {
                page: 10,
                rows: vec![
                    row("Фл-1", "Фундамент Фл-1", "348", "", "п.м."),
                    row("Фм1", "Фундамент монолитный Фм1", "20", "", "шт."),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            SpecTable {
                page: 15,
                rows: vec![
                    row("", "Фундамент Фл-1 (на 1 п.м)", "", "", ""),
                    row("C-1", "∅12 A-III м.п.", "27", "0.888", "24.0"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            // O'sha konstruksiya, endi 64 metrga yozilgan.
            SpecTable {
                page: 15,
                rows: vec![
                    row("", "Фундамент Фл-1 (на 64 п.м)", "", "", ""),
                    row("C-1", "∅12 A-III м.п.", "1728", "0.888", "1534"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
        ];
        let list = constructs(&all);
        assert!(list.iter().all(|c| c.register), "{list:?}");
        let ls = lines(&all, &list);
        assert_eq!(ls.len(), 1, "{ls:#?}");
        assert_eq!(ls[0].total(), 24.0 * 348.0);
        let rep = repeats(&all, &list);
        assert_eq!(rep.len(), 1);
        assert_eq!(rep[0].1, 15);
    }

    /// Ro'yxatdan tashqaridagi belgi boshqa jadvaldagi sarlavhani
    /// «egallab» olmaydi.
    #[test]
    fn a_part_heading_does_not_steal_the_construct() {
        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
            ..Default::default()
        };
        let all = vec![
            SpecTable {
                page: 10,
                rows: vec![
                    row("Фм3", "Фундамент монолитный Фм3", "4", "", "шт."),
                    row("Фм4", "Фундамент монолитный Фм4", "60", "", "шт."),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            SpecTable {
                page: 13,
                rows: vec![
                    row("", "Фундамент Фм3", "1", "", "шт."),
                    row("", "Анкер Ан-1", "", "", ""),
                    row("5", "∅28 A-III L=4000", "4", "19.3", "77.2"),
                    row("", "Бетон кл. В20(М250)W8", "", "", "3.66"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
            // Boshqa varaqda `Ан-1` o'zi konstruksiya sifatida yozilgan.
            SpecTable {
                page: 42,
                rows: vec![
                    row("Ан-1", "Анкерная группа Ан-1", "2", "", "шт."),
                    row("1", "∅20 A-III L=800", "4", "2.47", "9.88"),
                ],
                off: false,
                owner: String::new(),
                ai: false,
            },
        ];
        let list = constructs(&all);
        let ls = lines(&all, &list);
        let beton = ls.iter().find(|l| l.item.kind == Kind::Concrete).unwrap();
        // Beton poydevorniki: 3.66 × 4, anker soniga (2) emas.
        assert_eq!(beton.count, Some(4.0));
        let d28 = ls.iter().find(|l| l.item.material.contains("28")).unwrap();
        assert_eq!(d28.count, Some(4.0));
        // Ankerning o'z jadvali esa o'z soniga ko'payadi.
        let d20 = ls.iter().find(|l| l.item.material.contains("Ø20")).unwrap();
        assert_eq!(d20.count, Some(2.0));
    }

    /// Ro'yxat jadvalidagi konstruksiyadan keyingi qator uning soniga
    /// ko'paytirilmaydi.
    ///
    /// Haqiqiy loyihada shunday bo'lgan: «Обвязочный пояс Об1 — 804 п.м»
    /// dan keyin «Объем кирпича — 1050 м3» turadi va g'isht hajmi 804 ga
    /// ko'payib ketgan.
    #[test]
    fn a_register_row_does_not_multiply_what_follows() {
        let row = |pos: &str, name: &str, qty: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            note: note.into(),
            ..Default::default()
        };
        let all = vec![SpecTable {
            page: 23,
            rows: vec![
                row("См1", "Сердечник монолитный См1", "", "294 п.м."),
                row("Об1", "Обвязочный пояс Об1", "", "804 п.м."),
                row("", "Объем кирпича 380мм", "", "1050 м3"),
            ],
            off: false,
            owner: String::new(),
            ai: false,
        }];
        let list = constructs(&all);
        assert_eq!(list.len(), 2, "{list:?}");
        assert_eq!(list[1].count, 804.0);
        let ls = lines(&all, &list);
        let brick = ls
            .iter()
            .find(|l| l.item.material.contains("кирпича"))
            .unwrap();
        assert_eq!(brick.count, None);
        assert_eq!(brick.total(), 1050.0);
    }

    /// KMD varag'idagi metall tanlovi: massasi jami, qayta ko'paytirilmaydi.
    #[test]
    fn a_metal_takeoff_sheet_is_read_as_totals() {
        let pieces = vec![
            p(344.0, 642.0, "Профиль"),
            p(396.0, 642.0, "Материал"),
            p(455.0, 637.0, "общ"),
            p(493.0, 637.0, "с"),
            // зд-6, С245, 3309,12, 3415.0
            p(351.0, 623.0, "зд"),
            p(360.0, 623.0, "-6"),
            p(407.0, 623.0, "С"),
            p(411.0, 623.0, "245"),
            p(450.0, 623.0, "3309,12"),
            p(496.0, 623.0, "3415.0"),
            p(341.0, 603.0, "□"),
            p(347.0, 603.0, "80х80х4"),
            p(407.0, 603.0, "С"),
            p(411.0, 603.0, "255"),
            p(452.0, 603.0, "3856,8"),
            p(494.0, 603.0, "3980.22"),
            // Jami qatori sanalmaydi — aks holda massa ikkilanardi.
            p(345.0, 590.0, "Итого"),
            p(450.0, 590.0, "7165,92"),
        ];
        let t = tables(&pieces, 48);
        assert_eq!(t.len(), 1, "{t:#?}");
        assert_eq!(t[0].rows.len(), 2, "{:#?}", t[0].rows);
        let it = item(&t[0].rows[0]).expect("prokat");
        assert_eq!(it.kind, Kind::Steel);
        assert_eq!(it.material, "Prokat зд-6 С245");
        assert!((it.amount - 3309.12).abs() < 1e-9);
        let it = item(&t[0].rows[1]).unwrap();
        assert_eq!(it.material, "Prokat Kvadrat quvur 80х80х4 С255");

        let ls = lines(&t, &[]);
        assert_eq!(ls.len(), 2);
        let sum: f64 = ls.iter().map(|l| l.total()).sum();
        assert!((sum - 7165.92).abs() < 1e-6, "{sum}");
    }

    /// Hisobdan chiqarilgan jadval yig'indiga kirmaydi.
    #[test]
    fn a_switched_off_table_is_not_counted() {
        let mut tk = Takeoff {
            file: "x.pdf".into(),
            pages: 1,
            tables: vec![SpecTable {
                page: 35,
                rows: vec![SpecRow {
                    name: "Бетон В15".into(),
                    qty: "813.2".into(),
                    note: "м3".into(),
                    ..Default::default()
                }],
                off: false,
                owner: String::new(),
                ai: false,
            }],
            constructs: Vec::new(),
            ..Default::default()
        };
        assert_eq!(tk.lines().len(), 1);
        tk.tables[0].off = true;
        assert!(tk.lines().is_empty());
    }

    /// Quvur diametri armatura emas.
    #[test]
    fn a_pipe_diameter_is_not_rebar() {
        let r = SpecRow {
            pos: "2".into(),
            name: "Компенсационный патрубок Ø 100мм".into(),
            qty: "44".into(),
            unit: "шт".into(),
            ..Default::default()
        };
        let it = item(&r).expect("buyum");
        assert_eq!(it.kind, Kind::Other);
        assert_eq!(it.amount, 44.0);
        // Massasiz shveller buyumi ham dona bilan sanaladi.
        let r = SpecRow {
            pos: "12".into(),
            name: "Лоток из швеллера №20 длиной 3,0м".into(),
            qty: "8".into(),
            unit: "шт".into(),
            ..Default::default()
        };
        assert_eq!(item(&r).unwrap().kind, Kind::Other);
    }

    /// Tor pozitsiya ustuni: raqam nomga yopishib ketmaydi.
    #[test]
    fn a_narrow_position_column_keeps_its_number() {
        let pieces = vec![
            p(59.0, 788.0, "Позиция"),
            p(148.0, 788.0, "Наименование и техническая характеристика"),
            p(882.0, 796.0, "Единица"),
            p(947.0, 796.0, "Коли-"),
            p(82.0, 740.0, "1"),
            p(120.0, 740.0, "Воронка водосточная диаметром 100мм"),
            p(898.0, 742.0, "шт"),
            p(951.0, 742.0, "44"),
            p(82.0, 716.0, "2"),
            p(120.0, 717.0, "Компенсационный патрубок"),
            p(898.0, 719.0, "шт"),
            p(951.0, 719.0, "44"),
        ];
        let t = tables(&pieces, 38);
        assert_eq!(t.len(), 1, "{t:#?}");
        let r = &t[0].rows[0];
        assert_eq!(r.pos, "1");
        assert_eq!(r.name, "Воронка водосточная диаметром 100мм");
        assert_eq!(r.unit, "шт");
        assert_eq!(r.qty, "44");
    }

    /// Ustunlararo yoyilgan sarlavha bitta nom bo'lib o'qiladi, ikkinchi
    /// «Масса» ustuni — jami massa.
    #[test]
    fn a_heading_across_columns_stays_whole() {
        let pieces = vec![
            p(100.0, 500.0, "Поз."),
            p(160.0, 500.0, "Обозначение"),
            p(340.0, 500.0, "Наименование"),
            p(470.0, 500.0, "Кол"),
            p(500.0, 506.0, "Масса"),
            p(560.0, 506.0, "Масса"),
            // Sarlavha belgi ustunidan boshlanib nom ustuniga o'tadi.
            p(280.0, 480.0, "Монолитная перемычка Пм"),
            p(394.0, 480.0, "-1 (на 1 п.м.)"),
            p(104.0, 460.0, "1"),
            p(150.0, 460.0, "ГОСТ 5781-82"),
            p(330.0, 460.0, "Ø16 A-III м.п."),
            p(475.0, 460.0, "6"),
            p(505.0, 460.0, "1.58"),
            p(562.0, 460.0, "9.48"),
            // Hajm nomga yopishib yozilgan.
            p(280.0, 440.0, "Бетон кл"),
            p(320.0, 440.0, ".В7.5(подготовка"),
            p(400.0, 440.0, ")1.12"),
        ];
        let t = tables(&pieces, 23);
        assert_eq!(t.len(), 1, "{t:#?}");
        let r = &t[0].rows;
        assert_eq!(r[0].name, "Монолитная перемычка Пм-1 (на 1 п.м.)");
        assert!(r[0].designation.is_empty());
        assert_eq!(r[1].mass, "1.58");
        assert_eq!(r[1].note, "9.48");
        let it = item(&r[1]).expect("armatura");
        assert!((it.amount - 9.48).abs() < 1e-9);
        let it = item(&r[2]).expect("beton");
        assert_eq!(it.material, "Beton B7.5");
        assert!((it.amount - 1.12).abs() < 1e-9);
    }

    /// Nom raqamdan yuqoriroq yozilgan qator bitta bo'lib o'qiladi;
    /// miqdori matn ichida yozilgan qator ham taniladi.
    #[test]
    fn an_offset_name_joins_its_row_and_inline_amounts_are_read() {
        let pieces = vec![
            p(100.0, 500.0, "Поз."),
            p(160.0, 500.0, "Обозначение"),
            p(340.0, 500.0, "Наименование"),
            p(470.0, 500.0, "Кол."),
            p(540.0, 500.0, "Примеч."),
            p(104.0, 470.0, "Сп-1"),
            p(330.0, 471.0, "Сп-1, L=11450x1000"),
            p(475.0, 470.0, "528"),
            p(330.0, 449.0, "Сп-2, L=11350x1000"),
            p(104.0, 443.0, "Сп-2"),
            p(475.0, 443.0, "264"),
            p(150.0, 420.0, "Водосборный лоток сталь листовая ГОСТ "),
            p(338.0, 420.0, "19903-90* б=2 мм "),
            p(424.0, 420.0, "- 14.67 "),
            p(466.0, 420.0, "тонн"),
        ];
        let t = tables(&pieces, 34);
        assert_eq!(t.len(), 1, "{t:#?}");
        let r = &t[0].rows;
        assert_eq!(r.len(), 3, "{r:#?}");
        assert_eq!(r[1].pos, "Сп-2");
        assert_eq!(r[1].name, "Сп-2, L=11350x1000");
        assert_eq!(r[1].qty, "264");
        let it = item(&r[2]).expect("lotok");
        assert_eq!(it.unit, "t");
        assert!((it.amount - 14.67).abs() < 1e-9);
        assert!(
            it.material.starts_with("Водосборный лоток"),
            "{}",
            it.material
        );
        // Aniq shakl bo'lmasa — taxmin qilinmaydi.
        assert!(inline("Труба - примерно 5 тонн").is_none());
        assert!(inline("Уголок 50х5").is_none());
    }

    /// Shtampida bitta konstruksiya yozilgan varaq uning soniga
    /// ko'paytiriladi; ikkitasi yozilgan bo'lsa — yo'q.
    #[test]
    fn a_sheet_named_after_one_construct_is_multiplied() {
        let stamp = |title: &[(f64, &str)]| {
            let mut v = vec![
                p(1433.0, 102.0, "Изм."),
                p(1460.0, 102.0, "Кол. уч."),
                p(1587.0, 102.0, "Дата"),
            ];
            v.extend(title.iter().map(|(x, t)| p(*x, 30.0, t)));
            v
        };
        let list = vec![
            Construct {
                mark: "К3".into(),
                name: "2К138-6М3-c-а".into(),
                count: 12.0,
                unit: "dona".into(),
                page: 16,
                register: true,
            },
            Construct {
                mark: "К4".into(),
                name: "2К138-6М3-c-б".into(),
                count: 12.0,
                unit: "dona".into(),
                page: 16,
                register: true,
            },
        ];
        let one = stamp(&[(1660.0, "К"), (1665.0, "3(2"), (1680.0, "К80-6М3-c-а)")]);
        assert_eq!(sheet_owner(&one, &list).as_deref(), Some("К3"));
        let two = stamp(&[(1660.0, "Колонны К3, К4")]);
        assert_eq!(sheet_owner(&two, &list), None);
        assert_eq!(
            sheet_owner(&stamp(&[(1660.0, "Общие данные")]), &list),
            None
        );

        let table = SpecTable {
            page: 19,
            rows: vec![SpecRow {
                pos: "4".into(),
                name: "Ø8 A-I L=680".into(),
                qty: "36".into(),
                mass: "0.27".into(),
                note: "9.72".into(),
                ..Default::default()
            }],
            off: false,
            owner: "К3".into(),
            ai: false,
        };
        let ls = lines(&[table], &list);
        assert_eq!(ls.len(), 1);
        assert_eq!(ls[0].count, Some(12.0));
        assert!((ls[0].total() - 9.72 * 12.0).abs() < 1e-9);
    }

    /// Nomi bo'yicha havola o'z jadvalini topadi; tarkibi yozilmagan
    /// buyum massasi bilan hisobga kiradi.
    #[test]
    fn a_reference_by_name_and_a_bought_part() {
        let row = |pos: &str, name: &str, qty: &str, mass: &str, note: &str| SpecRow {
            pos: pos.into(),
            name: name.into(),
            qty: qty.into(),
            mass: mass.into(),
            note: note.into(),
            ..Default::default()
        };
        let list = vec![Construct {
            mark: "К3".into(),
            name: String::new(),
            count: 12.0,
            unit: "dona".into(),
            page: 16,
            register: true,
        }];
        let main = SpecTable {
            page: 19,
            rows: vec![
                // Jadval boshidagi sarlavhalar uni tugatmaydi.
                row("", "Сборочные единицы", "", "", ""),
                row("1", "Изделие закладное УП2-8", "4", "3.04кг", ""),
                row("3", "Изделие закладное МН2", "2", "89.6кг.", ""),
            ],
            off: false,
            owner: "К3".into(),
            ai: false,
        };
        let parts = SpecTable {
            page: 19,
            rows: vec![
                row("", "Изделие закладное МН2", "", "", "89.6"),
                row("13", "Лист 16x300x500", "2", "18.8", "37.6"),
            ],
            off: false,
            owner: "К3".into(),
            ai: false,
        };
        let ls = lines(&[main, parts], &list);
        assert_eq!(ls.len(), 2, "{ls:#?}");
        // УП2-8: tarkibi yo'q — 4 × 3.04 kg, 12 ta kolonnaga.
        assert_eq!(ls[0].item.material, "Изделие закладное УП2-8");
        assert!((ls[0].total() - 4.0 * 3.04 * 12.0).abs() < 1e-9);
        // МН2: tarkibi alohida jadvalda, 2 × 12 = 24 marta.
        assert_eq!(ls[1].count, Some(24.0));
        assert!((ls[1].total() - 37.6 * 24.0).abs() < 1e-9);
    }

    /// Chizmadagi «поз.1 - …» izohi jadval sarlavhasi emas.
    #[test]
    fn a_drawing_note_is_not_a_table_header() {
        assert_eq!(header_kind("Поз."), Some(Col::Pos));
        assert_eq!(header_kind("Позиция"), Some(Col::Pos));
        assert_eq!(header_kind("поз.1 - Строповочные петли;"), None);
    }

    /// Boshqa varaqda aynan qayta berilgan jadval ikki marta sanalmaydi.
    #[test]
    fn an_identical_table_on_another_sheet_is_counted_once() {
        let table = |page: usize, owner: &str| SpecTable {
            page,
            rows: vec![SpecRow {
                name: "Бетон В15".into(),
                qty: "10".into(),
                note: "м3".into(),
                ..Default::default()
            }],
            off: false,
            owner: owner.into(),
            ai: false,
        };
        let both = [table(34, ""), table(76, "")];
        assert_eq!(lines(&both, &[]).len(), 1);
        assert_eq!(repeats(&both, &[]), vec![("Бетон В15".to_string(), 76)]);
        // Egasi boshqa — bu boshqa konstruksiya, takror emas.
        assert_eq!(lines(&[table(19, "К3"), table(20, "К4")], &[]).len(), 2);
    }

    /// Orientir narx faqat manbada bor material uchun beriladi.
    #[test]
    fn reference_prices_cover_only_what_the_source_lists() {
        assert_eq!(
            orientir(Kind::Concrete, "Beton B20", "m3"),
            Some((675_000.0, ORIENTIR_CONCRETE))
        );
        assert_eq!(orientir(Kind::Concrete, "Beton B30", "m3"), None);
        assert_eq!(
            orientir(Kind::Rebar, "Armatura Ø12 A-III", "kg").map(|p| p.0),
            Some(8_350.0)
        );
        assert_eq!(
            orientir(Kind::Rebar, "Armatura Ø28 A-III", "kg").map(|p| p.0),
            Some(8_200.0)
        );
        // Praysda yo'q ingichka armatura — eng yaqin narx bilan, va bu
        // manbada yozilgan.
        assert_eq!(
            orientir(Kind::Rebar, "Armatura Ø8 A-I", "kg"),
            Some((8_350.0, ORIENTIR_THIN))
        );
        assert_eq!(
            orientir(Kind::Rebar, "Armatura Ø5 B-I", "kg"),
            Some((10_400.0, ORIENTIR_WIRE))
        );
        assert_eq!(
            orientir(Kind::Steel, "Prokat Kvadrat quvur 80х80х4 С255", "kg").map(|p| p.0),
            Some(10_700.0)
        );
        assert_eq!(
            orientir(Kind::Steel, "Изделие закладное УП2-8", "kg"),
            Some((12_400.0, ORIENTIR_PARTS))
        );
        // Hisoblab chiqariladigan narxlar.
        let (panel, _) = orientir(Kind::Other, "Сп-1,   L=11450x1000", "dona").unwrap();
        assert_eq!(panel, (150_522.0_f64 * 11.45).round());
        let (pipe, _) = orientir(
            Kind::Other,
            "Трубы стальные электросварные прямошовные Ø159*4,5",
            "m",
        )
        .unwrap();
        assert_eq!(pipe, (10_500.0_f64 * (159.0 - 4.5) * 4.5 * 0.02466).round());
        assert_eq!(
            orientir(Kind::Other, "Объем кирпича 380мм", "m3").map(|p| p.0),
            Some(400_000.0)
        );
        // Manbasi ham, hisoblash yo'li ham yo'q — narx o'ylab topilmaydi.
        assert_eq!(
            orientir(Kind::Other, "Вертикальная связь ВС7", "dona"),
            None
        );
        assert_eq!(orientir(Kind::Other, "Сп-2", "dona"), None);
        assert_eq!(
            orientir(Kind::Other, "Ревизия диаметром 100мм (на болтах)", "dona"),
            None
        );
    }

    /// AI o'qishi qo'llanadi, qoida nusxasi saqlanadi, farq ko'rinadi va
    /// voz kechilsa hammasi joyiga qaytadi.
    #[test]
    fn an_ai_reading_replaces_the_page_and_can_be_reverted() {
        let rebar = |total: &str| SpecRow {
            pos: "1".into(),
            name: "Ø14 A-III L=2550".into(),
            qty: "22".into(),
            mass: "3.09".into(),
            note: total.into(),
            ..Default::default()
        };
        let table = |page: usize, total: &str| SpecTable {
            page,
            rows: vec![rebar(total)],
            ..Default::default()
        };
        let mut tk = Takeoff {
            file: "x.pdf".into(),
            pages: 20,
            tables: vec![table(13, "68.0"), table(14, "68.0")],
            ..Default::default()
        };
        let before = tk.clone();
        tk.apply_ai(
            vec![
                // 13-varaq: AI boshqa son o'qidi.
                (13, String::new(), vec![table(0, "70.0")]),
                // 14-varaq: AI hech narsa topmadi — varaq o'zgarmaydi.
                (14, String::new(), vec![]),
            ],
            AiInfo {
                model: "m".into(),
                tokens: 10,
                ..Default::default()
            },
        );
        assert_eq!(tk.tables.len(), 2);
        assert!(tk.tables[0].ai && tk.tables[0].page == 13);
        assert!(!tk.tables[1].ai);
        assert_eq!(tk.rule_tables.len(), 1);
        assert_eq!(tk.ai.as_ref().unwrap().pages, vec![13]);
        let d = tk.diffs();
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].rule_kg, d[0].ai_kg), (68.0, 70.0));
        assert!(!d[0].agrees());
        // Hisob AI o'qiganidan chiqadi.
        assert_eq!(tk.lines()[0].item.amount, 70.0);

        tk.revert_ai();
        assert_eq!(tk.tables.len(), before.tables.len());
        assert!(tk.tables.iter().all(|t| !t.ai));
        assert!(tk.rule_tables.is_empty() && tk.ai.is_none());
        assert_eq!(tk.lines()[0].item.amount, 68.0);
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
