# Pack

Fast Ruby package management using the RubyGems registry.

Pack is a local developer tool from [Piper](https://piper.software). It is designed to replace the painful parts of `gem` and `bundle` with faster installs, clearer errors, and better dependency visibility.

## Commands

```sh
pack install     # Install missing gems
pack add rails   # Add a gem to Gemfile
pack exec rails s  # Execute a command
pack why nokogiri  # Explain why a gem is installed
pack doctor     # Diagnose the local Ruby project
```

## Status

Pack is early. The first versions prioritize compatibility with existing Ruby projects.

## Installation

```sh
cargo build --release
./target/release/pack --version
```
