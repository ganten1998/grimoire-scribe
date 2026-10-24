-- Optional Scribe UI scaling. Load BEFORE widgets cache their sizes (priority 1).
-- KOReader's default Auto uses screen width, and overrides average width with DPI. On a
-- Scribe this makes Auto larger than 320, with 480 close to Auto. Preserve the familiar
-- 320 layout, then scale proportionally to the selected DPI, including Auto's hardware DPI.
-- No settings are written. Delete this file and restart to restore KOReader's own policy.
local Framebuffer = require("ffi/framebuffer")
if Framebuffer._grimoire_scribe_dpi then return end
Framebuffer._grimoire_scribe_dpi = true
local original = Framebuffer.scaleBySize
function Framebuffer:scaleBySize(px)
    local model = self.device and self.device.model
    if type(model) ~= "string" or not model:match("^KindleScribe") then
        return original(self, px)
    end
    local dpi = self:getDPI()
    if type(dpi) ~= "number" or dpi <= 0 then return original(self, px) end
    local baseline = (math.min(self:getWidth(), self:getHeight()) / 600 + 2) / 2
    return math.ceil(px * baseline * dpi / 320)
end
