mod events;
mod recurrence;

pub use events::*;
pub use recurrence::*;

/// Calendar algorithms use a proleptic Gregorian calendar without timezone or OS dependencies.
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    }
}
pub fn days_from_date(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}
pub fn date_parts(timestamp: i64) -> (i32, u32, u32) {
    let z = timestamp.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (
        (year + i64::from(month <= 2)) as i32,
        month as u32,
        day as u32,
    )
}
pub fn first_weekday(year: i32, month: u32) -> usize {
    (days_from_date(year, month, 1) + 3).rem_euclid(7) as usize
}
pub fn date_string(year: i32, month: u32, day: u32) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}
pub fn time_text(timestamp: i64, time_24h: bool) -> String {
    let seconds = timestamp.rem_euclid(86400);
    let hour = seconds / 3600;
    let minute = seconds % 3600 / 60;
    if time_24h {
        format!("{hour:02}:{minute:02}")
    } else {
        format!(
            "{}:{minute:02} {}",
            if hour % 12 == 0 { 12 } else { hour % 12 },
            if hour < 12 { "AM" } else { "PM" }
        )
    }
}
pub fn weekday_name(timestamp: i64) -> &'static str {
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ][(timestamp.div_euclid(86400) + 3).rem_euclid(7) as usize]
}
pub fn month_name(month: u32) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .get(month.wrapping_sub(1) as usize)
    .copied()
    .unwrap_or("")
}
