Here is the English version of the README.md. It highlights the semantic parsing, multi-threading, and Git integration features of your project.

```markdown
# Smart Code Chunker 🧠✂️

**Smart Code Chunker** is a high-performance, semantic code chunking tool designed for RAG (Retrieval-Augmented Generation) and AI-based code search systems. Unlike plain text splitters, it analyzes the code's AST (Abstract Syntax Tree) to extract classes, functions, and structs without breaking their logical boundaries.

## 🌟 Key Features

* 🌲 **Semantic Chunking:** Uses `tree-sitter` to understand the syntactic structure of the code and splits it into logical blocks (class, struct, impl, function).
* ⚡ **High Performance (Multi-threading):** Processes large codebases in seconds using multi-threading powered by the `rayon` crate.
* 📏 **Token Awareness:** Uses `tiktoken-rs` (OpenAI `cl100k_base`) to keep chunks within a specified maximum token limit (default: 800). Large blocks are smartly split into sub-chunks.
* 🐙 **Git Integration (Smart Scan):** Integrates with Git to process only modified files. Using arguments like `--since HEAD~1`, you can target only the recently updated code.
* 🧬 **Rich Context Output:** Extracts the parent hierarchy (e.g., `mod > impl > function`), SHA256 ID, function signature, and preceding docstrings/comments for each chunk, exporting them in `.jsonl` format.

## 🛠️ Supported Languages

The tool currently includes native Tree-sitter drivers for the following languages:
* 🦀 **Rust** (`.rs`)
* 🐍 **Python** (`.py`)

*(Adding new language drivers is as easy as implementing the `LanguageDriver` trait.)*

## 🚀 Installation & Build

You need to have [Rust and Cargo](https://rustup.rs/) installed on your system to build the project.

```bash
# Clone the repository
git clone <repository-url>
cd smart-code-chunker

# Build in release mode (for maximum performance)
cargo build --release

```

## 💻 Usage

You can run the compiled binary or use `cargo run` directly.

### Basic Scan (All Files)

Scans all supported files in the target directory and writes to `output.jsonl`:

```bash
cargo run --release -- --path /path/to/your/project

```

### Git Diff Scan (Only Changed Files)

Process only the files that have changed since a specific commit:

```bash
cargo run --release -- --path /path/to/your/project --since HEAD~1

```

### Custom Output and Token Limit

Specify a different output file and adjust the token size limit for GPT-3.5/GPT-4:

```bash
cargo run --release -- --path /path/to/project --output chunks.jsonl --max-chunk-tokens 500

```

### CLI Arguments

* `-p, --path <PATH>`: The target folder path to scan.
* `-o, --output <OUTPUT>`: Output file name (Default: `output.jsonl`).
* `--since <SINCE>`: Scans only the files changed since the specified commit (e.g., `HEAD~1`, `main`).
* `-m, --max-chunk-tokens <MAX>`: Maximum number of tokens per chunk (Default: `800`).

## 📄 Output Format (JSONL)

The output file (`.jsonl`) contains rich metadata ready to be consumed by LLMs and Vector Databases. Each line is a valid JSON object:

```json
{
  "id": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "file_path": "/Users/dream/RustroverProjects/project/src/main.rs",
  "language": "Rust",
  "chunk_type": "function_item",
  "chunk_name": "process_data",
  "context": "mod(utils) > impl(DataProcessor)",
  "signature": "pub fn process_data(input: &str) -> Result<()> {",
  "comment": "/// Processes the incoming string and returns a result.",
  "code": "pub fn process_data(input: &str) -> Result<()> {\n    // ... \n}",
  "start_line": 42,
  "end_line": 55,
  "token_count": 128
}

```

## 🏗️ Project Architecture

* `main.rs`: Manages CLI arguments, sets up the thread pool, and coordinates file writing.
* `files.rs`: Handles Tree-sitter parsing, AST traversal, and token-based splitting.
* `git.rs`: Detects changed files using the `git diff` command.
* `lang_driver.rs`: Contains Tree-sitter queries and language-specific extraction rules.
* `hash.rs`: Calculates SHA256 hashes using the `sha2` crate for unique chunk IDs.
* `types.rs`: Defines core data structures like `ChunkData`.
