# smart-code-chunker

A CLI tool that splits your codebase into chunks for RAG or semantic search. Instead of blindly splitting by line count, it uses tree-sitter to parse the AST and extracts meaningful blocks like functions, structs, and classes. Chunks can be further split if they exeed the token limit.

## Requirements

- Rust (install via rustup: https://rustup.rs)
- Git must be installed if you use the `--since` flag

## Installation

```bash
git clone https://github.com/<username>/smart-code-chunker.git
cd smart-code-chunker
cargo build --release
```

The binary ends up in `target/release/smart-code-chunker`. You can add it to your PATH or just run with `cargo run --release --`.

## Usage

**Scan entire project, write to output.jsonl:**

```bash
cargo run --release -- -p /path/to/project
```

**Only process files changed since last commit** (handy for incremental indexing in CI):

```bash
cargo run --release -- -p /path/to/project --since HEAD~1
```

**Custom output file and token limit:**

```bash
cargo run --release -- -p ./src -o chunks.jsonl -m 500
```

**Arguments:**

| Param | Short | Description |
|-------|-------|-------------|
| `--path` | `-p` | Directory to scan |
| `--output` | `-o` | Output file (default: output.jsonl) |
| `--since` | - | Only files changed since this commit (e.g. HEAD~1, main) |
| `--max-chunk-tokens` | `-m` | Max tokens per chunk (default: 800) |
| `--verbose` / `-v` | - | Log level |

## Supported languages

- Rust (.rs)
- Python (.py)

To add another language that has a tree-sitter grammar, implement the `LanguageDriver` trait. See `lang_driver.rs`.

## Output format

Each line is a single JSON object (JSONL). Chunks contain:

- `id` — SHA256 hash (generated from file path + content + index)
- `file_path` — Source file
- `language` — Rust, Python, etc.
- `chunk_type` — function_item, struct_item, class_definition, etc.
- `chunk_name` — Name of the function/struct/class
- `context` — Parent hierachy (e.g. `mod(utils) > impl(MyStruct)`)
- `signature` — First line (function signature etc.)
- `comment` — Docstring/comment directly above the block
- `code` — Full code of the block
- `start_line`, `end_line` — Line range
- `token_count` — cl100k_base token count

Large blocks get split into sub-chunks when they exeed the limit. Each sub-chunk has its own `start_line` and `end_line`.

## How it works

1. If `--since` is given, runs `git diff --name-only` to get changed files. Otherwise walks the directory with the `ignore` crate (respects .gitignore).
2. Parses each file with tree-sitter, finds chunk nodes based on the langauge driver.
3. Chunks are split by token count (tiktoken cl100k_base).
4. Paralel processing via rayon, writer runs in a separate thread dumping JSONL.

## Contributing

PRs welcome. To add a new language, implement a driver in `lang_driver.rs` and wire it into `get_driver`.

## License

MIT
