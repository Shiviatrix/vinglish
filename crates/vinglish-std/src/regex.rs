use vinglish_bindgen::vinglish_bindgen;
use regex::Regex;

#[vinglish_bindgen]
pub fn regex_is_match(pattern: &str, text: &str) -> bool {
    if let Ok(re) = Regex::new(pattern) {
        re.is_match(text)
    } else {
        false
    }
}

#[vinglish_bindgen]
pub fn regex_replace(pattern: &str, text: &str, replacement: &str) -> String {
    if let Ok(re) = Regex::new(pattern) {
        re.replace_all(text, replacement).to_string()
    } else {
        text.to_string()
    }
}
