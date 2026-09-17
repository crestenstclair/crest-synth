//! Keep upstream modal equations while batching independent filters for SIMD.
use std::path::{Path, PathBuf};

fn replace(text: &mut String, before: &str, after: &str) {
    assert_eq!(
        text.matches(before).count(),
        1,
        "Mutable adaptation no longer matches: {before}"
    );
    *text = text.replace(before, after);
}

pub fn stage(output: &Path) -> PathBuf {
    let destination = output.join("mutable-source");
    let mut filter = std::fs::read_to_string("vendor/audio/stmlib/dsp/filter.h").unwrap();
    replace(
        &mut filter,
        "class Svf {\n public:",
        "class Svf {\n  template<size_t> friend class SvfBatch;\n public:",
    );
    write(&output.join("stmlib/dsp/filter.h"), &filter);
    let relative = "rings/dsp/resonator.cc";
    let mut source = std::fs::read_to_string(format!("vendor/audio/mutable/{relative}")).unwrap();
    source.insert_str(0, "#include \"svf_batch.h\"\n");
    replace(
        &mut source,
        "  ParameterInterpolator position(&previous_position_, position_, size);",
        r###"  const int32_t rendered_modes = (num_modes + 1) & ~1;
  SvfBatch<kMaxModes> bank(f_, rendered_modes);
  float response[kMaxModes], gains[kMaxModes];
  // Also recognize a nonzero interpolation increment that rounds away: every
  // upstream Next() then returns the same position, even before exact equality.
  const bool fixed = size && previous_position_ +
      (position_ - previous_position_) / size == previous_position_;
  if (fixed) {
    CosineOscillator amplitudes;
    amplitudes.Init<COSINE_OSCILLATOR_APPROXIMATE>(previous_position_);
    for (int32_t i=0; i<rendered_modes; ++i) gains[i]=amplitudes.Next();
  }
  ParameterInterpolator position(&previous_position_, position_, size);"###,
    );
    replace(
        &mut source,
        r###"    for (int32_t i = 0; i < num_modes;) {
      odd += amplitudes.Next() * f_[i++].Process<FILTER_MODE_BAND_PASS>(input);
      even += amplitudes.Next() * f_[i++].Process<FILTER_MODE_BAND_PASS>(input);
    }"###,
        r###"    if (bank.Process(input, response)) {
      if (fixed) {
        for (int32_t i=0; i<num_modes;) {
          odd += gains[i] * response[i]; ++i;
          even += gains[i] * response[i]; ++i;
        }
      } else {
        for (int32_t i=0; i<num_modes;) {
          odd += amplitudes.Next() * response[i++];
          even += amplitudes.Next() * response[i++];
        }
      }
    }"###,
    );
    replace(
        &mut source,
        "\n}\n\n}  // namespace rings",
        "\n  bank.Store();\n}\n\n}  // namespace rings",
    );
    write(&destination.join(relative), &source);
    destination
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    if std::fs::read_to_string(path).ok().as_deref() != Some(text) {
        std::fs::write(path, text).unwrap();
    }
}

pub fn add_sources(build: &mut cc::Build, directory: &Path, overrides: &Path) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            add_sources(build, &path, overrides);
        } else if path.extension().and_then(|p| p.to_str()) == Some("cc") {
            let relative = path.strip_prefix("vendor/audio/mutable").unwrap();
            build.file(if relative == Path::new("rings/dsp/resonator.cc") {
                overrides.join(relative)
            } else {
                path
            });
        }
    }
}
