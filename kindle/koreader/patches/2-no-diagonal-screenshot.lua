-- Turns off KOReader's two screenshot gestures: two fingers on opposite corners at once, and one
-- swipe across the whole diagonal. With a hand resting on the screen near one corner, a tap that
-- just misses a button in the opposite corner counts as the first. KOReader's menus have no
-- setting for these. It runs late (the 2- prefix), once KOReader is up. This file is not part of
-- KOReader: delete it from koreader/patches to have the gestures back.
local Screenshoter = require("ui/widget/screenshoter")
local function ignore() return true end
Screenshoter.onTapDiagonal = ignore
Screenshoter.onSwipeDiagonal = ignore
require("logger").warn("no diagonal screenshot patch: on")
