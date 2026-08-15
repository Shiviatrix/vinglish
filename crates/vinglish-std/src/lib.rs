pub mod math;
pub mod io;
pub mod net;
pub mod sys;
pub mod regex;
pub mod db;
pub mod scrape;
pub mod ai;
pub mod fuzzy;

use vinglish_bindgen::vinglish_bindgen;

#[vinglish_bindgen]
pub fn string_concat(a: &str, b: &str) -> String {
    format!("{}{}", a, b)
}

#[vinglish_bindgen]
pub fn string_len(a: &str) -> i64 {
    a.len() as i64
}
