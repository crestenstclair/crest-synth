#pragma once
#include <cstdlib>
#include <cstdint>
#include <random>
#include <algorithm>
#include <stdlib.h>
// Parse both standard-library interfaces before adapting upstream rand calls.
// libstdc++'s algorithm and C compatibility headers still refer to std::rand.
// Each prepared instance owns its generators. The scoped accessor only selects
// that state; it does not introduce a new noise algorithm or libc rand lock.
struct CrestRandomState {
    std::minstd_rand standard{1};
    uint32_t mutable_state=0x21;
};
bool crest_random_initialize() noexcept;
CrestRandomState& crest_random_state() noexcept;
CrestRandomState* crest_enter_random_state(CrestRandomState&) noexcept;
void crest_leave_random_state(CrestRandomState*) noexcept;
inline int crest_audio_random() noexcept { return static_cast<int>(crest_random_state().standard()); }
inline void crest_audio_seed(unsigned seed) noexcept { crest_random_state().standard.seed(seed); }
#define rand crest_audio_random
#define srand crest_audio_seed
