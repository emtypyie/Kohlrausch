use sha2::{Digest, Sha256};
use std::{env, fs, path::{Path, PathBuf}};

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(path).expect("source directory must be readable");
    for entry in entries {
        let entry = entry.expect("source directory entry must be readable");
        let path = entry.path();
        let kind = entry.file_type().expect("source entry type must be readable");
        if kind.is_dir() {
            collect_files(&path, files);
        } else if kind.is_file() {
            files.push(path);
        }
    }
}

fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory is set"));
    let mut files = Vec::new();
    for relative in ["Cargo.toml", "Cargo.lock", "tauri.conf.json", "build.rs"] {
        files.push(manifest_dir.join(relative));
    }
    for relative in ["src", "ui"] {
        collect_files(&manifest_dir.join(relative), &mut files);
    }
    files.sort();

    let mut hash = Sha256::new();
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path.strip_prefix(&manifest_dir).expect("source is inside the manifest directory");
        hash.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        hash.update([0]);
        hash.update(fs::read(path).expect("build input must be readable"));
        hash.update([0xff]);
    }
    hash.update(env::var("TARGET").expect("Cargo target is set").as_bytes());
    let build_id = format!("{:x}", hash.finalize());
    println!("cargo:rustc-env=KOHLRAUSCH_BUILD_ID={}", &build_id[..16]);
}
