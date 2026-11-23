use super::{InterchangeFile, MAX_INTERCHANGE_ITEMS, file, text::*};
use crate::Contact;

pub fn validate_contact_card(contact: &Contact) -> Result<(), String> {
    if contact.name.trim().is_empty() || contact.name.chars().count() > 256 {
        return Err("Give each contact a name of 1 to 256 characters.".into());
    }
    for value in [
        &contact.name,
        &contact.phone,
        &contact.email,
        &contact.company,
    ] {
        if value.len() > 4_096 || value.chars().any(|ch| ch.is_control()) {
            return Err("A contact field is too long or contains control characters.".into());
        }
    }
    Ok(())
}

/// Imports UTF-8 vCard 3.0/4.0 display names, one phone, one email and company.
/// Rich fields and additional addresses are refused so callers can retain the
/// original file and choose another contact provider instead of losing data.
pub fn import_vcards(bytes: &[u8]) -> Result<Vec<Contact>, String> {
    let mut cards = Vec::new();
    let mut properties = Vec::new();
    let mut inside = false;
    for line in lines(bytes)? {
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            if inside {
                return Err("A contact contains a nested vCard.".into());
            }
            inside = true;
        } else if line.eq_ignore_ascii_case("END:VCARD") {
            if !inside {
                return Err("The vCard end has no matching beginning.".into());
            }
            if cards.len() >= MAX_INTERCHANGE_ITEMS {
                return Err("Import up to 512 contacts at a time.".into());
            }
            cards.push(card(&properties)?);
            properties.clear();
            inside = false;
        } else if inside {
            properties.push(property(&line)?);
        } else {
            return Err("Choose a vCard (.vcf) contact file.".into());
        }
    }
    if inside || cards.is_empty() {
        return Err("The contact file is empty or incomplete.".into());
    }
    Ok(cards)
}

fn card(properties: &[Property]) -> Result<Contact, String> {
    let version = properties
        .first()
        .filter(|p| p.name == "VERSION")
        .ok_or("The first vCard property must be VERSION.")?;
    if !matches!(version.value.as_str(), "3.0" | "4.0") {
        return Err("Import UTF-8 vCard 3.0 or 4.0 contacts.".into());
    }
    allowed_params(version, &[])?;
    let (mut name, mut phone, mut email, mut company) = (None, None, None, None);
    for value in &properties[1..] {
        match value.name.as_str() {
            "FN" => {
                allowed_params(value, &["LANGUAGE"])?;
                once(&mut name, unescape(&value.value)?, "display name")?;
            }
            // N is the structured spelling of FN. This phone stores the display
            // name, so a new copy always obtains its own provider identity.
            "N" => {
                allowed_params(value, &["LANGUAGE"])?;
                if fields(&value.value, ';')?.len() != 5 {
                    return Err("A structured contact name must have five components.".into());
                }
            }
            "TEL" => {
                allowed_params(value, &["TYPE", "PREF", "VALUE"])?;
                label(value, &["CELL", "VOICE", "PREF"])?;
                let kind = value.params.get("VALUE").map(String::as_str).unwrap_or(
                    if version.value == "4.0" {
                        "uri"
                    } else {
                        "text"
                    },
                );
                if !kind.eq_ignore_ascii_case("text") && !kind.eq_ignore_ascii_case("uri") {
                    return Err("Unsupported phone number representation.".into());
                }
                let number = if kind.eq_ignore_ascii_case("uri") {
                    if !value
                        .value
                        .get(..4)
                        .is_some_and(|p| p.eq_ignore_ascii_case("tel:"))
                    {
                        return Err("Only tel: contact phone URIs can be imported.".into());
                    }
                    decode_phone_uri(&value.value[4..])?
                } else {
                    unescape(&value.value)?
                };
                once(&mut phone, number, "phone number")?;
            }
            "EMAIL" => {
                allowed_params(value, &["TYPE", "PREF"])?;
                label(value, &["INTERNET", "PREF"])?;
                once(&mut email, unescape(&value.value)?, "email address")?;
            }
            "ORG" => {
                allowed_params(value, &["LANGUAGE"])?;
                let parts = fields(&value.value, ';')?;
                if parts.iter().skip(1).any(|part| !part.is_empty()) {
                    return Err(
                        "Organization departments cannot be preserved by this phone.".into(),
                    );
                }
                once(&mut company, parts[0].clone(), "company")?;
            }
            "UID" | "REV" | "PRODID" => {
                allowed_params(value, &[])?;
            }
            _ => {
                return Err(format!(
                    "Contact field {} cannot be preserved. Import a card with name, phone, email and company only.",
                    value.name
                ));
            }
        }
    }
    let contact = Contact {
        name: name
            .ok_or("Each contact needs an FN display name.")?
            .trim()
            .into(),
        phone: phone.unwrap_or_default(),
        email: email.unwrap_or_default(),
        company: company.unwrap_or_default(),
        ..Default::default()
    };
    validate_contact_card(&contact)?;
    Ok(contact)
}

fn label(value: &Property, supported: &[&str]) -> Result<(), String> {
    if value.params.get("TYPE").is_some_and(|kinds| {
        kinds
            .split(',')
            .any(|kind| !supported.contains(&kind.to_ascii_uppercase().as_str()))
    }) {
        return Err(format!(
            "{} home/work or custom labels cannot be preserved by this phone.",
            value.name
        ));
    }
    if value.params.get("PREF").is_some_and(|preference| {
        preference
            .parse::<u8>()
            .ok()
            .is_none_or(|v| !(1..=100).contains(&v))
    }) {
        return Err("A contact preference must be between 1 and 100.".into());
    }
    Ok(())
}

fn decode_phone_uri(value: &str) -> Result<String, String> {
    let mut decoded = Vec::with_capacity(value.len());
    let mut bytes = value.as_bytes().iter();
    while let Some(byte) = bytes.next() {
        if *byte == b'%' {
            let hi = bytes
                .next()
                .and_then(|b| (*b as char).to_digit(16))
                .ok_or("Invalid telephone URI escape.")?;
            let lo = bytes
                .next()
                .and_then(|b| (*b as char).to_digit(16))
                .ok_or("Invalid telephone URI escape.")?;
            decoded.push((hi * 16 + lo) as u8);
        } else {
            decoded.push(*byte);
        }
    }
    String::from_utf8(decoded).map_err(|_| "A telephone URI contains invalid UTF-8.".into())
}

pub fn export_vcards(contacts: &[Contact]) -> Result<InterchangeFile, String> {
    if contacts.is_empty() || contacts.len() > MAX_INTERCHANGE_ITEMS {
        return Err("Choose between 1 and 512 contacts to export.".into());
    }
    let mut output = String::new();
    for contact in contacts {
        validate_contact_card(contact)?;
        for line in [
            "BEGIN:VCARD".into(),
            "VERSION:4.0".into(),
            format!("FN:{}", escape(&contact.name)),
        ] {
            push_line(&mut output, &line);
        }
        if !contact.phone.is_empty() {
            push_line(
                &mut output,
                &format!("TEL;VALUE=text:{}", escape(&contact.phone)),
            );
        }
        if !contact.email.is_empty() {
            push_line(&mut output, &format!("EMAIL:{}", escape(&contact.email)));
        }
        if !contact.company.is_empty() {
            push_line(&mut output, &format!("ORG:{}", escape(&contact.company)));
        }
        push_line(&mut output, "END:VCARD");
    }
    file(
        if contacts.len() == 1 {
            "contact.vcf"
        } else {
            "contacts.vcf"
        },
        "text/vcard;charset=utf-8",
        output,
    )
}
