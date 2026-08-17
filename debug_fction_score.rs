use strsim::jaro_winkler;

fn similarity_score(s1: &str, s2: &str) -> f64 {
    let distance = jaro_winkler(s1, s2); // Using original algorithm for comparison
    distance
}

fn main() {
    let s1 = "fction";
    let s2 = "function";
    let score = similarity_score(s1, s2);
    println!("Similarity score for '{}' -> '{}': {}", s1, s2, score);
    println!("Is score > 0.8? {}", score > 0.8);
    println!("Is score > 0.85? {}", score > 0.85);
    
    // Test with our new algorithm
    use strsim::damerau_levenshtein;
    let distance = damerau_levenshtein(s1, s2);
    let max_len = s1.len().max(s2.len()) as f64;
    let mut similarity = 1.0 - (distance as f64 / max_len);
    if s1.eq_ignore_ascii_case(s2) && s1.len() == s2.len() {
        similarity = similarity.max(0.95);
    }
    println!("Damerau-based similarity: {}", similarity);
    println!("Is damerau-based > 0.8? {}", similarity > 0.8);
}
