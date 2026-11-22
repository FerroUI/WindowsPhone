//! The browser journal records navigation requested through the phone UI.
//! Embedded cross-origin page navigation is deliberately outside this model.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_BROWSER_TABS: usize = 32;
const MAX_TAB_ENTRIES: usize = 128;
const MAX_HISTORY: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressButton {
    #[default]
    Tabs,
    Favorites,
    Refresh,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserVisit {
    pub url: String,
    pub title: String,
    pub visited: i64,
    #[serde(default = "one_visit")]
    pub visits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: u64,
    pub journal: Vec<String>,
    pub position: usize,
}
impl BrowserTab {
    fn blank(id: u64) -> Self {
        Self {
            id,
            journal: Vec::new(),
            position: 0,
        }
    }
    pub fn url(&self) -> &str {
        self.journal
            .get(self.position)
            .map(String::as_str)
            .unwrap_or("")
    }
    pub fn can_back(&self) -> bool {
        self.position > 0
    }
    pub fn can_forward(&self) -> bool {
        self.position + 1 < self.journal.len()
    }
    pub fn title(&self) -> String {
        page_title(self.url())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserData {
    pub tabs: Vec<BrowserTab>,
    pub active: u64,
    pub history: Vec<BrowserVisit>,
    pub next_tab: u64,
    pub address_button: AddressButton,
}
impl Default for BrowserData {
    fn default() -> Self {
        Self {
            tabs: vec![BrowserTab::blank(1)],
            active: 1,
            history: Vec::new(),
            next_tab: 2,
            address_button: AddressButton::Tabs,
        }
    }
}
#[derive(Clone, Debug)]
pub enum BrowserAction {
    Navigate(String),
    Back,
    Forward,
    NewTab,
    CloseTab(u64),
    ActivateTab(u64),
    ClearHistory,
    RemoveHistory(String),
    SetAddressButton(AddressButton),
}
impl BrowserData {
    pub fn current(&self) -> &BrowserTab {
        self.tabs
            .iter()
            .find(|tab| tab.id == self.active)
            .unwrap_or(&self.tabs[0])
    }
    pub fn url(&self) -> &str {
        self.current().url()
    }
    pub fn normalize(&mut self) -> Result<(), String> {
        if self.tabs.len() > MAX_BROWSER_TABS || self.history.len() > MAX_HISTORY {
            return Err("The browser backup exceeds its tab or history limit.".into());
        }
        if self.tabs.is_empty() {
            self.tabs.push(BrowserTab::blank(1));
        }
        let mut ids = BTreeSet::new();
        let mut highest = 0;
        for tab in &mut self.tabs {
            if tab.id == 0 || !ids.insert(tab.id) || tab.journal.len() > MAX_TAB_ENTRIES {
                return Err("The browser backup contains invalid tabs.".into());
            }
            highest = highest.max(tab.id);
            for entry in &mut tab.journal {
                *entry = canonical_url(entry)?;
            }
            tab.position = tab.position.min(tab.journal.len().saturating_sub(1));
        }
        self.next_tab = self.next_tab.max(
            highest
                .checked_add(1)
                .ok_or("The browser tab identifier is out of range.")?,
        );
        if !ids.contains(&self.active) {
            self.active = self.tabs[0].id;
        }
        let mut urls = BTreeSet::new();
        for item in &mut self.history {
            item.url = canonical_url(&item.url)?;
            item.title = item.title.chars().take(256).collect();
            item.visits = item.visits.max(1);
        }
        self.history
            .sort_by_key(|item| std::cmp::Reverse(item.visited));
        self.history.retain(|item| urls.insert(item.url.clone()));
        Ok(())
    }
    pub fn apply(&mut self, action: BrowserAction, now: i64) -> Result<bool, String> {
        match action {
            BrowserAction::Navigate(value) => {
                let url = canonical_url(&value)?;
                let tab = self
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.id == self.active)
                    .ok_or("No browser tab is selected.")?;
                if tab.url() == url {
                    return Ok(false);
                }
                if !tab.journal.is_empty() {
                    tab.journal.truncate(tab.position + 1);
                }
                tab.journal.push(url.clone());
                if tab.journal.len() > MAX_TAB_ENTRIES {
                    tab.journal.remove(0);
                }
                tab.position = tab.journal.len() - 1;
                self.visit(url, now);
                Ok(true)
            }
            BrowserAction::Back | BrowserAction::Forward => {
                let forward = matches!(action, BrowserAction::Forward);
                let tab = self
                    .tabs
                    .iter_mut()
                    .find(|tab| tab.id == self.active)
                    .ok_or("No browser tab is selected.")?;
                if (forward && !tab.can_forward()) || (!forward && !tab.can_back()) {
                    return Ok(false);
                }
                if forward {
                    tab.position += 1;
                } else {
                    tab.position -= 1;
                }
                Ok(true)
            }
            BrowserAction::NewTab => {
                if self.tabs.len() >= MAX_BROWSER_TABS {
                    return Err(format!(
                        "Close a tab before opening another. This app supports {MAX_BROWSER_TABS} open tabs."
                    ));
                }
                let id = self.next_tab;
                self.next_tab = id
                    .checked_add(1)
                    .ok_or("The browser tab identifier is out of range.")?;
                self.tabs.push(BrowserTab::blank(id));
                self.active = id;
                Ok(true)
            }
            BrowserAction::CloseTab(id) => {
                let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
                    return Ok(false);
                };
                if self.tabs.len() == 1 {
                    self.tabs[0] = BrowserTab::blank(id);
                    self.active = id;
                } else {
                    self.tabs.remove(index);
                    if self.active == id {
                        self.active = self.tabs[index.min(self.tabs.len() - 1)].id;
                    }
                }
                Ok(true)
            }
            BrowserAction::ActivateTab(id) => {
                if self.active == id || !self.tabs.iter().any(|tab| tab.id == id) {
                    return Ok(false);
                }
                self.active = id;
                Ok(true)
            }
            BrowserAction::ClearHistory => {
                let changed = !self.history.is_empty();
                self.history.clear();
                Ok(changed)
            }
            BrowserAction::RemoveHistory(url) => {
                let before = self.history.len();
                self.history.retain(|item| item.url != url);
                Ok(before != self.history.len())
            }
            BrowserAction::SetAddressButton(button) => {
                let changed = self.address_button != button;
                self.address_button = button;
                Ok(changed)
            }
        }
    }
    fn visit(&mut self, url: String, now: i64) {
        let visits = self
            .history
            .iter()
            .find(|item| item.url == url)
            .map(|item| item.visits.saturating_add(1))
            .unwrap_or(1);
        self.history.retain(|item| item.url != url);
        self.history.insert(
            0,
            BrowserVisit {
                title: page_title(&url),
                url,
                visited: now,
                visits,
            },
        );
        self.history.truncate(MAX_HISTORY);
    }
}
fn one_visit() -> u32 {
    1
}
pub fn canonical_url(value: &str) -> Result<String, String> {
    if value.len() > 8192 || value.chars().any(char::is_control) {
        return Err("The web address is invalid or too long.".into());
    }
    let url = url::Url::parse(value).map_err(|_| "Enter a valid http or https web address.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Use an http or https web address without embedded credentials.".into());
    }
    Ok(url.to_string())
}
pub fn page_title(value: &str) -> String {
    url::Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| "new tab".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tab_journals_branch_and_remain_independent() {
        let mut data = BrowserData::default();
        data.apply(BrowserAction::Navigate("https://a.example".into()), 1)
            .unwrap();
        data.apply(BrowserAction::Navigate("https://b.example".into()), 2)
            .unwrap();
        data.apply(BrowserAction::Back, 3).unwrap();
        data.apply(BrowserAction::Navigate("https://c.example".into()), 4)
            .unwrap();
        assert!(!data.current().can_forward());
        assert_eq!(
            data.current().journal,
            vec!["https://a.example/", "https://c.example/"]
        );
        data.apply(BrowserAction::NewTab, 5).unwrap();
        let second = data.active;
        assert_eq!(data.url(), "");
        data.apply(BrowserAction::Navigate("https://d.example".into()), 6)
            .unwrap();
        data.apply(BrowserAction::ActivateTab(1), 7).unwrap();
        assert_eq!(data.url(), "https://c.example/");
        data.apply(BrowserAction::CloseTab(1), 8).unwrap();
        assert_eq!(data.active, second);
        assert_eq!(data.url(), "https://d.example/");
    }
    #[test]
    fn bad_urls_cannot_partially_mutate_or_enter_history() {
        let mut data = BrowserData::default();
        let old = data.clone();
        for url in [
            "javascript:alert(1)",
            "data:text/html,hi",
            "https://user:secret@example.com/",
            "https://example.com/\n",
        ] {
            assert!(data.apply(BrowserAction::Navigate(url.into()), 1).is_err());
            assert_eq!(data, old);
        }
    }
    #[test]
    fn history_deduplicates_and_last_closed_tab_is_safe() {
        let mut data = BrowserData::default();
        for (i, url) in [
            "https://a.example",
            "https://b.example",
            "https://a.example",
        ]
        .iter()
        .enumerate()
        {
            data.apply(BrowserAction::Navigate((*url).into()), i as i64)
                .unwrap();
        }
        assert_eq!(data.history.len(), 2);
        assert_eq!(data.history[0].url, "https://a.example/");
        data.apply(BrowserAction::CloseTab(1), 4).unwrap();
        assert_eq!(data.tabs.len(), 1);
        assert_eq!(data.url(), "");
        data.apply(BrowserAction::ClearHistory, 5).unwrap();
        assert!(data.history.is_empty());
    }
    #[test]
    fn import_rejects_duplicate_ids_and_caps_journals() {
        let mut data = BrowserData::default();
        data.tabs.push(BrowserTab::blank(1));
        assert!(data.normalize().is_err());
        let mut data = BrowserData::default();
        data.tabs[0].journal = vec!["https://a.example".into(); MAX_TAB_ENTRIES + 1];
        assert!(data.normalize().is_err());
    }
}
