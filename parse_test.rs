fn main() {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .generate()
        .unwrap();
    println!("{}", bindings.to_string());
}
