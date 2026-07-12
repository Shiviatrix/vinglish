use eng_lexer::tokenize;

fn main() {
<<<<<<< Updated upstream
    let src = r#"function fibonacci(number n)
returns number
begin
    if n is below 2
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
end
"#;
    let (tokens, errs) = tokenize(src);
    for e in &errs { eprintln!("lex error: {:?}", e); }
=======
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: tok_dump <file>");
        return;
    }
    let src = std::fs::read_to_string(&args[1]).unwrap();
    let (tokens, errs) = tokenize(&src);
    for e in &errs {
        eprintln!("lex error: {:?}", e);
    }
>>>>>>> Stashed changes
    for t in &tokens {
        println!("{:?}", t);
    }
}
