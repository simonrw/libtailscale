fn main() {
    println!("cargo:rustc-link-search=.");
    println!("cargo:rustc-link-lib=static=tailscale2");

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=Security");
    }
}
