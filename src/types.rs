use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ChunkData {
    pub id: String,
    pub file_path: String,
    pub language: String,
    pub chunk_type: String,
    pub chunk_name: String,
    pub context: String,
    pub signature: String,
    pub comment: String,
    pub code: String,
    pub start_line: usize,
    pub end_line: usize,
    pub token_count: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SubChunkData {
    pub text: String,
    pub token_count: usize,
    pub line_offset: usize,
}

pub const VALID_KINDS: [&str; 7] = [
    "class", "function", "method", "struct", "impl", "mod", "enum",
];
