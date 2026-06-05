fn main() {
    cargo_build::rerun_if_changed("c/float_to_str.c");
    cc::Build::new()
        .file("c/float_to_str.c")
        .compile("float_to_str");
}
