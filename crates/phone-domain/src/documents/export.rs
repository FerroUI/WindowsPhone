use super::{archive::escape, models::*, *};
use serde::Serialize;

mod slides;
pub(super) const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
pub(super) const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
pub(super) const RN: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub(super) const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

#[derive(Clone, Debug, Serialize)]
pub struct ExportedDocument {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub fn export_office(document: &OfficeDocument) -> Result<ExportedDocument, String> {
    validate_document(document)?;
    let (root, mime, parts, types) = match &document.content {
        DocumentContent::Word(word) => word_parts(word),
        DocumentContent::Excel(book) => workbook_parts(book),
        DocumentContent::PowerPoint(deck) => slides::parts(deck),
    };
    let mut files = parts;
    files.push(("_rels/.rels".into(),format!("{XML}<Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rId1\" Type=\"{REL}officeDocument\" Target=\"{root}\"/></Relationships>")));
    files.push(("[Content_Types].xml".into(),format!("{XML}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/>{types}</Types>")));
    let suffix = if document.import_notice.is_empty() {
        ""
    } else {
        "-mobile-copy"
    };
    Ok(ExportedDocument {
        name: format!("{}{suffix}.{}", document.name, document.kind().extension()),
        mime: mime.into(),
        bytes: super::archive::write(files),
    })
}
pub(super) fn override_type(path: &str, kind: &str) -> String {
    format!(
        "<Override PartName=\"/{path}\" ContentType=\"application/vnd.openxmlformats-officedocument.{kind}+xml\"/>"
    )
}

type Parts = (&'static str, &'static str, Vec<(String, String)>, String);
fn word_parts(word: &WordDocument) -> Parts {
    let mut paragraphs = String::new();
    for paragraph in &word.paragraphs {
        let style = if paragraph.heading > 0 {
            format!("<w:pStyle w:val=\"Heading{}\"/>", paragraph.heading)
        } else {
            String::new()
        };
        let mut format = String::new();
        if paragraph.bold {
            format.push_str("<w:b/>");
        }
        if paragraph.italic {
            format.push_str("<w:i/>");
        }
        if paragraph.underline {
            format.push_str("<w:u w:val=\"single\"/>");
        }
        let text = format!(
            "{}{}",
            if paragraph.bullet { "• " } else { "" },
            paragraph.text
        );
        let text = text
            .split('\n')
            .map(|line| format!("<w:t xml:space=\"preserve\">{}</w:t>", escape(line)))
            .collect::<Vec<_>>()
            .join("<w:br/>");
        paragraphs.push_str(&format!(
            "<w:p><w:pPr>{style}</w:pPr><w:r><w:rPr>{format}</w:rPr>{text}</w:r></w:p>"
        ));
    }
    let main = format!(
        "{XML}<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{paragraphs}<w:sectPr><w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr></w:body></w:document>"
    );
    let mut styles = format!(
        "{XML}<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\"/><w:sz w:val=\"22\"/></w:rPr></w:style>"
    );
    for level in 1..=3 {
        styles.push_str(&format!("<w:style w:type=\"paragraph\" w:styleId=\"Heading{level}\"><w:name w:val=\"heading {level}\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:outlineLvl w:val=\"{}\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"{}\"/></w:rPr></w:style>",level-1,40-level*4));
    }
    styles.push_str("</w:styles>");
    (
        "word/document.xml",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        vec![
            ("word/document.xml".into(), main),
            ("word/styles.xml".into(), styles),
            (
                "word/_rels/document.xml.rels".into(),
                format!(
                    "{XML}<Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rId1\" Type=\"{REL}styles\" Target=\"styles.xml\"/></Relationships>"
                ),
            ),
        ],
        format!(
            "{}{}",
            override_type("word/document.xml", "wordprocessingml.document.main"),
            override_type("word/styles.xml", "wordprocessingml.styles")
        ),
    )
}

fn workbook_parts(book: &Workbook) -> Parts {
    let mut parts = Vec::new();
    let mut names = String::new();
    let mut relationships = String::new();
    let mut types = override_type("xl/workbook.xml", "spreadsheetml.sheet.main");
    for (index, sheet) in book.sheets.iter().enumerate() {
        let id = index + 1;
        names.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{id}\" r:id=\"rId{id}\"/>",
            escape(&sheet.name)
        ));
        relationships.push_str(&format!("<Relationship Id=\"rId{id}\" Type=\"{REL}worksheet\" Target=\"worksheets/sheet{id}.xml\"/>"));
        let mut rows = std::collections::BTreeMap::<usize, Vec<(usize, &str, &str)>>::new();
        for (address, value) in &sheet.cells {
            if let Some((column, row)) = cell_coordinates(address) {
                rows.entry(row).or_default().push((column, address, value));
            }
        }
        let mut data = String::new();
        for (row, mut cells) in rows {
            cells.sort_by_key(|(c, _, _)| *c);
            data.push_str(&format!("<row r=\"{}\">", row + 1));
            for (_, address, value) in cells {
                let content = if let Some(formula) = value.strip_prefix('=') {
                    let result = display_cell(sheet, address);
                    let cached = result
                        .parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite())
                        .map(|n| format!("<v>{n}</v>"))
                        .unwrap_or_default();
                    format!("<c r=\"{address}\"><f>{}</f>{cached}</c>", escape(formula))
                } else if matches!(value, "TRUE" | "FALSE") {
                    format!(
                        "<c r=\"{address}\" t=\"b\"><v>{}</v></c>",
                        if value == "TRUE" { 1 } else { 0 }
                    )
                } else if let Ok(number) = value.parse::<f64>() {
                    if number.is_finite() {
                        format!("<c r=\"{address}\"><v>{}</v></c>", escape(value))
                    } else {
                        inline_cell(address, value)
                    }
                } else {
                    inline_cell(address, value)
                };
                data.push_str(&content);
            }
            data.push_str("</row>");
        }
        parts.push((format!("xl/worksheets/sheet{id}.xml"),format!("{XML}<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>{data}</sheetData></worksheet>")));
        types.push_str(&override_type(
            &format!("xl/worksheets/sheet{id}.xml"),
            "spreadsheetml.worksheet",
        ));
    }
    parts.push(("xl/workbook.xml".into(),format!("{XML}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"{RN}\"><sheets>{names}</sheets><calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/></workbook>")));
    parts.push((
        "xl/_rels/workbook.xml.rels".into(),
        format!("{XML}<Relationships xmlns=\"{REL_NS}\">{relationships}</Relationships>"),
    ));
    (
        "xl/workbook.xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        parts,
        types,
    )
}
fn inline_cell(address: &str, value: &str) -> String {
    format!(
        "<c r=\"{address}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
        escape(value.strip_prefix('\'').unwrap_or(value))
    )
}
