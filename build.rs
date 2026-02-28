use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let status = Command::new("gcc")
        .args(&["src/geminiclient.c", "-c", "-fPIC", "-o"])
        .arg(out_dir.join("geminiclient.o"))
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("ar")
        .args(&["crus", "libgemini.a", "geminiclient.o"])
        .current_dir(&out_dir)
        .status()
        .unwrap();
    assert!(status.success());
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=gemini");
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
}
