local dir = arg[1] or "kindle/lab/kindlelabhid.koplugin"
package.path = dir .. "/?.lua;" .. package.path
local Mapping = require("kindlelab_hid_mapping")
local checks = 0
local function check(value) checks = checks + 1; assert(value, "check " .. checks) end
local p = Mapping.new()
local map = {[304] = "UserChoice", [116] = "Power", [330] = "Touch"}
check(p:apply(map))
check(map[304] == "UserChoice" and map[311] == "LPgFwd")
check(map[116] == "Power" and map[330] == "Touch")
map[310] = "OtherPluginChoice"
p:disable()
check(map[304] == "UserChoice" and map[310] == "OtherPluginChoice" and map[311] == nil)
check(not p:apply(nil))
check(p:apply(map)); check(p:apply(map)); check(map[544] == "Up")
local newmap = {}
check(p:apply(newmap)); check(map[544] == nil and newmap[544] == "Up")
p:disable(); check(newmap[544] == nil)
local settings, registered = {}, nil
G_reader_settings = {isTrue=function(_, k) return settings[k] == true end,
    saveSetting=function(_, k, v) settings[k] = v end}
package.preload.device = function() return {isKindle=function() return true end, input={event_map={}}} end
package.preload["ui/widget/container/widgetcontainer"] = function()
    return {extend=function(_, obj) return obj end}
end
local plugin = dofile(dir .. "/main.lua")
plugin.ui = {menu={registerToMainMenu=function(_, obj) registered = obj end}}
plugin:init()
local Device = require("device")
check(registered == plugin and not plugin.enabled and next(Device.input.event_map) == nil)
local menu = {}; plugin:addToMainMenu(menu)
menu.kindlelabhid.callback()
check(menu.kindlelabhid.checked_func() and settings.kindlelab_hid_enabled and Device.input.event_map[311] == "LPgFwd")
plugin:onResume(); check(Device.input.event_map[311] == "LPgFwd")
menu.kindlelabhid.callback(); check(Device.input.event_map[311] == nil)
menu.kindlelabhid.callback(); plugin:onCloseWidget(); check(Device.input.event_map[311] == nil)
print("KindleLab HID: " .. checks .. " checks passed")
