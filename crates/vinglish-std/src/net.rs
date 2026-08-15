use vinglish_bindgen::vinglish_bindgen;
use reqwest::blocking::get;

#[vinglish_bindgen]
pub fn net_fetch(url: &str) -> String {
    match get(url) {
        Ok(response) => response.text().unwrap_or_else(|e| format!("Error parsing response: {}", e)),
        Err(e) => format!("Error fetching: {}", e),
    }
}
