fn main() {
    let mut build = cc::Build::new();
    for entry in glob::glob(&format!("{}/*.c", env!("FAIRPLAY3_SRC"))).unwrap() {
        build.file(entry.unwrap());
    }
    build.cargo_warnings(false).compile("fairplay3");
}
