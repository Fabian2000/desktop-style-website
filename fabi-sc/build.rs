// Build WASM first, then copy server resources into target/<profile>/resources.
use std::{env, ffi::OsStr, fs, path::{Path, PathBuf}, process::Command};

fn main() {
    // Build the WASM crate
    let wasm_proj = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fabi-sc-code");

    let status = Command::new("wasm-pack")
        .args(["build", "--target", "web"])
        .current_dir(&wasm_proj)
        .status();

    match status {
        Ok(s) if s.success() => println!("WASM build successful."),
        Ok(s) => eprintln!("WASM build failed with status: {}", s),
        Err(e) => eprintln!("Failed to run wasm-pack: {}", e),
    }

    // Copy server resources into target/<profile>/resources
    println!("cargo:rerun-if-changed=resources");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let dest_dir = target_profile_dir_from_out_dir(&profile).unwrap_or_else(|| {
        // Fallback if OUT_DIR format is unexpected
        PathBuf::from("target").join(&profile)
    }).join("resources");

    if let Err(e) = copy_dir_all("resources", &dest_dir) {
        eprintln!("Failed to copy resources: {}", e);
    }

    let src_dir = PathBuf::from("resources");
    let abs_src_dir = src_dir.canonicalize().unwrap();
    println!("cargo:warning={} -> {}", abs_src_dir.display(), dest_dir.display());
}

/// Derive target/<profile> from OUT_DIR (works in workspaces and everywhere)
fn target_profile_dir_from_out_dir(profile: &str) -> Option<PathBuf> {
    let out = PathBuf::from(env::var("OUT_DIR").ok()?);
    // Find ancestor whose last component equals the profile ("debug" or "release")
    for a in out.ancestors() {
        if a.file_name() == Some(OsStr::new(profile)) {
            return Some(a.to_path_buf());
        }
    }
    None
}

/// Recursive copy
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
