fn main() {
    cc::Build::new()
        .file("c/float_to_str.c")
        .compile("float_to_str");
}
