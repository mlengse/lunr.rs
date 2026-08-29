use crate::pipeline::PipelineResult;
use crate::token::Token;

const STEP2LIST: &[(&str, &str)] = &[
    ("ational", "ate"),
    ("tional", "tion"),
    ("enci", "ence"),
    ("anci", "ance"),
    ("izer", "ize"),
    ("bli", "ble"),
    ("alli", "al"),
    ("entli", "ent"),
    ("eli", "e"),
    ("ousli", "ous"),
    ("ization", "ize"),
    ("ation", "ate"),
    ("ator", "ate"),
    ("alism", "al"),
    ("iveness", "ive"),
    ("fulness", "ful"),
    ("ousness", "ous"),
    ("aliti", "al"),
    ("iviti", "ive"),
    ("biliti", "ble"),
    ("logi", "log"),
];

const STEP3LIST: &[(&str, &str)] = &[
    ("icate", "ic"),
    ("ative", ""),
    ("alize", "al"),
    ("iciti", "ic"),
    ("ical", "ic"),
    ("ful", ""),
    ("ness", ""),
];

struct PorterStemmer;

#[inline(always)]
fn is_vowel(b: u8) -> bool {
    matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
}

impl PorterStemmer {
    fn new() -> Self {
        PorterStemmer
    }

    fn stem(&self, w: &str) -> String {
        if w.len() < 3 {
            return w.to_string();
        }

        let bytes = w.as_bytes();
        let firstch = bytes[0];
        let mut w: Vec<u8> = bytes.to_vec();

        if firstch == b'y' {
            w[0] = b'Y';
        }

        let mut w = self.step1a(&w);
        w = self.step1b(&w);
        w = self.step1c(&w);
        w = self.step2(&w);
        w = self.step3(&w);
        w = self.step4(&w);
        w = self.step5(&w);

        if firstch == b'y' && !w.is_empty() {
            w[0] = b'y';
        }

        // SAFETY-adjacent (safe): the working word is always ASCII letters, so a
        // UTF-8 byte vector is always valid UTF-8.
        String::from_utf8(w).unwrap()
    }

    fn step1a(&self, w: &[u8]) -> Vec<u8> {
        if w.ends_with(b"ies") && w.len() > 4 {
            let mut result = w[..w.len() - 2].to_vec();
            result.push(b's');
            result
        } else if w.ends_with(b"sses") {
            let mut result = w[..w.len() - 2].to_vec();
            result.push(b's');
            result
        } else if w.ends_with(b"ss") {
            w.to_vec()
        } else if w.len() > 2 && w.ends_with(b"s") {
            w[..w.len() - 1].to_vec()
        } else {
            w.to_vec()
        }
    }

    fn step1b(&self, w: &[u8]) -> Vec<u8> {
        if w.ends_with(b"eed") {
            let stem = &w[..w.len() - 3];
            if self.m(stem) > 0 {
                let mut result = stem.to_vec();
                result.extend_from_slice(b"ee");
                result
            } else {
                w.to_vec()
            }
        } else if w.ends_with(b"ed") || w.ends_with(b"ing") {
            let suffix_len = if w.ends_with(b"ed") { 2 } else { 3 };
            let stem = &w[..w.len() - suffix_len];
            if self.contains_vowel(stem) {
                let stem = if stem.ends_with(b"y")
                    && !self.contains_vowel(&stem[..stem.len() - 1])
                {
                    let mut v = stem[..stem.len() - 1].to_vec();
                    v.push(b'i');
                    v
                } else if stem.ends_with(b"at")
                    || stem.ends_with(b"b")
                    || stem.ends_with(b"l")
                    || stem.ends_with(b"iz")
                {
                    let mut v = stem.to_vec();
                    v.push(b'e');
                    v
                } else if stem.len() >= 2
                    && stem[stem.len() - 1] == stem[stem.len() - 2]
                    && !matches!(
                        stem[stem.len() - 1],
                        b's' | b'l' | b'z' | b'a' | b'e' | b'i' | b'o' | b'u'
                    )
                {
                    stem[..stem.len() - 1].to_vec()
                } else if stem.len() >= 3 {
                    let n = stem.len();
                    if !self.is_vowel(stem[n - 1])
                        && self.is_vowel(stem[n - 2])
                        && !self.is_vowel(stem[n - 3])
                        && !matches!(stem[n - 1], b'w' | b'x' | b'y')
                    {
                        let mut v = stem.to_vec();
                        v.push(b'e');
                        v
                    } else {
                        stem.to_vec()
                    }
                } else {
                    stem.to_vec()
                };
                stem
            } else {
                w.to_vec()
            }
        } else {
            w.to_vec()
        }
    }

    fn step1c(&self, w: &[u8]) -> Vec<u8> {
        if w.len() > 2 && w.ends_with(b"y") {
            let before_y = w[w.len() - 2];
            if !matches!(before_y, b'a' | b'e' | b'i' | b'o' | b'u') {
                let mut result = w[..w.len() - 1].to_vec();
                result.push(b'i');
                result
            } else {
                w.to_vec()
            }
        } else {
            w.to_vec()
        }
    }

    fn step2(&self, w: &[u8]) -> Vec<u8> {
        for (suffix, replacement) in STEP2LIST {
            let suffix = suffix.as_bytes();
            if w.ends_with(suffix) {
                let stem = &w[..w.len() - suffix.len()];
                if self.m(stem) > 0 {
                    let mut result = stem.to_vec();
                    result.extend_from_slice(replacement.as_bytes());
                    return result;
                }
            }
        }
        w.to_vec()
    }

    fn step3(&self, w: &[u8]) -> Vec<u8> {
        for (suffix, replacement) in STEP3LIST {
            let suffix = suffix.as_bytes();
            if w.ends_with(suffix) {
                let stem = &w[..w.len() - suffix.len()];
                if self.m(stem) > 0 {
                    let mut result = stem.to_vec();
                    result.extend_from_slice(replacement.as_bytes());
                    return result;
                }
            }
        }
        w.to_vec()
    }

    fn step4(&self, w: &[u8]) -> Vec<u8> {
        let suffixes: &[&[u8]] = &[
            b"al", b"ance", b"ence", b"er", b"ic", b"able", b"ible", b"ant", b"ement", b"ment",
            b"ent", b"ou", b"ism", b"ate", b"iti", b"ous", b"ive", b"ize",
        ];
        for suffix in suffixes {
            if w.ends_with(suffix) {
                let stem = &w[..w.len() - suffix.len()];
                if self.m(stem) > 1 {
                    return stem.to_vec();
                }
            }
        }
        if (w.ends_with(b"sion") || w.ends_with(b"tion")) && w.len() > 4 {
            let stem = &w[..w.len() - 3];
            if self.m(stem) > 1 {
                return stem.to_vec();
            }
        }
        w.to_vec()
    }

    fn step5(&self, w: &[u8]) -> Vec<u8> {
        if w.len() > 2 && w.ends_with(b"e") {
            let stem = &w[..w.len() - 1];
            if self.m(stem) > 1 || (self.m(stem) == 1 && !self.ends_cvc(stem)) {
                return stem.to_vec();
            }
        }
        if w.len() >= 2 && w.ends_with(b"ll") && self.m(w) > 1 {
            return w[..w.len() - 1].to_vec();
        }
        w.to_vec()
    }

    #[inline(always)]
    fn is_vowel(&self, b: u8) -> bool {
        is_vowel(b)
    }

    fn contains_vowel(&self, w: &[u8]) -> bool {
        w.iter().any(|&c| matches!(c, b'a' | b'e' | b'i' | b'o' | b'u'))
    }

    fn m(&self, w: &[u8]) -> usize {
        let mut m = 0;
        let mut i = 0;
        let n = w.len();

        while i < n && !self.is_vowel(w[i]) {
            i += 1;
        }

        while i < n {
            while i < n && self.is_vowel(w[i]) {
                i += 1;
            }
            if i >= n {
                break;
            }
            while i < n && !self.is_vowel(w[i]) {
                i += 1;
            }
            m += 1;
        }

        m
    }

    fn ends_cvc(&self, w: &[u8]) -> bool {
        let n = w.len();
        if n < 3 {
            return false;
        }
        let c = w[n - 1];
        let v = w[n - 2];
        let c2 = w[n - 3];

        !self.is_vowel(c)
            && self.is_vowel(v)
            && !self.is_vowel(c2)
            && !matches!(c, b'w' | b'x' | b'y')
    }
}

pub fn stemmer(token: Token) -> PipelineResult {
    let stemmer = PorterStemmer::new();
    let stemmed = stemmer.stem(&token.term);

    if stemmed.is_empty() {
        PipelineResult::None
    } else {
        PipelineResult::Token(Token {
            term: stemmed,
            metadata: token.metadata,
        })
    }
}
