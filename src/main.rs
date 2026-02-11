use anyhow::{Context, Result};
use clap::Parser;
use ignore::WalkBuilder;
use std::ffi::OsStr;
use std::fs;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    path: String,
}

fn is_fit_extension(ext: Option<&OsStr>) -> bool {
    match ext {
        Some(os_str) => match os_str.to_str() {
            Some("rs") => true,
            Some("md") => true,
            Some("txt") => true,
            _ => false,
        },
        None => false,
    }
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
                let line_count = content.lines().count();
                println!("\n line count {}", line_count);
                println!("👀 Preview: ");
                for (i, line) in content.lines().take(3).enumerate() {
                    println!("      {}: {}", i + 1, line);
                }
            }
        }
    }

    Ok(())
}
