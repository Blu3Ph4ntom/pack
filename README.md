# Pack

Pack is a Rust-powered Ruby package manager and Rails workflow CLI.

It keeps common gem-facing commands close together, adds project and Rails helpers, and aims to cut the usual Ruby CLI startup drag without changing the way Ruby developers already work.

- Website: https://blu3ph4ntom.github.io/pack/
- Docs: https://blu3ph4ntom.github.io/pack/docs/
- Releases: https://github.com/Blu3Ph4ntom/pack/releases

## Install

### Direct binary (no Ruby required)

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/Blu3Ph4ntom/pack/main/scripts/install.sh | bash
```

```powershell
# Windows PowerShell
powershell -ExecutionPolicy Bypass -c "iwr https://raw.githubusercontent.com/Blu3Ph4ntom/pack/main/scripts/install.ps1 -UseBasicParsing | iex"
```

Both installers verify the binary against `SHA256SUMS` from the GitHub release.

### RubyGems wrapper

```bash
gem install pack-rb
pack --help
```

`pack-rb` installs the `pack` launcher. On first run it downloads the matching Pack binary for the current platform from GitHub Releases and verifies it against `SHA256SUMS`.

`gem install pack-rb` is effectively the last `gem` command you need for installing Pack itself.

### From source

```bash
cargo build --release
target/release/pack --help
```

## What ships today

- RubyGems-compatible commands for install, list, env, uninstall, outdated, cleanup, and global update
- Native RubyGems.org registry reads for search and info
- Gemfile editing with `pack add` and `pack remove`
- Lockfile generation with `pack generate`
- Dependency inspection with `pack why`
- Direct executable dispatch with `pack exec`
- Rails helpers including `pack server`, `pack console`, `pack test`, `pack r-spec`, `pack db`, `pack assets`, `pack cache`, `pack rails`, and `pack rake`
- Task execution through `Packfile`
- `pack update` for project gems and `pack update --global` for globally installed gems
- `pack upgrade` for upgrading Pack through RubyGems or the direct binary installer
- Plugin management through `pack plugins`

## Benchmarks

The repo only advertises numbers that were measured locally or reported directly by Criterion.

### Command comparisons

| Command | Pack | Comparison target |
| --- | --- | --- |
| `pack --version` | `32.20 ms` | `bundle --version` at `368.24 ms` |
| `pack list` | `16.95 ms` | `gem list` at `496.37 ms` |

### Parser timings

| Benchmark | Result | Source |
| --- | --- | --- |
| Gemfile parse (100 deps) | `8.67–8.86 µs` | `cargo bench --bench gemfile_parse` |
| Lockfile parse (50 gems) | `48.45–50.08 µs` | `cargo bench --bench gemfile_parse` |
| Lockfile parse (200 gems) | `91.60–117.35 µs` | `cargo bench --bench lockfile_bench` |

## Release model

Releases are driven from `.github/workflows/release.yml`.

On each version tag, the workflow:

1. runs workspace tests
2. builds the `pack-rb` gem
3. builds platform binaries for Linux, Windows, and macOS
4. publishes a GitHub Release with binaries plus `SHA256SUMS`
5. publishes `pack-rb` to RubyGems

## Local verification

```bash
cargo test --workspace --quiet
cargo build --release -p pack-cli
cd web && npm install && npm run build
cd gems/pack-rb && gem build pack-rb.gemspec
```

## Repository layout

- `crates/pack-cli` — CLI entry point
- `crates/pack-gemfile` — Gemfile and lockfile parsing
- `crates/pack-registry` — RubyGems.org registry access
- `crates/pack-installer` — install orchestration
- `crates/pack-exec` — execution and plugin support
- `gems/pack-rb` — RubyGems wrapper package
- `web/` — website and docs

## License

MIT
