local Device = require("device")
local Mapping = require("kindlelab_hid_mapping")
local WidgetContainer = require("ui/widget/container/widgetcontainer")
local Plugin = WidgetContainer:extend{ name = "kindlelabhid", is_doc_only = false }
function Plugin:init()
    if not Device:isKindle() then return end
    self.mapping = Mapping.new()
    self.enabled = G_reader_settings:isTrue("kindlelab_hid_enabled")
    self:updateMappings()
    self.ui.menu:registerToMainMenu(self)
end
function Plugin:updateMappings()
    if self.enabled then
        self.mapping:apply(Device.input and Device.input.event_map)
    else
        self.mapping:disable()
    end
end
function Plugin:onResume()
    if self.mapping then self:updateMappings() end
end
function Plugin:onCloseWidget()
    if self.mapping then self.mapping:disable() end
end
function Plugin:addToMainMenu(menu)
    menu.kindlelabhid = {
        text = "KindleLab HID buttons", sorting_hint = "tools",
        checked_func = function() return self.enabled end,
        callback = function()
            self.enabled = not self.enabled
            G_reader_settings:saveSetting("kindlelab_hid_enabled", self.enabled)
            self:updateMappings()
        end,
    }
end
return Plugin
