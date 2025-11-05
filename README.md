# trotd

> **Trending repositories of the day** - minimal MOTD CLI

A lightweight, dependency-light CLI tool that prints a Message of the Day (MOTD) with the top trending repositories from GitHub, GitLab, and Gitea.

## Features

- **Multi-provider support**: GitHub, GitLab, Gitea (configurable base URL)
- **Parallel fetching**: Concurrent API calls with configurable timeout
- **Smart caching**: Filesystem-based cache with TTL (XDG-compliant)
- **Flexible configuration**: TOML config, environment variables, CLI flags
- **Language filtering**: Filter repositories by programming language
- **Beautiful output**: Colored terminal output with nerd font icons
- **JSON export**: Optional JSON output for scripting
- **Zero HTML scraping**: Uses only official APIs

## Installation

### From Source

```bash
git clone https://github.com/schausberger/trotd
cd trotd
cargo install --path .
```

### With Nix

```bash
nix build
# or
nix develop  # Enter development shell
```

## Usage

### Basic Usage

```bash
# Show trending repos from all providers
trotd

# Show top 5 repos per provider
trotd --max 5

# Filter by language
trotd --lang rust,go

# Specific providers only (gh=GitHub, gl=GitLab, ge=Gitea)
trotd --provider gh,gl

# JSON output
trotd --json

# Disable cache
trotd --no-cache
```

### Example Output

```
[GH] getzola/zola • Rust • A fast static site generator • ★50 today
[GH] rust-lang/rust • Rust • Empowering everyone to build reliable... • ★120 today
[GL] gitlab-org/gitlab • Ruby • GitLab Community Edition • ★2500 ~
[GE] gitea/gitea • Go • Git with a cup of tea • ★35000 ~
```

**Legend:**
- `[GH]` = GitHub, `[GL]` = GitLab, `[GE]` = Gitea
- `★N today` = Stars gained today
- `★N` = Total stars (when daily stars unavailable)
- `~` = Approximated (not from official trending API)

## Configuration

### Configuration File

trotd looks for configuration in:
1. `~/.config/trotd/trotd.toml` (XDG config directory)
2. `./trotd.toml` (current directory)

Example `trotd.toml`:

```toml
[general]
max_per_provider = 3
timeout_secs = 6
cache_ttl_mins = 60
language_filter = ["rust", "go"]

[providers]
github = true
gitlab = true
gitea = true

[auth]
github_token = ""
gitlab_token = ""
gitea_token = ""

[gitea]
base_url = "https://gitea.com"
```

### Environment Variables

Environment variables override config file settings:

```bash
export TROTD_MAX_PER_PROVIDER=5
export TROTD_LANGUAGE_FILTER="rust,go,python"
export TROTD_GITEA_BASE_URL="https://codeberg.org"
export TROTD_GITHUB_TOKEN="ghp_..."
export TROTD_GITLAB_TOKEN="glpat-..."
export TROTD_GITEA_TOKEN="..."
```

### Command-Line Flags

CLI flags override both config file and environment variables:

```bash
trotd --max 5 --lang rust --provider gh --no-cache --json
```

## Provider Details

### GitHub

- **API**: GitHub trending API (unofficial but reliable)
- **Endpoint**: `https://gh-trending-api.vicary.workers.dev/repositories`
- **Approximated**: No (official trending data)
- **Authentication**: Optional (increases rate limits)

### GitLab

- **API**: GitLab REST API v4
- **Endpoint**: `/api/v4/projects?order_by=created_at`
- **Approximated**: Yes (filters by creation date, sorted by stars)
- **Authentication**: Optional (private repos)

### Gitea

- **API**: Gitea REST API v1
- **Endpoint**: `{base_url}/api/v1/repos/search`
- **Approximated**: Yes (search API with date filtering)
- **Authentication**: Optional
- **Configurable**: Custom base URL (e.g., Codeberg, self-hosted)

## Architecture

```
src/
├── main.rs         # CLI entry point, parallel fetching
├── config.rs       # Configuration (TOML + env + CLI)
├── model.rs        # Repo struct, Provider trait
├── render.rs       # MOTD rendering with colors
├── cache.rs        # Filesystem cache with TTL
├── http.rs         # HTTP client wrapper
└── providers/
    ├── github.rs   # GitHub trending API
    ├── gitlab.rs   # GitLab explore API
    └── gitea.rs    # Gitea search API
```

**Design Philosophy:**
- **Minimal dependencies**: 16 total crates
- **Clean code**: Strict lints (forbid unsafe, clippy pedantic)
- **No HTML scraping**: API-only approach
- **Parallel execution**: FuturesUnordered for concurrent fetching
- **Error resilience**: Partial results on provider failures

## Development

### Prerequisites

- Rust 1.89+ (or use Nix flake)
- cargo-nextest (optional but recommended)

### Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
```

### Testing

```bash
# Run tests
cargo test

# With nextest
cargo nextest run

# Run pre-commit hooks
prek run --all-files
```

### Nix Development Environment

```bash
nix develop

# Available tools:
# - rust-analyzer
# - cargo-nextest
# - cargo-watch
# - prek (pre-commit hooks)
```

### Pre-commit Hooks

Install hooks with prek:

```bash
prek install
```

Hooks run:
1. `cargo fmt --all --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo nextest run` (or `cargo test` if nextest unavailable)

## Project Goals

- **Minimal**: Small binary, few dependencies
- **Fast**: Parallel fetching, smart caching
- **Clean**: Well-tested, documented, maintainable
- **Flexible**: Multiple providers, multiple config methods
- **Reliable**: API-only (no HTML scraping)

## Non-Goals

- HTML scraping (brittle, unreliable)
- Bitbucket, GitKraken, SourceTree support
- Interactive TUI (use as simple MOTD)
- Historical trending data

## Dependencies

**Runtime** (13 crates):
- tokio, reqwest - Async HTTP
- serde, serde_json, toml - Serialization
- clap - CLI parsing
- anyhow, thiserror - Error handling
- dirs - XDG directories
- async-trait - Trait async methods
- colored - Terminal colors
- chrono - Date handling
- futures - Concurrent streams

**Development** (1 crate):
- mockito - HTTP mocking

## License

MIT License - see [LICENSE](LICENSE) file

## Contributing

Contributions welcome! Please:
1. Run `cargo fmt` before committing
2. Ensure `cargo clippy` passes with no warnings
3. Add tests for new features
4. Update README if adding configuration options

## Roadmap

- [ ] Codeberg support (via Gitea provider)
- [ ] SourceHut support (if API available)
- [ ] Filtering by stars threshold
- [ ] Excluding specific topics
- [ ] Custom icon/color schemes
- [ ] Shell completion scripts
