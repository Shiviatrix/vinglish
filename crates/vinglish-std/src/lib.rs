pub mod ai;
pub mod db;
pub mod fuzzy;
pub mod io;
pub mod math;
pub mod net;
pub mod regex;
pub mod scrape;
pub mod sys;

use vinglish_bindgen::vinglish_bindgen;

#[vinglish_bindgen]
pub fn string_concat(a: &str, b: &str) -> String {
    format!("{}{}", a, b)
}

#[vinglish_bindgen]
pub fn string_len(a: &str) -> i64 {
    a.len() as i64
}
