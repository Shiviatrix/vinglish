use std::env;
use std::process::Command;
use vinglish_bindgen::vinglish_bindgen;

#[vinglish_bindgen]
pub fn sys_env(key: &str) -> String {
    env::var(key).unwrap_or_default()
}

#[vinglish_bindgen]
pub fn sys_exec(cmd: &str) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    };

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(e) => format!("Error executing command: {}", e),
    }
}
