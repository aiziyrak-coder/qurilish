//! Smeta: loyihadan ishlar, materiallar va narx.
//!
//! Yo'l olti bosqichdan iborat: yuklash → savollar → obyekt ma'lumoti →
//! spetsifikatsiya → smeta → mijozga taklif (KP). Bu modul — uning
//! **hisob yadrosi**: tuzilma, miqdor manbasi, narx tanlash va jamilar.
//! Tarmoq ham, ekran ham bu yerda yo'q.
//!
//! Uchta qoida butun modul bo'ylab amal qiladi:
//!
//! 1. **Har miqdorning manbasi ko'rinadi** ([`Source`]): loyihadan (varaq
//!    raqami bilan), hisob (formulasi bilan), me'yor bo'yicha yoki taxmin.
//!    Taxmin yashirilmaydi — u odam ko'zdan kechirishi kerak bo'lgan joy.
//! 2. **Sonni dastur hisoblaydi.** AI formulani yozadi, natijani esa
//!    [`eval`] chiqaradi; AI aytgan son formulaga to'g'ri kelmasa, formula
//!    ustun turadi.
//! 3. **Narx o'ylab topilmaydi.** Tartib: shu smetadagi narx → kompaniya
//!    katalogi → narx bazasi. Hech birida bo'lmasa — narx yo'q, qator
//!    jamiga kirmaydi va sanab ko'rsatiladi.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Miqdor qayerdan olingani.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Source {
    /// Loyihaning o'zidan: spetsifikatsiya yoki hajmlar vedomosti.
    Project { page: Option<usize> },
    /// Obyekt o'lchamlaridan hisoblangan; formula ko'rsatiladi.
    Calc { formula: String },
    /// Sarf me'yori yoki texnologiya bo'yicha.
    Standard { note: String },
    /// Ma'lumot yetmadi — qabul qilingan qiymat.
    Assumption { note: String },
    /// Odam qo'lda kiritgan.
    #[default]
    Manual,
}

impl Source {
    /// Qisqa belgi — tarjima kaliti.
    pub fn badge(&self) -> &'static str {
        match self {
            Source::Project { .. } => "sm_src_project",
            Source::Calc { .. } => "sm_src_calc",
            Source::Standard { .. } => "sm_src_standard",
            Source::Assumption { .. } => "sm_src_assumption",
            Source::Manual => "sm_src_manual",
        }
    }

    /// Tushuntirish: varaq, formula yoki izoh.
    pub fn detail(&self) -> String {
        match self {
            Source::Project { page: Some(p) } => format!("{} {p}", crate::i18n::t("pdf_page")),
            Source::Project { page: None } => String::new(),
            Source::Calc { formula } => formula.clone(),
            Source::Standard { note } | Source::Assumption { note } => note.clone(),
            Source::Manual => String::new(),
        }
    }
}

/// Material yoki resurs (texnika, yetkazish).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Resource {
    pub name: String,
    pub qty: f64,
    pub unit: String,
    pub source: Source,
    /// Faqat shu smeta uchun qo'yilgan narx.
    #[serde(default)]
    pub price: Option<f64>,
}

/// Ish va uning materiallari.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Work {
    pub name: String,
    pub qty: f64,
    pub unit: String,
    pub source: Source,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub materials: Vec<Resource>,
}

/// Bosqich: poydevor, karkas, tom…
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Stage {
    pub name: String,
    #[serde(default)]
    pub works: Vec<Work>,
    /// Bosqich ustamasi, foizda (manfiy — chegirma).
    #[serde(default)]
    pub markup: f64,
}

/// Obyekt haqidagi ma'lumot: maydon, hajm, o'lcham.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Fact {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub page: Option<usize>,
}

/// Loyihada ko'rinmagan narsa haqida savol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Question {
    pub topic: String,
    pub text: String,
    #[serde(default)]
    pub options: Vec<String>,
    /// Bo'sh — javob berilmagan (ma'lumot yo'q).
    #[serde(default)]
    pub answer: String,
}

/// Bitta varaqdan AI ajratib olgan narsa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PageDigest {
    pub page: usize,
    #[serde(default)]
    pub sheet: String,
    /// Bo'lim: АР, КЖ, КМ, ВК…
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub facts: Vec<Fact>,
    /// Spetsifikatsiyadan boshqa jadvallar (hajmlar vedomosti,
    /// eksplikatsiya, pardoz vedomosti): sarlavha, ustunlar, qatorlar.
    #[serde(default)]
    pub lists: Vec<List>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct List {
    pub title: String,
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

/// Mijozga taklif (KP) sozlamalari.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Offer {
    pub title: String,
    pub company: String,
    pub contacts: String,
    pub customer: String,
    /// Taklif necha kun amal qiladi.
    pub valid_days: u32,
    /// Shartlar.
    pub terms: String,
    /// Narxga kirmaydigan ishlar — alohida bo'lim.
    #[serde(default)]
    pub excluded: String,
    /// Mijozga bosqichlar ichidagi ishlar ham ko'rsatilsinmi.
    pub detailed: bool,
    /// Hujjat rangi (urg'u): `ACCENTS` dagi o'rin.
    #[serde(default)]
    pub accent: usize,
    /// Bo'limlar: «obyekt raqamlarda», to'lov jadvali.
    #[serde(default = "yes")]
    pub show_numbers: bool,
    #[serde(default = "yes")]
    pub show_schedule: bool,
    /// Avans, foizda; qolgani bosqichlar bo'yicha to'lanadi.
    #[serde(default = "advance")]
    pub advance_pct: f64,
}

fn yes() -> bool {
    true
}
fn advance() -> f64 {
    30.0
}

/// Hujjat uchun ranglar: nom va RGB.
pub const ACCENTS: [(&str, [u8; 3]); 4] = [
    ("Ko'k", [31, 78, 160]),
    ("Yashil", [34, 120, 80]),
    ("Terrakot", [176, 86, 48]),
    ("Grafit", [60, 64, 72]),
];

impl Default for Offer {
    fn default() -> Self {
        Offer {
            title: String::new(),
            company: String::new(),
            contacts: String::new(),
            customer: String::new(),
            valid_days: 14,
            terms: String::new(),
            excluded: String::new(),
            detailed: true,
            accent: 0,
            show_numbers: true,
            show_schedule: true,
            advance_pct: 30.0,
        }
    }
}

/// To'lov jadvalining qatori.
#[derive(Debug, Clone, PartialEq)]
pub struct Payment {
    pub title: String,
    pub amount: f64,
    pub pct: f64,
}

/// To'lov jadvali: avans, keyin har bosqich o'z qiymatiga mutanosib.
///
/// Jami noldan katta bo'lmasa jadval bo'sh — foizlarni bo'sh summaga
/// yozish ma'nosiz.
pub fn schedule(offer: &Offer, stages: &[(String, f64)], total: f64) -> Vec<Payment> {
    if total <= 0.0 {
        return Vec::new();
    }
    let adv = offer.advance_pct.clamp(0.0, 100.0);
    let mut out = vec![Payment {
        title: crate::i18n::t("sm_pay_advance").to_string(),
        amount: total * adv / 100.0,
        pct: adv,
    }];
    let rest = 100.0 - adv;
    let sum: f64 = stages.iter().map(|s| s.1).sum();
    if sum > 0.0 && rest > 0.0 {
        for (name, v) in stages.iter().filter(|s| s.1 > 0.0) {
            let pct = rest * v / sum;
            out.push(Payment {
                title: name.clone(),
                amount: total * pct / 100.0,
                pct,
            });
        }
    }
    out
}

// ================================================================= Xolst

/// Loyihasiz obyekt: reja xolstda chiziladi.
///
/// Namunadagidek — mijoz rejani yuborgan bo'lsa, u xolstda takrorlanadi
/// va shundan maydon, perimetr, devor yuzasi chiqadi. Bu **hisob**:
/// nuqtalar metrda, maydon Gauss formulasi bilan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sketch {
    /// Tashqi devor konturi, metrda (soat strelkasi bo'yicha yoki teskari).
    pub points: Vec<(f32, f32)>,
    pub floors: u32,
    /// Qavat balandligi, m.
    pub height: f32,
    /// Ichki devorlar uzunligi, m (bir qavatga).
    pub inner_walls: f32,
    pub windows: u32,
    pub doors: u32,
    pub roof: String,
    pub foundation: String,
    pub walls: String,
}

impl Default for Sketch {
    fn default() -> Self {
        Sketch {
            points: Vec::new(),
            floors: 1,
            height: 3.0,
            inner_walls: 0.0,
            windows: 0,
            doors: 1,
            roof: String::new(),
            foundation: String::new(),
            walls: String::new(),
        }
    }
}

impl Sketch {
    /// Kontur maydoni, m².
    pub fn area(&self) -> f64 {
        let n = self.points.len();
        if n < 3 {
            return 0.0;
        }
        let mut s = 0.0;
        for i in 0..n {
            let (x1, y1) = self.points[i];
            let (x2, y2) = self.points[(i + 1) % n];
            s += (x1 as f64) * (y2 as f64) - (x2 as f64) * (y1 as f64);
        }
        (s / 2.0).abs()
    }

    /// Perimetr, m.
    pub fn perimeter(&self) -> f64 {
        let n = self.points.len();
        if n < 2 {
            return 0.0;
        }
        (0..n)
            .map(|i| {
                let (x1, y1) = self.points[i];
                let (x2, y2) = self.points[(i + 1) % n];
                (((x2 - x1) as f64).powi(2) + ((y2 - y1) as f64).powi(2)).sqrt()
            })
            .sum()
    }

    /// Xolstdan chiqadigan ko'rsatkichlar — AI uchun ham, ekran uchun ham.
    pub fn facts(&self) -> Vec<Fact> {
        let f = |name: &str, value: String, unit: &str| Fact {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            page: None,
        };
        let area = self.area();
        let per = self.perimeter();
        let floors = self.floors.max(1) as f64;
        let h = self.height as f64;
        let mut out = vec![
            f("Площадь застройки (по контуру)", format!("{area:.1}"), "м2"),
            f(
                "Общая площадь (контур × этажи)",
                format!("{:.1}", area * floors),
                "м2",
            ),
            f("Периметр наружных стен", format!("{per:.1}"), "м"),
            f("Этажность", self.floors.to_string(), "эт."),
            f("Высота этажа", format!("{h:.2}"), "м"),
            f(
                "Площадь наружных стен (периметр × высота × этажи)",
                format!("{:.1}", per * h * floors),
                "м2",
            ),
            f(
                "Строительный объём",
                format!("{:.1}", area * h * floors),
                "м3",
            ),
            f("Окна", self.windows.to_string(), "шт"),
            f("Двери", self.doors.to_string(), "шт"),
        ];
        if self.inner_walls > 0.0 {
            out.push(f(
                "Внутренние стены (длина, все этажи)",
                format!("{:.1}", self.inner_walls as f64 * floors),
                "м",
            ));
        }
        for (name, v) in [
            ("Тип кровли", &self.roof),
            ("Тип фундамента", &self.foundation),
            ("Материал стен", &self.walls),
        ] {
            if !v.trim().is_empty() {
                out.push(f(name, v.clone(), ""));
            }
        }
        out
    }
}

/// AI tekshiruvining topilmasi.
///
/// `kind`: `qty` — miqdor shubhali (yangi qiymat taklifi bilan), `missing`
/// — yetishmayotgan ish, `dup` — takror, `price` — narx, `ask` — mijozdan
/// so'rash. Dastur topilmani **o'zi qo'llamaydi**: har biri odam tugmasi
/// bilan qo'llanadi yoki yopiladi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Finding {
    pub kind: String,
    pub text: String,
    /// Qator manzili: `s3.w2` (ish) yoki `s3.w2.m1` (material), `s3` (bosqich).
    #[serde(default)]
    pub id: String,
    /// Taklif qilingan miqdor (qty) yoki yangi ish miqdori (missing).
    #[serde(default)]
    pub qty: Option<f64>,
    #[serde(default)]
    pub unit: String,
    /// Yangi ish nomi (missing).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub done: bool,
}

/// Qator manzilini o'qiydi: `s3.w2.m1` → `(3, Some(2), Some(1))`.
pub fn locate(id: &str) -> Option<(usize, Option<usize>, Option<usize>)> {
    let mut si = None;
    let mut wi = None;
    let mut mi = None;
    for part in id.trim().split('.') {
        if !part.is_char_boundary(1) || part.len() < 2 {
            return None;
        }
        let (tag, num) = part.split_at(1);
        let n: usize = num.parse().ok()?;
        match tag {
            "s" => si = Some(n),
            "w" => wi = Some(n),
            "m" => mi = Some(n),
            _ => return None,
        }
    }
    Some((si?, wi, mi))
}

/// Ish o'chirilgach topilmalardagi manzillarni suradi.
pub fn shift_ids(findings: &mut [Finding], si: usize, removed_wi: usize) {
    for f in findings {
        if let Some((fs, Some(fw), fm)) = locate(&f.id) {
            if fs == si && fw > removed_wi {
                f.id = match fm {
                    Some(m) => format!("s{fs}.w{}.m{m}", fw - 1),
                    None => format!("s{fs}.w{}", fw - 1),
                };
            } else if fs == si && fw == removed_wi && !f.done {
                // O'chirilgan ishga tegishli topilma endi ma'nosiz.
                f.done = true;
            }
        }
    }
}

/// Bitta obyektning smetasi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Smeta {
    /// Obyektning qisqa tavsifi (AI loyihadan tuzgan).
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub digest: Vec<PageDigest>,
    #[serde(default)]
    pub facts: Vec<Fact>,
    #[serde(default)]
    pub questions: Vec<Question>,
    #[serde(default)]
    pub stages: Vec<Stage>,
    /// Butun smetaga ustama, foizda.
    #[serde(default)]
    pub markup: f64,
    #[serde(default)]
    pub offer: Offer,
    /// Loyiha o'rniga xolstda chizilgan reja.
    #[serde(default)]
    pub sketch: Option<Sketch>,
    /// AI tekshiruvi: qisqa xulosa va topilmalar.
    #[serde(default)]
    pub review_summary: String,
    #[serde(default)]
    pub review: Vec<Finding>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub tokens: u32,
}

// ================================================================ Formula

/// Arifmetik ifodani hisoblaydi: `2*(12+8)*3,0`.
///
/// Faqat sonlar, `+ - * /` va qavslar. Boshqa har qanday belgi — xato:
/// tushunilmagan formula taxmin qilib hisoblanmaydi.
pub fn eval(expr: &str) -> Option<f64> {
    let chars: Vec<char> = expr
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            ',' => '.',
            '×' | 'х' | 'x' | '·' => '*',
            '÷' | ':' => '/',
            '−' | '–' => '-',
            other => other,
        })
        .collect();
    let mut at = 0;
    let v = sum(&chars, &mut at)?;
    (at == chars.len() && v.is_finite()).then_some(v)
}

fn sum(s: &[char], at: &mut usize) -> Option<f64> {
    let mut v = product(s, at)?;
    while let Some(op) = s.get(*at).copied().filter(|c| *c == '+' || *c == '-') {
        *at += 1;
        let r = product(s, at)?;
        v = if op == '+' { v + r } else { v - r };
    }
    Some(v)
}

fn product(s: &[char], at: &mut usize) -> Option<f64> {
    let mut v = atom(s, at)?;
    while let Some(op) = s.get(*at).copied().filter(|c| *c == '*' || *c == '/') {
        *at += 1;
        let r = atom(s, at)?;
        if op == '/' && r == 0.0 {
            return None;
        }
        v = if op == '*' { v * r } else { v / r };
    }
    Some(v)
}

fn atom(s: &[char], at: &mut usize) -> Option<f64> {
    match s.get(*at)? {
        '(' => {
            *at += 1;
            let v = sum(s, at)?;
            (s.get(*at) == Some(&')')).then(|| *at += 1)?;
            Some(v)
        }
        '-' => {
            *at += 1;
            atom(s, at).map(|v| -v)
        }
        _ => {
            let start = *at;
            while s.get(*at).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
                *at += 1;
            }
            let text: String = s[start..*at].iter().collect();
            text.parse().ok()
        }
    }
}

// ================================================================== Narx

/// Kompaniya katalogi: nom va birlik bo'yicha narx. Barcha obyektlarga
/// umumiy — bir marta kiritilgan narx keyingi smetalarda o'zi chiqadi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Catalog {
    #[serde(default)]
    pub works: BTreeMap<String, f64>,
    #[serde(default)]
    pub materials: BTreeMap<String, f64>,
    /// O'zgarishlar tarixi: `(kun, ish?, kalit, narx)` — narx qachon va
    /// nimaga o'zgargani pozitsiya bo'yicha ko'rinib tursin.
    #[serde(default)]
    pub log: Vec<(String, bool, String, f64)>,
}

impl Catalog {
    /// Narxni yozadi va tarixga qo'shadi; nol — o'chiradi.
    pub fn set(&mut self, work: bool, key: &str, price: f64, day: &str) {
        let map = if work {
            &mut self.works
        } else {
            &mut self.materials
        };
        if price > 0.0 {
            map.insert(key.to_string(), price);
        } else {
            map.remove(key);
        }
        self.log
            .push((day.to_string(), work, key.to_string(), price));
        // Tarix cheksiz o'smasin.
        if self.log.len() > 5000 {
            let extra = self.log.len() - 5000;
            self.log.drain(..extra);
        }
    }

    /// Pozitsiya tarixi, yangisidan eskisiga.
    pub fn history(&self, work: bool, key: &str) -> Vec<(String, f64)> {
        self.log
            .iter()
            .rev()
            .filter(|l| l.1 == work && l.2 == key)
            .map(|l| (l.0.clone(), l.3))
            .collect()
    }
}

/// Katalog kalitidan nom va birlik.
pub fn split_key(key: &str) -> (&str, &str) {
    key.rsplit_once('|').unwrap_or((key, ""))
}

/// Katalog kaliti: nom kichik harfda, ortiqcha bo'shliqsiz, birlik bilan.
pub fn key(name: &str, unit: &str) -> String {
    let n = name
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    format!("{n}|{}", unit.trim().to_lowercase())
}

/// Narx qayerdan olingani.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Faqat shu smetada qo'yilgan.
    Line,
    /// Kompaniya katalogidan.
    Catalog,
    /// Yuklangan narx bazasidan (prays).
    Book,
    /// AI taklif qilgan taxminiy narx — tekshirilmagan, qabul qilinmagan.
    Hint,
    None,
}

impl Origin {
    pub fn label(self) -> &'static str {
        match self {
            Origin::Line => "sm_price_line",
            Origin::Catalog => "sm_price_catalog",
            Origin::Book => "sm_price_book",
            Origin::Hint => "sm_price_hint",
            Origin::None => "sm_price_none",
        }
    }

    /// Odam tasdiqlagan narxmi (shu smeta, katalog, prays).
    pub fn trusted(self) -> bool {
        matches!(self, Origin::Line | Origin::Catalog | Origin::Book)
    }
}

/// Narxlangan qator.
#[derive(Debug, Clone, PartialEq)]
pub struct Priced {
    pub price: Option<f64>,
    pub origin: Origin,
    /// Summa; narx yo'q bo'lsa — yo'q (nol emas).
    pub sum: Option<f64>,
}

/// O'xshash nom bo'yicha narx qidiruvi: `(ishmi, nom, birlik)` → narx va
/// manba.
pub type Lookup<'a> = dyn Fn(bool, &str, &str) -> Option<(f64, Origin)> + 'a;

/// Narx manbalari — `view` ga beriladi.
pub struct Sources<'a> {
    /// Aniq kalit bo'yicha katalog.
    pub catalog: &'a Catalog,
    /// Nom bo'yicha o'xshash qidiruv: katalog yoki narx bazasi;
    /// qaytaradi narx va qayerdan topilgani.
    pub lookup: &'a Lookup<'a>,
    /// AI taklif qilgan taxminiy narxlar, kalit bo'yicha.
    pub hints: &'a BTreeMap<String, f64>,
}

fn priced(own: Option<f64>, work: bool, name: &str, unit: &str, qty: f64, src: &Sources) -> Priced {
    let positive = |p: &f64| *p > 0.0;
    let k = key(name, unit);
    let map = if work {
        &src.catalog.works
    } else {
        &src.catalog.materials
    };
    let (price, origin) = if let Some(p) = own.filter(positive) {
        (Some(p), Origin::Line)
    } else if let Some(p) = map.get(&k).copied().filter(positive) {
        (Some(p), Origin::Catalog)
    } else if let Some((p, o)) = (src.lookup)(work, name, unit).filter(|x| x.0 > 0.0) {
        (Some(p), o)
    } else if let Some(p) = src.hints.get(&k).copied().filter(positive) {
        (Some(p), Origin::Hint)
    } else {
        (None, Origin::None)
    };
    Priced {
        price,
        origin,
        sum: price.map(|p| p * qty),
    }
}

/// Bosqich ko'rinishi: har ish va material narxi bilan.
#[derive(Debug, Clone, PartialEq)]
pub struct StageView {
    pub works: Vec<(Priced, Vec<Priced>)>,
    /// Ishlar summasi (ustamasiz).
    pub work_sum: f64,
    pub material_sum: f64,
    /// Narxsiz qatorlar — jamiga kirmagan.
    pub missing: usize,
    /// AI taxminiy narxdagi qatorlar — jamiga kirgan, lekin tekshirilmagan.
    pub hinted: usize,
    /// Ustama bilan jami.
    pub total: f64,
}

/// Butun smeta ko'rinishi.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct View {
    pub stages: Vec<StageView>,
    pub work_sum: f64,
    pub material_sum: f64,
    pub missing: usize,
    pub hinted: usize,
    /// Bosqich ustamalari va umumiy ustama bilan jami.
    pub total: f64,
}

/// Smetani narxlaydi va jamilarni chiqaradi.
///
/// Tartib: shu smetadagi narx → katalog (aniq kalit) → o'xshash qidiruv
/// (katalog, keyin narx bazasi) → AI taxmini. Taxmin jamiga kiradi, lekin
/// alohida sanaladi — u odam tasdig'ini kutayotgan narx.
pub fn view(smeta: &Smeta, src: &Sources) -> View {
    let mut out = View::default();
    for stage in &smeta.stages {
        let mut sv = StageView {
            works: Vec::new(),
            work_sum: 0.0,
            material_sum: 0.0,
            missing: 0,
            hinted: 0,
            total: 0.0,
        };
        for w in &stage.works {
            let wp = priced(w.price, true, &w.name, &w.unit, w.qty, src);
            match wp.sum {
                Some(s) => sv.work_sum += s,
                None => sv.missing += 1,
            }
            sv.hinted += (wp.origin == Origin::Hint) as usize;
            let mut ms = Vec::new();
            for m in &w.materials {
                let mp = priced(m.price, false, &m.name, &m.unit, m.qty, src);
                match mp.sum {
                    Some(s) => sv.material_sum += s,
                    None => sv.missing += 1,
                }
                sv.hinted += (mp.origin == Origin::Hint) as usize;
                ms.push(mp);
            }
            sv.works.push((wp, ms));
        }
        sv.total = (sv.work_sum + sv.material_sum) * (1.0 + stage.markup / 100.0);
        out.work_sum += sv.work_sum;
        out.material_sum += sv.material_sum;
        out.missing += sv.missing;
        out.hinted += sv.hinted;
        out.total += sv.total;
        out.stages.push(sv);
    }
    out.total *= 1.0 + smeta.markup / 100.0;
    out
}

/// Narxsiz qatorlar ro'yxati: `(ishmi, nom, birlik)` — AI dan narx
/// so'rash va katalogni to'ldirish uchun. Takrorlar olib tashlangan.
pub fn unpriced(smeta: &Smeta, view: &View) -> Vec<(bool, String, String)> {
    let mut out: Vec<(bool, String, String)> = Vec::new();
    let mut push = |work: bool, name: &str, unit: &str| {
        let k = key(name, unit);
        if !out.iter().any(|o| o.0 == work && key(&o.1, &o.2) == k) {
            out.push((work, name.to_string(), unit.to_string()));
        }
    };
    for (si, st) in smeta.stages.iter().enumerate() {
        let Some(sv) = view.stages.get(si) else {
            continue;
        };
        for (wi, w) in st.works.iter().enumerate() {
            let Some((wp, mps)) = sv.works.get(wi) else {
                continue;
            };
            if !wp.origin.trusted() {
                push(true, &w.name, &w.unit);
            }
            for (mi, m) in w.materials.iter().enumerate() {
                if mps.get(mi).is_some_and(|p| !p.origin.trusted()) {
                    push(false, &m.name, &m.unit);
                }
            }
        }
    }
    out
}

impl Smeta {
    /// Qatorlar soni: ishlar va materiallar.
    #[allow(dead_code)]
    pub fn lines(&self) -> usize {
        self.stages
            .iter()
            .flat_map(|s| &s.works)
            .map(|w| 1 + w.materials.len())
            .sum()
    }

    /// Ko'zdan kechirish kerak bo'lgan qatorlar — taxmin bilan olinganlar.
    pub fn assumptions(&self) -> usize {
        let is = |s: &Source| matches!(s, Source::Assumption { .. });
        self.stages
            .iter()
            .flat_map(|s| &s.works)
            .map(|w| is(&w.source) as usize + w.materials.iter().filter(|m| is(&m.source)).count())
            .sum()
    }

    /// Javob berilgan savollar soni.
    pub fn answered(&self) -> usize {
        self.questions
            .iter()
            .filter(|q| !q.answer.trim().is_empty())
            .count()
    }
}

/// Bosqichlar orasidagi takrorlarni olib tashlaydi.
///
/// Bosqichlar parallel tuziladi va bir-birini ko'rmaydi, shuning uchun
/// bitta loyiha soni («Снятие растительного слоя — 3427,6 м³», «Арматура
/// Ø12 — 147 т») ikki-uch bosqichga tushib qolishi mumkin. Qoida: nomi,
/// birligi va miqdori **aynan bir xil** qator ikkinchi marta uchrasa —
/// takror, birinchisi qoladi. Miqdori boshqa qator takror emas: bir xil
/// material haqiqatan ikki bosqichda ketishi mumkin.
pub fn dedupe(smeta: &mut Smeta) -> usize {
    let mut seen_work: Vec<(String, f64)> = Vec::new();
    let mut seen_mat: Vec<(String, f64)> = Vec::new();
    let mut removed = 0;
    for stage in &mut smeta.stages {
        stage.works.retain(|w| {
            let k = (key(&w.name, &w.unit), w.qty);
            if seen_work
                .iter()
                .any(|s| s.0 == k.0 && (s.1 - k.1).abs() < 1e-9)
            {
                removed += 1 + w.materials.len();
                return false;
            }
            seen_work.push(k);
            true
        });
        for w in &mut stage.works {
            w.materials.retain(|m| {
                // Faqat loyihadan olingan son takror bo'la oladi: me'yor
                // bo'yicha hisoblangan material har ishda o'ziniki.
                if !matches!(m.source, Source::Project { .. }) {
                    return true;
                }
                let k = (key(&m.name, &m.unit), m.qty);
                if seen_mat
                    .iter()
                    .any(|s| s.0 == k.0 && (s.1 - k.1).abs() < 1e-9)
                {
                    removed += 1;
                    return false;
                }
                seen_mat.push(k);
                true
            });
        }
    }
    removed
}

/// Xarid ro'yxati: bir xil material bosqichlar bo'ylab qo'shiladi.
///
/// Ombor va ta'minot ekraniga hali ulanmagan.
#[allow(dead_code)]
///
/// Ombor va ta'minot uchun: nima, qancha, qaysi bosqichlarga.
pub fn purchase(smeta: &Smeta) -> Vec<(String, String, f64, Vec<String>)> {
    let mut out: Vec<(String, String, f64, Vec<String>)> = Vec::new();
    for stage in &smeta.stages {
        for m in stage.works.iter().flat_map(|w| &w.materials) {
            let k = key(&m.name, &m.unit);
            match out.iter_mut().find(|r| key(&r.0, &r.1) == k) {
                Some(row) => {
                    row.2 += m.qty;
                    if !row.3.contains(&stage.name) {
                        row.3.push(stage.name.clone());
                    }
                }
                None => out.push((
                    m.name.clone(),
                    m.unit.clone(),
                    m.qty,
                    vec![stage.name.clone()],
                )),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_are_computed_by_the_app() {
        assert_eq!(eval("2*(12+8)*3,0"), Some(120.0));
        assert_eq!(eval("10 − 4 / 2"), Some(8.0));
        assert_eq!(eval("12 х 8"), Some(96.0));
        assert_eq!(eval("-(3+2)"), Some(-5.0));
        // Tushunilmagan ifoda taxmin qilinmaydi.
        assert_eq!(eval("S пола * 1.05"), None);
        assert_eq!(eval("5/0"), None);
        assert_eq!(eval("(2+3"), None);
        assert_eq!(eval(""), None);
    }

    fn sample() -> Smeta {
        Smeta {
            stages: vec![Stage {
                name: "Poydevor".into(),
                markup: 10.0,
                works: vec![
                    Work {
                        name: "Beton quyish".into(),
                        qty: 10.0,
                        unit: "m3".into(),
                        source: Source::Project { page: Some(11) },
                        price: None,
                        materials: vec![
                            Resource {
                                name: "Beton B20".into(),
                                qty: 10.2,
                                unit: "m3".into(),
                                source: Source::Standard {
                                    note: "1.02 m3/m3".into(),
                                },
                                price: Some(700.0),
                            },
                            Resource {
                                name: "Vibrator ijarasi".into(),
                                qty: 1.0,
                                unit: "smena".into(),
                                source: Source::Assumption {
                                    note: "smena soni noma'lum".into(),
                                },
                                price: None,
                            },
                        ],
                    },
                    Work {
                        name: "Qolip".into(),
                        qty: 40.0,
                        unit: "m2".into(),
                        source: Source::Calc {
                            formula: "2*(12+8)*1".into(),
                        },
                        price: None,
                        materials: vec![],
                    },
                ],
            }],
            markup: 5.0,
            ..Default::default()
        }
    }

    /// Narx tartibi: smetadagi → katalog → baza; yo'q bo'lsa — yo'q.
    #[test]
    fn prices_come_in_order_and_are_never_invented() {
        let smeta = sample();
        let mut catalog = Catalog::default();
        catalog.works.insert(key("beton  QUYISH", "m3"), 150.0);
        let lookup =
            |_: bool, name: &str, _: &str| (name == "Qolip").then_some((50.0, Origin::Book));
        let hints = BTreeMap::new();
        let v = view(
            &smeta,
            &Sources {
                catalog: &catalog,
                lookup: &lookup,
                hints: &hints,
            },
        );
        let s = &v.stages[0];
        assert_eq!(s.works[0].0.origin, Origin::Catalog);
        assert_eq!(s.works[0].0.sum, Some(1500.0));
        assert_eq!(s.works[0].1[0].origin, Origin::Line);
        assert_eq!(s.works[0].1[1].origin, Origin::None);
        assert_eq!(s.works[0].1[1].sum, None);
        assert_eq!(s.works[1].0.origin, Origin::Book);
        // Jami: ishlar 1500 + 2000, material 7140; narxsiz bitta qator
        // jamiga kirmagan va sanalgan.
        assert_eq!((s.work_sum, s.missing), (3500.0, 1));
        assert!((s.material_sum - 7140.0).abs() < 1e-6);
        assert!((s.total - 10_640.0 * 1.1).abs() < 1e-6);
        assert!((v.total - 10_640.0 * 1.1 * 1.05).abs() < 1e-6);
        assert_eq!(smeta.lines(), 4);
        assert_eq!(smeta.assumptions(), 1);
        // Narxsizlar ro'yxati — AI taklifi uchun.
        let need = unpriced(&smeta, &v);
        assert_eq!(
            need,
            vec![(false, "Vibrator ijarasi".to_string(), "smena".to_string())]
        );

        // AI taxmini: jamiga kiradi, lekin ishonchli emas va sanaladi.
        let mut hints = BTreeMap::new();
        hints.insert(key("Vibrator ijarasi", "smena"), 80.0);
        let v2 = view(
            &smeta,
            &Sources {
                catalog: &catalog,
                lookup: &lookup,
                hints: &hints,
            },
        );
        assert_eq!(v2.missing, 0);
        assert_eq!(v2.hinted, 1);
        assert_eq!(v2.stages[0].works[0].1[1].origin, Origin::Hint);
        assert!((v2.material_sum - 7220.0).abs() < 1e-6);
        // Taxmin hali ham «narxsiz» ro'yxatida — tasdiq kutadi.
        assert_eq!(unpriced(&smeta, &v2).len(), 1);
    }

    /// Bir xil loyiha soni ikki bosqichda — ikkinchisi tashlanadi; boshqa
    /// miqdor yoki me'yor bo'yicha material qoladi.
    #[test]
    fn identical_project_lines_are_counted_once_across_stages() {
        let mut smeta = sample();
        let mut second = smeta.stages[0].clone();
        second.name = "Karkas".into();
        // Ish nomi va miqdori bir xil — takror; materiallari bilan ketadi.
        smeta.stages.push(second.clone());
        // Boshqa miqdorli ish qoladi, uning loyihadan olingan materiali
        // ham qoladi.
        second.works[0].qty = 5.0;
        second.works[0].materials[0].source = Source::Project { page: Some(3) };
        second.works[0].materials[0].qty = 99.0;
        second.works[1].qty = 7.0;
        smeta.stages.push(second);
        let removed = dedupe(&mut smeta);
        assert_eq!(removed, 4);
        assert_eq!(smeta.stages[1].works.len(), 0);
        assert_eq!(smeta.stages[2].works.len(), 2);
        assert_eq!(smeta.stages[2].works[0].materials.len(), 2);
    }

    #[test]
    fn a_sketch_gives_area_perimeter_and_facts() {
        let sk = Sketch {
            points: vec![(0.0, 0.0), (12.0, 0.0), (12.0, 8.0), (0.0, 8.0)],
            floors: 2,
            height: 3.0,
            windows: 6,
            ..Default::default()
        };
        assert!((sk.area() - 96.0).abs() < 1e-9);
        assert!((sk.perimeter() - 40.0).abs() < 1e-9);
        let facts = sk.facts();
        let get = |n: &str| {
            facts
                .iter()
                .find(|f| f.name.starts_with(n))
                .unwrap()
                .value
                .clone()
        };
        assert_eq!(get("Общая площадь"), "192.0");
        assert_eq!(get("Площадь наружных стен"), "240.0");
        assert_eq!(Sketch::default().area(), 0.0);
    }

    #[test]
    fn the_payment_schedule_sums_to_the_total() {
        let offer = Offer {
            advance_pct: 30.0,
            ..Default::default()
        };
        let stages = vec![
            ("A".to_string(), 600.0),
            ("B".to_string(), 400.0),
            ("C".to_string(), 0.0),
        ];
        let plan = schedule(&offer, &stages, 1000.0);
        assert_eq!(plan.len(), 3, "nol bosqich jadvalga kirmaydi");
        assert!((plan.iter().map(|p| p.amount).sum::<f64>() - 1000.0).abs() < 1e-9);
        assert!((plan[1].amount - 420.0).abs() < 1e-9);
        assert!(schedule(&offer, &stages, 0.0).is_empty());
    }

    #[test]
    fn the_catalog_keeps_a_history() {
        let mut c = Catalog::default();
        c.set(false, &key("Beton B20", "m3"), 650_000.0, "2026-10-01");
        c.set(false, &key("Beton B20", "m3"), 700_000.0, "2026-10-04");
        assert_eq!(c.materials.len(), 1);
        assert_eq!(
            c.history(false, &key("Beton B20", "m3"))[0],
            ("2026-10-04".to_string(), 700_000.0)
        );
        c.set(false, &key("Beton B20", "m3"), 0.0, "2026-10-05");
        assert!(c.materials.is_empty());
        assert_eq!(split_key("beton b20|m3"), ("beton b20", "m3"));
    }

    #[test]
    fn finding_ids_are_parsed_and_shifted() {
        assert_eq!(locate("s3.w2.m1"), Some((3, Some(2), Some(1))));
        assert_eq!(locate("s0"), Some((0, None, None)));
        assert_eq!(locate("w2"), None);
        assert_eq!(locate(""), None);
        assert_eq!(locate("s"), None);
        assert_eq!(locate("s3.x1"), None);
        let mut fs = vec![
            Finding {
                id: "s1.w0".into(),
                ..Default::default()
            },
            Finding {
                id: "s1.w1".into(),
                ..Default::default()
            },
            Finding {
                id: "s1.w2.m0".into(),
                ..Default::default()
            },
            Finding {
                id: "s2.w1".into(),
                ..Default::default()
            },
        ];
        shift_ids(&mut fs, 1, 1);
        assert!(fs[1].done);
        assert_eq!(fs[2].id, "s1.w1.m0");
        assert_eq!(fs[3].id, "s2.w1");
    }

    #[test]
    fn the_purchase_list_adds_the_same_material_across_stages() {
        let mut smeta = sample();
        let mut second = smeta.stages[0].clone();
        second.name = "Karkas".into();
        smeta.stages.push(second);
        let list = purchase(&smeta);
        let beton = list.iter().find(|r| r.0 == "Beton B20").unwrap();
        assert!((beton.2 - 20.4).abs() < 1e-9);
        assert_eq!(beton.3, vec!["Poydevor".to_string(), "Karkas".to_string()]);
    }

    #[test]
    fn a_smeta_survives_saving() {
        let smeta = sample();
        let text = serde_json::to_string(&smeta).unwrap();
        assert_eq!(serde_json::from_str::<Smeta>(&text).unwrap(), smeta);
        // Eski yoki qisman yozuv ham o'qiladi.
        assert_eq!(
            serde_json::from_str::<Smeta>("{}").unwrap(),
            Smeta::default()
        );
    }
}
