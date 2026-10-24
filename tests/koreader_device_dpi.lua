-- Run from an installed KOReader directory: ./luajit /path/to/this.lua /path/to/1-grimoire-scribe-dpi.lua
-- Loads the real framebuffer API and Screen DPI menu with in-memory settings/device doubles.
-- Does not instantiate a framebuffer, open input, write settings, or affect the running reader.
package.path = "./?.lua;./frontend/?.lua;" .. package.path
local fb = require("ffi/framebuffer")
local screen = setmetatable({device={model="KindleScribe", display_dpi=300}}, {__index=fb})
function screen:getWidth() return 1860 end
function screen:getHeight() return 2480 end
local original = fb.scaleBySize
screen:setDPI(320)
local before = screen:scaleBySize(24)
dofile(assert(arg[1], "patch path required"))
assert(screen:scaleBySize(24) == before, "320 changed")
local values = {}
local restarts = 0
G_reader_settings = {
    readSetting=function(_, key) return values[key] end,
    saveSetting=function(_, key, value) values[key]=value end,
}
package.loaded["device"] = {screen=screen, setScreenDPI=function(_, dpi) screen:setDPI(dpi) end}
package.loaded["gettext"] = function(text) return text end
package.loaded["ui/uimanager"] = {askForRestart=function() restarts=restarts+1 end}
local menu = require("ui/elements/screen_dpi_menu_table").sub_item_table
local samples = {}
for _, item in ipairs({{1,"Auto"},{5,"320"},{6,"480"},{7,"640"}}) do
    menu[item[1]].callback()
    assert(menu[item[1]].checked_func(), "selection not reflected: " .. item[2])
    samples[#samples+1] = screen:scaleBySize(24)
    print("KOReader menu DPI " .. item[2] .. ": 24-unit widget => " .. samples[#samples] .. " px")
end
assert(samples[1] < samples[2] and samples[2] < samples[3] and samples[3] < samples[4])
values.custom_screen_dpi = 420
menu[8].callback({updateItems=function() end})
assert(screen:getDPI() == 420 and menu[8].checked_func(), "custom DPI not selected")
assert(restarts == 5, "every selection must retain KOReader's restart requirement")
print("PASS: installed KOReader framebuffer + real Auto/320/480/640/custom menu callbacks; no screen/input/settings writes")
