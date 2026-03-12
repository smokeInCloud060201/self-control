use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreMedia");

        cc::Build::new()
            .file("src/sys/SCKAudioCapture.m")
            .flag("-fobjc-arc")
            .compile("SCKAudioCapture");
    }
}
