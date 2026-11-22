use super::*;

#[test]
fn office_roundtrips_editable_content_and_unicode() {
    let documents = [
        DocumentContent::Word(WordDocument {
            paragraphs: vec![
                Paragraph {
                    text: "Zażółć gęślą <&>\nsecond line".into(),
                    bold: true,
                    italic: true,
                    underline: true,
                    heading: 2,
                    bullet: true,
                },
                Paragraph::default(),
            ],
        }),
        DocumentContent::Excel(Workbook {
            sheets: vec![
                Worksheet {
                    name: "Budget".into(),
                    cells: [
                        ("A1".into(), "12.5".into()),
                        ("B1".into(), "=A1*2".into()),
                        ("C2".into(), "notes <&>".into()),
                    ]
                    .into(),
                },
                Worksheet {
                    name: "Two".into(),
                    cells: Default::default(),
                },
            ],
        }),
        DocumentContent::PowerPoint(Presentation {
            slides: vec![
                Slide {
                    title: "One & two".into(),
                    body: "First\nSecond".into(),
                    notes: "Speaker notes".into(),
                },
                Slide {
                    title: "Next".into(),
                    body: "Last".into(),
                    notes: String::new(),
                },
            ],
        }),
    ];
    for content in documents {
        let original = OfficeDocument {
            name: "Test".into(),
            content,
            ..Default::default()
        };
        let export = export_office(&original).unwrap();
        let loaded = import_office(&export.name, &export.bytes).unwrap();
        assert_eq!(original.content, loaded.content, "{}", export.name);
        assert!(export_office(&loaded).unwrap().name.contains("mobile-copy"));
    }
}

#[test]
fn formulas_evaluate_references_ranges_and_fail_safely() {
    let mut sheet = Worksheet::default();
    for (address, value) in [
        ("A1", "12"),
        ("A2", "8"),
        ("A3", "text"),
        ("B1", "=SUM(A1:A3)"),
        ("B2", "=AVERAGE(A1:A3)"),
        ("B3", "=ROUND((B1/3),2)"),
        ("C1", "=C2"),
        ("C2", "=C1"),
        ("C3", "=1/0"),
        ("D1", "=NO_SUCH_FUNCTION(3)"),
        ("D2", "=COUNT(A1:A3)"),
        ("D3", "=2^3+10%"),
    ] {
        sheet.cells.insert(address.into(), value.into());
    }
    for (address, expected) in [
        ("B1", "20"),
        ("B2", "10"),
        ("B3", "6.67"),
        ("C1", "#CYCLE!"),
        ("C3", "#DIV/0!"),
        ("D1", "#NAME?"),
        ("D2", "2"),
        ("D3", "8.1"),
    ] {
        assert_eq!(display_cell(&sheet, address), expected);
    }
    sheet.cells.insert(
        "A4".into(),
        format!("={}1{}", "(".repeat(100), ")".repeat(100)),
    );
    assert_eq!(display_cell(&sheet, "A4"), "#LIMIT!");
}

#[test]
fn csv_handles_quotes_newlines_and_sparse_cells() {
    let csv = "name,notes,value\r\n\"Anna, Alex\",\"first\nsecond \"\"quote\"\"\",=2+3\r\n";
    let sheet = sheet_from_csv(csv).unwrap();
    assert_eq!(sheet.cells["B2"], "first\nsecond \"quote\"");
    assert_eq!(display_cell(&sheet, "C2"), "5");
    assert_eq!(sheet_from_csv(&sheet_to_csv(&sheet)).unwrap(), sheet);
    assert!(sheet_from_csv("\"unclosed").is_err());
}

#[test]
fn archive_rejects_traversal_corruption_and_truncation() {
    assert!(archive::read(&archive::write(vec![("../escape.xml".into(), "x".into())])).is_err());
    assert!(
        archive::read(&archive::write(vec![
            ("same".into(), "x".into()),
            ("same".into(), "y".into())
        ]))
        .is_err()
    );
    let valid = archive::write(vec![("test.xml".into(), "<root>hello</root>".into())]);
    for length in 0..valid.len() {
        assert!(archive::read(&valid[..length]).is_err());
    }
    let mut corrupted = valid;
    corrupted[40] ^= 1;
    assert!(archive::read(&corrupted).is_err());
}

#[test]
fn unsupported_rich_word_is_rejected_without_flattening() {
    let original = export_office(&OfficeDocument::default()).unwrap();
    let package = archive::read(&original.bytes).unwrap();
    let parts = package
        .into_iter()
        .map(|(name, bytes)| {
            let mut content = String::from_utf8(bytes).unwrap();
            if name == "word/document.xml" {
                content = content.replace("<w:body>", "<w:body><w:tbl/>");
            }
            (name, content)
        })
        .collect();
    let error = import_office("test.docx", &archive::write(parts)).unwrap_err();
    assert!(error.contains("cannot preserve"));
}

#[test]
fn library_rejects_cycles_and_case_insensitive_duplicates() {
    let folders = vec![
        DocumentFolder {
            id: 1,
            parent: Some(2),
            name: "One".into(),
        },
        DocumentFolder {
            id: 2,
            parent: Some(1),
            name: "Two".into(),
        },
    ];
    assert!(validate_library(&[], &folders).is_err());
    let a = OfficeDocument {
        id: 1,
        name: "Letter".into(),
        ..Default::default()
    };
    let b = OfficeDocument {
        id: 2,
        name: "letter".into(),
        ..Default::default()
    };
    assert!(validate_library(&[a, b], &[]).is_err());
}

#[test]
fn workbook_interchange_preserves_literal_formula_text_and_booleans() {
    let document = OfficeDocument {
        content: DocumentContent::Excel(Workbook {
            sheets: vec![Worksheet {
                name: "Types".into(),
                cells: [
                    ("A1".into(), "'=SUM(B1:B2)".into()),
                    ("B1".into(), "'001".into()),
                    ("C1".into(), "TRUE".into()),
                    ("D1".into(), "'TRUE".into()),
                    ("E1".into(), "1.0".into()),
                ]
                .into(),
            }],
        }),
        ..Default::default()
    };
    let export = export_office(&document).unwrap();
    let imported = import_office(&export.name, &export.bytes).unwrap();
    assert_eq!(document.content, imported.content);
    let DocumentContent::Excel(book) = imported.content else {
        panic!("workbook")
    };
    assert_eq!(display_cell(&book.sheets[0], "A1"), "=SUM(B1:B2)");
    assert_eq!(display_cell(&book.sheets[0], "B1"), "001");
}
