--[[
Grimoire for KOReader: Tools > Grimoire opens Grimoire, the handwriting notebook for the Kindle
Scribe, from inside KOReader, and Grimoire's Library > Back to KOReader comes back. In a book,
Annotate this page in Grimoire opens the page being read in Grimoire, to write on.

Copyright (C) 2026 the Grimoire authors (https://github.com/ganten1998/grimoire-scribe)
SPDX-License-Identifier: GPL-3.0-or-later

This program is free software: you can redistribute it and/or modify it under the terms of the
GNU General Public License as published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
General Public License for more details. You should have received a copy of the GNU General
Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

How the hand-over works. KOReader keeps the pen, the touch screen and the screen to itself until
its own Device:exit(), and koreader.sh starts KOReader again, in the same session, whenever it
exits with code 85 (its Restart). So Open asks for that restart and wraps Device.exit for that
one exit: once KOReader's own exit has let go of everything, the wrapper runs Grimoire in the
foreground (grimoire/app/grimoire.sh --hosted koreader). koreader.sh keeps the Kindle UI paused
all along, so nothing of the Kindle's own screens shows in between. When Grimoire ends, KOReader
starts as usual, and its own start (start_with, lastfile) reopens the book.

A launch marker in KOReader's settings folder says a hand-over is under way. The wrapper removes
it once Grimoire has run; found at KOReader's next start, it means Grimoire never ran (the
wrapper did not, or grimoire.sh refused), and the user is told so.

Grimoire is installed by its own scriptlet, documents/Grimoire.sh. Its "# Capabilities:" line
says which flags it knows, and only that line is trusted: an older Grimoire.sh would take a flag
it does not know for an ordinary start, which undoes KOReader's pause of the Kindle UI.

Annotate this page in Grimoire draws the page the reader is on into a picture of the Kindle's page,
upright: the whole page fitted to it for a paged document (PDF, DjVu, comics, pictures), as the
document has it (not reflowed, cleaned up or inverted for night mode), or the text as the screen
shows it for a reflowable one (EPUB, FB2, MOBI, TXT), without KOReader's menus, its status bar or
highlights. A KOReader without the calls for that (or where they fail) gives a
screenshot of the reader instead, taken once the menu has closed. The picture (a PNG) and a request
naming the book (KOReader's partial MD5 of its file), its title, the page and the page count go into
grimoire/import/.annotate/, each written whole (a temporary file, then a rename), and the hand-over
runs grimoire.sh --hosted koreader --annotate, which tells Grimoire to open that page (its notebook
for the book, the page in it: docs/FORMAT.md, Annotated pages). Coming back is as from Open: the same
book, at the same page. Only app/grimoire.sh's "annotate" capability allows it. A book in Grimoire's
PDF mirror (grimoire/PDFs) is refused: a notebook's own PDF would become a notebook of a notebook.

What this plugin logs (KOReader's crash.log goes into bug reports too) never names a book, its title
or its file: page numbers and counts only.
]]

local ConfirmBox = require("ui/widget/confirmbox")
local DataStorage = require("datastorage")
local Device = require("device")
local Dispatcher = require("dispatcher")
local InfoMessage = require("ui/widget/infomessage")
local UIManager = require("ui/uimanager")
local WidgetContainer = require("ui/widget/container/widgetcontainer")
local lfs = require("libs/libkoreader-lfs")
local logger = require("logger")

-- The plugin's own words (KOReader's catalogs do not have them), picked by KOReader's language
-- (Settings > Language); any other language gets English.
local STRINGS = {
    en = {
        open = "Open Grimoire",
        pdfs = "Notebook PDFs",
        reset = "Reset pen calibration",
        preparing = "Preparing Grimoire...",
        could_not_start = "Grimoire could not start. See grimoire/grimoire.log.",
        pdfs_note = "Grimoire rewrites these PDFs whenever you close a notebook, so highlights KOReader writes into them are lost: read or annotate copies.",
        no_pdfs = "No notebook PDFs yet: Grimoire writes one when you close a notebook.",
        kindle_only = "Grimoire runs only on a Kindle.",
        not_koreader_sh = "Grimoire opens only from a KOReader started by koreader.sh (its Library item or KUAL).",
        not_paused = "This KOReader left the Kindle's own screens running, so Grimoire cannot open from it. Start KOReader from its Library item.",
        not_installed = "Grimoire is not installed: copy Grimoire.sh into the Kindle's documents folder.",
        too_old = "This Grimoire cannot open from KOReader: copy a newer Grimoire.sh into the Kindle's documents folder.",
        unpack_failed = "Grimoire could not unpack itself. See grimoire/grimoire.log.",
        reset_confirm = "Forget Grimoire's pen and touch calibration? You can calibrate again in Grimoire's Settings.",
        reset_ok = "Reset",
        reset_done = "Pen and touch calibration reset.",
        reset_failed = "Could not reset the calibration. See grimoire/grimoire.log.",
        nothing_to_reset = "Nothing to reset: Grimoire has no settings on this Kindle yet.",
        annotate = "Annotate this page in Grimoire",
        annotate_no_book = "Open a book first: Grimoire annotates the page you are reading.",
        annotate_too_old = "This Grimoire cannot annotate pages: copy a newer Grimoire.sh into the Kindle's documents folder.",
        annotate_failed = "Could not save this page for Grimoire. See koreader/crash.log.",
        annotate_own_pdf = "This PDF is a Grimoire notebook: open that notebook in Grimoire to write on it.",
    },
    -- The words of Grimoire's own Spanish (po/es.po, po/es-glossary.md) where they are the same:
    -- la pantalla táctil, Ajustes, cuaderno, and "detalles en" for where the log says more.
    es = {
        open = "Abrir Grimoire",
        pdfs = "PDF de los cuadernos",
        reset = "Restablecer la calibración del lápiz",
        preparing = "Preparando Grimoire...",
        could_not_start = "Grimoire no pudo iniciarse. Detalles en grimoire/grimoire.log.",
        pdfs_note = "Grimoire reescribe estos PDF cada vez que cierras un cuaderno, así que los subrayados que KOReader escriba en ellos se pierden: lee o anota copias.",
        no_pdfs = "Aún no hay PDF de cuadernos: Grimoire escribe uno al cerrar un cuaderno.",
        kindle_only = "Grimoire solo funciona en un Kindle.",
        not_koreader_sh = "Grimoire solo se abre desde un KOReader iniciado con koreader.sh (su elemento de la Biblioteca o KUAL).",
        not_paused = "Este KOReader dejó funcionando las pantallas propias del Kindle, así que Grimoire no puede abrirse desde él. Inicia KOReader desde su elemento de la Biblioteca.",
        not_installed = "Grimoire no está instalado: copia Grimoire.sh en la carpeta documents del Kindle.",
        too_old = "Este Grimoire no puede abrirse desde KOReader: copia un Grimoire.sh más reciente en la carpeta documents del Kindle.",
        unpack_failed = "Grimoire no pudo desempaquetarse. Detalles en grimoire/grimoire.log.",
        reset_confirm = "¿Olvidar la calibración del lápiz y de la pantalla táctil de Grimoire? Puedes volver a calibrar en los Ajustes de Grimoire.",
        reset_ok = "Restablecer",
        reset_done = "Se restableció la calibración del lápiz y de la pantalla táctil.",
        reset_failed = "No se pudo restablecer la calibración. Detalles en grimoire/grimoire.log.",
        nothing_to_reset = "No hay nada que restablecer: Grimoire aún no tiene ajustes en este Kindle.",
        annotate = "Anotar esta página en Grimoire",
        annotate_no_book = "Primero abre un libro: Grimoire anota la página que estás leyendo.",
        annotate_too_old = "Este Grimoire no puede anotar páginas: copia un Grimoire.sh más reciente en la carpeta documents del Kindle.",
        annotate_failed = "No se pudo guardar esta página para Grimoire. Detalles en koreader/crash.log.",
        annotate_own_pdf = "Este PDF es un cuaderno de Grimoire: abre ese cuaderno en Grimoire para escribir en él.",
    },
}

local function T(key)
    local lang = G_reader_settings:readSetting("language")
    local words = STRINGS[type(lang) == "string" and lang:sub(1, 2) or "en"] or STRINGS.en
    return words[key] or STRINGS.en[key]
end

-- One hand-over per KOReader run: Open can be tapped twice before KOReader goes.
local handing_over = false

local function is_file(path)
    return lfs.attributes(path, "mode") == "file"
end

local function quote(s)
    local q = s:gsub("'", "'\\''")
    return "'" .. q .. "'"
end

-- os.execute's answer as an exit status. LuaJIT gives the wait status as a number (Lua 5.1),
-- Lua 5.2 and later true or nil, "exit", the status; -1 for a command killed by a signal.
local function exit_status(status, how, code)
    if type(status) == "number" then
        if status % 256 ~= 0 then return -1 end
        return math.floor(status / 256)
    end
    if status == true then return 0 end
    if how == "exit" and type(code) == "number" then return code end
    return -1
end

-- The words of a launcher's "# Capabilities:" line, as a set. Only its first ten lines count:
-- Grimoire.sh carries the app after its header, and the app holds such a line of its own.
local function capabilities(path)
    local caps = {}
    local f = io.open(path, "rb")
    if not f then return caps end
    for _ = 1, 10 do
        local line = f:read("*l")
        if not line then break end
        local words = line:match("^# Capabilities: (.*)$")
        if words then
            for word in words:gmatch("%S+") do caps[word] = true end
            break
        end
    end
    f:close()
    return caps
end

-- Still there at KOReader's next start: the hand-over never ran Grimoire.
local function launch_marker()
    return DataStorage:getSettingsDir() .. "/grimoire_launch.txt"
end

local function is_nonempty_file(path)
    return is_file(path) and (lfs.attributes(path, "size") or 0) > 0
end

local function make_dir(path)
    if lfs.attributes(path, "mode") == "directory" then return true end
    lfs.mkdir(path)
    return lfs.attributes(path, "mode") == "directory"
end

-- text written whole: into a temporary file, then renamed over path, so Grimoire never reads half.
local function write_whole(path, text)
    local tmp = path .. ".tmp"
    local f = io.open(tmp, "wb")
    if not f then return false end
    local ok = f:write(text) ~= nil
    ok = f:close() and ok
    if ok and os.rename(tmp, path) then return true end
    os.remove(tmp)
    return false
end

-- Removes what a folder holds (its files), when it is there.
local function empty_dir(dir)
    if lfs.attributes(dir, "mode") ~= "directory" then return end
    for name in lfs.dir(dir) do
        if name ~= "." and name ~= ".." then os.remove(dir .. "/" .. name) end
    end
end

-- Sets t's keys to values, and returns what puts them back as they were (nil ones too).
local function set_aside(t, values)
    local was = {}
    for k in pairs(values) do was[k] = t[k] end
    for k, v in pairs(values) do t[k] = v end
    return function()
        for k in pairs(values) do t[k] = was[k] end
    end
end

-- A book key as Grimoire takes one (notebook.ini's source=): letters and digits, under 64.
local function key_ok(key)
    return type(key) == "string" and #key > 0 and #key < 64 and key:match("^%w+$") ~= nil
end

-- path made absolute (from KOReader's working folder) and plain: no "." or ".." parts, no doubled
-- or trailing slash. Its letters stay as they are.
local function plain_path(path)
    if path:sub(1, 1) ~= "/" then path = (lfs.currentdir and lfs.currentdir() or "") .. "/" .. path end
    local parts = {}
    for part in path:gmatch("[^/]+") do
        if part == ".." then
            table.remove(parts)
        elseif part ~= "." then
            table.insert(parts, part)
        end
    end
    return "/" .. table.concat(parts, "/")
end

local Grimoire = WidgetContainer:extend{
    name = "grimoire",
    is_doc_only = false,
    -- Where things are on the Kindle (tests/koplugin_test.lua points these into a scratch tree).
    us = "/mnt/us",
    koreader_sh = "/var/tmp/koreader.sh", -- koreader.sh runs from this copy while KOReader is up
}

function Grimoire:init()
    -- Grimoire is a Kindle Scribe app: nothing to offer anywhere else.
    if not Device:isKindle() then return end
    self:onDispatcherRegisterActions()
    self.ui.menu:registerToMainMenu(self)
    self:afterHandOver()
end

-- "Open Grimoire", and in a book "Annotate this page in Grimoire", for KOReader's gestures
-- (Settings > Taps and gestures).
function Grimoire:onDispatcherRegisterActions()
    if not Device:isKindle() then return end
    Dispatcher:registerAction("grimoire_open", {
        category = "none",
        event = "OpenGrimoire",
        title = T("open"),
        general = true,
    })
    Dispatcher:registerAction("grimoire_annotate", {
        category = "none",
        event = "AnnotateInGrimoire",
        title = T("annotate"),
        reader = true,
    })
end

function Grimoire:addToMainMenu(menu_items)
    local items = {
        {
            text = T("open"),
            callback = function() self:onOpenGrimoire() end,
        },
        {
            text = T("pdfs"),
            keep_menu_open = true, -- closed first, so the file browser comes up alone
            callback = function(touchmenu)
                if touchmenu and touchmenu.closeMenu then touchmenu:closeMenu() end
                self:showPDFs()
            end,
        },
        {
            text = T("reset"),
            callback = function() self:askReset() end,
        },
    }
    if self.ui.document then
        table.insert(items, 2, {
            text = T("annotate"),
            keep_menu_open = true, -- closed first, so a screenshot (an older KOReader's) shows the page alone
            callback = function(touchmenu)
                if touchmenu and touchmenu.closeMenu then touchmenu:closeMenu() end
                self:onAnnotateInGrimoire()
            end,
        })
    end
    menu_items.grimoire = {
        text = "Grimoire",
        sorting_hint = "tools",
        sub_item_table = items,
    }
end

function Grimoire:tell(key)
    UIManager:show(InfoMessage:new{ text = T(key) })
end

-- Why Grimoire cannot open from this KOReader, as a STRINGS key; nil when it can.
function Grimoire:refusal()
    if not Device:isKindle() then return "kindle_only" end
    -- koreader.sh runs from this copy for as long as KOReader is up, and its restart loop is
    -- what brings KOReader back after Grimoire. (grimoire.sh checks the processes themselves.)
    local dir = os.getenv("KOREADER_DIR")
    if not is_file(self.koreader_sh) and not (dir and is_file(dir .. "/koreader.sh")) then
        return "not_koreader_sh"
    end
    -- Launchers such as KPVBooklet leave the native UI running. grimoire.sh now pauses and
    -- restores that UI itself, after it has verified ancestry and KOReader's input teardown.
    return nil
end

-- Grimoire's launcher, ready to run hosted, and with need (a capability it must have besides
-- "hosted": "annotate") when given: the app unpacked first from documents/Grimoire.sh when that is
-- newer, under "Preparing Grimoire...", which stays up for the hand-over (the second answer). nil
-- once the user has been told why not.
function Grimoire:prepare(need)
    local scriptlet = self.us .. "/documents/Grimoire.sh"
    local launcher = self.us .. "/grimoire/app/grimoire.sh"
    local busy
    if is_file(scriptlet) then
        local caps = capabilities(scriptlet)
        if not (caps["unpack-only"] and caps.hosted) then
            self:tell("too_old")
            return nil
        end
        -- Unpacks the app when this Grimoire.sh is newer than it (the first time takes a moment).
        busy = InfoMessage:new{ text = T("preparing") }
        UIManager:show(busy)
        UIManager:forceRePaint()
        local status = exit_status(os.execute("sh " .. quote(scriptlet) .. " --unpack-only"))
        if status ~= 0 then
            logger.warn("Grimoire: Grimoire.sh --unpack-only exited with", status)
            UIManager:close(busy)
            self:tell("unpack_failed")
            return nil
        end
    elseif not is_file(launcher) then
        self:tell("not_installed")
        return nil
    end
    -- (A renamed Grimoire.sh: the app unpacked already is what runs.)
    local caps = capabilities(launcher)
    if not caps.hosted or (need and not caps[need]) then
        if busy then UIManager:close(busy) end
        self:tell(caps.hosted and need == "annotate" and "annotate_too_old" or "too_old")
        return nil
    end
    return launcher, busy
end

-- Tools > Grimoire > Open Grimoire, and the gesture.
function Grimoire:onOpenGrimoire()
    if handing_over then return true end
    local refusal = self:refusal()
    if refusal then
        self:tell(refusal)
        return true
    end
    local launcher, busy = self:prepare()
    if not launcher then return true end
    -- "Preparing Grimoire..." stays up until Grimoire draws.
    self:handOver(launcher, busy)
    return true
end

-- Tools > Grimoire > Annotate this page in Grimoire (in a book), and its gesture.
function Grimoire:onAnnotateInGrimoire()
    if handing_over then return true end
    local refusal = self:refusal()
    if refusal then
        self:tell(refusal)
        return true
    end
    if not (self.ui.document and self.ui.view) then
        self:tell("annotate_no_book")
        return true
    end
    if self:inMirror(self.ui.document.file) then
        logger.info("Grimoire: this book is a notebook's own PDF, from grimoire/PDFs: not annotated")
        self:tell("annotate_own_pdf")
        return true
    end
    logger.info("Grimoire: annotate the page the reader is on")
    local launcher, busy = self:prepare("annotate")
    if not launcher then return true end
    if not busy then
        busy = InfoMessage:new{ text = T("preparing") }
        UIManager:show(busy)
        UIManager:forceRePaint()
    end
    if not self:sendPage(busy) then
        UIManager:close(busy)
        self:tell("annotate_failed")
        return true
    end
    -- "Preparing Grimoire..." stays up until Grimoire draws.
    self:handOver(launcher, busy, " --annotate")
    return true
end

-- Whether file lies in Grimoire's PDF mirror, grimoire/PDFs, or in a folder in it: annotating a
-- notebook's own PDF would make a notebook of a notebook. The two paths are compared made plain;
-- then each folder the file lies in is compared with the mirror as the filesystem has them (by
-- device and inode), which finds the mirror reached another way: through a link, or by a name in
-- other letter case on a filesystem that ignores case.
function Grimoire:inMirror(file)
    if type(file) ~= "string" or file == "" then return false end
    local mirror, path = plain_path(self.us .. "/grimoire/PDFs"), plain_path(file)
    if path:sub(1, #mirror + 1) == mirror .. "/" then return true end
    local m = lfs.attributes(mirror)
    if not (m and m.mode == "directory" and m.ino) then return false end
    local dir = path:match("^(.+)/[^/]*$")
    while dir do
        local d = lfs.attributes(dir)
        if d and d.dev == m.dev and d.ino == m.ino then return true end
        dir = dir:match("^(.+)/[^/]*$")
    end
    return false
end

-- The page the reader is on.
function Grimoire:currentPage()
    local page = self.ui.getCurrentPage and self.ui:getCurrentPage()
    if type(page) ~= "number" then page = self.ui.document:getCurrentPage() end
    return math.floor(page)
end

-- The book's key, as KOReader keeps it in the book's settings (its partial MD5, which the
-- statistics and progress sync use too: the same file has the same one however it is renamed or
-- moved), else worked out from the file; nil without either.
function Grimoire:bookKey()
    local key = self.ui.doc_settings and self.ui.doc_settings:readSetting("partial_md5_checksum")
    if key_ok(key) then return key end
    local ok, util = pcall(require, "util")
    key = ok and util.partialMD5 and util.partialMD5(self.ui.document.file)
    return key_ok(key) and key or nil
end

-- The book's title, on one line: its metadata's, else its file's name without the extension.
function Grimoire:bookTitle()
    local title = self.ui.doc_props and self.ui.doc_props.display_title
    if type(title) ~= "string" or title == "" then
        local ok, props = pcall(function() return self.ui.document:getProps() end)
        title = ok and type(props) == "table" and props.title
    end
    if type(title) ~= "string" or title == "" then
        title = (self.ui.document.file or ""):match("([^/]+)$") or ""
        title = title:gsub("%.[^.]*$", "")
    end
    title = title:gsub("%c", " ")
    return title:sub(1, 500)
end

-- The page the reader is on, drawn clean (the top of this file says how) into a Gray8 BlitBuffer
-- of w x h, the Kindle's page upright. Raises an error where this KOReader lacks a call it needs.
function Grimoire:drawPage(w, h)
    local Blitbuffer = require("ffi/blitbuffer")
    local doc, view = self.ui.document, self.ui.view
    local page = self:currentPage()
    local bb = Blitbuffer.new(w, h, Blitbuffer.TYPE_BB8)
    -- The reader's settings that would draw the page as it is not (reflowed, its scan cleaned up,
    -- inverted for night mode, whose screen inverts it back) are set aside while it is drawn, and put
    -- back before anything else runs: the book keeps its own.
    local put_back
    if self.ui.paging then
        put_back = set_aside(doc.configurable or {}, { text_wrap = 0, page_opt = 0, auto_straighten = 0,
            white_threshold = 255, nightmode_document = 0 })
    else
        put_back = set_aside(doc, { _nightmode_images = false })
    end
    local ok, err = pcall(function()
        bb:fill(Blitbuffer.COLOR_WHITE)
        if self.ui.paging then
            -- The whole page fitted and centred, on none of the reader's zoom, crop or scroll.
            local Geom = require("ui/geometry")
            local native = doc:getNativePageDimensions(page)
            local zoom = math.min(w / native.w, h / native.h)
            local dim = doc:getPageDimensions(page, zoom, 0)
            local state = view.state or {}
            -- The reader's contrast. (drawPage's last argument is the colour saturation since
            -- KOReader 2024, the render mode before: each as the reader passes it.)
            local last = state.saturation
            if last == nil then last = view.render_mode end
            doc:drawPage(bb, math.floor((w - dim.w) / 2), math.floor((h - dim.h) / 2),
                Geom:new{ x = 0, y = 0, w = dim.w, h = dim.h }, page, zoom, 0, state.gamma or 1, last)
            return
        end
        -- The text as the screen shows it, at the screen's size: crengine's own status bar at the
        -- top (KOReader's alt status bar) left white; KOReader's footer is drawn apart, never here.
        -- Sideways (or any shape but the page's), fitted into the upright page.
        local area = view.visible_area
        local shot = Blitbuffer.new(area.w, area.h, Blitbuffer.TYPE_BB8)
        local drew, e = pcall(function()
            shot:fill(Blitbuffer.COLOR_WHITE)
            if view.view_mode == "scroll" then
                doc:drawCurrentViewByPos(shot, 0, 0, area, view.state.pos)
            else
                doc:drawCurrentViewByPage(shot, 0, 0, area, view.state.page or page)
            end
            local header = doc.getHeaderHeight and doc:getHeaderHeight() or 0
            if header > 0 then shot:paintRect(0, 0, area.w, header, Blitbuffer.COLOR_WHITE) end
            local scale = math.min(w / area.w, h / area.h)
            local sw, sh = math.floor(area.w * scale + 0.5), math.floor(area.h * scale + 0.5)
            local fitted = shot
            if sw ~= area.w or sh ~= area.h then
                fitted = require("ui/renderimage"):scaleBlitBuffer(shot, sw, sh, false)
            end
            bb:blitFrom(fitted, math.floor((w - sw) / 2), math.floor((h - sh) / 2), 0, 0, sw, sh)
            if fitted ~= shot then fitted:free() end
        end)
        shot:free()
        if not drew then error(e, 0) end
    end)
    put_back()
    if not ok then
        bb:free()
        error(err, 0)
    end
    return bb
end

-- The page's picture and its request into grimoire/import/.annotate/, emptied first (one request at
-- a time), each file written whole. True, or false once logged.
function Grimoire:sendPage(busy)
    local dir = self.us .. "/grimoire/import/.annotate"
    if not (make_dir(self.us .. "/grimoire") and make_dir(self.us .. "/grimoire/import") and make_dir(dir)) then
        logger.warn("Grimoire: could not make grimoire/import/.annotate")
        return false
    end
    empty_dir(dir)
    local key = self:bookKey()
    if not key then
        logger.warn("Grimoire: this book has no key to find its notebook by")
        return false
    end
    local page, pages = self:currentPage(), math.floor(self.ui.document:getPageCount())
    local sw, sh = Device.screen:getWidth(), Device.screen:getHeight()
    local w, h = math.min(sw, sh), math.max(sw, sh)
    local png, tmp = dir .. "/page.png", dir .. "/page.png.tmp"
    local how = "drawn"
    -- writePNG and Screen:shot answer false and why when the PNG could not be written (in a newer
    -- KOReader), nothing in an older one: then a file that is there and not empty is taken as
    -- written. (A PNG cut short all the same is refused by Grimoire, which checks every chunk of it.)
    local ran, wrote, why
    local ok, bb = pcall(self.drawPage, self, w, h)
    if ok then
        ran, wrote, why = pcall(bb.writePNG, bb, tmp)
        bb:free()
    else
        -- An older KOReader, or one where drawing the page alone failed: a screenshot of the reader
        -- (KOReader's Screen:shot), with the menu closed already, "Preparing Grimoire..." taken down
        -- and the page drawn again first. It has the screen's shape: Grimoire fits it to its page.
        logger.info("Grimoire: the page could not be drawn by itself; a screenshot of the reader instead")
        how = "a screenshot"
        UIManager:close(busy)
        UIManager:forceRePaint()
        ran, wrote, why = pcall(Device.screen.shot, Device.screen, tmp)
        UIManager:show(busy)
        UIManager:forceRePaint()
    end
    ok = ran and wrote ~= false and is_nonempty_file(tmp)
    if not (ok and os.rename(tmp, png)) then
        os.remove(tmp)
        -- Why, in lodepng's words or the error's (neither names a book or its file).
        local reason = not ran and wrote or why
        if not reason then reason = ok and "it could not be renamed into place" or "no file, or an empty one" end
        logger.warn("Grimoire: could not write the page's picture:", tostring(reason))
        return false
    end
    local request = table.concat({
        "# Grimoire: a page to annotate, from KOReader's Grimoire plugin (docs/FORMAT.md)",
        "version=1",
        "book=" .. key,
        "title=" .. self:bookTitle(),
        string.format("page=%d", page),
        string.format("pages=%d", math.max(pages, page)),
        "kind=" .. (self.ui.paging and "paged" or "reflowable"),
        "picture=page.png",
        string.format("time=%d", os.time()),
    }, "\n") .. "\n"
    if not write_whole(dir .. "/request.ini", request) then
        os.remove(png)
        logger.warn("Grimoire: could not write the request")
        return false
    end
    logger.info("Grimoire: page", page, "of", pages, "sent to annotate,", how)
    return true
end

-- KOReader restarts, and its exit runs Grimoire once it has let go of the screen and the input.
-- extra: more for grimoire.sh (" --annotate").
-- Export the configured image/cover without ever showing KOReader's sleep UI. The file is a
-- session snapshot: a random-image folder chooses once for this hand-over. Errors must never
-- prevent opening Grimoire. Unsupported screensaver widgets fall back to Grimoire's page.
function Grimoire:sendWallpaper()
    local path = self.us .. "/grimoire/koreader-sleep.png"
    os.remove(path)
    os.remove(path .. ".tmp")
    local mode = G_reader_settings:readSetting("screensaver_type")
    if mode ~= "random_image" and mode ~= "document_cover" and mode ~= "cover" then return end
    local bb, widget, screensaver
    local ok, err = pcall(function()
        screensaver = require("ui/screensaver")
        screensaver:setup()
        if not screensaver.image and not screensaver.image_file then return end
        local Blitbuffer = require("ffi/blitbuffer")
        local w, h = Device.screen:getWidth(), Device.screen:getHeight()
        bb = Blitbuffer.new(w, h, Blitbuffer.TYPE_BB8)
        bb:fill(screensaver.screensaver_background == "black" and Blitbuffer.COLOR_BLACK or Blitbuffer.COLOR_WHITE)
        widget = require("ui/widget/imagewidget"):new{
            image = screensaver.image, file = screensaver.image_file,
            file_do_cache = false, image_disposable = true, width = w, height = h,
            scale_factor = not G_reader_settings:readSetting("screensaver_stretch_images") and 0 or nil,
            stretch_limit_percentage = G_reader_settings:readSetting("screensaver_stretch_limit_percentage"),
            alpha = true,
        }
        widget:paintTo(bb, 0, 0)
        local result = bb:writePNG(path .. ".tmp")
        if result == false or not is_nonempty_file(path .. ".tmp") or not os.rename(path .. ".tmp", path) then
            error("wallpaper could not be written")
        end
    end)
    if widget then pcall(function() widget:free() end) end
    if bb then bb:free() end
    if screensaver then pcall(function() screensaver:cleanup() end) end
    os.remove(path .. ".tmp")
    if not ok then
        os.remove(path)
        logger.warn("Grimoire: wallpaper export unavailable:", err)
    end
end

function Grimoire:handOver(launcher, busy, extra)
    self:sendWallpaper()
    handing_over = true
    local marker = launch_marker()
    local f = io.open(marker, "w")
    if f then
        f:write(os.date("%Y-%m-%d %H:%M:%S"), "\n")
        f:close()
    else
        logger.warn("Grimoire: could not write", marker)
    end
    self:comeBackHere()
    local command = "sh " .. quote(launcher) .. " --hosted koreader" .. (extra or "")
    local own_exit = rawget(Device, "exit")
    local koreader_exit = Device.exit
    Device.exit = function(...)
        rawset(Device, "exit", own_exit) -- for this one exit
        -- A failure here must not take reader.lua down with it: it would exit with another code
        -- than 85, and koreader.sh would hand the Kindle back to its home screen. Caught, KOReader
        -- just starts again, and the marker left behind says so.
        -- Keep KOReader's frameworkStarted service alive for powerd when KUAL stopped the
        -- native framework. Its normal exit closes this handle too early for an external app.
        local framework = Device.framework_lipc_handle
        local own_framework = rawget(Device, "framework_lipc_handle")
        if framework then Device.framework_lipc_handle = false end
        local ok, err = pcall(function(...)
            koreader_exit(...)
            collectgarbage()
            collectgarbage()
            logger.info("Grimoire: running", command)
            local status = exit_status(os.execute(command))
            logger.info("Grimoire: back in KOReader, grimoire.sh status", status)
            if status == 0 then os.remove(marker) end
        end, ...)
        if framework then
            rawset(Device, "framework_lipc_handle", own_framework)
            pcall(function()
                framework:set_string_property("com.lab126.blanket", "load", "screensaver")
                framework:close()
            end)
        end
        if not ok then logger.warn("Grimoire: the hand-over failed:", err) end
    end
    -- From the file browser, exitOrRestart closes it and asks for the restart at once, so a failure
    -- comes back to this pcall and is undone below. From a book, KOReader closes the book and asks
    -- on its next tick, past this pcall: a failure there ends KOReader without Device:exit(), and
    -- the launch marker left behind tells the user at KOReader's next start.
    local ok, err = pcall(function()
        self.ui.menu:exitOrRestart(function() UIManager:restartKOReader() end)
    end)
    if not ok then
        -- KOReader stays: take it all back, or its next ordinary exit would run Grimoire.
        logger.warn("Grimoire: could not restart KOReader:", err)
        rawset(Device, "exit", own_exit)
        os.remove(marker)
        self:afterHandOver()
        handing_over = false
        if busy then UIManager:close(busy) end
        self:tell("could_not_start")
    end
end

-- KOReader's own start brings it back where it was: the book open now (start_with "last"
-- reopens the last file), or the file browser. start_with is borrowed for that one start only;
-- afterHandOver gives it back. KOReader opens the one document itself, never a second.
function Grimoire:comeBackHere()
    local current = G_reader_settings:readSetting("start_with")
    local want
    if self.ui.document then
        want = "last"
    elseif current == "last" then
        want = "filemanager"
    end
    if want and want ~= current and G_reader_settings:readSetting("grimoire_start_with") == nil then
        G_reader_settings:saveSetting("grimoire_start_with", current or "")
        G_reader_settings:saveSetting("start_with", want)
    end
end

-- KOReader's start after a hand-over: start_with goes back to what it was, and a launch marker
-- still there (Grimoire never ran) is told once.
function Grimoire:afterHandOver()
    local saved = G_reader_settings:readSetting("grimoire_start_with")
    if saved ~= nil then
        if saved == "" then
            G_reader_settings:delSetting("start_with")
        else
            G_reader_settings:saveSetting("start_with", saved)
        end
        G_reader_settings:delSetting("grimoire_start_with")
    end
    local marker = launch_marker()
    if is_file(marker) then
        os.remove(marker)
        logger.warn("Grimoire: the last hand-over did not run Grimoire")
        -- A page it sent to annotate, if it was one: no Grimoire would open it now.
        empty_dir(self.us .. "/grimoire/import/.annotate")
        -- Once the book or the file browser is up, over it.
        UIManager:scheduleIn(1, function() self:tell("could_not_start") end)
    end
end

-- Tools > Grimoire > Notebook PDFs: Grimoire's PDF copy of each notebook, in the file browser.
function Grimoire:showPDFs()
    local dir = self.us .. "/grimoire/PDFs"
    if lfs.attributes(dir, "mode") ~= "directory" then
        self:tell("no_pdfs")
        return
    end
    if self.ui.file_chooser then
        self.ui.file_chooser:changeToPath(dir)
    else
        -- In a book: close it, then the file browser.
        self.ui:onClose()
        self.ui:showFileManager(dir .. "/")
    end
    UIManager:show(InfoMessage:new{ text = T("pdfs_note"), timeout = 6 })
end

-- Tools > Grimoire > Reset pen calibration, for strokes that land away from the pen tip when
-- Grimoire's own calibration is hard to reach.
function Grimoire:askReset()
    local launcher = self.us .. "/grimoire/app/grimoire.sh"
    if not is_file(launcher) then
        self:tell("nothing_to_reset")
        return
    end
    UIManager:show(ConfirmBox:new{
        text = T("reset_confirm"),
        ok_text = T("reset_ok"),
        ok_callback = function()
            local status = exit_status(os.execute("sh " .. quote(launcher) .. " --reset-calibration"))
            self:tell(status == 0 and "reset_done" or "reset_failed")
        end,
    })
end

return Grimoire
