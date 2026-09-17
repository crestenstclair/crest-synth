#pragma once
#include <cstddef>

constexpr size_t daisy_sequence_samples = 4096;
constexpr size_t daisy_sequence_outputs = daisy_sequence_samples * 4;

// One sequence exercises unchanged blocks, per-sample gain, scalar changes,
// clamped endpoints, first-use initialization, and reinitialization at a new rate.
template<class Filter, class ResonatorFilter>
void daisy_filter_sequence(float* output) {
    Filter filter;
    ResonatorFilter resonator;
    for (size_t i = 0; i < daisy_sequence_samples; ++i) {
        if (i % 2048 == 0) {
            filter.Init(i == 0 ? 48000.f : 96000.f);
            resonator.Init();
        }
        const float frequencies[] = {200.f, 0.f, 1200.f, 26000.f};
        const float resonances[] = {.5f, .1f, 1.f, .8f};
        const size_t block = i / 64;
        // Change call order as well as values: SetFreq/SetRes share damping.
        if (block % 2) filter.SetRes(resonances[block % 4]);
        filter.SetFreq(frequencies[block % 4]);
        filter.SetDrive(static_cast<float>(i % 11));
        filter.SetRes(resonances[block % 4]);
        const float input = (static_cast<int>(i * 17 % 127) - 63) * .0001f;
        filter.Process(input);
        output[4*i] = filter.Low();
        output[4*i+1] = filter.Band();
        output[4*i+2] = filter.High();
        float f[4], q[4], gain[4];
        for (size_t band = 0; band < 4; ++band) {
            f[band] = .005f + .02f * static_cast<float>((block + band) % 19);
            q[band] = .7f + .1f * static_cast<float>((block + 3*band) % 17);
            gain[band] = .1f * static_cast<float>((i + band) % 5);
        }
        resonator.template Process<ResonatorFilter::BAND_PASS, false>(
            f, q, gain, input, &output[4*i+3]);
    }
}
