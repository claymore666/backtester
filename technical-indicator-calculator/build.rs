fn main() {
    // Link with TA-Lib - note the hyphen in the library name
    println!("cargo:rustc-link-lib=ta-lib");
    
    // Specify where to find the library
    println!("cargo:rustc-link-search=/lib");
    
    // Add to runtime path so it's found at runtime too
    println!("cargo:rustc-link-arg=-Wl,-rpath,/lib");
    
    // Regenerate if these files change
    println!("cargo:rerun-if-changed=src/talib_bindings.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
