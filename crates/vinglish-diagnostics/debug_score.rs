extern crate strsim;
use strsim::damerau_levenshtein;

fn similarity_score(s1: &str, s2: &str) -> f64 {
    let distance = damerau_levenshtein(s1, s2);
    let max_len = s1.len().max(s2.len()) as f64;
    
    if max_len == 0.0 {
        return 1.0;
    }
    
    let similarity = 1.0 - (distance as f64 / max_len);
    
    if s1.eq_ignore_ascii_case(s2) && s1.len() == s2.len() {
        return similarity.max(0.95);
    }
    
    similarity
}

fn main() {
    let s1 = "funtion";
    let s2 = "function";
    let distance = damerau_levenshtein(s1, s2);
    let score = similarity_score(s1, s2);
    println!("distance: {}", distance);
    println!("score: {}", score);
    println!("score > 0.8: {}", score > 0.8);
}
