# Companion code for "Rust and Systems Programming: The Complete Guide"

Everything here was compiled and run while writing the guide (Rust 1.75, g++ 13).

| Folder | Stage | How to run |
| --- | --- | --- |
| cpp-cache-lab | 1 | `g++ -O2 -std=c++20 cache_lab.cpp -o cache_lab && ./cache_lab` |
| std-samples | 2, 3, 4, 5, 6, 7, 9, 11, 12 | `cargo test` for the library modules; `cargo run --release --example NAME` for count_allocs, executor, http_server, size_class_alloc, layout_bench, crash_test |
| arena-tree | 2 | `rustc -O tree.rs && ./tree` |
| ffi-sample | 8 | `gcc -c counter.c && ar rcs libcounter.a counter.o && rustc main.rs -L . -l static=counter && ./main` |
| crate-samples | 5, 6, 8, 10, 11, 12 | needs crates.io: `cargo run --release --example NAME` for backpressure, parallel, shutdown, proxy; `cargo run` runs src/main.rs |

Notes:
- std-samples uses only the standard library. The pager is Unix-only and the AVX2 code is x86-64 only.
- crate-samples/Cargo.lock pins versions that build on Rust 1.75. On a newer toolchain, delete it and let Cargo resolve fresh versions.
- The sandbox used for verification had one CPU core, so the parallel and false-sharing examples were checked for correctness only. Measure speedups on your own multi-core machine.
- Starter code, not production code.
