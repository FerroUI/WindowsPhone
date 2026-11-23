use phone_domain::CalendarEvent;

pub(crate) fn event(event: &CalendarEvent) -> Result<(), String> {
    phone_domain::validate_calendar_event(event)
}
pub(crate) fn web_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}
