#pragma once
#include <cstdint>
#include <random>

// Prepared instance state selected at each native boundary. PRNG streams and
// sample rate belong to the processor, never to a shared device/global clock.
struct CrestProcessorState {
    std::minstd_rand standard{1};
    uint32_t mutable_state=0x21;
    double sample_rate=0; // Zero preserves STK's global rate for reference use.
};
bool crest_processor_initialize() noexcept;
CrestProcessorState& crest_processor_state() noexcept;
CrestProcessorState* crest_enter_processor_state(CrestProcessorState&) noexcept;
void crest_leave_processor_state(CrestProcessorState*) noexcept;
double crest_stk_sample_rate(double fallback) noexcept;

class CrestProcessorScope {
    CrestProcessorState* previous_;
public:
    explicit CrestProcessorScope(CrestProcessorState& state) noexcept
        : previous_(crest_enter_processor_state(state)) {}
    ~CrestProcessorScope() { crest_leave_processor_state(previous_); }
    CrestProcessorScope(const CrestProcessorScope&) = delete;
    CrestProcessorScope& operator=(const CrestProcessorScope&) = delete;
};
