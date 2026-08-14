use vinglish_bindgen::vinglish_bindgen;

#[vinglish_bindgen]
pub fn string_concat(a: &str, b: &str) -> String {
    format!("{}{}", a, b)
}

#[vinglish_bindgen]
pub fn string_len(a: &str) -> i64 {
    a.len() as i64
}
