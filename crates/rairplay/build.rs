fn main() {
    let dir = option_env!("FAIRPLAY3_SRC").unwrap_or("3rdparty/shairplay");
    println!("cargo::rerun-if-changed={dir}");

    let sources = glob::glob(&format!("{}/*.c", glob::Pattern::escape(dir)))
        .unwrap()
        .map(Result::unwrap);
    cc::Build::new()
        .files(sources)
        .cargo_warnings(false)
        .compile("fairplay3");
}
