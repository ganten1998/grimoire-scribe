--[[
Grimoire for KOReader: Tools > Grimoire opens Grimoire, the handwriting notebook for the Kindle
Scribe, from inside KOReader, and Grimoire's Library > Back to KOReader comes back.

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
    },
    es = {
        open = "Abrir Grimoire",
        pdfs = "PDF de los cuadernos",
        reset = "Restablecer la calibración del lápiz",
        preparing = "Preparando Grimoire...",
        could_not_start = "Grimoire no pudo iniciarse. Consulta grimoire/grimoire.log.",
        pdfs_note = "Grimoire reescribe estos PDF cada vez que cierras un cuaderno, así que los subrayados que KOReader escriba en ellos se pierden: lee o anota copias.",
        no_pdfs = "Aún no hay PDF de cuadernos: Grimoire escribe uno al cerrar un cuaderno.",
        kindle_only = "Grimoire solo funciona en un Kindle.",
        not_koreader_sh = "Grimoire solo se abre desde un KOReader iniciado con koreader.sh (su elemento de la Biblioteca o KUAL).",
        not_paused = "Este KOReader dejó funcionando las pantallas propias del Kindle, así que Grimoire no puede abrirse desde él. Inicia KOReader desde su elemento de la Biblioteca.",
        not_installed = "Grimoire no está instalado: copia Grimoire.sh en la carpeta documents del Kindle.",
        too_old = "Este Grimoire no puede abrirse desde KOReader: copia un Grimoire.sh más reciente en la carpeta documents del Kindle.",
        unpack_failed = "Grimoire no pudo desempaquetarse. Consulta grimoire/grimoire.log.",
        reset_confirm = "¿Olvidar la calibración del lápiz y del tacto de Grimoire? Puedes volver a calibrar en los Ajustes de Grimoire.",
        reset_ok = "Restablecer",
        reset_done = "Calibración del lápiz y del tacto restablecida.",
        reset_failed = "No se pudo restablecer la calibración. Consulta grimoire/grimoire.log.",
        nothing_to_reset = "No hay nada que restablecer: Grimoire aún no tiene ajustes en este Kindle.",
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

-- "Open Grimoire" for KOReader's gestures (Settings > Taps and gestures).
function Grimoire:onDispatcherRegisterActions()
    if not Device:isKindle() then return end
    Dispatcher:registerAction("grimoire_open", {
        category = "none",
        event = "OpenGrimoire",
        title = T("open"),
        general = true,
    })
end

function Grimoire:addToMainMenu(menu_items)
    menu_items.grimoire = {
        text = "Grimoire",
        sorting_hint = "tools",
        sub_item_table = {
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
        },
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
    if not is_file(self.koreader_sh) then return "not_koreader_sh" end
    -- koreader.sh paused the Kindle UI, or stopped it; started from KPVBooklet, it pauses nothing,
    -- and the Kindle's screens would draw over Grimoire.
    if os.getenv("STOP_FRAMEWORK") ~= "yes" and os.getenv("PILLOW_HARD_DISABLED") ~= "yes"
            and os.getenv("AWESOME_STOPPED") ~= "yes" then
        return "not_paused"
    end
    return nil
end

-- Tools > Grimoire > Open Grimoire, and the gesture.
function Grimoire:onOpenGrimoire()
    if handing_over then return true end
    local refusal = self:refusal()
    if refusal then
        self:tell(refusal)
        return true
    end
    local scriptlet = self.us .. "/documents/Grimoire.sh"
    local launcher = self.us .. "/grimoire/app/grimoire.sh"
    local busy
    if is_file(scriptlet) then
        local caps = capabilities(scriptlet)
        if not (caps["unpack-only"] and caps.hosted) then
            self:tell("too_old")
            return true
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
            return true
        end
    elseif not is_file(launcher) then
        self:tell("not_installed")
        return true
    end
    -- (A renamed Grimoire.sh: the app unpacked already is what runs.)
    if not capabilities(launcher).hosted then
        if busy then UIManager:close(busy) end
        self:tell("too_old")
        return true
    end
    -- "Preparing Grimoire..." stays up until Grimoire draws.
    self:handOver(launcher, busy)
    return true
end

-- KOReader restarts, and its exit runs Grimoire once it has let go of the screen and the input.
function Grimoire:handOver(launcher, busy)
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
    local command = "sh " .. quote(launcher) .. " --hosted koreader"
    local own_exit = rawget(Device, "exit")
    local koreader_exit = Device.exit
    Device.exit = function(...)
        rawset(Device, "exit", own_exit) -- for this one exit
        -- A failure here must not take reader.lua down with it: it would exit with another code
        -- than 85, and koreader.sh would hand the Kindle back to its home screen. Caught, KOReader
        -- just starts again, and the marker left behind says so.
        local ok, err = pcall(function(...)
            koreader_exit(...)
            collectgarbage()
            collectgarbage()
            logger.info("Grimoire: running", command)
            local status = exit_status(os.execute(command))
            logger.info("Grimoire: back in KOReader, grimoire.sh status", status)
            if status == 0 then os.remove(marker) end
        end, ...)
        if not ok then logger.warn("Grimoire: the hand-over failed:", err) end
    end
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
