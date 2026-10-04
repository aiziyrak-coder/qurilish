//! IFC (ISO 10303-21) faylini o'qish va bilimlar grafiga aylantirish (TZ II.1–2).
//!
//! IFC — ochiq matnli format, shuning uchun uni tashqi kutubxonasiz o'qish
//! mumkin. DWG va RVT yopiq formatlar: ular uchun alohida kutubxona kerak va
//! bu yerda ko'zda tutilmagan — ular hujjat sifatida biriktiriladi.
//!
//! O'qish ikki bosqichda: avval **STEP fizik fayli** tokenlarga ajratiladi
//! (`parse`), keyin kerakli obyektlar loyiha elementlari va ular orasidagi
//! bog'lanishlarga o'giriladi (`to_graph`). Shu bo'linish tufayli parserni
//! elementlar modelidan alohida sinash mumkin.
//!
//! Dastur IFC dagi ma'lumotni o'ylab topmaydi: nomi bo'lmagan element «nomsiz»
//! bo'lib qoladi, tanilmagan tur `Other` bo'ladi va bo'lim `None` — muhandis
//! ularni qo'lda aniqlashtiradi.

// Bu modul hozir ekrandan chaqirilmaydi: loyiha sahifasi kalkulyatsiyaga
// aylantirilgach, IFC/DXF yuklash tugmalari olib tashlandi. Kod va uning
// sinovlari saqlangan — qaytarish yoki o'chirish alohida qaror.
#![allow(dead_code)]

use crate::domain::{Element, ElementKind, Relation};
use crate::model::Section;
use std::collections::HashMap;

/// STEP faylidagi bitta argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `#12` — boshqa yozuvga havola.
    Ref(u64),
    /// `'matn'`
    Text(String),
    Number(f64),
    /// `.T.`, `.ELEMENT.` — sanoq qiymati.
    Enum(String),
    /// `$` — ko'rsatilmagan, yoki `*` — meros.
    Unset,
    List(Vec<Value>),
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_ref_id(&self) -> Option<u64> {
        match self {
            Value::Ref(r) => Some(*r),
            _ => None,
        }
    }

    /// Ro'yxatdagi havolalar; ro'yxat bo'lmasa — bo'sh.
    pub fn refs(&self) -> Vec<u64> {
        match self {
            Value::List(v) => v.iter().filter_map(|x| x.as_ref_id()).collect(),
            Value::Ref(r) => vec![*r],
            _ => Vec::new(),
        }
    }
}

/// Fayldagi bitta yozuv: `#12 = IFCWALL(...)`.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    /// Tur nomi doim katta harfda saqlanadi.
    pub kind: String,
    pub args: Vec<Value>,
}

impl Entity {
    fn arg(&self, i: usize) -> Option<&Value> {
        self.args.get(i)
    }

    fn text(&self, i: usize) -> Option<&str> {
        self.arg(i).and_then(|v| v.as_text())
    }
}

/// O'qilgan fayl.
#[derive(Debug, Default)]
pub struct Model {
    pub entities: HashMap<u64, Entity>,
    /// Sxema nomi (`IFC4`, `IFC2X3`) — HEADER dan.
    pub schema: String,
}

impl Model {
    pub fn get(&self, id: u64) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Berilgan turdagi yozuvlar, `id` bo'yicha tartiblangan.
    ///
    /// Tartib muhim: import ikki marta bajarilsa, elementlar bir xil ketma-ketlikda
    /// tushishi kerak, aks holda natijani solishtirib bo'lmaydi.
    pub fn by_kind<'a>(&'a self, kinds: &[&str]) -> Vec<&'a Entity> {
        let mut v: Vec<&Entity> = self
            .entities
            .values()
            .filter(|e| kinds.contains(&e.kind.as_str()))
            .collect();
        v.sort_by_key(|e| e.id);
        v
    }
}

// ================================================================ Parser

/// Faylni tokenlarga ajratadi.
///
/// Xato yuz berganda o'qish to'xtamaydi: buzilgan yozuv tashlab yuboriladi va
/// qolgani o'qilaveradi — bitta noto'g'ri qator butun faylni yo'qotmasin.
pub fn parse(src: &str) -> Model {
    let mut model = Model::default();
    let bytes: Vec<char> = strip_comments(src).chars().collect();
    let mut i = 0;
    let n = bytes.len();

    // HEADER dagi FILE_SCHEMA(('IFC4')) dan sxema nomini olamiz.
    while i < n {
        let (stmt, next) = read_statement(&bytes, i);
        i = next;
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }
        let upper = stmt.to_ascii_uppercase();
        if upper.starts_with("FILE_SCHEMA") {
            if let Some(s) = first_quoted(stmt) {
                model.schema = s;
            }
            continue;
        }
        if !stmt.starts_with('#') {
            continue;
        }
        if let Some(e) = parse_entity(stmt) {
            model.entities.insert(e.id, e);
        }
    }
    model
}

/// `/* ... */` izohlarini olib tashlaydi. Qo'shtirnoq ichidagisi tegilmaydi.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_text = false;
    while let Some(c) = chars.next() {
        if in_text {
            out.push(c);
            if c == '\'' {
                // `''` — matn ichidagi qo'shtirnoq, matn tugamaydi.
                if chars.peek() == Some(&'\'') {
                    out.push(chars.next().unwrap());
                } else {
                    in_text = false;
                }
            }
            continue;
        }
        if c == '\'' {
            in_text = true;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut prev = ' ';
            for c2 in chars.by_ref() {
                if prev == '*' && c2 == '/' {
                    break;
                }
                prev = c2;
            }
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

/// Keyingi `;` gacha bo'lgan bo'lakni oladi (matn ichidagi `;` hisobga olinmaydi).
fn read_statement(src: &[char], from: usize) -> (String, usize) {
    let mut out = String::new();
    let mut i = from;
    let mut in_text = false;
    while i < src.len() {
        let c = src[i];
        if in_text {
            out.push(c);
            if c == '\'' {
                if src.get(i + 1) == Some(&'\'') {
                    out.push('\'');
                    i += 2;
                    continue;
                }
                in_text = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => {
                in_text = true;
                out.push(c);
            }
            ';' => {
                i += 1;
                return (out, i);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    (out, i)
}

fn first_quoted(s: &str) -> Option<String> {
    let start = s.find('\'')? + 1;
    let rest = &s[start..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

/// `#12 = IFCWALL(args)` ni yozuvga aylantiradi.
fn parse_entity(stmt: &str) -> Option<Entity> {
    let eq = stmt.find('=')?;
    let id: u64 = stmt[1..eq].trim().parse().ok()?;
    let rest = stmt[eq + 1..].trim();
    let open = rest.find('(')?;
    let kind = rest[..open].trim().to_ascii_uppercase();
    if kind.is_empty() {
        return None;
    }
    let close = rest.rfind(')')?;
    if close <= open {
        return None;
    }
    let args = parse_list(&rest[open + 1..close]);
    Some(Entity { id, kind, args })
}

/// Vergul bilan ajratilgan argumentlar ro'yxati (ichma-ich qavslarni hisobga oladi).
fn parse_list(src: &str) -> Vec<Value> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_text = false;
    let mut start = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if in_text {
            if c == '\'' {
                if chars.get(i + 1) == Some(&'\'') {
                    i += 2;
                    continue;
                }
                in_text = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_text = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(parse_value(&chars[start..i].iter().collect::<String>()));
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let tail: String = chars[start..].iter().collect();
    if !tail.trim().is_empty() || !out.is_empty() {
        out.push(parse_value(&tail));
    }
    out
}

fn parse_value(raw: &str) -> Value {
    let s = raw.trim();
    if s.is_empty() || s == "$" || s == "*" {
        return Value::Unset;
    }
    if let Some(rest) = s.strip_prefix('#') {
        if let Ok(v) = rest.trim().parse::<u64>() {
            return Value::Ref(v);
        }
    }
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return Value::Text(unquote(&s[1..s.len() - 1]));
    }
    if s.starts_with('(') && s.ends_with(')') {
        return Value::List(parse_list(&s[1..s.len() - 1]));
    }
    if s.starts_with('.') && s.ends_with('.') && s.len() > 2 {
        return Value::Enum(s[1..s.len() - 1].to_string());
    }
    // `IFCLABEL('...')` kabi o'ram: ichidagi qiymatni olamiz.
    if let (Some(open), Some(close)) = (s.find('('), s.rfind(')')) {
        if open > 0 && close > open {
            let inner = parse_list(&s[open + 1..close]);
            return inner.into_iter().next().unwrap_or(Value::Unset);
        }
    }
    if let Ok(v) = s.parse::<f64>() {
        return Value::Number(v);
    }
    Value::Text(s.to_string())
}

/// `''` — bitta qo'shtirnoq; `\X2\...\X0\` — kengaytirilgan belgilar.
fn unquote(s: &str) -> String {
    let mut out = s.replace("''", "'");
    // IFC da kirill harflari `\X2\0410\X0\` ko'rinishida kelishi mumkin.
    if out.contains("\\X2\\") {
        out = decode_x2(&out);
    }
    out
}

/// `\X2\0410 0411\X0\` — UTF-16 kodlari ketma-ketligi.
fn decode_x2(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(pos) = rest.find("\\X2\\") {
        out.push_str(&rest[..pos]);
        rest = &rest[pos + 4..];
        let end = rest.find("\\X0\\").unwrap_or(rest.len());
        let hex = &rest[..end];
        let mut units = Vec::new();
        let mut chunk = String::new();
        for c in hex.chars() {
            if c.is_ascii_hexdigit() {
                chunk.push(c);
                if chunk.len() == 4 {
                    if let Ok(v) = u16::from_str_radix(&chunk, 16) {
                        units.push(v);
                    }
                    chunk.clear();
                }
            }
        }
        out.push_str(&String::from_utf16_lossy(&units));
        rest = if end < rest.len() {
            &rest[end + 4..]
        } else {
            ""
        };
    }
    out.push_str(rest);
    out
}

// ================================================================ Grafga o'girish

/// Import natijasi: elementlar va ular orasidagi bog'lanishlar.
///
/// `links` dagi indekslar `elements` ro'yxatidagi o'rinlarga ishora qiladi —
/// baza identifikatorlari yozishdan keyin ma'lum bo'ladi.
#[derive(Debug, Default)]
pub struct Graph {
    pub elements: Vec<Element>,
    pub links: Vec<(usize, usize, Relation)>,
    /// O'qilgan, ammo element sifatida olinmagan turlar soni.
    pub skipped: usize,
}

/// IFC turini loyiha elementiga moslash. Tanilmagan tur `None`.
fn map_kind(ifc: &str) -> Option<(ElementKind, Section)> {
    let k = match ifc {
        "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCWALLELEMENTEDCASE" => {
            (ElementKind::Wall, Section::Ar)
        }
        "IFCDOOR" | "IFCDOORSTANDARDCASE" => (ElementKind::Door, Section::Ar),
        "IFCWINDOW" | "IFCWINDOWSTANDARDCASE" => (ElementKind::Window, Section::Ar),
        "IFCSPACE" => (ElementKind::Room, Section::Ar),
        "IFCCOLUMN" | "IFCCOLUMNSTANDARDCASE" => (ElementKind::Column, Section::Kj),
        "IFCBEAM" | "IFCBEAMSTANDARDCASE" => (ElementKind::Beam, Section::Kj),
        "IFCSLAB" | "IFCSLABSTANDARDCASE" | "IFCFOOTING" => (ElementKind::Slab, Section::Kj),
        "IFCOPENINGELEMENT" | "IFCOPENINGSTANDARDCASE" => (ElementKind::Opening, Section::Kj),
        "IFCMEMBER" | "IFCPLATE" => (ElementKind::Beam, Section::Km),
        "IFCPIPESEGMENT" | "IFCPIPEFITTING" | "IFCFLOWSEGMENT" => (ElementKind::Pipe, Section::Vk),
        "IFCSANITARYTERMINAL" | "IFCPUMP" | "IFCTANK" | "IFCVALVE" => {
            (ElementKind::Device, Section::Vk)
        }
        "IFCDUCTSEGMENT" | "IFCDUCTFITTING" => (ElementKind::Duct, Section::Ov),
        "IFCAIRTERMINAL" | "IFCFAN" | "IFCSPACEHEATER" | "IFCBOILER" => {
            (ElementKind::Device, Section::Ov)
        }
        "IFCCABLESEGMENT" | "IFCCABLECARRIERSEGMENT" | "IFCCABLEFITTING" => {
            (ElementKind::Cable, Section::Eom)
        }
        "IFCLIGHTFIXTURE"
        | "IFCELECTRICAPPLIANCE"
        | "IFCOUTLET"
        | "IFCSWITCHINGDEVICE"
        | "IFCELECTRICDISTRIBUTIONBOARD"
        | "IFCDISTRIBUTIONBOARD" => (ElementKind::Device, Section::Eom),
        "IFCALARM" | "IFCSENSOR" | "IFCFIRESUPPRESSIONTERMINAL" => {
            (ElementKind::Device, Section::Pb)
        }
        "IFCSTAIR" | "IFCSTAIRFLIGHT" | "IFCRAILING" | "IFCROOF" | "IFCCOVERING"
        | "IFCCURTAINWALL" => (ElementKind::Other, Section::Ar),
        _ => return None,
    };
    Some(k)
}

/// IFC ob'ektining nomi: `Tag` bo'lsa marka sifatida, bo'lmasa `Name`.
///
/// Revit va Archicad `Tag` maydoniga chizmadagi markani yozadi — bizga aynan
/// shu kerak. `Name` odatda «Basic Wall:Interior — 200mm» ko'rinishida bo'ladi.
fn mark_of(e: &Entity) -> String {
    let tag = e.text(7).unwrap_or("").trim();
    if !tag.is_empty() {
        return tag.to_string();
    }
    let name = e.text(2).unwrap_or("").trim();
    if !name.is_empty() {
        // «Turi:Kichik turi:123» — oxirgi bo'lak odatda nusxa raqami.
        return name.to_string();
    }
    String::new()
}

// ================================================================ Joylashuv

/// Elementning modeldagi o'rni (TZ VI.13, VIII.6).
///
/// IFC da har element **joylashuv zanjiri** bilan bog'lanadi: element →
/// qavat → bino → uchastka. Har bo'g'inda o'z koordinatasi turadi va
/// haqiqiy o'rin ularning yig'indisi bo'ladi.
///
/// **Nima hisobga olinmaydi va nega:** burilish (rotatsiya). To'liq
/// hisob uchun har bo'g'inning o'q yo'nalishini ham qo'llash kerak.
/// Burilgan qavat kam uchraydi, xato esa qavat ichida qoladi — shuning
/// uchun bu yerda faqat siljish yig'iladi va natija **reja** deb
/// ataladi, 3D model deb emas.
pub fn position(model: &Model, entity: &Entity) -> Option<[f64; 3]> {
    // IfcProduct: 5-argument — ObjectPlacement.
    let placement = entity.arg(5)?.as_ref_id()?;
    let mut sum = [0.0f64; 3];
    let mut at = Some(placement);
    // Zanjir uzunligi cheklangan: buzilgan fayl bizni cheksiz aylanishga
    // tortmasligi kerak (havolalar halqa hosil qilishi mumkin).
    for _ in 0..32 {
        let Some(id) = at else { break };
        let Some(node) = model.get(id) else { break };
        if node.kind != "IFCLOCALPLACEMENT" {
            break;
        }
        if let Some(p) = axis_point(model, node.arg(1).and_then(|v| v.as_ref_id())) {
            for i in 0..3 {
                sum[i] += p[i];
            }
        }
        at = node.arg(0).and_then(|v| v.as_ref_id());
    }
    (sum != [0.0; 3]).then_some(sum)
}

/// `IfcAxis2Placement3D` dan nuqtani oladi.
fn axis_point(model: &Model, id: Option<u64>) -> Option<[f64; 3]> {
    let node = model.get(id?)?;
    if node.kind != "IFCAXIS2PLACEMENT3D" && node.kind != "IFCAXIS2PLACEMENT2D" {
        return None;
    }
    cartesian(model, node.arg(0).and_then(|v| v.as_ref_id()))
}

/// `IfcCartesianPoint` koordinatalari.
fn cartesian(model: &Model, id: Option<u64>) -> Option<[f64; 3]> {
    let node = model.get(id?)?;
    if node.kind != "IFCCARTESIANPOINT" {
        return None;
    }
    let Value::List(coords) = node.arg(0)? else {
        return None;
    };
    let mut out = [0.0f64; 3];
    for (i, v) in coords.iter().take(3).enumerate() {
        if let Value::Number(n) = v {
            out[i] = *n;
        }
    }
    Some(out)
}

/// Joylashuvni yozuvga saqlash ko'rinishi: `x,y,z`.
pub fn pos_text(p: Option<[f64; 3]>) -> String {
    match p {
        Some(v) => format!("{:.3},{:.3},{:.3}", v[0], v[1], v[2]),
        None => String::new(),
    }
}

/// Saqlangan matndan joylashuvni o'qiydi.
pub fn pos_parse(s: &str) -> Option<[f64; 3]> {
    let parts: Vec<f64> = s
        .split(',')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    (parts.len() == 3).then(|| [parts[0], parts[1], parts[2]])
}

// ================================================================ O'lcham

/// Elementning o'q bo'yicha tekislangan qutisi (AABB), metrda:
/// `[minX, minY, minZ, maxX, maxY, maxZ]`.
///
/// IFC da shakl bir necha xil yoziladi. Bu yerda **ikkitasi** tushuniladi
/// va ular real modellarning katta qismini qoplaydi:
///
/// 1. `IfcBoundingBox` — modelning o'zi bergan quti. Eng ishonchlisi.
/// 2. `IfcExtrudedAreaSolid` — profil (to'rtburchak, doira, I-simon)
///    ma'lum yo'nalishda cho'zilgan. Devor, ustun va rigelning aksariyati
///    shunday yoziladi.
///
/// **Nima tushunilmaydi va nega:** BREP (yuzalar to'plami), CSG (kesish
/// va birlashtirish) va aylantirilgan profil. Ular geometriya yadrosini
/// talab qiladi. Bunday elementda quti **bo'lmaydi** — «taxminiy quti»
/// qo'yish kolliziya hisobini yolg'on qilardi.
///
/// **Burilish hisobga olinmaydi.** Quti o'qlar bo'yicha tekislangan,
/// shuning uchun burilgan devorning qutisi haqiqiy devordan kattaroq
/// bo'ladi. Bu **ortiqcha topilma** beradi, tushib qolgan topilma emas —
/// va ekranda shunday deb aytiladi.
pub fn bbox(model: &Model, entity: &Entity, origin: [f64; 3]) -> Option<[f64; 6]> {
    // IfcProduct: 6-argument — Representation.
    let shape = model.get(entity.arg(6)?.as_ref_id()?)?;
    if shape.kind != "IFCPRODUCTDEFINITIONSHAPE" {
        return None;
    }
    let mut best: Option<[f64; 6]> = None;
    for rep_id in shape.arg(2).map(|v| v.refs()).unwrap_or_default() {
        let Some(rep) = model.get(rep_id) else {
            continue;
        };
        if rep.kind != "IFCSHAPEREPRESENTATION" {
            continue;
        }
        for item_id in rep.arg(3).map(|v| v.refs()).unwrap_or_default() {
            let Some(item) = model.get(item_id) else {
                continue;
            };
            let local = match item.kind.as_str() {
                "IFCBOUNDINGBOX" => bounding_box(model, item),
                "IFCEXTRUDEDAREASOLID" => extruded(model, item),
                _ => None,
            };
            // `IfcBoundingBox` aniqroq: u modelning o'z hisobidan keladi,
            // cho'zilgan profil esa bizning soddalashtirishimiz. Shuning
            // uchun quti topilsa u har doim ustun turadi.
            let exact = item.kind == "IFCBOUNDINGBOX";
            if let Some(l) = local {
                if exact || best.is_none() {
                    best = Some(l);
                }
            }
        }
    }
    let l = best?;
    Some([
        l[0] + origin[0],
        l[1] + origin[1],
        l[2] + origin[2],
        l[3] + origin[0],
        l[4] + origin[1],
        l[5] + origin[2],
    ])
}

/// `IfcBoundingBox(Corner, XDim, YDim, ZDim)`.
fn bounding_box(model: &Model, item: &Entity) -> Option<[f64; 6]> {
    let corner = cartesian(model, item.arg(0).and_then(|v| v.as_ref_id()))?;
    let dims = [num(item, 1)?, num(item, 2)?, num(item, 3)?];
    if dims.iter().any(|d| *d <= 0.0) {
        return None;
    }
    Some([
        corner[0],
        corner[1],
        corner[2],
        corner[0] + dims[0],
        corner[1] + dims[1],
        corner[2] + dims[2],
    ])
}

/// `IfcExtrudedAreaSolid(SweptArea, Position, ExtrudedDirection, Depth)`.
fn extruded(model: &Model, item: &Entity) -> Option<[f64; 6]> {
    let depth = num(item, 3)?;
    if depth <= 0.0 {
        return None;
    }
    let (dx, dy) = profile(model, item.arg(0).and_then(|v| v.as_ref_id()))?;
    // Profilning o'z siljishi: `Position` — IfcAxis2Placement3D.
    let at = axis_point(model, item.arg(1).and_then(|v| v.as_ref_id())).unwrap_or([0.0; 3]);
    // Cho'zish yo'nalishi odatda Z. Boshqa o'q bo'lsa ham quti shu
    // o'lchamda qoladi: burilish baribir hisobga olinmaydi va bu
    // yuqorida aytilgan.
    Some([
        at[0] - dx / 2.0,
        at[1] - dy / 2.0,
        at[2],
        at[0] + dx / 2.0,
        at[1] + dy / 2.0,
        at[2] + depth,
    ])
}

/// Profilning kengligi va chuqurligi.
///
/// Tanilmagan profil `None` beradi — taxmin qilinmaydi.
fn profile(model: &Model, id: Option<u64>) -> Option<(f64, f64)> {
    let p = model.get(id?)?;
    match p.kind.as_str() {
        // IfcRectangleProfileDef(ProfileType, Name, Position, XDim, YDim)
        "IFCRECTANGLEPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
            Some((num(p, 3)?, num(p, 4)?))
        }
        // IfcCircleProfileDef(..., Radius)
        "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
            let r = num(p, 3)?;
            Some((r * 2.0, r * 2.0))
        }
        // IfcIShapeProfileDef(..., OverallWidth, OverallDepth, ...)
        "IFCISHAPEPROFILEDEF" => Some((num(p, 3)?, num(p, 4)?)),
        // IfcRectangleHollowProfileDef ham to'rtburchak o'lchamiga ega.
        "IFCRECTANGLEHOLLOWPROFILEDEF" => Some((num(p, 3)?, num(p, 4)?)),
        _ => None,
    }
    .filter(|(x, y)| *x > 0.0 && *y > 0.0)
}

/// Argumentdagi son.
fn num(e: &Entity, i: usize) -> Option<f64> {
    match e.arg(i)? {
        Value::Number(n) => Some(*n),
        _ => None,
    }
}

/// Qutini yozuvga saqlash ko'rinishi.
pub fn bbox_text(b: Option<[f64; 6]>) -> String {
    match b {
        Some(v) => v
            .iter()
            .map(|x| format!("{x:.3}"))
            .collect::<Vec<_>>()
            .join(","),
        None => String::new(),
    }
}

/// Saqlangan matndan qutini o'qiydi.
pub fn bbox_parse(s: &str) -> Option<[f64; 6]> {
    let v: Vec<f64> = s
        .split(',')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    (v.len() == 6).then(|| [v[0], v[1], v[2], v[3], v[4], v[5]])
}

/// O'qilgan modeldan bilimlar grafi tuzadi.
pub fn to_graph(model: &Model, project_id: i64) -> Graph {
    let mut g = Graph::default();
    // IFC identifikatori -> `elements` dagi o'rin.
    let mut index: HashMap<u64, usize> = HashMap::new();

    // ---- Qavatlar: element qaysi qavatda turgani shulardan olinadi.
    let mut storey_of: HashMap<u64, String> = HashMap::new();
    for rel in model.by_kind(&["IFCRELCONTAINEDINSPATIALSTRUCTURE"]) {
        let Some(spatial) = rel.arg(5).and_then(|v| v.as_ref_id()) else {
            continue;
        };
        let name = model
            .get(spatial)
            .and_then(|s| s.text(2))
            .unwrap_or("")
            .to_string();
        for child in rel.arg(4).map(|v| v.refs()).unwrap_or_default() {
            storey_of.insert(child, name.clone());
        }
    }

    // ---- Xonalar: element qaysi xonaga tegishli (IFCRELSPACEBOUNDARY).
    let mut room_of: HashMap<u64, String> = HashMap::new();
    for rel in model.by_kind(&[
        "IFCRELSPACEBOUNDARY",
        "IFCRELSPACEBOUNDARY1STLEVEL",
        "IFCRELSPACEBOUNDARY2NDLEVEL",
    ]) {
        let (Some(space), Some(elem)) = (
            rel.arg(4).and_then(|v| v.as_ref_id()),
            rel.arg(5).and_then(|v| v.as_ref_id()),
        ) else {
            continue;
        };
        if let Some(name) = model.get(space).and_then(|s| s.text(2)) {
            room_of.entry(elem).or_insert_with(|| name.to_string());
        }
    }

    // ---- Elementlar. Tartib `id` bo'yicha — import takrorlanuvchan bo'lsin.
    let mut all: Vec<&Entity> = model.entities.values().collect();
    all.sort_by_key(|e| e.id);
    for e in all {
        let Some((kind, section)) = map_kind(&e.kind) else {
            // Bog'lanish va geometriya yozuvlari element emas — ular sanalmaydi.
            if e.kind.starts_with("IFCREL") || !e.kind.starts_with("IFC") {
                continue;
            }
            g.skipped += 1;
            continue;
        };
        index.insert(e.id, g.elements.len());
        let pos = position(model, e);
        g.elements.push(Element {
            id: 0,
            project_id,
            section,
            kind,
            mark: mark_of(e),
            room: room_of.get(&e.id).cloned().unwrap_or_default(),
            axis: String::new(),
            level: storey_of.get(&e.id).cloned().unwrap_or_default(),
            size: 0.0,
            unit: String::new(),
            value: 0.0,
            value_name: String::new(),
            // Manba ko'rsatiladi: bu qiymat qo'lda emas, IFC dan kelgan.
            sheet: "IFC".into(),
            note: e.kind.clone(),
            pos,
            bbox: pos.and_then(|origin| bbox(model, e, origin)),
        });
    }

    // ---- Bog'lanishlar.
    let push = |g: &mut Graph, from: u64, to: u64, rel: Relation| {
        if let (Some(a), Some(b)) = (index.get(&from), index.get(&to)) {
            if a != b {
                g.links.push((*a, *b, rel));
            }
        }
    };

    // Devor teshikni o'z ichiga oladi.
    for rel in model.by_kind(&["IFCRELVOIDSELEMENT"]) {
        if let (Some(wall), Some(opening)) = (
            rel.arg(4).and_then(|v| v.as_ref_id()),
            rel.arg(5).and_then(|v| v.as_ref_id()),
        ) {
            push(&mut g, wall, opening, Relation::Contains);
        }
    }
    // Teshikni eshik yoki deraza to'ldiradi.
    for rel in model.by_kind(&["IFCRELFILLSELEMENT"]) {
        if let (Some(opening), Some(filler)) = (
            rel.arg(4).and_then(|v| v.as_ref_id()),
            rel.arg(5).and_then(|v| v.as_ref_id()),
        ) {
            push(&mut g, opening, filler, Relation::Contains);
        }
    }
    // Xona o'z chegarasidagi elementlarni o'z ichiga oladi.
    for rel in model.by_kind(&[
        "IFCRELSPACEBOUNDARY",
        "IFCRELSPACEBOUNDARY1STLEVEL",
        "IFCRELSPACEBOUNDARY2NDLEVEL",
    ]) {
        if let (Some(space), Some(elem)) = (
            rel.arg(4).and_then(|v| v.as_ref_id()),
            rel.arg(5).and_then(|v| v.as_ref_id()),
        ) {
            push(&mut g, space, elem, Relation::Contains);
        }
    }
    // Bir tuzilma ichidagi qismlar (masalan zinapoya va uning marshlari).
    for rel in model.by_kind(&["IFCRELAGGREGATES"]) {
        let Some(parent) = rel.arg(4).and_then(|v| v.as_ref_id()) else {
            continue;
        };
        for child in rel.arg(5).map(|v| v.refs()).unwrap_or_default() {
            push(&mut g, parent, child, Relation::Contains);
        }
    }

    // Bir xil bog'lanish ikki marta tushmasin.
    g.links.sort();
    g.links.dedup();
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TZ II, VII.31: element o'lchami IFC dan olinadi.
    ///
    /// Ikki manba tekshiriladi: modelning o'z qutisi va cho'zilgan
    /// profil. Tanilmagan shakl esa **quti bermaydi** — taxminiy quti
    /// kolliziya hisobini yolg'on qilardi.
    #[test]
    fn a_shape_becomes_a_box_or_nothing_at_all() {
        let src = r#"
ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((0.,0.,0.));
#2= IFCAXIS2PLACEMENT3D(#1,$,$);
#3= IFCLOCALPLACEMENT($,#2);

/* Cho'zilgan to'rtburchak: 0,2 x 4,0, balandligi 3,0 */
#10= IFCRECTANGLEPROFILEDEF(.AREA.,'Devor',$,0.2,4.0);
#11= IFCEXTRUDEDAREASOLID(#10,$,$,3.0);
#12= IFCSHAPEREPRESENTATION($,'Body','SweptSolid',(#11));
#13= IFCPRODUCTDEFINITIONSHAPE($,$,(#12));
#14= IFCWALLSTANDARDCASE('w1',$,'Devor',$,$,#3,#13,'D-1');

/* Modelning o'z qutisi: burchakdan 1,0 x 2,0 x 0,5 */
#20= IFCCARTESIANPOINT((1.,1.,0.));
#21= IFCBOUNDINGBOX(#20,1.0,2.0,0.5);
#22= IFCSHAPEREPRESENTATION($,'Box','BoundingBox',(#21));
#23= IFCPRODUCTDEFINITIONSHAPE($,$,(#22));
#24= IFCCOLUMN('c1',$,'Ustun',$,$,#3,#23,'K-1');

/* Tanilmagan shakl: BREP */
#30= IFCFACETEDBREP(#1);
#31= IFCSHAPEREPRESENTATION($,'Body','Brep',(#30));
#32= IFCPRODUCTDEFINITIONSHAPE($,$,(#31));
#33= IFCBEAM('b1',$,'Rigel',$,$,#3,#32,'R-1');
ENDSEC;
END-ISO-10303-21;
"#;
        let model = parse(src);

        // Cho'zilgan profil: markazdan yarim-yarim, balandligi 0..3.
        let wall = model.get(14).expect("devor");
        let b = bbox(&model, wall, [0.0, 0.0, 0.0]).expect("quti");
        assert!((b[0] - -0.1).abs() < 1e-9, "{b:?}");
        assert!((b[1] - -2.0).abs() < 1e-9, "{b:?}");
        assert!((b[3] - 0.1).abs() < 1e-9, "{b:?}");
        assert!((b[4] - 2.0).abs() < 1e-9, "{b:?}");
        assert!((b[5] - 3.0).abs() < 1e-9, "{b:?}");

        // Modelning o'z qutisi burchakdan o'lchanadi.
        let column = model.get(24).expect("ustun");
        let b = bbox(&model, column, [0.0, 0.0, 0.0]).expect("quti");
        assert_eq!([b[0], b[1], b[2]], [1.0, 1.0, 0.0]);
        assert_eq!([b[3], b[4], b[5]], [2.0, 3.0, 0.5]);

        // Joylashuv qo'shiladi.
        let b = bbox(&model, column, [10.0, 20.0, 3.0]).expect("quti");
        assert_eq!([b[0], b[1], b[2]], [11.0, 21.0, 3.0]);

        // Tanilmagan shakl — quti yo'q.
        let beam = model.get(33).expect("rigel");
        assert!(bbox(&model, beam, [0.0; 3]).is_none());

        // Saqlash va qayta o'qish.
        let text = bbox_text(Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]));
        assert_eq!(bbox_parse(&text), Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]));
        assert_eq!(bbox_text(None), "");
        assert!(bbox_parse("1,2,3").is_none());
    }

    /// TZ VI.13: element joyi joylashuv zanjiridan yig'iladi.
    ///
    /// Zanjir: element → qavat → bino. Har bo'g'inda o'z siljishi bor va
    /// haqiqiy o'rin ularning yig'indisi bo'ladi.
    #[test]
    fn a_position_is_summed_along_the_placement_chain() {
        let src = r#"
ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((100.,200.,0.));
#2= IFCAXIS2PLACEMENT3D(#1,$,$);
#3= IFCLOCALPLACEMENT($,#2);
#4= IFCCARTESIANPOINT((0.,0.,3.5));
#5= IFCAXIS2PLACEMENT3D(#4,$,$);
#6= IFCLOCALPLACEMENT(#3,#5);
#7= IFCCARTESIANPOINT((5.,7.,0.));
#8= IFCAXIS2PLACEMENT3D(#7,$,$);
#9= IFCLOCALPLACEMENT(#6,#8);
#10= IFCWALLSTANDARDCASE('3vB',$,'Devor',$,$,#9,$,'D-1');
#11= IFCWALLSTANDARDCASE('4vB',$,'Joysiz devor',$,$,$,$,'D-2');
ENDSEC;
END-ISO-10303-21;
"#;
        let model = parse(src);
        let wall = model.get(10).expect("devor");
        let pos = position(&model, wall).expect("joylashuv");
        // 100+0+5, 200+0+7, 0+3,5+0
        assert!((pos[0] - 105.0).abs() < 1e-9, "{pos:?}");
        assert!((pos[1] - 207.0).abs() < 1e-9, "{pos:?}");
        assert!((pos[2] - 3.5).abs() < 1e-9, "{pos:?}");

        // Joylashuvsiz element — «yo'q», nol emas.
        let other = model.get(11).expect("ikkinchi devor");
        assert!(position(&model, other).is_none());

        // Saqlash va qayta o'qish qiymatni buzmaydi.
        let text = pos_text(Some(pos));
        let back = pos_parse(&text).expect("qayta o'qildi");
        assert!((back[0] - 105.0).abs() < 1e-3);
        assert_eq!(pos_text(None), "");
        assert!(pos_parse("").is_none());
        assert!(pos_parse("1,2").is_none());
    }

    /// Halqali havola cheksiz aylanishga olib kelmaydi.
    #[test]
    fn a_looping_placement_does_not_hang() {
        let src = r#"
ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCARTESIANPOINT((1.,1.,1.));
#2= IFCAXIS2PLACEMENT3D(#1,$,$);
#3= IFCLOCALPLACEMENT(#4,#2);
#4= IFCLOCALPLACEMENT(#3,#2);
#10= IFCWALLSTANDARDCASE('3vB',$,'Devor',$,$,#3,$,'D-1');
ENDSEC;
END-ISO-10303-21;
"#;
        let model = parse(src);
        let wall = model.get(10).expect("devor");
        // Muhimi — qaytib kelishi; qiymat esa cheklangan qadamlar
        // yig'indisi bo'ladi.
        let pos = position(&model, wall).expect("joylashuv");
        assert!(pos[0].is_finite());
    }

    const SAMPLE: &str = r#"
ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('demo.ifc','2026-09-01T10:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCPROJECT('0Yv',$,'Navro''z',$,$,$,$,$,$);
#5= IFCBUILDINGSTOREY('1St',$,'1-qavat',$,$,$,$,$,.ELEMENT.,0.);
#10= IFCWALLSTANDARDCASE('3vB',$,'Basic Wall:Tashqi:200',$,'Tashqi devor',$,$,'D-1');
#11= IFCWINDOW('4wN',$,'Deraza 1500x1500',$,$,$,$,'OK-1');
#12= IFCOPENINGELEMENT('5op',$,'Teshik',$,$,$,$,'PR-1');
#13= IFCSPACE('6sp',$,'101-xona',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#14= IFCPIPESEGMENT('7pp',$,'Truba PP 110',$,$,$,$,'K1-1');
#15= IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$);
/* izoh: bu yozuv o'qilmaydi */
#20= IFCRELCONTAINEDINSPATIALSTRUCTURE('8rc',$,$,$,(#10,#11,#14),#5);
#21= IFCRELVOIDSELEMENT('9rv',$,$,$,#10,#12);
#22= IFCRELFILLSELEMENT('10rf',$,$,$,#12,#11);
#23= IFCRELSPACEBOUNDARY('11rs',$,$,$,#13,#10,$,.PHYSICAL.,.EXTERNAL.);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn header_and_entities_are_read() {
        let m = parse(SAMPLE);
        assert_eq!(m.schema, "IFC4");
        assert!(m.entities.len() >= 12, "yozuvlar: {}", m.entities.len());
        let wall = m.get(10).expect("devor");
        assert_eq!(wall.kind, "IFCWALLSTANDARDCASE");
        assert_eq!(wall.text(7), Some("D-1"));
        // Qo'shtirnoq ichidagi `''` bitta belgiga aylanadi.
        assert_eq!(m.get(1).unwrap().text(2), Some("Navro'z"));
    }

    #[test]
    fn refs_and_lists_are_parsed() {
        let m = parse(SAMPLE);
        let rel = m.get(20).unwrap();
        let mut kids = rel.arg(4).unwrap().refs();
        kids.sort();
        assert_eq!(kids, vec![10, 11, 14]);
        assert_eq!(rel.arg(5).unwrap().as_ref_id(), Some(5));
    }

    #[test]
    fn graph_maps_kinds_sections_and_levels() {
        let m = parse(SAMPLE);
        let g = to_graph(&m, 7);
        let find = |mark: &str| g.elements.iter().find(|e| e.mark == mark).expect(mark);

        let wall = find("D-1");
        assert_eq!(wall.kind, ElementKind::Wall);
        assert_eq!(wall.section, Section::Ar);
        assert_eq!(wall.level, "1-qavat");
        assert_eq!(wall.project_id, 7);
        assert_eq!(wall.sheet, "IFC");

        assert_eq!(find("K1-1").kind, ElementKind::Pipe);
        assert_eq!(find("K1-1").section, Section::Vk);
        assert_eq!(find("PR-1").kind, ElementKind::Opening);
        // Xona nomi `Name` dan olinadi — `Tag` bo'sh.
        assert_eq!(find("101-xona").kind, ElementKind::Room);
        // Geometriya yozuvi element emas.
        assert!(!g.elements.iter().any(|e| e.mark.contains("Model")));
    }

    #[test]
    fn relations_are_built() {
        let m = parse(SAMPLE);
        let g = to_graph(&m, 1);
        let idx = |mark: &str| g.elements.iter().position(|e| e.mark == mark).unwrap();
        let (wall, opening, window, space) =
            (idx("D-1"), idx("PR-1"), idx("OK-1"), idx("101-xona"));

        assert!(g.links.contains(&(wall, opening, Relation::Contains)));
        assert!(g.links.contains(&(opening, window, Relation::Contains)));
        assert!(g.links.contains(&(space, wall, Relation::Contains)));
        // Xona chegarasi elementning xonasini ham to'ldiradi.
        assert_eq!(g.elements[wall].room, "101-xona");
    }

    /// Buzilgan fayl butun importni yo'qotmaydi.
    #[test]
    fn broken_lines_are_skipped_not_fatal() {
        let broken = "ISO-10303-21;\nDATA;\n#1= IFCWALL('a',$,'W',$,$,$,$,'W-1');\nbu qator noto'g'ri\n#nomer= IFCDOOR(;\n#3= IFCDOOR('b',$,'D',$,$,$,$,'D-9');\nENDSEC;";
        let m = parse(broken);
        let g = to_graph(&m, 1);
        assert!(g.elements.iter().any(|e| e.mark == "W-1"));
        assert!(g.elements.iter().any(|e| e.mark == "D-9"));
    }

    #[test]
    fn empty_input_is_safe() {
        let m = parse("");
        assert!(m.entities.is_empty());
        let g = to_graph(&m, 1);
        assert!(g.elements.is_empty() && g.links.is_empty());
    }

    /// Kirill harflari `\X2\...\X0\` ko'rinishida kelsa ham to'g'ri o'qiladi.
    #[test]
    fn extended_characters_are_decoded() {
        let src = "DATA;\n#1= IFCSPACE('a',$,'\\X2\\041A043E043C043D04300442values\\X0\\',$,$,$,$,$,$,$,$);\nENDSEC;";
        let m = parse(src);
        let name = m.get(1).unwrap().text(2).unwrap();
        assert!(name.starts_with("Комнат"), "nomi: {name}");
    }

    /// Bir xil faylni ikki marta o'qish bir xil tartib beradi.
    #[test]
    fn import_is_repeatable() {
        let m = parse(SAMPLE);
        let a = to_graph(&m, 1);
        let b = to_graph(&m, 1);
        let marks = |g: &Graph| {
            g.elements
                .iter()
                .map(|e| e.mark.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(marks(&a), marks(&b));
        assert_eq!(a.links, b.links);
    }
}
