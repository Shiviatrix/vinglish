extern crate strsim;
use strsim::jaro_winkler;

fn main() {
    let tests = [
        ("funtion", "function"), // transposition
        ("fction", "function"),  // missing char
        ("funcction", "function"), // duplicated char
        ("FUNCTION", "function"), // case error
        ("hello", "function"),   // different word
    ];
    
    for &(s1, s2) in tests.iter() {
        let score = jaro_winkler(s1, s2);
        println!("{:>12} -> {:<8} | jaro_winkler: {:.3} | >0.85: {}", 
                 s1, s2, score, score > 0.85);
    }
}
