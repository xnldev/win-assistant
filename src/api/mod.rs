pub mod gemini;
pub mod openai;

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum QueryPayload {
    ScreenImageBase64(String),
    ClipboardText(String),
}

#[allow(dead_code)]
pub fn execute_query(config: &Config, payload: QueryPayload) -> Result<String, String> {
    execute_query_stream(config, payload, |_| {})
}

pub fn execute_query_stream<F>(
    config: &Config,
    payload: QueryPayload,
    on_chunk: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    let keys = config.get_active_keys();
    let primary_key = keys.first().cloned().unwrap_or_else(|| config.api_key.clone());

    match payload {
        QueryPayload::ScreenImageBase64(img_b64) => {
            if config.provider.eq_ignore_ascii_case("openai") {
                openai::request_openai_stream(
                    &primary_key,
                    &config.model,
                    &config.system_prompt,
                    config.api_url.as_deref(),
                    Some(&img_b64),
                    None,
                    on_chunk,
                )
            } else {
                gemini::request_gemini_stream(
                    &keys,
                    &config.model,
                    &config.system_prompt,
                    Some(&img_b64),
                    None,
                    on_chunk,
                )
            }
        }
        QueryPayload::ClipboardText(text) => {
            if config.provider.eq_ignore_ascii_case("openai") {
                openai::request_openai_stream(
                    &primary_key,
                    &config.model,
                    &config.system_prompt,
                    config.api_url.as_deref(),
                    None,
                    Some(&text),
                    on_chunk,
                )
            } else {
                gemini::request_gemini_stream(
                    &keys,
                    &config.model,
                    &config.system_prompt,
                    None,
                    Some(&text),
                    on_chunk,
                )
            }
        }
    }
}
