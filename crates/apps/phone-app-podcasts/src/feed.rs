//! Bounded RSS 2 / Atom enclosure import. XML never enters a web/HTML surface.
use crate::model::{Episode, Show, MAX_EPISODES};
use roxmltree::Node;
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}
fn text(node: Node<'_, '_>, name: &str, max: usize) -> String {
    child(node, name)
        .map(|child| {
            plain(
                &child
                    .descendants()
                    .filter_map(|part| if part.is_text() { part.text() } else { None })
                    .collect::<String>(),
                max,
                1024,
            )
        })
        .unwrap_or_default()
}
/// Bound both display length and UTF-8 storage while collapsing markup/spacing.
/// A pending separator and its following character are admitted together so
/// truncation never exceeds either budget or leaves a trailing space.
pub fn plain(input: &str, max_chars: usize, max_bytes: usize) -> String {
    let mut tag = false;
    let mut output = String::with_capacity(input.len().min(max_bytes));
    let mut chars = 0;
    let mut space = false;
    for ch in input.chars() {
        if ch == '<' {
            tag = true;
            space = true;
            continue;
        }
        if ch == '>' && tag {
            tag = false;
            continue;
        }
        if tag {
            continue;
        }
        if ch.is_whitespace() || ch.is_control() {
            space = true;
            continue;
        }
        let separator = usize::from(space && !output.is_empty());
        let needed_chars = separator + 1;
        let needed_bytes = separator + ch.len_utf8();
        if needed_chars > max_chars.saturating_sub(chars)
            || needed_bytes > max_bytes.saturating_sub(output.len())
        {
            break;
        }
        if separator != 0 {
            output.push(' ');
        }
        space = false;
        output.push(ch);
        chars += needed_chars;
    }
    output
}
pub fn parse(bytes: &[u8], feed: Option<String>) -> Result<Show, String> {
    if bytes.len() > phone_host::http::MAX_RESPONSE_BYTES as usize {
        return Err("The podcast feed exceeds 2 MiB.".into());
    }
    let input = std::str::from_utf8(bytes).map_err(|_| "Choose a UTF-8 RSS or Atom feed.")?;
    let doc = roxmltree::Document::parse_with_options(
        input,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 60_000,
        },
    )
    .map_err(|_| {
        "The podcast feed is not supported XML. DTDs and external entities are not allowed."
    })?;
    let root = doc.root_element();
    let atom = root.tag_name().name() == "feed";
    let channel = if atom {
        root
    } else if root.tag_name().name() == "rss" {
        child(root, "channel").ok_or("The RSS feed has no channel.")?
    } else {
        return Err("Choose an RSS 2.0 or Atom podcast feed.".into());
    };
    let title = text(channel, "title", 240);
    if title.is_empty() {
        return Err("This podcast has no title.".into());
    }
    let supplied = feed
        .map(|feed| phone_host::http::validate_url(&feed))
        .transpose()?;
    let inferred = channel
        .children()
        .find(|node| {
            node.is_element()
                && node.tag_name().name() == "link"
                && node.attribute("rel") == Some("self")
        })
        .and_then(|node| node.attribute("href"))
        .and_then(|url| phone_host::http::validate_url(url).ok());
    let mut show = Show {
        id: 0,
        title,
        description: text(channel, if atom { "subtitle" } else { "description" }, 512),
        feed: supplied.or(inferred),
        episodes: Vec::new(),
    };
    let mut sources = std::collections::HashSet::new();
    for item in channel.children().filter(|node| {
        node.is_element() && node.tag_name().name() == if atom { "entry" } else { "item" }
    }) {
        let enclosure = item.children().find(|node| {
            node.is_element()
                && ((atom
                    && node.tag_name().name() == "link"
                    && node.attribute("rel") == Some("enclosure"))
                    || (!atom && node.tag_name().name() == "enclosure"))
        });
        let Some(enclosure) = enclosure else {
            continue;
        };
        let mime = enclosure.attribute("type").unwrap_or("");
        if !mime.starts_with("audio/") && !mime.starts_with("video/") {
            continue;
        }
        let Some(source) = enclosure
            .attribute(if atom { "href" } else { "url" })
            .and_then(|url| phone_host::http::validate_url(url).ok())
        else {
            continue;
        };
        if !sources.insert(source.clone()) {
            continue;
        }
        let title = text(item, "title", 240);
        if title.is_empty() {
            continue;
        }
        let description = text(item, if atom { "summary" } else { "description" }, 512);
        let date = text(item, if atom { "published" } else { "pubDate" }, 100);
        let published = chrono::DateTime::parse_from_rfc2822(&date)
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(&date))
            .map_or(0, |date| date.timestamp());
        let duration = duration(&text(item, "duration", 40)).unwrap_or(0);
        show.episodes.push(Episode {
            title,
            source,
            description,
            published,
            duration,
            video: mime.starts_with("video/"),
            played: false,
            position: 0,
        });
        // Collect a bounded input window before choosing the newest episodes.
        if show.episodes.len() >= 256 {
            break;
        }
    }
    show.episodes
        .sort_by_key(|episode| std::cmp::Reverse(episode.published));
    show.episodes.truncate(MAX_EPISODES);
    if show.episodes.is_empty() {
        return Err("This feed has no playable HTTPS audio or video enclosures.".into());
    }
    Ok(show)
}
fn duration(value: &str) -> Option<u32> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut seconds = 0u32;
    for (index, part) in parts.iter().enumerate() {
        let value = part.parse::<u32>().ok()?;
        if index > 0 && value > 59 {
            return None;
        }
        seconds = seconds.checked_mul(60)?.checked_add(value)?;
    }
    (seconds <= 31_536_000).then_some(seconds)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_text_budgets_include_pending_separators_and_whole_utf8_characters() {
        assert_eq!(plain("ab c", 3, 1024), "ab");
        assert_eq!(plain("ab c", 4, 1024), "ab c");
        assert_eq!(plain("ab \t<b>c</b>", 3, 1024), "ab");
        assert_eq!(plain("  abc \n", 3, 3), "abc");
        assert_eq!(plain("abc", 0, 1024), "");
        assert_eq!(plain("abc", 3, 0), "");
        assert_eq!(plain("ab 🎙", 4, 6), "ab");
        assert_eq!(plain("ab 🎙", 4, 7), "ab 🎙");
        assert_eq!(plain(&"語".repeat(400), 512, 1024), "語".repeat(341));
        assert_eq!(plain(&"🎙".repeat(400), 512, 1024), "🎙".repeat(256));
    }
    #[test]
    fn unicode_feed_metadata_and_boundary_space_titles_remain_persistable() {
        let title = format!("{} B", "A".repeat(239));
        let xml = format!(
            "<rss version=\"2.0\"><channel><title>{title}</title><description>{}</description><item><title>{title}</title><description>{}</description><enclosure url=\"https://example.org/episode.mp3\" type=\"audio/mpeg\"/></item></channel></rss>",
            "語".repeat(400),
            "🎙".repeat(400),
        );
        let show = parse(xml.as_bytes(), None).unwrap();
        assert_eq!(show.title, "A".repeat(239));
        assert_eq!(show.episodes[0].title, show.title);
        assert_eq!(show.description, "語".repeat(341));
        assert_eq!(show.episodes[0].description, "🎙".repeat(256));
        let mut saved = crate::model::Saved::default();
        saved.upsert(show).unwrap();
        saved.validate().unwrap();
    }
    #[test]
    fn rss_enclosures_are_secure_unique_and_preserve_duration() {
        let feed=br#"<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"><channel><title>Show</title><item><title>One</title><description><![CDATA[<b>Hello</b> world]]></description><enclosure url="https://example.org/one.mp3" type="audio/mpeg"/><itunes:duration>01:02:03</itunes:duration></item><item><title>Duplicate</title><enclosure url="https://example.org/one.mp3" type="audio/mpeg"/></item><item><title>Local</title><enclosure url="https://127.0.0.1/file" type="audio/mpeg"/></item></channel></rss>"#;
        let show = parse(feed, None).unwrap();
        assert_eq!(show.episodes.len(), 1);
        assert_eq!(show.episodes[0].duration, 3723);
        assert_eq!(show.episodes[0].description, "Hello world");
    }
    #[test]
    fn atom_import_and_update_preserve_listened_state() {
        let xml=br#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Atom show</title><entry><title>Episode</title><link rel="enclosure" href="https://example.org/episode.mp3" type="audio/mpeg"/><published>2026-11-23T10:00:00Z</published></entry></feed>"#;
        let show = parse(xml, Some("https://example.org/feed".into())).unwrap();
        let mut saved = crate::model::Saved::default();
        let id = saved.upsert(show.clone()).unwrap();
        saved.shows[0].episodes[0].played = true;
        saved.shows[0].episodes[0].position = 42;
        assert_eq!(saved.upsert(show).unwrap(), id);
        assert!(saved.shows[0].episodes[0].played);
        assert_eq!(saved.shows[0].episodes[0].position, 42);
    }
    #[test]
    fn malformed_dtd_or_non_audio_feeds_are_rejected() {
        for xml in [
            "<html/>",
            "<!DOCTYPE rss [<!ENTITY hi 'test'>]><rss><channel><title>&hi;</title></channel></rss>",
            "<rss><channel><title>No episodes</title></channel></rss>",
        ] {
            assert!(parse(xml.as_bytes(), None).is_err());
        }
        assert_eq!(duration("1:99"), None);
    }
}
