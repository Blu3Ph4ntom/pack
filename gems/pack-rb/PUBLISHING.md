# Publishing `pack-rb`

## Name choice

As of April 30, 2026, `pack` is already taken on RubyGems.org, so the distributable gem name here is `pack-rb`.

## Release flow

1. Tag a release like `v0.1.0`.
2. The `release-binaries` workflow builds and uploads release assets named:
   - `pack-x86_64-unknown-linux-gnu`
   - `pack-x86_64-pc-windows-msvc.exe`
   - `pack-x86_64-apple-darwin`
   - `pack-aarch64-apple-darwin`
3. The `publish-pack-rb` workflow builds `pack-rb-0.1.0.gem` and pushes it to RubyGems.org.
4. End users run:

   ```bash
   gem install pack-rb
   pack --help
   ```

## Credentials

The workflow currently expects a `RUBYGEMS_API_KEY` repository secret.

RubyGems officially recommends unique gem names and supports trusted publishing for GitHub Actions. If you want to eliminate long-lived API keys, migrate this workflow to RubyGems trusted publishing after the gem is created.
