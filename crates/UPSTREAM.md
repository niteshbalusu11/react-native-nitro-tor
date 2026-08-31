# Imported Rust crates

The Rust crates in this directory are maintained as part of this repository. They were initially imported from repositories owned by this project so Cargo can resolve the complete native dependency chain through workspace paths.

| Local crate | Original repository | Imported commit |
| --- | --- | --- |
| `tor`, `logger`, `utils` | <https://github.com/niteshbalusu11/tor-rust-sdk> | `6ee9f071b769171de295e6dc817511533d345df6` |
| `libtor`, `libtor-derive` | <https://github.com/niteshbalusu11/libtor> | `ee242cdfbd995af7a81fe0ad0f481db4477e7a86` |
| `libtor-sys` | <https://github.com/niteshbalusu11/libtor-sys> | `9d527c4ab5f4d279a13ba12e786349afcfd0381f` |

The former `libtor-src` build crate was folded into `libtor-sys`. Its source staging, patch application, and Autotools setup now live directly in `libtor-sys/build.rs`.

The original MIT notices for `libtor`, `libtor-derive`, and `libtor-sys` are preserved with the imported crates and in the package's third-party license directory.
