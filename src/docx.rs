//! Word (.docx) hujjat yozish — hisobot va taklif uchun.
//!
//! DOCX — ZIP ichidagi bir nechta XML. Kutubxona ishlatilmaydi: kerakli
//! qism kichik (sarlavha, abzats, jadval, plitka), va o'z qo'limizda
//! bo'lsa Word'da qanday ochilishini aniq bilamiz. Rang va jadval
//! bezaklari hisobotdagidek: rangli sarlavhalar, sarlavha fonli
//! jadvallar, o'ngga tekis sonlar.

use std::io::Write;
use std::path::Path;

/// Hujjat bloki.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Title(String),
    Heading(String),
    Sub(String),
    Para(String),
    Muted(String),
    /// Jadval: sarlavhalar va qatorlar. Birinchi ustun matn, qolganlari
    /// o'ngga tekis.
    Table(Vec<String>, Vec<Vec<String>>),
    /// Ko'rsatkich plitkalari: `(nom, qiymat, izoh)`.
    Tiles(Vec<(String, String, String)>),
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if c.is_control() && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

fn hex(rgb: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// Och fon rangi: urg'u rangining 12 % i.
fn light(rgb: [u8; 3]) -> [u8; 3] {
    let f = |c: u8| (255 - (255 - c as u16) * 12 / 100) as u8;
    [f(rgb[0]), f(rgb[1]), f(rgb[2])]
}

/// Bitta abzats: `size` — punkt, `color` — hex yoki bo'sh.
fn para(
    text: &str,
    size: f32,
    bold: bool,
    color: &str,
    before: u32,
    after: u32,
    right: bool,
) -> String {
    let mut rpr = format!("<w:sz w:val=\"{}\"/>", (size * 2.0).round() as u32);
    if bold {
        rpr.push_str("<w:b/>");
    }
    if !color.is_empty() {
        rpr.push_str(&format!("<w:color w:val=\"{color}\"/>"));
    }
    let jc = if right { "<w:jc w:val=\"right\"/>" } else { "" };
    // Satr ichidagi `\n` — qator uzilishi.
    let runs: Vec<String> = text
        .split('\n')
        .map(|line| {
            format!(
                "<w:r><w:rPr>{rpr}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>",
                esc(line)
            )
        })
        .collect();
    format!(
        "<w:p><w:pPr><w:spacing w:before=\"{before}\" w:after=\"{after}\"/>{jc}</w:pPr>{}</w:p>",
        runs.join("<w:r><w:br/></w:r>")
    )
}

fn cell(inner: &str, width: u32, fill: &str) -> String {
    let shd = if fill.is_empty() {
        String::new()
    } else {
        format!("<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"{fill}\"/>")
    };
    format!(
        "<w:tc><w:tcPr><w:tcW w:w=\"{width}\" w:type=\"pct\"/>{shd}<w:tcMar><w:left w:w=\"80\" w:type=\"dxa\"/><w:right w:w=\"80\" w:type=\"dxa\"/></w:tcMar></w:tcPr>{inner}</w:tc>"
    )
}

/// Jadval: foizda eni, ingichka chegaralar.
fn table(headers: &[String], rows: &[Vec<String>], rgb: [u8; 3]) -> String {
    let n = headers.len().max(1);
    // Birinchi ustun keng (46 %), qolganlari teng. Birlik — foizning 1/50 i
    // (pct: 5000 = 100 %).
    let first = if n > 1 { 2300 } else { 5000 };
    let rest = if n > 1 {
        (5000 - first) / (n as u32 - 1)
    } else {
        0
    };
    let width = |i: usize| if i == 0 { first } else { rest };
    let border = "<w:top w:val=\"single\" w:sz=\"4\" w:color=\"D0D4DA\"/>\
<w:left w:val=\"single\" w:sz=\"4\" w:color=\"D0D4DA\"/>\
<w:bottom w:val=\"single\" w:sz=\"4\" w:color=\"D0D4DA\"/>\
<w:right w:val=\"single\" w:sz=\"4\" w:color=\"D0D4DA\"/>\
<w:insideH w:val=\"single\" w:sz=\"4\" w:color=\"E4E7EB\"/>\
<w:insideV w:val=\"single\" w:sz=\"4\" w:color=\"E4E7EB\"/>";
    let mut out = format!(
        "<w:tbl><w:tblPr><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblBorders>{border}</w:tblBorders></w:tblPr>"
    );
    let accent = hex(rgb);
    let fill = hex(light(rgb));
    out.push_str("<w:tr><w:trPr><w:tblHeader/></w:trPr>");
    for (i, h) in headers.iter().enumerate() {
        out.push_str(&cell(
            &para(h, 9.0, true, &accent, 40, 40, i > 0),
            width(i),
            &fill,
        ));
    }
    out.push_str("</w:tr>");
    for row in rows {
        out.push_str("<w:tr>");
        for i in 0..n {
            let c = row.get(i).map(String::as_str).unwrap_or("");
            out.push_str(&cell(&para(c, 9.0, i > 0, "", 20, 20, i > 0), width(i), ""));
        }
        out.push_str("</w:tr>");
    }
    out.push_str("</w:tbl>");
    out.push_str(&para("", 4.0, false, "", 0, 0, false));
    out
}

fn tiles(items: &[(String, String, String)], rgb: [u8; 3]) -> String {
    let n = items.len().max(1) as u32;
    let w = 5000 / n;
    let fill = hex(light(rgb));
    let accent = hex(rgb);
    let mut out = String::from(
        "<w:tbl><w:tblPr><w:tblW w:w=\"5000\" w:type=\"pct\"/><w:tblCellSpacing w:w=\"40\" w:type=\"dxa\"/></w:tblPr><w:tr>",
    );
    for (name, value, hint) in items {
        let inner = format!(
            "{}{}{}",
            para(name, 8.0, false, "6B7280", 40, 0, false),
            para(value, 13.0, true, &accent, 0, 0, false),
            para(hint, 7.5, false, "6B7280", 0, 40, false)
        );
        out.push_str(&cell(&inner, w, &fill));
    }
    out.push_str("</w:tr></w:tbl>");
    out.push_str(&para("", 4.0, false, "", 0, 0, false));
    out
}

/// Bloklarni `document.xml` tanasiga aylantiradi.
pub fn body(blocks: &[Block], rgb: [u8; 3]) -> String {
    let accent = hex(rgb);
    let mut out = String::new();
    for b in blocks {
        out.push_str(&match b {
            Block::Title(s) => para(s, 18.0, true, &accent, 120, 80, false),
            Block::Heading(s) => para(s, 13.0, true, &accent, 200, 60, false),
            Block::Sub(s) => para(s, 10.5, true, "", 100, 40, false),
            Block::Para(s) => para(s, 10.0, false, "", 0, 80, false),
            Block::Muted(s) => para(s, 8.5, false, "6B7280", 0, 60, false),
            Block::Table(h, r) => table(h, r, rgb),
            Block::Tiles(t) => tiles(t, rgb),
        });
    }
    out
}

/// Hujjatni faylga yozadi.
pub fn write(path: &Path, title: &str, blocks: &[Block], rgb: [u8; 3]) -> Result<(), String> {
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:body>{}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
<w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\" w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/>\
</w:sectPr></w:body></w:document>",
        body(blocks, rgb)
    );
    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\" w:eastAsia=\"Calibri\"/>\
<w:sz w:val=\"20\"/><w:lang w:val=\"ru-RU\"/></w:rPr></w:rPrDefault>\
<w:pPrDefault><w:pPr><w:spacing w:after=\"80\" w:line=\"264\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\
</w:styles>";
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
</Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
</Relationships>";
    let doc_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
</Relationships>";
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>QURAi</dc:creator></cp:coreProperties>",
        esc(title)
    );

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in [
        ("[Content_Types].xml", content_types.to_string()),
        ("_rels/.rels", rels.to_string()),
        ("word/document.xml", document),
        ("word/styles.xml", styles.to_string()),
        ("word/_rels/document.xml.rels", doc_rels.to_string()),
        ("docProps/core.xml", core),
    ] {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_escaped_and_blocks_become_xml() {
        let b = body(
            &[
                Block::Title("A & B <c>".into()),
                Block::Table(
                    vec!["Nom".into(), "Soni".into()],
                    vec![vec!["Beton".into(), "10".into()]],
                ),
                Block::Tiles(vec![("Jami".into(), "5".into(), String::new())]),
            ],
            [31, 78, 160],
        );
        assert!(b.contains("A &amp; B &lt;c&gt;"));
        assert!(b.contains("<w:tblHeader/>"));
        assert!(b.contains(&format!("w:fill=\"{}\"", hex(light([31, 78, 160])))));
        assert!(b.matches("<w:tbl>").count() == 2);
        assert!(b.contains("<w:jc w:val=\"right\"/>"));
    }

    /// Fayl haqiqatan ZIP va ichida kerakli qismlar bor.
    #[test]
    fn a_document_is_written_as_a_zip_package() {
        let path = std::env::temp_dir().join(format!("qurai_docx_{}.docx", std::process::id()));
        write(
            &path,
            "Sinov",
            &[
                Block::Heading("Bo'lim".into()),
                Block::Para("Matn\nikkinchi".into()),
            ],
            [34, 120, 80],
        )
        .expect("yozish");
        let file = std::fs::File::open(&path).unwrap();
        let mut z = zip::ZipArchive::new(file).expect("zip");
        let names: Vec<String> = (0..z.len())
            .map(|i| z.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.contains(&"word/document.xml".to_string()));
        assert!(names.contains(&"[Content_Types].xml".to_string()));
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("Bo'lim") && doc.contains("<w:br/>"));
        let _ = std::fs::remove_file(&path);
    }
}
