use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[allow(dead_code)]
pub fn request_openai(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    api_url: Option<&str>,
    image_base64: Option<&str>,
    text_query: Option<&str>,
) -> Result<String, String> {
    request_openai_stream(api_key, model, system_prompt, api_url, image_base64, text_query, |_| {})
}

pub fn request_openai_stream<F>(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    api_url: Option<&str>,
    image_base64: Option<&str>,
    text_query: Option<&str>,
    mut on_chunk: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    let clean_key = api_key.trim();
    if clean_key.is_empty() || clean_key == "YOUR_GEMINI_OR_OPENAI_API_KEY" {
        return Err("API key is not configured! Please specify a valid api_key in config.toml".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(35))
        .build()
        .map_err(|e| format!("HTTP Client error: {}", e))?;

    let url = api_url.unwrap_or("https://api.openai.com/v1/chat/completions");

    let mut messages: Vec<Value> = Vec::new();

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));

    if let Some(img_b64) = image_base64 {
        let user_prompt = match text_query {
            Some(custom) => format!("Analyze the image and solve the problem:\n{}", custom),
            None => "Thoroughly scan the entire image. If there are MULTIPLE questions or tasks, answer EVERY single one in order (1, 2, 3...) with the direct correct answer, options, and concise code. Do not skip any question.".to_string(),
        };

        messages.push(json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": user_prompt
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", img_b64),
                        "detail": "high"
                    }
                }
            ]
        }));
    } else {
        let user_text = text_query.unwrap_or("Provide the answer to the question.");
        messages.push(json!({
            "role": "user",
            "content": user_text
        }));
    }

    let payload = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.15,
        "max_tokens": 2048,
        "stream": true
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", clean_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&payload)
        .send()
        .map_err(|e| format!("Network request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        if let Ok(err_json) = serde_json::from_str::<Value>(&body_text) {
            if let Some(msg) = err_json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                return Err(format!("OpenAI API ({}): {}", status, msg));
            }
        }
        return Err(format!("OpenAI API HTTP {}: {}", status, body_text));
    }

    let mut full_text = String::new();
    let reader = BufReader::new(resp);

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed == "data: [DONE]" {
            break;
        }
        if let Some(json_str) = trimmed.strip_prefix("data: ") {
            if let Ok(json_resp) = serde_json::from_str::<Value>(json_str) {
                if let Some(delta) = json_resp
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c0| c0.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|cnt| cnt.as_str())
                {
                    full_text.push_str(delta);
                    on_chunk(delta);
                }
            }
        }
    }

    if !full_text.trim().is_empty() {
        Ok(full_text.trim().to_string())
    } else {
        Err("Received empty response stream from OpenAI API".to_string())
    }
}
