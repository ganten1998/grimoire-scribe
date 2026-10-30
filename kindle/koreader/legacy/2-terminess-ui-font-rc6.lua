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
local candidates = {}
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
                candidates[#candidates + 1] = path
            end
        end
    end
end
look("/mnt/us/fonts", 1)
look("./fonts", 1)
-- Directory enumeration order varies across filesystems. Pick a stable regular face and its
-- matching bold family, rather than accidentally mixing proportional and Mono faces.
local function rank(path)
    local name = path:match("[^/]+$"):lower()
    return name:find("mono", 1, true) and 2 or name:find("propo", 1, true) and 3 or 1
end
table.sort(candidates, function(a, b)
    if rank(a) ~= rank(b) then return rank(a) < rank(b) end
    return a < b
end)
for _, path in ipairs(candidates) do
    if not path:lower():find("bold", 1, true) then regular = path; break end
end
local function family(path)
    return path:lower():gsub("regular", ""):gsub("bold", ""):gsub("[-_]", "")
end
if regular then
    for _, path in ipairs(candidates) do
        if path:lower():find("bold", 1, true) and family(path) == family(regular) then bold = path; break end
    end
end
if not regular then
    logger.warn("terminess ui font patch: no Terminess or Terminus font in /mnt/us/fonts or koreader/fonts")
    return
end
bold = bold or regular
-- Register real bold variants so KOReader does not synthesize bold over an already-bold face.
if Font.bold_font_variant then Font.bold_font_variant[regular] = bold end
if Font.regular_font_variant and bold ~= regular then Font.regular_font_variant[bold] = regular end
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

-- The dropdown status line concatenates a fallback battery glyph and the percentage with no
-- gap. Nerd Font glyph ink can extend past its advance. Add one space in this widget alone;
-- do not change powerd symbols globally or alter reading footers/other plugins.
pcall(function()
    local TouchMenu = require("ui/widget/touchmenu")
    if TouchMenu._grimoire_terminess_gap then return end
    TouchMenu._grimoire_terminess_gap = true
    local update = TouchMenu.updateItems
    function TouchMenu:updateItems(...)
        local button = self.time_info
        if button and not button._grimoire_terminess_gap then
            button._grimoire_terminess_gap = true
            local set = button.setText
            function button:setText(text, ...)
                local spaced = type(text) == "string" and text:gsub("(%d+%%)", " %1") or text
                return set(self, spaced, ...)
            end
        end
        return update(self, ...)
    end
end)
