mod files_fetch;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use ignore::WalkBuilder;

#[derive(Parser, Debug)]
#[command(
    name = "smart-chunker",
    about = "High-performance semantic code chunker for RAG"
)]
struct Args {
    #[arg(short, long, help = "File path for search folder")]
    path: String,

    #[arg(short, long, default_value = "output.jsonl", help = "Output file name")]
    output: String,

    #[arg(long, help = "Scan the folder since this commit (Example: HEAD~1)")]
    since: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let files = files_fetch::get_files(&args.path, &args.since)?;
    println!("Found {} files", files.len());
    anyhow::Ok(())
}
