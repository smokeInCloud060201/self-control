fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        // The first choice is Windows because DXGI is amazing.
        println!("cargo:rustc-cfg=dxgi");
    } else if target.contains("apple") || target.contains("darwin") {
        // Quartz is second because macOS is the (annoying) exception.
        println!("cargo:rustc-cfg=quartz");
    } else if target.contains("linux") || target.contains("unix") {
        // On UNIX we pray that X11 (with XCB) is available.
        println!("cargo:rustc-cfg=x11");
    }
}
