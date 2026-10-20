# Grimoire build. `build/` is a symlink out of Dropbox (~/.cache/grimoire-build).
#   make sim       desktop simulator (SDL3 window)   -> build/host/grimoire-sim
#   make headless  scripted runner, no SDL           -> build/host/grimoire-headless
#   make test      unit tests, ASan+UBSan            -> runs build/test/*
#   make tools     host CLI grimoire-export             -> build/host/grimoire-export
#   make kindle    static armhf binary (zig cc)      -> build/kindle/grimoire
#   make package   Kindle zip (scriptlet + app)      -> build/dist/Grimoire.zip
#   make check     tests + scenarios + share server + exporter + ARM build
#   make all       sim + test + tools + kindle + package

BUILD   ?= build
HOSTCC  ?= clang
ZIG     ?= zig
KCC     := $(ZIG) cc -target arm-linux-musleabihf -mcpu=cortex_a7
VERSION := $(shell cat VERSION 2>/dev/null || echo 0.1.0)

WARN    := -Wall -Wextra -Wno-unused-parameter -Wno-missing-field-initializers
COMMON  := -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -I$(BUILD)/gen -DGRIMOIRE_VERSION='"$(VERSION)"'

CORE_SRC := src/util.c src/canvas.c src/font.c src/paper.c src/brush.c src/page.c \
            src/history.c src/store.c src/export.c src/stb_impl.c src/calib_fit.c src/bgjob.c
CORE_SRC += src/power.c
UI_SRC   := src/ui.c src/keyboard.c
APP_SRC  := src/app.c src/library.c src/editor.c src/pensettings.c src/calibrate.c \
            src/share.c src/httpd.c src/dispmenu.c src/musicbuild.c src/main.c
GEN_SRC  := $(BUILD)/gen/fonts.c
WEB_GEN  := $(BUILD)/gen/web.c
KINDLE_PLAT := src/display_fbink.c src/input_evdev.c src/light_kindle.c
SIM_PLAT    := src/sim_sdl.c

FBINK_DIR := third_party/FBInk
FBINK_SRC := $(FBINK_DIR)/fbink.c $(FBINK_DIR)/cutef8/utf8.c $(FBINK_DIR)/cutef8/dfa.c
FBINK_DEF := -DFBINK_FOR_KINDLE -DFBINK_MINIMAL -DFBINK_VERSION='"master-886f25f"'

HDRS := $(wildcard src/*.h) VERSION  # the version is compiled in: a bump rebuilds everything

SDL_CFLAGS := $(shell pkg-config --cflags sdl3 2>/dev/null)
SDL_LIBS   := $(shell pkg-config --libs sdl3 2>/dev/null)

.PHONY: all sim headless test tools kindle package scenarios check clean icon logo
all: sim headless test tools kindle package

# ---- generated: embedded fonts ----
FONTS := TerminessNerdFontMono-Regular.ttf TerminessNerdFontMono-Bold.ttf Jacquard24-Regular.ttf \
         Jersey10-Regular.ttf
$(BUILD)/gen/fonts.c: $(addprefix assets/,$(FONTS))
	@mkdir -p $(BUILD)/gen
	( cd assets && for f in $(FONTS); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

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

# Every tests/scenarios/*.qs against a fresh data root; screenshots in build/shots/<name>/.
scenarios: $(BUILD)/host/grimoire-headless
	@set -e; for s in tests/scenarios/*.qs; do n=$$(basename $$s .qs); rm -rf $(BUILD)/scen/$$n $(BUILD)/shots/$$n; \
	  echo "== $$n"; GRIMOIRE_ROOT=$(BUILD)/scen/$$n GRIMOIRE_SCRIPT=$$s GRIMOIRE_SHOTS=$(BUILD)/shots/$$n $(BUILD)/host/grimoire-headless; done

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
TEST_FLAGS := -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer
test: $(TESTS)
	@set -e; for t in $(TESTS); do echo "== $$t"; GRIMOIRE_TEST_TMP=$(BUILD)/test/tmp $$t; done; echo "ALL TESTS PASSED"
$(BUILD)/test/%: tests/%.c tests/testlib.h $(CORE_SRC) $(UI_SRC) $(GEN_SRC) $(HDRS)
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
PKG := $(BUILD)/dist/stage
package: $(BUILD)/dist/Grimoire.zip
$(BUILD)/dist/Grimoire.zip: $(BUILD)/kindle/grimoire kindle/Grimoire.sh kindle/app/grimoire.sh assets/icon.png \
                           kindle/README.txt LICENSE VERSION
	rm -rf $(PKG) && mkdir -p $(PKG)/documents $(PKG)/app/licenses
	cp $(BUILD)/kindle/grimoire kindle/app/grimoire.sh $(PKG)/app/
	cp LICENSE $(PKG)/app/licenses/GPL-3.0.txt
	cp assets/*-OFL-1.1.txt $(PKG)/app/licenses/
	chmod +x $(PKG)/app/grimoire $(PKG)/app/grimoire.sh
	COPYFILE_DISABLE=1 tar -cf $(BUILD)/dist/app.tar --format ustar -C $(PKG)/app grimoire grimoire.sh licenses
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
# the share server against a real HTTP client, the host exporter, and the ARM build.
check: test scenarios tools kindle
	tests/share_check.sh $(BUILD)/scen/b_library
	$(BUILD)/host/grimoire-export --force $(BUILD)/scen/b_library $(BUILD)/export-check
	@echo "CHECK PASSED"

clean:
	rm -rf $(BUILD)/host $(BUILD)/test $(BUILD)/kindle $(BUILD)/dist $(BUILD)/gen
