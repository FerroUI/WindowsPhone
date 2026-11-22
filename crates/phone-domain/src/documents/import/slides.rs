use super::relationships;
use crate::documents::{
    archive::{self, Package},
    *,
};

pub(super) fn read(package: &Package) -> Result<Presentation, String> {
    let xml = archive::xml(package, "ppt/presentation.xml")?;
    let relationships = relationships(package, "ppt/presentation.xml")?;
    let mut slides = Vec::new();
    for item in xml
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "sldId")
    {
        let id = item
            .attributes()
            .find(|a| a.name() == "id" && a.namespace().is_some())
            .map(|a| a.value())
            .ok_or("A slide is missing its relationship.")?;
        let path = relationships
            .iter()
            .find(|(key, kind, _)| key == id && kind.ends_with("/slide"))
            .map(|(_, _, p)| p)
            .ok_or("A presentation slide relationship is invalid.")?;
        let slide = archive::xml(package, path)?;
        if slide.descendants().any(|n| {
            n.is_element()
                && [
                    "pic",
                    "graphicFrame",
                    "oleObj",
                    "videoFile",
                    "audioFile",
                    "timing",
                    "transition",
                    "grpSp",
                    "cxnSp",
                ]
                .contains(&n.tag_name().name())
        }) {
            return Err("This presentation contains images, charts, media, grouped shapes or animation that cannot be preserved. Only simple text slides can be imported for editing.".into());
        }
        let mut title = String::new();
        let mut body = Vec::new();
        for shape in slide
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "sp")
        {
            let text = text_of(shape);
            let is_title = shape.descendants().any(|n| {
                n.is_element()
                    && n.tag_name().name() == "ph"
                    && matches!(n.attribute("type"), Some("title" | "ctrTitle"))
            });
            if is_title && title.is_empty() {
                title = text;
            } else if !text.is_empty() {
                body.push(text);
            }
        }
        let notes_path = super::relationships(package, path)?
            .into_iter()
            .find(|(_, kind, _)| kind.ends_with("/notesSlide"))
            .map(|(_, _, path)| path);
        let notes = if let Some(path) = notes_path {
            let notes = archive::xml(package, &path)?;
            notes
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "sp")
                .filter(|shape| {
                    shape.descendants().any(|n| {
                        n.is_element()
                            && n.tag_name().name() == "ph"
                            && n.attribute("type") == Some("body")
                    })
                })
                .map(text_of)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };
        slides.push(Slide {
            title,
            body: body.join("\n"),
            notes,
        });
    }
    if slides.is_empty() {
        return Err("This presentation contains no slides.".into());
    }
    Ok(Presentation { slides })
}
fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "p")
        .map(|p| {
            p.descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "t")
                .filter_map(|n| n.text())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
