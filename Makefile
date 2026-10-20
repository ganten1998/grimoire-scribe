# Grimoire build. `build/` is a symlink out of Dropbox (~/.cache/grimoire-build).
#   make sim       desktop simulator (SDL3 window)   -> build/host/grimoire-sim
#   make headless  scripted runner, no SDL           -> build/host/grimoire-headless
#   make test      unit tests, ASan+UBSan            -> runs build/test/*
#   make tools     host CLI grimoire-export             -> build/host/grimoire-export
#   make kindle    static armhf binary (zig cc)      -> build/kindle/grimoire
#   make package   Kindle zip (scriptlet + app)      -> build/dist/Grimoire.zip
#   make check     tests + scenarios + share server + exporter + ARM build + the zip + its launchers
#                  + the languages (make l10n-check)
#   make all       sim + test + tools + kindle + package
#   make l10n      (maintainer, needs GNU gettext) po/grimoire.pot from the sources, then every
#                  po/<lang>.po merged with it and compiled to po/<lang>.mo

BUILD   ?= build
HOSTCC  ?= clang
ZIG     ?= zig
KCC     := $(ZIG) cc -target arm-linux-musleabihf -mcpu=cortex_a7
VERSION := $(shell cat VERSION 2>/dev/null || echo 0.1.0)

WARN    := -Wall -Wextra -Wno-unused-parameter -Wno-missing-field-initializers
COMMON  := -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -I$(BUILD)/gen -DGRIMOIRE_VERSION='"$(VERSION)"'

CORE_SRC := src/util.c src/canvas.c src/font.c src/paper.c src/brush.c src/page.c \
            src/history.c src/store.c src/export.c src/stb_impl.c src/calib_fit.c src/bgjob.c
CORE_SRC += src/power.c src/fbcheck.c src/i18n.c
UI_SRC   := src/ui.c src/keyboard.c src/qr.c third_party/qrcodegen/qrcodegen.c
APP_SRC  := src/app.c src/library.c src/editor.c src/pensettings.c src/calibrate.c \
            src/share.c src/httpd.c src/dispmenu.c src/musicbuild.c src/main.c
GEN_SRC  := $(BUILD)/gen/fonts.c $(BUILD)/gen/l10n.c
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

.PHONY: all sim headless test tools kindle package scenarios check clean icon logo l10n l10n-check
all: sim headless test tools kindle package

# ---- generated: embedded fonts ----
FONTS := TerminessNerdFontMono-Regular.ttf TerminessNerdFontMono-Bold.ttf Jacquard24-Regular.ttf \
         Jersey10-Regular.ttf
$(BUILD)/gen/fonts.c: $(addprefix assets/,$(FONTS))
	@mkdir -p $(BUILD)/gen
	( cd assets && for f in $(FONTS); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

# ---- generated: the languages ----
# po/<lang>.po is a language's translations, for translators; po/<lang>.mo is it compiled by
# msgfmt, committed like assets/icon.png so that building needs no gettext. Each .mo is embedded
# like the fonts, with a table src/i18n.c finds them in by name (po/es.mo: "es"; the language list
# itself is in src/i18n.c).
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
WEB := studio.js studio.css
$(WEB_GEN): $(addprefix web/,$(WEB))
	@mkdir -p $(BUILD)/gen
	( cd web && for f in $(WEB); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

# ---- simulator ----
SIM_FLAGS := -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer
sim: $(BUILD)/host/grimoire-sim
$(BUILD)/host/grimoire-sim: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) $(SDL_CFLAGS) -DGRIMOIRE_SIM -DGRIMOIRE_SIM_SDL \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) -o $@ $(SDL_LIBS) -lm

headless: $(BUILD)/host/grimoire-headless
$(BUILD)/host/grimoire-headless: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) -DGRIMOIRE_SIM \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(WEB_GEN) -o $@ -lm

# Every tests/scenarios/*.qs against a fresh data root (a script with a "# root: NAME" line starts
# from a copy of scenario NAME's data, so NAME must sort before it; a run ends only once the PDF
# exports it started are written, so that data is whole); screenshots in build/shots/<name>/. A
# failed expect makes a run exit with status 3, and a run that leaves a file half-written (*.tmp)
# fails too, as does one whose "# pdfs: N" line does not match the PDFs left in its root's PDFs/
# (the mirror): every scenario still runs, then the rule fails naming them. "# env: K=V" lines
# set a scenario's environment; nothing else of the caller's reaches the app's test hooks. Last,
# the runner checks itself: tests/selftest/pass.qs must exit 0 with its notebook's PDF written,
# and in fail.qs every line but the comments, waits and taps must be reported as failed, with
# exit status 3.
SIM_HOOKS := GRIMOIRE_PASSCODE GRIMOIRE_SIM_ADDRS GRIMOIRE_HTTPD_DELAY_MS GRIMOIRE_BG_DELAY_MS GRIMOIRE_SIM_PRESSURE \
             GRIMOIRE_SIM_STATS GRIMOIRE_SIM_SIZE GRIMOIRE_CHECK_FIT GRIMOIRE_LANG
# The scenarios tap by coordinates and check English text: they run in English whatever the
# caller's LANG says (GRIMOIRE_LANG ranks first), unless a scenario's "# env:" line sets another.
SCEN_LANG := GRIMOIRE_LANG=en
scenarios: $(BUILD)/host/grimoire-headless
	@unset $(SIM_HOOKS); \
	failed=; for s in tests/scenarios/*.qs; do n=$$(basename $$s .qs); rm -rf $(BUILD)/scen/$$n $(BUILD)/shots/$$n; \
	  echo "== $$n"; from=$$(sed -n 's/^# root: *//p' $$s | head -n 1); \
	  if [ -n "$$from" ] && ! cp -R $(BUILD)/scen/$$from $(BUILD)/scen/$$n; then failed="$$failed $$n"; continue; fi; \
	  if [ -n "$$GRIMOIRE_TEST_PORT" ]; then r=$(BUILD)/scen/$$n; mkdir -p $$r; \
	    { grep -v '^http_port=' $$r/settings.ini 2>/dev/null; echo "http_port=$$GRIMOIRE_TEST_PORT"; } > $$r/settings.ini.port; \
	    mv $$r/settings.ini.port $$r/settings.ini; fi; \
	  envs=$$(sed -n 's/^# env: *//p' $$s | tr '\n' ' '); \
	  env $(SCEN_LANG) $$envs GRIMOIRE_ROOT=$(BUILD)/scen/$$n GRIMOIRE_SCRIPT=$$s GRIMOIRE_SHOTS=$(BUILD)/shots/$$n \
	    $(BUILD)/host/grimoire-headless || failed="$$failed $$n"; \
	  if [ -n "$$(find $(BUILD)/scen/$$n -name '*.tmp' 2>/dev/null)" ]; then \
	    echo "$$n left a file half-written"; failed="$$failed $$n"; fi; \
	  pdfs=$$(sed -n 's/^# pdfs: *//p' $$s | head -n 1); \
	  if [ -n "$$pdfs" ]; then got=$$(find $(BUILD)/scen/$$n/PDFs -name '*.pdf' 2>/dev/null | wc -l | tr -d ' '); \
	    if [ "$$got" != "$$pdfs" ]; then echo "$$n left $$got PDFs in PDFs/, wanted $$pdfs"; failed="$$failed $$n"; fi; fi; \
	done; \
	d=$(BUILD)/selftest; rm -rf $$d; mkdir -p $$d; \
	echo "== selftest pass (every expect holds: exit 0, and the PDF export is done)"; \
	$(SCEN_LANG) GRIMOIRE_ROOT=$$d/pass GRIMOIRE_SCRIPT=tests/selftest/pass.qs GRIMOIRE_SHOTS=$$d/shots \
	  $(BUILD)/host/grimoire-headless || failed="$$failed selftest/pass"; \
	if ! ls $$d/pass/PDFs/*.pdf >/dev/null 2>&1; then \
	  echo "selftest pass: the run ended before its PDF export"; failed="$$failed selftest/pass"; fi; \
	echo "== selftest fail (each line meant to fail is reported: exit 3; log in $$d/fail.log)"; rc=0; \
	$(SCEN_LANG) GRIMOIRE_ROOT=$$d/fail GRIMOIRE_SCRIPT=tests/selftest/fail.qs GRIMOIRE_SHOTS=$$d/shots \
	  $(BUILD)/host/grimoire-headless 2>$$d/fail.log || rc=$$?; \
	want=$$(awk '!/^[[:space:]]*(#|$$)/ && $$1 != "wait" && $$1 != "tap" { printf " %d", NR }' tests/selftest/fail.qs); \
	got=$$(sed -n 's/^expect failed (line \([0-9]*\)).*/ \1/p' $$d/fail.log | tr -d '\n'); \
	if [ $$rc -ne 3 ] || [ "$$got" != "$$want" ]; then cat $$d/fail.log; \
	  echo "selftest fail: exit $$rc with lines$$got failed, wanted exit 3 with lines$$want"; failed="$$failed selftest/fail"; fi; \
	if [ -n "$$failed" ]; then echo "SCENARIOS FAILED:$$failed"; exit 1; fi

# ---- languages ----
# For the maintainer, with GNU gettext installed: xgettext collects every marked string of src/
# (with the TRANSLATORS comment above it) into po/grimoire.pot, and the share studio's (web/
# studio.js: t, tPages and N_) under the context "studio", which httpd.c sends the studio as
# window.GRIMOIRE_L10N. msgmerge brings each po/<lang>.po up to date with it, and msgfmt --check
# compiles each to po/<lang>.mo, failing on a translation whose printf conversions (positional
# ones too) differ from its English. Commit what changed. A new language is a new po/<lang>.po
# (msginit) plus its line in src/i18n.c.
XGETTEXT_FLAGS := --from-code=UTF-8 --language=C --add-comments=TRANSLATORS --add-location=file \
                  -k_ -kN_ -kC_:1c,2 -kNC_:1c,2 -kNG_:1,2 \
                  --flag=_:1:pass-c-format --flag=C_:2:pass-c-format \
                  --flag=NG_:1:pass-c-format --flag=NG_:2:pass-c-format \
                  --package-name=Grimoire --package-version=$(VERSION) \
                  --msgid-bugs-address=https://github.com/ganten1998/grimoire-scribe/issues
XGETTEXT_JS_FLAGS := --from-code=UTF-8 --language=JavaScript --add-comments=TRANSLATORS --add-location=file \
                     -kt -kN_ -ktPages:1,2
# No hash table and one byte order: the same .po makes the same .mo on any machine (l10n-check).
MSGFMT_FLAGS := --check --no-hash --endianness=little
l10n:
	@mkdir -p $(BUILD)/l10n
	xgettext $(XGETTEXT_FLAGS) -o $(BUILD)/l10n/c.pot $(sort $(wildcard src/*.c))
	xgettext $(XGETTEXT_JS_FLAGS) -o $(BUILD)/l10n/studio.pot web/studio.js
	awk '/^msgid / { if (n++) print "msgctxt \"studio\"" } { print }' $(BUILD)/l10n/studio.pot > $(BUILD)/l10n/studio-ctx.pot
	msgcat --use-first -o po/grimoire.pot $(BUILD)/l10n/c.pot $(BUILD)/l10n/studio-ctx.pot
	for p in $(wildcard po/*.po); do msgmerge --quiet --update --backup=none $$p po/grimoire.pot || exit 1; done
	for p in $(wildcard po/*.po); do msgfmt $(MSGFMT_FLAGS) -o $${p%.po}.mo $$p || exit 1; done

# The languages, checked (part of make check). Each committed po/<lang>.mo must be what msgfmt
# makes of po/<lang>.po now: a .po changed without make l10n fails (where gettext is not
# installed this part is skipped, and says so). Then tests/scenarios/y_all_screens.qs, which visits
# every screen and sheet, runs in every language, Gothic and Classic, with GRIMOIRE_CHECK_FIT=1:
# a run that cuts any text short to fit lists each ("fit[es]: ...") and exits with status 3. Its
# shots, build/shots/l10n/<lang>-<style>/, show translators every screen in context. Under make
# check it waits for the scenarios: both open the share server on the same port.
L10N_SCENARIO := tests/scenarios/y_all_screens.qs
L10N_LANGS := en $(basename $(notdir $(L10N_MO)))
l10n-check: $(BUILD)/host/grimoire-headless $(if $(filter check,$(MAKECMDGOALS)),scenarios)
	@unset $(SIM_HOOKS); failed=; \
	if command -v msgfmt >/dev/null 2>&1; then \
	  mkdir -p $(BUILD)/l10n; for p in po/*.po; do [ -f "$$p" ] || continue; m=$${p%.po}.mo; t=$(BUILD)/l10n/$${m##*/}; \
	    if ! msgfmt $(MSGFMT_FLAGS) -o $$t $$p; then echo "l10n: $$p does not compile"; failed="$$failed $$p"; \
	    elif ! cmp -s $$t $$m; then echo "l10n: $$m is not what msgfmt makes of $$p now (run make l10n)"; \
	      failed="$$failed $$m"; else echo "l10n: $$m is what msgfmt makes of $$p"; fi; \
	  done; \
	else echo "l10n: skipped the check that po/*.mo match po/*.po (msgfmt is not installed)"; fi; \
	for lang in $(L10N_LANGS); do for style in gothic classic; do n=$$lang-$$style; r=$(BUILD)/scen/l10n/$$n; \
	  rm -rf $$r $(BUILD)/shots/l10n/$$n; mkdir -p $$r; echo "== l10n $$n"; \
	  echo "theme=$$style" > $$r/settings.ini; \
	  if [ -n "$$GRIMOIRE_TEST_PORT" ]; then echo "http_port=$$GRIMOIRE_TEST_PORT" >> $$r/settings.ini; fi; \
	  GRIMOIRE_LANG=$$lang GRIMOIRE_CHECK_FIT=1 GRIMOIRE_SIM_ADDRS=wlan0=192.168.0.20 GRIMOIRE_ROOT=$$r \
	    GRIMOIRE_SCRIPT=$(L10N_SCENARIO) GRIMOIRE_SHOTS=$(BUILD)/shots/l10n/$$n \
	    $(BUILD)/host/grimoire-headless || failed="$$failed $$n"; \
	done; done; \
	if [ -n "$$failed" ]; then echo "L10N CHECK FAILED:$$failed"; exit 1; fi; \
	echo "l10n-check: every language fits ($(L10N_LANGS))"

# ---- host tools ----
tools: $(BUILD)/host/grimoire-export
$(BUILD)/host/grimoire-export: tools/grimoire-export.c $(CORE_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -O2 tools/grimoire-export.c $(CORE_SRC) $(GEN_SRC) -o $@ -lm

# The Library cover, drawn with the gothic widgets (committed; run by hand after a design change).
icon: $(BUILD)/host/mkicon
	$(BUILD)/host/mkicon assets/icon.png
$(BUILD)/host/mkicon: tools/mkicon.c tools/bat.h $(CORE_SRC) src/ui.c $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -Itools -O2 tools/mkicon.c $(CORE_SRC) src/ui.c $(GEN_SRC) -o $@ -lm

# The logo for the README and releases, black and white on transparent (committed, like the icon).
# Pixel blocks of 8: shown at half size they land on whole pixels on ordinary and Retina screens.
logo: $(BUILD)/host/mklogo
	$(BUILD)/host/mklogo crest 8 docs/logo.png
	$(BUILD)/host/mklogo crest 8 docs/logo-dark.png white
$(BUILD)/host/mklogo: tools/mklogo.c tools/bat.h $(CORE_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -Itools -O2 tools/mklogo.c $(CORE_SRC) $(GEN_SRC) -o $@ -lm

# ---- unit tests (core + ui only; no platform) ----
TESTS := $(patsubst tests/%.c,$(BUILD)/test/%,$(wildcard tests/test_*.c))
# Contraction (fused multiply-add) on, as clang does by default: tests/test_golden.c's renders
# were recorded so, and turning it off changes them.
TEST_FLAGS := -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer -ffp-contract=on
test: $(TESTS)
	@set -e; for t in $(TESTS); do echo "== $$t"; GRIMOIRE_TEST_TMP=$(BUILD)/test/tmp $$t; done; echo "ALL TESTS PASSED"
$(BUILD)/test/%: tests/%.c $(wildcard tests/*.h) $(CORE_SRC) $(UI_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/test/tmp
	$(HOSTCC) $(COMMON) $(WARN) $(TEST_FLAGS) -Itests $< $(CORE_SRC) $(UI_SRC) $(GEN_SRC) -o $@ -lm

# ---- kindle ----
kindle: $(BUILD)/kindle/grimoire
# FBInk is third-party: built on its own with warnings muted (an upstream format-string bug in
# fbink_state_dump, which Grimoire never calls); our sources stay warning-clean.
FBINK_OBJ := $(patsubst $(FBINK_DIR)/%.c,$(BUILD)/kindle/fbink/%.o,$(FBINK_SRC))
$(BUILD)/kindle/fbink/%.o: $(FBINK_DIR)/%.c
	@mkdir -p $(dir $@)
	$(KCC) -std=gnu11 -D_GNU_SOURCE -O2 -w $(FBINK_DEF) -I$(FBINK_DIR) -c $< -o $@
$(BUILD)/kindle/grimoire: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(KINDLE_PLAT) $(GEN_SRC) $(WEB_GEN) $(FBINK_OBJ) $(HDRS)
	@mkdir -p $(BUILD)/kindle
	$(KCC) $(COMMON) $(WARN) -O2 -I$(FBINK_DIR) $(FBINK_DEF) -DGRIMOIRE_KINDLE -static -s \
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
# against zig's musl, whose COPYRIGHT ships as musl-COPYRIGHT.txt.
PKG := $(BUILD)/dist/stage
RESET_SH := kindle/Grimoire\ -\ reset\ pen\ calibration.sh
VENDOR_LICENSES := $(wildcard third_party/*/LICENSE* third_party/*/COPYING* third_party/*/CREDITS*)
package: $(BUILD)/dist/Grimoire.zip
$(BUILD)/dist/Grimoire.zip: $(BUILD)/kindle/grimoire kindle/Grimoire.sh kindle/app/grimoire.sh $(RESET_SH) \
                           assets/icon.png kindle/README.txt LICENSE VERSION Makefile $(wildcard assets/*-OFL-1.1.txt) \
                           $(wildcard third_party/stb_*.h) $(VENDOR_LICENSES)
	rm -rf $(PKG) && mkdir -p $(PKG)/documents $(PKG)/app/licenses
	cp $(BUILD)/kindle/grimoire kindle/app/grimoire.sh $(PKG)/app/
	sed -n 's/^# Capabilities: //p' kindle/Grimoire.sh > $(PKG)/app/CAPABILITIES
	cp LICENSE $(PKG)/app/licenses/GPL-3.0.txt
	cp assets/*-OFL-1.1.txt $(PKG)/app/licenses/
	for d in third_party/*/; do d=$${d%/}; n=$${d##*/}; out=$(PKG)/app/licenses/$$n-LICENSE.txt; \
	  for f in $$d/LICENSE* $$d/COPYING*; do [ -f "$$f" ] || continue; \
	    [ -s $$out ] && printf '\n\n' >> $$out; cat "$$f" >> $$out; done; \
	  for f in $$d/CREDITS*; do [ -f "$$f" ] && cp "$$f" $(PKG)/app/licenses/$$n-CREDITS.txt; done; \
	done; true
	names=; : > $(PKG)/stb.all; for h in third_party/stb_*.h; do \
	  awk '/^This software is available under 2 licenses/ { p = 1 } p && /^\*\// { exit } p' $$h > $(PKG)/stb.one; \
	  [ -s $(PKG)/stb.one ] || { echo "package: no license text found in $$h"; exit 1; }; \
	  names="$${names:+$$names, }$${h##*/}"; cmp -s $(PKG)/stb.one $(PKG)/stb.all || cat $(PKG)/stb.one >> $(PKG)/stb.all; \
	done; { echo "$$names: single-file libraries by Sean Barrett (https://github.com/nothings/stb)."; echo; \
	  cat $(PKG)/stb.all; } > $(PKG)/app/licenses/stb-LICENSE.txt; rm -f $(PKG)/stb.one $(PKG)/stb.all
	lib=$$($(ZIG) env 2>/dev/null | sed -n 's/.*lib_dir"* *[=:] *"\([^"]*\)".*/\1/p'); \
	  if [ -f "$$lib/libc/musl/COPYRIGHT" ]; then cat "$$lib/libc/musl/COPYRIGHT" > $(PKG)/app/licenses/musl-COPYRIGHT.txt; \
	  else echo "package: musl's COPYRIGHT is not in zig's lib dir ($$lib)"; exit 1; fi
	chmod +x $(PKG)/app/grimoire $(PKG)/app/grimoire.sh
	COPYFILE_DISABLE=1 tar -cf $(BUILD)/dist/app.tar --format ustar -C $(PKG)/app grimoire grimoire.sh CAPABILITIES licenses
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
	( cd $(PKG) && rm -f ../Grimoire.zip && COPYFILE_DISABLE=1 zip -qrX ../Grimoire.zip documents README.txt LICENSE.txt )
	cp $@ $(BUILD)/dist/Grimoire-$(VERSION).zip
	@echo "package: $@ (and Grimoire-$(VERSION).zip)"

# Everything that can be verified without the Kindle: unit tests, every scripted scenario,
# the share server against a real HTTP client, the host exporter, the ARM build, and the zip
# people install: what it holds, and its launchers run against stand-ins for the Kindle's tools.
.PHONY: package-check launcher-check
package-check: package
	tests/package_check.sh $(BUILD)/dist/Grimoire.zip
launcher-check: package
	tests/launcher_check.sh $(BUILD)/dist/Grimoire.zip
check: test scenarios tools kindle package-check launcher-check l10n-check
	BUILD=$(BUILD) $(SCEN_LANG) tests/share_check.sh $(BUILD)/scen/b_library
	$(BUILD)/host/grimoire-export --force $(BUILD)/scen/b_library $(BUILD)/export-check
	@echo "CHECK PASSED"

clean:
	rm -rf $(BUILD)/host $(BUILD)/test $(BUILD)/kindle $(BUILD)/dist $(BUILD)/gen
