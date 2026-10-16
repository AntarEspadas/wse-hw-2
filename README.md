# Query Processor

Requires stable Rust. The web interface also requires Node.js 24, pnpm, and
Docker Compose.

## Index and search


```bash
mkdir -p output
cargo run --release --bin indexer -- data/collection.tsv
cargo run --release --bin merger -- output
cargo run --release --bin query
```

## Web interface

Install SQLx CLI, then set up the database and apply migrations:

```bash
cargo install sqlx-cli
docker compose up -d --wait db
cd web
cp .env.example .env
cargo sqlx migrate run
```

Import the collection once, then build the frontend and start the backend, still
from `web/`:

```bash
cargo run --release --bin update_db
pnpm install
pnpm generate
cargo run --release --bin web
```

## CLI help

Run `cargo run --release --bin <name> -- --help` to display these locally.

### indexer --help

```text
Usage: indexer [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input TSV file [default: ./data/collection.tsv]

Options:
  -o, --out <OUT>                Output folder for files [default: ./output/]
  -w, --workers <WORKERS>        Number of worker threads [default: 8]
  -c, --chunk-size <CHUNK_SIZE>  Chunk size in bytes [default: 100000000]
      --write-txt                Enables writing output in plaintext format
      --skip-bin                 Disables writing output in binary format
  -h, --help                     Print help
  -V, --version                  Print version
```

### merger --help

```text
Usage: merger [OPTIONS] [INPUT_DIR]

Arguments:
  [INPUT_DIR]  Directory containing partial index files [default: ./output]

Options:
  -o, --out <OUT>                  Directory where merged index files will be written [default: ./output]
  -i, --input-count <INPUT_COUNT>  Number of partial index files that will be simultaneously opened for merging [default: 4]
  -b, --buffer-size <BUFFER_SIZE>  Number of index entries that will be kept in memory for each file [default: 100]
  -h, --help                       Print help
  -V, --version                    Print version
```

### query --help

```text
Usage: query [OPTIONS]

Options:
  -i, --index-dir <INDEX_DIR>
          Directory containing index files (index.bin, lexicon.bin) [default: ./output]
  -t, --top-results <TOP_RESULTS>
          Number of top results to be shown for each query [default: 10]
  -s, --score-threshold <SCORE_THRESHOLD>
          Cutoff score for results [default: 5]
  -h, --help
          Print help
  -V, --version
          Print version
```
