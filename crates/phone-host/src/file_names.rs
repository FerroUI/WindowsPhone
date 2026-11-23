//! Cross-platform, bounded leaf names for app-generated document/media copies.
pub fn copy_name(original: &str, prefix: &str, extension: &str) -> String {
    let mut head: String = prefix
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
        .take(80)
        .collect();
    if head.is_empty() {
        head.push_str("WindowsPhone");
    }
    let extension: String = extension
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase();
    let extension = if extension.is_empty() {
        "bin"
    } else {
        &extension
    };
    let stem = original.rsplit_once('.').map_or(original, |(stem, _)| stem);
    let mut clean = String::new();
    // 240 UTF-8 bytes stays below common 255-byte filesystem component limits.
    let budget = 240 - head.len() - extension.len() - 2;
    for value in stem.chars() {
        let value = if value.is_control() || "<>:\"/\\|?*".contains(value) {
            '_'
        } else {
            value
        };
        if clean.len() + value.len_utf8() > budget {
            break;
        }
        clean.push(value);
    }
    let clean = clean.trim_matches([' ', '.']);
    format!(
        "{head}-{}.{}",
        if clean.is_empty() { "file" } else { clean },
        extension
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_single_components_and_preserve_actual_format() {
        let name = copy_name("../CON<>:\"/\\|?*\n.png", "WindowsPhone", "jpg");
        assert!(name.starts_with("WindowsPhone-"));
        assert!(name.ends_with(".jpg"));
        assert!(!name
            .chars()
            .any(|value| value.is_control() || "<>:\"/\\|?*".contains(value)));
        assert_eq!(copy_name(".jpg", "", "jpg"), "WindowsPhone-file.jpg");
    }
    #[test]
    fn unicode_names_and_untrusted_prefixes_remain_bounded() {
        let name = copy_name(
            &format!("{}.docx", "🪟".repeat(200)),
            &"../x".repeat(100),
            "docx",
        );
        assert!(name.len() <= 240);
        assert!(name.ends_with(".docx"));
        assert!(!name.contains('/'));
        assert_eq!(
            copy_name("CON", "WindowsPhone", "txt"),
            "WindowsPhone-CON.txt"
        );
    }
}
