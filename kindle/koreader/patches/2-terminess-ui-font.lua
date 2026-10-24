-- KOReader's menus, file browser and dialogs in Terminess or Terminus instead of its built-in Noto
-- Sans (KOReader's menus have no setting for this). Put the font's .ttf or .otf files in the
-- Kindle's fonts folder (fonts/ at the top of its USB storage) or in koreader/fonts, then restart
-- KOReader. Any file whose name has "Termin" in it counts, in those folders or folders inside them (TerminessNerdFont-Regular.ttf,
-- TerminusTTF-4.49.3.ttf, ...); italic files are left out. It says in koreader/crash.log what it
-- used, or that it found none. It runs late (the 2- prefix), once KOReader is up. Not part of
-- KOReader: delete this file from koreader/patches to have Noto Sans back.
local Font = require("ui/font")
local lfs = require("libs/libkoreader-lfs")
local logger = require("logger")

local regular, bold
-- The fonts folders and the folders in them (a font unzipped as fonts/Terminus/...), 3 deep.
local function look(dir, depth)
    if depth > 3 or lfs.attributes(dir, "mode") ~= "directory" then return end
    for name in lfs.dir(dir) do
        if name:sub(1, 1) ~= "." then
            local path, low = dir .. "/" .. name, name:lower()
            if lfs.attributes(path, "mode") == "directory" then
                look(path, depth + 1)
            elseif low:find("termin", 1, true) and (low:match("%.ttf$") or low:match("%.otf$"))
                    and not low:find("italic", 1, true) then
                if low:find("bold", 1, true) then
                    bold = bold or path
                elseif not regular or (regular:lower():find("mono", 1, true) and not low:find("mono", 1, true)) then
                    regular = path  -- the plain face before a Mono one
                end
            end
        end
    end
end
look("/mnt/us/fonts", 1)
look("./fonts", 1)
if not regular then
    logger.warn("terminess ui font patch: no Terminess or Terminus font in /mnt/us/fonts or koreader/fonts")
    return
end
bold = bold or regular
for key, file in pairs(Font.fontmap) do
    if type(file) == "string" and file:match("^NotoSans") then
        Font.fontmap[key] = file:match("Bold") and bold or regular
    end
end
-- The title bar (the file browser's "KOReader" and its folder line) picks its fonts once, when
-- KOReader first loads it, which can be before this patch: pick them again.
pcall(function()
    local TitleBar = require("ui/widget/titlebar")
    TitleBar.title_face_fullscreen = Font:getFace("smalltfont")
    TitleBar.title_face_not_fullscreen = Font:getFace("x_smalltfont")
    TitleBar.subtitle_face = Font:getFace("xx_smallinfofont")
    TitleBar.info_text_face = Font:getFace("x_smallinfofont")
end)
logger.warn("terminess ui font patch: menus in", regular, "and", bold)
