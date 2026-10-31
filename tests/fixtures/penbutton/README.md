# Pen-button scenario fixtures

These are text bindings for the Phase 6 headless scenarios, not device captures.

- pages binds the first button's drag to Pages.
- pan binds the first button's drag to Pan (g_gest_15_link_then_finger.qs takes a lasso over with it).
- single binds tap to Undo while the built-in triple still requires a 350 ms wait.
- single-now clears the triple, allowing tap to resolve at release.
- button2 supplies a second-button binding and an Eraser line. Unsupported slots must survive
  saves as carried lines; Button2 becomes live only with GRIMOIRE_SIM_PEN_BUTTON2=1.
- gallery supplies multiple source bindings for row suffixes and both pen button names.

The v_gestures_4*.qs scenarios check actual dispatch, erasing, clear/reset, pan/zoom, capability
filtering and source recording. The language-neutral v_gestures_4f_ui.qs gallery was also run
in English/Spanish and Gothic/Classic, upright and sideways, with GRIMOIRE_CHECK_FIT=1.
