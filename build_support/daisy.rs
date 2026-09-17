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
