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

    const CHORUS_SOURCES: &[&str] = &[
        "native/chorus/crest_chorus.cpp",
        "vendor/chorus/rings/resources.cc",
    ];
    let mut chorus = cc::Build::new();
    chorus
        .cpp(true)
        .std("c++11")
        .include("vendor/chorus")
        .include("native/chorus")
        .define("TEST", None)
        .flag_if_supported("-fno-exceptions")
        .flag_if_supported("-fno-rtti")
        .warnings(false);
    for source in CHORUS_SOURCES {
        println!("cargo:rerun-if-changed={source}");
        chorus.file(source);
    }
    println!("cargo:rerun-if-changed=native/chorus/crest_chorus.h");
    println!("cargo:rerun-if-changed=vendor/chorus/PROVENANCE.md");
    println!("cargo:rerun-if-changed=vendor/chorus/SHA256SUMS");
    chorus.compile("crest_chorus");

    // Tauri webview-shell scaffolding (mission webview-shell-foundation):
    // generates the context consumed by `tauri::generate_context!` for the
    // window-only, no-frontend-bundle setup declared in `tauri.conf.json`.
    // Extends the existing C++ vendor build; never replaces it.
    tauri_build::build();
}
