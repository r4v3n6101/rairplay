fn main() {
    let mut build = cc::Build::new();
    for entry in glob::glob("shairplay/src/lib/playfair/*.c").unwrap() {
        build.file(entry.unwrap());
    }
    build.warnings(false).compile("fairplay3");

    println!("cargo:rerun-if-changed=ffmpeg_test/h264decode.c");
    println!("cargo:rustc-link-search=native={}", env!("FFMPEG_LIBS"));
    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avutil");
    cc::Build::new()
        .file("ffmpeg_test/h264decode.c")
        .include(env!("FFMPEG_INCLUDE"))
        .compile("h264test");
}
