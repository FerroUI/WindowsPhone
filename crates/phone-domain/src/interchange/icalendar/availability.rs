use super::super::text::Property;
use crate::CalendarStatus;
use std::collections::BTreeMap;

pub(super) fn read(fields: &BTreeMap<&str, &Property>) -> Result<CalendarStatus, String> {
    let value = |name: &str| fields.get(name).map(|v| v.value.to_ascii_uppercase());
    let status = value("STATUS");
    if status
        .as_deref()
        .is_some_and(|v| !matches!(v, "TENTATIVE" | "CONFIRMED"))
    {
        return Err("Cancelled appointments cannot be imported as active appointments.".into());
    }
    let transparency = match value("TRANSP").as_deref() {
        Some("TRANSPARENT") => Some(true),
        Some("OPAQUE") => Some(false),
        None => None,
        _ => return Err("Invalid appointment availability.".into()),
    };
    if let Some(busy) = value("X-MICROSOFT-CDO-BUSYSTATUS") {
        let busy = match busy.as_str() {
            "FREE" => CalendarStatus::Free,
            "TENTATIVE" => CalendarStatus::Tentative,
            "BUSY" => CalendarStatus::Busy,
            "OOF" => CalendarStatus::OutOfOffice,
            _ => return Err("Unsupported appointment availability.".into()),
        };
        if transparency.is_some_and(|free| free != (busy == CalendarStatus::Free))
            || status.as_deref().is_some_and(|status| {
                (status == "TENTATIVE") != (busy == CalendarStatus::Tentative)
            })
        {
            return Err("Conflicting calendar availability fields cannot be preserved.".into());
        }
        return Ok(busy);
    }
    if status.as_deref() == Some("TENTATIVE") {
        if transparency == Some(true) {
            return Err("A tentative, free appointment cannot be represented by one phone availability setting.".into());
        }
        Ok(CalendarStatus::Tentative)
    } else if transparency == Some(true) {
        Ok(CalendarStatus::Free)
    } else {
        Ok(CalendarStatus::Busy)
    }
}
