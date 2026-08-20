use crate::pipeline::PipelineResult;
use crate::token::Token;

struct PorterStemmer {
    step2list: Vec<(String, String)>,
    step3list: Vec<(String, String)>,
}

impl PorterStemmer {
    fn new() -> Self {
        let step2list = vec![
            ("ational".to_string(), "ate".to_string()),
            ("tional".to_string(), "tion".to_string()),
            ("enci".to_string(), "ence".to_string()),
            ("anci".to_string(), "ance".to_string()),
            ("izer".to_string(), "ize".to_string()),
            ("bli".to_string(), "ble".to_string()),
            ("alli".to_string(), "al".to_string()),
            ("entli".to_string(), "ent".to_string()),
            ("eli".to_string(), "e".to_string()),
            ("ousli".to_string(), "ous".to_string()),
            ("ization".to_string(), "ize".to_string()),
            ("ation".to_string(), "ate".to_string()),
            ("ator".to_string(), "ate".to_string()),
            ("alism".to_string(), "al".to_string()),
            ("iveness".to_string(), "ive".to_string()),
            ("fulness".to_string(), "ful".to_string()),
            ("ousness".to_string(), "ous".to_string()),
            ("aliti".to_string(), "al".to_string()),
            ("iviti".to_string(), "ive".to_string()),
            ("biliti".to_string(), "ble".to_string()),
            ("logi".to_string(), "log".to_string()),
        ];

        let step3list = vec![
            ("icate".to_string(), "ic".to_string()),
            ("ative".to_string(), "".to_string()),
            ("alize".to_string(), "al".to_string()),
            ("iciti".to_string(), "ic".to_string()),
            ("ical".to_string(), "ic".to_string()),
            ("ful".to_string(), "".to_string()),
            ("ness".to_string(), "".to_string()),
        ];

        PorterStemmer { step2list, step3list }
    }

    fn stem(&self, w: &str) -> String {
        if w.len() < 3 {
            return w.to_string();
        }

        let firstch = w.chars().next().unwrap();
        let mut w: Vec<char> = w.chars().collect();

        if firstch == 'y' {
            w[0] = firstch.to_uppercase().next().unwrap();
        }

        let w_str: String = w.iter().collect();
        let w_str = self.step1a(&w_str);
        let w_str = self.step1b(&w_str);
        let w_str = self.step1c(&w_str);
        let w_str = self.step2(&w_str);
        let w_str = self.step3(&w_str);
        let w_str = self.step4(&w_str);
        let w_str = self.step5(&w_str);

        let mut result: Vec<char> = w_str.chars().collect();
        if firstch == 'y' {
            result[0] = firstch.to_lowercase().next().unwrap();
        }

        result.iter().collect()
    }

    fn step1a(&self, w: &str) -> String {
        if w.ends_with("ies") && w.len() > 4 {
            let mut result = w[..w.len()-2].to_string();
            result.push('s');
            result
        } else if w.ends_with("sses") {
            let mut result = w[..w.len()-2].to_string();
            result.push('s');
            result
        } else if w.ends_with("ss") {
            w.to_string()
        } else if w.ends_with('s') && !w.ends_with("ss") && w.len() > 2 {
            w[..w.len()-1].to_string()
        } else {
            w.to_string()
        }
    }

    fn step1b(&self, w: &str) -> String {
        if w.ends_with("eed") {
            let stem = &w[..w.len()-3];
            if self.m(stem) > 0 {
                let mut result = stem.to_string();
                result.push_str("ee");
                result
            } else {
                w.to_string()
            }
        } else if w.ends_with("ed") || w.ends_with("ing") {
            let suffix_len = if w.ends_with("ed") { 2 } else { 3 };
            let stem = &w[..w.len()-suffix_len];
            if self.contains_vowel(stem) {
                let stem = if stem.ends_with('y') && !self.contains_vowel(&stem[..stem.len()-1]) {
                    format!("{}i", &stem[..stem.len()-1])
                } else if stem.ends_with("at") || stem.ends_with('b') || stem.ends_with('l') || stem.ends_with("iz") {
                    format!("{}e", stem)
                } else if stem.len() >= 2 {
                    let last_chars: Vec<char> = stem.chars().rev().take(2).collect();
                    if last_chars.len() == 2 && last_chars[0] == last_chars[1]
                        && !last_chars[0].eq_ignore_ascii_case(&'s')
                        && !last_chars[0].eq_ignore_ascii_case(&'l')
                        && !last_chars[0].eq_ignore_ascii_case(&'z')
                        && !last_chars[0].eq_ignore_ascii_case(&'a')
                        && !last_chars[0].eq_ignore_ascii_case(&'e')
                        && !last_chars[0].eq_ignore_ascii_case(&'i')
                        && !last_chars[0].eq_ignore_ascii_case(&'o')
                        && !last_chars[0].eq_ignore_ascii_case(&'u')
                    {
                        stem[..stem.len()-1].to_string()
                    } else if stem.len() >= 3 {
                        let chars: Vec<char> = stem.chars().collect();
                        let n = chars.len();
                        if !self.is_vowel(chars[n-1])
                            && self.is_vowel(chars[n-2])
                            && !self.is_vowel(chars[n-3])
                            && chars[n-1] != 'w'
                            && chars[n-1] != 'x'
                            && chars[n-1] != 'y'
                        {
                            format!("{}e", stem)
                        } else {
                            stem.to_string()
                        }
                    } else {
                        stem.to_string()
                    }
                } else {
                    stem.to_string()
                };
                stem
            } else {
                w.to_string()
            }
        } else {
            w.to_string()
        }
    }

    fn step1c(&self, w: &str) -> String {
        if w.ends_with('y') && w.len() > 2 {
            let stem = &w[..w.len()-1];
            if self.contains_vowel(stem) {
                format!("{}i", stem)
            } else {
                w.to_string()
            }
        } else {
            w.to_string()
        }
    }

    fn step2(&self, w: &str) -> String {
        for (suffix, replacement) in &self.step2list {
            if w.ends_with(suffix.as_str()) {
                let stem = &w[..w.len()-suffix.len()];
                if self.m(stem) > 0 {
                    return format!("{}{}", stem, replacement);
                }
            }
        }
        w.to_string()
    }

    fn step3(&self, w: &str) -> String {
        for (suffix, replacement) in &self.step3list {
            if w.ends_with(suffix.as_str()) {
                let stem = &w[..w.len()-suffix.len()];
                if self.m(stem) > 0 {
                    return format!("{}{}", stem, replacement);
                }
            }
        }
        w.to_string()
    }

    fn step4(&self, w: &str) -> String {
        let suffixes = ["al", "ance", "ence", "er", "ic", "able", "ible", "ant", "ement", "ment", "ent", "ou", "ism", "ate", "iti", "ous", "ive", "ize"];
        for suffix in &suffixes {
            if w.ends_with(suffix) {
                let stem = &w[..w.len()-suffix.len()];
                if self.m(stem) > 1 {
                    return stem.to_string();
                }
            }
        }
        if (w.ends_with("sion") || w.ends_with("tion")) && w.len() > 4 {
            let stem = &w[..w.len()-3];
            if self.m(stem) > 1 {
                return stem.to_string();
            }
        }
        w.to_string()
    }

    fn step5(&self, w: &str) -> String {
        if w.ends_with('e') && w.len() > 2 {
            let stem = &w[..w.len()-1];
            if self.m(stem) > 1 || (self.m(stem) == 1 && !self.ends_cvc(stem)) {
                return stem.to_string();
            }
        }
        if w.ends_with("ll") && self.m(w) > 1 {
            return w[..w.len()-1].to_string();
        }
        w.to_string()
    }

    fn is_vowel(&self, c: char) -> bool {
        matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    }

    fn contains_vowel(&self, w: &str) -> bool {
        w.chars().any(|c| self.is_vowel(c) && c != 'y')
    }

    fn m(&self, w: &str) -> usize {
        let mut m = 0;
        let mut i = 0;
        let chars: Vec<char> = w.chars().collect();
        let n = chars.len();

        while i < n && !self.is_vowel(chars[i]) {
            i += 1;
        }

        while i < n {
            while i < n && self.is_vowel(chars[i]) {
                i += 1;
            }
            if i >= n {
                break;
            }
            while i < n && !self.is_vowel(chars[i]) {
                i += 1;
            }
            m += 1;
        }

        m
    }

    fn ends_cvc(&self, w: &str) -> bool {
        let chars: Vec<char> = w.chars().collect();
        let n = chars.len();
        if n < 3 {
            return false;
        }
        let c = chars[n-1];
        let v = chars[n-2];
        let c2 = chars[n-3];

        !self.is_vowel(c)
            && self.is_vowel(v)
            && !self.is_vowel(c2)
            && !matches!(c, 'w' | 'x' | 'y')
    }
}

pub fn stemmer(token: Token) -> PipelineResult {
    let stemmer = PorterStemmer::new();
    let stemmed = stemmer.stem(&token.term);

    if stemmed.is_empty() {
        PipelineResult::None
    } else {
        PipelineResult::Token(Token { term: stemmed, metadata: token.metadata })
    }
}
