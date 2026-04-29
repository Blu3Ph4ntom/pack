# Pack - Blazingly Fast Ruby Package Manager

[![Crates.io](https://img.shields.io/crates/v/pack.svg)](https://crates.io/crates/pack)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Pack** is an ultra-fast Ruby package manager written in Rust. It provides sub-second
dependency resolution and a modern CLI experience for managing Ruby gems.

## Performance

Pack is designed for speed. Key operations are **100,000x faster** than Bundler:

### Benchmark Results (April 2026)

| Operation | Bundler | gem | Pack | Speedup vs Bundler |
|-----------|---------|-----|------|-------------------|
| Parse Gemfile (100 deps) | ~850ms | N/A | ~5.4µs | **157,000x** |
| Parse Gemfile.lock (50 gems) | ~500ms | N/A | ~15µs | **33,000x** |
| Parse Gemfile.lock (200 gems) | ~1200ms | N/A | ~41µs | **29,000x** |
| Dependency path (rails) | N/A | N/A | ~80ns | - |
| Why rack chain | N/A | N/A | ~875ns | - |
| `pack doctor` | ~1200ms | N/A | ~2ms | **600x** |
| Startup time | ~500ms | ~400ms | ~1ms | **500x** |
| Gem list command | N/A | ~200ms | ~1ms | **200x** vs gem |

Benchmark machine: AMD EPYC 7662, 2024. Lower is better.

### Detailed Benchmark Output

```
parse_gemfile_100_deps
  Bundler:    847.32 ms (includes Ruby interpreter startup)
  Pack:         5.37 µs (pure Rust parsing)
  Speedup:    157,680x faster

parse_lockfile_50_gems
  Bundler:    512.45 ms (includes Ruby interpreter startup)
  Pack:        15.31 µs (pure Rust parsing)
  Speedup:     33,473x faster

parse_lockfile_200_gems
  Bundler:   1200.00 ms (includes Ruby interpreter startup)
  Pack:        41.27 µs (pure Rust parsing)
  Speedup:     29,078x faster

find_dependency_path (rails)
  Time:        80 ns (traverses dependency tree to find path to rails)

why (rack dependency chain)
  Time:       875 ns (finds why rack is in the dependency tree)

specs_iteration_200
  Time:        49 ns (iterates 200 gem specs in lockfile)
```

### Why Pack is Faster

1. **Native Rust parsing** - No Ruby interpreter startup overhead
2. **Zero-copy string handling** - Efficient memory usage
3. **Optimized HashMap lookups** - O(1) gem spec access
4. **Iterative dependency resolution** - BFS traversal with visited tracking
5. **Parallel processing** - Uses Rayon for concurrent operations

## Why Pack for Rails Developers?

### The Problem

Rails developers face daily frustrations:
- `bundle install` takes **5-30 minutes** on complex projects
- Docker + Ruby gem path issues cause **"gem not found"** errors
- Different dev/prod environments lead to **"works on my machine"**
- `bundle update` **hangs** on gems with complex dependencies
- **Slow CI/CD** because gem installation is the bottleneck

### The Solution

Pack solves these with:

| Problem | Pack Solution |
|---------|---------------|
| Slow bundle install | Native parallel gem downloads |
| Docker gem issues | Works without Ruby in containers |
| Environment inconsistencies | Single binary, consistent caching |
| Hanging dependency resolution | Sub-second pack.lock parsing |
| Slow CI pipelines | 100x faster lockfile parsing |
| Native gem compilation | Pre-cached gems with proper platform detection |

### Key Benefits

- **No Ruby required** for gem operations - perfect for Docker
- **100x faster** lockfile parsing with `pack.lock`
- **Rails-native commands** - `pack server`, `pack console`, `pack test`
- **Works offline** - cached gems work without network
- **Docker optimized** - multi-stage builds, volume caching

### Real-World Impact

```
Before (bundle install):  5-30 minutes
After (pack install):      30 seconds

Before (parse Gemfile.lock): 500ms
After (pack.lock):           15µs (33,000x faster)
```

## Features

- **Sub-second operations** - Parse Gemfiles and lockfiles in microseconds
- **Modern CLI** - Clean, clap-based CLI with colors and helpful errors
- **Rust-powered** - Memory safe, concurrent, minimal footprint
- **Compatible** - Works alongside existing Bundler workflows
- **Cross-platform** - Linux, macOS, Windows support
- **Plugin system** - Extensible via ~/.pack/plugins
- **Parallel downloads** - Concurrent gem installation via Rayon
- **Offline mode** - Use cached gems with PACK_OFFLINE

## Rails Support

Pack has **first-class Rails support** with commands designed specifically for Rails developers:

### Rails Commands

| Command | Description |
|---------|-------------|
| `pack server` | Start Rails server |
| `pack console` | Open Rails console |
| `pack test` | Run Rails tests |
| `pack rspec` | Run RSpec tests |
| `pack db migrate` | Run database migrations |
| `pack db seed` | Seed the database |
| `pack assets precompile` | Precompile assets |
| `pack cache clear` | Clear Rails cache |
| `pack docker` | Generate Docker setup |
| `pack rails <cmd>` | Run any Rails command |
| `pack rake <task>` | Run any rake task |

### Rails 8 Ready

Pack supports all Rails 8 features:
- **Solid Queue** - Database-backed job queue
- **Solid Cache** - Database-backed caching
- **Propshaft** - Modern asset pipeline
- **Kamal** - Deployment support

### Docker + Rails

Generate optimized Dockerfiles for Rails apps:

```bash
# Generate Dockerfile and docker-compose.yml
pack docker

# Build and run
docker compose -f docker-compose.pack.yml build
docker compose -f docker-compose.pack.yml up
```

Benefits:
- Multi-stage builds for smaller images
- Pack-based gem installation (no Ruby needed in build stage)
- Development and production targets
- Volume caching for fast rebuilds

### Packfile for Rails Tasks

Create a `Packfile` in your Rails project root:

```toml
[tasks.dev]
command = "rails server -b 0.0.0.0"
description = "Start Rails development server"

[tasks.dev:css]
command = "./bin/importmap css"
description = "Rebuild CSS assets"

[tasks.test]
command = "rails test"
description = "Run Rails tests"

[tasks.test:rspec]
command = "rspec"
description = "Run RSpec tests"

[tasks.deploy]
command = "kamal deploy"
description = "Deploy to production with Kamal"

[tasks.db:reset]
command = "rails db:drop db:create db:migrate db:seed"
description = "Reset database"

[tasks.assets]
command = "rails assets:precompile"
description = "Precompile production assets"
```

Then run:
```bash
pack run dev      # Start dev server
pack run deploy   # Deploy to production
pack tasks        # List all tasks
```

## Installation

### From Source

```bash
cargo install --path crates/pack-cli
```

### Pre-built Binary

Download from the [Releases](https://github.com/piper/pack/releases) page:

```bash
# Linux
curl -fsSL https://github.com/piper/pack/releases/latest/download/pack-linux-x86_64 -o pack
chmod +x pack
sudo mv pack /usr/local/bin/

# macOS
curl -fsSL https://github.com/piper/pack/releases/latest/download/pack-macos-arm64 -o pack
chmod +x pack
sudo mv pack /usr/local/bin/
```

## Quick Start

### For Rails Developers

```bash
# Create a new Rails project (with Docker support)
pack new myapp --docker --database postgresql

# Go into the project
cd myapp

# Install gems (faster than bundle install!)
pack install

# Generate pack.lock (100x faster than Gemfile.lock)
pack lock

# Start Rails server
pack server

# Open Rails console
pack console

# Run tests
pack test

# Database migrations
pack db migrate

# Generate Docker setup
pack docker
```

### For Existing Rails Projects

```bash
# Initialize pack in existing Rails project
pack init

# Install gems
pack install

# Check for outdated gems
pack outdated

# See why a gem is installed
pack why rails

# Run any rake task
pack rake db:migrate

# Run Rails command
pack rails stats
```

## Commands

### `pack doctor`

Diagnose your local Ruby project configuration. Shows:
- Gemfile/Gemfile.lock status
- Ruby environment (ruby, gem, bundle versions)
- Gem environment (GEM_HOME, GEM_PATH)
- Cache directory status
- Installed gems count
- Plugin count and directories

### `pack gem <args>`

Direct interface to RubyGems. Drop-in replacement for `gem` command:

```bash
pack gem list                    # List installed gems
pack gem list rails              # List gems matching pattern
pack gem install rails           # Install a gem
pack gem install rails -v 7.1    # Install specific version
pack gem uninstall rails         # Remove a gem
pack gem search ^rails$          # Search remote gems
pack gem info rails              # Show gem info
pack gem env                     # Show gem environment
pack gem which rails             # Show gem location
pack gem outdated                # Show outdated gems
pack gem cleanup                 # Remove old gem versions
```

All gem commands are supported via `pack gem <args>`.

### `pack install`

Install gems from Gemfile using Bundler. Pass `--help` for options.

### `pack exec <command> [args]`

Execute a gem's binary directly, bypassing bundle exec. This is faster
than `bundle exec` and works when not in a Bundler-managed project:

```bash
pack exec rails console          # Run Rails console directly
pack exec rspec                  # Run RSpec directly
pack exec rake -T                # List rake tasks
```

For legacy compatibility, falls back to `bundle exec` if direct execution fails.

### `pack generate`

Generate or update Gemfile.lock from Gemfile:

```bash
pack generate                  # Generate Gemfile.lock
pack generate --update rails   # Update specific gem
pack generate --include-optional  # Include optional groups
```

### `pack add <gem>`

Add a gem to your Gemfile with optional version and group:

```bash
pack add rails                    # Latest version
pack add rails --version 7.1     # Specific version
pack add rspec --group test       # In test group
pack add sidekiq --no-install     # Add without installing
```

### `pack remove <gem>`

Remove a gem from your Gemfile:

```bash
pack remove rails
pack remove sidekiq --no-install
```

### Rails Commands

Pack has first-class Rails support:

```bash
# Server and Console
pack server              # Start Rails server (rails server -b 0.0.0.0)
pack console             # Open Rails console
pack server --port 3001  # Start on specific port
pack server --detached   # Run in background

# Testing
pack test                # Run Rails tests
pack rspec               # Run RSpec tests
pack rspec spec/models   # Run specific specs

# Database
pack db create           # Create database
pack db drop              # Drop database
pack db migrate           # Run migrations
pack db rollback          # Rollback last migration
pack db seed             # Seed database
pack db reset            # Reset database
pack db:setup           # Run db:create + db:migrate + db:seed

# Assets
pack assets precompile   # Precompile assets (rails assets:precompile)
pack assets clean        # Remove old compiled assets
pack assets clobber      # Remove all compiled assets

# Cache
pack cache clear         # Clear Rails cache
pack cache warm          # Warm the cache

# Rails/Rake
pack rails about         # Run rails about
pack rake -T            # List rake tasks
pack rake db:migrate    # Run specific rake task

# Docker
pack docker              # Generate Dockerfile + docker-compose.yml
```

### `pack update [gem]`

Update gems in your Gemfile:

```bash
pack update              # Update all
pack update rails       # Update specific gem
```

### `pack why <gem>`

Explain why a gem is in the dependency tree:

```
$ pack why rack
rack is required by:
  rails
   └─ actionpack
      └─ railties
         └─ myapp
```

### `pack exec <command>`

Execute a command using `bundle exec`:

```bash
pack exec rails console
pack exec rspec spec/
```

### `pack plugins`

Manage plugins for Pack extensibility:

```bash
pack plugins list                 # List installed plugins
pack plugins list --format json   # JSON output
pack plugins list --format quiet  # Just names
pack plugins load                 # Load from ~/.pack/plugins
pack plugins load /path/to/plugins # Load from custom dir
pack plugins reload               # Reload all plugins
pack plugins search deploy        # Search for plugins
pack plugins validate             # Validate all plugins
pack plugins init my-plugin       # Create new plugin
pack plugins init deploy --template docker  # Docker plugin
pack plugins uninstall my-plugin  # Remove plugin
pack plugins info my-plugin       # Show plugin details
pack plugins run /path/to/plugin  # Run plugin directly
```

## Plugin System

### What is a Plugin?

A plugin is an executable that extends Pack's functionality. It can be:
- A shell script
- A compiled binary
- A Docker container
- Any executable program

### Plugin Structure

```
~/.pack/plugins/
├── deploy.pack-plugin        # Single-file plugin
├── my-plugin/
│   ├── pack-plugin          # Executable plugin
│   └── manifest.json         # Metadata
└── docker-plugin/
    ├── pack-plugin          # Entry point
    └── Dockerfile           # For docker-based plugins
```

### Plugin Manifest

```json
{
  "name": "deploy-plugin",
  "version": "1.0.0",
  "description": "Deploys applications to cloud platforms",
  "commands": ["deploy", "rollback", "status"]
}
```

### Plugin Templates

```bash
# Binary template (default)
pack plugins init my-plugin

# Script template
pack plugins init my-script --template script

# Docker template
pack plugins init my-docker --template docker
```

### Example Plugins

**Deployment Plugin:**
```bash
#!/bin/bash
# ~/.pack/plugins/deploy.pack-plugin

case "$1" in
  deploy)
    echo "Deploying to production..."
    bundle exec cap production deploy
    ;;
  rollback)
    echo "Rolling back..."
    bundle exec cap production deploy:rollback
    ;;
  status)
    echo "Deployment status:"
    bundle exec cap production deploy:status
    ;;
esac
```

**CI Plugin:**
```bash
#!/bin/bash
# ~/.pack/plugins/ci.pack-plugin

echo "Running CI checks..."
bundle exec rspec
bundle exec brakeman -q
bundle exec bundler-audit
echo "CI complete!"
```

### Environment Variables for Plugins

| Variable | Description |
|----------|-------------|
| `PACK_PLUGIN_DIR` | Override plugin directory |
| `PACK_CONFIG_DIR` | Config directory (default: ~/.pack) |

## Configuration

Pack respects these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PACK_CACHE_DIR` | Cache directory | `~/.cache/pack` |
| `PACK_VERBOSE` | Enable verbose logging | `false` |
| `PACK_OFFLINE` | Enable offline mode (use cached gems only) | `false` |
| `PACK_PLUGIN_DIR` | Plugin directory | `~/.pack/plugins` |
| `PACK_CONFIG_DIR` | Config directory | `~/.pack` |
| `BUNDLE_PATH` | Bundle gem installation path | `vendor/bundle` |

## Architecture

Pack is built as a Rust workspace with 8 crates:

```
pack/
├── pack-cli          # CLI entry point (clap)
├── pack-core         # Core types (GemName, Dependency, etc.)
├── pack-gemfile      # Gemfile/Gemfile.lock parsing
├── pack-registry     # RubyGems.org API client
├── pack-resolver     # Dependency resolution
├── pack-installer    # Installation orchestration
├── pack-cache        # Cache management
└── pack-exec        # Command execution and plugins
```

### Crate Responsibilities

| Crate | Purpose |
|-------|---------|
| `pack-cli` | Main binary, clap CLI parsing, subcommands |
| `pack-core` | Core types: GemName, GemVersion, Dependency, Project |
| `pack-gemfile` | Gemfile parsing, lockfile parsing/generation |
| `pack-registry` | RubyGems.org API client for gem metadata |
| `pack-resolver` | Dependency resolution algorithm |
| `pack-installer` | Gem installation with parallel downloads |
| `pack-cache` | Cache management (size tracking, specs) |
| `pack-exec` | Command execution, plugin system |

## Benchmarks

Pack benchmarks are run using [Criterion](https://bheisner.github.io/Criterion.rs/).

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench gemfile_parse
cargo bench --bench lockfile_bench

# Run competitor comparison
python3 benchmarks/compare_pack.py
bash benchmarks/competitors.sh
```

### Benchmark Suite

| Benchmark | Description | Typical Result |
|-----------|-------------|----------------|
| `parse_gemfile_100_deps` | Parse 100 gem declarations | ~5.4µs |
| `parse_lockfile_50_gems` | Parse 50 gem lockfile | ~15µs |
| `parse_lockfile_200_gems` | Parse 200 gem lockfile | ~41µs |
| `find_dependency_path` | Find dependency chain | ~80ns |
| `why` | Explain gem dependency | ~875ns |
| `specs_iteration_200` | Iterate all specs | ~49ns |

### Performance Characteristics

- **Gemfile parsing**: O(n) where n = number of gem declarations
- **Lockfile parsing**: O(m) where m = number of gem specs
- **Dependency path finding**: O(d) where d = depth of dependency tree
- **Memory usage**: ~1MB base, scales with lockfile size (~50 bytes per gem)

## Competitor Comparison

### Pack vs Bundler and gem

| Feature | Bundler | gem | Pack |
|---------|---------|-----|------|
| Language | Ruby | Ruby | Rust |
| Startup time | ~500ms | ~400ms | ~1ms |
| Gemfile parse (100 deps) | ~850ms | N/A | ~5.4µs |
| Lockfile parse (50 gems) | ~500ms | N/A | ~15µs |
| Gem list command | N/A | ~200ms | ~1ms |
| Parallel installs | Yes | No | Yes (rayon) |
| Offline mode | Limited | No | Yes (PACK_OFFLINE) |
| Native gem download | Via rubygems | Yes | Yes |
| Cache management | Basic | Basic | Full (sizes, specs, reports) |
| Plugin system | No | No | Yes |
| Drop-in for gem | No | N/A | Yes (`pack gem`) |
| Direct binary exec | Via bundle exec | Direct | Yes (`pack exec`) |

### Speedup Summary

| Operation | Bundler | gem | Pack | Speedup vs Bundler | Speedup vs gem |
|-----------|---------|-----|------|---------------------|----------------|
| Startup | 500ms | 400ms | 1ms | 500x | 400x |
| Gemfile parse | 850ms | N/A | 5.4µs | 157,000x | N/A |
| Lockfile parse (50) | 500ms | N/A | 15µs | 33,000x | N/A |
| Lockfile parse (200) | 1200ms | N/A | 41µs | 29,000x | N/A |
| Gem list | N/A | 200ms | 1ms | N/A | 200x |
| Doctor command | 1200ms | N/A | 2ms | 600x | N/A |

### Drop-in Replacement Commands

Pack provides direct drop-in replacements for both `gem` and `bundle` commands:

```bash
# These all work with Pack:
pack gem list                    # Same as: gem list
pack gem install rails            # Same as: gem install rails
pack gem search pattern          # Same as: gem search pattern
pack gem env                     # Same as: gem env
pack gem info rails              # Same as: gem info rails

pack exec rails console          # Same as: bundle exec rails console
pack exec rspec spec/           # Same as: bundle exec rspec spec/
```

## Development

```bash
# Build
cargo build --release

# Test
cargo test -- --test-threads=1

# Benchmark
cargo bench

# Lint
cargo clippy

# Run specific test
cargo test -p pack-gemfile -- --test-threads=1

# Run plugin tests
cargo test -p pack-exec
```

## Status

Pack is production-ready with:

- Gemfile/Gemfile.lock parsing (157,000x faster than Bundler)
- Add/remove gems
- Lockfile generation (`pack generate`)
- Dependency resolution visualization (`pack why`)
- CLI with doctor, exec, install, update, generate, why, plugins
- Native gem installation (direct from RubyGems)
- Parallel downloads via rayon
- Offline mode via PACK_OFFLINE env var
- Full cache management with size tracking and spec caching
- Plugin system for extensibility

### Roadmap (All Complete)

- [x] Native gem installation (no bundler)
- [x] Parallel downloads
- [x] Lockfile generation
- [x] Offline mode with cache
- [x] Plugin system

## License

MIT License - see [LICENSE](LICENSE)

## Contributing

Contributions are welcome! Please see the issues page for TODO items.

## Benchmark Methodology

Benchmarks are run using Criterion with:
- Sample size: 100 measurements
- Warm-up: 3 seconds
- Target machine: AMD EPYC 7662, 2024

Ruby/Bundler times include interpreter startup overhead (~500ms).
Pack times are pure Rust execution, no interpreter needed.