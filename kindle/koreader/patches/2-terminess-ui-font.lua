-- KOReader's menus, file browser and dialogs in Terminess or Terminus instead of its built-in Noto
-- Sans (KOReader's menus have no setting for this). Grimoire's own copy of Terminess comes first
-- whenever it is there (koreader/fonts/Grimoire, which Grimoire writes and keeps the same as its
-- own: Terminus's text and Grimoire's icons, nothing where KOReader draws its battery). Without it,
-- put the font's .ttf or .otf files in the Kindle's fonts folder (fonts/ at the top of its USB
-- storage) or in koreader/fonts, then restart KOReader. Any file whose name has "Termin" in it
-- counts, in those folders or folders inside them (TerminessNerdFont-Regular.ttf,
-- TerminusTTF-4.49.3.ttf, ...); italic files are left out, and so is a font with a glyph where
-- KOReader's battery symbols are, while another will do. It says in koreader/crash.log what it
-- used and why it passed others over, or that it found none. It runs late (the 2- prefix), once
-- KOReader is up. Not part of KOReader: delete this file from koreader/patches to have Noto Sans
-- back.
local Font = require("ui/font")
local lfs = require("libs/libkoreader-lfs")
local logger = require("logger")
-- Nerd Fonts reuse private-use codepoints. KOReader's battery glyphs must come from its
-- shipped symbols font, rather than whichever unrelated icon Terminess has at that codepoint.
local status_font = Font.fontmap.ffont or Font.fontmap.cfont or "NotoSans-Regular.ttf"
-- Grimoire's own copy (Settings > KOReader in Grimoire writes it, and each start of Grimoire
-- brings it up to date).
local own_regular = "./fonts/Grimoire/TerminessNerdFontMono-Regular.ttf"
local own_bold = "./fonts/Grimoire/TerminessNerdFontMono-Bold.ttf"
-- KOReader's battery symbols (frontend/device/generic/powerd.lua, getBatterySymbol), which reach
-- the dropdown and the reading footer from its nerdfonts/symbols.ttf through its fallback fonts.
-- A full Nerd Font has other icons there first: U+E781 is a "JS" badge.
local battery = {0xE778, 0xE779, 0xE77A, 0xE77B, 0xE77C, 0xE77D, 0xE77E, 0xE77F, 0xE780, 0xE781,
                 0xE782, 0xE783, 0xEDA3}

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
local function is_file(path) return lfs.attributes(path, "mode") == "file" end
local function is_bold(path) return path:lower():find("bold", 1, true) ~= nil end
-- Whether object's method finds a glyph for cp: true or false, nil when it cannot say. Each ask is
-- guarded: a face's ftsize is a cdata, and asking one for a method it lacks raises an error.
local function ask(object, method, cp)
    if object == nil then return nil end
    local ok, has = pcall(function() return object[method](object, cp) end)
    if not ok then return nil end
    if type(has) == "boolean" then return has end
    if type(has) == "number" then return has ~= 0 end
end
-- Whether a face has a glyph for cp, through KOReader's own face API: hasGlyph on its ftsize (true
-- or false), as KOReader v2026.07.2 has it (ffi/freetype.lua); failing that, the checkGlyph of older
-- builds (0 for none), on ftsize or on ftface. nil when none answers.
local function has_glyph(face, cp)
    local has = ask(face.ftsize, "hasGlyph", cp)
    if has == nil then has = ask(face.ftsize, "checkGlyph", cp) end
    if has == nil then has = ask(face.ftface, "checkGlyph", cp) end
    return has
end
-- The first battery code point the font has a glyph for, false for none, nil when its glyphs could
-- not be read (KOReader could not open it, or its face API did not answer).
local function clash(path)
    local ok, face = pcall(function() return Font:getFace(path, 20) end)
    if not ok or type(face) ~= "table" then return nil end
    for _, cp in ipairs(battery) do
        local has = has_glyph(face, cp)
        if has == nil then return nil end
        if has then return cp end
    end
    return false
end
local function why(found)
    if found then return string.format("(it has a glyph at U+%04X, where KOReader's battery symbols are)", found) end
    return "(its glyphs could not be read)"
end
-- What crash.log says after the fonts the menus use: each regular face passed over, and why.
local notes = {}
if is_file(own_regular) then
    regular = own_regular
    bold = is_file(own_bold) and own_bold or nil
    -- Only an earlier Grimoire's copy can have one, and Grimoire's next start replaces it. A copy
    -- whose glyphs could not be read is said too: then nothing here could check a font.
    local found = clash(own_regular)
    if found then
        notes[#notes + 1] = {"used anyway:", own_regular, why(found), "- an earlier Grimoire's copy, which its next start replaces"}
    elseif found == nil then
        notes[#notes + 1] = {"not checked:", own_regular, why(found)}
    end
    for _, path in ipairs(candidates) do
        if path ~= own_regular and not is_bold(path) then
            notes[#notes + 1] = {"passed over", path, "(Grimoire's own copy in koreader/fonts/Grimoire comes first)"}
        end
    end
else
    -- Grimoire's copy is missing: the first other regular face with no glyph where the battery
    -- symbols are, else the first (its dropdown and footers still use KOReader's own font).
    local checked = {}
    for _, path in ipairs(candidates) do
        if not is_bold(path) then
            local found = clash(path)
            checked[#checked + 1] = {path, found}
            if found == false then break end
        end
    end
    local pick, read = checked[#checked], false
    for _, item in ipairs(checked) do read = read or item[2] ~= nil end
    if pick and pick[2] ~= false then
        pick = checked[1]
        notes[#notes + 1] = {"used anyway:", pick[1], why(pick[2]),
                             read and "- no other Terminess or Terminus without one" or "- no Terminess or Terminus could be checked"}
    end
    for _, item in ipairs(checked) do
        if item ~= pick then notes[#notes + 1] = {"passed over", item[1], why(item[2])} end
    end
    regular = pick and pick[1]
end
local function family(path)
    return path:lower():gsub("regular", ""):gsub("bold", ""):gsub("[-_]", "")
end
if regular and regular ~= own_regular then
    for _, path in ipairs(candidates) do
        if is_bold(path) and family(path) == family(regular) then bold = path; break end
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
    -- Reading footers carry the same battery symbols as the dropdown. Their native font
    -- must remain first, so KOReader can reach its matching private-use symbol fallback.
    if type(file) == "string" and file:match("^NotoSans") and not key:match("ffont$") then
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
if regular == own_regular then
    logger.warn("terminess ui font patch: menus in", regular, "and", bold, "(Grimoire's own copy)")
else
    logger.warn("terminess ui font patch: menus in", regular, "and", bold)
end
for _, note in ipairs(notes) do
    logger.warn("terminess ui font patch: " .. note[1], unpack(note, 2))
end

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
            button.text_font_face = status_font
            button.text_font_bold = false
            -- Older Button versions take a face object instead of text_font_face.
            if button.face then
                button.face = Font:getFace(status_font, button.face.orig_size)
            end
            button._grimoire_status_font_refresh = true
            local set = button.setText
            function button:setText(text, ...)
                local spaced = type(text) == "string" and text:gsub("(%d+%%)", " %1") or text
                if self._grimoire_status_font_refresh and type(spaced) == "string" then
                    self._grimoire_status_font_refresh = nil
                    self.text = nil -- force Button:setText to rebuild its label with the new face
                end
                return set(self, spaced, ...)
            end
        end
        return update(self, ...)
    end
end)
