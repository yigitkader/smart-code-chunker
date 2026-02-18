mod files;
mod git;
mod hash;
mod lang_driver;
mod types;

use crate::files::process_file;
use crate::types::ChunkData;
use crate::types::ThreadSafeParser;
use anyhow::{Result, anyhow};
use clap::Parser;
use crossbeam_channel::bounded;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

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

    #[arg(
        short,
        long,
        default_value_t = 800,
        help = "Max tokens per chunk, default is 800 for GPT-4"
    )]
    max_chunk_tokens: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let files: Vec<PathBuf> = files::get_files(&args.path, &args.since)?;
    if files.is_empty() {
        println!("No files found in the specified path.");
        return Ok(());
    }

    let (tx, rx) = bounded::<ChunkData>(1000);

    let output_path = args.output.clone();
    let writer_handle = thread::spawn(move || -> Result<usize> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&output_path)?;

        let mut writer = BufWriter::new(file);
        let mut count = 0;

        for chunk in rx {
            writeln!(writer, "{}", serde_json::to_string(&chunk)?)?;
            count += 1;
            if count % 10 == 0 {
                println!("{} chunks written to file...", count);
            }
        }
        Ok(count)
    });

    println!(
        "Scanning: {} files with thread size: {}",
        files.len(),
        rayon::current_num_threads()
    );

    let parser_pool = Arc::new(ThreadSafeParser::new());
    files.par_iter().for_each(|path| {
        let tx_clone = tx.clone();
        let parser_pool_clone = parser_pool.clone();
        if let Err(err) = process_file(path, &parser_pool_clone, &tx_clone, args.max_chunk_tokens) {
            eprintln!("Error processing file {}: {}", path.display(), err);
        }
    });

    drop(tx);
    let total_chunks = writer_handle
        .join()
        .map_err(|_| anyhow!("Writer thread panicked"))?;
    println!(
        "Processing completed. Total chunks written: {:?}",
        total_chunks
    );
    println!("Output file: {}", args.output);
    Ok(())
}
