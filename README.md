# WebCrawler

A small Rust web crawler starter project.

## Run

```powershell
cargo run -- https://www.rust-lang.org
```

## Structure

- `src/main.rs`: command-line entry point.
- `src/lib.rs`: library exports.
- `src/config.rs`: crawler configuration.
- `src/error.rs`: shared error type.
- `src/crawler/`: HTTP fetching and page model.
- `src/parser/`: HTML parsing helpers.
- `src/storage/`: placeholder for persistence.
- `config/default.toml`: default runtime settings.
