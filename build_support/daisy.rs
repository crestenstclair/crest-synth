//! Preserve pinned DaisySP sources while caching unchanged filter coefficients.
use std::path::{Path, PathBuf};

pub fn stage(output: &Path) -> PathBuf {
    let destination = output.join("daisy-source");
    copy_tree(Path::new("vendor/audio/daisysp/Source"), &destination);
    destination
}

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let source = entry.unwrap().path();
        let target = destination.join(source.file_name().unwrap());
        if source.is_dir() {
            copy_tree(&source, &target);
            continue;
        }
        let mut bytes = std::fs::read(&source).unwrap();
        let relative = source.strip_prefix("vendor/audio/daisysp/Source").unwrap();
        for (path, before, after) in EDITS {
            if relative == Path::new(path) {
                let text = String::from_utf8(bytes).unwrap();
                assert_eq!(
                    text.matches(before).count(),
                    1,
                    "DaisySP coefficient adaptation no longer matches {}",
                    source.display()
                );
                bytes = text.replace(before, after).into_bytes();
            }
        }
        if std::fs::read(&target).ok().as_deref() != Some(&bytes) {
            std::fs::write(&target, bytes).unwrap();
        }
    }
}

// Setters retain their original calculations, including the first call after
// Init. Changes of frequency, resonance, or drive still update dependent state.
// ResonatorSvf caches only coefficients; gain and filter history always advance.
const EDITS: &[(&str, &str, &str)] = &[
    (
        "Drums/analogbassdrum.h",
        "    float accent_, f0_, tone_, decay_;",
        "    float accent_, f0_, tone_, decay_;\n    float cached_q_, cached_tone_;",
    ),
    (
        "Drums/analogbassdrum.cpp",
        "1500.0f * powf(2.f, kOneTwelfth * decay_ * 80.0f)",
        "cached_q_",
    ),
    (
        "Drums/analogbassdrum.cpp",
        "powf(2.f, kOneTwelfth * tone_ * 108.0f)",
        "cached_tone_",
    ),
    (
        "Drums/analogbassdrum.cpp",
        "    tone_ = fclamp(tone, 0.f, 1.f);",
        "    tone_ = fclamp(tone, 0.f, 1.f);\n    cached_tone_ = powf(2.f, kOneTwelfth * tone_ * 108.0f);",
    ),
    (
        "Drums/analogbassdrum.cpp",
        "    decay_ -= .1f;",
        "    decay_ -= .1f;\n    cached_q_ = 1500.0f * powf(2.f, kOneTwelfth * decay_ * 80.0f);",
    ),
    (
        "PhysicalModeling/resonator.h",
        "    float                        mode_amplitude_[kMaxNumModes];",
        r###"    bool modes_ready_;
    float cached_frequency_, cached_structure_, cached_brightness_, cached_damping_;
    float prepared_f_[kMaxNumModes], prepared_q_[kMaxNumModes], prepared_a_[kMaxNumModes];
    float                        mode_amplitude_[kMaxNumModes];"###,
    ),
    (
        "PhysicalModeling/resonator.cpp",
        "    sample_rate_ = sample_rate;",
        "    sample_rate_ = sample_rate;\n    modes_ready_ = false;",
    ),
    (
        "PhysicalModeling/resonator.cpp",
        "    float stiffness  = CalcStiff(structure_);",
        r###"    if(!modes_ready_ || frequency_ != cached_frequency_ || structure_ != cached_structure_
       || brightness_ != cached_brightness_ || damping_ != cached_damping_)
    {
    modes_ready_ = true;
    cached_frequency_ = frequency_; cached_structure_ = structure_;
    cached_brightness_ = brightness_; cached_damping_ = damping_;
    float stiffness  = CalcStiff(structure_);"###,
    ),
    (
        "PhysicalModeling/resonator.cpp",
        r###"    float mode_q[kModeBatchSize];
    float mode_f[kModeBatchSize];
    float mode_a[kModeBatchSize];
    int   batch_counter = 0;

    ResonatorSvf<kModeBatchSize>* batch_processor = &mode_filters_[0];
"###,
        "",
    ),
    (
        "PhysicalModeling/resonator.cpp",
        r###"        mode_f[batch_counter] = mode_frequency;
        mode_q[batch_counter] = 1.0f + mode_frequency * q;
        mode_a[batch_counter] = mode_amplitude_[i] * mode_attenuation;
        ++batch_counter;

        if(batch_counter == kModeBatchSize)
        {
            batch_counter = 0;
            batch_processor
                ->Process<ResonatorSvf<kModeBatchSize>::BAND_PASS, true>(
                    mode_f, mode_q, mode_a, in, &out);
            ++batch_processor;
        }"###,
        r###"        prepared_f_[i] = mode_frequency;
        prepared_q_[i] = 1.0f + mode_frequency * q;
        prepared_a_[i] = mode_amplitude_[i] * mode_attenuation;"###,
    ),
    (
        "PhysicalModeling/resonator.cpp",
        "    return out;",
        r###"    }
    for(int batch = 0; batch < resolution_ / kModeBatchSize; ++batch)
    {
        const int offset = batch * kModeBatchSize;
        mode_filters_[batch].Process<ResonatorSvf<kModeBatchSize>::BAND_PASS, true>(
            prepared_f_ + offset, prepared_q_ + offset, prepared_a_ + offset, in, &out);
    }
    return out;"###,
    ),
    (
        "Filters/svf.h",
        r###"    float sr_, fc_, res_, drive_, freq_, damp_;"###,
        r###"    bool frequency_ready_, resonance_ready_;
    float sr_, fc_, res_, drive_, freq_, damp_;"###,
    ),
    (
        "Filters/svf.cpp",
        r###"    sr_        = sample_rate;"###,
        r###"    frequency_ready_ = resonance_ready_ = false;
    sr_        = sample_rate;"###,
    ),
    (
        "Filters/svf.cpp",
        r###"    fc_ = fclamp(f, 1.0e-6, fc_max_);"###,
        r###"    const float next = fclamp(f, 1.0e-6, fc_max_);
    if(frequency_ready_ && next == fc_) return;
    frequency_ready_ = true;
    fc_ = next;"###,
    ),
    (
        "Filters/svf.cpp",
        r###"    res_      = res;"###,
        r###"    if(resonance_ready_ && res == res_) return;
    resonance_ready_ = true;
    res_      = res;"###,
    ),
    (
        "PhysicalModeling/resonator.h",
        r###"    void Init()
    {"###,
        r###"    void Init()
    {
        coefficients_ready_ = false;"###,
    ),
    (
        "PhysicalModeling/resonator.h",
        r###"            g[i]        = fasttan(f[i]);
            r[i]        = 1.0f / q[i];
            h[i]        = 1.0f / (1.0f + r[i] * g[i] + g[i] * g[i]);
            r_plus_g[i] = r[i] + g[i];"###,
        r###"            if(!coefficients_ready_ || f[i] != cached_f_[i] || q[i] != cached_q_[i])
            {
                cached_f_[i] = f[i];
                cached_q_[i] = q[i];
                const float g = fasttan(f[i]);
                const float r = 1.0f / q[i];
                cached_g_[i] = g;
                cached_h_[i] = 1.0f / (1.0f + r * g + g * g);
                cached_r_plus_g_[i] = r + g;
            }
            g[i] = cached_g_[i];
            h[i] = cached_h_[i];
            r_plus_g[i] = cached_r_plus_g_[i];"###,
    ),
    (
        "PhysicalModeling/resonator.h",
        r###"        float r[batch_size];
"###,
        r###""###,
    ),
    (
        "PhysicalModeling/resonator.h",
        r###"        float s_in  = in;"###,
        r###"        coefficients_ready_ = true;
        float s_in  = in;"###,
    ),
    (
        "PhysicalModeling/resonator.h",
        r###"    float state_1_[batch_size];"###,
        r###"    bool coefficients_ready_;
    float cached_f_[batch_size], cached_q_[batch_size];
    float cached_g_[batch_size], cached_h_[batch_size], cached_r_plus_g_[batch_size];
    float state_1_[batch_size];"###,
    ),
];
