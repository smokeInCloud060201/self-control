fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreMedia");

        cc::Build::new()
            .file("src/sys/SCKAudioCapture.m")
            .flag("-fobjc-arc")
            .compile("SCKAudioCapture");
    }
}
