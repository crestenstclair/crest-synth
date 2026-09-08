#pragma once
#include "random.h"
#undef rand
#undef srand

// Scope every upstream boundary, including construction and asset loading.
// Restoring the previous pointer also permits nested adapter calls.
class CrestRandomScope {
    CrestRandomState* previous_;
public:
    explicit CrestRandomScope(CrestRandomState& state) noexcept : previous_(crest_enter_random_state(state)) {}
    ~CrestRandomScope() { crest_leave_random_state(previous_); }
};
