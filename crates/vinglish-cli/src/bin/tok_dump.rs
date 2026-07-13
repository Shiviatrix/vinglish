use vinglish_lexer::tokenize;

fn main() {
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
    for t in &tokens {
        println!("{:?}", t);
    }
}
