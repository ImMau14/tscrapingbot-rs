mod types;
use reqwest::Client;
use serde_json::Value;
use types::*;

pub struct Gemini {
    pub api_key: String,
}

impl Gemini {
    pub fn new(api_key: String) -> Gemini {
        Gemini { api_key }
    }

    pub async fn ask(&self, prompt: String) -> String {
        let client = Client::new();
        let model_name = "gemini-2.0-flash";
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model_name, self.api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
        };

        let res = match client.post(url).json(&request_body).send().await {
            Ok(r) => r,
            Err(e) => return format!("HTTP request error: {}", e),
        };

        let status = res.status();
        let headers = res.headers().clone();
        let ct = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<no content-type>");

        let body_text = match res.text().await {
            Ok(t) => t,
            Err(e) => return format!("Error reading response body text: {}", e),
        };

        if !status.is_success() {
            return format!(
                "HTTP {} {}\nContent-Type: {}\nBody:\n{}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                ct,
                body_text
            );
        }

        match serde_json::from_str::<GenerateContentResponse>(&body_text) {
            Ok(parsed) => {
                if let Some(candidate) = parsed.candidates.first() {
                    if let Some(part) = candidate.content.parts.first() {
                        return part.text.clone();
                    }
                    return "No parts in candidate".into();
                }
                "No candidates".into()
            }
            Err(e) => match serde_json::from_str::<Value>(&body_text) {
                Ok(v) => {
                    format!(
                        "JSON deserialization mismatch: {}\nContent-Type: {}\nJSON body (abridged):\n{}",
                        e,
                        ct,
                        serde_json::to_string_pretty(&v)
                            .unwrap_or_else(|_| "<couldn't pretty print>".into())
                    )
                }
                Err(e2) => {
                    format!(
                        "Failed to parse body as JSON: {}\nAlso failed to parse as serde_json::Value: {}\nRaw body:\n{}",
                        e, e2, body_text
                    )
                }
            },
        }
    }
}
