use anyhow::{Context, Result, anyhow};
use clap::Parser;
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::fs;
use std::sync::{LazyLock, Mutex};
use tree_sitter::{Parser as TreeParser, Query, QueryCursor, Tree};
use tree_sitter_rust::language;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    path: String,
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
    parser.parse(content, None).with_context(|| {
        format!(
            "Failed to parse tree form: {}",
            content.chars().take(50).collect::<String>()
        )
    })
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

pub fn find_chunks<'a>(content: &'a str, query_code: &str) -> Result<Vec<(&'a str, usize, usize)>> {
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
            let chunk_text = &content[start_byte..end_byte];
            response.push((chunk_text, start_byte, end_byte));
        }
    }

    Ok(response)
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("--- '{}' folder is searching ---\n", args.path);

    let walker = WalkBuilder::new(&args.path)
        .standard_filters(true) // Ignore .gitignore and other ignore files
        .build();

    for result in walker {
        if let Ok(entry) = result {
            let path_buf = entry.path();
            if path_buf.is_file() && is_fit_extension(path_buf.extension()) {
                println!("\n📂 File: {}", path_buf.display());

                let content = fs::read_to_string(path_buf)
                    .with_context(|| format!("could not read file: {}", path_buf.display()))?;

                let query_code = "(function_item) @function";

                let chunks = find_chunks(&content, query_code)?;

                for chunk in chunks {
                    let (chunk_text, start_byte, end_byte): (&str, usize, usize) = chunk;
                    println!("      ---------------------------------");
                    println!("      🔹 Chunk ({} - {} bytes)", start_byte, end_byte);
                    let first_line = chunk_text.lines().next().unwrap_or("");
                    println!("      Code: {} ...", first_line);
                }
            }
        }
    }

    Ok(())
}
