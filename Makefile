# Grimoire build. `build/` is a symlink out of Dropbox (~/.cache/grimoire-build).
# The app is the Rust port (rust/: one crate, rust/PORTING.md), built with cargo; the C tree it was
# ported from (src/) still builds, under c-* names, until it is retired.
#   make headless  scripted runner, no SDL           -> build/host/grimoire-headless
#   make test      unit tests (cargo test)           -> the host build's, then the headless build's
#   make tools     host CLI grimoire-export          -> build/host/grimoire-export
#   make kindle    static armhf binary (cargo zigbuild) -> build/kindle/grimoire
#   make package   Kindle zip (scriptlet + app)      -> build/dist/Grimoire.zip
#   make dev       build a separate "Grimoire (dev)" and copy it over MTP (tools/kindle-dev)
#   make dev-ssh   build, patch and start a separate "Grimoire (dev)" over SSH (tools/kindle-dev-ssh;
#                  --release replaces the release launcher instead)
#   make dist      complete installable bundle       -> dist/Grimoire-VERSION-COMMIT/
#   make check     make rust-check (tests + scenarios + the languages + share server + exporter +
#                  ARM build) + the zip, its launchers, the dev launcher's patch, the KPM package
#                  and the KOReader patches
#   make all       sim + test + tools + kindle + dist
#   make l10n      (maintainer, needs GNU gettext) po/grimoire.pot from the sources, then every
#                  po/<lang>.po merged with it and compiled to po/<lang>.mo
#   make sim       desktop simulator (SDL3 window): the C tree's, not ported -> build/host/grimoire-sim
#   make c-*       the C tree, in build/c: c-headless, c-test (ASan+UBSan), c-tools, c-kindle (zig
#                  cc), c-check (make rust-check's checks against them), c-package (the zip with it)

BUILD   ?= build
# The C tree's binaries (the c-* rules), apart from the app's.
CBUILD  ?= $(BUILD)/c
DIST    ?= dist
HOSTCC  ?= clang
ZIG     ?= zig
KCC     := $(ZIG) cc -target arm-linux-musleabihf -mcpu=cortex_a7
VERSION := $(shell cat VERSION 2>/dev/null || echo 0.1.0)

WARN    := -Wall -Wextra -Wno-unused-parameter -Wno-missing-field-initializers
COMMON  := -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -I$(BUILD)/gen -DGRIMOIRE_VERSION='"$(VERSION)"'

CORE_SRC := src/util.c src/canvas.c src/font.c src/paper.c src/brush.c src/page.c \
            src/history.c src/store.c src/export.c src/stb_impl.c src/calib_fit.c src/bgjob.c \
            src/bgwork_compact.c src/bgwork_template.c
CORE_SRC += src/power.c src/fbcheck.c src/i18n.c
CORE_SRC += src/fbpack.c src/latency.c src/pensync.c
CORE_SRC += src/nav.c
CORE_SRC += src/bgimage.c
CORE_SRC += src/annotate.c src/koreader.c src/shortcuts.c src/hidkeys.c
CORE_SRC += src/pair.c src/live.c src/json_min.c src/btapi.c src/bluetooth.c src/update.c
UI_SRC   := src/ui.c src/keyboard.c src/qr.c third_party/qrcodegen/qrcodegen.c src/tplsheet.c
APP_SRC  := src/app.c src/library.c src/editor.c src/pensettings.c src/calibrate.c \
            src/share.c src/httpd.c src/dispmenu.c src/musicbuild.c src/main.c
APP_SRC  += src/pagesview.c
GEN_SRC  := $(BUILD)/gen/fonts.c $(BUILD)/gen/l10n.c $(BUILD)/gen/eraser_sample.c
WEB_GEN  := $(BUILD)/gen/web.c
KINDLE_PLAT := src/display_fbink.c src/input_evdev.c src/light_kindle.c
SIM_PLAT    := src/sim_sdl.c

FBINK_DIR := third_party/FBInk
FBINK_SRC := $(FBINK_DIR)/fbink.c $(FBINK_DIR)/cutef8/utf8.c $(FBINK_DIR)/cutef8/dfa.c
FBINK_DEF := -DFBINK_FOR_KINDLE -DFBINK_MINIMAL -DFBINK_VERSION='"master-886f25f"'

# The version is compiled in, and the Makefile holds every flag: a change to either rebuilds
# everything.
HDRS := $(wildcard src/*.h) third_party/qrcodegen/qrcodegen.h VERSION Makefile

SDL_CFLAGS := $(shell pkg-config --cflags sdl3 2>/dev/null)
SDL_LIBS   := $(shell pkg-config --libs sdl3 2>/dev/null)

.PHONY: all sim headless test tools tablet kindle package dev dev-ssh dev-check scenarios check clean icon logo l10n l10n-check
.PHONY: c-headless c-test c-tools c-kindle c-check c-package c-icon c-logo
all: sim headless test tools kindle dist

# ---- generated: embedded fonts ----
FONTS := TerminessNerdFontMono-Regular.ttf TerminessNerdFontMono-Bold.ttf Jacquard24-Regular.ttf \
         Jersey10-Regular.ttf DotGothic16-Regular.ttf
# The Classic fonts the Font sheet offers beside Terminess (2.1, Rust only: rust/src/assets.rs embeds
# them; the C tree's build/gen/fonts.c keeps FONTS alone). Made by make fonts-subset.
FONTS_CLASSIC := AtkinsonHyperlegibleNext-Regular.ttf AtkinsonHyperlegibleNext-Bold.ttf \
         NotoSans-Regular.ttf NotoSans-Bold.ttf Literata-Regular.ttf Literata-Bold.ttf
$(BUILD)/gen/fonts.c: $(addprefix assets/,$(FONTS))
	@mkdir -p $(BUILD)/gen
	( cd assets && for f in $(FONTS); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

# The maintainer's handwritten eraser practice, kept as strokes rather than a flat picture.
$(BUILD)/gen/eraser_sample.c: assets/eraser-sample.qpg
	@mkdir -p $(BUILD)/gen
	( echo '#include "erasersample.h"'; xxd -i -n grimoire_eraser_sample $< | sed 's/^unsigned /const unsigned /' ) > $@.tmp && mv $@.tmp $@

# ---- generated: the languages ----
# po/<lang>.po is a language's translations, for translators; po/<lang>.mo is it compiled by
# msgfmt, committed like assets/icon.png so that building needs no gettext. Each .mo is embedded
# like the fonts, with a table src/i18n.c finds them in by name (po/es.mo: "es", po/ja.mo: "ja";
# the language list itself is in src/i18n.c).
L10N_MO := $(sort $(wildcard po/*.mo))
$(BUILD)/gen/l10n.c: $(L10N_MO) Makefile
	@mkdir -p $(BUILD)/gen
	( echo '// The catalogs of po/*.mo, embedded by make (see the Makefile).'; echo '#include "i18n.h"'; \
	  for f in $(notdir $(L10N_MO)); do ( cd po && xxd -i $$f ); done; \
	  echo 'const I18nCatalog i18n_catalogs[] = {'; \
	  for f in $(notdir $(L10N_MO)); do v=$$(printf '%s' "$$f" | tr -c 'A-Za-z0-9' _); \
	    echo "    {\"$${f%.mo}\", $$v, sizeof $$v},"; done; \
	  echo '    {0, 0, 0},'; echo '};' ) > $@.tmp && mv $@.tmp $@

# ---- generated: the share studio's page (served inline by httpd.c) ----
WEB := studio.js studio.css download.js live.js live.css
$(WEB_GEN): $(addprefix web/,$(WEB))
	@mkdir -p $(BUILD)/gen
	( cd web && for f in $(WEB); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

# ---- the app: the Rust port (rust/: one crate; rust/PORTING.md has the rules) ----
# The same app as the C tree, module for module, built with cargo, its binaries where the C's went:
# $(BUILD)/host/grimoire-headless, $(BUILD)/host/grimoire-export, $(BUILD)/kindle/grimoire.
# Homebrew's rustc shadows rustup's on this Mac: RUST_BIN puts rustup's stable toolchain first.
# cargo runs inside rust/ (its .cargo/config.toml, the Kindle target's flags, is found from there),
# with its target dir under $(BUILD) (out of Dropbox, as build/ is: a cargo target inside Dropbox
# breaks). cargo decides what to rebuild, so each binary's rule runs it every time (FORCE) and
# copies the binary only when its bytes changed: an unchanged one keeps its time, and what is made
# from it (the zip) is not made again for nothing.
CARGO     ?= cargo
RUST_BIN  ?= $(HOME)/.rustup/toolchains/stable-aarch64-apple-darwin/bin
RTARGET   ?= $(abspath $(BUILD))/cargo
RKINDLE   := armv7-unknown-linux-musleabihf
RCARGO    = cd rust && $(1) PATH="$(RUST_BIN):$$PATH" CARGO_TARGET_DIR="$(RTARGET)" $(CARGO)
RCOPY     = mkdir -p $(dir $(2)) && { cmp -s $(1) $(2) || { cp $(1) $(2).tmp && mv $(2).tmp $(2); }; }
# The Kindle binary names no path of the machine that built it (panic locations name sources):
# a build at another place or on another Mac makes the same bytes, as kindle/no-comment.ld does for
# the toolchain's names. (cargo joins this to .cargo/config.toml's rustflags.) cargo-zigbuild links
# with $(ZIG), the compiler the C build used, as it compiles the vendored C (FBInk, stb, qrcodegen).
RKFLAGS   := --config 'target.$(RKINDLE).rustflags=["--remap-path-prefix=$(HOME)=/home","--remap-path-prefix=$(CURDIR)=/grimoire"]'
RKENV     = CARGO_ZIGBUILD_ZIG_PATH="$(ZIG)"

headless: $(BUILD)/host/grimoire-headless
$(BUILD)/host/grimoire-headless: FORCE
	$(call RCARGO) build --features sim --bin grimoire-headless
	@$(call RCOPY,$(RTARGET)/debug/grimoire-headless,$@)

# ---- simulator (the C tree's SDL3 window: a development aid, not ported) ----
SIM_FLAGS := -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer
sim: $(BUILD)/host/grimoire-sim
$(BUILD)/host/grimoire-sim: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) $(SDL_CFLAGS) -DGRIMOIRE_SIM -DGRIMOIRE_SIM_SDL \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) -o $@ $(SDL_LIBS) -lm

# The C tree's scripted runner.
c-headless: $(CBUILD)/host/grimoire-headless
$(CBUILD)/host/grimoire-headless: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) $(HDRS)
	@mkdir -p $(CBUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) -DGRIMOIRE_SIM \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) -o $@ -lm

# Every tests/scenarios/*.qs against a fresh data root (a script with a "# root: NAME" line starts
# from a copy of scenario NAME's data, once NAME has run; a run ends only once the PDF
# exports it started are written, so that data is whole); screenshots in build/shots/<name>/. A
# failed expect makes a run exit with status 3, and a run that leaves a file half-written (*.tmp)
# fails too, as does one whose "# pdfs: N" line does not match the PDFs left in its root's PDFs/
# (the mirror): every scenario still runs, then the rule fails naming them. "# env: K=V" lines
# set a scenario's environment; nothing else of the caller's reaches the app's test hooks.
# "# copy: DIR DEST" lines copy what the folder DIR (in the repository, e.g. tests/fixtures/...)
# holds into DEST under the scenario's data root before the run (templates, import folders). Last,
# the runner checks itself: tests/selftest/pass.qs must exit 0 with its notebook's PDF written,
# and in fail.qs every line but the comments, waits and taps must be reported as failed, with
# exit status 3. Then the same for tests/selftest/pass-rust.qs and fail-rust.qs, which use what only
# the Rust build has (its commands and probes). With SCEN_SKIP_RUST_ONLY set (make c-check sets it),
# a scenario with a "# rust-only" line is skipped, unless tests/c-scenarios/ has its C variant, which runs in
# its place (the two roots whose data others use: b_library, which share_check, the exporter check, d_persist and
# x_turn_calibrated start from, and z_import_1_startup);
# one whose "# root:" was skipped is skipped with it (it would copy a data root that was never made); and so
# are the two rust selftests.
# tests/scenario_runner.py runs them, SCEN_JOBS at a time (default: the number of CPUs; SCEN_JOBS=1 is
# one after another), each with a port of its own for the share server (GRIMOIRE_TEST_PORT, else
# 8765, plus the job's slot), the longest first by tests/scenario-times.txt; a dependant starts once
# its root has finished. Each run's output is in build/scen-logs/NAME.log; one line per run is
# printed as it ends, the failed runs' logs whole at the end, then "SCENARIOS FAILED: ..." as before.
SIM_HOOKS := GRIMOIRE_WIFI_SCRIPT GRIMOIRE_HOST GRIMOIRE_PASSCODE GRIMOIRE_SIM_ADDRS GRIMOIRE_HTTPD_DELAY_MS GRIMOIRE_BG_DELAY_MS GRIMOIRE_SIM_PRESSURE \
             GRIMOIRE_SIM_PEN_BUTTON2 GRIMOIRE_SIM_TOUCH_SLOTS GRIMOIRE_TOUCH_TUNE SIM_TOUCH_SLOTS TOUCH_TUNE \
             GRIMOIRE_BT_API GRIMOIRE_UPDATE_SCRIPT GRIMOIRE_UPDATE_REQUEST FAKE_UPDATE_PREVIEW FAKE_UPDATE_STABLE FAKE_UPDATE_FAIL \
             GRIMOIRE_SIM_STATS GRIMOIRE_SIM_SIZE GRIMOIRE_CHECK_FIT GRIMOIRE_LANG GRIMOIRE_L10N_DIR GRIMOIRE_ANNOTATE \
             GRIMOIRE_SLEEP_MINUTE_MS GRIMOIRE_SIM_RAWPEN GRIMOIRE_SIM_RAWPEN_MAIN GRIMOIRE_SIM_STALL_MS \
             GRIMOIRE_LIVE_PEN_SOURCE GRIMOIRE_SIM_MAP_NOMEM GRIMOIRE_KEEP_RENDER_MS GRIMOIRE_SIM_LATE_MS GRIMOIRE_SIM_CLOCK \
             GRIMOIRE_SIM_DENSE_DRAW_MS GRIMOIRE_SIM_CORES GRIMOIRE_SIM_BACKLINKS_STEP_MS GRIMOIRE_CACHE_WINDOW_MS GRIMOIRE_STORAGE \
             GRIMOIRE_IMPORT_STABLE GRIMOIRE_PDF_SLICE_PAGES GRIMOIRE_PDF_BANDS GRIMOIRE_PDF_NO_MAP GRIMOIRE_PDF_FREE_BYTES GRIMOIRE_PDF_WATCHDOG_MS \
             GRIMOIRE_PDF_TEST_CRASH_PAGE GRIMOIRE_PDF_TEST_HANG_PAGE GRIMOIRE_PDF_TEST_ALLOC_PAGE GRIMOIRE_WORK_FAIL_STEP GRIMOIRE_WORK_EXE GRIMOIRE_SANDBOX
# The scenarios tap by coordinates and check English text: they run in English whatever the
# caller's LANG says (GRIMOIRE_LANG ranks first), unless a scenario's "# env:" line sets another.
SCEN_LANG := GRIMOIRE_LANG=en
# SCENARIOS: the scripts the rule runs (all of them); SCEN_ENV: test hooks every run gets on top of
# its own "# env:" lines; SCEN_NO_SELFTEST: leave out the runner's self-check (scenarios-late does);
# SCEN_JOBS: runs at once; SCEN_SHARD=I/N: only CI shard I of N (whole # root: chains, balanced by
# the timing table; the selftest goes with shard 1); SCEN_TIMES: where the run's times are written
# (make scenario-times reads them).
SCENARIOS ?= tests/scenarios/*.qs
SCEN_RUN  := python3 -I tests/scenario_runner.py
SCEN_JOBS ?=
SCEN_SHARD ?=
SCEN_TIMES ?= $(BUILD)/scen-times.txt
# The scenarios that run alone, after the parallel batch, because they depend on an idle machine and
# cannot be made not to yet (each with its reason; none today).
SCEN_SERIAL ?=
scenarios: $(BUILD)/host/grimoire-headless
	@$(SCEN_RUN) run --build $(BUILD) --jobs "$(SCEN_JOBS)" --unset "$(SIM_HOOKS)" --env "$(SCEN_ENV)" \
	  --serial "$(SCEN_SERIAL)" $(if $(SCEN_SKIP_RUST_ONLY),--skip-rust-only) $(if $(SCEN_NO_SELFTEST),--no-selftest) \
	  $(if $(SCEN_SHARD),--shard $(SCEN_SHARD)) $(if $(SCEN_TIMES),--times-out $(SCEN_TIMES)) $(SCENARIOS)

# The timing table the runner starts the longest scenarios first by, and the CI shards are balanced
# by (tests/scenario-times.txt), from a full run of the scenarios: make scenario-times, then commit it.
.PHONY: scenario-times
scenario-times: scenarios
	$(SCEN_RUN) times --build $(BUILD)

# The scenarios that time a press or a finger shape by their waits, run again with every wait ending
# 400 ms late (GRIMOIRE_SIM_LATE_MS), as a loaded machine's timers wake: the release runner's did, and
# these two failed there on every release while passing here. A script times a contact by the
# script's own clock (docs/SIM.md), so they hold however late the machine is. After the scenarios
# (s_shape_4_guards starts from s_shape_1_record's data). z_pages_drag_hold times the Pages view's
# 300 ms drag hold against the card's 600 ms timer, which reads the real clock: its short holds are
# frame lines, so a late wake cannot open the card first. u_keyboard_hold_delete times a held delete's repeats by
# frame lines, against the screen's tick, which reads the real clock.
# z_links_3_bar_learns runs late for the waits around its learning (the Links bar opened before the
# check is through, the hold on a row past the long press). Late waits do not slow the backlinks check
# itself, as a slow runner did: the scenario's own GRIMOIRE_SIM_BACKLINKS_STEP_MS does, in both runs,
# and its waitfor waits for the rows however long the check takes.
LATE_SCENARIOS := y_status_press s_shape_4_guards z_pages_drag_hold u_keyboard_hold_delete y_status_corner z_rc8_pages_refresh \
  z_links_3_bar_learns
.PHONY: scenarios-late
scenarios-late: $(BUILD)/host/grimoire-headless
	@echo "== the scenarios again with every wait 400 ms late: $(LATE_SCENARIOS)"
	@$(MAKE) --no-print-directory -o $(BUILD)/host/grimoire-headless scenarios SCEN_NO_SELFTEST=1 SCEN_TIMES= \
	  SCEN_SHARD= SCENARIOS="$(LATE_SCENARIOS:%=tests/scenarios/%.qs)" SCEN_ENV=GRIMOIRE_SIM_LATE_MS=400

# ---- languages ----
# For the maintainer, with GNU gettext installed: xgettext collects every marked string of
# rust/src (with the TRANSLATORS comment above it) into po/grimoire.pot, and the share studio's
# (web/studio.js: t, tPages and N_) under the context "studio", which httpd.rs sends the studio as
# window.GRIMOIRE_L10N. msgmerge brings each po/<lang>.po up to date with it, and msgfmt --check
# compiles each to po/<lang>.mo, failing on a translation whose printf conversions (positional
# ones too) differ from its English. Commit what changed. A new language is a new po/<lang>.po
# (msginit) plus its line in rust/src/i18n.rs.
# The Rust sources mark their strings with calls the C parser reads as it read src/*.c's macros
# (tr for _, n_ for N_, c_ for C_, nc_ for NC_, ng_ for NG_), and it is the parser that reads them:
# the Rust one (gettext 0.24 and later) leaves the printf formats unmarked (it knows only Rust's own
# {} formats), and without "c-format" msgfmt --check would no longer compare a translation's
# conversions with the English. (It warns of unterminated character constants: Rust's lifetimes,
# which it reads as C's quotes, to the end of their line.)
XGETTEXT_FLAGS := --from-code=UTF-8 --language=C --add-comments=TRANSLATORS --add-location=file \
                  -ktr -kn_ -kc_:1c,2 -knc_:1c,2 -kng_:1,2 \
                  --flag=tr:1:pass-c-format --flag=c_:2:pass-c-format \
                  --flag=ng_:1:pass-c-format --flag=ng_:2:pass-c-format \
                  --package-name=Grimoire --package-version=$(VERSION) \
                  --msgid-bugs-address=https://github.com/ganten1998/grimoire-scribe/issues
XGETTEXT_JS_FLAGS := --from-code=UTF-8 --language=JavaScript --add-comments=TRANSLATORS --add-location=file \
                     -kt -kN_ -ktPages:1,2
# No hash table and one byte order: the same .po makes the same .mo on any machine (l10n-check).
MSGFMT_FLAGS := --check --no-hash --endianness=little
l10n:
	@mkdir -p $(BUILD)/l10n
	xgettext $(XGETTEXT_FLAGS) -o $(BUILD)/l10n/app.pot $(sort $(wildcard rust/src/*.rs rust/src/editor/*.rs))
	xgettext $(XGETTEXT_JS_FLAGS) -o $(BUILD)/l10n/studio.pot web/studio.js
	awk '/^msgid / { if (n++) print "msgctxt \"studio\"" } { print }' $(BUILD)/l10n/studio.pot > $(BUILD)/l10n/studio-ctx.pot
	msgcat --use-first -o po/grimoire.pot $(BUILD)/l10n/app.pot $(BUILD)/l10n/studio-ctx.pot
	for p in $(wildcard po/*.po); do msgmerge --quiet --update --backup=none $$p po/grimoire.pot || exit 1; done
	for p in $(wildcard po/*.po); do msgfmt $(MSGFMT_FLAGS) -o $${p%.po}.mo $$p || exit 1; done

# The languages, checked (part of make check). Each committed po/<lang>.mo must be what msgfmt
# makes of po/<lang>.po now: a .po changed without make l10n fails (where gettext is not
# installed this part is skipped, and says so). Then tests/scenarios/y_all_screens.qs, which visits
# every screen and sheet, runs in every language, Gothic and Classic, with GRIMOIRE_CHECK_FIT=1:
# a run that cuts any text short to fit lists each ("fit[es]: ...") and exits with status 3. Its
# shots, build/shots/l10n/<lang>-<style>/, show translators every screen in context. Then, in each
# language, tests/l10n/long_root.qs opens Settings from a data root whose path is longer than the
# sheet is wide: its Data line, a path, is cut short as names are, which the check must not count
# (else make check would fail in a deep build directory). Last, tests/l10n/hosted.qs, in every
# language and style: the Library's top bar as KOReader opens it (Back to KOReader in Exit's place,
# the one button that takes the width it needs). Then tests/l10n/templates.qs, in every language and
# style: the Templates sheet with templates in it, over New notebook and the editor's page, on a
# data root its "# copy:" line fills first, as make scenarios does. Then (Rust build only) the
# unified Index, the notebook links selector, page management and backlinks scenarios from their
# fixtures, and z_wifi.qs with and without an address, in every language and style. Under make check
# it waits for
# the scenarios: both give their runs the same ports. With SCEN_SKIP_RUST_ONLY set (make
# c-check), y_all_screens.qs is skipped: it visits screens only the Rust build has.
# The runs go through tests/scenario_runner.py, SCEN_JOBS at a time, as the scenarios do (logs in
# build/scen-logs/l10n/). L10N_SKIP names language runs of y_all_screens.qs to leave out: make
# rust-check (and CI) skip en-gothic, the run make scenarios already made (the same script in English
# and Gothic, the default style, with the same GRIMOIRE_CHECK_FIT=1 and GRIMOIRE_SIM_ADDRS from its
# "# env:" lines; its "# settings:" line, status_mode=1, is what every settings.ini reads as anyway).
L10N_SKIP ?=
L10N_SCENARIO := tests/scenarios/y_all_screens.qs
# (the Rust build's panel has a button more, so its Paper button is elsewhere: it runs templates-rust.qs)
L10N_TEMPLATES := $(if $(SCEN_SKIP_RUST_ONLY),tests/l10n/templates.qs,tests/l10n/templates-rust.qs)
L10N_LANGS := en $(basename $(notdir $(L10N_MO)))
l10n-check: $(BUILD)/host/grimoire-headless $(if $(filter check,$(MAKECMDGOALS)),scenarios)
	@$(SCEN_RUN) l10n --build $(BUILD) --jobs "$(SCEN_JOBS)" --unset "$(SIM_HOOKS)" --langs "$(L10N_LANGS)" \
	  --scenario $(L10N_SCENARIO) --templates $(L10N_TEMPLATES) --msgfmt-flags "$(MSGFMT_FLAGS)" \
	  --skip "$(L10N_SKIP)" $(if $(SCEN_SKIP_RUST_ONLY),--skip-rust-only)

# ---- host tools (rust/src/bin: tools/*.c's ports) ----
tools: $(BUILD)/host/grimoire-export $(BUILD)/host/tgstudy
$(BUILD)/host/grimoire-export: FORCE
	$(call RCARGO) build --bin grimoire-export
	@$(call RCOPY,$(RTARGET)/debug/grimoire-export,$@)
$(BUILD)/host/tgstudy: FORCE
	$(call RCARGO) build --bin tgstudy
	@$(call RCOPY,$(RTARGET)/debug/tgstudy,$@)

# The Library cover, drawn with the gothic widgets (committed; run by hand after a design change).
icon: $(BUILD)/host/mkicon
	$(BUILD)/host/mkicon assets/icon.png
$(BUILD)/host/mkicon: FORCE
	$(call RCARGO) build --bin mkicon
	@$(call RCOPY,$(RTARGET)/debug/mkicon,$@)

# The Terminess diet (committed, like the icon; run by hand, never by make check): the two
# assets/TerminessNerdFontMono-*.ttf cut from Nerd Fonts' release down to the code points
# assets/terminess-keep.txt lists. tools/fonts_subset.py downloads the release and upstream Terminus
# TTF once into FONTS_CACHE (outside the repo), checks them by SHA-256, checks the list's text against
# upstream Terminus, and writes the fonts (assets/README.md). It needs fontTools 4.68.0 (pip install
# fonttools==4.68.0, the version that made the committed fonts), best in a venv outside the repo: make
# fonts-subset FONTS_PYTHON=<venv>/bin/python. Then the Classic fonts (2.1): tools/fonts_classic.py
# downloads Atkinson Hyperlegible Next, Noto Sans and Literata from Google Fonts' repository at pinned
# commits into the same cache, checks them by SHA-256, and writes each family's Regular and Bold, static
# and subset, with its OFL text (FONTS_CLASSIC below). FONTS_SUBSET_FLAGS=--verify writes nothing and
# checks that the committed fonts are what the recipes make.
FONTS_PYTHON ?= python3
FONTS_CACHE ?= $(HOME)/.cache/grimoire-fonts
FONTS_SUBSET_FLAGS ?=
.PHONY: fonts-subset
fonts-subset:
	$(FONTS_PYTHON) -I tools/fonts_subset.py --cache "$(FONTS_CACHE)" --keep assets/terminess-keep.txt --out assets $(FONTS_SUBSET_FLAGS)
	$(FONTS_PYTHON) -I tools/fonts_classic.py --cache "$(FONTS_CACHE)" --out assets $(FONTS_SUBSET_FLAGS)

# The logo for the README and releases, black and white on transparent (committed, like the icon).
# Pixel blocks of 8: shown at half size they land on whole pixels on ordinary and Retina screens.
logo: $(BUILD)/host/mklogo
	$(BUILD)/host/mklogo crest 8 docs/logo.png
	$(BUILD)/host/mklogo crest 8 docs/logo-dark.png white
$(BUILD)/host/mklogo: FORCE
	$(call RCARGO) build --bin mklogo
	@$(call RCOPY,$(RTARGET)/debug/mklogo,$@)

# The Kindle Scribe's pen as a Mac tablet, over SSH (docs/TABLET.md; macOS only, not in check:
# tests/tablet_check.sh checks it).
tablet: $(BUILD)/host/scribe-tablet
$(BUILD)/host/scribe-tablet: tools/scribe-tablet.c
	@mkdir -p $(BUILD)/host
	$(HOSTCC) -std=gnu11 $(WARN) -O2 tools/scribe-tablet.c -o $@ -framework ApplicationServices

# MTP: a separate "Grimoire (dev)" with its own data root; the release and its notebooks stay
# untouched. The Kindle must be in USB file transfer (MTP) mode, and the app is tapped in the
# Library afterwards (MTP cannot start it). tools/kindle-dev --mount/--name/--root/--no-build/--zip.
dev:
	tools/kindle-dev

# SSH: build, patch and start a separate "Grimoire (dev)" on a Kindle over SSH (tools/kindle-dev-ssh:
# GRIMOIRE_HOST, default "kindle"; --release replaces the release launcher, keeping a backup;
# --no-build, --zip and --logs).
dev-ssh:
	tools/kindle-dev-ssh

# The dev launcher's patch, headlessly: tools/kindle-dev into a scratch mount, the launcher it
# patches out of the zip, and the refusals that keep --name and --root off the release
# (tests/kindle_dev_check.sh).
dev-check: package
	tests/kindle_dev_check.sh $(BUILD)/dist/Grimoire.zip

# The C tree's: tools/grimoire-export.c, mkicon.c and mklogo.c (the same bytes as the Rust ones).
c-tools: $(CBUILD)/host/grimoire-export
$(CBUILD)/host/grimoire-export: tools/grimoire-export.c $(CORE_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(CBUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -O2 tools/grimoire-export.c $(CORE_SRC) $(GEN_SRC) -o $@ -lm
c-icon: $(CBUILD)/host/mkicon
	$(CBUILD)/host/mkicon assets/icon.png
$(CBUILD)/host/mkicon: tools/mkicon.c $(CORE_SRC) src/ui.c $(GEN_SRC) $(HDRS)
	@mkdir -p $(CBUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -Itools -O2 tools/mkicon.c $(CORE_SRC) src/ui.c $(GEN_SRC) -o $@ -lm
c-logo: $(CBUILD)/host/mklogo
	$(CBUILD)/host/mklogo crest 8 docs/logo.png
	$(CBUILD)/host/mklogo crest 8 docs/logo-dark.png white
$(CBUILD)/host/mklogo: tools/mklogo.c $(CORE_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(CBUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -Itools -O2 tools/mklogo.c $(CORE_SRC) $(GEN_SRC) -o $@ -lm

# ---- unit tests (rust/tests: tests/test_*.c's ports, golden renders included) ----
# One test at a time: the ported tests keep the C's process-wide state (the store's root, the
# language), as each C test binary runs its tests in order. Twice: on the host's platform, then in
# the headless build (--features sim), whose tests check the runner itself, the Share screen and
# the pen that reports tilt.
# (test-host and test-sim are the two halves, which CI runs as two jobs.)
.PHONY: test-host test-sim
test:
	@$(MAKE) --no-print-directory test-host
	@$(MAKE) --no-print-directory test-sim
	@echo "ALL TESTS PASSED"
test-host:
	@mkdir -p $(BUILD)/test/tmp
	$(call RCARGO,GRIMOIRE_TEST_TMP="$(abspath $(BUILD))/test/tmp") test -- --test-threads=1
test-sim:
	@mkdir -p $(BUILD)/test/tmp
	$(call RCARGO,GRIMOIRE_TEST_TMP="$(abspath $(BUILD))/test/tmp") test --features sim -- --test-threads=1

# The PDFs the scenarios copy (tests/fixtures/pdf/), made by the fixture writer in rust/tests/pdffix, never
# edited by hand; rust/tests/test_pdf_fixtures.rs checks the committed bytes are what it makes now.
.PHONY: pdf-fixtures
pdf-fixtures:
	@mkdir -p $(abspath tests/fixtures/pdf)
	$(call RCARGO,GRIMOIRE_WRITE_PDF_FIXTURES="$(abspath tests/fixtures/pdf)") test --test test_pdf_fixtures

# The C tree's unit tests (core + ui only; no platform).
TESTS := $(patsubst tests/%.c,$(CBUILD)/test/%,$(wildcard tests/test_*.c))
# Contraction (fused multiply-add) on, as clang does by default: tests/test_golden.c's renders
# were recorded so, and turning it off changes them.
TEST_FLAGS := -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=undefined -fno-omit-frame-pointer -ffp-contract=on
c-test: $(TESTS)
	@set -e; for t in $(TESTS); do echo "== $$t"; GRIMOIRE_TEST_TMP=$(CBUILD)/test/tmp $$t; done; echo "ALL TESTS PASSED"
$(CBUILD)/test/%: tests/%.c $(wildcard tests/*.h) $(CORE_SRC) $(UI_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(CBUILD)/test/tmp
	$(HOSTCC) $(COMMON) $(WARN) $(TEST_FLAGS) -Itests $< $(CORE_SRC) $(UI_SRC) $(GEN_SRC) -o $@ -lm

# ---- kindle ----
# The static armhf binary: cargo zigbuild, with rust/.cargo/config.toml's flags (cortex-a7, static,
# kindle/no-comment.ld, which leaves out the toolchain's name, so any build of the same zig and
# rustc makes the same binary: a local one and the release workflow's) and RKFLAGS.
kindle: $(BUILD)/kindle/grimoire
$(BUILD)/kindle/grimoire: FORCE
	$(call RCARGO,$(RKENV)) zigbuild --release --target $(RKINDLE) --features kindle --bin grimoire $(RKFLAGS)
	@$(call RCOPY,$(RTARGET)/$(RKINDLE)/release/grimoire,$@)

# The C tree's (zig cc).
c-kindle: $(CBUILD)/kindle/grimoire
# FBInk is third-party: built on its own with warnings muted (an upstream format-string bug in
# fbink_state_dump, which Grimoire never calls); our sources stay warning-clean.
FBINK_OBJ := $(patsubst $(FBINK_DIR)/%.c,$(CBUILD)/kindle/fbink/%.o,$(FBINK_SRC))
$(CBUILD)/kindle/fbink/%.o: $(FBINK_DIR)/%.c
	@mkdir -p $(dir $@)
	$(KCC) -std=gnu11 -D_GNU_SOURCE -O2 -w $(FBINK_DEF) -I$(FBINK_DIR) -c $< -o $@
# kindle/no-comment.ld leaves out the toolchain's name, so any build of the same zig version makes
# the same binary (a local zig and the release workflow's).
$(CBUILD)/kindle/grimoire: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(KINDLE_PLAT) $(GEN_SRC) $(WEB_GEN) $(FBINK_OBJ) $(HDRS) kindle/no-comment.ld
	@mkdir -p $(CBUILD)/kindle
	$(KCC) $(COMMON) $(WARN) -O2 -I$(FBINK_DIR) $(FBINK_DEF) -DGRIMOIRE_KINDLE -static -s -Wl,-T,kindle/no-comment.ld \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(KINDLE_PLAT) $(GEN_SRC) $(WEB_GEN) $(FBINK_OBJ) -o $@ -lm

# ---- Kindle package (scriptlet for Vera / KPM-era jailbreaks) ----
# One self-contained file: documents/Grimoire.sh (Library entry, icon embedded) with the app tar
# appended after a marker line; it unpacks itself into /mnt/us/grimoire/app on first run. Copying
# a single file into documents/ over MTP is all an install or update takes.
# The app also carries CAPABILITIES, the words of Grimoire.sh's "# Capabilities:" line (for a tool
# that must not run a launcher to learn what it supports), and licenses/: the GPL, the fonts'
# licenses, and one for each third-party part of the binary. Every third_party/<dir> ships its
# LICENSE* / COPYING* as <dir>-LICENSE.txt (and CREDITS* as <dir>-CREDITS.txt); the stb headers
# carry their license inside, shipped once as stb-LICENSE.txt; the binary is linked static
# against zig's musl, whose COPYRIGHT ships as musl-COPYRIGHT.txt, and zig cc links its own
# compiler-rt into it (64-bit division, float conversions, memcpy and more), so zig's LICENSE
# ships as zig-compiler-rt-LICENSE.txt. The binary is Rust's: rustc links the Rust standard library
# into it (MIT and Apache-2.0, and the crates it is built from, each under its own terms), whose
# notices ship as rust-std-LICENSE.txt (the toolchain's share/doc/rust/COPYRIGHT-library.html,
# as text), and the libc crate, rust-libc-LICENSE.txt. FBInk compiles in two parts of its own tree with their
# own notices: cutef8 (cutef8/utf8.c) and Bjoern Hoehrmann's UTF-8 decoder (cutef8/dfa.c). Their
# notices are only in the sources' heads and FBInk's CREDITS, so cutef8-LICENSE.txt and
# dfa-LICENSE.txt carry both, as the tree has them.
# KOReader's Grimoire plugin (kindle/koreader/grimoire.koplugin) goes twice: in the app, as
# app/koreader/grimoire.koplugin, which a Library start of Grimoire.sh installs into KOReader; and
# in the zip as "KOReader plugin (optional)/grimoire.koplugin", for a hand install into the
# Kindle's koreader/plugins. Never as a top-level koreader folder: copied over MTP, that could
# replace someone's KOReader.
# tools/archive.py writes the app's tar and both zips: one time for every entry (the HEAD commit's,
# or SOURCE_DATE_EPOCH), owner root, modes 755 and 644, names in order. Nothing else here may
# depend on the builder either: the stb notice names the headers in $(sort) order, not the
# locale's. With the same zig version, the same commit makes the same bytes on any Mac, so
# BUILD_ID and every release file can be compared with a local build (tests/repro_check.sh, part
# of make package-check, builds twice from clean and compares). $(BUILD)/dist/source-date holds
# that time and changes only when it does: a new commit makes the zip again even when no file in
# it changed, so the zip never keeps an older commit's time.
PKG := $(BUILD)/dist/stage
RESET_SH := kindle/Grimoire\ -\ reset\ pen\ calibration.sh
VENDOR_LICENSES := $(wildcard third_party/*/LICENSE* third_party/*/COPYING* third_party/*/CREDITS*)
KOPLUGIN_SRC := $(wildcard kindle/koreader/grimoire.koplugin/*.lua)
KOPATCH_SRC := $(wildcard kindle/koreader/patches/*.lua)
KOLEGACY_SRC := $(wildcard kindle/koreader/legacy/*.lua)
KOPLUGIN_ZIP := KOReader plugin (optional)
# Terminess's icons are Material Design Icons (Apache-2.0): their notice and the license's text go
# beside the fonts' OFL texts, in app/licenses and in the KOReader patches' fonts/Grimoire.
ICON_LICENSES := assets/MaterialDesignIcons-LICENSE.txt assets/MaterialDesignIcons-Apache-2.0.txt
package: $(BUILD)/dist/Grimoire.zip
$(BUILD)/dist/Grimoire.zip: $(BUILD)/kindle/grimoire kindle/Grimoire.sh kindle/app/grimoire.sh kindle/app/bluetooth.sh kindle/app/wifi.sh kindle/app/update.sh $(RESET_SH) \
                           assets/icon.png kindle/README.txt LICENSE VERSION Makefile tools/archive.py $(BUILD)/dist/source-date $(wildcard assets/*-OFL-1.1.txt) $(ICON_LICENSES) \
                           $(wildcard third_party/stb_*.h) $(VENDOR_LICENSES) $(FBINK_SRC) $(KOPLUGIN_SRC) $(KOPATCH_SRC) $(KOLEGACY_SRC) kindle/koreader/PATCHES.txt
	rm -rf $(PKG) && mkdir -p $(PKG)/documents $(PKG)/app/licenses
	cp $(BUILD)/kindle/grimoire kindle/app/grimoire.sh kindle/app/bluetooth.sh kindle/app/wifi.sh kindle/app/update.sh $(PKG)/app/
	mkdir -p $(PKG)/app/koreader/grimoire.koplugin "$(PKG)/$(KOPLUGIN_ZIP)/grimoire.koplugin"
	cp $(KOPLUGIN_SRC) $(PKG)/app/koreader/grimoire.koplugin/
	mkdir -p $(PKG)/app/koreader/patches "$(PKG)/KOReader patches (optional)"
	cp $(KOPATCH_SRC) $(PKG)/app/koreader/patches/
	mkdir -p $(PKG)/app/koreader/legacy
	cp $(KOLEGACY_SRC) $(PKG)/app/koreader/legacy/
	cp $(KOPATCH_SRC) "$(PKG)/KOReader patches (optional)/"
	mkdir -p "$(PKG)/KOReader patches (optional)/fonts/Grimoire"
	cp assets/TerminessNerdFontMono-*.ttf "$(PKG)/KOReader patches (optional)/fonts/Grimoire/"
	cp assets/Terminess-OFL-1.1.txt $(ICON_LICENSES) "$(PKG)/KOReader patches (optional)/fonts/Grimoire/"
	cp kindle/koreader/PATCHES.txt "$(PKG)/KOReader patches (optional)/README.txt"
	cp $(KOPLUGIN_SRC) "$(PKG)/$(KOPLUGIN_ZIP)/grimoire.koplugin/"
	sed -n 's/^# Capabilities: //p' kindle/Grimoire.sh > $(PKG)/app/CAPABILITIES
	cp LICENSE $(PKG)/app/licenses/GPL-3.0.txt
	cp assets/*-OFL-1.1.txt $(ICON_LICENSES) $(PKG)/app/licenses/
	for d in third_party/*/; do d=$${d%/}; n=$${d##*/}; out=$(PKG)/app/licenses/$$n-LICENSE.txt; \
	  for f in $$d/LICENSE* $$d/COPYING*; do [ -f "$$f" ] || continue; \
	    [ -s $$out ] && printf '\n\n' >> $$out; cat "$$f" >> $$out; done; \
	  for f in $$d/CREDITS*; do [ -f "$$f" ] && cp "$$f" $(PKG)/app/licenses/$$n-CREDITS.txt; done; \
	done; true
	names=; : > $(PKG)/stb.all; for h in $(sort $(wildcard third_party/stb_*.h)); do \
	  awk '/^This software is available under 2 licenses/ { p = 1 } p && /^\*\// { exit } p' $$h > $(PKG)/stb.one; \
	  [ -s $(PKG)/stb.one ] || { echo "package: no license text found in $$h"; exit 1; }; \
	  names="$${names:+$$names, }$${h##*/}"; cmp -s $(PKG)/stb.one $(PKG)/stb.all || cat $(PKG)/stb.one >> $(PKG)/stb.all; \
	done; { echo "$$names: single-file libraries by Sean Barrett (https://github.com/nothings/stb)."; echo; \
	  cat $(PKG)/stb.all; } > $(PKG)/app/licenses/stb-LICENSE.txt; rm -f $(PKG)/stb.one $(PKG)/stb.all
	lib=$$($(ZIG) env 2>/dev/null | sed -n 's/.*lib_dir"* *[=:] *"\([^"]*\)".*/\1/p'); \
	  if [ -f "$$lib/libc/musl/COPYRIGHT" ]; then cat "$$lib/libc/musl/COPYRIGHT" > $(PKG)/app/licenses/musl-COPYRIGHT.txt; \
	  else echo "package: musl's COPYRIGHT is not in zig's lib dir ($$lib)"; exit 1; fi; \
	  lic=; for f in "$$lib/compiler_rt/LICENSE" "$$lib/../LICENSE" "$$lib/../../LICENSE"; do \
	    if grep -qs 'Zig contributors' "$$f"; then lic=$$f; break; fi; done; \
	  if [ -z "$$lic" ]; then echo "package: zig's LICENSE (for compiler-rt) is not beside zig's lib dir ($$lib)"; exit 1; fi; \
	  { echo "From zig (https://ziglang.org): compiler-rt and the rest of the Zig code that zig cc links into"; \
	    echo "the Kindle binary. Zig's license:"; echo; cat "$$lic"; } > $(PKG)/app/licenses/zig-compiler-rt-LICENSE.txt
	set -e; sysroot=$$(PATH="$(RUST_BIN):$$PATH" rustc --print sysroot); doc=$$sysroot/share/doc/rust/COPYRIGHT-library.html; \
	  if [ ! -s "$$doc" ]; then echo "package: the Rust standard library's notices are not in the toolchain ($$doc)"; exit 1; fi; \
	  { echo "From Rust (https://www.rust-lang.org): its standard library (core, alloc, std and the crates they are"; \
	    echo "built from), which rustc links into the Kindle binary. Its notices, as the toolchain ships them"; \
	    echo "(share/doc/rust/COPYRIGHT-library.html, here as text):"; echo; \
	    python3 -I -c 'import html, re, sys; t = open(sys.argv[1], encoding="utf-8").read(); t = re.sub(r"(?is)<(script|style)\b.*?</\1>", "", t); t = html.unescape(re.sub(r"<[^>]+>", "", t)); t = re.sub(r"\n{3,}", "\n\n", re.sub(r"[ \t]+\n", "\n", t)); sys.stdout.buffer.write((t.strip() + "\n").encode("utf-8"))' "$$doc"; \
	  } > $(PKG)/app/licenses/rust-std-LICENSE.txt
	set -e; libc=$$(cd rust && PATH="$(RUST_BIN):$$PATH" $(CARGO) metadata --format-version 1 --frozen | \
	    python3 -I -c 'import json, os, sys; p = [p for p in json.load(sys.stdin)["packages"] if p["name"] == "libc"]; print(len(p) == 1 and os.path.dirname(p[0]["manifest_path"]) + " " + p[0]["version"] or "")'); \
	  dir=$${libc% *}; ver=$${libc##* }; \
	  if [ -z "$$libc" ] || [ ! -s "$$dir/LICENSE-MIT" ] || [ ! -s "$$dir/LICENSE-APACHE" ]; then \
	    echo "package: the libc crate's licenses are not where cargo keeps it ($$libc)"; exit 1; fi; \
	  { echo "From the libc crate $$ver (https://github.com/rust-lang/libc): Rust's declarations of the C library's"; \
	    echo "functions and types, compiled into the Kindle binary. It is dual-licensed under the MIT and"; \
	    echo "Apache-2.0 licenses:"; echo; cat "$$dir/LICENSE-MIT"; printf '\n\n'; cat "$$dir/LICENSE-APACHE"; \
	  } > $(PKG)/app/licenses/rust-libc-LICENSE.txt
	set -e; c=$(FBINK_DIR)/cutef8; out=$(PKG)/app/licenses; \
	  awk 'NR == 1 && !/^\/\*/ { exit } { print } /\*\// { exit }' $$c/utf8.c > $(PKG)/utf8.head; \
	  awk '!/^\/\// || /NOTE:/ { exit } { print }' $$c/dfa.c > $(PKG)/dfa.head; \
	  awk '/^\t\* cutef8\/utf8\./ { p = 1 } p && /^$$/ { exit } p' $(FBINK_DIR)/CREDITS > $(PKG)/utf8.credits; \
	  awk '/^\t\* cutef8\/dfa\./ { p = 1 } p && /^$$/ { exit } p' $(FBINK_DIR)/CREDITS > $(PKG)/dfa.credits; \
	  for f in utf8.head dfa.head utf8.credits dfa.credits; do \
	    [ -s $(PKG)/$$f ] || { echo "package: no notice found for $$f in $(FBINK_DIR)"; exit 1; }; done; \
	  { echo "cutef8 (cutef8/utf8.c and utf8.h in FBInk's tree), compiled into the Kindle binary with FBInk."; \
	    echo; echo "The head of cutef8/utf8.c:"; echo; cat $(PKG)/utf8.head; \
	    echo; echo "FBInk's CREDITS:"; echo; cat $(PKG)/utf8.credits; } > $$out/cutef8-LICENSE.txt; \
	  { echo "dfa (cutef8/dfa.c and dfa.h in FBInk's tree, Bjoern Hoehrmann's UTF-8 decoder), compiled into"; \
	    echo "the Kindle binary with FBInk."; \
	    echo; echo "The head of cutef8/dfa.c:"; echo; cat $(PKG)/dfa.head; \
	    echo; echo "FBInk's CREDITS:"; echo; cat $(PKG)/dfa.credits; } > $$out/dfa-LICENSE.txt; \
	  rm -f $(PKG)/utf8.head $(PKG)/dfa.head $(PKG)/utf8.credits $(PKG)/dfa.credits
	chmod +x $(PKG)/app/grimoire $(PKG)/app/grimoire.sh
	python3 tools/archive.py tar $(BUILD)/dist/app.tar -C $(PKG)/app grimoire grimoire.sh bluetooth.sh wifi.sh update.sh CAPABILITIES licenses koreader
	base64 -i assets/icon.png | tr -d '\n' > $(BUILD)/dist/icon.b64
	id=$$(shasum $(BUILD)/dist/app.tar | cut -c1-12); \
	awk -v id="$$id" 'FNR==NR { icon = $$0; next } \
	  /^# Icon: @ICON@$$/ { print "# Icon: data:image/png;base64," icon; next } \
	  /^BUILD_ID=@BUILD_ID@$$/ { print "BUILD_ID=" id; next } { print }' \
	  $(BUILD)/dist/icon.b64 kindle/Grimoire.sh > $(PKG)/documents/Grimoire.sh
	printf '__GRIMOIRE_APP_ARCHIVE__\n' >> $(PKG)/documents/Grimoire.sh
	cat $(BUILD)/dist/app.tar >> $(PKG)/documents/Grimoire.sh
	cp "kindle/Grimoire - reset pen calibration.sh" "$(PKG)/documents/"
	chmod +x $(PKG)/documents/*.sh
	sed 's/@VERSION@/$(VERSION)/' kindle/README.txt > $(PKG)/README.txt
	cp LICENSE $(PKG)/LICENSE.txt
	python3 tools/archive.py zip $@ -C $(PKG) documents README.txt LICENSE.txt "$(KOPLUGIN_ZIP)" "KOReader patches (optional)"
	cp $@ $(BUILD)/dist/Grimoire-$(VERSION).zip
	rm -rf $(BUILD)/dist/patchstage && mkdir -p $(BUILD)/dist/patchstage/KOReader-patches
	cp -R "$(PKG)/KOReader patches (optional)/." $(BUILD)/dist/patchstage/KOReader-patches/
	python3 tools/archive.py zip $(BUILD)/dist/KOReader-patches.zip -C $(BUILD)/dist/patchstage KOReader-patches
	@echo "package: $@ (and Grimoire-$(VERSION).zip)"
$(BUILD)/dist/source-date: FORCE
	@mkdir -p $(BUILD)/dist
	@python3 tools/archive.py source-date > $@.new; if cmp -s $@.new $@; then rm -f $@.new; else mv $@.new $@; fi
FORCE:

# KPM v2 accepts only numeric version triples; the builder preserves prerelease ordering.
KPM_BASE_URL ?= https://github.com/ganten1998/grimoire-scribe/releases/download/v$(VERSION)
.PHONY: kpm-package kpm-check kopatch-check
.PHONY: dist
dist: kpm-package
	python3 tools/dist.py --build "$(BUILD)" --output "$(DIST)"

kpm-package: package
	python3 tools/kpm-package.py --zip $(BUILD)/dist/Grimoire.zip --version $(VERSION) \
	  --output $(BUILD)/kpm --base-url "$(KPM_BASE_URL)"
kopatch-check:
	luajit tests/kopatch_check.lua

kpm-check: kpm-package
	python3 -m unittest discover -s tests -p 'test_kpm.py' -v
	python3 -I tests/test_update_sh.py

# Everything that can be verified without the Kindle: make rust-check (unit tests, every scripted
# scenario, the languages, the share server against a real HTTP client, Live view, Bluetooth, the
# host exporter, the ARM build), and the zip people install: what it holds, that two clean builds
# make the same bytes, and its launchers run against stand-ins for the Kindle's tools; then the KPM
# package and the KOReader patches.
# package-check is package-contents-check (what the zip holds, tools/archive.py's tests) and
# repro-check (two clean builds make the same bytes): pull requests' CI runs only the first, pushes
# to main and releases both.
.PHONY: package-check package-contents-check repro-check launcher-check mirror-cleanup-check usb-stage-check
mirror-cleanup-check: $(BUILD)/host/grimoire-headless
	BUILD=$(BUILD) python3 -m unittest discover -s tests -p 'test_mirror_cleanup.py' -v
package-check: package-contents-check repro-check
package-contents-check: package
	tests/package_check.sh $(BUILD)/dist/Grimoire.zip
	python3 -m unittest discover -s tests -p 'test_archive.py' -v
repro-check: package
	ZIG="$(ZIG)" tests/repro_check.sh $(BUILD)
launcher-check: package
	tests/launcher_check.sh $(BUILD)/dist/Grimoire.zip
usb-stage-check:
	python3 -m unittest discover -s tests -p 'test_usb_stage.py' -v
# The scenario runner's own checks (tests/test_scenario_runner.py): the CI shards' split at any
# shard count, the skipped scenarios' order, what a failed run reports.
.PHONY: runner-check
runner-check:
	python3 -m unittest discover -s tests -p 'test_scenario_runner.py' -v
check: rust-check package-check launcher-check dev-check kpm-check kopatch-check wifi-check usb-stage-check runner-check dist
	@echo "CHECK PASSED"

clean:
	rm -rf $(BUILD)/host $(BUILD)/test $(BUILD)/kindle $(BUILD)/dist $(BUILD)/gen $(CBUILD)

# ---- the app's gate, and the C tree's ----
# make check's checks of the app, against the binaries test, headless, tools and kindle made (the
# sub-makes take the runner as it is: -o): the unit tests beside the scripted checks, and those in
# order (the scenarios first: the host checks read two of their data roots).
#   make rust-check   unit tests, scenarios, the languages, mirror cleanup, the share server, Live
#                     view, Bluetooth, the exporter, the ARM build
# The port's first rule names stay: rust-headless, rust-tools, rust-kindle, rust-test,
# rust-scenarios, rust-l10n-check and rust-package are headless, tools, kindle, test, scenarios,
# l10n-check and package.
.PHONY: rust-headless rust-tools rust-kindle rust-test rust-scenarios rust-l10n-check rust-check rust-package
rust-check: test kindle rust-scripted
	@echo "RUST CHECK PASSED"
# The scripted checks, beside the unit tests (make -j): the scenarios, then the languages and the
# host checks, which run at once under make -j on ports apart (the languages' runs from
# GRIMOIRE_TEST_PORT, the host checks' from SCEN_JOBS ports further on: on the sub-make's command
# line, which a GRIMOIRE_TEST_PORT given to make's own command line would otherwise override).
.PHONY: rust-scripted rust-after-scenarios rust-l10n rust-host
SCEN_NJOBS = $(or $(SCEN_JOBS),$(shell getconf _NPROCESSORS_ONLN 2>/dev/null || echo 8))
rust-scripted: headless tools
	$(MAKE) -o $(BUILD)/host/grimoire-headless scenarios
	$(MAKE) -o $(BUILD)/host/grimoire-headless rust-after-scenarios
rust-after-scenarios: rust-l10n rust-host
# The English Gothic language run of y_all_screens.qs is the run make scenarios made, when it ran
# that scenario (all of them, not one CI shard).
rust-l10n:
	$(MAKE) -o $(BUILD)/host/grimoire-headless l10n-check \
	  $(if $(and $(filter %/y_all_screens.qs,$(wildcard $(SCENARIOS))),$(if $(SCEN_SHARD),,1)),L10N_SKIP=en-gothic)
# (The runner takes SCEN_JOBS=0 as 1, so the shift is at least 1.)
rust-host:
	n=$(SCEN_NJOBS); [ "$$n" -ge 1 ] 2>/dev/null || n=1; \
	$(MAKE) -o $(BUILD)/host/grimoire-headless host-checks GRIMOIRE_TEST_PORT=$$(( $${GRIMOIRE_TEST_PORT:-8765} + n ))
# What make rust-check runs after the scenarios and the languages, against the binaries already
# built (CI's host-checks job runs it on its own, after the two scenarios whose data it uses:
# b_library, which the share server's and the exporter's checks read, and s_shape_1_record, which
# s_shape_4_guards starts from).
.PHONY: host-checks
host-checks:
	$(MAKE) -o $(BUILD)/host/grimoire-headless scenarios-late
	python3 -I tests/pages_cache_check.py $(BUILD)/host/grimoire-headless $(BUILD)/pages-cache-check
	$(MAKE) -o $(BUILD)/host/grimoire-headless mirror-cleanup-check
	BUILD=$(BUILD) $(SCEN_LANG) tests/share_check.sh $(BUILD)/scen/b_library
	BUILD=$(BUILD) tests/live_check.sh
	BUILD=$(BUILD) tests/bt_check.sh
	$(BUILD)/host/grimoire-export --force $(BUILD)/scen/b_library $(BUILD)/export-check
rust-headless: headless
rust-tools: tools
rust-kindle: kindle
rust-test: test
rust-scenarios: scenarios
rust-l10n-check: l10n-check
rust-package: package

# The C tree's binaries go where the app's go, under $(CBUILD) instead of $(BUILD), and the same
# checks run against them: each rule below runs the app's rule with BUILD=$(CBUILD), and with -o
# naming the C binary, which make then takes as up to date (never built with cargo), so tests/*.sh
# and the scenario runner find the C build where they look for the app.
#   make c-check     rust-check's checks against the C binaries (c-test for the unit tests),
#                    leaving out what only the Rust build runs (SCEN_SKIP_RUST_ONLY: the scenarios
#                    with a "# rust-only" line, the rust selftests and y_all_screens.qs)
#   make c-package   the Kindle zip with the C binary in it, in $(CBUILD)/dist (its licenses/ still
#                    names the Rust parts)
c-check: c-test c-headless c-tools c-kindle
	$(MAKE) BUILD=$(CBUILD) SCEN_SKIP_RUST_ONLY=1 -o $(CBUILD)/host/grimoire-headless scenarios
	$(MAKE) BUILD=$(CBUILD) SCEN_SKIP_RUST_ONLY=1 -o $(CBUILD)/host/grimoire-headless l10n-check
	$(MAKE) BUILD=$(CBUILD) -o $(CBUILD)/host/grimoire-headless mirror-cleanup-check
	BUILD=$(CBUILD) $(SCEN_LANG) tests/share_check.sh $(CBUILD)/scen/b_library
	BUILD=$(CBUILD) tests/live_check.sh
	BUILD=$(CBUILD) BT_SCENARIOS=tests/c-bt tests/bt_check.sh
	$(CBUILD)/host/grimoire-export --force $(CBUILD)/scen/b_library $(CBUILD)/export-check
	@echo "C CHECK PASSED"
c-package: c-kindle
	$(MAKE) BUILD=$(CBUILD) -o $(CBUILD)/kindle/grimoire package

.PHONY: wifi-check
wifi-check:
	python3 -I tests/test_wifi_sh.py
