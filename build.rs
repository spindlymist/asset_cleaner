fn main() {
    cargo_build::rerun_if_changed("src/convert.c");
    cc::Build::new()
        .file("src/convert.c")
        .compile("convert");
}
