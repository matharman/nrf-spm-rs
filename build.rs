// Modified from cortex-m-quickstart

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=memory.x");

    let memory_x = "memory.x";

    // Put memory configuration in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    std::fs::copy(memory_x, out.join("memory.x")).expect("failed to copy memory.x");
    println!("cargo:rustc-link-search={}", out.display());
}
