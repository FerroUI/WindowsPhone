use phone_domain::*;

#[test]
fn tile_layout_never_overlaps_for_mixed_sizes_and_columns() {
    for columns in [4, 6] {
        let tiles = (0..200)
            .map(|id| {
                Tile::new(
                    id + 1,
                    AppId::Phone,
                    match id % 3 {
                        0 => TileSize::Small,
                        1 => TileSize::Medium,
                        _ => TileSize::Wide,
                    },
                )
            })
            .collect::<Vec<_>>();
        let placements = pack_tiles(&tiles, columns);
        let mut occupied = std::collections::HashSet::new();
        assert_eq!(placements.len(), tiles.len());
        assert_eq!(placements, pack_tiles(&tiles, columns));
        for item in placements {
            assert!(item.column + item.width <= columns);
            for y in item.row..item.row + item.height {
                for x in item.column..item.column + item.width {
                    assert!(occupied.insert((x, y)));
                }
            }
        }
    }
}
#[test]
fn persistence_roundtrip_and_id_repair() {
    let data = PhoneData::demo();
    let mut parsed = PhoneData::from_json(&data.to_json().unwrap()).unwrap();
    assert_eq!(parsed, data);
    parsed.next_id = 1;
    parsed.normalize().unwrap();
    assert!(parsed.next_id > 951);
    parsed.tiles.push(parsed.tiles[0].clone());
    assert!(parsed.normalize().is_err());
}
#[test]
fn future_documents_and_invalid_backups_are_rejected() {
    assert!(PhoneData::from_json("{\"version\":999}").is_err());
    assert!(PhoneData::from_json("nope").is_err());
    let mut data = PhoneData::demo();
    data.settings.accent = "not a color".into();
    data.settings.volume = 255;
    data.normalize().unwrap();
    assert_eq!(data.settings.volume, 30);
    assert_eq!(data.settings.accent, "#1BA1E2");
}
#[test]
fn calendar_handles_leap_centuries_and_negative_unix_dates() {
    assert_eq!(days_in_month(2000, 2), 29);
    assert_eq!(days_in_month(2100, 2), 28);
    assert_eq!(date_parts(0), (1970, 1, 1));
    assert_eq!(date_parts(-1), (1969, 12, 31));
    assert_eq!(first_weekday(2026, 11), 6);
    assert_eq!(
        date_parts(days_from_date(2026, 11, 22) * 86400),
        (2026, 11, 22)
    );
}
#[test]
fn calculator_preserves_repeat_equals_and_contextual_percent() {
    let mut calc = Calculator::default();
    for key in ["2", "0", "0", "+", "1", "0", "%", "="] {
        calc.press(key);
    }
    assert_eq!(calc.display(), "220");
    assert_eq!(calc.press("="), "240");
    for key in ["C", "8", "÷", "0", "="] {
        calc.press(key);
    }
    assert_eq!(calc.display(), "Error");
    assert_eq!(calc.press("5"), "5");
    for key in ["M+", "C", "MR"] {
        calc.press(key);
    }
    assert_eq!(calc.display(), "5");
}
