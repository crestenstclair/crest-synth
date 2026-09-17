// Compile the retained, unmodified upstream implementation under another
// namespace; the witness compares it with the staged production implementation.
#define daisysp crest_reference_daisy
#include "../../../vendor/audio/daisysp/Source/Filters/svf.cpp"
#include "../../../vendor/audio/daisysp/Source/PhysicalModeling/resonator.h"
#include "../../../vendor/audio/daisysp/Source/PhysicalModeling/resonator.cpp"
#include "../../../vendor/audio/daisysp/Source/Drums/analogbassdrum.cpp"
#include "daisy_sequence.h"

void daisy_reference_sequence(float* output) {
    daisy_filter_sequence<crest_reference_daisy::Svf,
        crest_reference_daisy::ResonatorSvf<4>>(output);
}

void daisy_reference_voices(float* output) {
    daisy_voice_sequence<crest_reference_daisy::AnalogBassDrum,
        crest_reference_daisy::Resonator>(output);
}
