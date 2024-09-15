use std::env;
use std::path::PathBuf;

fn main() {
    println!(
        "cargo:rustc-link-search=native={}",
        env::current_dir().unwrap().display()
    );
    println!("cargo:rustc-link-lib=static=lnd");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rerun-if-changed=liblnd.h");
    println!("cargo:rustc-link-lib=resolv");

    let bindings = bindgen::Builder::default()
        .header("liblnd.h")
        .allowlist_file("liblnd.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
