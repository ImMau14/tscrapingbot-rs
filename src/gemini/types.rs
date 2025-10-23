use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Part {
    pub text: String,
}

#[derive(Serialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Serialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
}

#[derive(Deserialize, Debug)]
pub struct ResponsePart {
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct ResponseContent {
    pub parts: Vec<ResponsePart>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: ResponseContent,
}

#[derive(Deserialize, Debug)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
}
