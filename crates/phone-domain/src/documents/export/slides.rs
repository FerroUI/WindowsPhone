use super::{override_type, Parts, REL, REL_NS, RN, XML};
use crate::documents::{archive::escape, Presentation};
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const GROUP: &str = "<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>";
const COLOR_MAP: &str = "<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>";

pub(super) fn parts(deck: &Presentation) -> Parts {
    let mut parts = Vec::new();
    let mut ids = String::new();
    let mut rels = format!(
        "<Relationship Id=\"rIdMaster\" Type=\"{REL}slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/><Relationship Id=\"rIdNotes\" Type=\"{REL}notesMaster\" Target=\"notesMasters/notesMaster1.xml\"/>"
    );
    let mut types = override_type("ppt/presentation.xml", "presentationml.presentation.main");
    for (index, slide) in deck.slides.iter().enumerate() {
        let id = index + 1;
        ids.push_str(&format!("<p:sldId id=\"{}\" r:id=\"rId{id}\"/>", 255 + id));
        rels.push_str(&format!(
            "<Relationship Id=\"rId{id}\" Type=\"{REL}slide\" Target=\"slides/slide{id}.xml\"/>"
        ));
        let shapes = format!(
            "{}{}",
            shape(
                2,
                "Title",
                &slide.title,
                400000,
                250000,
                8300000,
                1100000,
                3200,
                Some("title")
            ),
            shape(
                3,
                "Body",
                &slide.body,
                400000,
                1500000,
                8300000,
                3200000,
                2200,
                Some("body")
            )
        );
        parts.push((format!("ppt/slides/slide{id}.xml"),format!("{XML}<p:sld xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\"><p:cSld><p:spTree>{GROUP}{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>")));
        parts.push((format!("ppt/slides/_rels/slide{id}.xml.rels"),relationships(&format!("<Relationship Id=\"rId1\" Type=\"{REL}slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"{REL}notesSlide\" Target=\"../notesSlides/notesSlide{id}.xml\"/>"))));
        types.push_str(&override_type(
            &format!("ppt/slides/slide{id}.xml"),
            "presentationml.slide",
        ));
        let notes = shape(
            2,
            "Notes",
            &slide.notes,
            400000,
            400000,
            6000000,
            6000000,
            1400,
            Some("body"),
        );
        parts.push((format!("ppt/notesSlides/notesSlide{id}.xml"),format!("{XML}<p:notes xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\"><p:cSld><p:spTree>{GROUP}{notes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>")));
        parts.push((format!("ppt/notesSlides/_rels/notesSlide{id}.xml.rels"),relationships(&format!("<Relationship Id=\"rId1\" Type=\"{REL}slide\" Target=\"../slides/slide{id}.xml\"/><Relationship Id=\"rId2\" Type=\"{REL}notesMaster\" Target=\"../notesMasters/notesMaster1.xml\"/>"))));
        types.push_str(&override_type(
            &format!("ppt/notesSlides/notesSlide{id}.xml"),
            "presentationml.notesSlide",
        ));
    }
    parts.push(("ppt/presentation.xml".into(),format!("{XML}<p:presentation xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rIdMaster\"/></p:sldMasterIdLst><p:notesMasterIdLst><p:notesMasterId r:id=\"rIdNotes\"/></p:notesMasterIdLst><p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx=\"9144000\" cy=\"5143500\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/></p:presentation>")));
    parts.push((
        "ppt/_rels/presentation.xml.rels".into(),
        relationships(&rels),
    ));
    parts.push(("ppt/slideMasters/slideMaster1.xml".into(),format!("{XML}<p:sldMaster xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\"><p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld>{COLOR_MAP}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>")));
    parts.push(("ppt/slideMasters/_rels/slideMaster1.xml.rels".into(),relationships(&format!("<Relationship Id=\"rId1\" Type=\"{REL}slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"{REL}theme\" Target=\"../theme/theme1.xml\"/>"))));
    parts.push(("ppt/slideLayouts/slideLayout1.xml".into(),format!("{XML}<p:sldLayout xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\" type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree>{GROUP}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>")));
    parts.push(("ppt/slideLayouts/_rels/slideLayout1.xml.rels".into(),relationships(&format!("<Relationship Id=\"rId1\" Type=\"{REL}slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/>"))));
    parts.push(("ppt/notesMasters/notesMaster1.xml".into(),format!("{XML}<p:notesMaster xmlns:p=\"{P}\" xmlns:a=\"{A}\" xmlns:r=\"{RN}\"><p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld>{COLOR_MAP}<p:notesStyle/></p:notesMaster>")));
    parts.push((
        "ppt/notesMasters/_rels/notesMaster1.xml.rels".into(),
        relationships(&format!(
            "<Relationship Id=\"rId1\" Type=\"{REL}theme\" Target=\"../theme/theme1.xml\"/>"
        )),
    ));
    parts.push(("ppt/theme/theme1.xml".into(), theme()));
    for (path, kind) in [
        (
            "ppt/slideMasters/slideMaster1.xml",
            "presentationml.slideMaster",
        ),
        (
            "ppt/slideLayouts/slideLayout1.xml",
            "presentationml.slideLayout",
        ),
        (
            "ppt/notesMasters/notesMaster1.xml",
            "presentationml.notesMaster",
        ),
        ("ppt/theme/theme1.xml", "theme"),
    ] {
        types.push_str(&override_type(path, kind));
    }
    (
        "ppt/presentation.xml",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        parts,
        types,
    )
}
fn relationships(body: &str) -> String {
    format!("{XML}<Relationships xmlns=\"{REL_NS}\">{body}</Relationships>")
}
#[allow(clippy::too_many_arguments)]
fn shape(
    id: usize,
    name: &str,
    text: &str,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    size: usize,
    placeholder: Option<&str>,
) -> String {
    let placeholder = placeholder
        .map(|kind| format!("<p:ph type=\"{kind}\"/>"))
        .unwrap_or_default();
    let paragraphs=text.split('\n').map(|line|format!("<a:p><a:r><a:rPr lang=\"en-US\" sz=\"{size}\"/><a:t>{}</a:t></a:r><a:endParaRPr lang=\"en-US\"/></a:p>",escape(line))).collect::<String>();
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"{name}\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr>{placeholder}</p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{w}\" cy=\"{h}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"
    )
}
fn theme() -> String {
    let colors = [
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "1F497D"),
        ("lt2", "EEECE1"),
        ("accent1", "4F81BD"),
        ("accent2", "C0504D"),
        ("accent3", "9BBB59"),
        ("accent4", "8064A2"),
        ("accent5", "4BACC6"),
        ("accent6", "F79646"),
        ("hlink", "0000FF"),
        ("folHlink", "800080"),
    ]
    .into_iter()
    .map(|(name, value)| format!("<a:{name}><a:srgbClr val=\"{value}\"/></a:{name}>"))
    .collect::<String>();
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line = format!("<a:ln w=\"9525\">{fill}<a:prstDash val=\"solid\"/></a:ln>");
    format!(
        "{XML}<a:theme xmlns:a=\"{A}\" name=\"Office\"><a:themeElements><a:clrScheme name=\"Office\">{colors}</a:clrScheme><a:fontScheme name=\"Office\"><a:majorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"Calibri\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme><a:fmtScheme name=\"Office\"><a:fillStyleLst>{}</a:fillStyleLst><a:lnStyleLst>{}</a:lnStyleLst><a:effectStyleLst>{}</a:effectStyleLst><a:bgFillStyleLst>{}</a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>",
        fill.repeat(3),
        line.repeat(3),
        "<a:effectStyle><a:effectLst/></a:effectStyle>".repeat(3),
        fill.repeat(3)
    )
}
