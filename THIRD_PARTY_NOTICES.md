# Third-party notices

The prebuilt native libraries distributed by `react-native-nitro-tor` include the following components. Their complete license texts are packaged in `licenses/third-party`.

| Component | Version | License file |
| --- | --- | --- |
| Tor | 0.4.9.11 | `licenses/third-party/Tor.txt` |
| Libevent | 2.1.12-stable | `licenses/third-party/Libevent.txt` |
| OpenSSL | 3.5.4 | `licenses/third-party/OpenSSL.txt` |
| zlib | 1.3.1, through `libz-sys` | `licenses/third-party/zlib.txt` |
| libtor and libtor-sys | imported project forks | `licenses/third-party/libtor-MIT.txt` |

Tor is built in its default BSD-compatible mode with the GPL-only hidden-service proof-of-work module disabled.

This notice covers the principal native components embedded in the distributed archives. Rust dependency licensing remains described by the corresponding packages recorded in `Cargo.lock`.
