#include "Filters/svf.h"
#include "PhysicalModeling/resonator.h"
#include "daisy_sequence.h"
#include <array>
#include <cmath>
#include <cstdio>

void daisy_reference_sequence(float* output);

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
    return true;
}
