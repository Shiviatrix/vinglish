use vinglish_bindgen::vinglish_bindgen;
use reqwest::blocking::Client;
use std::env;

#[vinglish_bindgen]
pub fn ai_prompt(instruction: &str, input_text: &str) -> String {
    let client = Client::new();
    // Default to a dummy URL if no API key is provided, or a local Ollama instance
    let url = env::var("AI_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434/api/generate".to_string());
    
    // Simplistic JSON payload for Ollama
    let payload = serde_json::json!({
        "model": "llama3",
        "prompt": format!("{}\n\n{}", instruction, input_text),
        "stream": false
    });
    
    match client.post(&url).json(&payload).send() {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>() {
                if let Some(response) = json.get("response").and_then(|r| r.as_str()) {
                    return response.to_string();
                }
            }
            "AI response unparseable or API not running.".to_string()
        },
        Err(_) => "Failed to reach AI endpoint (is Ollama running?)".to_string(),
    }
}

#[vinglish_bindgen]
pub fn ai_extract_json(instruction: &str, text: &str) -> String {
    let client = Client::new();
    let url = env::var("AI_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434/api/generate".to_string());
    
    let strict_instruction = format!(
        "{}\n\nYou MUST return ONLY raw JSON. No markdown formatting, no backticks, no explanations. Just parseable JSON.",
        instruction
    );
    
    let payload = serde_json::json!({
        "model": "llama3",
        "prompt": format!("{}\n\n{}", strict_instruction, text),
        "stream": false,
        "format": "json" // Many local LLM engines support this natively
    });
    
    match client.post(&url).json(&payload).send() {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>() {
                if let Some(response) = json.get("response").and_then(|r| r.as_str()) {
                    return response.trim().to_string();
                }
            }
            "{}".to_string()
        },
        Err(_) => "{}".to_string(),
    }
}
