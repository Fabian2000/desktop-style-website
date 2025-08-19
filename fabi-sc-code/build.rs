use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=pkg/fabi_sc_code_bg.wasm");
    println!("cargo:rerun-if-changed=pkg/fabi_sc_code.js");

    let dest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fabi-sc")
        .join("resources")
        .join("script");

    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pkg");

    let files = ["fabi_sc_code_bg.wasm", "fabi_sc_code.js"];

    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!("Failed to create script dir: {e}");
    }

    for file in files {
        let src = src_dir.join(file);
        let dst = dest_dir.join(file);

        if let Err(e) = fs::copy(&src, &dst) {
            eprintln!("Failed to copy {file}: {e}");
        }
    }
    println!("cargo:warning=Copyied resources {}, {} to {}", files[0], files[1], dest_dir.display());
}
