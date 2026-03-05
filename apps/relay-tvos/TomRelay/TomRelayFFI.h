#ifndef TomRelayFFI_h
#define TomRelayFFI_h

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Expected symbols to expose from Rust library.
// Return 0 on success.
int32_t tom_relay_start(const char* http_addr, const char* metrics_addr);
void tom_relay_stop(void);

#ifdef __cplusplus
}
#endif

#endif /* TomRelayFFI_h */
