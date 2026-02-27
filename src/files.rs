use crate::git::get_git_changes;
use crate::hash::compute_hash;
use crate::lang_driver::get_driver;
use crate::types::{ChunkData, SubChunkData, VALID_KINDS};
use anyhow::{Error, Result, anyhow};
use ignore::WalkBuilder;
use log::{error, info, warn};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tiktoken_rs::{CoreBPE, cl100k_base};
use tree_sitter::{Node, Parser, Query, QueryCursor};

pub fn get_files(path: &str, since: &Option<String>) -> Result<Vec<PathBuf>, Error> {
    let files: Vec<PathBuf> = if let Some(commit_hash) = &since {
        info!("Smart chunker started with git hash: {}", commit_hash);
        get_git_changes(path, commit_hash)?
    } else {
        info!("Smart chunker started with full scan");
        let walker = WalkBuilder::new(path).standard_filters(true).build();
        walker
            .filter_map(|r| r.ok().map(|e| e.into_path()).filter(|p| p.is_file()))
            .collect()
    };
    Ok(files)
}

static TOKENIZER: once_cell::sync::Lazy<CoreBPE> =
    once_cell::sync::Lazy::new(|| cl100k_base().expect("Failed to load tokenizer"));

fn get_file_extension(path: &Path) -> String {
    path.extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_lowercase()
}

pub fn process_file(
    path: &Path,
    parser: &mut Parser,
    tx_sender: &crossbeam_channel::Sender<ChunkData>,
    max_chunk_tokens: usize,
) -> Result<()> {
    let extension = get_file_extension(path);
    let driver = match get_driver(&extension) {
        Some(driver) => driver,
        None => {
            warn!("No driver found for file: {:?}", path);
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
                if VALID_KINDS.contains(&kind) {
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

            for (i, sub_chunk) in sub_chunks.into_iter().enumerate() {
                let text = sub_chunk.text;
                let line_offset = sub_chunk.line_offset;
                let token_count = sub_chunk.token_count;
                let unique_content = format!("{}-{}-{}", path.display(), text, i);
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
                    code: text.clone(),
                    start_line: original_start_line + line_offset,
                    end_line: original_start_line + line_offset + text.lines().count().saturating_sub(1),

                    token_count,
                };

                if tx_sender.send(chunk).is_err() {
                    error!("Failed to send chunk to channel for file: {:?}", path);
                    break;
                }
            }
        }
    }

    Ok(())
}

fn split_text_by_token_limit(
    text: &str,
    tokenizer: &CoreBPE,
    max_tokens: usize,
) -> Vec<SubChunkData> {
    let encoded = tokenizer.encode_with_special_tokens(text);
    if encoded.len() <= max_tokens {
        return vec![SubChunkData {
            text: text.to_string(),
            token_count: encoded.len(),
            line_offset: 0,
        }];
    }

    let mut chunks: Vec<SubChunkData> = Vec::new();
    let mut current_chunk_lines: Vec<&str> = Vec::new();
    let mut current_tokens = 0;
    let mut current_line_offset = 0;

    for line in text.lines() {
        let line_len = tokenizer.encode_with_special_tokens(line).len();
        if current_tokens + line_len + 1 > max_tokens {
            if !current_chunk_lines.is_empty() {
                let chunk_str = current_chunk_lines.join("\n");
                chunks.push(SubChunkData {
                    text: chunk_str,
                    token_count: current_tokens,
                    line_offset: current_line_offset,
                });
                current_line_offset += current_chunk_lines.len();
                current_chunk_lines.clear();
                current_tokens = 0;
            }
        }
        current_chunk_lines.push(line);
        current_tokens += line_len + 1;
    }

    if !current_chunk_lines.is_empty() {
        chunks.push(SubChunkData {
            text: current_chunk_lines.join("\n"),
            token_count: current_tokens,
            line_offset: current_line_offset,
        });
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

mod file_tests {
    use super::*;

    #[test]
    fn test_split_text_by_token_limit_if_not_reach_limit() {
        let text = "Here is some sample code:\nfn example() {\n    println!(\"Hello, world!\");\n}\n// This is a comment";
        let tokenizer = cl100k_base().expect("Failed to load tokenizer");
        let max_tokens = 100;

        let expected = [SubChunkData { text: "Here is some sample code:\nfn example() {\n    println!(\"Hello, world!\");\n}\n// This is a comment".to_string(), token_count: 23, line_offset: 0 }];

        assert_eq!(
            split_text_by_token_limit(text, &tokenizer, max_tokens),
            expected
        );
    }

    #[test]
    fn test_split_text_by_token_limit_if_more_than_limit() {
        let text = "Here is some sample code:\nfn example() {\n    println!(\"Hello, world!\");\n}\n// This is a comment";
        let tokenizer = cl100k_base().expect("Failed to load tokenizer");
        let max_tokens = 4;

        let expected = [
            SubChunkData {
                text: "Here is some sample code:".to_string(),
                token_count: 7,
                line_offset: 0,
            },
            SubChunkData {
                text: "fn example() {".to_string(),
                token_count: 5,
                line_offset: 1,
            },
            SubChunkData {
                text: "    println!(\"Hello, world!\");".to_string(),
                token_count: 8,
                line_offset: 2,
            },
            SubChunkData {
                text: "}".to_string(),
                token_count: 2,
                line_offset: 3,
            },
            SubChunkData {
                text: "// This is a comment".to_string(),
                token_count: 6,
                line_offset: 4,
            },
        ];

        assert_eq!(
            split_text_by_token_limit(text, &tokenizer, max_tokens),
            expected
        );
    }
}
