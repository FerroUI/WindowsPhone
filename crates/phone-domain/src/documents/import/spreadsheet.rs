use super::{attribute, relationships};
use crate::documents::{
    archive::{self, Package},
    *,
};

pub(super) fn read(package: &Package) -> Result<Workbook, String> {
    if package.keys().any(|p| {
        [
            "xl/drawings/",
            "xl/charts/",
            "xl/pivot",
            "xl/tables/",
            "xl/comments",
            "xl/threadedComments",
        ]
        .iter()
        .any(|prefix| p.starts_with(prefix))
    }) {
        return Err("This workbook contains drawings, charts, tables, comments or pivot data that this editor cannot preserve. Export a worksheet as CSV to import its values.".into());
    }
    let xml = archive::xml(package, "xl/workbook.xml")?;
    let relationships = relationships(package, "xl/workbook.xml")?;
    let shared = if package.contains_key("xl/sharedStrings.xml") {
        let xml = archive::xml(package, "xl/sharedStrings.xml")?;
        xml.descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "si")
            .map(|node| {
                node.descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "t")
                    .filter_map(|n| n.text())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut sheets = Vec::new();
    for node in xml
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sheet")
    {
        let relation =
            attribute(node, "id").ok_or("A workbook sheet is missing its relationship.")?;
        let path = relationships
            .iter()
            .find(|(id, kind, _)| id == relation && kind.ends_with("/worksheet"))
            .map(|(_, _, path)| path)
            .ok_or("This workbook worksheet relationship is invalid.")?;
        let worksheet = archive::xml(package, path)?;
        if worksheet.descendants().any(|n| {
            n.is_element()
                && [
                    "mergeCell",
                    "conditionalFormatting",
                    "dataValidation",
                    "sheetProtection",
                    "autoFilter",
                    "drawing",
                ]
                .contains(&n.tag_name().name())
        }) {
            return Err("This worksheet uses merged cells, conditional formatting, validation, filters or protection that cannot be preserved. Import a CSV copy instead.".into());
        }
        let mut sheet = Worksheet {
            name: node.attribute("name").unwrap_or("Sheet").into(),
            ..Default::default()
        };
        for cell in worksheet
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "c")
        {
            let address = cell
                .attribute("r")
                .ok_or("This worksheet cell has no address.")?
                .to_ascii_uppercase();
            if cell_coordinates(&address).is_none() {
                return Err("This worksheet contains cells outside A1:IV10000.".into());
            }
            let value = cell
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "v")
                .and_then(|n| n.text())
                .unwrap_or("");
            let formula = cell
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "f");
            let value = if let Some(formula) = formula {
                if formula.attribute("t").is_some_and(|v| v != "normal") {
                    return Err("Shared and array formulas cannot be preserved. Re-save the workbook with ordinary formulas or import CSV.".into());
                }
                format!("={}", formula.text().unwrap_or(""))
            } else {
                match cell.attribute("t") {
                    Some("s") => protect_text(
                        shared
                            .get(
                                value
                                    .parse::<usize>()
                                    .map_err(|_| "This shared-string index is invalid.")?,
                            )
                            .ok_or("A worksheet string is missing.")?
                            .clone(),
                    ),
                    Some("inlineStr") => protect_text(
                        cell.descendants()
                            .filter(|n| n.is_element() && n.tag_name().name() == "t")
                            .filter_map(|n| n.text())
                            .collect(),
                    ),
                    Some("str") => protect_text(value.to_owned()),
                    Some("b") => {
                        if value == "1" {
                            "TRUE".into()
                        } else {
                            "FALSE".into()
                        }
                    }
                    _ => value.to_owned(),
                }
            };
            if !value.is_empty() {
                if sheet.cells.insert(address, value).is_some() {
                    return Err("This worksheet has duplicate cell addresses.".into());
                }
            }
        }
        sheets.push(sheet);
    }
    if sheets.is_empty() {
        return Err("This workbook contains no worksheets.".into());
    }
    Ok(Workbook { sheets })
}

fn protect_text(value: String) -> String {
    if value.starts_with(['=', '\''])
        || value.parse::<f64>().is_ok()
        || matches!(value.as_str(), "TRUE" | "FALSE")
    {
        format!("'{value}")
    } else {
        value
    }
}
