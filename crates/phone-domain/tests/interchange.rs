use phone_domain::{interchange::*, *};

fn contact() -> Contact {
    Contact {
        name: "Zoë Żółć, Jr.; research".into(),
        phone: "+48 555 010 100".into(),
        email: "zoe@example.test".into(),
        company: "Lab, division; A \\ B".into(),
        ..Default::default()
    }
}
fn event() -> CalendarEvent {
    CalendarEvent {
        title: "Design, review; A \\ B".into(),
        date: "2026-11-23".into(),
        time: "09:15".into(),
        duration_minutes: 75,
        details: "Line one\nBEGIN:VEVENT\nLine two".into(),
        location: "Room 2".into(),
        reminder_minutes: Some(15),
        status: CalendarStatus::OutOfOffice,
        ..Default::default()
    }
}

#[test]
fn vcard_roundtrip_preserves_supported_fields_and_folds_unicode_by_octets() {
    let mut value = contact();
    value.name = "Żółć ".repeat(40).trim().into();
    let file = export_vcards(&[value.clone()]).unwrap();
    assert_eq!(file.mime, "text/vcard;charset=utf-8");
    for line in std::str::from_utf8(&file.bytes).unwrap().split("\r\n") {
        assert!(line.len() <= 75);
    }
    assert_eq!(import_vcards(&file.bytes).unwrap(), [value]);
}

#[test]
fn vcard_import_handles_version_three_groups_folds_and_telephone_uris() {
    let bytes = b"BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Alex\r\n  Example\r\nN:Example;Alex;;;\r\nitem1.TEL;TYPE=CELL;VALUE=uri:tel:%2B12025550101\r\nEMAIL;TYPE=INTERNET:alex@example.test\r\nEND:VCARD\r\n";
    let values = import_vcards(bytes).unwrap();
    assert_eq!(values[0].name, "Alex Example");
    assert_eq!(values[0].phone, "+12025550101");
    assert_eq!(values[0].email, "alex@example.test");
}

#[test]
fn unsupported_vcard_content_and_partial_batches_are_rejected() {
    let base = "BEGIN:VCARD\nVERSION:4.0\nFN:Alex\n";
    for tail in [
        "TEL:1\nTEL:2",
        "PHOTO:data:image/png;base64,AAAA",
        "ADR:;;Main street;Town;;;",
        "TEL;ENCODING=b:AAAA",
        "VERSION:4.0",
        "ORG:Company;Department",
        "FN:Other",
        "TEL;VALUE=uri:https://example.test/",
        "TEL;TYPE=HOME;VALUE=text:5550100",
        "EMAIL;TYPE=WORK:alex@example.test",
    ] {
        assert!(
            import_vcards(format!("{base}{tail}\nEND:VCARD\n").as_bytes()).is_err(),
            "{tail}"
        );
    }
    let mut bytes = export_vcards(&[contact()]).unwrap().bytes;
    bytes.extend_from_slice(b"BEGIN:VCARD\nVERSION:4.0\nFN:Incomplete\n");
    assert!(import_vcards(&bytes).is_err());
    assert!(import_vcards(&vec![b'x'; MAX_INTERCHANGE_BYTES + 1]).is_err());
    assert!(import_vcards(b" BEGIN:VCARD\n").is_err());
}

#[test]
fn calendar_roundtrip_preserves_every_supported_recurrence_and_exception() {
    for repeat in CalendarRepeat::ALL {
        let mut item = event();
        item.repeat = repeat;
        if repeat != CalendarRepeat::Never {
            item.repeat_until = Some("2028-11-23".into());
            let from = calendar_day(&item.date).unwrap();
            let next = event_occurrences(&[item.clone()], from + 1, from + 366);
            if let Some(next) = next.first() {
                item.excluded_dates.push(next.date.clone());
            }
        }
        let file = export_icalendar(&[item.clone()], 1_795_420_000).unwrap();
        assert_eq!(import_icalendar(&file.bytes).unwrap(), [item], "{repeat:?}");
        for line in std::str::from_utf8(&file.bytes).unwrap().split("\r\n") {
            assert!(line.len() <= 75);
        }
    }
}

#[test]
fn calendar_all_day_exclusive_end_and_compound_duration_import_correctly() {
    let text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:example\r\nBEGIN:VEVENT\r\nUID:one\r\nSUMMARY:Two days\r\nDTSTART;VALUE=DATE:20261231\r\nDTEND;VALUE=DATE:20270102\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:two\r\nSUMMARY:Overnight\r\nDTSTART:20261123T231500\r\nDURATION:PT1H30M\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let events = import_icalendar(text.as_bytes()).unwrap();
    assert!(events[0].all_day);
    assert_eq!(events[0].duration_minutes, 2_880);
    assert_eq!(events[1].duration_minutes, 90);
    assert_eq!(
        import_icalendar(&export_icalendar(&events, 1_795_420_000).unwrap().bytes).unwrap(),
        events
    );
}

#[test]
fn calendar_rejects_nonrepresentable_zone_meeting_and_recurrence_semantics() {
    let source =
        String::from_utf8(export_icalendar(&[event()], 1_795_420_000).unwrap().bytes).unwrap();
    for (from, to) in [
        ("DTSTART:20261123T091500", "DTSTART:20261123T091500Z"),
        (
            "DTSTART:20261123T091500",
            "DTSTART;TZID=Europe/Warsaw:20261123T091500",
        ),
        (
            "DURATION:PT75M",
            "DURATION:PT75M\r\nATTENDEE:mailto:alex@example.test",
        ),
        (
            "DURATION:PT75M",
            "DURATION:PT75M\r\nRRULE:FREQ=DAILY;COUNT=10",
        ),
        (
            "DURATION:PT75M",
            "DURATION:PT75M\r\nRRULE:FREQ=MONTHLY;BYDAY=1MO",
        ),
        ("ACTION:DISPLAY", "ACTION:EMAIL"),
        ("STATUS:CONFIRMED", "STATUS:TENTATIVE"),
        ("TRANSP:OPAQUE", "TRANSP:TRANSPARENT"),
        ("TRIGGER:-PT15M", "TRIGGER;RELATED=END:-PT15M"),
        (
            "DURATION:PT75M",
            "DURATION:PT75M\r\nRECURRENCE-ID:20261123T091500",
        ),
    ] {
        assert!(
            import_icalendar(source.replace(from, to).as_bytes()).is_err(),
            "{to}"
        );
    }
}

#[test]
fn nonrepresentable_weekday_anchors_and_hidden_date_fields_are_not_silently_exported() {
    let mut value = event();
    value.repeat = CalendarRepeat::Weekdays;
    value.date = "2026-11-21".into();
    assert!(export_icalendar(&[value.clone()], 1_795_420_000).is_err());
    value.date = "2026-11-23".into();
    let file = export_icalendar(&[value.clone()], 1_795_420_000).unwrap();
    let text = String::from_utf8(file.bytes)
        .unwrap()
        .replace("DTSTART:20261123", "DTSTART:20261121");
    assert!(import_icalendar(text.as_bytes()).is_err());
    value.repeat = CalendarRepeat::Never;
    value.repeat_until = Some("2026-12-01".into());
    assert!(export_icalendar(&[value.clone()], 1_795_420_000).is_err());
    value.repeat_until = None;
    value.all_day = true;
    value.duration_minutes = 1440;
    assert!(export_icalendar(&[value], 1_795_420_000).is_err());
}

#[test]
fn malformed_calendar_dates_and_out_of_range_inputs_never_panic() {
    let source =
        String::from_utf8(export_icalendar(&[event()], 1_795_420_000).unwrap().bytes).unwrap();
    for value in [
        "",
        "żółć",
        "20260230T091500",
        "20261123T241500",
        "20261123T091501",
        "00001123T091500",
        "20261123",
        "20261123T091500+0100",
    ] {
        assert!(
            import_icalendar(source.replace("20261123T091500", value).as_bytes()).is_err(),
            "{value}"
        );
    }
    assert!(export_icalendar(&[event()], i64::MAX).is_err());
    assert!(export_icalendar(&[], 0).is_err());
}
