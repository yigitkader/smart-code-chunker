use anyhow::{Result, anyhow};
use serde::Serialize;
use std::sync::Mutex;
use tree_sitter::{Language, Node, Parser as TreeParser, Query, QueryCursor};

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
