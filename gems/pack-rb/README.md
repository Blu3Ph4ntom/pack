# pack-rb

`pack-rb` installs a `pack` executable through RubyGems and delegates to the Pack Rust binary.

```bash
gem install pack-rb
pack --help
```

On first run, the wrapper detects the current platform, downloads the matching Pack release asset from GitHub Releases, stores it in the user cache, and execs it.

Environment variables:

- `PACK_RB_VERSION` forces a specific Pack version instead of the gem version.
- `PACK_RB_GITHUB_REPOSITORY` overrides the default release repository.
- `PACK_RB_DOWNLOAD_BASE_URL` overrides the release download base URL completely.
- `PACK_RB_INSTALL_DIR` overrides the local binary cache directory.
