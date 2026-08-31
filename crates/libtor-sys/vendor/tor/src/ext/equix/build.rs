fn main() {
    cc::Build::new()
        .files(vec![
            "src/context.c",
            "src/equix.c",
            "src/solver.c",
            "hashx/src/blake2.c",
            "hashx/src/compiler.c",
            "hashx/src/compiler_a64.c",
            "hashx/src/compiler_x86.c",
            "hashx/src/context.c",
            "hashx/src/hashx.c",
            "hashx/src/program.c",
            "hashx/src/program_exec.c",
            "hashx/src/siphash.c",
            "hashx/src/siphash_rng.c",
            "hashx/src/virtual_memory.c",
        ])
        // Activate our patch for hashx_rng_callback
        .define("HASHX_RNG_CALLBACK", "1")
        // Equi-X always uses HashX size 8 (64-bit output)
        .define("HASHX_SIZE", "8")
        // Avoid shared library API declarations, link statically
        .define("HASHX_STATIC", "1")
        .define("EQUIX_STATIC", "1")
        .includes(vec!["include", "src", "hashx/include", "hashx/src"])
        .compile("equix");

    // Run bindgen to automatically extract types and functions. This time set
    // HASHX_SHARED and EQUIX_SHARED, so the function symbols are not hidden.
    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindgen::Builder::default()
        .header_contents(
            "wrapper.h",
            r#"
                #define HASHX_RNG_CALLBACK 1
                #define HASHX_SIZE 8
                #define HASHX_SHARED 1
                #define EQUIX_SHARED 1
                #include "hashx/include/hashx.h"
                #include "include/equix.h"
            "#,
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: true,
        })
        .bitfield_enum(".*_flags")
        .generate()
        .unwrap()
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
