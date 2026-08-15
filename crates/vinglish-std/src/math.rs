use vinglish_bindgen::vinglish_bindgen;
use rand::Rng;

#[vinglish_bindgen]
pub fn math_sin(a: f64) -> f64 {
    a.sin()
}

#[vinglish_bindgen]
pub fn math_cos(a: f64) -> f64 {
    a.cos()
}

#[vinglish_bindgen]
pub fn math_tan(a: f64) -> f64 {
    a.tan()
}

#[vinglish_bindgen]
pub fn math_sqrt(a: f64) -> f64 {
    a.sqrt()
}

#[vinglish_bindgen]
pub fn math_pow(a: f64, b: f64) -> f64 {
    a.powf(b)
}

#[vinglish_bindgen]
pub fn math_random() -> f64 {
    rand::random::<f64>()
}
