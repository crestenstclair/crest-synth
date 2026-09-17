#pragma once
#include <cstddef>
#include "zero_initialized.h"

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

constexpr size_t daisy_voice_samples = 8192;
template<class Drum, class Body>
void daisy_voice_sequence(float* output) {
    ZeroInitialized<Drum> drum_storage;
    ZeroInitialized<Body> body_storage;
    auto& drum=drum_storage.get(); auto& body=body_storage.get();
    for(size_t i=0; i<daisy_voice_samples; ++i) {
        if(i%512==0) {
            const float rate=i<4096 ? 48000.f : 96000.f;
            const int resolutions[]={4,8,16,23,24};
            drum.Init(rate);
            body.Init((i/512%4)*.25f,resolutions[i/512%5],rate);
        }
        const size_t section=i/64;
        const float frequencies[]={20,110,440,3000,10000};
        drum.SetFreq(frequencies[section%5]); body.SetFreq(frequencies[section%5]);
        drum.SetTone((section%5)*.25f); drum.SetDecay((section%3)*.5f);
        drum.SetAccent(.2f+(section%5)*.1f); drum.SetSustain(section%7==0);
        drum.SetAttackFmAmount((section%3)*.2f); drum.SetSelfFmAmount((section%3)*.1f);
        body.SetStructure((section%5)*.25f); body.SetBrightness((section%3)*.5f);
        body.SetDamping((section%4)/3.f);
        const float input=(static_cast<int>(i*17%127)-63)*.0001f;
        output[2*i]=drum.Process(i%127==0);
        output[2*i+1]=body.Process(input);
    }
}
