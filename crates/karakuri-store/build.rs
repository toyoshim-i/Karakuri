use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../examples");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bundled_presets.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let examples_dir = Path::new(&manifest_dir).join("../../examples");

    writeln!(f, "pub static BUNDLED_PRESETS: &[(&str, &str)] = &[").unwrap();
    if let Ok(entries) = fs::read_dir(&examples_dir) {
        let mut sorted = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "kset" || ext == "kir" {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        sorted.push((file_name.to_string(), path.clone()));
                    }
                }
            }
        }
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, path) in sorted {
            writeln!(
                f,
                "    ({:?}, include_str!({:?})),",
                name,
                path.to_str().unwrap()
            )
            .unwrap();
        }
    }
    writeln!(f, "];").unwrap();
}
