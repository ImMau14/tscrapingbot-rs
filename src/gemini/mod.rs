// The Gemini Client

mod types;
use reqwest::Client;
use serde_json::{Map, Value, json};
use types::*;

// Gemini client that holds the API key.
pub struct Gemini {
    // API key used for requests.
    pub api_key: String,
}

impl Gemini {
    // Create a new Gemini client.
    pub fn new(api_key: String) -> Self {
        Gemini { api_key }
    }

    // Start a new request builder.
    pub fn request(&self) -> GeminiRequestBuilder {
        GeminiRequestBuilder {
            api_key: self.api_key.clone(),
            model: "gemini-2.5-flash".to_string(),
            parts: Vec::new(),
            temperature: None,
            top_p: None,
            top_k: None,
            max_output_tokens: None,
            candidate_count: None,
            system_instruction: None,
            thinking_budget: None,
            include_thoughts: None,
            response_mime_type: None,
        }
    }
}

// Result returned by the builder's send method.
pub struct GeminiResult {
    // Primary assistant answer text.
    pub answer: String,
    // Collected reasoning/thought parts (may be empty).
    pub thoughts: Vec<String>,
    // Raw typed response for advanced inspection.
    pub raw: GenerateContentResponse,
}

impl GeminiResult {
    // Return formatted string showing thoughts conditionally.
    pub fn formatted(&self, show_thoughts: bool) -> String {
        if show_thoughts && !self.thoughts.is_empty() {
            format!(
                "Thoughts:\n{}\n\nAnswer:\n{}",
                self.thoughts.join("\n\n"),
                self.answer
            )
        } else {
            self.answer.clone()
        }
    }
}

// Builder to configure a generateContent request.
pub struct GeminiRequestBuilder {
    // API key for the request.
    api_key: String,
    // Model name to call.
    model: String,

    // CONTENT PARTS
    parts: Vec<InputPart>,

    // Optional sampling params.
    temperature: Option<f64>,
    top_p: Option<f64>,
    top_k: Option<u32>,
    max_output_tokens: Option<u32>,
    candidate_count: Option<u32>,

    // Optional system instruction content.
    system_instruction: Option<String>,

    // Thinking config options.
    thinking_budget: Option<i32>,
    include_thoughts: Option<bool>,

    // Optional response mime type.
    response_mime_type: Option<String>,
}

impl GeminiRequestBuilder {
    // Set the model to use.
    pub fn set_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    // =========================================================================
    // MULTIMODAL INPUT METHODS
    // =========================================================================

    // Add text to the request.
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.parts.push(InputPart::Text(text.into()));
        self
    }

    // Add binary data (Images, small Audio, small PDF) in Base64.
    // mime_type e.g.: "image/jpeg", "audio/mp3", "application/pdf"
    pub fn add_base64_media(
        mut self,
        mime_type: impl Into<String>,
        base64_data: impl Into<String>,
    ) -> Self {
        self.parts.push(InputPart::InlineData(InputBlob {
            mime_type: mime_type.into(),
            data: base64_data.into(),
        }));
        self
    }

    // Add reference to an already uploaded file (File API) or a public resource.
    // Useful for Videos or heavy documents.
    pub fn add_file_uri(
        mut self,
        mime_type: impl Into<String>,
        file_uri: impl Into<String>,
    ) -> Self {
        self.parts.push(InputPart::FileData(InputFile {
            mime_type: Some(mime_type.into()),
            file_uri: file_uri.into(),
        }));
        self
    }

    // =========================================================================
    // CONFIGURATION (EXISTING)
    // =========================================================================

    // Set temperature.
    pub fn set_temperature(mut self, t: f64) -> Self {
        self.temperature = Some(t);
        self
    }

    // Set nucleus sampling p.
    pub fn set_top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    // Set top-k sampling.
    pub fn set_top_k(mut self, k: u32) -> Self {
        self.top_k = Some(k);
        self
    }

    // Set maximum output tokens.
    pub fn set_max_output_tokens(mut self, n: u32) -> Self {
        self.max_output_tokens = Some(n);
        self
    }

    // Request multiple candidates.
    pub fn set_candidate_count(mut self, n: u32) -> Self {
        self.candidate_count = Some(n);
        self
    }

    // Provide a system instruction.
    pub fn set_system_instruction(mut self, inst: impl Into<String>) -> Self {
        self.system_instruction = Some(inst.into());
        self
    }

    // Set thinking budget.
    pub fn set_thinking_budget(mut self, budget: i32) -> Self {
        self.thinking_budget = Some(budget);
        self
    }

    // Request that the API include thought parts.
    pub fn set_include_thoughts(mut self, include: bool) -> Self {
        self.include_thoughts = Some(include);
        self
    }

    // Set response mime type (e.g., "application/json").
    pub fn set_response_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.response_mime_type = Some(mime.into());
        self
    }

    // =========================================================================
    // SEND (MODIFIED)
    // =========================================================================

    // Send the request using the accumulated parts.
    // No longer takes "prompt" as parameter, you must use .add_text().
    pub async fn send(self) -> Result<GeminiResult, String> {
        if self.parts.is_empty() {
            return Err("No content parts provided. Use add_text, add_base64_media, or add_file_uri before sending.".to_string());
        }

        // Build base body. "parts" is now a Vec<InputPart> which serializes correctly via serde.
        let mut body = json!({
            "contents": [
                { "parts": self.parts }
            ]
        });

        // Attach systemInstruction if provided.
        if let Some(sys) = self.system_instruction {
            body["systemInstruction"] = json!({
                "parts": [ { "text": sys } ]
            });
        }

        // Assemble generationConfig with provided fields.
        let mut gen_cfg = Map::new();

        if let Some(t) = self.temperature {
            gen_cfg.insert("temperature".to_string(), json!(t));
        }
        if let Some(tp) = self.top_p {
            gen_cfg.insert("topP".to_string(), json!(tp));
        }
        if let Some(tk) = self.top_k {
            gen_cfg.insert("topK".to_string(), json!(tk));
        }
        if let Some(m) = self.max_output_tokens {
            gen_cfg.insert("maxOutputTokens".to_string(), json!(m));
        }
        if let Some(c) = self.candidate_count {
            gen_cfg.insert("candidateCount".to_string(), json!(c));
        }
        if let Some(rm) = self.response_mime_type {
            gen_cfg.insert("responseMimeType".to_string(), json!(rm));
        }

        // Add thinkingConfig if any thinking option present.
        if self.thinking_budget.is_some() || self.include_thoughts.is_some() {
            let mut thinking = Map::new();
            if let Some(b) = self.thinking_budget {
                thinking.insert("thinkingBudget".to_string(), json!(b));
            }
            if let Some(inc) = self.include_thoughts {
                thinking.insert("includeThoughts".to_string(), json!(inc));
            }
            gen_cfg.insert("thinkingConfig".to_string(), Value::Object(thinking));
        }

        // Attach generationConfig only when not empty.
        if !gen_cfg.is_empty() {
            body["generationConfig"] = Value::Object(gen_cfg);
        }

        // Send HTTP POST.
        let client = Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let res = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {}", e))?;

        // Diagnostics: status and content-type.
        let status = res.status();
        let ct = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<no content-type>")
            .to_string();

        // Read body text.
        let body_text = res
            .text()
            .await
            .map_err(|e| format!("Error reading response body text: {}", e))?;

        // Return HTTP error with diagnostics.
        if !status.is_success() {
            return Err(format!(
                "HTTP {} {}\nContent-Type: {}\nBody:\n{}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                ct,
                body_text
            ));
        }

        // Deserialize typed response or return diagnostics.
        let parsed: GenerateContentResponse = serde_json::from_str(&body_text).map_err(|parse_err| {
            match serde_json::from_str::<Value>(&body_text) {
                Ok(v) => format!(
                    "JSON deserialization mismatch: {}\nContent-Type: {}\nJSON body (abridged):\n{}",
                    parse_err,
                    ct,
                    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "<couldn't pretty print>".into())
                ),
                Err(parse_err2) => format!(
                    "Failed to parse body as JSON: {}\nAlso failed to parse as serde_json::Value: {}\nRaw body:\n{}",
                    parse_err, parse_err2, body_text
                ),
            }
        })?;

        // Extract thoughts and the main answer from parts.
        let mut thoughts: Vec<String> = Vec::new();
        let mut answer: Option<String> = None;

        if let Some(candidates) = &parsed.candidates {
            for cand in candidates {
                if let Some(content) = &cand.content
                    && let Some(parts) = &content.parts
                {
                    for p in parts {
                        // Clone text if present.
                        let text_opt = p.text.clone();

                        // Detect thought flag in flattened "other" fields.
                        let is_thought = p
                            .other
                            .get("thought")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        // If this part is a thought, collect it.
                        if is_thought {
                            if let Some(t) = text_opt.clone() {
                                thoughts.push(t);
                            }
                            continue;
                        }

                        // Use the first non-thought part as the answer.
                        if answer.is_none()
                            && let Some(t) = text_opt.clone()
                        {
                            answer = Some(t);
                        }
                    }
                }
                // Stop early if we have at least one answer and some thoughts.
                if answer.is_some() && !thoughts.is_empty() {
                    break;
                }
            }
        }

        // Fallback concatenation if no direct answer found.
        let final_answer = if let Some(a) = answer {
            a
        } else {
            parsed.get_all_text()
        };

        Ok(GeminiResult {
            answer: final_answer,
            thoughts,
            raw: parsed,
        })
    }
}
