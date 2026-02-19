use crate::git::get_git_changes;
use crate::hash::compute_hash;
use crate::lang_driver::get_driver;
use crate::types::ChunkData;
use anyhow::{Error, Result, anyhow};
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tiktoken_rs::{CoreBPE, cl100k_base};
use tree_sitter::{Node, Parser, Query, QueryCursor};

pub fn get_files(path: &str, since: &Option<String>) -> Result<Vec<PathBuf>, Error> {
    let files: Vec<PathBuf> = if let Some(commit_hash) = &since {
        println!("Smart chunker started with git hash: {}", commit_hash);
        get_git_changes(path, commit_hash)?
    } else {
        println!("Smart chunker started with full scan");
        let walker = WalkBuilder::new(path).standard_filters(true).build();
        walker
            .filter_map(|r| r.ok().map(|e| e.into_path()).filter(|p| p.is_file()))
            .collect()
    };
    Ok(files)
}

static TOKENIZER: once_cell::sync::Lazy<CoreBPE> =
    once_cell::sync::Lazy::new(|| cl100k_base().expect("Failed to load tokenizer"));

pub fn process_file(
    path: &Path,
    parser: &mut Parser,
    tx_sender: &crossbeam_channel::Sender<ChunkData>,
    max_chunk_tokens: usize,
) -> Result<()> {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();
    let driver = match get_driver(&extension) {
        Some(driver) => driver,
        None => {
            println!("No driver found for file: {:?}", path);
            return Ok(());
        }
    };

    let content = fs::read_to_string(path)?;
    parser.set_language(driver.get_language())?;
    parser.reset();
    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| anyhow!("Failed to parse file"))?;
    let mut cursor = QueryCursor::new();
    let query = Query::new(driver.get_language(), driver.get_query())?;
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    for m in matches {
        for capture in m.captures {
            let node = capture.node;

            let mut context_parts = Vec::new();
            let mut parent = node.parent();
            while let Some(p) = parent {
                let kind = p.kind();
                if kind.contains("class")
                    || kind.contains("function")
                    || kind.contains("method")
                    || kind.contains("struct")
                    || kind.contains("impl")
                    || kind.contains("mod")
                    || kind.contains("enum")
                {
                    let name = driver.extract_name(&p, &content).unwrap_or("?");
                    let clean_kind = kind.replace("_item", "").replace("_definition", "");
                    context_parts.push(format!("{}({})", clean_kind, name));
                }
                parent = p.parent();
            }

            context_parts.reverse();
            let context = if context_parts.is_empty() {
                "root".to_string()
            } else {
                context_parts.join(" > ")
            };

            let chunk_name = driver
                .extract_name(&node, &content)
                .unwrap_or("anonymous")
                .to_string();

            let raw_code_bytes = &content[node.start_byte()..node.end_byte()];
            let comments = get_preceding_comments(&node, &content).unwrap_or_default();

            let signature = raw_code_bytes.lines().next().unwrap_or("").to_string();

            let full_text_for_ai = format!("{}\n{}", comments, raw_code_bytes);

            let sub_chunks =
                split_text_by_token_limit(&full_text_for_ai, &TOKENIZER, max_chunk_tokens);

            for (i, (sub_text, token_count, line_offset)) in sub_chunks.into_iter().enumerate() {
                let unique_content = format!("{}-{}", sub_text, i);
                let id = compute_hash(&unique_content);

                let original_start_line = node.start_position().row + 1;

                let chunk = ChunkData {
                    id,
                    file_path: path.to_string_lossy().to_string(),
                    language: driver.get_name().to_string(),
                    chunk_type: node.kind().to_string(),
                    chunk_name: chunk_name.clone(),
                    context: context.clone(),
                    signature: signature.clone(),
                    comment: comments.clone(),
                    code: sub_text,
                    start_line: original_start_line + line_offset,
                    end_line: original_start_line
                        + line_offset
                        + raw_code_bytes.lines().count().min(1),
                    token_count,
                };

                if tx_sender.send(chunk).is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn split_text_by_token_limit(
    text: &String,
    tokenizer: &CoreBPE,
    max_tokens: usize,
) -> Vec<(String, usize, usize)> {
    // (Text, TokenCount, LineOffset)
    let encoded = tokenizer.encode_with_special_tokens(text);
    if encoded.len() <= max_tokens {
        return vec![(text.to_string(), encoded.len(), 0)];
    }

    let mut chunks: Vec<(String, usize, usize)> = Vec::new();
    let mut current_chunk_lines: Vec<&str> = Vec::new();
    let mut current_tokens = 0;
    let mut current_line_offset = 0;

    for line in text.lines() {
        let line_len = tokenizer.encode_with_special_tokens(line).len();
        if current_tokens + line_len + 1 > max_tokens {
            if !current_chunk_lines.is_empty() {
                let chunk_str = current_chunk_lines.join("\n");
                chunks.push((chunk_str, current_tokens, current_line_offset));
                current_line_offset += current_chunk_lines.len();
                current_chunk_lines.clear();
                current_tokens = 0;
            }
        }
        current_chunk_lines.push(line);
        current_tokens += line_len + 1;
    }

    if !current_chunk_lines.is_empty() {
        chunks.push((
            current_chunk_lines.join("\n"),
            current_tokens,
            current_line_offset,
        ));
    }

    chunks
}

fn get_preceding_comments(node: &Node, content: &str) -> Option<String> {
    let mut comments: Vec<String> = Vec::new();
    let mut current = node.prev_sibling();
    while let Some(sibling) = current {
        let kind = sibling.kind();
        if kind.contains("comment") {
            let text = &content[sibling.start_byte()..sibling.end_byte()];
            comments.push(text.trim().to_string());
        } else if !kind.trim().is_empty() {
            break;
        }
        current = sibling.prev_sibling();
    }
    if comments.is_empty() {
        None
    } else {
        comments.reverse();
        Some(comments.join("\n"))
    }
}
