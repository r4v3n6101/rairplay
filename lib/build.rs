fn main() {
    let mut build = cc::Build::new();
    for entry in glob::glob("shairplay/src/lib/playfair/*.c").unwrap() {
        build.file(entry.unwrap());
    }
    build.compile("fairplay3");
}
