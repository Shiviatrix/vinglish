use vinglish_bindgen::vinglish_bindgen;
use scraper::{Html, Selector};
use reqwest::blocking::get;
use readability::extractor;
use url::Url;

#[vinglish_bindgen]
pub fn scrape_url(url: &str, css_selector: &str) -> String {
    let html = match get(url) {
        Ok(res) => res.text().unwrap_or_default(),
        Err(_) => return String::new(),
    };
    
    let document = Html::parse_document(&html);
    let selector = match Selector::parse(css_selector) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    
    let mut results = Vec::new();
    for element in document.select(&selector) {
        let text = element.text().collect::<Vec<_>>().join(" ");
        results.push(text.trim().to_string());
    }
    
    results.join("\n")
}

#[vinglish_bindgen]
pub fn scrape_attr(url: &str, css_selector: &str, attr: &str) -> String {
    let html = match get(url) {
        Ok(res) => res.text().unwrap_or_default(),
        Err(_) => return String::new(),
    };
    
    let document = Html::parse_document(&html);
    let selector = match Selector::parse(css_selector) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    
    let mut results = Vec::new();
    for element in document.select(&selector) {
        if let Some(val) = element.value().attr(attr) {
            results.push(val.to_string());
        }
    }
    
    results.join("\n")
}

#[vinglish_bindgen]
pub fn scrape_article_text(url: &str) -> String {
    use std::io::Read;
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return String::new(),
    };
    
    let mut html = String::new();
    if let Ok(mut res) = get(url) {
        if res.read_to_string(&mut html).is_err() {
            return String::new();
        }
    } else {
        return String::new();
    }
    
    let mut cursor = std::io::Cursor::new(html);
    match extractor::extract(&mut cursor, &parsed_url) {
        Ok(product) => product.text,
        Err(_) => String::new(),
    }
}
