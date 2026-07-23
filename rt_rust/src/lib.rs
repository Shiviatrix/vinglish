#![allow(clippy::not_unsafe_ptr_arg_deref)]

use minifb::{Window, WindowOptions, Key};
use std::sync::Mutex;
use std::collections::HashMap;
use vinglish_macro::vinglish_export;

struct WindowWrapper(Window);
unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}

struct BufferWrapper(Vec<u32>, usize, usize);
unsafe impl Send for BufferWrapper {}
unsafe impl Sync for BufferWrapper {}

lazy_static::lazy_static! {
    static ref WINDOWS: Mutex<HashMap<i32, WindowWrapper>> = Mutex::new(HashMap::new());
    static ref BUFFERS: Mutex<HashMap<i32, BufferWrapper>> = Mutex::new(HashMap::new());
    static ref NEXT_ID: Mutex<i32> = Mutex::new(1);
}

fn next_id() -> i32 {
    let mut id_lock = NEXT_ID.lock().unwrap();
    let id = *id_lock;
    *id_lock += 1;
    id
}

#[vinglish_export]
pub fn ui_create_window(title: String, width: i32, height: i32) -> i32 {
    let options = WindowOptions { resize: true, ..Default::default() };
    match Window::new(&title, width as usize, height as usize, options) {
        Ok(window) => {
            let id = next_id();
            WINDOWS.lock().unwrap().insert(id, WindowWrapper(window));
            id
        },
        Err(_) => -1,
    }
}

#[vinglish_export]
pub fn ui_window_is_open(id: i32) -> i32 {
    if let Some(wrapper) = WINDOWS.lock().unwrap().get(&id) {
        if wrapper.0.is_open() && !wrapper.0.is_key_down(Key::Escape) {
            return 1;
        }
    }
    0
}

#[vinglish_export]
pub fn ui_space_pressed(id: i32) -> i32 {
    if let Some(wrapper) = WINDOWS.lock().unwrap().get(&id) {
        if wrapper.0.is_key_down(Key::Space) {
            return 1;
        }
    }
    0
}

#[vinglish_export]
pub fn ui_create_buffer(width: i32, height: i32) -> i32 {
    let buf = vec![0; (width * height) as usize];
    let id = next_id();
    BUFFERS.lock().unwrap().insert(id, BufferWrapper(buf, width as usize, height as usize));
    id
}

#[vinglish_export]
pub fn ui_set_pixel(buf_id: i32, x: i32, y: i32, color: i32) {
    if let Some(wrapper) = BUFFERS.lock().unwrap().get_mut(&buf_id) {
        if x >= 0 && (x as usize) < wrapper.1 && y >= 0 && (y as usize) < wrapper.2 {
            let idx = (y as usize) * wrapper.1 + (x as usize);
            wrapper.0[idx] = color as u32;
        }
    }
}

#[vinglish_export]
pub fn ui_window_update(win_id: i32, buf_id: i32) {
    let mut buffers = BUFFERS.lock().unwrap();
    let mut windows = WINDOWS.lock().unwrap();
    
    if let (Some(win), Some(buf)) = (windows.get_mut(&win_id), buffers.get_mut(&buf_id)) {
        let _ = win.0.update_with_buffer(&buf.0, buf.1, buf.2);
    }
}

#[vinglish_export]
pub fn ui_fill_buffer(buf_id: i32, color: i32) {
    if let Some(wrapper) = BUFFERS.lock().unwrap().get_mut(&buf_id) {
        for pixel in wrapper.0.iter_mut() {
            *pixel = color as u32;
        }
    }
}
