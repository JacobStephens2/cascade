// Thin wrapper so `cargo run --bin uniffi-bindgen` can invoke uniffi's
// bindgen entry point against this crate's library. The Android Gradle
// task uses this to generate Kotlin bindings.
fn main() {
    uniffi::uniffi_bindgen_main()
}
