use super::{super::text::*, dates};
use std::collections::BTreeMap;

pub(super) fn reminder(values: &[Property], title: &str) -> Result<u32, String> {
    let mut fields = BTreeMap::new();
    for value in values {
        if fields.insert(value.name.as_str(), value).is_some() {
            return Err("Duplicate calendar reminder property.".into());
        }
        match value.name.as_str() {
            "ACTION" | "DESCRIPTION" => allowed_params(value, &[])?,
            "TRIGGER" => {
                allowed_params(value, &["RELATED", "VALUE"])?;
                if value
                    .params
                    .get("RELATED")
                    .is_some_and(|v| !v.eq_ignore_ascii_case("START"))
                    || value
                        .params
                        .get("VALUE")
                        .is_some_and(|v| !v.eq_ignore_ascii_case("DURATION"))
                {
                    return Err(
                        "Only reminders relative to the appointment start can be imported.".into(),
                    );
                }
            }
            _ => {
                return Err(format!(
                    "Reminder field {} cannot be preserved.",
                    value.name
                ));
            }
        }
    }
    if fields
        .get("ACTION")
        .is_none_or(|v| !v.value.eq_ignore_ascii_case("DISPLAY"))
    {
        return Err("Only display reminders can be imported.".into());
    }
    if fields
        .get("DESCRIPTION")
        .map(|v| unescape(&v.value))
        .transpose()?
        .as_deref()
        != Some(title)
    {
        return Err(
            "A reminder with a separate message cannot be preserved; use the appointment subject."
                .into(),
        );
    }
    let trigger = fields
        .get("TRIGGER")
        .ok_or("The reminder needs a trigger.")?;
    dates::duration(&trigger.value, true)
}
