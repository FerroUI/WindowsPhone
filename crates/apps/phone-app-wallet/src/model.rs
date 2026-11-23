use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    Membership,
    Ticket,
    Offer,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Membership => "membership",
            Self::Ticket => "ticket",
            Self::Offer => "offer",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Membership => Self::Ticket,
            Self::Ticket => Self::Offer,
            Self::Offer => Self::Membership,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Barcode {
    #[default]
    Text,
    Code39,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Card {
    pub id: u32,
    pub title: String,
    pub issuer: String,
    pub code: String,
    pub expires: String,
    pub notes: String,
    pub kind: Kind,
    pub barcode: Barcode,
    pub favorite: bool,
    pub archived: bool,
}
impl Card {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty()
            || self.title.chars().count() > 120
            || self.issuer.chars().count() > 120
            || self.code.chars().count() > 256
            || self.notes.len() > 4096
            || [&self.title, &self.issuer, &self.code]
                .iter()
                .any(|value| value.chars().any(char::is_control))
        {
            return Err("Enter a title and keep card fields within their limits.".into());
        }
        if !self.expires.is_empty()
            && chrono::NaiveDate::parse_from_str(&self.expires, "%Y-%m-%d").is_err()
        {
            return Err("Enter the expiry date as YYYY-MM-DD, or leave it blank.".into());
        }
        if self.barcode == Barcode::Code39 {
            phone_controls::barcode::code39_modules(&self.code)?;
        }
        Ok(())
    }
    pub fn expired(&self, local_now: i64) -> bool {
        let today = chrono::DateTime::from_timestamp(local_now, 0).map(|date| date.date_naive());
        let expiry = chrono::NaiveDate::parse_from_str(&self.expires, "%Y-%m-%d").ok();
        matches!((today,expiry),(Some(today),Some(expiry))if expiry<today)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Saved {
    pub schema: u32,
    pub next_id: u32,
    pub cards: Vec<Card>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            schema: 1,
            next_id: 1,
            cards: Vec::new(),
        }
    }
}
impl Saved {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 || self.next_id == 0 || self.cards.len() > 100 {
            return Err("The wallet format is unsupported or exceeds 100 cards.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for card in &self.cards {
            card.validate()?;
            if card.id == 0 || card.id >= self.next_id || !ids.insert(card.id) {
                return Err("The wallet contains invalid card identifiers.".into());
            }
        }
        Ok(())
    }
    pub fn save(&mut self, mut card: Card) -> Result<u32, String> {
        card.validate()?;
        if card.id == 0 {
            if self.cards.len() >= 100 {
                return Err("Your wallet holds up to 100 cards.".into());
            }
            card.id = self.next_id;
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or("Wallet identifiers are exhausted.")?;
            let id = card.id;
            self.cards.push(card);
            Ok(id)
        } else {
            let id = card.id;
            let existing = self
                .cards
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("This card no longer exists.")?;
            *existing = card;
            Ok(id)
        }
    }
    pub fn import(&mut self, bytes: &[u8]) -> Result<usize, String> {
        let imported: Saved = serde_json::from_slice(bytes)
            .map_err(|_| "Choose a wallet JSON file exported by this app.")?;
        imported.validate()?;
        let count = imported.cards.len();
        if count + self.cards.len() > 100 {
            return Err("The imported cards would exceed the 100-card limit.".into());
        }
        let mut candidate = self.clone();
        for mut card in imported.cards {
            card.id = 0;
            candidate.save(card)?;
        }
        candidate.validate()?;
        *self = candidate;
        Ok(count)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cards_validate_dates_barcodes_and_import_fresh_identifiers_atomically() {
        let mut saved = Saved::default();
        let id = saved
            .save(Card {
                title: "Library".into(),
                code: "ABC123".into(),
                barcode: Barcode::Code39,
                expires: "2026-11-23".into(),
                ..Default::default()
            })
            .unwrap();
        let bytes = serde_json::to_vec(&saved).unwrap();
        saved.import(&bytes).unwrap();
        assert_ne!(saved.cards[1].id, id);
        assert!(!saved.cards[0].expired(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 23)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp()
        ));
        assert!(saved.cards[0].expired(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 24)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp()
        ));
        let before = saved.cards.clone();
        assert!(saved
            .import(br#"{"schema":2,"next_id":1,"cards":[]}"#)
            .is_err());
        assert_eq!(before, saved.cards);
        assert!(Card {
            title: "Bad".into(),
            expires: "2026-02-30".into(),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
