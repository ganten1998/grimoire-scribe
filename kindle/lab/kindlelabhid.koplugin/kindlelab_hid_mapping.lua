-- Add only absent controller mappings; respect KOReader and other plugins' existing choices.
local Mapping = {}
Mapping.__index = Mapping
local codes = {
    [304] = "Press", [305] = "Back", [314] = "Menu",
    [310] = "LPgBack", [311] = "LPgFwd",
    [544] = "Up", [545] = "Down", [546] = "Left", [547] = "Right",
}
function Mapping.new()
    return setmetatable({ applied = {} }, Mapping)
end
function Mapping:disable()
    if self.map then
        for code, value in pairs(self.applied) do
            if self.map[code] == value then self.map[code] = nil end
        end
    end
    self.map, self.applied = nil, {}
end
function Mapping:apply(map)
    self:disable()
    if type(map) ~= "table" then return false end
    self.map = map
    for code, value in pairs(codes) do
        if map[code] == nil then
            map[code] = value
            self.applied[code] = value
        end
    end
    return true
end
return Mapping
