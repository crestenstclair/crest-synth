// A separate image is required: dyld excludes the interposer's own image.
// Count only the selected witness thread, with no TLS initialization in hooks.
#include <atomic>
#include <cstdlib>
#include <pthread.h>
static std::atomic<bool> enabled{false};
static std::atomic<pthread_t> measured_thread;
static size_t heap_operations=0,locks=0;
static bool measuring() {
    return enabled.load(std::memory_order_acquire) && pthread_equal(measured_thread.load(std::memory_order_relaxed),pthread_self());
}
extern "C" void crest_witness_begin() {
    measured_thread.store(pthread_self(),std::memory_order_relaxed);heap_operations=locks=0;
    enabled.store(true,std::memory_order_release);
}
extern "C" void crest_witness_end(size_t* heap,size_t* lock_count) {
    enabled.store(false,std::memory_order_release);
    *heap=heap_operations;*lock_count=locks;
}
#define INTERPOSE(replacement, original) \
__attribute__((used)) static struct { const void* replacement; const void* original; } interpose_##original \
__attribute__((section("__DATA,__interpose"))) = { (const void*)&replacement, (const void*)&original };
static void* tracked_malloc(size_t n) { if(measuring())++heap_operations; return malloc(n); }
static void* tracked_calloc(size_t n,size_t m) { if(measuring())++heap_operations; return calloc(n,m); }
static void* tracked_realloc(void* p,size_t n) { if(measuring())++heap_operations; return realloc(p,n); }
static void tracked_free(void* p) { if(p&&measuring())++heap_operations; free(p); }
static int tracked_mutex_lock(pthread_mutex_t* p) { if(measuring())++locks; return pthread_mutex_lock(p); }
static int tracked_rwlock_rdlock(pthread_rwlock_t* p) { if(measuring())++locks; return pthread_rwlock_rdlock(p); }
static int tracked_rwlock_wrlock(pthread_rwlock_t* p) { if(measuring())++locks; return pthread_rwlock_wrlock(p); }
INTERPOSE(tracked_malloc,malloc)
INTERPOSE(tracked_calloc,calloc)
INTERPOSE(tracked_realloc,realloc)
INTERPOSE(tracked_free,free)
INTERPOSE(tracked_mutex_lock,pthread_mutex_lock)
INTERPOSE(tracked_rwlock_rdlock,pthread_rwlock_rdlock)
INTERPOSE(tracked_rwlock_wrlock,pthread_rwlock_wrlock)
