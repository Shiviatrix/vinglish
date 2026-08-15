use vinglish_bindgen::vinglish_bindgen;
use strsim::{jaro_winkler, levenshtein};

#[vinglish_bindgen]
pub fn fuzzy_match_score(target: &str, query: &str) -> i64 {
    let score = jaro_winkler(target, query);
    (score * 100.0) as i64
}

#[vinglish_bindgen]
pub fn fuzzy_levenshtein(target: &str, query: &str) -> i64 {
    levenshtein(target, query) as i64
}

#[vinglish_bindgen]
pub fn fuzzy_find_best(query: &str, options_comma_separated: &str) -> String {
    let mut best_match = "";
    let mut best_score = 0.0;
    
    for option in options_comma_separated.split(',') {
        let option = option.trim();
        if option.is_empty() { continue; }
        
        let score = jaro_winkler(option, query);
        if score > best_score {
            best_score = score;
            best_match = option;
        }
    }
    
    best_match.to_string()
}
