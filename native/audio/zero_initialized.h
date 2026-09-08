#pragma once
#include <new>
#include <cstddef>

// Mutable firmware objects originally live in zero-initialized static memory.
// Preserve that initialization contract when preparing independent heap voices.
template<class T> class ZeroInitialized {
    alignas(T) unsigned char storage_[sizeof(T)]{};
public:
    ZeroInitialized() { new (storage_) T(); }
    ~ZeroInitialized() { get().~T(); }
    T& get() { return *std::launder(reinterpret_cast<T*>(storage_)); }
    ZeroInitialized(const ZeroInitialized&) = delete;
    ZeroInitialized& operator=(const ZeroInitialized&) = delete;
};
