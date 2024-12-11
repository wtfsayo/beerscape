# BeerScape Recipe Downloader

A high-performance concurrent recipe downloader built in Rust that efficiently downloads and manages beer recipes.

## Features

- Concurrent downloading with configurable batch sizes
- Progress tracking with ETA and success rates
- Duplicate detection and skipping
- Automatic retry mechanism
- Detailed download statistics
- Smart rate limiting to prevent server overload

## Configuration

Key constants can be adjusted in the code:

```rust
const TOTAL_RECIPES_TARGET: usize = 10_000;
const MIN_RECIPE_ID: u32 = 1;
const MAX_RECIPE_ID: u32 = 4_000_000;
const CONCURRENT_REQUESTS: usize = 10;
```

## Dependencies

- tokio (async runtime)
- reqwest (HTTP client)
- indicatif (progress bars)
- glob (file pattern matching)
- rand (random number generation)

## Usage

1. Clone the repository
2. Run with cargo:

```bash
cargo run --release
```

The program will:
- Create a `recipes` directory if it doesn't exist
- Scan for existing recipes
- Download new recipes until target is reached
- Display progress and statistics

## Output

The program provides detailed statistics including:
- Number of existing recipes
- Newly downloaded recipes
- Failed attempts
- Overall success rate
- Progress bar with ETA

## License

MIT
