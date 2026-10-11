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

// Other bits of the Kindle system Quire drives.
// Ask the power service to go to sleep, exactly as a power button press would.
void sys_request_sleep(void);
// Resume / pause the Kindle's window manager (only if the launcher paused it): the Kindle's own
// screensaver can only show while it runs.
void sys_kindle_ui(bool run);
// Keep the power service from sleeping on its own (only for "Sleep after: never").
void sys_prevent_sleep(bool prevent);
// Tell the power service the user is active (Quire grabs the pen and touch, so it cannot see).
void sys_activity(void);
