fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "linux" | "android" | "freebsd" => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libespeak-ng.so.1");
        }
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libespeak-ng.1.dylib");
        }
        _ => {}
    }
}
