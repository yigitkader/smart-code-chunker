extern crate core;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::sync::{LazyLock, Mutex};
use tiktoken_rs::cl100k_base;
use tree_sitter::{Parser as TreeParser, Query, QueryCursor, Tree};
use tree_sitter_rust::language;

#[derive(Debug, Serialize, Deserialize)]
struct ChunkData {
    file_path: String,
    chunk_type: String,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    token_count: usize,
    code: String,
}

static TREE_PARSER: LazyLock<Mutex<TreeParser>> = LazyLock::new(|| {
    let mut parser = TreeParser::new();
    parser
        .set_language(tree_sitter_rust::language())
        .expect("Failed to set language");
    Mutex::new(parser)
});

pub fn tree_parse(content: &str) -> Result<Tree> {
    let mut parser = TREE_PARSER
        .lock()
        .map_err(|e| anyhow!("Parser lock poisoned: {}", e))?;
    parser.reset();
    parser.parse(content, None).with_context(|| {
        format!(
            "Failed to parse tree form: {}",
            content.chars().take(50).collect::<String>()
        )
    })
}

pub fn find_chunks<'a>(
    content: &'a str,
    query_code: &str,
) -> Result<Vec<(&'a str, usize, usize, usize, usize)>> {
    let tree = tree_parse(&content)?;
    let mut cursor = QueryCursor::new();
    let query = Query::new(language(), query_code).expect("Query creation failed");
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut response = Vec::new();
    println!(" Found Chunks (Functions):");
    for m in matches {
        for capture in m.captures {
            let node = capture.node;
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let start_line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;
            let chunk_text = &content[start_byte..end_byte];
            response.push((chunk_text, start_byte, end_byte, start_line, end_line));
        }
    }

    Ok(response)
}

fn is_fit_extension(ext: Option<&OsStr>) -> bool {
    match ext {
        Some(os_str) => match os_str.to_str() {
            Some("rs") => true,
            _ => false,
        },
        None => false,
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value = "output.jsonl")]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("--- '{}' folder is searching ---\n", args.path);

    let _ = fs::write(&args.output, "")
        .with_context(|| format!("could not create output file: {}", args.output))?;
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(&args.output)
        .context(format!("could not open output file: {}", args.output))?;

    let walker = WalkBuilder::new(&args.path)
        .standard_filters(true) // Ignore .gitignore and other ignore files
        .build();

    let tokenizer = cl100k_base().context("Failed to create tokenizer")?;
    let mut total_tokens = 0;
    let mut total_chunks = 0;

    for result in walker {
        if let Ok(entry) = result {
            let path_buf = entry.path();
            if path_buf.is_file() && is_fit_extension(path_buf.extension()) {
                println!("\n📂 File: {}", path_buf.display());

                let content = match fs::read_to_string(path_buf) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", path_buf.display(), e);
                        continue;
                    }
                };

                let query_code = "(function_item) @function";

                let chunks = find_chunks(&content, query_code)?;

                for (chunk_text, start_byte, end_byte, start_line, end_line) in chunks {
                    let token_count = tokenizer.encode_with_special_tokens(chunk_text).len();
                    total_tokens += token_count;
                    total_chunks += 1;

                    let chunk_data = ChunkData {
                        file_path: path_buf.display().to_string(),
                        chunk_type: "function".to_string(),
                        start_byte: start_byte,
                        end_byte: end_byte,
                        start_line: start_line,
                        end_line: end_line,
                        token_count: chunk_text.split_whitespace().count(),
                        code: chunk_text.to_string(),
                    };

                    let json_line = serde_json::to_string(&chunk_data)?;
                    writeln!(output_file, "{}", json_line).with_context(|| {
                        format!("could not write to output file: {}", args.output)
                    })?;
                }
                print!(".");
                std::io::stdout().flush()?;
            }
        }
    }

    println!(
        "\n\n✅ Operation completed!\n📁 File: '{}'\n🧩 Total chunk: {}\n🔢 Total Token: {}",
        args.output, total_chunks, total_tokens
    );
    Ok(())
}
