# Native source provenance

## Tor

- Version: `0.4.9.11`
- Git tag: `tor-0.4.9.11`
- Git commit: `f3d28b2e0978ca075ec324834bec077673478ded`
- Repository: <https://gitlab.torproject.org/tpo/core/tor.git>
- Release archive: <https://archive.torproject.org/tor-package-archive/tor-0.4.9.11.tar.gz>
- Release archive SHA-256: `2e6c1720118c812acf0079fd47cf91b6bfaba5d766c321c4d3d2a28d6a11a8ed`
- Imported: 2026-08-31

The source was imported from the official Git tag. Nested Cargo manifests were omitted because Cargo treats them as package boundaries when this tree is vendored inside `libtor-sys`; they are maintenance tools and are not part of the C Tor build.

Tor is configured without `module-pow`. Enabling that module requires Tor's `--enable-gpl` option and is a separate licensing and distribution decision.

## Libevent

- Version: `2.1.12-stable`
- Imported from the `libevent-src` tree at <https://github.com/niteshbalusu11/libtor-sys/tree/9d527c4ab5f4d279a13ba12e786349afcfd0381f/libtor-src/libevent-src>
- Imported commit: `9d527c4ab5f4d279a13ba12e786349afcfd0381f`
- Imported: 2026-08-31

Local changes to Tor and Libevent remain in `../patches` so an upstream update can check and review each deviation explicitly.

Run `scripts/update-tor-source.sh <version>` from the repository root for future Tor imports. The script verifies the signed official tag, removes nested Cargo package boundaries, checks every maintained Tor patch, and replaces the vendored tree. It intentionally leaves provenance and crate version edits for review.
