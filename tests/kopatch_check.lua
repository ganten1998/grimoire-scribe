-- Pure Lua integration doubles. Safe to run with KOReader's own LuaJIT without opening devices.
local root = arg[1] or "."
local checks = 0
local function check(value, message)
    checks = checks + 1
    assert(value, message or ("check " .. checks))
end
local Font = {fontmap = {cfont="NotoSans-Regular.ttf", ffont="NotoSans-Regular.ttf", tfont="NotoSans-Bold.ttf", other="DroidSansMono.ttf"},
              bold_font_variant={}, regular_font_variant={}}
-- Faces with no glyph where KOReader's battery symbols are (the cases below give some).
local no_glyph = {checkGlyph=function() return 0 end}
function Font:getFace(name) return {orig_font=name, ftsize=no_glyph} end
local TitleBar = {}
local status = "12:34 ⌁battery50% +aux100%"
local TouchMenu = {}
function TouchMenu:updateItems(page) self.time_info:setText(status); return page end
package.loaded["ui/font"] = Font
package.loaded["ui/widget/titlebar"] = TitleBar
package.loaded["ui/widget/touchmenu"] = TouchMenu
package.loaded["logger"] = {warn=function() end}
local files = {"TerminessNerdFontMono-Bold.ttf", "TerminessNerdFontPropo-Regular.ttf", "TerminessNerdFontMono-Regular.ttf",
               "TerminessNerdFont-Regular.ttf", "TerminessNerdFont-Bold.ttf", "TerminessNerdFont-Italic.ttf"}
local dir = "/mnt/us/fonts"
package.loaded["libs/libkoreader-lfs"] = {
    attributes=function(path)
        if path == dir then return "directory" end
        for _, name in ipairs(files) do if path == dir .. "/" .. name then return "file" end end
    end,
    dir=function() local i=0; return function() i=i+1; return files[i] end end,
}
dofile(root .. "/kindle/koreader/patches/2-terminess-ui-font.lua")
check(Font.fontmap.cfont == dir .. "/TerminessNerdFont-Regular.ttf", "stable regular preference")
check(Font.fontmap.tfont == dir .. "/TerminessNerdFont-Bold.ttf", "matching bold family")
check(Font.fontmap.ffont == "NotoSans-Regular.ttf", "reading footer keeps native battery symbol fallback")
check(Font.fontmap.other == "DroidSansMono.ttf", "unrelated fonts unchanged")
check(Font.bold_font_variant[Font.fontmap.cfont] == Font.fontmap.tfont, "real bold registered")
local button = {setText=function(self, text, extra) self.text=text; self.extra=extra end}
local menu = setmetatable({time_info=button}, {__index=TouchMenu})
check(menu:updateItems(3) == 3, "original menu return value")
check(button.text == "12:34 ⌁battery 50% +aux 100%", "main and auxiliary percentage gaps")
check(button.text_font_face == "NotoSans-Regular.ttf", "battery uses KOReader's native font and symbol fallback")
check(button.text_font_bold == false, "battery is not synthesized from an incompatible bold icon")
for _, glyph in ipairs({"", "", "", "", ""}) do
    status = "12:34 ⌁" .. glyph .. "50%"
    menu:updateItems()
    check(button.text:find(glyph, 1, true) ~= nil and button.text_font_face == "NotoSans-Regular.ttf", "charging and unplugged glyphs retain native symbol rendering")
end
status = "12:34 ⌁battery50% +aux100%"
menu:updateItems(4)
check(button.text == "12:34 ⌁battery 50% +aux 100%", "no accumulating spaces")
local setter = button.setText
dofile(root .. "/kindle/koreader/patches/2-terminess-ui-font.lua")
menu:updateItems(5)
check(setter == button.setText, "menu wrapper idempotent")
status = "12:34"
menu:updateItems()
check(button.text == status, "no battery leaves clock unchanged")
status = "12:34 \226\129\166battery\226\129\169\226\129\16650%\226\129\169"
menu:updateItems()
check(button.text:find("\226\129\166 50%\226\129\169", 1, true) ~= nil, "bidi wrappers preserved")
local ordinary = {setText=function(self,text) self.text=text end}
ordinary:setText("battery50%")
check(ordinary.text == "battery50%", "only menu status widget changes")

-- The menus' font, case by case: a fresh KOReader each time, with the fonts folders as a tree and
-- faces whose glyphs at the battery code points the case gives (KOReader's face API, checkGlyph:
-- 0 for none). Returns the Font, crash.log's lines and the paths whose faces were opened.
local battery = {0xE778, 0xE779, 0xE77A, 0xE77B, 0xE77C, 0xE77D, 0xE77E, 0xE77F, 0xE780, 0xE781,
                 0xE782, 0xE783, 0xEDA3}
local nerd = {}  -- a full Nerd Font: Devicons and Font Awesome at every one (U+E781 the "JS" badge)
for _, cp in ipairs(battery) do nerd[cp] = true end
local function world(tree, glyphs, broken)
    local F = {fontmap={cfont="NotoSans-Regular.ttf", ffont="NotoSans-Regular.ttf", tfont="NotoSans-Bold.ttf",
                        smallinfofontbold="NotoSans-Bold.ttf", infont="DroidSansMono.ttf"},
               bold_font_variant={}, regular_font_variant={}}
    local log, opened = {}, {}
    function F:getFace(name, size)
        if type(name) == "string" and name:find("/", 1, true) then
            opened[#opened + 1] = name
            if broken and broken[name] then error("no such face") end
            local has = glyphs[name] or {}
            return {orig_font=name, size=size, ftsize={checkGlyph=function(_, cp) return has[cp] and 1 or 0 end}}
        end
        return {orig_font=name, ftsize=no_glyph}
    end
    package.loaded["ui/font"] = F
    package.loaded["ui/widget/titlebar"] = {}
    package.loaded["ui/widget/touchmenu"] = {updateItems=function() end}
    package.loaded["logger"] = {warn=function(...)
        local parts = {}
        for i = 1, select("#", ...) do parts[i] = tostring((select(i, ...))) end
        log[#log + 1] = table.concat(parts, " ")
    end}
    package.loaded["libs/libkoreader-lfs"] = {
        attributes=function(path, mode)
            if tree[path] then return "directory" end
            local parent, name = path:match("^(.*)/([^/]+)$")
            for _, n in ipairs(parent and tree[parent] or {}) do if n == name then return "file" end end
        end,
        dir=function(path) local i, names = 0, tree[path] or {}; return function() i = i + 1; return names[i] end end,
    }
    dofile(root .. "/kindle/koreader/patches/2-terminess-ui-font.lua")
    return F, log, opened
end
local function logged(log, text)
    for _, line in ipairs(log) do if line:find(text, 1, true) then return true end end
    return false
end
local own, own_bold = "./fonts/Grimoire/TerminessNerdFontMono-Regular.ttf", "./fonts/Grimoire/TerminessNerdFontMono-Bold.ttf"
local term = "/mnt/us/fonts/Terminus"
-- The owner's Kindle: the whole Terminess Nerd Font unzipped into fonts/Terminus.
local owners = {"TerminessNerdFont-Regular.ttf", "TerminessNerdFont-Bold.ttf", "TerminessNerdFontMono-Regular.ttf",
                "TerminessNerdFontMono-Bold.ttf", "TerminessNerdFontPropo-Regular.ttf", "TerminessNerdFont-Italic.ttf"}
local full = {}
for _, name in ipairs(owners) do full[term .. "/" .. name] = nerd end

-- Grimoire's own copy comes first whenever it is there, over the full Nerd Font another folder holds.
local F, log, opened = world({["/mnt/us/fonts"]={"Terminus"}, [term]=owners, ["./fonts"]={"Grimoire", "noto"},
                              ["./fonts/Grimoire"]={"TerminessNerdFontMono-Regular.ttf", "TerminessNerdFontMono-Bold.ttf",
                                                    "Terminess-OFL-1.1.txt"}, ["./fonts/noto"]={}}, full)
check(F.fontmap.cfont == own, "Grimoire's own copy first, over a full Nerd Font in fonts/Terminus")
check(F.fontmap.tfont == own_bold and F.fontmap.smallinfofontbold == own_bold, "Grimoire's own bold with it")
check(F.bold_font_variant[own] == own_bold and F.regular_font_variant[own_bold] == own, "Grimoire's real bold registered")
check(F.fontmap.ffont == "NotoSans-Regular.ttf" and F.fontmap.infont == "DroidSansMono.ttf", "footers and other fonts unchanged")
check(logged(log, "terminess ui font patch: menus in " .. own .. " and " .. own_bold .. " (Grimoire's own copy)"),
      "crash.log names Grimoire's own copy")
check(logged(log, "passed over " .. term .. "/TerminessNerdFont-Regular.ttf (Grimoire's own copy in koreader/fonts/Grimoire comes first)"),
      "crash.log says why the owner's Nerd Font was passed over")
check(not logged(log, "TerminessNerdFont-Bold.ttf") and not logged(log, "Italic"), "only regular faces are named as passed over")
check(#opened == 1 and opened[1] == own, "only Grimoire's own copy opened to check")
check(not logged(log, "used anyway"), "a clean copy of Grimoire's is not remarked on")

-- Grimoire's own copy comes first over a clean Terminus too, and without its bold it is its own bold.
F, log = world({["/mnt/us/fonts"]={"TerminusTTF-4.49.3.ttf", "TerminusTTF-Bold-4.49.3.ttf"}, ["./fonts"]={"Grimoire"},
                ["./fonts/Grimoire"]={"TerminessNerdFontMono-Regular.ttf"}}, {})
check(F.fontmap.cfont == own and F.fontmap.tfont == own, "Grimoire's own copy over a clean Terminus, as its own bold")
check(logged(log, "passed over /mnt/us/fonts/TerminusTTF-4.49.3.ttf (Grimoire's own copy"), "the clean Terminus passed over")

-- Without Grimoire's copy, a font with a glyph at a battery code point is skipped while another will do.
F, log = world({["/mnt/us/fonts"]={"Terminus", "TerminusTTF-4.49.3.ttf", "TerminusTTF-Bold-4.49.3.ttf"},
                [term]={"TerminessNerdFont-Regular.ttf", "TerminessNerdFont-Bold.ttf"}},
               {[term .. "/TerminessNerdFont-Regular.ttf"]={[0xE781]=true}, [term .. "/TerminessNerdFont-Bold.ttf"]=nerd})
check(F.fontmap.cfont == "/mnt/us/fonts/TerminusTTF-4.49.3.ttf", "a Nerd Font sorted first is skipped for a clean Terminus")
check(F.fontmap.tfont == "/mnt/us/fonts/TerminusTTF-Bold-4.49.3.ttf", "the clean Terminus's own bold")
check(logged(log, "passed over " .. term .. "/TerminessNerdFont-Regular.ttf (it has a glyph at U+E781, where KOReader's battery symbols are)"),
      "crash.log names the code point it passed a font over for")
check(logged(log, "menus in /mnt/us/fonts/TerminusTTF-4.49.3.ttf and /mnt/us/fonts/TerminusTTF-Bold-4.49.3.ttf"), "crash.log names the clean Terminus")
check(not logged(log, "Grimoire's own copy"), "no word of Grimoire's copy where there is none")

-- Grimoire's bold alone is not its copy: the other fonts decide.
F = world({["/mnt/us/fonts"]={"TerminusTTF-4.49.3.ttf"}, ["./fonts"]={"Grimoire"}, ["./fonts/Grimoire"]={"TerminessNerdFontMono-Bold.ttf"}}, {})
check(F.fontmap.cfont == "/mnt/us/fonts/TerminusTTF-4.49.3.ttf" and F.fontmap.tfont == F.fontmap.cfont, "Grimoire's bold alone is not taken")

-- Every candidate has a glyph there: the first by rank, as before, and crash.log says so.
F, log = world({["/mnt/us/fonts"]={"Terminus"}, [term]=owners}, full)
check(F.fontmap.cfont == term .. "/TerminessNerdFont-Regular.ttf" and F.fontmap.tfont == term .. "/TerminessNerdFont-Bold.ttf",
      "with no clean font, the first by rank and its bold")
check(logged(log, "used anyway: " .. term .. "/TerminessNerdFont-Regular.ttf (it has a glyph at U+E778, where KOReader's battery symbols are) - no other Terminess or Terminus without one"),
      "crash.log says the menus' font has battery glyphs")
check(logged(log, "passed over " .. term .. "/TerminessNerdFontMono-Regular.ttf (it has a glyph at U+E778")
      and logged(log, "passed over " .. term .. "/TerminessNerdFontPropo-Regular.ttf (it has a glyph at U+E778"), "and the others passed over")
check(F.fontmap.ffont == "NotoSans-Regular.ttf", "the footer keeps KOReader's own font even then")

-- A face that cannot be read is passed over (the pcall), as when KOReader's face API is not there.
F, log = world({["/mnt/us/fonts"]={"TerminessNerdFont-Regular.ttf", "TerminusTTF-4.49.3.ttf"}}, {},
               {["/mnt/us/fonts/TerminessNerdFont-Regular.ttf"]=true})
check(F.fontmap.cfont == "/mnt/us/fonts/TerminusTTF-4.49.3.ttf", "an unreadable face passed over for a readable one")
check(logged(log, "passed over /mnt/us/fonts/TerminessNerdFont-Regular.ttf (its glyphs could not be read)"), "crash.log says it could not be read")
F, log = world({["/mnt/us/fonts"]={"TerminessNerdFont-Regular.ttf", "TerminusTTF-4.49.3.ttf"}}, {},
               {["/mnt/us/fonts/TerminessNerdFont-Regular.ttf"]=true, ["/mnt/us/fonts/TerminusTTF-4.49.3.ttf"]=true})
check(F.fontmap.cfont == "/mnt/us/fonts/TerminessNerdFont-Regular.ttf", "none readable: the first by rank, as before")

-- An earlier Grimoire's copy (the whole Nerd Font, before 2.0.0-beta.2) is still Grimoire's: used, and remarked on.
F, log = world({["/mnt/us/fonts"]={"TerminusTTF-4.49.3.ttf"}, ["./fonts"]={"Grimoire"},
                ["./fonts/Grimoire"]={"TerminessNerdFontMono-Regular.ttf", "TerminessNerdFontMono-Bold.ttf"}}, {[own]=nerd, [own_bold]=nerd})
check(F.fontmap.cfont == own and F.fontmap.tfont == own_bold, "an earlier copy of Grimoire's is still first")
check(logged(log, "used anyway: " .. own .. " (it has a glyph at U+E778, where KOReader's battery symbols are) - an earlier Grimoire's copy"),
      "crash.log says the copy is an earlier Grimoire's")

-- No Terminess or Terminus anywhere: nothing changes, and crash.log says so.
F, log = world({["/mnt/us/fonts"]={"DejaVuSans.ttf"}, ["./fonts"]={"Grimoire"}, ["./fonts/Grimoire"]={"Terminess-OFL-1.1.txt"}}, {})
check(F.fontmap.cfont == "NotoSans-Regular.ttf" and F.fontmap.tfont == "NotoSans-Bold.ttf", "no Terminess: the menus as they were")
check(logged(log, "no Terminess or Terminus font in /mnt/us/fonts or koreader/fonts"), "crash.log says none was found")
print("KOReader patches: " .. checks .. " checks passed")
