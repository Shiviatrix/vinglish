#[cfg(test)]
mod tests {
    use strsim::{damerau_levenshtein, levenshtein, jaro_winkler};

    #[test]
    fn test_string_distances() {
        let s1 = "funtion";
        let s2 = "function";
        println!("damerau_levenshtein: {}", damerau_levenshtein(s1, s2));
        println!("levenshtein: {}", levenshtein(s1, s2));
        println!("jaro_winkler: {}", jaro_winkler(s1, s2));
        
        let s3 = "funcction"; // duplicated c
        println!("funcction -> function damerau_levenshtein: {}", damerau_levenshtein(s3, s2));
        
        let s4 = "fction"; // missing u
        println!("fction -> function damerau_levenshtein: {}", damerau_levenshtein(s4, s2));
        
        let s5 = "fuiton"; // transposed t and i
        println!("fuiton -> function damerau_levenshtein: {}", damerau_levenshtein(s5, s2));
        
        let s6 = "FUNCTION"; // wrong case
        println!("FUNCTION -> function damerau_levenshtein: {}", damerau_levenshtein(s6.to_lowercase(), s2));
        
        // Test confidence scoring
        let max_len = s2.len().max(s1.len()) as f64;
        let confidence = 1.0 - (damerau_levenshtein(s1, s2) as f64 / max_len);
        println!("confidence for funtion: {}", confidence);
    }
}
