# Fuzzing

Fuzz targets for ratiomaster-core using [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) (libFuzzer).

## Setup

```sh
cargo install cargo-fuzz
```

## Running

```sh
# Fuzz the bencode decoder
cargo fuzz run bencode_decode

# Fuzz the torrent parser
cargo fuzz run torrent_parse
```

## Options

```sh
# Run with a time limit (seconds)
cargo fuzz run bencode_decode -- -max_total_time=60

# Run with multiple jobs in parallel
cargo fuzz run bencode_decode --jobs 4
```

Corpus files are stored in `fuzz/corpus/<target>/`.
