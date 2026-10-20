-- tests/koplugin_test.lua: KOReader's Grimoire plugin (kindle/koreader/grimoire.koplugin) run
-- against stand-ins for the KOReader modules it uses: the device, UIManager, the menus, the
-- dispatcher, the settings, os.execute and os.getenv. Each case builds a scratch Kindle (a
-- documents/Grimoire.sh with a header, an unpacked app, /var/tmp/koreader.sh) and plays one way
-- of using the plugin, down to reader.lua's Device:exit() and KOReader's next start.
-- tests/koplugin_check.sh runs it with LuaJIT.
-- usage: luajit tests/koplugin_test.lua PLUGIN_DIR SCRATCH_DIR
local plugin_dir, scratch = arg[1], arg[2]
assert(plugin_dir and scratch, "usage: luajit tests/koplugin_test.lua PLUGIN_DIR SCRATCH_DIR")

local real_execute = os.execute
local fails, count = 0, 0
local function check(name, ok)
    count = count + 1
    print((ok and "ok   " or "FAIL ") .. name)
    if not ok then fails = fails + 1 end
end

local function q(s) return "'" .. s:gsub("'", "'\\''") .. "'" end
local function sh(cmd)
    local r = real_execute(cmd)
    return r == 0 or r == true
end
local function write(path, text)
    local f = assert(io.open(path, "wb"))
    f:write(text)
    f:close()
end
local function is_file(path) return sh("test -f " .. q(path)) end

-- The world of one case: what the stand-ins answer, and every call they saw (W.calls, in order).
local W

-- KOReader's modules. Device is the device's class table, as require("device") returns it on a
-- Kindle, with exit an own field (kindle/device.lua sets KindleScribe.exit = KindleTouch.exit);
-- reader.lua calls Device:exit() on its way out.
local Device = {}
local function device_exit(self)
    table.insert(W.calls, "exit")
    assert(self == Device, "Device:exit() called on something else")
    if W.exit_error then error("KOReader's exit failed") end
end
Device.isKindle = function() return W.kindle end
-- nextTick queues its task for run_ticks, KOReader's UI loop: UIManager runs its tasks without a
-- pcall, so an error in one ends reader.lua.
local UIManager = {
    show = function(_, widget)
        table.insert(W.shown, widget)
        table.insert(W.calls, "show " .. tostring(widget.text))
    end,
    close = function(_, widget) table.insert(W.closed, widget) end,
    forceRePaint = function() table.insert(W.calls, "repaint") end,
    nextTick = function(_, f) table.insert(W.ticks, f) end,
    scheduleIn = function(_, _, f) table.insert(W.scheduled, f) end,
    restartKOReader = function() table.insert(W.calls, "restart") end,
}
local function run_ticks()
    while W.ticks[1] do
        table.remove(W.ticks, 1)()
    end
end
local WidgetContainer = {}
function WidgetContainer:extend(o)
    o = o or {}
    setmetatable(o, self)
    self.__index = self
    return o
end
function WidgetContainer:new(o)
    o = self:extend(o)
    if o.init then o:init() end
    return o
end
package.loaded["device"] = Device
package.loaded["ui/uimanager"] = UIManager
package.loaded["ui/widget/container/widgetcontainer"] = WidgetContainer
package.loaded["ui/widget/infomessage"] = { new = function(_, o) o.kind = "info"; return o end }
package.loaded["ui/widget/confirmbox"] = { new = function(_, o) o.kind = "confirm"; return o end }
package.loaded["dispatcher"] = {
    registerAction = function(_, name, value)
        W.actions[name] = value
        return true
    end,
}
package.loaded["datastorage"] = { getSettingsDir = function() return W.dir .. "/settings" end }
package.loaded["libs/libkoreader-lfs"] = {
    attributes = function(path, what)
        local mode = sh("test -d " .. q(path)) and "directory" or (sh("test -f " .. q(path)) and "file") or nil
        if what == "mode" then return mode end
        return mode and { mode = mode } or nil
    end,
}
-- The plugin never opens a document itself (KOReader's own start does): this one would show it.
package.loaded["apps/reader/readerui"] = {
    showReader = function() table.insert(W.calls, "showReader") end,
}
package.loaded["logger"] = {
    info = function() end,
    dbg = function() end,
    warn = function() table.insert(W.calls, "warn") end,
    err = function() table.insert(W.calls, "err") end,
}
G_reader_settings = { -- luacheck: ignore (KOReader's global)
    readSetting = function(_, key) return W.settings[key] end,
    saveSetting = function(self, key, value)
        W.settings[key] = value
        return self
    end,
    delSetting = function(self, key)
        W.settings[key] = nil
        return self
    end,
}
-- KOReader's environment, as koreader.sh exports it.
os.getenv = function(name) return W.env[name] end -- luacheck: ignore
-- The launchers: Grimoire.sh --unpack-only (writes app/grimoire.sh, as an unpack would),
-- grimoire.sh --hosted koreader and --reset-calibration, answering with W.status as LuaJIT's
-- os.execute does (the wait status: 768 is exit status 3), or with a function's answers.
os.execute = function(cmd) -- luacheck: ignore
    table.insert(W.calls, "execute " .. cmd)
    local answer
    if cmd:find(" --unpack-only", 1, true) then
        if W.unpack_writes then write(W.app_sh, W.unpack_writes) end
        answer = W.status.unpack
    elseif cmd:find(" --hosted koreader", 1, true) then
        answer = W.status.hosted
    elseif cmd:find(" --reset-calibration", 1, true) then
        answer = W.status.reset
    end
    if type(answer) == "function" then return answer() end
    return answer or 0
end

-- A documents/Grimoire.sh whose header (line 6) lists CAPS, or has no such line (1.0.0-beta).
-- Either way a Capabilities line listing everything comes after line 10, as the app appended to
-- the real one holds app/grimoire.sh: only the first ten lines may count.
local function scriptlet(caps)
    local lines = { "#!/bin/sh", "# Name: Grimoire", "# Author: Ganten",
        "# Icon: data:image/png;base64," .. string.rep("iVBORw0KGgo", 4000), "# DontUseFBInk" }
    if caps then table.insert(lines, "# Capabilities: " .. caps) end
    for _ = #lines, 11 do table.insert(lines, "#") end
    table.insert(lines, "# Capabilities: guard unpack-only hosted")
    write(W.doc, table.concat(lines, "\n") .. "\n")
end
local function app(caps) write(W.app_sh, "#!/bin/sh\n# Capabilities: " .. caps .. "\n") end

local function world(name)
    W = {
        dir = scratch .. "/" .. name,
        kindle = true,
        env = { KOREADER_DIR = "/mnt/us/koreader", STOP_FRAMEWORK = "no", AWESOME_STOPPED = "yes",
            PILLOW_HARD_DISABLED = "yes" },
        settings = {},
        status = {},
        calls = {},
        shown = {},
        closed = {},
        scheduled = {},
        ticks = {},
        actions = {},
        menus = 0,
        unpack_writes = "#!/bin/sh\n# Capabilities: guard hosted\n",
    }
    W.us = W.dir .. "/us"
    W.doc = W.us .. "/documents/Grimoire.sh"
    W.app_sh = W.us .. "/grimoire/app/grimoire.sh"
    W.koreader_sh = W.dir .. "/var/tmp/koreader.sh"
    W.marker = W.dir .. "/settings/grimoire_launch.txt"
    W.pdfs = W.us .. "/grimoire/PDFs"
    assert(sh("rm -rf " .. q(W.dir) .. " && mkdir -p " .. q(W.us .. "/documents") .. " " ..
        q(W.us .. "/grimoire/app") .. " " .. q(W.dir .. "/settings") .. " " .. q(W.dir .. "/var/tmp")))
    write(W.koreader_sh, "#!/bin/sh\n")
    scriptlet("guard unpack-only hosted")
end

-- The fake ui of a KOReader screen: the file browser, or a book. W.restart_error makes closing it
-- fail. Their menus' exitOrRestart differ as KOReader's do: the file browser's (FileManagerMenu)
-- closes it and runs the callback at once; a book's (ReaderMenu) does both on the next tick.
local function new_ui(in_book)
    local ui = {}
    local function close_then(callback)
        if W.restart_error then error("the screen would not close") end
        ui:onClose()
        if callback then callback() end
    end
    ui.menu = {
        registerToMainMenu = function(_, plugin)
            W.menus = W.menus + 1
            W.registered = plugin
        end,
        exitOrRestart = function(_, callback)
            table.insert(W.calls, "exitOrRestart")
            if in_book then
                UIManager:nextTick(function() close_then(callback) end)
            else
                close_then(callback)
            end
        end,
    }
    if in_book then
        ui.document = { file = W.us .. "/documents/book.epub" }
    else
        ui.file_chooser = { changeToPath = function(_, path) table.insert(W.calls, "changeToPath " .. path) end }
    end
    ui.onClose = function() table.insert(W.calls, "onClose") end
    ui.showFileManager = function(_, path) table.insert(W.calls, "showFileManager " .. path) end
    return ui
end

-- One KOReader run: the plugin loaded afresh (reader.lua starts a new process), one instance on
-- the file browser or a book.
local function koreader(in_book)
    Device.exit = device_exit
    local Plugin = dofile(plugin_dir .. "/main.lua")
    Plugin.us = W.us
    Plugin.koreader_sh = W.koreader_sh
    return Plugin:new{ ui = new_ui(in_book) }
end

local function at(prefix) -- the first call that starts with PREFIX, by its place in W.calls
    for i, c in ipairs(W.calls) do
        if c:sub(1, #prefix) == prefix then return i end
    end
    return nil
end
local function calls_of(prefix)
    local n = 0
    for _, c in ipairs(W.calls) do
        if c:sub(1, #prefix) == prefix then n = n + 1 end
    end
    return n
end
local function last_shown() return W.shown[#W.shown] and W.shown[#W.shown].text end
local function wrapped() return rawget(Device, "exit") ~= device_exit end
local function told(text) return last_shown() == text end
local function hosted_runs() -- grimoire.sh --hosted koreader runs so far
    local n = 0
    for _, c in ipairs(W.calls) do
        if c:find(" --hosted koreader", 1, true) then n = n + 1 end
    end
    return n
end

-- Nothing of a hand-over happened: nothing run, no wrapper, no marker, no restart.
local function untouched(s)
    check(s .. ": nothing run", calls_of("execute") == 0)
    check(s .. ": Device.exit not wrapped", not wrapped())
    check(s .. ": no launch marker", not is_file(W.marker))
    check(s .. ": no restart", not at("exitOrRestart"))
end

do -- On a Kindle: Tools > Grimoire and the gesture.
    world("menu")
    local g = koreader(false)
    check("menu: registered once", W.menus == 1 and W.registered == g)
    local a = W.actions.grimoire_open
    check("gesture: grimoire_open registered", a ~= nil)
    check("gesture: sends OpenGrimoire", a and a.event == "OpenGrimoire")
    check("gesture: titled Open Grimoire", a and a.title == "Open Grimoire")
    check("gesture: in the file browser and in books", a and a.general == true and a.category == "none")
    local items = {}
    g:addToMainMenu(items)
    local m = items.grimoire
    check("menu: Grimoire in the Tools tab", m and m.text == "Grimoire" and m.sorting_hint == "tools")
    local sub = m and m.sub_item_table or {}
    check("menu: Open Grimoire, Notebook PDFs, Reset pen calibration",
        #sub == 3 and sub[1].text == "Open Grimoire" and sub[2].text == "Notebook PDFs"
        and sub[3].text == "Reset pen calibration")
    check("menu: nothing run by showing it", calls_of("execute") == 0)
    check("menu: no message at an ordinary start", #W.scheduled == 0 and #W.shown == 0)
end

do -- Not a Kindle: no menu, no gesture, and Open refuses.
    world("not-kindle")
    W.kindle = false
    local g = koreader(false)
    check("not a Kindle: no menu", W.menus == 0)
    check("not a Kindle: no gesture", next(W.actions) == nil)
    g:onDispatcherRegisterActions()
    check("not a Kindle: no gesture on a dispatcher reload either", next(W.actions) == nil)
    g:onOpenGrimoire()
    check("not a Kindle: Open says why", told("Grimoire runs only on a Kindle."))
    untouched("not a Kindle")
end

do -- KOReader not started by koreader.sh (no /var/tmp/koreader.sh).
    world("no-koreader-sh")
    os.remove(W.koreader_sh)
    local g = koreader(false)
    g:onOpenGrimoire()
    check("no koreader.sh: says why", told("Grimoire opens only from a KOReader started by koreader.sh (its Library item or KUAL)."))
    untouched("no koreader.sh")
end

do -- koreader.sh paused nothing (a KPVBooklet start).
    world("not-paused")
    W.env = { KOREADER_DIR = "/mnt/us/koreader", STOP_FRAMEWORK = "no", AWESOME_STOPPED = "no" }
    local g = koreader(false)
    g:onOpenGrimoire()
    check("Kindle UI not paused: says why", told("This KOReader left the Kindle's own screens running, so Grimoire cannot open from it. Start KOReader from its Library item."))
    untouched("Kindle UI not paused")
end

do -- A Grimoire.sh that does not know --hosted (1.0.0-beta.2), or no Capabilities at all in its
    -- header (1.0.0-beta): never run, since it would take the flag for an ordinary start.
    for _, c in ipairs({ { "beta.2", "guard unpack-only" }, { "1.0.0-beta", nil } }) do
        world("old-" .. c[1])
        scriptlet(c[2])
        local g = koreader(false)
        g:onOpenGrimoire()
        local s = "a " .. c[1] .. " Grimoire.sh"
        check(s .. ": update it, says the message", told("This Grimoire cannot open from KOReader: copy a newer Grimoire.sh into the Kindle's documents folder."))
        untouched(s)
    end
end

do -- The hand-over, from the file browser.
    world("open")
    local g = koreader(false)
    g:onOpenGrimoire()
    local unpack = "execute sh '" .. W.doc .. "' --unpack-only"
    check("open: Preparing Grimoire... shows, drawn, before the unpack",
        at("show Preparing Grimoire...") and at("repaint") and at(unpack)
        and at("show Preparing Grimoire...") < at("repaint") and at("repaint") < at(unpack))
    check("open: Grimoire.sh --unpack-only runs once", calls_of(unpack) == 1)
    check("open: the message stays up for the hand-over", #W.closed == 0)
    check("open: a launch marker", is_file(W.marker))
    check("open: Device.exit wrapped", wrapped())
    check("open: KOReader restarts (exitOrRestart, restartKOReader)",
        at("exitOrRestart") and at("restart") and at("exitOrRestart") < at("restart"))
    check("open: nothing hosted runs before KOReader's exit", hosted_runs() == 0)
    check("open: from the file browser, start_with left alone", W.settings.start_with == nil
        and W.settings.grimoire_start_with == nil)
    g:onOpenGrimoire()
    check("open: tapped again, nothing more", calls_of(unpack) == 1 and calls_of("exitOrRestart") == 1)
    -- reader.lua: Device:exit(), then exit code 85.
    W.calls = {}
    Device:exit()
    local cmd = "execute sh '" .. W.app_sh .. "' --hosted koreader"
    check("exit: KOReader's own exit first, then grimoire.sh --hosted koreader",
        at("exit") == 1 and at(cmd) == 2 and #W.calls == 2)
    check("exit: the marker goes once Grimoire ran", not is_file(W.marker))
    check("exit: Device.exit back to KOReader's own", not wrapped())
    W.calls = {}
    Device:exit()
    check("exit: a second exit runs no Grimoire", #W.calls == 1 and W.calls[1] == "exit")
    -- koreader.sh starts KOReader again.
    W.shown, W.scheduled = {}, {}
    koreader(false)
    check("back: nothing to tell", #W.scheduled == 0 and #W.shown == 0)
end

do -- STOP_FRAMEWORK=yes (KUAL's no-framework start) counts as paused.
    world("framework-stopped")
    W.env = { KOREADER_DIR = "/mnt/us/koreader", STOP_FRAMEWORK = "yes", AWESOME_STOPPED = "no" }
    local g = koreader(false)
    g:onOpenGrimoire()
    check("framework stopped: Open goes ahead", wrapped() and at("restart") ~= nil)
end

do -- grimoire.sh refused (exit status 3): the marker stays, and the next start says so, once.
    world("refused")
    W.status.hosted = 768
    local g = koreader(false)
    g:onOpenGrimoire()
    Device:exit()
    check("refused: the hosted run happened", hosted_runs() == 1)
    check("refused: the marker stays", is_file(W.marker))
    W.shown, W.scheduled = {}, {}
    koreader(false)
    check("refused, next start: the marker is consumed", not is_file(W.marker))
    check("refused, next start: one message, once the screen is up", #W.scheduled == 1 and #W.shown == 0)
    if W.scheduled[1] then W.scheduled[1]() end
    check("refused, next start: Grimoire could not start", told("Grimoire could not start. See grimoire/grimoire.log."))
    W.shown, W.scheduled = {}, {}
    koreader(true)
    check("refused, the start after: nothing more", #W.scheduled == 0 and #W.shown == 0)
end

do -- KOReader's own exit fails: pcall keeps it from taking reader.lua down, no Grimoire runs.
    world("exit-error")
    local g = koreader(false)
    g:onOpenGrimoire()
    W.exit_error = true
    local ok = pcall(Device.exit, Device)
    check("exit fails: Device:exit() still returns", ok)
    check("exit fails: no Grimoire without KOReader's exit", hosted_runs() == 0)
    check("exit fails: the marker stays, to be told", is_file(W.marker))
    check("exit fails: Device.exit back to KOReader's own", not wrapped())
end

do -- The unpack fails: nothing else happens.
    world("unpack-fails")
    W.status.unpack = 256
    local g = koreader(false)
    g:onOpenGrimoire()
    check("unpack fails: says so", told("Grimoire could not unpack itself. See grimoire/grimoire.log."))
    check("unpack fails: Preparing Grimoire... closed", #W.closed == 1 and W.closed[1].text == "Preparing Grimoire...")
    check("unpack fails: no wrapper, marker or restart", not wrapped() and not is_file(W.marker)
        and not at("exitOrRestart"))
end

do -- An unpacked launcher that does not know --hosted, even after the unpack.
    world("old-app")
    W.unpack_writes = "#!/bin/sh\n# Capabilities: guard\n"
    local g = koreader(false)
    g:onOpenGrimoire()
    check("old app/grimoire.sh: update it", told("This Grimoire cannot open from KOReader: copy a newer Grimoire.sh into the Kindle's documents folder."))
    check("old app/grimoire.sh: Preparing Grimoire... closed", #W.closed == 1)
    check("old app/grimoire.sh: no wrapper, marker or restart", not wrapped() and not is_file(W.marker)
        and not at("exitOrRestart"))
end

do -- Grimoire.sh renamed or gone: the app unpacked already runs; neither: not installed.
    world("renamed")
    os.remove(W.doc)
    app("guard hosted")
    local g = koreader(false)
    g:onOpenGrimoire()
    check("no Grimoire.sh, app there: no unpack", calls_of("execute") == 0)
    check("no Grimoire.sh, app there: the hand-over goes ahead", wrapped() and at("restart") ~= nil)
    world("not-installed")
    os.remove(W.doc)
    g = koreader(false)
    g:onOpenGrimoire()
    check("not installed: says so", told("Grimoire is not installed: copy Grimoire.sh into the Kindle's documents folder."))
    untouched("not installed")
end

do -- The file browser will not close, so KOReader will not restart: its exitOrRestart fails at
    -- once, inside the plugin's pcall. Everything is taken back (start_with too, borrowed here as
    -- "filemanager"), and Open works again later.
    world("restart-error")
    W.settings.start_with = "last"
    W.restart_error = true
    local g = koreader(false)
    g:onOpenGrimoire()
    check("no restart: Device.exit back to KOReader's own", not wrapped())
    check("no restart: no marker", not is_file(W.marker))
    check("no restart: start_with given back", W.settings.start_with == "last"
        and W.settings.grimoire_start_with == nil)
    check("no restart: Preparing Grimoire... closed", #W.closed == 1)
    check("no restart: says so", told("Grimoire could not start. See grimoire/grimoire.log."))
    W.restart_error = false
    g:onOpenGrimoire()
    check("no restart: Open works again", wrapped() and at("restart") ~= nil)
end

do -- From a book, KOReader closes it and restarts on its next tick, once Open has returned.
    world("open-book")
    local g = koreader(true)
    g:onOpenGrimoire()
    check("open from a book: the restart waits for the next tick", wrapped() and is_file(W.marker)
        and at("exitOrRestart") ~= nil and not at("onClose") and not at("restart"))
    run_ticks()
    check("open from a book: the book closes, then KOReader restarts",
        at("onClose") ~= nil and at("restart") ~= nil and at("onClose") < at("restart"))
    W.calls = {}
    Device:exit()
    check("open from a book: KOReader's own exit, then grimoire.sh --hosted koreader",
        at("exit") == 1 and hosted_runs() == 1 and not is_file(W.marker))
end

do -- The book will not close: that fails on the tick, past the plugin's pcall, and ends reader.lua
    -- with no Device:exit(), so no Grimoire (koreader.sh then puts the Kindle UI back). The launch
    -- marker tells the user at KOReader's next start, which also gives start_with back.
    world("restart-error-book")
    W.settings.start_with = "filemanager"
    W.restart_error = true
    local g = koreader(true)
    g:onOpenGrimoire()
    check("book will not close: Open goes ahead up to the tick", wrapped() and is_file(W.marker)
        and W.settings.start_with == "last")
    check("book will not close: the failure ends KOReader", not pcall(run_ticks))
    check("book will not close: no restart, no Grimoire", not at("restart") and hosted_runs() == 0)
    check("book will not close: the marker stays", is_file(W.marker))
    W.restart_error = false
    W.shown, W.scheduled = {}, {}
    koreader(false)
    check("book will not close, next start: start_with given back", W.settings.start_with == "filemanager"
        and W.settings.grimoire_start_with == nil)
    check("book will not close, next start: the marker is consumed", not is_file(W.marker))
    check("book will not close, next start: one message, once the screen is up", #W.scheduled == 1)
    if W.scheduled[1] then W.scheduled[1]() end
    check("book will not close, next start: Grimoire could not start",
        told("Grimoire could not start. See grimoire/grimoire.log."))
end

do -- Back to the book: KOReader's own start reopens it; start_with is borrowed for one start.
    local cases = {
        { "a book, start_with unset", true, nil, "last" },
        { "a book, start_with filemanager", true, "filemanager", "last" },
        { "a book, start_with last", true, "last", "last" },
        { "the file browser, start_with last", false, "last", "filemanager" },
        { "the file browser, start_with history", false, "history", "history" },
    }
    for i, c in ipairs(cases) do
        world("return-" .. i)
        W.settings.start_with = c[3]
        W.settings.lastfile = W.us .. "/documents/book.epub"
        local g = koreader(c[2])
        g:onOpenGrimoire()
        run_ticks()
        check("from " .. c[1] .. ": KOReader starts with " .. c[4], W.settings.start_with == c[4])
        check("from " .. c[1] .. ": the original kept aside only when borrowed",
            (W.settings.grimoire_start_with ~= nil) == (c[3] ~= c[4]))
        Device:exit()
        koreader(c[4] == "last")
        check("from " .. c[1] .. ": start_with back to " .. tostring(c[3]),
            W.settings.start_with == c[3] and W.settings.grimoire_start_with == nil)
        check("from " .. c[1] .. ": no document opened by the plugin", calls_of("showReader") == 0)
    end
end

do -- os.execute answering the Lua 5.2 way (true or nil, "exit", status).
    world("lua52")
    W.status.unpack = function() return true, "exit", 0 end
    W.status.hosted = function() return true, "exit", 0 end
    local g = koreader(false)
    g:onOpenGrimoire()
    Device:exit()
    check("Lua 5.2 answers: success read as success", wrapped() == false and not is_file(W.marker)
        and hosted_runs() == 1)
    world("lua52-refused")
    W.status.hosted = function() return nil, "exit", 3 end
    g = koreader(false)
    g:onOpenGrimoire()
    Device:exit()
    check("Lua 5.2 answers: exit status 3 keeps the marker", is_file(W.marker))
end

do -- Tools > Grimoire > Notebook PDFs.
    world("pdfs")
    assert(sh("mkdir -p " .. q(W.pdfs)))
    local g = koreader(false)
    local items = {}
    g:addToMainMenu(items)
    local item = items.grimoire.sub_item_table[2]
    local closed = false
    item.callback({ closeMenu = function() closed = true end })
    check("PDFs: the menu closes first (kept open by TouchMenu)", closed and item.keep_menu_open == true)
    check("PDFs: the file browser goes to grimoire/PDFs", at("changeToPath " .. W.pdfs) ~= nil)
    local note = last_shown()
    check("PDFs: the note about highlights", note == "Grimoire rewrites these PDFs whenever you close a notebook, so highlights KOReader writes into them are lost: read or annotate copies.")
    check("PDFs: the note goes by itself", W.shown[#W.shown].timeout ~= nil)
    W.calls = {}
    g = koreader(true)
    g:showPDFs()
    check("PDFs from a book: the book closes, then the file browser",
        at("onClose") == 1 and at("showFileManager " .. W.pdfs .. "/") == 2)
    world("no-pdfs")
    g = koreader(false)
    g:showPDFs()
    check("no PDFs yet: says so", told("No notebook PDFs yet: Grimoire writes one when you close a notebook."))
    check("no PDFs yet: the file browser stays", not at("changeToPath"))
end

do -- Tools > Grimoire > Reset pen calibration.
    world("reset")
    app("guard hosted")
    local g = koreader(false)
    g:askReset()
    local box = W.shown[#W.shown]
    check("reset: asks first", box and box.kind == "confirm" and calls_of("execute") == 0
        and box.ok_text == "Reset")
    if box and box.ok_callback then box.ok_callback() end
    check("reset: runs grimoire.sh --reset-calibration", at("execute sh '" .. W.app_sh .. "' --reset-calibration") ~= nil)
    check("reset: says it is done", told("Pen and touch calibration reset."))
    W.status.reset = 256
    g:askReset()
    box = W.shown[#W.shown]
    if box and box.ok_callback then box.ok_callback() end
    check("reset fails: says so", told("Could not reset the calibration. See grimoire/grimoire.log."))
    check("reset: no hand-over", not wrapped() and not at("exitOrRestart"))
    world("reset-no-app")
    g = koreader(false)
    g:askReset()
    check("reset without the app: nothing to reset", told("Nothing to reset: Grimoire has no settings on this Kindle yet."))
    check("reset without the app: nothing run", calls_of("execute") == 0)
end

do -- KOReader in Spanish: the plugin's own table; in another language, English.
    world("spanish")
    W.settings.language = "es"
    local g = koreader(false)
    local items = {}
    g:addToMainMenu(items)
    local sub = items.grimoire.sub_item_table
    check("Spanish: the menu", sub[1].text == "Abrir Grimoire" and sub[2].text == "PDF de los cuadernos"
        and sub[3].text == "Restablecer la calibración del lápiz")
    check("Spanish: the gesture", W.actions.grimoire_open.title == "Abrir Grimoire")
    os.remove(W.koreader_sh)
    g:onOpenGrimoire()
    check("Spanish: the messages", last_shown() and last_shown():find("^Grimoire solo se abre desde") ~= nil)
    world("portuguese")
    W.settings.language = "pt_BR"
    g = koreader(false)
    items = {}
    g:addToMainMenu(items)
    check("another language: English", items.grimoire.sub_item_table[1].text == "Open Grimoire")
end

do -- _meta.lua: the name and description in KOReader's plugin list.
    world("meta")
    local meta = dofile(plugin_dir .. "/_meta.lua")
    check("_meta: fullname Grimoire", meta.fullname == "Grimoire")
    check("_meta: an English description", type(meta.description) == "string" and meta.description:find("^Opens Grimoire") ~= nil)
    W.settings.language = "es"
    meta = dofile(plugin_dir .. "/_meta.lua")
    check("_meta: a Spanish description", meta.description:find("^Abre Grimoire") ~= nil)
end

do -- No em dash (U+2014) anywhere in the plugin.
    for _, name in ipairs({ "main.lua", "_meta.lua" }) do
        local f = assert(io.open(plugin_dir .. "/" .. name, "rb"))
        local src = f:read("*a")
        f:close()
        check("no em dash in " .. name, not src:find("\226\128\148", 1, true))
    end
end

print(string.format("koplugin test: %d of %d checks failed", fails, count))
os.exit(fails == 0 and 0 or 1)
