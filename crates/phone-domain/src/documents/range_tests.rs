use super::*;

fn sheet(values: &[(&str, &str)]) -> Worksheet {
    Worksheet {
        cells: values
            .iter()
            .map(|(a, v)| ((*a).into(), (*v).into()))
            .collect(),
        ..Default::default()
    }
}

#[test]
fn selections_normalize_reversed_addresses_and_bound_work() {
    let range = CellRange::parse(" c12 : $a$1 ").unwrap();
    assert_eq!(range.label(), "A1:C12");
    assert_eq!(range.count(), 36);
    assert!(range.contains(1, 4));
    assert!(!range.contains(3, 4));
    assert_eq!(CellRange::parse("IV10000").unwrap().label(), "IV10000");
    for invalid in ["", "A0", "IW1", "A10001", "A1:B2:C3", "A1:IV10000"] {
        assert!(CellRange::parse(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn autosum_validates_before_writing_and_handles_empty_references() {
    let mut values = sheet(&[("A1", "12"), ("A2", "8"), ("A3", "label")]);
    let original = values.clone();
    let range = CellRange::parse("A1:A3").unwrap();
    for (function, destination) in [("SUM", "A1"), ("BAD", "B1"), ("SUM", "IW1")] {
        assert!(autosum_cell_range(&mut values, range, function, destination).is_err());
        assert_eq!(values, original);
    }
    assert_eq!(
        autosum_cell_range(&mut values, range, "sum", "b1").unwrap(),
        "B1"
    );
    assert_eq!(display_cell(&values, "B1"), "20");
    autosum_cell_range(&mut values, CellRange::parse("C1").unwrap(), "COUNT", "D1").unwrap();
    assert_eq!(display_cell(&values, "D1"), "0");
    let current = values.clone();
    assert!(autosum_cell_range(&mut values, range, "SUM", "B1").is_err());
    assert_eq!(values, current);

    // The selected columns/rows are exact; neighboring values and a heading
    // must not be silently included when a selection starts away from A1.
    let mut offset = sheet(&[
        ("A1", "1000"),
        ("B3", "500"),
        ("C2", "heading"),
        ("C3", "3"),
        ("D3", "4"),
        ("C4", "5"),
        ("D4", "6"),
    ]);
    autosum_cell_range(&mut offset, CellRange::parse("C3:D4").unwrap(), "SUM", "C5").unwrap();
    assert_eq!(offset.cells["C5"], "=SUM(C3:D4)");
    assert_eq!(display_cell(&offset, "C5"), "18");
}

#[test]
fn sort_preserves_row_pairs_headers_blank_order_and_stable_ties() {
    let mut values = sheet(&[
        ("A1", "score"),
        ("B1", "name"),
        ("A2", "12"),
        ("B2", "Anna"),
        ("A3", "8"),
        ("B3", "Ben"),
        ("B4", "Blank"),
        ("A5", "8"),
        ("B5", "Chris"),
        ("C1", "outside"),
    ]);
    let range = CellRange::parse("A1:B5").unwrap();
    sort_cell_range(&mut values, range, 0, false, true).unwrap();
    assert_eq!(values.cells["A1"], "score");
    assert_eq!(values.cells["B2"], "Ben");
    assert_eq!(values.cells["B3"], "Chris");
    assert_eq!(values.cells["B4"], "Anna");
    assert_eq!(values.cells["B5"], "Blank");
    assert!(!values.cells.contains_key("A5"));
    assert_eq!(values.cells["C1"], "outside");
    sort_cell_range(&mut values, range, 0, true, true).unwrap();
    assert_eq!(values.cells["B2"], "Anna");
    assert_eq!(values.cells["B5"], "Blank");
}

#[test]
fn sort_rejects_formulas_atomically_and_clear_respects_selection() {
    let mut values = sheet(&[("A1", "12"), ("A2", "8"), ("B2", "=A1+A2"), ("C1", "keep")]);
    let original = values.clone();
    assert!(
        sort_cell_range(
            &mut values,
            CellRange::parse("A1:B2").unwrap(),
            0,
            false,
            false
        )
        .is_err()
    );
    assert_eq!(values, original);
    assert!(
        sort_cell_range(
            &mut values,
            CellRange::parse("A1:A2").unwrap(),
            1,
            false,
            false
        )
        .is_err()
    );
    assert_eq!(values, original);
    clear_cell_range(&mut values, CellRange::parse("A2:B2").unwrap());
    assert_eq!(
        values.cells,
        [("A1".into(), "12".into()), ("C1".into(), "keep".into())].into()
    );
}

#[test]
fn find_uses_sheet_row_column_order_and_wraps_without_evaluating_formulas() {
    let book = Workbook {
        sheets: vec![
            sheet(&[("A10", "needle"), ("A2", "Needle"), ("B1", "=NEEDLE()")]),
            sheet(&[("A1", "needle")]),
        ],
    };
    assert_eq!(
        find_workbook_cell(&book, "needle", None),
        Some((0, "B1".into()))
    );
    assert_eq!(
        find_workbook_cell(&book, "needle", Some((0, "B1"))),
        Some((0, "A2".into()))
    );
    assert_eq!(
        find_workbook_cell(&book, "needle", Some((0, "A2"))),
        Some((0, "A10".into()))
    );
    assert_eq!(
        find_workbook_cell(&book, "needle", Some((0, "A10"))),
        Some((1, "A1".into()))
    );
    assert_eq!(
        find_workbook_cell(&book, "needle", Some((1, "A1"))),
        Some((0, "B1".into()))
    );
    assert!(find_workbook_cell(&book, " ", None).is_none());
}
