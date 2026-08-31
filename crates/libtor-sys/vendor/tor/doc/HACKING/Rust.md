# Rust support in C Tor

The [Arti project](https://gitlab.torproject.org/tpo/core/arti) is the team's
ongoing effort to write a pure-Rust implementation of Tor.

Arti is not yet feature complete but it's in active development. That's where
you want to go if you're interested in Tor and Rust together.

This document describes something with niche interest: the C implementation of
Tor can expose Rust crates which are used for internal testing, benchmarking,
comparison, fuzzing, and so on. This could be useful for comparing the C
implementation against new Rust implementations, or for simply using Rust
tooling for writing tests against C.

## Crates

Right now we are only using this mechanism for one crate:

- `tor-c-equix` -- Wraps the `src/ext/equix` module,
  containing Equi-X and HashX algorithms.

## Stability

This is not a stable API and we have no plans to develop a stable Rust interface
to the C implementation of Tor.

## Files

We use only a few of the standard Rust file types in order to build our
wrapper crates. Here's a summary:

- `Cargo.toml` in the repository root defines a Cargo *workspace*. It will
  list all subdirectories that contain crates with their own `Cargo.toml`.
- A per-crate `Cargo.toml` defines metadata and dependencies. These crates
  should all be marked `publish = false`.
- `build.rs` implements a simple build system that does not interact with
  autotools. It uses the `cc` and `bindgen` crates to get from `.c`/`.h`
  files to a static library and matching auto-generated bindings. Prefer to
  include bindgen wrapper headers inline within `build.rs` instead of adding
  `.h` files that are only used by the Rust bindings.
- `lib.rs` publishes the low-level `ffi` interface produced with `cc` and
  `bindgen`. This is also where we can add any wrappers or additions we want
  for making the Rust API more convenient.
