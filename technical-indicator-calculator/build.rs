use std::env;

fn main() {
    // Link with TA-Lib - note the hyphen in the library name
    println!("cargo:rustc-link-lib=ta-lib");
    
    // Specify where to find the library (may need adjusting for your environment)
    let ta_lib_path = env::var("TA_LIB_PATH").unwrap_or_else(|_| "/usr/lib".to_string());
    println!("cargo:rustc-link-search={}", ta_lib_path);
    
    // If TA-Lib is installed in a non-standard location, tell the linker where to find it
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", ta_lib_path);
    }
    
    // Regenerate if these files change
    println!("cargo:rerun-if-changed=src/talib_bindings.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TA_LIB_PATH");
}
