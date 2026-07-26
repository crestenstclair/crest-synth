fn main() {
    const SOURCES: &[&str] = &[
        "native/braids/crest_braids.cpp",
        "vendor/braids/braids/analog_oscillator.cc",
        "vendor/braids/braids/digital_oscillator.cc",
        "vendor/braids/braids/macro_oscillator.cc",
        "vendor/braids/braids/resources.cc",
        "vendor/braids/stmlib/utils/random.cc",
    ];

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++11")
        .include("vendor/braids")
        .include("native/braids")
        .flag_if_supported("-fno-exceptions")
        .flag_if_supported("-fno-rtti")
        .warnings(false);
    for source in SOURCES {
        println!("cargo:rerun-if-changed={source}");
        build.file(source);
    }
    println!("cargo:rerun-if-changed=native/braids/crest_braids.h");
    println!("cargo:rerun-if-changed=vendor/braids/PROVENANCE.md");
    build.compile("crest_braids");
}
