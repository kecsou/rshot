fn main() {
    // No test-only Windows manifest: the unit tests are built from the bin target, so they get
    // tauri-build's Common-Controls v6 manifest through rustc-link-arg-bins like the app does.
    // (rustc-link-arg-tests would fail the build: this package has no tests/ target.)
    tauri_build::build()
}
