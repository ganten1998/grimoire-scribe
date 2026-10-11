// testlib.h: minimal test harness. Each tests/test_*.c is its own binary.
#pragma once
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int tl_failures = 0;
static int tl_checks = 0;

#define CHECK(cond)                                                                 \
    do {                                                                            \
        tl_checks++;                                                                \
        if (!(cond)) {                                                              \
            tl_failures++;                                                          \
            fprintf(stderr, "  FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);      \
        }                                                                           \
    } while (0)

#define CHECK_NEAR(a, b, eps) CHECK(fabs((double)(a) - (double)(b)) <= (eps))

#define TEST(name) static void name(void)
#define RUN(name)                                  \
    do {                                           \
        fprintf(stderr, "- %s\n", #name);          \
        name();                                    \
    } while (0)

static inline int tl_done(void) {
    fprintf(stderr, "%d checks, %d failures\n", tl_checks, tl_failures);
    return tl_failures ? 1 : 0;
}

// Scratch dir for tests (Makefile sets QUIRE_TEST_TMP).
static inline const char *tl_tmp(void) {
    const char *t = getenv("QUIRE_TEST_TMP");
    return t ? t : "/tmp";
}
