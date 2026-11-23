use super::super::{InterchangeFile, MAX_INTERCHANGE_ITEMS, file, text::*};
use crate::{
    CalendarEvent, CalendarRepeat, CalendarStatus, calendar_date, calendar_day,
    validate_calendar_event,
};

pub fn export_icalendar(
    events: &[CalendarEvent],
    utc_stamp: i64,
) -> Result<InterchangeFile, String> {
    if events.is_empty() || events.len() > MAX_INTERCHANGE_ITEMS {
        return Err("Choose between 1 and 512 appointments to export.".into());
    }
    let stamp_day = utc_stamp.div_euclid(86_400);
    if !(crate::days_from_date(1, 1, 1)..=crate::days_from_date(9999, 12, 31)).contains(&stamp_day)
    {
        return Err("The host clock is outside the supported calendar range.".into());
    }
    let day = calendar_date(stamp_day);
    let second = utc_stamp.rem_euclid(86_400);
    let stamp = format!(
        "{}T{:02}{:02}{:02}Z",
        day.replace('-', ""),
        second / 3600,
        second / 60 % 60,
        second % 60
    );
    let mut output = String::new();
    for line in [
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//FerroUI//Windows Phone//EN",
        "CALSCALE:GREGORIAN",
        "METHOD:PUBLISH",
    ] {
        push_line(&mut output, line);
    }
    for event in events {
        validate_calendar_event(event)?;
        if event.all_day && event.time != "00:00" {
            return Err(
                "An all-day appointment must use midnight before it can be exported.".into(),
            );
        }
        if event.repeat == CalendarRepeat::Never && event.repeat_until.is_some() {
            return Err(
                "Remove the repeat end date from a nonrecurring appointment before exporting."
                    .into(),
            );
        }
        if event.repeat == CalendarRepeat::Weekdays
            && (calendar_day(&event.date).unwrap() + 3).rem_euclid(7) >= 5
        {
            return Err("A weekday series must begin on a weekday to preserve its iCalendar occurrence set.".into());
        }
        push_line(&mut output, "BEGIN:VEVENT");
        push_line(
            &mut output,
            &format!("UID:phone-{}-{}@ferroui.com", event.id, event.date),
        );
        push_line(&mut output, &format!("DTSTAMP:{stamp}"));
        push_line(&mut output, &format!("SUMMARY:{}", escape(&event.title)));
        push_line(
            &mut output,
            &format!(
                "DTSTART{}:{}",
                if event.all_day { ";VALUE=DATE" } else { "" },
                datetime(&event.date, &event.time, event.all_day)
            ),
        );
        push_line(
            &mut output,
            &if event.all_day {
                format!("DURATION:P{}D", event.duration_minutes / 1_440)
            } else {
                format!("DURATION:PT{}M", event.duration_minutes)
            },
        );
        if !event.location.is_empty() {
            push_line(
                &mut output,
                &format!("LOCATION:{}", escape(&event.location)),
            );
        }
        if !event.details.is_empty() {
            push_line(
                &mut output,
                &format!("DESCRIPTION:{}", escape(&event.details)),
            );
        }
        if event.repeat != CalendarRepeat::Never {
            let mut rule = match event.repeat {
                CalendarRepeat::Daily => "FREQ=DAILY",
                CalendarRepeat::Weekdays => "FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR",
                CalendarRepeat::Weekly => "FREQ=WEEKLY",
                CalendarRepeat::Biweekly => "FREQ=WEEKLY;INTERVAL=2",
                CalendarRepeat::Monthly => "FREQ=MONTHLY",
                CalendarRepeat::Yearly => "FREQ=YEARLY",
                CalendarRepeat::Never => unreachable!(),
            }
            .to_owned();
            if let Some(until) = &event.repeat_until {
                rule.push_str(&format!(
                    ";UNTIL={}",
                    datetime(until, &event.time, event.all_day)
                ));
            }
            push_line(&mut output, &format!("RRULE:{rule}"));
            for excluded in &event.excluded_dates {
                push_line(
                    &mut output,
                    &format!(
                        "EXDATE{}:{}",
                        if event.all_day { ";VALUE=DATE" } else { "" },
                        datetime(excluded, &event.time, event.all_day)
                    ),
                );
            }
        }
        push_line(
            &mut output,
            if event.status == CalendarStatus::Free {
                "TRANSP:TRANSPARENT"
            } else {
                "TRANSP:OPAQUE"
            },
        );
        push_line(
            &mut output,
            if event.status == CalendarStatus::Tentative {
                "STATUS:TENTATIVE"
            } else {
                "STATUS:CONFIRMED"
            },
        );
        push_line(
            &mut output,
            &format!(
                "X-MICROSOFT-CDO-BUSYSTATUS:{}",
                match event.status {
                    CalendarStatus::Free => "FREE",
                    CalendarStatus::Tentative => "TENTATIVE",
                    CalendarStatus::Busy => "BUSY",
                    CalendarStatus::OutOfOffice => "OOF",
                }
            ),
        );
        if let Some(minutes) = event.reminder_minutes {
            for line in [
                "BEGIN:VALARM".into(),
                "ACTION:DISPLAY".into(),
                format!("TRIGGER:-PT{minutes}M"),
                format!("DESCRIPTION:{}", escape(&event.title)),
                "END:VALARM".into(),
            ] {
                push_line(&mut output, &line);
            }
        }
        push_line(&mut output, "END:VEVENT");
    }
    push_line(&mut output, "END:VCALENDAR");
    file(
        if events.len() == 1 {
            "appointment.ics"
        } else {
            "calendar.ics"
        },
        "text/calendar;charset=utf-8",
        output,
    )
}

fn datetime(date: &str, time: &str, all_day: bool) -> String {
    let date = date.replace('-', "");
    if all_day {
        date
    } else {
        format!("{date}T{}00", time.replace(':', ""))
    }
}
