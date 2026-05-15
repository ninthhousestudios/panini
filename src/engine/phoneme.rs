const VOWEL_PHONEMES: &[&str] = &[
    "ai", "au", "ā", "ī", "ū", "ṝ", "ṛ", "ḷ", "a", "i", "u", "e", "o",
];

const CONSONANT_DIGRAPHS: &[&str] = &[
    "kh", "gh", "ch", "jh", "ṭh", "ḍh", "th", "dh", "ph", "bh",
];

pub fn tokenize(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut remaining = s;
    while !remaining.is_empty() {
        let mut matched = false;
        for &phoneme in VOWEL_PHONEMES {
            if remaining.starts_with(phoneme) {
                result.push(&remaining[..phoneme.len()]);
                remaining = &remaining[phoneme.len()..];
                matched = true;
                break;
            }
        }
        if !matched {
            for &digraph in CONSONANT_DIGRAPHS {
                if remaining.starts_with(digraph) {
                    result.push(&remaining[..digraph.len()]);
                    remaining = &remaining[digraph.len()..];
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            let ch_len = remaining.chars().next().unwrap().len_utf8();
            result.push(&remaining[..ch_len]);
            remaining = &remaining[ch_len..];
        }
    }
    result
}

pub fn first_phoneme(s: &str) -> Option<&str> {
    tokenize(s).into_iter().next()
}

pub fn last_phoneme(s: &str) -> Option<&str> {
    tokenize(s).into_iter().last()
}

pub fn phoneme_ends_with(haystack: &str, needle: &str) -> bool {
    let h = tokenize(haystack);
    let n = tokenize(needle);
    h.len() >= n.len() && h[h.len() - n.len()..] == n[..]
}

pub fn phoneme_starts_with(haystack: &str, needle: &str) -> bool {
    let h = tokenize(haystack);
    let n = tokenize(needle);
    h.len() >= n.len() && h[..n.len()] == n[..]
}

pub fn phoneme_strip_suffix<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    if phoneme_ends_with(s, suffix) {
        Some(&s[..s.len() - suffix.len()])
    } else {
        None
    }
}

pub fn phoneme_strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if phoneme_starts_with(s, prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_vowels() {
        assert_eq!(tokenize("a"), vec!["a"]);
        assert_eq!(tokenize("ai"), vec!["ai"]);
        assert_eq!(tokenize("au"), vec!["au"]);
        assert_eq!(tokenize("ā"), vec!["ā"]);
    }

    #[test]
    fn tokenize_mixed() {
        assert_eq!(tokenize("mahā"), vec!["m", "a", "h", "ā"]);
        assert_eq!(tokenize("vai"), vec!["v", "ai"]);
    }

    #[test]
    fn diphthong_not_confused_with_simple() {
        assert!(!phoneme_starts_with("ai", "a"));
        assert!(!phoneme_starts_with("au", "a"));
        assert!(phoneme_starts_with("ai", "ai"));
        assert!(phoneme_starts_with("ā", "ā"));
        assert!(phoneme_starts_with("a", "a"));
    }

    #[test]
    fn ends_with_phoneme_boundary() {
        assert!(phoneme_ends_with("mahā", "ā"));
        assert!(!phoneme_ends_with("mai", "i"));
        assert!(phoneme_ends_with("mai", "ai"));
    }

    #[test]
    fn strip_functions() {
        assert_eq!(phoneme_strip_suffix("mahā", "ā"), Some("mah"));
        assert_eq!(phoneme_strip_suffix("mai", "i"), None);
        assert_eq!(phoneme_strip_prefix("ai", "a"), None);
        assert_eq!(phoneme_strip_prefix("ai", "ai"), Some(""));
    }

    #[test]
    fn tokenize_aspirates() {
        assert_eq!(tokenize("bhyām"), vec!["bh", "y", "ā", "m"]);
        assert_eq!(tokenize("dha"), vec!["dh", "a"]);
        assert_eq!(tokenize("kha"), vec!["kh", "a"]);
    }

    #[test]
    fn first_last_phoneme() {
        assert_eq!(first_phoneme("bhyām"), Some("bh"));
        assert_eq!(last_phoneme("devaḥ"), Some("ḥ"));
        assert_eq!(first_phoneme("ais"), Some("ai"));
        assert_eq!(last_phoneme(""), None);
        assert_eq!(first_phoneme(""), None);
    }
}
