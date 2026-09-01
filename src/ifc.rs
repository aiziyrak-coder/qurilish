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
        rest = if end < rest.len() { &rest[end + 4..] } else { "" };
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
        "IFCPIPESEGMENT" | "IFCPIPEFITTING" | "IFCFLOWSEGMENT" => {
            (ElementKind::Pipe, Section::Vk)
        }
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
        "IFCLIGHTFIXTURE" | "IFCELECTRICAPPLIANCE" | "IFCOUTLET" | "IFCSWITCHINGDEVICE"
        | "IFCELECTRICDISTRIBUTIONBOARD" | "IFCDISTRIBUTIONBOARD" => {
            (ElementKind::Device, Section::Eom)
        }
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
    for rel in model.by_kind(&["IFCRELSPACEBOUNDARY", "IFCRELSPACEBOUNDARY1STLEVEL", "IFCRELSPACEBOUNDARY2NDLEVEL"]) {
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
    for rel in model.by_kind(&["IFCRELSPACEBOUNDARY", "IFCRELSPACEBOUNDARY1STLEVEL", "IFCRELSPACEBOUNDARY2NDLEVEL"]) {
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
        let marks = |g: &Graph| g.elements.iter().map(|e| e.mark.clone()).collect::<Vec<_>>();
        assert_eq!(marks(&a), marks(&b));
        assert_eq!(a.links, b.links);
    }
}
