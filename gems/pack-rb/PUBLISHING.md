# Publishing `pack-rb`

## Name choice

As of April 30, 2026, `pack` is already taken on RubyGems.org, so the distributable gem name here is `pack-rb`.

## Release flow

1. Tag a release like `v0.1.1`.
2. The `release` workflow builds and uploads release assets named:
   - `pack-x86_64-unknown-linux-gnu`
   - `pack-x86_64-pc-windows-msvc.exe`
   - `pack-aarch64-apple-darwin`
3. The same workflow publishes a `SHA256SUMS` file alongside the binaries.
4. After the GitHub Release is live, the workflow builds `pack-rb-0.1.1.gem` and pushes it to RubyGems.org.
5. End users run:

   ```bash
   gem install pack-rb
   pack --help
   ```

## Credentials

The workflow expects a `RUBYGEMS_API_KEY` repository secret.

## Integrity

The wrapper now downloads `SHA256SUMS` from the release and verifies the binary checksum before installing it.

For local development or custom mirrors, set `PACK_RB_SKIP_CHECKSUM=1` only if your custom download source does not publish a matching `SHA256SUMS` file.
