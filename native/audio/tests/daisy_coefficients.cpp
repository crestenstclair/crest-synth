#include "Filters/svf.h"
#include "PhysicalModeling/resonator.h"
#include "Drums/analogbassdrum.h"
#include "daisy_sequence.h"
#include <array>
#include <cmath>
#include <cstdio>

void daisy_reference_sequence(float* output);
void daisy_reference_voices(float* output);

bool daisy_coefficient_witness() {
    std::array<float, daisy_sequence_outputs> reference{}, actual{};
    daisy_reference_sequence(reference.data());
    daisy_filter_sequence<daisysp::Svf, daisysp::ResonatorSvf<4>>(actual.data());
    for (size_t i = 0; i < actual.size(); ++i) {
        // Reference TU and production archive may use different optimization
        // profiles; permit ordinary float rounding, never nonfinite output.
        if (!std::isfinite(actual[i]) || !std::isfinite(reference[i])
            || std::abs(actual[i] - reference[i]) > 1e-6f) {
            std::printf("DAISY COEFFICIENT MISMATCH %zu: %.9g != %.9g\n",
                i, actual[i], reference[i]);
            return false;
        }
    }
    std::array<float, daisy_voice_samples*2> reference_voices{}, actual_voices{};
    daisy_reference_voices(reference_voices.data());
    daisy_voice_sequence<daisysp::AnalogBassDrum,daisysp::Resonator>(actual_voices.data());
    for(size_t i=0; i<actual_voices.size(); ++i) {
        if(!std::isfinite(actual_voices[i]) || !std::isfinite(reference_voices[i])
            || std::abs(actual_voices[i]-reference_voices[i])>1e-6f) {
            std::printf("DAISY VOICE MISMATCH %zu: %.9g != %.9g\n",
                i,actual_voices[i],reference_voices[i]);
            return false;
        }
    }
    return true;
}
