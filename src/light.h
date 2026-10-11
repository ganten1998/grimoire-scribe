// light.h: the Kindle's front light (brightness + warmth), through the system power service so
// the stock UI stays in sync (same lipc properties KOReader uses: com.lab126.powerd flIntensity
// and currentAmberLevel). Implemented by light_kindle.c and the simulator.
#pragma once
#include <stdbool.h>

#define LIGHT_MAX 24

bool light_available(void);
// Current levels (0..LIGHT_MAX); -1 where unknown.
void light_get(int *intensity, int *warmth);
void light_set_intensity(int level);
void light_set_warmth(int level);
