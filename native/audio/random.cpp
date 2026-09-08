#include "random_state.h"
#ifdef __APPLE__
#include <pthread.h>
// Darwin lazily allocates C++ thread_local storage on first use. pthread TSD
// slots live in the existing pthread record and do not allocate on access.
// Allocate the key once during preparation, before any handle can render.
static pthread_key_t random_key;
static pthread_once_t random_once=PTHREAD_ONCE_INIT;
static bool random_ready=false;
static void create_random_key() { random_ready=pthread_key_create(&random_key,nullptr)==0; }
bool crest_random_initialize() noexcept {
    return pthread_once(&random_once,create_random_key)==0 && random_ready;
}
static CrestRandomState* current_state() noexcept { return static_cast<CrestRandomState*>(pthread_getspecific(random_key)); }
static void set_state(CrestRandomState* state) noexcept { pthread_setspecific(random_key,state); }
#else
// Constant-initialized POD TLS in the statically linked native library.
static thread_local CrestRandomState* current=nullptr;
bool crest_random_initialize() noexcept { return true; }
static CrestRandomState* current_state() noexcept { return current; }
static void set_state(CrestRandomState* state) noexcept { current=state; }
#endif
CrestRandomState& crest_random_state() noexcept { return *current_state(); }
CrestRandomState* crest_enter_random_state(CrestRandomState& state) noexcept {
    auto* previous=current_state();set_state(&state);return previous;
}
void crest_leave_random_state(CrestRandomState* previous) noexcept { set_state(previous); }
