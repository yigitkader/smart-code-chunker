extern crate core;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt::Pointer;
use std::fs;
use std::io::Write;
use std::sync::{LazyLock, Mutex};
use tiktoken_rs::cl100k_base;
use tree_sitter::{Language, Node, Parser as TreeParser, Query, QueryCursor, Tree};

#[derive(Debug, Clone, Copy)]
enum SupportedLanguage {
    Rust,
    Python,
}
#[derive(Debug, Serialize, Deserialize)]
struct ChunkData {
    file_path: String,
    language: String,
    chunk_type: String,
    context: String,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    token_count: usize,
    code: String,
}

static TREE_PARSER: LazyLock<Mutex<TreeParser>> = LazyLock::new(|| {
    let parser = TreeParser::new();
    Mutex::new(parser)
});

pub fn tree_parse(content: &str, language: Language) -> Result<Tree> {
    let mut parser = TREE_PARSER
        .lock()
        .map_err(|e| anyhow!("Parser lock poisoned: {}", e))?;

    parser.set_language(language)?;
    parser.reset();

    parser.parse(content, None).with_context(|| {
        format!(
            "Failed to parse tree form: {}",
            content.chars().take(50).collect::<String>()
        )
    })
}

fn extract_name_from_node<'a>(node: &Node, content: &'a str) -> Option<&'a str> {
    if let Some(name_node) = node.child_by_field_name("name") {
        let start = name_node.start_byte();
        let end = name_node.end_byte();
        return Some(&content[start..end]);
    }

    if node.kind() == "impl_item" {
        if let Some(type_node) = node.child_by_field_name("type") {
            let start = type_node.start_byte();
            let end = type_node.end_byte();
            return Some(&content[start..end]);
        }
    }
    None
}

pub fn find_chunks<'a>(
    content: &'a str,
    language: Language,
    query_code: &str,
) -> Result<Vec<(&'a str, &'a str, String, usize, usize, usize, usize)>> {
    let tree = tree_parse(&content, language)?;
    let mut cursor = QueryCursor::new();
    let query = Query::new(language, query_code).expect("Query creation failed");
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut response = Vec::new();
    println!(" Found Chunks (Functions):");
    for m in matches {
        for capture in m.captures {
            let node = capture.node;

            let mut context_parts: Vec<String> = Vec::new();
            let mut parent = node.parent();

            while let Some(p) = parent {
                let kind = p.kind();
                if kind.contains("class")
                    || kind.contains("struct")
                    || kind.contains("impl")
                    || kind.contains("mod")
                {
                    if let Some(name) = extract_name_from_node(&p, content) {
                        let clean_kind = kind.replace("_item", "").replace("_definition", "");
                        context_parts.push(format!("{}({})", clean_kind, name));
                    } else {
                        context_parts.push(kind.to_string());
                    }
                }
                parent = p.parent();
            }
            context_parts.reverse();
            let breadcrumbs = if context_parts.is_empty() {
                "root".to_string()
            } else {
                context_parts.join(" > ")
            };

            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let start_line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;
            let chunk_text = &content[start_byte..end_byte];
            let type_name = node.kind();
            response.push((
                chunk_text,
                type_name,
                breadcrumbs,
                start_byte,
                end_byte,
                start_line,
                end_line,
            ));
        }
    }

    Ok(response)
}

fn get_language_config(ext: &str) -> Option<(SupportedLanguage, Language, &'static str)> {
    // &'static str, -> this means that the string slice has a static lifetime, meaning it is valid for the entire duration of the program. This is often used for string literals, which are stored in the binary and exist for the lifetime of the program.
    match ext {
        "rs" => Some((
            SupportedLanguage::Rust,
            tree_sitter_rust::language(),
            r#"
            [
                (function_item)
                (struct_item)
                (impl_item)
            ] @chunk
            "#,
        )),
        "py" => Some((
            SupportedLanguage::Python,
            tree_sitter_python::language(),
            r#"
            [
                (function_definition)
                (class_definition)
            ] @chunk
            "#,
        )),
        _ => None,
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

            if !path_buf.is_file() {
                continue;
            }
            let ext = match path_buf.extension().and_then(OsStr::to_str) {
                Some(e) => e,
                None => continue,
            };

            if let Some((lang_type, language, query_code)) = get_language_config(ext) {
                println!("\n📂 File: {} ({:?})", path_buf.display(), lang_type);

                let content = match fs::read_to_string(path_buf) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read file {}: {}", path_buf.display(), e);
                        continue;
                    }
                };

                let chunks = find_chunks(&content, language, query_code)?;

                for (
                    chunk_text,
                    type_name,
                    breadcrumbs,
                    start_byte,
                    end_byte,
                    start_line,
                    end_line,
                ) in chunks
                {
                    let token_count = tokenizer.encode_with_special_tokens(chunk_text).len();
                    total_tokens += token_count;
                    total_chunks += 1;

                    let chunk_data = ChunkData {
                        file_path: path_buf.display().to_string(),
                        language: format!("{:?}", lang_type),
                        chunk_type: type_name.to_string(),
                        context: breadcrumbs,
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
