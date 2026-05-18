use super::phoneme::tokenize;

struct ScriptMap {
    iast_to_akshara: &'static [(&'static str, &'static str)],
    vowel_signs: &'static [(&'static str, &'static str)],
    consonants: &'static [&'static str],
    virama: &'static str,
}

const DEVA: ScriptMap = ScriptMap {
    iast_to_akshara: &[
        ("ai", "ऐ"), ("au", "औ"),
        ("kh", "ख"), ("gh", "घ"), ("ch", "छ"), ("jh", "झ"),
        ("ṭh", "ठ"), ("ḍh", "ढ"), ("th", "थ"), ("dh", "ध"),
        ("ph", "फ"), ("bh", "भ"),
        ("ṅ", "ङ"), ("ñ", "ञ"), ("ṇ", "ण"), ("ṃ", "ं"), ("ḥ", "ः"),
        ("ā", "आ"), ("ī", "ई"), ("ū", "ऊ"), ("ṝ", "ॠ"), ("ṛ", "ऋ"), ("ḷ", "ऌ"),
        ("a", "अ"), ("i", "इ"), ("u", "उ"), ("e", "ए"), ("o", "ओ"),
        ("k", "क"), ("g", "ग"), ("c", "च"), ("j", "ज"),
        ("ṭ", "ट"), ("ḍ", "ड"), ("t", "त"), ("d", "द"), ("n", "न"),
        ("p", "प"), ("b", "ब"), ("m", "म"),
        ("y", "य"), ("r", "र"), ("l", "ल"), ("v", "व"),
        ("ś", "श"), ("ṣ", "ष"), ("s", "स"), ("h", "ह"),
    ],
    vowel_signs: &[
        ("आ", "ा"), ("इ", "ि"), ("ई", "ी"), ("उ", "ु"), ("ऊ", "ू"),
        ("ऋ", "ृ"), ("ॠ", "ॄ"), ("ऌ", "ॢ"),
        ("ए", "े"), ("ऐ", "ै"), ("ओ", "ो"), ("औ", "ौ"),
    ],
    consonants: &[
        "क", "ख", "ग", "घ", "ङ", "च", "छ", "ज", "झ", "ञ",
        "ट", "ठ", "ड", "ढ", "ण", "त", "थ", "द", "ध", "न",
        "प", "फ", "ब", "भ", "म", "य", "र", "ल", "व", "श", "ष", "स", "ह",
    ],
    virama: "्",
};

fn is_vowel(akshara: &str, map: &ScriptMap) -> bool {
    map.iast_to_akshara.iter().any(|(iast, ak)| {
        *ak == akshara && is_iast_vowel(iast)
    })
}

fn is_iast_vowel(token: &str) -> bool {
    matches!(
        token,
        "a" | "ā" | "i" | "ī" | "u" | "ū" | "ṛ" | "ṝ" | "ḷ" | "e" | "ai" | "o" | "au"
    )
}

fn vowel_sign(akshara: &str, map: &ScriptMap) -> Option<&'static str> {
    map.vowel_signs.iter().find(|(ind, _)| *ind == akshara).map(|(_, sign)| *sign)
}

fn is_consonant(akshara: &str, map: &ScriptMap) -> bool {
    map.consonants.contains(&akshara)
}

fn iast_to_akshara<'a>(token: &str, map: &'a ScriptMap) -> Option<&'a str> {
    map.iast_to_akshara
        .iter()
        .find(|(iast, _)| *iast == token)
        .map(|(_, ak)| *ak)
}

fn transliterate(iast: &str, map: &ScriptMap) -> String {
    let lowered = iast.to_lowercase();
    let tokens = tokenize(&lowered);
    let a = iast_to_akshara("a", map).unwrap();
    let aksharas: Vec<&str> = tokens
        .iter()
        .map(|t| iast_to_akshara(t, map).unwrap_or(t))
        .collect();

    let mut result = String::new();
    let mut i = 0;
    while i < aksharas.len() {
        let ak = aksharas[i];
        if is_consonant(ak, map) {
            if let Some(&next) = aksharas.get(i + 1) {
                if is_vowel(next, map) {
                    if next == a {
                        result.push_str(ak);
                    } else if let Some(sign) = vowel_sign(next, map) {
                        result.push_str(ak);
                        result.push_str(sign);
                    } else {
                        result.push_str(ak);
                        result.push_str(next);
                    }
                    i += 2;
                    continue;
                }
            }
            result.push_str(ak);
            result.push_str(map.virama);
        } else {
            result.push_str(ak);
        }
        i += 1;
    }
    result
}

pub fn to_devanagari(iast: &str) -> String {
    if iast.is_empty() {
        return String::new();
    }
    transliterate(iast, &DEVA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_words() {
        assert_eq!(to_devanagari("deva"), "देव");
        assert_eq!(to_devanagari("devaḥ"), "देवः");
        assert_eq!(to_devanagari("dharmaḥ"), "धर्मः");
    }

    #[test]
    fn conjuncts() {
        assert_eq!(to_devanagari("kṣatriyaḥ"), "क्षत्रियः");
        assert_eq!(to_devanagari("jña"), "ज्ञ");
        assert_eq!(to_devanagari("śra"), "श्र");
    }

    #[test]
    fn vowels_standalone() {
        assert_eq!(to_devanagari("a"), "अ");
        assert_eq!(to_devanagari("ā"), "आ");
        assert_eq!(to_devanagari("i"), "इ");
        assert_eq!(to_devanagari("ī"), "ई");
        assert_eq!(to_devanagari("u"), "उ");
        assert_eq!(to_devanagari("ū"), "ऊ");
        assert_eq!(to_devanagari("ṛ"), "ऋ");
        assert_eq!(to_devanagari("e"), "ए");
        assert_eq!(to_devanagari("ai"), "ऐ");
        assert_eq!(to_devanagari("o"), "ओ");
        assert_eq!(to_devanagari("au"), "औ");
    }

    #[test]
    fn vowel_signs() {
        assert_eq!(to_devanagari("kā"), "का");
        assert_eq!(to_devanagari("ki"), "कि");
        assert_eq!(to_devanagari("ku"), "कु");
        assert_eq!(to_devanagari("ke"), "के");
        assert_eq!(to_devanagari("kai"), "कै");
        assert_eq!(to_devanagari("ko"), "को");
        assert_eq!(to_devanagari("kau"), "कौ");
        assert_eq!(to_devanagari("kṛ"), "कृ");
    }

    #[test]
    fn anusvara_visarga() {
        assert_eq!(to_devanagari("saṃskṛtam"), "संस्कृतम्");
        assert_eq!(to_devanagari("deveṣu"), "देवेषु");
        assert_eq!(to_devanagari("ṛṣiḥ"), "ऋषिः");
    }

    #[test]
    fn full_diacritics() {
        assert_eq!(to_devanagari("ṃ"), "ं");
        assert_eq!(to_devanagari("ḥ"), "ः");
        assert_eq!(to_devanagari("ṅ"), "ङ्");
        assert_eq!(to_devanagari("ñ"), "ञ्");
        assert_eq!(to_devanagari("ś"), "श्");
        assert_eq!(to_devanagari("ṣ"), "ष्");
        assert_eq!(to_devanagari("ṭ"), "ट्");
        assert_eq!(to_devanagari("ḍ"), "ड्");
        assert_eq!(to_devanagari("ṇ"), "ण्");
    }

    #[test]
    fn aspirates() {
        assert_eq!(to_devanagari("kha"), "ख");
        assert_eq!(to_devanagari("gha"), "घ");
        assert_eq!(to_devanagari("cha"), "छ");
        assert_eq!(to_devanagari("jha"), "झ");
        assert_eq!(to_devanagari("ṭha"), "ठ");
        assert_eq!(to_devanagari("ḍha"), "ढ");
        assert_eq!(to_devanagari("tha"), "थ");
        assert_eq!(to_devanagari("dha"), "ध");
        assert_eq!(to_devanagari("pha"), "फ");
        assert_eq!(to_devanagari("bha"), "भ");
    }

    #[test]
    fn paradigm_forms() {
        assert_eq!(to_devanagari("devam"), "देवम्");
        assert_eq!(to_devanagari("devena"), "देवेन");
        assert_eq!(to_devanagari("devāya"), "देवाय");
        assert_eq!(to_devanagari("devāt"), "देवात्");
        assert_eq!(to_devanagari("devasya"), "देवस्य");
        assert_eq!(to_devanagari("deve"), "देवे");
    }

    #[test]
    fn empty() {
        assert_eq!(to_devanagari(""), "");
    }

    #[test]
    fn passthrough_non_iast() {
        assert_eq!(to_devanagari("123"), "123");
        assert_eq!(to_devanagari("deva 123"), "देव 123");
    }
}
