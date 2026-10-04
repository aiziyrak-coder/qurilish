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
    /// Shartlar va narxga kirmaydigan narsalar.
    pub terms: String,
    /// Mijozga bosqichlar ichidagi ishlar ham ko'rsatilsinmi.
    pub detailed: bool,
}

impl Default for Offer {
    fn default() -> Self {
        Offer {
            title: String::new(),
            company: String::new(),
            contacts: String::new(),
            customer: String::new(),
            valid_days: 14,
            terms: String::new(),
            detailed: true,
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
    None,
}

impl Origin {
    pub fn label(self) -> &'static str {
        match self {
            Origin::Line => "sm_price_line",
            Origin::Catalog => "sm_price_catalog",
            Origin::Book => "sm_price_book",
            Origin::None => "sm_price_none",
        }
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

fn priced(
    own: Option<f64>,
    catalog: &BTreeMap<String, f64>,
    name: &str,
    unit: &str,
    qty: f64,
    book: &dyn Fn(&str, &str) -> Option<f64>,
) -> Priced {
    let positive = |p: &f64| *p > 0.0;
    let (price, origin) = if let Some(p) = own.filter(positive) {
        (Some(p), Origin::Line)
    } else if let Some(p) = catalog.get(&key(name, unit)).copied().filter(positive) {
        (Some(p), Origin::Catalog)
    } else if let Some(p) = book(name, unit).filter(positive) {
        (Some(p), Origin::Book)
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
    /// Bosqich ustamalari va umumiy ustama bilan jami.
    pub total: f64,
}

/// Smetani narxlaydi va jamilarni chiqaradi.
///
/// `book` — narx bazasidan izlash (`nom, birlik → narx`); ishlar uchun ham,
/// materiallar uchun ham chaqiriladi.
pub fn view(smeta: &Smeta, catalog: &Catalog, book: &dyn Fn(&str, &str) -> Option<f64>) -> View {
    let mut out = View::default();
    for stage in &smeta.stages {
        let mut sv = StageView {
            works: Vec::new(),
            work_sum: 0.0,
            material_sum: 0.0,
            missing: 0,
            total: 0.0,
        };
        for w in &stage.works {
            let wp = priced(w.price, &catalog.works, &w.name, &w.unit, w.qty, book);
            match wp.sum {
                Some(s) => sv.work_sum += s,
                None => sv.missing += 1,
            }
            let mut ms = Vec::new();
            for m in &w.materials {
                let mp = priced(m.price, &catalog.materials, &m.name, &m.unit, m.qty, book);
                match mp.sum {
                    Some(s) => sv.material_sum += s,
                    None => sv.missing += 1,
                }
                ms.push(mp);
            }
            sv.works.push((wp, ms));
        }
        sv.total = (sv.work_sum + sv.material_sum) * (1.0 + stage.markup / 100.0);
        out.work_sum += sv.work_sum;
        out.material_sum += sv.material_sum;
        out.missing += sv.missing;
        out.total += sv.total;
        out.stages.push(sv);
    }
    out.total *= 1.0 + smeta.markup / 100.0;
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
        let book = |name: &str, _: &str| (name == "Qolip").then_some(50.0);
        let v = view(&smeta, &catalog, &book);
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
