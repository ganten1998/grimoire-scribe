-- Pure Lua integration doubles. Safe to run with KOReader's own LuaJIT without opening devices.
local root = arg[1] or "."
local checks = 0
local function check(value, message)
    checks = checks + 1
    assert(value, message or ("check " .. checks))
end
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
