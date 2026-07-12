use eng_lexer::tokenize;

fn main() {
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
    for t in &tokens {
        println!("{:?}", t);
    }
}
