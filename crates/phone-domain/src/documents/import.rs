use super::{
    archive::{self, Package},
    *,
};

mod slides;
mod spreadsheet;

pub fn import_office(name: &str, bytes: &[u8]) -> Result<OfficeDocument, String> {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (stem, extension) = basename
        .rsplit_once('.')
        .ok_or("Choose a .docx, .xlsx, .pptx, .txt or .csv file.")?;
    let name = stem.trim().to_owned();
    validate_name(&name)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("This file exceeds the 8 MB import limit.".into());
    }
    let extension = extension.to_ascii_lowercase();
    let content=match extension.as_str(){
        "txt"=>{
            let text=std::str::from_utf8(bytes).map_err(|_|"Text documents must use UTF-8.")?.trim_start_matches('\u{feff}');
            DocumentContent::Word(WordDocument{paragraphs:text.replace("\r\n","\n").split('\n').map(|line|Paragraph{text:line.into(),..Default::default()}).collect()})
        },
        "csv"=>DocumentContent::Excel(Workbook{sheets:vec![sheet_from_csv(std::str::from_utf8(bytes).map_err(|_|"CSV files must use UTF-8.")?)?]}),
        "docx"|"xlsx"|"pptx"=>{
            let package=archive::read(bytes)?;
            reject_active_content(&package)?;
            match extension.as_str(){"docx"=>DocumentContent::Word(word(&package)?),"xlsx"=>DocumentContent::Excel(spreadsheet::read(&package)?),_=>DocumentContent::PowerPoint(slides::read(&package)?) }
        },
        _=>return Err("Choose a .docx, .xlsx, .pptx, .txt or .csv file. Legacy binary Office files and PDFs are not editable here.".into()),
    };
    let document = OfficeDocument {
        name,
        content,
        import_notice: if matches!(extension.as_str(), "txt" | "csv") {
            String::new()
        } else {
            "Imported mobile copy. Text and supported formatting are editable; desktop fonts, page layout, themes and document metadata are not preserved. Export creates a separate mobile-copy file.".into()
        },
        ..Default::default()
    };
    validate_document(&document)?;
    Ok(document)
}

fn reject_active_content(package: &Package) -> Result<(), String> {
    for name in package.keys() {
        let name = name.to_ascii_lowercase();
        if name.contains("vbaproject")
            || name.contains("activex/")
            || name.contains("embeddings/")
            || name.contains("externallinks/")
            || name.contains("digitalsignature")
            || name.ends_with("encryptedpackage")
        {
            return Err("This document contains macros, embedded objects, signatures or external links that cannot be preserved. Open the original in its Office application.".into());
        }
    }
    Ok(())
}

fn word(package: &Package) -> Result<WordDocument, String> {
    let document = archive::xml(package, "word/document.xml")?;
    if document.descendants().any(|n| {
        n.is_element()
            && [
                "drawing",
                "pict",
                "tbl",
                "sdt",
                "ins",
                "del",
                "fldChar",
                "fldSimple",
                "hyperlink",
                "footnoteReference",
                "endnoteReference",
                "commentRangeStart",
                "numPr",
                "headerReference",
                "footerReference",
            ]
            .contains(&n.tag_name().name())
    }) {
        return Err("This Word file includes tables, images, lists, fields, comments or tracked changes that this editor cannot preserve. Save a plain-text copy in Word to import its text.".into());
    }
    let mut paragraphs = Vec::new();
    let body = document
        .descendants()
        .find(|n| {
            n.has_tag_name((
                "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
                "body",
            ))
        })
        .ok_or("This Word document has no body.")?;
    for p in body
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "p")
    {
        let text = p
            .descendants()
            .filter(|n| n.is_element())
            .filter_map(|n| match n.tag_name().name() {
                "t" => Some(n.text().unwrap_or("").to_owned()),
                "br" | "cr" => Some("\n".into()),
                "tab" => Some("\t".into()),
                _ => None,
            })
            .collect::<String>();
        let styles = p
            .descendants()
            .filter(|n| {
                n.is_element()
                    && n.tag_name().name() == "r"
                    && n.descendants()
                        .any(|c| c.is_element() && c.tag_name().name() == "t")
            })
            .map(|run| {
                let enabled = |name| {
                    run.children()
                        .filter(|n| n.is_element() && n.tag_name().name() == "rPr")
                        .flat_map(|n| n.children())
                        .any(|n| {
                            n.is_element()
                                && n.tag_name().name() == name
                                && !matches!(attribute(n, "val"), Some("0" | "false" | "none"))
                        })
                };
                (enabled("b"), enabled("i"), enabled("u"))
            })
            .collect::<Vec<_>>();
        let style = styles.first().copied().unwrap_or_default();
        if styles.iter().any(|s| *s != style) {
            return Err("This Word paragraph mixes character formatting that this editor cannot preserve. Save a plain-text copy to import the text.".into());
        }
        let heading = p
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "pStyle")
            .and_then(|n| attribute(n, "val"))
            .and_then(|s| s.strip_prefix("Heading"))
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0)
            .min(3);
        let (bullet, text) = text
            .strip_prefix("• ")
            .map(|s| (true, s.to_owned()))
            .unwrap_or((false, text));
        paragraphs.push(Paragraph {
            text,
            bold: style.0,
            italic: style.1,
            underline: style.2,
            heading,
            bullet,
        });
    }
    if paragraphs.is_empty() {
        paragraphs.push(Paragraph::default());
    }
    Ok(WordDocument { paragraphs })
}

pub(super) fn attribute<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| a.value())
}
pub(super) fn relationships(
    package: &Package,
    part: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let (directory, name) = part.rsplit_once('/').unwrap_or(("", part));
    let path = if directory.is_empty() {
        format!("_rels/{name}.rels")
    } else {
        format!("{directory}/_rels/{name}.rels")
    };
    if !package.contains_key(&path) {
        return Ok(Vec::new());
    }
    let xml = archive::xml(package, &path)?;
    xml.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Relationship")
        .map(|node| {
            if node.attribute("TargetMode") == Some("External") {
                return Err(
                    "External document relationships cannot be preserved by this editor.".into(),
                );
            }
            let id = node
                .attribute("Id")
                .ok_or("An Office relationship is missing its ID.")?
                .to_owned();
            let kind = node.attribute("Type").unwrap_or("").to_owned();
            let target = node
                .attribute("Target")
                .ok_or("An Office relationship is missing its target.")?;
            Ok((id, kind, resolve(directory, target)?))
        })
        .collect()
}
fn resolve(directory: &str, target: &str) -> Result<String, String> {
    if target.contains(['\\', ':', '?', '#']) {
        return Err("An Office relationship has an unsupported target.".into());
    }
    let mut parts = if target.starts_with('/') {
        Vec::new()
    } else {
        directory
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    };
    for piece in target.split('/') {
        match piece {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("An Office relationship escapes the document package.".into());
                }
            }
            _ => parts.push(piece),
        }
    }
    Ok(parts.join("/"))
}
