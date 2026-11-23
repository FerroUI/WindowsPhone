//! Checkboxes use portable Markdown syntax inside the existing note body.
pub struct Item<'a> {
    pub marker: usize,
    pub checked: bool,
    pub label: &'a str,
}
pub fn parse(line: &str) -> Option<Item<'_>> {
    let line = line.trim_end_matches('\r');
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let mut start = indent;
    if line[start..].starts_with("- ") || line[start..].starts_with("* ") {
        start += 2;
    }
    let rest = &line[start..];
    if rest.len() < 3 || rest.as_bytes()[0] != b'[' || rest.as_bytes()[2] != b']' {
        return None;
    }
    let checked = match rest.as_bytes()[1] {
        b' ' => false,
        b'x' | b'X' => true,
        _ => return None,
    };
    if rest.len() > 3 && !rest[3..].starts_with(' ') {
        return None;
    }
    Some(Item {
        marker: start + 1,
        checked,
        label: rest[3..].trim_start_matches(' '),
    })
}
pub fn toggle(body: &str, line: usize) -> Result<String, String> {
    let mut offset = 0;
    for (index, content) in body.split_inclusive('\n').enumerate() {
        if index == line {
            let content = content.strip_suffix('\n').unwrap_or(content);
            let item =
                parse(content).ok_or("This to-do item changed. Reopen the note and try again.")?;
            let mut result = body.to_owned();
            result.replace_range(
                offset + item.marker..offset + item.marker + 1,
                if item.checked { " " } else { "x" },
            );
            return Ok(result);
        }
        offset += content.len();
    }
    Err("This to-do item no longer exists.".into())
}
pub fn summary(body: &str) -> Option<(usize, usize)> {
    let mut total = 0;
    let mut done = 0;
    for item in body.lines().filter_map(parse) {
        total += 1;
        done += usize::from(item.checked);
    }
    (total > 0).then_some((done, total))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggling_preserves_unicode_indentation_and_line_endings() {
        let body = "żółć\r\n  - [ ] buy milk\r\n* [X] done\nlast line";
        let changed = toggle(body, 1).unwrap();
        assert_eq!(changed, "żółć\r\n  - [x] buy milk\r\n* [X] done\nlast line");
        assert_eq!(summary(&changed), Some((2, 2)));
        assert_eq!(toggle(&changed, 1).unwrap(), body);
        assert!(toggle(body, 0).is_err());
        assert!(toggle(body, 99).is_err());
        assert!(parse("[x]word").is_none());
        assert_eq!(parse("[ ]").unwrap().label, "");
    }
}
