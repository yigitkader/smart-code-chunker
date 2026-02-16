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

pub struct ThreadSafeParser {
    parser: Mutex<TreeParser>,
}
impl ThreadSafeParser {
    pub fn new() -> Self {
        Self {
            parser: Mutex::new(TreeParser::new()),
        }
    }
    pub fn parse(&self, content: &str, language: Language) -> Result<tree_sitter::Tree> {
        let mut parser = self.parser.lock().map_err(|_| anyhow!("Lock poisoned"))?;
        parser.set_language(language)?;
        parser.reset();
        parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Parse failed"))
    }
}
