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
    let score = similarity_score("funtion", "function");
    println!("Score: {}", score);
    println!("Result: {}", score > 0.8);
    
    let score2 = similarity_score("fction", "function");
    println!("Score2: {}", score2);
    println!("Result2: {}", score2 > 0.7);
    
    let score3 = similarity_score("funcction", "function");
    println!("Score3: {}", score3);
    println!("Result3: {}", score3 > 0.7);
    
    let score4 = similarity_score("FUNCTION", "function");
    println!("Score4: {}", score4);
    println!("Result4: {}", score4 > 0.9);
}
