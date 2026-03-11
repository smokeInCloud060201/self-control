fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/SCKAudioCapture.m");
        cc::Build::new()
            .file("src/SCKAudioCapture.m")
            .flag("-fobjc-arc")
            .compile("sck_audio");

        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
        println!("cargo:rustc-link-lib=framework=CoreMedia");
    }
}
