use std::fs;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=rust-toolchain-status.txt");

    let rustc_version = Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|| "rustc unknown".to_string());

    let updated_at = read_updated_at("rust-toolchain-status.txt")
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=TOVA_RUSTC_VERSION={rustc_version}");
    println!("cargo:rustc-env=TOVA_RUST_UPDATED_AT={updated_at}");
}

fn read_updated_at(path: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let (key, value) = line.split_once('=')?;
        if key.trim() == "updated_at" {
            return Some(value.trim().to_string());
        }
    }
    None
}
