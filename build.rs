fn main() {
    let mut build = cc::Build::new();
    let shairplay_path = option_env!("FAIRPLAY3_SRC").unwrap_or("shairplay/src/lib/playfair");
    for entry in glob::glob(&format!("{}/*.c", shairplay_path)).unwrap() {
        build.file(entry.unwrap());
    }
    build.cargo_warnings(false).compile("fairplay3");
}
