# Keyboard and controller shortcuts

Enable **Library > Settings > Shortcuts > Enable shortcuts**. This writes starter mappings to
`/mnt/us/grimoire/shortcuts.ini` only if that file does not exist. Edit it over USB/SSH and choose
**Reload shortcuts.ini**. The settings sheet reports loaded/rejected mappings and the last key
or controller button code. Disable shortcuts to close the optional input devices again.

The default keyboard uses 1–9 for the nine drawing tools, E for eraser, W for white ink,
Ctrl+Z / Ctrl+Y for undo/redo, left/right for existing pages, and PageUp/PageDown for tool cycling.
Generic gamepad defaults use evdev codes 304 undo, 305 eraser, 307 redo, 308 white,
310/311 previous/next tool, and 314/315 previous/next page. These are starting points: controller
modes can report different codes. An 8BitDo in keyboard mode uses `key.` bindings; in gamepad
mode it uses `pad.` bindings when exposed as a compatible Linux input device.

```ini
key.1=tool:pencil
key.Ctrl+z=undo
key.Shift+Left=previous-page
key.code106=next-page
pad.305=tool:eraser
pad.311=next-tool
```

Actions: `tool:pencil`, `tool:mech`, `tool:ballpoint`, `tool:fountain`, `tool:fineliner`,
`tool:marker`, `tool:charcoal`, `tool:calligraphy`, `tool:highlighter`, `tool:eraser`, `undo`,
`redo`, `white`, `previous-page`, `next-page`, `previous-tool`, `next-tool`.

Key names include letters, digits, Left/Right/Up/Down, PageUp/PageDown, Space, Enter, Tab,
Escape and `codeNN` (Linux evdev number 1–767). Modifiers Ctrl, Shift, Alt, Meta match exactly.
Gamepad numbers must be 256–767. Later duplicate bindings replace earlier ones. Up to 48 bindings
and 64 KiB of configuration are accepted. An existing empty file deliberately disables mappings.

Shortcuts act only in the idle notebook editor. They do nothing while drawing, pinching,
sleeping or displaying a sheet. Page navigation never creates a new page at the end. Presses
act once; releases and repeats do not trigger actions. Native PIN input is discarded and never
logged. The app logs attachment paths, not key presses. The visible last-input display belongs
to the current unlocked Grimoire session.

The Kindle must already expose the keyboard/controller through `/dev/input/event*`. Grimoire
adds no Bluetooth pairing service or device drivers. It watches up to four external keyboard/
controller devices and checks for hotplug every five seconds without exclusive grabs. Analog
axes and directional hats are not mapped. **Physical 8BitDo compatibility is unconfirmed.**
The simulator tests exercise raw button events and safety gates; they do not prove Bluetooth,
USB host support, controller discovery or real device modifier synchronization.

In the SDL simulator, enabling shortcuts repurposes the number/arrows/E keys from pressure/tilt/
eraser simulation. P and S remain reserved for simulated power/pen-side-button; Escape quits.
