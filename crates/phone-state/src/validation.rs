use phone_domain::{days_in_month, CalendarEvent};

pub(crate) fn event(event: &CalendarEvent) -> Result<(), String> {
    if event.title.trim().is_empty() {
        return Err("Give your appointment a subject.".into());
    }
    let parts = event.date.split('-').collect::<Vec<_>>();
    let valid_date = if parts.len() == 3 {
        let year = parts[0].parse::<i32>().ok();
        let month = parts[1].parse::<u32>().ok();
        let day = parts[2].parse::<u32>().ok();
        match (year, month, day) {
            (Some(y), Some(m), Some(d)) => {
                (1..=9999).contains(&y)
                    && (1..=12).contains(&m)
                    && d > 0
                    && d <= days_in_month(y, m)
            }
            _ => false,
        }
    } else {
        false
    };
    if !valid_date {
        return Err("Enter a date in YYYY-MM-DD format.".into());
    }
    let parts = event.time.split(':').collect::<Vec<_>>();
    let valid_time = parts.len() == 2
        && parts[0].parse::<u8>().is_ok_and(|h| h < 24)
        && parts[1].parse::<u8>().is_ok_and(|m| m < 60);
    if !valid_time {
        return Err("Enter a time in HH:MM format.".into());
    }
    if event.duration_minutes == 0 || event.duration_minutes > 10080 {
        return Err("Choose a duration between 1 minute and 7 days.".into());
    }
    Ok(())
}
pub(crate) fn web_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}
