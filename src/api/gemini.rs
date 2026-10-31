use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[allow(dead_code)]
pub fn request_gemini(
    api_keys: &[String],
    model: &str,
    system_prompt: &str,
    image_base64: Option<&str>,
    text_query: Option<&str>,
) -> Result<String, String> {
    request_gemini_stream(api_keys, model, system_prompt, image_base64, text_query, |_| {})
}

pub fn request_gemini_stream<F>(
    api_keys: &[String],
    model: &str,
    system_prompt: &str,
    image_base64: Option<&str>,
    text_query: Option<&str>,
    mut on_chunk: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    if api_keys.is_empty() {
        return Err("API key is not configured! Please specify a valid api_key in config.toml".to_string());
    }

    let std_client = build_client(false)?;
    let anycast_client = build_client(true)?;

    // Migrate deprecated or nonexistent models to official working endpoints
    let primary_model = match model.trim() {
        "gemini-2.5-flash" | "gemini-2.0-flash" | "gemini-1.5-flash" | "gemini-3.5-flash" | "gemini-3.8-flash" | "" => "gemini-flash-latest",
        "gemini-flash-lite" => "gemini-flash-lite-latest",
        "gemini-pro" => "gemini-pro-latest",
        other => other,
    };

    let mut parts: Vec<Value> = Vec::new();

    let full_prompt = match text_query {
        Some(custom) => format!("{}\n\n[Context / Question]:\n{}", system_prompt, custom),
        None => format!(
            "{}\n\n[Instructions]:\n\
            1. If there are QUESTIONS / TESTS / TASKS: Answer EVERY single one sequentially (1, 2, 3...) without skipping. State question number, exact correct option letter and text, and copy-paste ready code.\n\
            2. If there are NO direct questions (only theses, notes, slides, or theory): NEVER merely rewrite the text. Provide a high-yield breakdown: 🎯 Core Takeaways (what it means), 💡 Key Concept Explanation, and ⚡ Likely Test Questions & Answers.\n\
            3. Match the language of the screen (Ukrainian/English). Avoid filler intros.",
            system_prompt
        ),
    };

    parts.push(json!({
        "text": full_prompt
    }));

    // In Gemini REST API v1beta, the schema requires camelCase inlineData / mimeType
    if let Some(img_b64) = image_base64 {
        parts.push(json!({
            "inlineData": {
                "mimeType": "image/jpeg",
                "data": img_b64
            }
        }));
    }

    let payload = json!({
        "contents": [
            {
                "parts": parts
            }
        ],
        "generationConfig": {
            "temperature": 0.15,
            "maxOutputTokens": 2048
        }
    });

    // Smart Multi-Model Fallback Chain using verified endpoints
    let mut candidate_models = vec![primary_model];
    for m in &["gemini-flash-latest", "gemini-flash-lite-latest", "gemini-pro-latest"] {
        if !candidate_models.contains(m) {
            candidate_models.push(m);
        }
    }

    let mut last_err = String::new();

    for (key_idx, raw_key) in api_keys.iter().enumerate() {
        let clean_key = raw_key.trim();
        if clean_key.is_empty() || clean_key == "YOUR_GEMINI_OR_OPENAI_API_KEY" {
            continue;
        }

        let mut key_quota_exhausted = false;

        for candidate in &candidate_models {
            match send_gemini_with_retry_stream(&std_client, &anycast_client, clean_key, candidate, &payload, &mut on_chunk) {
                Ok(ans) => return Ok(ans),
                Err(err) => {
                    // If 429 quota exhausted or 403 forbidden, switch to backup key immediately
                    if err.contains("429") || err.contains("403") {
                        if api_keys.len() > 1 && key_idx + 1 < api_keys.len() {
                            eprintln!("[KEY-FALLBACK] Key #{} exceeded quota (429). Switching to backup key #{}...", key_idx + 1, key_idx + 2);
                        }
                        last_err = err;
                        key_quota_exhausted = true;
                        break;
                    }
                    last_err = err;
                }
            }
        }

        if !key_quota_exhausted && last_err.is_empty() {
            break;
        }
    }

    if last_err.contains("429") && api_keys.len() > 1 {
        return Err(format!(
            "All {} configured API keys reached Google's quota limit (429).\n\n\
            Important: Keys created in the same Google Cloud project share the exact same quota pool!\n\
            To get a separate quota, create the backup key in a NEW project or under a second Google account in Google AI Studio.",
            api_keys.len()
        ));
    }

    Err(last_err)
}

fn build_client(use_custom_dns: bool) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .connect_timeout(Duration::from_secs(6))
        .timeout(Duration::from_secs(22));

    if use_custom_dns {
        let google_addrs = [
            "172.217.116.4:443".parse().unwrap(),
            "172.217.117.4:443".parse().unwrap(),
            "142.250.74.202:443".parse().unwrap(),
        ];
        builder = builder.resolve_to_addrs("generativelanguage.googleapis.com", &google_addrs);
    }

    builder.build().map_err(|e| format!("HTTP Client initialization error: {}", e))
}

fn send_gemini_with_retry_stream<F>(
    std_client: &Client,
    anycast_client: &Client,
    api_key: &str,
    model: &str,
    payload: &Value,
    on_chunk: &mut F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    match send_single_request(std_client, api_key, model, payload) {
        Ok(ans) => {
            on_chunk(&ans);
            Ok(ans)
        }
        Err(err) => {
            if err.contains("429") || err.contains("403") || err.contains("404") {
                return Err(err);
            }
            // Fast failover to direct Google Anycast IP client
            let ans = send_single_request(anycast_client, api_key, model, payload)?;
            on_chunk(&ans);
            Ok(ans)
        }
    }
}

#[allow(dead_code)]
fn send_single_request_stream<F>(
    client: &Client,
    api_key: &str,
    model: &str,
    payload: &Value,
    on_chunk: &mut F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
        model, api_key
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(payload)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                format!("Network timeout connecting to model {}. Check your connection.", model)
            } else {
                format!("Network request failed: {}", e)
            }
        })?;

    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let body_text = resp.text().unwrap_or_default();
        if code == 429 {
            return Err("Gemini Free Tier Quota Exceeded (429): You have temporarily reached Google's rate limit for this API key. Please wait 1-2 minutes for the quota window to reset, or obtain a fresh key from Google AI Studio.".to_string());
        }
        if code == 503 {
            return Err(format!("Gemini High Demand (503): Model '{}' is temporarily overloaded.", model));
        }

        if let Ok(err_json) = serde_json::from_str::<Value>(&body_text) {
            if let Some(msg) = err_json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                return Err(format!("Gemini API error ({}) [{}]: {}", status, model, msg));
            }
        }
        return Err(format!("Gemini API HTTP {} [{}]: {}", status, model, body_text));
    }

    let mut full_text = String::new();
    let reader = BufReader::new(resp);

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if let Some(json_str) = trimmed.strip_prefix("data: ") {
            if let Ok(json_resp) = serde_json::from_str::<Value>(json_str) {
                if let Some(chunk) = json_resp
                    .get("candidates")
                    .and_then(|c| c.get(0))
                    .and_then(|c0| c0.get("content"))
                    .and_then(|cnt| cnt.get("parts"))
                    .and_then(|parts| parts.get(0))
                    .and_then(|p0| p0.get("text"))
                    .and_then(|t| t.as_str())
                {
                    full_text.push_str(chunk);
                    on_chunk(chunk);
                }
            }
        }
    }

    if !full_text.trim().is_empty() {
        Ok(full_text.trim().to_string())
    } else {
        // Fallback: if SSE stream had no chunks, perform standard generateContent
        send_single_request(client, api_key, model, payload)
    }
}

fn send_single_request(
    client: &Client,
    api_key: &str,
    model: &str,
    payload: &Value,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(payload)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                format!("Network timeout connecting to model {}. Check your connection.", model)
            } else {
                format!("Network request failed: {}", e)
            }
        })?;

    let status = resp.status();
    let body_text = resp
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if !status.is_success() {
        let code = status.as_u16();
        if code == 429 {
            return Err("Gemini Free Tier Quota Exceeded (429): You have temporarily reached Google's rate limit for this API key. Please wait 1-2 minutes for the quota window to reset, or obtain a fresh key from Google AI Studio.".to_string());
        }
        if code == 503 {
            return Err(format!("Gemini High Demand (503): Model '{}' is temporarily overloaded.", model));
        }

        if let Ok(err_json) = serde_json::from_str::<Value>(&body_text) {
            if let Some(msg) = err_json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                return Err(format!("Gemini API error ({}) [{}]: {}", status, model, msg));
            }
        }
        return Err(format!("Gemini API HTTP {} [{}]: {}", status, model, body_text));
    }

    let json_resp: Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    let candidate_text = json_resp
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("content"))
        .and_then(|cnt| cnt.get("parts"))
        .and_then(|parts| parts.get(0))
        .and_then(|p0| p0.get("text"))
        .and_then(|t| t.as_str());

    match candidate_text {
        Some(text) => Ok(text.trim().to_string()),
        None => Err("Received empty text candidate from Gemini model".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_connection() {
        let keys = vec![
            "AQ.Ab8RN6L_czzg24ZyEfygIBpnjNEFXBx4CtL54xQ4yTxK-NiKig".to_string(),
            "AQ.Ab8RN6JEPCeTRHd_E3pJQIJbmUZy8YR2N121qNJIc67XQ5LQQQ".to_string(),
        ];
        let res = request_gemini(&keys, "gemini-flash-latest", "Respond only with PONG", None, Some("Ping"));
        println!("\n=== TEST TEXT RESULT: {:?} ===\n", res);
        assert!(res.is_ok(), "Request failed: {:?}", res.err());
    }

    #[test]
    fn test_gemini_with_image() {
        let keys = vec!["AQ.Ab8RN6L_czzg24ZyEfygIBpnjNEFXBx4CtL54xQ4yTxK-NiKig".to_string()];
        // 1x1 red PNG base64
        let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let res = request_gemini(&keys, "gemini-flash-latest", "What color is this image? Reply in 1 word.", Some(b64), None);
        println!("\n=== TEST IMAGE RESULT: {:?} ===\n", res);
        assert!(res.is_ok(), "Image request failed: {:?}", res.err());
    }
}

