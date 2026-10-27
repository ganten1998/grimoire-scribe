// stb_image_impl.c: stb_image compiled exactly as src/bgimage.c compiles it (the same STBI_*
// defines), with its allocations routed to Rust (bgimage.rs: grimoire_stb_malloc and friends),
// which counts them against the decode's budget as bgimage.c's stb_malloc does. Also what the
// decode-cost estimate reads from stb_image's internals (bgimage.c's sizeof(stbi__jpeg)). Nothing
// else of Grimoire is here: the logic stays in Rust.
#include <stddef.h>

void *grimoire_stb_malloc(size_t n);
void *grimoire_stb_realloc(void *p, size_t n);
void grimoire_stb_free(void *p);

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wsign-compare"
#pragma clang diagnostic ignored "-Wunused-function"
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
#pragma clang diagnostic ignored "-Wimplicit-fallthrough"
#elif defined(__GNUC__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wsign-compare"
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#pragma GCC diagnostic ignored "-Wimplicit-fallthrough"
#endif
#define STB_IMAGE_IMPLEMENTATION
#define STBI_ONLY_PNG
#define STBI_ONLY_JPEG
#define STBI_ONLY_PNM
#define STBI_MAX_DIMENSIONS 8192
#define STBI_NO_STDIO
#define STBI_NO_SIMD
#define STBI_NO_LINEAR
#define STBI_NO_HDR
#define STBI_MALLOC(sz) grimoire_stb_malloc(sz)
#define STBI_REALLOC(p, newsz) grimoire_stb_realloc(p, newsz)
#define STBI_FREE(p) grimoire_stb_free(p)
#include "stb_image.h"
#if defined(__clang__)
#pragma clang diagnostic pop
#elif defined(__GNUC__)
#pragma GCC diagnostic pop
#endif

// bgimage.c's JPEG cost adds sizeof(stbi__jpeg), the decoder's own state.
size_t grimoire_stbi_jpeg_size(void) { return sizeof(stbi__jpeg); }

// ffi_stb.rs keeps room for an stbtt_fontinfo in Rust's memory: a test holds it to this size.
#include "stb_truetype.h"
size_t grimoire_stbtt_fontinfo_size(void) { return sizeof(stbtt_fontinfo); }
