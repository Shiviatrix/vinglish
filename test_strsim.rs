extern crate strsim;
use strsim::{levenshtein, jaro_winkler};

fn main() {
    let s1 = "funtion";
    let s2 = "function";
    println!("levenshtein: {}", levenshtein(s1, s2));
    println!("jaro_winkler: {}", jaro_winkler(s1, s2));
    
    let s3 = "funcction"; // duplicated c
    println!("funcction -> function levenshtein: {}", levenshtein(s3, s2));
    
    let s4 = "fction"; // missing u
    println!("fction -> function levenshtein: {}", levenshtein(s4, s2));
    
    let s5 = "fuiton"; // transposed t and i
    println!("fuiton -> function levenshtein: {}", levenshtein(s5, s2));
}
