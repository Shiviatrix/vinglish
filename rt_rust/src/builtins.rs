use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn to_text(val: i64) -> *const c_char {
    let s = val.to_string();
    CString::new(s).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn to_number(s: *const c_char) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        if let Ok(str_slice) = CStr::from_ptr(s).to_str() {
            str_slice.parse::<i64>().unwrap_or(0)
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn max(a: i64, b: i64) -> i64 {
    a.max(b)
}

#[no_mangle]
pub extern "C" fn min(a: i64, b: i64) -> i64 {
    a.min(b)
}
