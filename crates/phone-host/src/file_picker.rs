//! Common admission rules for browser and native SDK file pickers.
pub const MAX_BYTES: u32 = 16 * 1024 * 1024;

pub fn validate_options(extensions: &[String], max_bytes: u32) -> Result<(), String> {
    if max_bytes == 0 || max_bytes > MAX_BYTES {
        return Err("Choose a file size limit between 1 byte and 16 MiB.".into());
    }
    if extensions.len() > 16
        || extensions.iter().any(|extension| {
            let suffix = extension.strip_prefix('.').unwrap_or_default();
            suffix.is_empty()
                || suffix.len() > 16
                || !suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
    {
        return Err("Choose at most 16 lowercase file extensions, such as .txt or .csv.".into());
    }
    Ok(())
}

/// Picker filters guide the UI; they are not a guarantee that a provider has
/// returned an admitted type. Check the actual leaf name again before reading.
pub fn accepts_name(extensions: &[String], name: &str) -> bool {
    extensions.is_empty()
        || extensions.iter().any(|extension| {
            name.get(name.len().saturating_sub(extension.len())..)
                .is_some_and(|suffix| suffix.eq_ignore_ascii_case(extension))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picker_limits_reject_patterns_mime_strings_and_oversized_reads() {
        for extension in [
            "txt",
            ".TXT",
            ".",
            ".a/b",
            ".tar.gz",
            "image/png",
            ".*",
            ".é",
        ] {
            assert!(
                validate_options(&[extension.into()], MAX_BYTES).is_err(),
                "{extension}"
            );
        }
        assert!(validate_options(&vec![".txt".into(); 17], 1).is_err());
        assert!(validate_options(&[], 0).is_err());
        assert!(validate_options(&[], MAX_BYTES + 1).is_err());
        assert!(validate_options(&[".mp3".into()], MAX_BYTES).is_ok());
    }
    #[test]
    fn selected_suffix_is_checked_after_native_filtering() {
        let extensions = vec![".txt".into()];
        assert!(accepts_name(&extensions, "notes.TXT"));
        assert!(!accepts_name(&extensions, "notes.txt.exe"));
        assert!(!accepts_name(&extensions, "é"));
        assert!(accepts_name(&[], "any.file"));
    }
}
