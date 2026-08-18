use std::fs;
use vinglish_bindgen::vinglish_bindgen;

#[vinglish_bindgen]
pub fn io_read_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {}", e))
}

#[vinglish_bindgen]
pub fn io_write_file(path: &str, content: &str) -> bool {
    fs::write(path, content).is_ok()
}

#[vinglish_bindgen]
pub fn io_print(content: &str) {
    println!("{}", content);
}
