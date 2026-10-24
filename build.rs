fn main() {
    println!("cargo:rerun-if-changed=frontend/dist");
    assert!(
        std::path::Path::new("frontend/dist/index.html").is_file(),
        "Missing frontend/dist/index.html; provide the built UI before compiling the backend"
    );
}
