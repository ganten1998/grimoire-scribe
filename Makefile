# Quire build. `build/` is a symlink out of Dropbox (~/.cache/quire-build).
#   make sim       desktop simulator (SDL3 window)   -> build/host/quire-sim
#   make headless  scripted runner, no SDL           -> build/host/quire-headless
#   make test      unit tests, ASan+UBSan            -> runs build/test/*
#   make tools     host CLI quire-export             -> build/host/quire-export
#   make kindle    static armhf binary (zig cc)      -> build/kindle/quire
#   make package   Kindle zip (scriptlet + app)      -> build/dist/Quire.zip
#   make check     tests + scenarios + share server + exporter + ARM build
#   make all       sim + test + tools + kindle + package

BUILD   ?= build
HOSTCC  ?= clang
ZIG     ?= zig
KCC     := $(ZIG) cc -target arm-linux-musleabihf -mcpu=cortex_a7
VERSION := $(shell cat VERSION 2>/dev/null || echo 0.1.0)

WARN    := -Wall -Wextra -Wno-unused-parameter -Wno-missing-field-initializers
COMMON  := -std=gnu11 -D_GNU_SOURCE -Isrc -Ithird_party -I$(BUILD)/gen -DQUIRE_VERSION='"$(VERSION)"'

CORE_SRC := src/util.c src/canvas.c src/font.c src/paper.c src/brush.c src/page.c \
            src/history.c src/store.c src/export.c src/stb_impl.c src/calib_fit.c src/bgjob.c
UI_SRC   := src/ui.c src/keyboard.c
APP_SRC  := src/app.c src/library.c src/editor.c src/pensettings.c src/calibrate.c \
            src/share.c src/httpd.c src/dispmenu.c src/musicbuild.c src/main.c
GEN_SRC  := $(BUILD)/gen/fonts.c
KINDLE_PLAT := src/display_fbink.c src/input_evdev.c src/light_kindle.c
SIM_PLAT    := src/sim_sdl.c

FBINK_DIR := third_party/FBInk
FBINK_SRC := $(FBINK_DIR)/fbink.c $(FBINK_DIR)/cutef8/utf8.c $(FBINK_DIR)/cutef8/dfa.c
FBINK_DEF := -DFBINK_FOR_KINDLE -DFBINK_MINIMAL -DFBINK_VERSION='"master-886f25f"'

HDRS := $(wildcard src/*.h)

SDL_CFLAGS := $(shell pkg-config --cflags sdl3 2>/dev/null)
SDL_LIBS   := $(shell pkg-config --libs sdl3 2>/dev/null)

.PHONY: all sim headless test tools kindle package scenarios check clean
all: sim headless test tools kindle package

# ---- generated: embedded fonts ----
FONTS := TerminessNerdFontMono-Regular.ttf TerminessNerdFontMono-Bold.ttf UnifrakturMaguntia-Book.ttf \
         IMFeENrm28P.ttf IMFeENsc28P.ttf
$(BUILD)/gen/fonts.c: $(addprefix assets/,$(FONTS))
	@mkdir -p $(BUILD)/gen
	( cd assets && for f in $(FONTS); do xxd -i $$f; done ) > $@.tmp && mv $@.tmp $@

# ---- simulator ----
SIM_FLAGS := -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer
sim: $(BUILD)/host/quire-sim
$(BUILD)/host/quire-sim: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) $(SDL_CFLAGS) -DQUIRE_SIM -DQUIRE_SIM_SDL \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) -o $@ $(SDL_LIBS) -lm

headless: $(BUILD)/host/quire-headless
$(BUILD)/host/quire-headless: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) $(SIM_FLAGS) -DQUIRE_SIM \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(SIM_PLAT) $(GEN_SRC) -o $@ -lm

# Every tests/scenarios/*.qs against a fresh data root; screenshots in build/shots/<name>/.
scenarios: $(BUILD)/host/quire-headless
	@set -e; for s in tests/scenarios/*.qs; do n=$$(basename $$s .qs); rm -rf $(BUILD)/scen/$$n $(BUILD)/shots/$$n; \
	  echo "== $$n"; QUIRE_ROOT=$(BUILD)/scen/$$n QUIRE_SCRIPT=$$s QUIRE_SHOTS=$(BUILD)/shots/$$n $(BUILD)/host/quire-headless; done

# ---- host tools ----
tools: $(BUILD)/host/quire-export
$(BUILD)/host/quire-export: tools/quire-export.c $(CORE_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/host
	$(HOSTCC) $(COMMON) $(WARN) -O2 tools/quire-export.c $(CORE_SRC) $(GEN_SRC) -o $@ -lm

# ---- unit tests (core + ui only; no platform) ----
TESTS := $(patsubst tests/%.c,$(BUILD)/test/%,$(wildcard tests/test_*.c))
TEST_FLAGS := -O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer
test: $(TESTS)
	@set -e; for t in $(TESTS); do echo "== $$t"; QUIRE_TEST_TMP=$(BUILD)/test/tmp $$t; done; echo "ALL TESTS PASSED"
$(BUILD)/test/%: tests/%.c tests/testlib.h $(CORE_SRC) $(UI_SRC) $(GEN_SRC) $(HDRS)
	@mkdir -p $(BUILD)/test/tmp
	$(HOSTCC) $(COMMON) $(WARN) $(TEST_FLAGS) -Itests $< $(CORE_SRC) $(UI_SRC) $(GEN_SRC) -o $@ -lm

# ---- kindle ----
kindle: $(BUILD)/kindle/quire
# FBInk is third-party: built on its own with warnings muted (an upstream format-string bug in
# fbink_state_dump, which Quire never calls); our sources stay warning-clean.
FBINK_OBJ := $(patsubst $(FBINK_DIR)/%.c,$(BUILD)/kindle/fbink/%.o,$(FBINK_SRC))
$(BUILD)/kindle/fbink/%.o: $(FBINK_DIR)/%.c
	@mkdir -p $(dir $@)
	$(KCC) -std=gnu11 -D_GNU_SOURCE -O2 -w $(FBINK_DEF) -I$(FBINK_DIR) -c $< -o $@
$(BUILD)/kindle/quire: $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(KINDLE_PLAT) $(GEN_SRC) $(FBINK_OBJ) $(HDRS)
	@mkdir -p $(BUILD)/kindle
	$(KCC) $(COMMON) $(WARN) -O2 -I$(FBINK_DIR) $(FBINK_DEF) -DQUIRE_KINDLE -static -s \
	  $(CORE_SRC) $(UI_SRC) $(APP_SRC) $(KINDLE_PLAT) $(GEN_SRC) $(FBINK_OBJ) -o $@ -lm

# ---- Kindle package (scriptlet for Vera / KPM-era jailbreaks) ----
# One self-contained file: documents/Quire.sh (Library entry, icon embedded) with the app tar
# appended after a marker line; it unpacks itself into /mnt/us/quire/app on first run. Copying
# a single file into documents/ over MTP is all an install or update takes.
PKG := $(BUILD)/dist/stage
package: $(BUILD)/dist/Quire.zip
$(BUILD)/dist/Quire.zip: $(BUILD)/kindle/quire kindle/Quire.sh kindle/app/quire.sh assets/icon.png
	rm -rf $(PKG) && mkdir -p $(PKG)/documents $(PKG)/app
	cp $(BUILD)/kindle/quire kindle/app/quire.sh $(PKG)/app/
	cp assets/Terminess-OFL-1.1.txt $(PKG)/app/FONT-LICENSE.txt
	chmod +x $(PKG)/app/quire $(PKG)/app/quire.sh
	COPYFILE_DISABLE=1 tar -cf $(BUILD)/dist/app.tar --format ustar -C $(PKG)/app quire quire.sh FONT-LICENSE.txt
	base64 -i assets/icon.png | tr -d '\n' > $(BUILD)/dist/icon.b64
	id=$$(shasum $(BUILD)/dist/app.tar | cut -c1-12); \
	awk -v id="$$id" 'FNR==NR { icon = $$0; next } \
	  /^# Icon: @ICON@$$/ { print "# Icon: data:image/png;base64," icon; next } \
	  /^BUILD_ID=@BUILD_ID@$$/ { print "BUILD_ID=" id; next } { print }' \
	  $(BUILD)/dist/icon.b64 kindle/Quire.sh > $(PKG)/documents/Quire.sh
	printf '__QUIRE_APP_ARCHIVE__\n' >> $(PKG)/documents/Quire.sh
	cat $(BUILD)/dist/app.tar >> $(PKG)/documents/Quire.sh
	cp "kindle/Quire - reset pen calibration.sh" "$(PKG)/documents/"
	chmod +x $(PKG)/documents/*.sh
	( cd $(PKG) && rm -f ../Quire.zip && zip -qr ../Quire.zip documents )
	@echo "package: $@"

# Everything that can be verified without the Kindle: unit tests, every scripted scenario,
# the share server against a real HTTP client, the host exporter, and the ARM build.
check: test scenarios tools kindle
	tests/share_check.sh $(BUILD)/scen/b_library
	$(BUILD)/host/quire-export --force $(BUILD)/scen/b_library $(BUILD)/export-check
	@echo "CHECK PASSED"

clean:
	rm -rf $(BUILD)/host $(BUILD)/test $(BUILD)/kindle $(BUILD)/dist $(BUILD)/gen
