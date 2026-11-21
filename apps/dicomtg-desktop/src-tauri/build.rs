fn main() {
    // SAFETY: this single-threaded build script mutates its environment before doing other work.
    unsafe { std::env::remove_var("STATIC_VCRUNTIME") };
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new().static_vc_runtime(true)),
    )
    .expect("failed to run the Tauri build script");
}
