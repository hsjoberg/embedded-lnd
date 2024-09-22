use std::env;
use std::path::PathBuf;

fn main() {
    // Look for LND_LIB_DIR environment variable
    let lnd_lib_dir = env::var("LND_LIB_DIR").expect("LND_LIB_DIR must be set");

    println!("cargo:rustc-link-search=native={}", lnd_lib_dir);
    println!("cargo:rustc-link-lib=lnd");
    println!("cargo:rerun-if-changed=./liblnd.h");
    println!("cargo:rerun-if-env-changed=LND_LIB_DIR");

    // Platform-specific configurations
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=resolv");
    } else if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=userenv");
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=advapi32");
    } else {
        // Linux and other Unix-like systems
        println!("cargo:rustc-link-lib=resolv");
    }

    let bindings = bindgen::Builder::default()
        .header("./liblnd.h")
        .allowlist_file("./liblnd.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
