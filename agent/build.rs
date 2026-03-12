use dotenvy;

fn main() {
    // Load .env at build time to bake defaults into the binary
    let _ = dotenvy::dotenv();
    let server = std::env::var("PROXY_SERVER").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "8080".to_string());

    println!("cargo:rustc-env=DEFAULT_PROXY_SERVER={}", server);
    println!("cargo:rustc-env=DEFAULT_PROXY_PORT={}", port);

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreMedia");

        cc::Build::new()
            .file("src/sys/SCKAudioCapture.m")
            .flag("-fobjc-arc")
            .compile("SCKAudioCapture");
    }
}
