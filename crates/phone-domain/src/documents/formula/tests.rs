use super::*;
fn eval(formula: &str) -> String {
    let sheet = Worksheet {
        cells: [("A1".into(), formula.into())].into(),
        ..Default::default()
    };
    display_cell(&sheet, "A1")
}
#[test]
fn arithmetic_uses_excel_precedence_and_bounded_types() {
    for (formula, result) in [
        ("=-2^2", "4"),
        ("=2^3^2", "64"),
        ("=2^3%", "1.0210121257"),
        ("=1+2*3=7", "TRUE"),
        ("=2<>3", "TRUE"),
        ("=4>=5", "FALSE"),
        ("=TRUE+2", "3"),
        ("=\"Total: \"&ROUND(1.245,2)", "Total: 1.25"),
        ("=\"AbC\"=\"abc\"", "TRUE"),
        ("=\"4\"=4", "FALSE"),
        ("=1e3+2", "1002"),
        ("=1e309", "#NUM!"),
        ("=A1$", "#REF!"),
        ("=NO_SUCH(1)", "#NAME?"),
    ] {
        assert_eq!(eval(formula), result, "{formula}");
    }
}
#[test]
fn branches_are_lazy_but_all_syntax_is_validated() {
    for (formula, result) in [
        ("=IF(2>1,42,1/0)", "42"),
        ("=IF(FALSE,1/0,\"ready\")", "ready"),
        ("=IF(FALSE,42)", "FALSE"),
        ("=IFERROR(1/0,\"unavailable\")", "unavailable"),
        ("=IFERROR(3,1/0)", "3"),
        ("=AND(TRUE,2,NOT(FALSE))", "TRUE"),
        ("=OR(FALSE,0,2)", "TRUE"),
        ("=AND(FALSE,1/0)", "#DIV/0!"),
        ("=IF(TRUE,1,(2+))", "#VALUE!"),
        ("=IF(1)", "#VALUE!"),
    ] {
        assert_eq!(eval(formula), result, "{formula}");
    }
}
#[test]
fn aggregate_references_preserve_text_blank_boolean_and_error_semantics() {
    let mut sheet = Worksheet {
        cells: [
            ("A1".into(), "10".into()),
            ("A2".into(), "'30".into()),
            ("A3".into(), "TRUE".into()),
            ("A4".into(), "=\"\"".into()),
            ("A5".into(), "=2*10".into()),
        ]
        .into(),
        ..Default::default()
    };
    for (formula, result) in [
        ("=SUM(A1:A6)", "30"),
        ("=SUM(A2,A3,A6)", "0"),
        ("=SUM(\"30\",TRUE)", "31"),
        ("=AVERAGE(A1:A6)", "15"),
        ("=COUNT(A1:A6)", "2"),
        ("=COUNTA(A1:A6)", "5"),
        ("=COUNTBLANK(A1:A6)", "2"),
        ("=PRODUCT(A1:A6)", "200"),
        ("=MEDIAN(A1:A6)", "15"),
        ("=SUMSQ(A1:A6)", "500"),
        ("=SUM(A6:A1)", "30"),
        ("=SUM($A$1:$A$6)", "30"),
    ] {
        sheet.cells.insert("B1".into(), formula.into());
        assert_eq!(display_cell(&sheet, "B1"), result, "{formula}");
    }
    sheet.cells.insert("A6".into(), "=1/0".into());
    for (formula, result) in [
        ("=SUM(A1:A6)", "#DIV/0!"),
        ("=COUNT(A1:A6)", "2"),
        ("=COUNTA(A1:A6)", "6"),
    ] {
        sheet.cells.insert("B1".into(), formula.into());
        assert_eq!(display_cell(&sheet, "B1"), result, "{formula}");
    }
}
#[test]
fn scientific_rounding_and_text_functions_are_usable() {
    for (formula, result) in [
        ("=ROUND(-123.456,-1)", "-120"),
        ("=ROUNDUP(-1.21,1)", "-1.3"),
        ("=ROUNDDOWN(-1.29,1)", "-1.2"),
        ("=TRUNC(-3.9)", "-3"),
        ("=INT(-3.9)", "-4"),
        ("=MOD(-3,2)", "1"),
        ("=MOD(3,-2)", "-1"),
        ("=POWER(2,4)+SQRT(81)", "25"),
        ("=LOG(8,2)", "3"),
        ("=SQRT(-1)", "#NUM!"),
        ("=MOD(3,0)", "#DIV/0!"),
        ("=LOWER(\"ŻÓŁĆ\")", "żółć"),
        ("=UPPER(\"notes\")", "NOTES"),
        ("=TRIM(\"  one   two  \")", "one two"),
        ("=LEFT(\"Phone\",2)", "Ph"),
        ("=RIGHT(\"Phone\",3)", "one"),
        ("=MID(\"Phone\",2,3)", "hon"),
        ("=LEN(\"a💡b\")", "4"),
        ("=MID(\"a💡b\",2,2)", "💡"),
        ("=CONCATENATE(\"a\";TRUE();2)", "aTRUE2"),
        ("=\"say \"\"hi\"\"\"", "say \"hi\""),
    ] {
        assert_eq!(eval(formula), result, "{formula}");
    }
}
#[test]
fn cycles_and_computation_limits_remain_explicit() {
    assert_eq!(eval("=A1"), "#CYCLE!");
    assert_eq!(eval("=SUM(B1:IV10000)"), "#LIMIT!");
    assert_eq!(
        eval(&format!("={}1{}", "(".repeat(100), ")".repeat(100))),
        "#LIMIT!"
    );
    assert_eq!(
        eval(&format!(
            "=CONCATENATE({})",
            std::iter::repeat_n("\"x\"", 256)
                .collect::<Vec<_>>()
                .join(",")
        )),
        "#LIMIT!"
    );
    assert_eq!(cell_coordinates("$A$1"), Some((0, 0)));
    for invalid in ["$$A1", "A1$", "A$1$", "1", "A0", "IW1"] {
        assert!(cell_coordinates(invalid).is_none(), "{invalid}");
    }
}

#[test]
fn long_flat_formulas_do_not_consume_recursive_depth() {
    let formula = format!(
        "={}",
        std::iter::repeat_n("1", 4096).collect::<Vec<_>>().join("+")
    );
    assert_eq!(eval(&formula), "4096");
    assert_eq!(eval("=IF(TRUE,42,missing_name)"), "42");
    assert_eq!(eval("=IFERROR(missing_name,7)"), "7");
}
