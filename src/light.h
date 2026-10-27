// light.h: the Kindle's front light (brightness + warmth), through the system power service so
// the stock UI stays in sync (same lipc properties KOReader uses: com.lab126.powerd flIntensity
// and currentAmberLevel). Implemented by light_kindle.c and the simulator.
#pragma once
#include <stdbool.h>
#include <stddef.h>

#define LIGHT_MAX 24

bool light_available(void);
// Asks the power service for the levels (blocks: two lipc runs, 95-191 ms on the Scribe 2). Once, at
// the start, before the pens are opened; light_get asks again only while the brightness is unknown.
void light_refresh(void);
// Current levels (0..LIGHT_MAX) as last read or set; -1 where unknown.
void light_get(int *intensity, int *warmth);
void light_set_intensity(int level);
void light_set_warmth(int level);
// The warmth as last read (light_get) or set, without asking the power service: -1 before either,
// or on a Kindle without warm light.
int light_warmth(void);

// Other bits of the Kindle system Grimoire drives.
// Ask the power service to go to sleep, exactly as a power button press would. It turns
// preventScreenSaver off first, in the same shell (the button is ignored while it is on), so a
// "Sleep after: never" policy has to be applied again afterwards.
void sys_request_sleep(void);
// The power service's state, best effort (`lipc-get-prop -s com.lab126.powerd state`): 0 with e.g.
// active, screenSaver, readyToSuspend or suspended in `out`, or -1 if it cannot be read. Blocks
// (lipc can take half a second; at most about 1.5 s): only for the rare sleep and wake fallbacks,
// never on the drawing path.
int sys_powerd_state(char *out, size_t cap);
// Resume / pause the Kindle's window manager (only if the launcher paused it): the Kindle's own
// screensaver can only show while it runs.
bool sys_kindle_ui(bool run);
// Keep the power service from sleeping on its own (only for "Sleep after: never").
void sys_prevent_sleep(bool prevent);
// Tell the power service the user is active (Grimoire grabs the pen and touch, so it cannot see).
void sys_activity(void);
