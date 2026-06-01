// SPDX-License-Identifier: Apache-2.0
// Generate the C header (include/rfl.h) from the extern "C" surface via cbindgen.

use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = crate_dir.join("include").join("rfl.h");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    match cbindgen::generate(&crate_dir) {
        Ok(bindings) => {
            bindings.write_to_file(&out);
        }
        // A header-generation failure must not block the cdylib build (e.g. on a
        // toolchain where cbindgen cannot parse); the committed header still serves.
        Err(e) => println!("cargo:warning=cbindgen header generation skipped: {e}"),
    }
}
