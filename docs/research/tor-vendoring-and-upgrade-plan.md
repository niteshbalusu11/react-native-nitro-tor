# Local Tor crates and C Tor upgrade plan

Research date: 2026-08-31

## Recommendation

Move the owned Rust crates into this workspace and upgrade the embedded C Tor directly from 0.4.7.16 to **0.4.9.11**. Do not stop at 0.4.8: Tor 0.4.8 reached end of life on 2026-06-01, and the Tor Project says C Tor 0.4.8 and earlier are intended to stop working on the network after **2026-09-01**. This application currently embeds 0.4.7.16, so the upgrade is urgent rather than routine maintenance. ([Tor Project sunset announcement](https://blog.torproject.org/sunsetting-tor-048/), [Tor 0.4.9.11 release announcement](https://forum.torproject.org/t/security-release-0-4-9-11/21786), [official 0.4.9.11 archive](https://archive.torproject.org/tor-package-archive/tor-0.4.9.11.tar.gz))

The upgrade appears low-risk at the Rust/C boundary: `src/feature/api/tor_api.h` and `tor_api.c` are byte-for-byte identical between Tor 0.4.7.16 and 0.4.9.11. Disposable builds using the current local `libtor-sys` fixes and Tor 0.4.9.11 passed for `aarch64-linux-android` (about 90 seconds) and `aarch64-apple-ios-sim` (about 74 seconds). These are feasibility checks, not substitutes for the full seven-target build and runtime tests. ([0.4.7.16 API](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.7.16/src/feature/api/tor_api.h), [0.4.9.11 API](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/src/feature/api/tor_api.h))

## Current dependency graph

The resolved owned-code path is:

```text
react_native_nitro_tor
├── tor (Git: tor-rust-sdk)
│   └── libtor (Git: libtor)
│       ├── libtor-derive (same Git repository)
│       └── libtor-sys (local Cargo source override)
│           └── libtor-src (Git: libtor-sys)
└── logger (Git: tor-rust-sdk)
```

The root manifest declares `tor` and `logger` from `tor-rust-sdk`, and overrides the Git `libtor-sys` package with `vendor/libtor-sys`. The local `libtor-sys` then fetches `libtor-src` from Git, so a clean build still depends on all three repositories and two moving `master` branches. ([root manifest at the current merge](https://github.com/niteshbalusu11/react-native-nitro-tor/blob/d0719203c1467325b1565c3a680557c6e7f45eb4/Cargo.toml), [SDK `tor` manifest](https://github.com/niteshbalusu11/tor-rust-sdk/blob/6ee9f071b769171de295e6dc817511533d345df6/tor/Cargo.toml), [`libtor` manifest](https://github.com/niteshbalusu11/libtor/blob/ee242cdfbd995af7a81fe0ad0f481db4477e7a86/libtor/Cargo.toml), [`libtor-sys` manifest](https://github.com/niteshbalusu11/libtor-sys/blob/9d527c4ab5f4d279a13ba12e786349afcfd0381f/Cargo.toml))

The owned Git packages resolved today are `tor`, `logger`, `libtor`, `libtor-derive`, and `libtor-src`; `libtor-sys` is already the locally overridden package. The `tor-rust-sdk` workspace also contains the path-only `utils` crate used by `tor` tests. ([SDK workspace](https://github.com/niteshbalusu11/tor-rust-sdk/blob/6ee9f071b769171de295e6dc817511533d345df6/Cargo.toml))

## Proposed repository layout

Use ordinary workspace crates and path dependencies:

```text
crates/
├── lib/                    # existing React Native Rust crate
├── tor/
├── logger/
├── tor-utils/              # package name can remain `utils`; needed for SDK tests
├── libtor/
├── libtor-derive/
└── libtor-sys/             # FFI and native build implementation
    ├── patches/
    └── vendor/
        ├── tor/                # unpacked, exact official release source
        │   ├── UPSTREAM.md     # version, source URL, SHA-256, import date
        │   └── ...
        └── libevent/           # currently 2.1.12-stable
```

Importing only `libtor` and `libtor-sys` is possible, but it does **not** produce a fully path-based graph: remote `tor-rust-sdk` declares its own Git `libtor` dependency, so the root would still need a `[patch."https://github.com/niteshbalusu11/libtor"]` source override. The cleaner end state is to import `tor`, `logger`, and the small test-only `utils` crate at the same time. Then every owned crate is a normal workspace path dependency, and the current Git-source override can be deleted instead of replaced by another override.

Use `libtor-src` only as a temporary migration boundary. It has one consumer and one narrow job—stage Tor and libevent sources, apply patches, and expose their paths to `libtor-sys`—so it is a shallow adapter rather than a useful long-term module. In the final layout, move that implementation and the upstream sources under `libtor-sys`. This leaves `libtor-sys` as the deep native-build module: its small unsafe FFI interface hides the much larger C source, patching, and cross-compilation implementation. The existing `libtor-src` build script remains the behavioral reference for the move. ([`libtor-src` build script](https://github.com/niteshbalusu11/libtor-sys/blob/9d527c4ab5f4d279a13ba12e786349afcfd0381f/libtor-src/build.rs))

## Tor version choice

### Target 0.4.9.11 directly

0.4.9.11 is the newest stable C Tor tarball in the official archive as of the research date. It is a security release fixing, among other issues, an onion-service impersonation race and a client crash triggered by a malformed onion-service introduction point. The Tor Project strongly recommends upgrading. ([0.4.9.11 ChangeLog](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/ChangeLog), [official archive checksum](https://archive.torproject.org/tor-package-archive/tor-0.4.9.11.tar.gz.sha256sum))

0.4.9 also adds Counter Galois Onion circuit cryptography and forward-compatible descriptor handling needed for the network's removal of obsolete TAP onion keys. This is the compatibility reason older clients are being sunset. ([0.4.9.5 release notes within ChangeLog](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/ChangeLog#L318), [sunset rationale](https://blog.torproject.org/sunsetting-tor-048/))

0.4.8.25 is useful only as a diagnostic bisect point if an unexpected regression appears. It should not be shipped: it is end-of-life and is included in the announced network cutoff.

### Build compatibility

- Tor 0.4.9 requires libevent 2.0.10 or later and OpenSSL 1.1.1 or later. The project already embeds libevent 2.1.12-stable and resolves vendored OpenSSL 3.5.4, so no prerequisite upgrade is required. ([Tor configure checks](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/configure.ac), [current libevent source](https://github.com/niteshbalusu11/libtor-sys/blob/9d527c4ab5f4d279a13ba12e786349afcfd0381f/libtor-src/libevent-src/configure.ac))
- The Android target-host mapping, NDK 28 zlib staging, iOS compiler/linker selection, and `ac_cv_func_pipe2=no` fixes remain relevant. The representative disposable Android and iOS simulator builds passed with them unchanged.
- Tor's release tarball contains generated Autotools files but not `autogen.sh`; the Git tag contains `autogen.sh`. Pick one canonical import source and make the update script consistent with it. The simplest match for the current build is the official Git tag source plus its recorded commit, or the official release tarball with `libtor-src` changed not to require `autogen.sh`. Do not silently combine files from the two distributions. ([official Git tag](https://gitlab.torproject.org/tpo/core/tor/-/tree/tor-0.4.9.11), [official release tarball](https://archive.torproject.org/tor-package-archive/tor-0.4.9.11.tar.gz))
- Remove the now-unrecognized `--disable-module-dircache` and `--disable-rust` configure arguments. Explicitly add `--disable-module-pow` so the BSD licensing choice is visible and cannot change accidentally. Tor's Autotools wrapper also emits harmless warnings for generic `--disable-shared`/`--enable-static`; those can be cleaned up separately only if the Rust `autotools` crate permits it without a workaround.

### Patch disposition

The current `libtor-src` carries five Tor patches. Against 0.4.9.11:

| Patch | Mechanical result | Planned action |
| --- | --- | --- |
| `tor-0004-ignore-libcap.patch` | Applies | Keep initially, then verify whether a configure cache variable can replace it. |
| `tor-0006-include-openssl-engine.patch` | Applies | Keep initially; verify against OpenSSL 3.5 on both platforms. |
| `tor-0007-disable-tools.patch` | Needs rebasing because nearby Windows include layout changed | Rebase and retain. |
| `tor-0008-exclude-unused-file.patch` | Needs rebasing because whitespace/context changed | Rebase and retain, then verify why it remains necessary. |
| `tor-0009-remove-symdef-sorted-libs.patch` | Mechanically applies | Delete: Tor 0.4.8 incorporated the Darwin `__.SYMDEF*` cleanup upstream, so applying this patch would duplicate the fix. ([upstream `combine_libs`](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/scripts/build/combine_libs)) |

The broader rule should be: edit the owned Rust crates directly, but keep modifications to upstream Tor and libevent as small named patch files. That preserves a reviewable boundary and makes future upstream upgrades auditable.

## Licensing and distribution

The owned `libtor` and `libtor-sys` forks retain the original Magical Bitcoin MIT notice; moving them does not erase that attribution. Preserve each repository's `LICENSE` alongside the imported crates. ([libtor license](https://github.com/niteshbalusu11/libtor/blob/ee242cdfbd995af7a81fe0ad0f481db4477e7a86/LICENSE), [libtor-sys license](https://github.com/niteshbalusu11/libtor-sys/blob/9d527c4ab5f4d279a13ba12e786349afcfd0381f/LICENSE))

Tor is primarily 3-clause BSD but includes separately licensed components. Preserve Tor's full `LICENSE`, libevent's `LICENSE`, and the licenses of the statically linked OpenSSL/zlib sources. Because the npm package distributes prebuilt static archives while its `files` list excludes the Rust/vendor source tree, add a packaged `THIRD_PARTY_NOTICES.md` (and include it in `package.json`) so binary-distribution attribution travels with the package. ([Tor 0.4.9.11 license inventory](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/LICENSE), [libevent license](https://github.com/niteshbalusu11/libtor-sys/blob/9d527c4ab5f4d279a13ba12e786349afcfd0381f/libtor-src/libevent-src/LICENSE))

Tor 0.4.9's hidden-service proof-of-work module is a special decision. It is built only with `--enable-gpl`; Tor's configure logic states that this changes the Tor/libtor build from its usual BSD license to GPL. This application creates onion services, but the PoW defense must remain explicitly disabled in the default MIT/BSD package unless the project deliberately accepts GPL distribution obligations. If PoW is desired, handle that as a separate licensing/product decision, not as an incidental upgrade flag. ([Tor license option and PoW module configuration](https://gitlab.torproject.org/tpo/core/tor/-/blob/tor-0.4.9.11/configure.ac#L50), [Tor PoW source licensing context](https://gitlab.torproject.org/tpo/core/tor/-/tree/tor-0.4.9.11/src/ext/equix))

## Staged execution plan

### 1. Localize without changing behavior

1. Import the pinned Rust sources at the exact commits currently in `Cargo.lock`: `tor`/`logger` (and `utils` for tests) plus `libtor`/`libtor-derive`; move the corrected local `libtor-sys` into `crates/libtor-sys`.
2. Preserve all upstream license/readme files and add provenance notes with original repository and commit IDs.
3. Convert manifests to workspace/path dependencies and delete the Git-source `[patch]` override.
4. Temporarily point local `libtor-sys` at the current `libtor-src` by its exact commit (`9d527c4ab5f4d279a13ba12e786349afcfd0381f`), not a moving branch. Keep Tor at 0.4.7.16 for this commit and verify that it is the only remaining owned Git source.
5. Run the full seven-target Craby build. This commit creates a clean behavioral baseline and makes any later failure attributable to the Tor upgrade. It also avoids committing the roughly 38 MB old Tor tree only to replace it immediately, which would retain both versions in Git history.

### 2. Upgrade Tor to 0.4.9.11

1. Import only the exact official 0.4.9.11 source and current libevent source under `crates/libtor-sys/vendor`, and record their checksum/tag provenance.
2. Move the staging and patch-application implementation from `libtor-src` into `libtor-sys`, then delete the temporary `libtor-src` Git dependency. Verify that the workspace now has no owned Git sources.
3. Update `libtor-sys` and `libtor` version metadata consistently; use a simple project version such as `49.11.0+0.4.9.11` only if external consumers rely on these crate versions. Workspace-private crates can instead share the repository version and record Tor's version separately.
4. Rebase patches 0007/0008, remove 0009, and revalidate 0004/0006.
5. Remove obsolete configure flags and explicitly disable the GPL PoW module.
6. Run targeted Android arm64 and iOS simulator builds first, then the full seven-target build and archive stripping checks.

### 3. Validate runtime behavior and package output

1. On Android and iOS, exercise bootstrap, SOCKS traffic, an HTTP request through Tor, onion-service creation, onion-service reachability, deletion, shutdown, and a second startup in the same process.
2. Exercise app background/foreground and data-directory reuse because those are common embedded-daemon lifecycle failures that a compile test cannot detect.
3. Build both example applications and inspect final link output for duplicate OpenSSL/zlib/Tor symbols.
4. Regenerate all committed Android archives and the iOS XCFramework, then confirm the npm tarball contains the binaries, root license, and third-party notices but not the large vendored source tree.

### 4. Make future Tor updates repeatable

Add one update script that accepts a Tor version and:

1. downloads from the official Tor archive or tag selected as the canonical source;
2. verifies the recorded SHA-256 and, preferably, the Tor release signature;
3. replaces the vendored source in a temporary directory;
4. runs `git apply --check` for every maintained patch before replacing the repository copy;
5. updates `UPSTREAM.md` and version metadata; and
6. prints the exact targeted and full-build commands required before commit.

Vendoring removes Dependabot's visibility into the C Tor version. Add a recurring release/security check against the official Tor release announcements, with updates handled as normal reviewed PRs. Never track an unpinned branch for the embedded security boundary.

## Acceptance criteria

- No owned Rust package in `Cargo.lock` has a Git source; all resolve from workspace paths.
- The embedded Tor reports `0.4.9.11` and the source provenance matches the official checksum/tag.
- BSD mode and `module-pow: no` appear in configure output; no GPL code is included accidentally.
- Android `arm64-v8a`, `armeabi-v7a`, `x86`, and `x86_64`, plus iOS device, arm64 simulator, and x86_64 simulator builds pass.
- Android and iOS runtime smoke tests pass for bootstrap, proxied HTTP, and onion-service lifecycle.
- The published npm tarball includes third-party license notices and the regenerated native binaries.
- The update procedure is documented and can reapply or flag every local Tor/libevent patch against the next stable release.
