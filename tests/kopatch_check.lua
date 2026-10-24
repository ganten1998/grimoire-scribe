-- Pure Lua integration doubles. Safe to run with KOReader's own LuaJIT without opening devices.
local root = arg[1] or "."
local checks = 0
local function check(value, message)
    checks = checks + 1
    assert(value, message or ("check " .. checks))
end
local fb = {}
function fb:scaleBySize(px)
    local size = math.min(self:getWidth(), self:getHeight()) / 600
    return math.ceil(px * (size + (self.dpi_override and self.dpi / 160 or size)) / 2)
end
function fb:getDPI() return self.dpi end
function fb:getWidth() return self.w end
function fb:getHeight() return self.h end
package.loaded["ffi/framebuffer"] = fb
local original = fb.scaleBySize
local screen = setmetatable({device = {model = "KindleScribe"}, w = 1860, h = 2480, dpi = 320, dpi_override = true}, {__index=fb})
dofile(root .. "/kindle/koreader/patches/1-grimoire-scribe-dpi.lua")
for px = 1, 100 do check(screen:scaleBySize(px) == original(screen, px), "320 layout must remain identical") end
for _, size in ipairs({12, 18, 24, 32, 48}) do
    local previous = 0
    for dpi = 90, 900, 10 do
        screen.dpi = dpi
        local result = screen:scaleBySize(size)
        check(result >= previous, "DPI must never reverse")
        previous = result
    end
    screen.dpi = 300; screen.dpi_override = nil
    local auto = screen:scaleBySize(size)
    screen.dpi = 320; screen.dpi_override = true
    local working = screen:scaleBySize(size)
    screen.dpi = 480; local larger = screen:scaleBySize(size)
    screen.dpi = 640; local largest = screen:scaleBySize(size)
    check(auto < working and working < larger and larger < largest, "Auto/320/480/640 distinct and ordered")
end
screen.dpi = 320; screen.w, screen.h = 2480, 1860
check(screen:scaleBySize(24) == 62, "rotation preserves scale")
screen.device.model = "KindlePaperWhite"
check(screen:scaleBySize(24) == original(screen, 24), "other devices unchanged")
screen.device = nil
check(screen:scaleBySize(24) == original(screen, 24), "no device yet falls through")
local patched = fb.scaleBySize
dofile(root .. "/kindle/koreader/patches/1-grimoire-scribe-dpi.lua")
check(patched == fb.scaleBySize, "DPI patch idempotent")

local Font = {fontmap = {cfont="NotoSans-Regular.ttf", tfont="NotoSans-Bold.ttf", other="DroidSansMono.ttf"},
              bold_font_variant={}, regular_font_variant={}}
function Font:getFace(name) return {orig_font=name} end
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
check(Font.fontmap.other == "DroidSansMono.ttf", "unrelated fonts unchanged")
check(Font.bold_font_variant[Font.fontmap.cfont] == Font.fontmap.tfont, "real bold registered")
local button = {setText=function(self, text, extra) self.text=text; self.extra=extra end}
local menu = setmetatable({time_info=button}, {__index=TouchMenu})
check(menu:updateItems(3) == 3, "original menu return value")
check(button.text == "12:34 ⌁battery 50% +aux 100%", "main and auxiliary percentage gaps")
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
print("KOReader patches: " .. checks .. " checks passed")
