// Payload and responses types

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// INPUT TYPES
// =============================================================================

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InputBlob {
    pub mime_type: String,
    pub data: String, // Base64 string
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InputFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub file_uri: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum InputPart {
    #[serde(rename = "text")]
    Text(String),

    #[serde(rename = "inlineData")]
    InlineData(InputBlob),

    #[serde(rename = "fileData")]
    FileData(InputFile),
}

// =============================================================================
// RESPONSE TYPES
// =============================================================================

// Piece of response content, usually contains text.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResponsePart {
    // Text returned by the model, if present.
    pub text: Option<String>,

    // Catch-all for any other unexpected fields.
    #[serde(flatten)]
    pub other: Value,
}

// Content wrapper in a candidate, may carry mime type or parts.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResponseContent {
    // Parts may be absent in some responses.
    pub parts: Option<Vec<ResponsePart>>,

    // Optional mime type for the content (e.g., application/json).
    pub mime_type: Option<String>,

    // Catch-all for extra content-level fields.
    #[serde(flatten)]
    pub other: Value,
}

// Candidate from the model (one of possible completions).
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    // Optional index for ordering among candidates.
    pub index: Option<u32>,

    // The actual content for this candidate, if present.
    pub content: Option<ResponseContent>,

    // Catch-all for candidate-level extras (safety, annotations, etc.).
    #[serde(flatten)]
    pub other: Value,
}

// Top-level response for generateContent.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    // List of candidates; may be absent for some streaming patterns.
    pub candidates: Option<Vec<Candidate>>,

    // Catch-all for other top-level fields (usage, safety, etc.).
    #[serde(flatten)]
    pub other: Value,
}

// =============================================================================
// HELPER IMPLEMENTATIONS
// =============================================================================

impl GenerateContentResponse {
    // Return concatenated text from all candidates and parts.
    pub fn get_all_text(&self) -> String {
        let mut out = Vec::new();
        if let Some(cands) = &self.candidates {
            for cand in cands {
                if let Some(content) = &cand.content
                    && let Some(parts) = &content.parts
                {
                    for p in parts {
                        if let Some(t) = &p.text {
                            out.push(t.clone());
                        }
                    }
                }
            }
        }
        // Join parts with spacing for readability.
        out.join("\n\n")
    }

    // Try to parse the first part's text as JSON and return it.
    pub fn parse_first_candidate_json(&self) -> Option<Value> {
        if let Some(cands) = &self.candidates
            && let Some(c0) = cands.first()
            && let Some(content) = &c0.content
            && let Some(parts) = &content.parts
            && let Some(p0) = parts.first()
            && let Some(text) = &p0.text
            && let Ok(v) = serde_json::from_str::<Value>(text)
        {
            return Some(v);
        }

        None
    }
}
