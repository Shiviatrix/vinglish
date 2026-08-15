fn main() {
    let bindings = bindgen::Builder::default()
        .header_contents("wrapper.h", "struct Point { int x; int y; };")
        .generate()
        .unwrap();
    println!("{}", bindings.to_string());
}
