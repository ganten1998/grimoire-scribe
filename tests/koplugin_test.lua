-- tests/koplugin_test.lua: KOReader's Grimoire plugin (kindle/koreader/grimoire.koplugin) run
-- against stand-ins for the KOReader modules it uses: the device and its screen, UIManager, the
-- menus, the dispatcher, the settings, BlitBuffers, the documents (paged and reflowable, and one
-- of an older KOReader that cannot draw a page by itself), os.execute and os.getenv. Each case
-- builds a scratch Kindle (a documents/Grimoire.sh with a header, an unpacked app,
-- /var/tmp/koreader.sh) and plays one way of using the plugin, down to reader.lua's Device:exit()
-- and KOReader's next start. tests/koplugin_check.sh runs it with LuaJIT.
-- usage: luajit tests/koplugin_test.lua PLUGIN_DIR SCRATCH_DIR
local plugin_dir, scratch = arg[1], arg[2]
assert(plugin_dir and scratch, "usage: luajit tests/koplugin_test.lua PLUGIN_DIR SCRATCH_DIR")

local real_execute, real_open, real_rename = os.execute, io.open, os.rename
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
local function write(path, text) -- (the test's own writes: never counted as the plugin's)
    local f = assert(real_open(path, "wb"))
    f:write(text)
    f:close()
end
local function is_file(path) return sh("test -f " .. q(path)) end

-- The world of one case: what the stand-ins answer, and every call they saw (W.calls, in order).
local W

-- Every file the plugin writes, by the path it writes it at (W.wrote: io.open in a mode that
-- writes, writePNG, Screen:shot), and every rename, "from -> to" (W.renamed): a request and its
-- picture are held to "written into a temporary file, then renamed into place". W.events has the
-- same in the order they came: "open P", "write P" and "close P" for a file io.open opened to
-- write (writePNG and Screen:shot write theirs whole: "open P", "close P"), and "rename F -> T", so
-- a rename that came before its file was written and closed is seen. A rename to a path that ends
-- in W.rename_fails fails, as on a full disk.
local function logged_file(f, path) -- f, with its writes and its close in W.events
    return setmetatable({}, { __index = function(_, k)
        local method = f[k]
        if type(method) ~= "function" then return method end
        return function(_, ...)
            if k == "write" or k == "close" then table.insert(W.events, k .. " " .. path) end
            return method(f, ...)
        end
    end })
end
io.open = function(path, mode) -- luacheck: ignore
    if not (W and mode and mode:find("[wa+]")) then return real_open(path, mode) end
    table.insert(W.wrote, path)
    table.insert(W.events, "open " .. path)
    local f, err = real_open(path, mode)
    if not f then return f, err end
    return logged_file(f, path)
end
os.rename = function(from, to) -- luacheck: ignore
    if W then
        table.insert(W.renamed, from .. " -> " .. to)
        table.insert(W.events, "rename " .. from .. " -> " .. to)
        if W.rename_fails and to:sub(-#W.rename_fails) == W.rename_fails then return nil, "No space left on device" end
    end
    return real_rename(from, to)
end

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
Device.hasWifiRestore = function() return W.wifi_supported ~= false end
local Network = {
    restoreWifiAsync = function() W.wifi_restores=W.wifi_restores+1;if W.wifi_error then error("network unavailable") end;W.wifi_on=true end,
    queryNetworkState = function() if W.wifi_error then error("network unavailable") end end,
    isWifiOn = function() return W.wifi_on end,
    isConnected = function() return W.wifi_connected end,
    scheduleConnectivityCheck = function() W.wifi_checks=W.wifi_checks+1 end,
    toggleWifiOff = function() W.wifi_on=false end,
    toggleWifiOn = function() W.wifi_on=true end,
}
package.loaded["ui/network/manager"]=Network
-- KindlePowerD's lipc handle, for preventScreenSaver (W.prevent; W.lipc_error: the read fails),
-- each read in W.lipc_reads. world() puts it back on Device.powerd, and a fresh PluginShare
-- (W.share).
local function lipc_handle()
    return { get_int_property = function(_, service, property)
        table.insert(W.lipc_reads, service .. " " .. property)
        if W.lipc_error then error("lipc: timed out") end
        return W.prevent
    end }
end
-- The screen, of W.screen's size; its shot (KOReader's Screen:shot) writes what it shows, and
-- answers as the screen's writePNG does (W.png_answers, below).
local function png_answer(path, text)
    table.insert(W.wrote, path)
    table.insert(W.events, "open " .. path)
    table.insert(W.events, "close " .. path)
    if W.png_answers == "false" then -- a newer KOReader, the disk full part way
        write(path, text:sub(1, 3))
        return false, "failed to write"
    end
    write(path, W.png_answers == "empty" and "" or text)
    if W.png_answers == "nothing" or W.png_answers == "empty" then return end -- (an older KOReader's)
    return true
end
-- Its rotation, as KOReader's framebuffer has it on a Scribe (software rotation): the native rotation
-- KindleScribe:init picked (world() makes it upright, as the framebuffer's 3 says), the framebuffer's
-- own (W.hw_rotation; W.hw_error: the ioctl fails) and the rotation (W.rotation), each setRotationMode
-- in W.rotations as "<mode>@<native>".
Device.screen = {
    DEVICE_ROTATED_UPRIGHT = 0, DEVICE_ROTATED_CLOCKWISE = 1, DEVICE_ROTATED_UPSIDE_DOWN = 2,
    DEVICE_ROTATED_COUNTER_CLOCKWISE = 3,
    getHWRotation = function()
        if W.hw_error then error("cannot get variable screen info") end
        return W.hw_rotation
    end,
    getRotationMode = function() return W.rotation end,
    setRotationMode = function(self, mode)
        table.insert(W.rotations, mode .. "@" .. tostring(self.native_rotation_mode))
        W.rotation = mode
    end,
    getWidth = function() return W.screen.w end,
    getHeight = function() return W.screen.h end,
    shot = function(_, path)
        table.insert(W.calls, "shot")
        return png_answer(path, string.format("SHOT %dx%d\n", W.screen.w, W.screen.h))
    end,
}
-- KOReader's input, as KindleScribe:init and Device:init leave it with an accelerometer: its gyro
-- handler (Input:handleMiscGyroEv, as KOReader has it) as handleGyroEv, unless the accelerometer
-- is ignored (W.gsensor_ignored: KOReader's voidEv instead); the lock as W.locked has it, a turn
-- under it set straight on the screen (and UIManager:onRotation, in W.calls). The events it is
-- given are in W.gyro as "<type>:<code>:<value>". KOReader's MSC_GYRO values, as ffi/linux_input_h
-- declares them, for the plugin's require of it.
local ffi = require("ffi")
ffi.cdef[[
static const unsigned EV_MSC = 4;
static const unsigned MSC_GYRO = 71;
static const unsigned DEVICE_ROTATED_UPRIGHT = 0;
static const unsigned DEVICE_ROTATED_CLOCKWISE = 1;
static const unsigned DEVICE_ROTATED_UPSIDE_DOWN = 2;
static const unsigned DEVICE_ROTATED_COUNTER_CLOCKWISE = 3;
]]
package.loaded["ffi/linux_input_h"] = true
local function handle_misc_gyro(_, ev)
    table.insert(W.gyro, string.format("%d:%d:%d", ev.type, ev.code, ev.value))
    local rotation = ({ [0] = 0, 1, 2, 3 })[ev.value]
    if not rotation then return end
    local old_rotation = Device.screen:getRotationMode()
    if Device:isGSensorLocked() then
        if rotation ~= old_rotation and rotation % 2 == old_rotation % 2 then
            Device.screen:setRotationMode(rotation)
            table.insert(W.calls, "onRotation")
        end
    elseif rotation ~= old_rotation then
        return { name = "SetRotationMode", args = { rotation } }
    end
end
local function void_ev() end
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
    -- A SetRotationMode turns the screen, as the reader's or the file browser's handler does.
    sendEvent = function(_, event)
        table.insert(W.calls, "sendEvent " .. event.name .. " " .. tostring(event.args[1]))
        if event.name == "SetRotationMode" then Device.screen:setRotationMode(event.args[1]) end
    end,
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
local function file_size(path)
    local f = io.open(path, "rb")
    if not f then return nil end
    local n = f:seek("end")
    f:close()
    return n
end
-- lfs.attributes(path) as a table has the device and the inode too, as stat has them after links
-- (GNU stat, else BSD's), so two names of one folder are seen as one.
local function first_line(cmd)
    local p = io.popen(cmd)
    local line = p:read("*l")
    p:close()
    return line
end
package.loaded["libs/libkoreader-lfs"] = {
    attributes = function(path, what)
        local mode = sh("test -d " .. q(path)) and "directory" or (sh("test -f " .. q(path)) and "file") or nil
        if what == "mode" then return mode end
        if what == "size" then return mode == "file" and file_size(path) or nil end
        if not mode then return nil end
        local dev, ino = (first_line("stat -L -c '%d %i' " .. q(path) .. " 2>/dev/null || stat -L -f '%d %i' "
            .. q(path) .. " 2>/dev/null") or ""):match("^(%d+) (%d+)$")
        return { mode = mode, dev = tonumber(dev), ino = tonumber(ino) }
    end,
    currentdir = function() return first_line("pwd") end,
    mkdir = function(path)
        if sh("mkdir " .. q(path) .. " 2>/dev/null") then return true end
        return nil, "cannot"
    end,
    dir = function(path)
        local p = io.popen("ls -a " .. q(path) .. " 2>/dev/null")
        local names = {}
        for name in p:lines() do table.insert(names, name) end
        p:close()
        local i = 0
        return function()
            i = i + 1
            return names[i]
        end
    end,
}
-- BlitBuffers: their size, what was drawn into them (W.drawn), and the PNG each writes (a line
-- naming its size and what it holds). W.png_error makes writing raise an error; W.png_answers how
-- it answers: "false" (cut short, and false with why, as a newer KOReader does), "nothing" (an
-- older KOReader's: no answer), "empty" (no answer, an empty file); else true.
local function new_bb(w, h, what)
    local bb = { w = w, h = h, what = what or "blank" }
    function bb.getWidth() return bb.w end
    function bb.getHeight() return bb.h end
    function bb.fill(_, color) bb.filled = color end
    function bb.paintRect(_, x, y, rw, rh, color)
        table.insert(W.drawn, string.format("paintRect %d %d %d %d %d", x, y, rw, rh, color))
    end
    function bb.blitFrom(_, src, x, y, ox, oy, bw, bh)
        bb.what = src.what
        table.insert(W.drawn, string.format("blitFrom %s %d %d %d %d %d %d", src.what, x, y, ox, oy, bw, bh))
    end
    function bb.writePNG(_, path)
        if W.png_error then error("disk full") end
        return png_answer(path, string.format("PNG %dx%d %s\n", bb.w, bb.h, bb.what))
    end
    function bb.free() W.freed = W.freed + 1 end
    return bb
end
package.loaded["ffi/blitbuffer"] = {
    TYPE_BB8 = 1,
    COLOR_WHITE = 255,
    new = function(w, h, kind)
        assert(kind == 1, "a page picture is Gray8")
        return new_bb(w, h)
    end,
}
package.loaded["ui/geometry"] = { new = function(_, o) return o end }
package.loaded["ui/renderimage"] = {
    scaleBlitBuffer = function(_, bb, w, h, free_orig)
        table.insert(W.drawn, string.format("scale %dx%d to %dx%d", bb.w, bb.h, w, h))
        assert(free_orig == false, "the plugin frees what it made itself")
        return new_bb(w, h, "scaled " .. bb.what)
    end,
}
package.loaded["util"] = { partialMD5 = function(file)
    table.insert(W.calls, "partialMD5")
    return W.md5_of_file
end }
-- The plugin never shows a document itself (KOReader's own start does): this one would show it.
package.loaded["apps/reader/readerui"] = {
    showReader = function() table.insert(W.calls, "showReader") end,
}
-- KOReader's Trapper: wrap runs the work at once; each info is in W.calls ("trapper <text>", its line
-- break as " | ") and answers true, or false from the W.stop_at'th on (a tap on it stopped the work).
package.loaded["ui/trapper"] = {
    wrap = function(_, f) return f() end,
    info = function(_, text)
        table.insert(W.calls, "trapper " .. text:gsub("\n", " | "))
        W.infos = (W.infos or 0) + 1
        return not (W.stop_at and W.infos >= W.stop_at)
    end,
    clear = function() table.insert(W.calls, "trapper clear") end,
}
-- KOReader's renderPage puts the contrast and the last argument (the saturation, the render mode
-- before 2024) into its cache key, and fails on a page drawn without either, as crash.log has it.
local function draw_args_ok(gamma, last)
    if gamma == nil or last == nil then
        error("frontend/document/document.lua:398: attempt to concatenate local 'saturation' (a nil value)", 0)
    end
end
-- A paged document of book b (W.files[path], or a reader's), any page drawn: W.drawn has each.
local function paged_doc(b)
    local doc = { file = b.file, is_locked = b.locked }
    function doc.getPageCount() return b.pages end
    function doc.getProps() return { title = b.title } end
    function doc.getNativePageDimensions(_, page)
        assert(page >= 1 and page <= b.pages, "a page of the book")
        return { w = b.native.w, h = b.native.h }
    end
    function doc.getPageDimensions(_, page, zoom, rotation)
        assert(page >= 1 and page <= b.pages and rotation == 0)
        return { x = 0, y = 0, w = math.floor(b.native.w * zoom + 0.5), h = math.floor(b.native.h * zoom + 0.5) }
    end
    function doc.drawPage(_, target, x, y, rect, page, zoom, rotation, gamma, last)
        draw_args_ok(gamma, last)
        table.insert(W.drawn, string.format("drawPage %d %d %dx%d %d %.4f %d %s %s", x, y, rect.w, rect.h,
            page, zoom, rotation, tostring(gamma), tostring(last)))
        target.what = "page " .. page
    end
    return doc
end
-- The file browser's document registry: the auxiliary provider registered (W.aux), and the books
-- Open with opens (W.files), each opened and closed in W.calls.
package.loaded["document/documentregistry"] = {
    addAuxProvider = function(_, provider) W.aux = provider end,
    openDocument = function(_, file)
        table.insert(W.calls, "openDocument")
        local b = W.files and W.files[file]
        if not b then error("no such book") end
        return paged_doc(b)
    end,
    closeDocument = function(_, file)
        table.insert(W.calls, "closeDocument")
        assert(W.files and W.files[file], "the book it opened")
    end,
}
-- A book's settings, as KOReader keeps them (its key, the page it was left at).
package.loaded["docsettings"] = {
    open = function(_, file)
        return { readSetting = function(_, key)
            local b = W.files and W.files[file]
            if not b then return nil end
            if key == "partial_md5_checksum" then return b.md5 end
            if key == "last_page" then return b.last_page end
            return b.settings and b.settings[key]
        end }
    end,
}
-- Every line logged, as KOReader's crash.log would have it (W.logs), to hold it to what it may say.
local function log_line(...)
    local parts = {}
    for i = 1, select("#", ...) do parts[i] = tostring((select(i, ...))) end
    table.insert(W.logs, table.concat(parts, " "))
end
package.loaded["logger"] = {
    info = function(...) log_line(...) end,
    dbg = function() end,
    warn = function(...)
        log_line(...)
        table.insert(W.calls, "warn")
    end,
    err = function(...)
        log_line(...)
        table.insert(W.calls, "err")
    end,
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
G_defaults = { -- luacheck: ignore (KOReader's global: its defaults.lua, W.defaults over it)
    readSetting = function(_, key) return W.defaults and W.defaults[key] end,
}
-- ReaderView, for the calls it has: onSaturationUpdate from KOReader 2024 on (W.old_view: before).
package.preload["apps/reader/modules/readerview"] = function()
    return setmetatable({}, { __index = function(_, k)
        if k == "onSaturationUpdate" and not W.old_view then return function() end end
    end })
end
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
        wifi_on = true, wifi_connected = true, wifi_restores = 0, wifi_checks = 0,
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
        unpack_writes = "#!/bin/sh\n# Capabilities: guard hosted annotate\n",
        drawn = {},
        freed = 0,
        logs = {},
        wrote = {},
        renamed = {},
        events = {},
        screen = { w = 1860, h = 2480 },
        -- The book a book's screen has open (new_ui): kind "paged" (a PDF), "reflowable" (an
        -- EPUB) or "old" (a PDF in a KOReader that cannot draw a page by itself).
        book = { kind = "paged", page = 12, pages = 96, md5 = "0f3c6e2a9b1d4c7e8f5a2b3c4d5e6f70",
            title = "Le Petit Prince", native = { w = 612, h = 792 }, gamma = 1.5, saturation = 1, header = 0 },
    }
    W.us = W.dir .. "/us"
    W.book.file = W.us .. "/documents/book.epub"
    W.annotate = W.us .. "/grimoire/import/.annotate"
    W.doc = W.us .. "/documents/Grimoire.sh"
    W.app_sh = W.us .. "/grimoire/app/grimoire.sh"
    W.koreader_sh = W.dir .. "/var/tmp/koreader.sh"
    W.marker = W.dir .. "/settings/grimoire_launch.txt"
    W.pdfs = W.us .. "/grimoire/PDFs"
    Network.pending_connection,Network.pending_connectivity_check=false,false
    W.prevent, W.lipc_reads, W.share = 0, {}, {}
    Device.model, Device.screen.native_rotation_mode = "KindleScribe", 0
    W.hw_rotation, W.rotation, W.rotations = 3, 0, {}
    Device.powerd = { lipc_handle = lipc_handle() }
    -- The orientation sensor's current code (W.sensor; nil: none found; W.sensor_error: the read
    -- fails), each read counted in W.sensor_reads; KOReader's accelerometer (W.gsensor), its lock.
    W.sensor, W.sensor_reads, W.gsensor, W.locked, W.gyro = nil, 0, true, false, {}
    W.back = W.dir .. "/settings/grimoire_back.txt"
    Device.hasGSensor = function() return W.gsensor end
    Device.isGSensorLocked = function() return W.locked end
    Device.input = { handleMiscGyroEv = handle_misc_gyro, handleGyroEv = handle_misc_gyro }
    package.loaded["pluginshare"] = W.share
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
        local b = W.book
        local doc = { file = b.file }
        function doc.getPageCount() return b.pages end
        function doc.getCurrentPage() return b.page end
        function doc.getProps() return { title = b.title } end
        ui.document = doc
        ui.getCurrentPage = function() return b.page end
        ui.doc_settings = { readSetting = function(_, key)
            if key == "partial_md5_checksum" then return b.md5 end
            return b.settings and b.settings[key]
        end }
        ui.doc_props = { display_title = b.title }
        if b.kind == "reflowable" then
            ui.rolling = {}
            ui.view = { view_mode = b.view_mode or "page", state = { page = b.page, pos = 4321 },
                visible_area = { x = 0, y = 0, w = W.screen.w, h = W.screen.h } }
            doc._nightmode_images = true  -- (night mode's: images drawn inverted for an inverted screen)
            function doc.drawCurrentViewByPage(_, target, x, y, rect, page)
                table.insert(W.drawn, string.format("drawCurrentViewByPage %d %d %dx%d %d", x, y, rect.w, rect.h, page))
                table.insert(W.drawn, "night images " .. tostring(doc._nightmode_images))
                target.what = "text " .. page
            end
            function doc.drawCurrentViewByPos(_, target, x, y, rect, pos)
                table.insert(W.drawn, string.format("drawCurrentViewByPos %d %d %dx%d %d", x, y, rect.w, rect.h, pos))
                target.what = "text at " .. pos
            end
            function doc.getHeaderHeight() return b.header end
        else
            ui.paging = {}
            -- A PDF read reflowed, its scan cleaned up, inverted for night mode.
            doc.configurable = { text_wrap = 1, page_opt = 1, auto_straighten = 5, white_threshold = 200,
                nightmode_document = 1 }
            ui.view = { state = { page = b.page, gamma = b.gamma, saturation = b.saturation }, render_mode = b.render_mode }
            if b.kind == "old" then
                ui.view.render_mode = 0
            else
                function doc.getNativePageDimensions(_, page)
                    assert(page == b.page or (b.whole and page >= 1 and page <= b.pages), "the page the reader is on")
                    return { w = b.native.w, h = b.native.h }
                end
                function doc.getPageDimensions(_, page, zoom, rotation)
                    assert((page == b.page or (b.whole and page >= 1 and page <= b.pages)) and rotation == 0)
                    return { x = 0, y = 0, w = math.floor(b.native.w * zoom + 0.5), h = math.floor(b.native.h * zoom + 0.5) }
                end
                function doc.drawPage(_, target, x, y, rect, page, zoom, rotation, gamma, last)
                    draw_args_ok(gamma, last)
                    table.insert(W.drawn, string.format("drawPage %d %d %dx%d %d %.4f %d %s %s", x, y, rect.w, rect.h,
                        page, zoom, rotation, tostring(gamma), tostring(last)))
                    local c = doc.configurable
                    table.insert(W.drawn, string.format("settings %s %s %s %s %s", tostring(c.text_wrap),
                        tostring(c.page_opt), tostring(c.auto_straighten), tostring(c.white_threshold),
                        tostring(c.nightmode_document)))
                    target.what = "page " .. page
                end
            end
        end
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
    Plugin.orientation_sensor_code = function()
        W.sensor_reads = W.sensor_reads + 1
        if W.sensor_error then error("ioctl failed") end
        if W.sensor == nil then return nil, "no orientation sensor found" end
        return W.sensor, "/dev/input/event2"
    end
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
        #sub == 5 and sub[1].text == "Open Grimoire" and sub[2].text == "Notebook PDFs"
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

do -- Keep alive as the Kindle has it: KOReader starts again with the power service keeping the
    -- Kindle awake (its Keep alive from before Grimoire, put back by grimoire.sh) and its own flag
    -- off, so AutoSuspend's Kindle timer would spin. The flag comes back, once per KOReader run.
    world("keepalive")
    W.prevent = 1
    local g = koreader(false)
    local s = "kept awake"
    check(s .. ": Keep alive's flag on again", W.share.keepalive == true)
    check(s .. ": read once, from the power service",
        #W.lipc_reads == 1 and W.lipc_reads[1] == "com.lab126.powerd preventScreenSaver")
    check(s .. ": logged", W.logs[#W.logs] == "Grimoire: the Kindle is kept awake (preventScreenSaver 1): KOReader's Keep alive is on again")
    check(s .. ": the menu still registered", W.menus == 1)
    g:new{ ui = new_ui(true) }
    check(s .. ": a book's screen in the same run reads it no more", #W.lipc_reads == 1)
    untouched(s)

    world("keepalive-off")
    koreader(false)
    check("not kept awake: the flag left alone, nothing logged",
        W.share.keepalive == nil and #W.lipc_reads == 1 and #W.logs == 0)

    world("keepalive-already")
    W.prevent, W.share.keepalive = 1, true
    koreader(false)
    check("Keep alive already on: not read", #W.lipc_reads == 0 and W.share.keepalive == true and #W.logs == 0)

    world("keepalive-no-lipc")
    W.prevent, Device.powerd = 1, {}
    koreader(false)
    check("no lipc handle: the flag left alone, no warning",
        W.share.keepalive == nil and not at("warn") and W.menus == 1)

    world("keepalive-lipc-fails")
    W.prevent, W.lipc_error = 1, true
    koreader(false)
    check("the read fails: the flag left alone, a warning, the menu registered",
        W.share.keepalive == nil and at("warn") and W.menus == 1)

    world("keepalive-not-kindle")
    W.kindle, W.prevent = false, 1
    koreader(false)
    check("not a Kindle: not read", #W.lipc_reads == 0 and W.share.keepalive == nil)
end

do -- KOReader's native rotation as the framebuffer has it: back from Grimoire, the window manager
    -- answered with a turn it never applied (D, so native upside down) while the framebuffer is as
    -- upright (3): drawing and touch 180 degrees apart. The native rotation follows the framebuffer,
    -- and the rotation the last run closed with (2) is set again over it. Once per KOReader run.
    world("rotation-apart")
    Device.screen.native_rotation_mode, W.rotation = 2, 2
    local g = koreader(false)
    local s = "rotation apart"
    check(s .. ": the native rotation the framebuffer's (upright)", Device.screen.native_rotation_mode == 0)
    check(s .. ": the same rotation set again over it, once", #W.rotations == 1 and W.rotations[1] == "2@0")
    check(s .. ": logged", W.logs[#W.logs] == "Grimoire: KOReader's native rotation 2 against the framebuffer's 3 "
        .. "(drawing and touch apart): native rotation 0 and rotation 2 again")
    Device.screen.native_rotation_mode = 2
    g:new{ ui = new_ui(true) }
    check(s .. ": a book's screen in the same run looks no more", #W.rotations == 1)
    untouched(s)

    world("rotation-apart-other-way")
    Device.screen.native_rotation_mode, W.hw_rotation = 0, 1
    koreader(false)
    check("the framebuffer upside down (1), KOReader upright: native upside down, rotation 0 again",
        Device.screen.native_rotation_mode == 2 and #W.rotations == 1 and W.rotations[1] == "0@2")

    for _, c in ipairs({ { "upright", 0, 3 }, { "upside-down", 2, 1 }, { "upright-0", 0, 0 } }) do
        world("rotation-agree-" .. c[1])
        Device.screen.native_rotation_mode, W.hw_rotation, W.rotation = c[2], c[3], 2
        koreader(false)
        check("rotation agreeing (native " .. c[2] .. ", framebuffer " .. c[3] .. "): left alone, nothing logged",
            Device.screen.native_rotation_mode == c[2] and #W.rotations == 0 and #W.logs == 0)
    end

    for _, m in ipairs({ "KindleScribe3", "KindleScribeColorSoft", "KindlePaperWhite5" }) do
        world("rotation-" .. m)
        Device.model, Device.screen.native_rotation_mode = m, 2
        koreader(false)
        check("another Kindle (" .. m .. "): left alone", Device.screen.native_rotation_mode == 2 and #W.rotations == 0)
    end

    world("rotation-sideways-native")
    Device.screen.native_rotation_mode = 1
    koreader(false)
    check("a native rotation KOReader's Scribe code never picks: left alone",
        Device.screen.native_rotation_mode == 1 and #W.rotations == 0)

    world("rotation-ioctl-fails")
    Device.screen.native_rotation_mode, W.hw_error = 2, true
    koreader(false)
    check("the framebuffer's rotation does not read: left alone, a warning, the menu registered",
        Device.screen.native_rotation_mode == 2 and #W.rotations == 0 and at("warn") and W.menus == 1)

    world("rotation-not-kindle")
    W.kindle, Device.screen.native_rotation_mode = false, 2
    koreader(false)
    check("not a Kindle: left alone", Device.screen.native_rotation_mode == 2 and #W.rotations == 0)
end

do -- KOReader back from Grimoire takes its rotation from a window manager kept stopped all through
    -- Grimoire, and the orientation sensor tells only of turns: held still upside down, KOReader
    -- stayed upright (the owner, 2026-10-28). Back from a hand-over, on a Scribe, the sensor's code
    -- now goes to KOReader's own accelerometer handler, on the next tick, after the native rotation.
    local function back_from_grimoire(name, setup, in_book, before_start)
        world(name)
        if setup then setup() end
        local g = koreader(false)
        g:onOpenGrimoire()
        W.calls = {}
        Device:exit()
        check(name .. ": the back marker written", is_file(W.back))
        W.ticks = {} -- reader.lua exited: its unrun startup task does not survive into the new process.
        -- koreader.sh starts KOReader again: KindleScribe:init has set its rotation (before_start).
        if before_start then before_start() end
        W.rotations = {}
        if in_book == nil then in_book = true end
        koreader(in_book)
    end
    local s = "back upside down"
    back_from_grimoire("sensor-upside-down", function() W.sensor = 16 end)
    check(s .. ": a marker says the hand-over came back", not is_file(W.back) and at("execute sh "))
    check(s .. ": the sensor read on the next tick, not before", W.sensor_reads == 0 and W.rotation == 0)
    run_ticks()
    check(s .. ": read once", W.sensor_reads == 1)
    check(s .. ": KOReader's gyro handler had what KindleGyroTransform makes of 16",
        #W.gyro == 1 and W.gyro[1] == "4:71:2")
    check(s .. ": SetRotationMode upside down, sent on", at("sendEvent SetRotationMode 2") ~= nil)
    check(s .. ": the screen turned, the native rotation kept", W.rotation == 2 and W.rotations[#W.rotations] == "2@0"
        and Device.screen.native_rotation_mode == 0)
    check(s .. ": logged", W.logs[#W.logs] == "Grimoire: back from Grimoire, the orientation sensor reads 16 "
        .. "(DEVICE_ROTATED_UPSIDE_DOWN): rotation 0 -> 2")
    koreader(true)
    run_ticks()
    check(s .. ": the next ordinary start also checks its initial orientation", W.sensor_reads == 2 and W.rotation == 2)

    -- With baa43ea's step: KindleScribe:init took the window manager's stale D (native and rotation
    -- upside down) while the framebuffer is upright (3). The native rotation becomes the framebuffer's
    -- first (drawing and touch together again, rotation 2); the sensor, upside down, keeps it.
    back_from_grimoire("sensor-after-native", function() W.sensor = 16 end, true, function()
        Device.screen.native_rotation_mode, W.rotation = 2, 2
    end)
    run_ticks()
    s = "stale D, held upside down"
    check(s .. ": native the framebuffer's, rotation set again, then left alone", Device.screen.native_rotation_mode == 0
        and #W.rotations == 1 and W.rotations[1] == "2@0" and W.rotation == 2 and not at("sendEvent"))
    -- The framebuffer upside down (1), the window manager's stale U: native upside down first, then
    -- the sensor's turn on top of it (2 at native 2: drawn as the framebuffer is, touch at 2).
    back_from_grimoire("sensor-fb-upside-down", function() W.sensor = 16 end, true, function()
        W.hw_rotation = 1
    end)
    run_ticks()
    s = "framebuffer upside down, stale U"
    check(s .. ": native upside down, then SetRotationMode 2 over it", Device.screen.native_rotation_mode == 2
        and #W.rotations == 2 and W.rotations[1] == "0@2" and W.rotations[2] == "2@2")

    back_from_grimoire("sensor-ignored", function()
        W.sensor = 16
        Device.input.handleGyroEv = void_ev
    end)
    run_ticks()
    check("accelerometer ignored: nothing read, nothing turned", W.sensor_reads == 0 and #W.gyro == 0
        and W.rotation == 0 and not at("sendEvent"))
    check("accelerometer ignored: logged", W.logs[#W.logs] == "Grimoire: back from Grimoire, KOReader ignores the "
        .. "accelerometer: rotation 0 left alone")

    back_from_grimoire("sensor-locked-sideways", function()
        W.sensor, W.locked = 17, true
    end)
    run_ticks()
    check("locked, the sensor sideways from portrait: nothing turned", #W.gyro == 1 and W.gyro[1] == "4:71:1"
        and W.rotation == 0 and not at("sendEvent") and not at("onRotation"))
    check("locked, sideways: logged as left alone", W.logs[#W.logs] == "Grimoire: back from Grimoire, the "
        .. "orientation sensor reads 17 (DEVICE_ROTATED_CLOCKWISE): rotation 0 -> 0 (left alone: the same, or the "
        .. "orientation lock keeps it)")

    back_from_grimoire("sensor-locked-upside-down", function()
        W.sensor, W.locked = 16, true
    end)
    run_ticks()
    check("locked, upside down from portrait: turned within portrait, as the lock allows",
        W.rotation == 2 and at("onRotation") and not at("sendEvent"))

    back_from_grimoire("sensor-sideways", function() W.sensor = 18 end)
    run_ticks()
    check("unlocked, landscape right: SetRotationMode counter-clockwise", W.gyro[1] == "4:71:3"
        and at("sendEvent SetRotationMode 3") and W.rotation == 3)

    back_from_grimoire("sensor-same", function() W.sensor = 15 end)
    run_ticks()
    check("upright and KOReader upright: left alone", #W.gyro == 1 and W.rotation == 0 and not at("sendEvent")
        and W.logs[#W.logs]:find("rotation 0 -> 0 (left alone: the same)", 1, true) ~= nil)

    back_from_grimoire("sensor-none", function() W.sensor = nil end)
    local ok = pcall(run_ticks)
    check("no sensor: nothing turned, no error, no warning", ok and W.sensor_reads == 1 and #W.gyro == 0
        and W.rotation == 0 and not at("warn"))
    check("no sensor: logged", W.logs[#W.logs] == "Grimoire: back from Grimoire, the orientation sensor reads "
        .. "no orientation sensor found: rotation 0 left alone")

    back_from_grimoire("sensor-face-up", function() W.sensor = 19 end)
    run_ticks()
    check("a code with no turn (face up): left alone", #W.gyro == 0 and W.rotation == 0)

    back_from_grimoire("sensor-fails", function() W.sensor_error = true end)
    ok = pcall(run_ticks)
    check("the read fails: a warning, the tick does not raise", ok and at("warn") and W.rotation == 0)

    back_from_grimoire("sensor-refused", function()
        W.sensor, W.status.hosted = 16, 768
    end)
    run_ticks()
    check("grimoire.sh refused: back all the same, the sensor applied", W.rotation == 2 and not is_file(W.back))

    for _, m in ipairs({ "KindleScribe3", "KindleScribeColorSoft" }) do
        back_from_grimoire("sensor-" .. m, function()
            W.sensor = 16
            Device.model = m
        end)
        run_ticks()
        check(m .. ": the sensor applied", W.rotation == 2)
    end

    back_from_grimoire("sensor-paperwhite", function()
        W.sensor = 16
        Device.model = "KindlePaperWhite5"
    end)
    run_ticks()
    check("not a Scribe: no sensor read, the marker gone", W.sensor_reads == 0 and W.rotation == 0
        and not is_file(W.back))

    back_from_grimoire("sensor-no-gsensor", function()
        W.sensor, W.gsensor = 16, false
    end)
    run_ticks()
    check("no accelerometer in KOReader: no sensor read", W.sensor_reads == 0 and W.rotation == 0)

    world("sensor-ordinary-start")
    W.sensor = 16
    koreader(true)
    run_ticks()
    check("an ordinary start: the missing initial sensor event is supplied", W.sensor_reads == 1 and W.rotation == 2)
end

do -- Ordinary Kindle starts have no back marker. Supply the initial event after the browser or
    -- reader restores its settings, even when an upright default was persisted earlier.
    for _, view in ipairs({ "browser", "paged", "reflowable" }) do
        for code = 15, 18 do
            local name = "startup-" .. view .. "-" .. code
            world(name)
            W.sensor = code
            W.book.kind = view == "reflowable" and "reflowable" or "paged"
            local g = koreader(view ~= "browser")
            check(name .. ": deferred until the view is ready", W.sensor_reads == 0 and W.rotation == 0)
            run_ticks()
            check(name .. ": follows the sensor with native rotation unchanged", W.sensor_reads == 1
                and W.rotation == ({ [15] = 0, [16] = 2, [17] = 1, [18] = 3 })[code]
                and Device.screen.native_rotation_mode == 0)
            check(name .. ": no marker or saved rotation written", not is_file(W.back) and next(W.settings) == nil)
            g:new{ ui = new_ui(true) }
            run_ticks()
            check(name .. ": only once across views in one process", W.sensor_reads == 1)
        end
    end

    for _, c in ipairs({
        { "browser-upright", false, "fm_rotation_mode", 0 },
        { "browser-landscape", false, "fm_rotation_mode", 1 },
        { "paged-default", true, "kopt_rotation_mode", 0 },
        { "reflowable-default", true, "copt_rotation_mode", 0, "reflowable" },
        { "reflowable-landscape", true, "copt_rotation_mode", 3, "reflowable" },
        { "paged-document-upright", true, "kopt_rotation_mode", 0, "paged", true },
        { "reflowable-document-upright", true, "copt_rotation_mode", 0, "reflowable", true },
        { "keep-current", true, "lock_rotation", true },
    }) do
        world("startup-saved-" .. c[1])
        local rotation = c[4] == true and 1 or c[4]
        W.sensor, W.rotation = 16, rotation
        W.book.kind = c[5] or "paged"
        local g = koreader(c[2])
        -- As ReaderUI does, set/read document settings after plugin init and before the first tick.
        if c[6] then W.book.settings = { [c[3]] = c[4] } else W.settings[c[3]] = c[4] end
        run_ticks()
        check(c[1] .. ": sensor wins over restored rotation", W.sensor_reads == 1
            and W.gyro[1] == "4:71:2" and W.rotation == 2)
        check(c[1] .. ": saved value kept", (c[6] and W.book.settings or W.settings)[c[3]] == c[4])
        g:new{ ui = new_ui(c[2]) }; run_ticks()
        check(c[1] .. ": not retried in the same process", W.sensor_reads == 1)
    end

    world("startup-unrelated-settings")
    W.settings.fm_rotation_mode, W.settings.copt_rotation_mode = 1, 3
    W.book.settings, W.sensor = { copt_rotation_mode = 1 }, 16
    koreader(true); run_ticks()
    check("a paged book ignores saved rotations belonging to other views", W.sensor_reads == 1 and W.rotation == 2)

    world("startup-ignore")
    W.sensor = 16
    koreader(false)
    Device.input.handleGyroEv = void_ev -- a setting/plugin can change this before the first tick
    run_ticks()
    check("startup: ignored accelerometer is not read", W.sensor_reads == 0 and W.rotation == 0)

    world("startup-persisted-zero-ignore")
    W.sensor, W.settings.fm_rotation_mode = 16, 0
    Device.input.handleGyroEv = void_ev
    koreader(false); run_ticks()
    check("persisted 0 with gyro ignored: no read or rotation", W.sensor_reads == 0 and W.rotation == 0)

    world("startup-persisted-zero-lock")
    W.sensor, W.settings.lock_rotation, W.settings.fm_rotation_mode, W.locked = 16, true, 0, true
    koreader(false); run_ticks()
    check("persisted 0 with gyro locked: upside down still allowed within portrait", W.sensor_reads == 1
        and W.rotation == 2 and at("onRotation") and not at("sendEvent"))

    for _, c in ipairs({ { 17, 0, 0 }, { 16, 0, 2 }, { 18, 1, 3 }, { 15, 1, 1 } }) do
        world("startup-locked-" .. c[1])
        W.sensor, W.rotation, W.locked = c[1], c[2], true
        koreader(false); run_ticks()
        check("startup: locked gyro " .. c[1] .. " preserves the orientation pair", W.sensor_reads == 1 and W.rotation == c[3])
    end

    world("startup-native-first")
    W.sensor, W.hw_rotation, Device.screen.native_rotation_mode = 16, 1, 0
    koreader(false); run_ticks()
    check("startup: native correction precedes gyro correction", #W.rotations == 2
        and W.rotations[1] == "0@2" and W.rotations[2] == "2@2")

    for _, model in ipairs({ "KindleScribe3", "KindleScribeColorSoft", "KindlePaperWhite5" }) do
        world("startup-" .. model)
        Device.model, W.sensor = model, 16
        koreader(false); run_ticks()
        check(model .. ": startup sensor is restricted to Scribes", W.sensor_reads == (model == "KindlePaperWhite5" and 0 or 1))
    end
    world("startup-no-gsensor")
    W.gsensor, W.sensor = false, 16
    koreader(false); run_ticks()
    check("startup: no accelerometer capability, no read", W.sensor_reads == 0 and W.rotation == 0)

    for _, code in ipairs({ 0, 19, 20 }) do
        world("startup-no-turn-" .. code)
        W.sensor = code
        koreader(false); run_ticks()
        check("startup: non-rotation code " .. code .. " leaves rotation alone", W.sensor_reads == 1 and #W.gyro == 0 and W.rotation == 0)
    end
    world("startup-no-sensor")
    koreader(false)
    check("startup: missing sensor is harmless", pcall(run_ticks) and W.sensor_reads == 1 and W.rotation == 0 and not at("warn"))
    world("startup-sensor-error")
    W.sensor_error = true
    koreader(false)
    check("startup: sensor error cannot escape the UI task", pcall(run_ticks) and W.sensor_reads == 1 and W.rotation == 0 and at("warn"))

    world("return-keeps-existing-policy")
    W.sensor, W.settings.fm_rotation_mode, W.settings.lock_rotation = 16, 0, true
    write(W.back, "back\n")
    koreader(false); run_ticks()
    check("return: marker still follows gyro despite saved startup preferences", W.sensor_reads == 1 and W.rotation == 2
        and not is_file(W.back) and W.settings.fm_rotation_mode == 0 and W.settings.lock_rotation == true)
end

do -- KOReader not started by koreader.sh (no /var/tmp/koreader.sh).
    world("no-koreader-sh")
    os.remove(W.koreader_sh)
    local g = koreader(false)
    g:onOpenGrimoire()
    check("no koreader.sh: says why", told("Grimoire opens only from a KOReader started by koreader.sh (its Library item or KUAL)."))
    untouched("no koreader.sh")
end

do -- KPVBooklet and other launchers may leave the native UI running.
    world("not-paused")
    W.env = { KOREADER_DIR = "/mnt/us/koreader", STOP_FRAMEWORK = "no", AWESOME_STOPPED = "no" }
    local g = koreader(false)
    check("unpaused host: accepted for launcher-managed pause", g:refusal() == nil)
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
    local handle = {
        set_string_property = function() table.insert(W.calls, "framework restore screensaver") end,
        close = function() table.insert(W.calls, "framework close") end,
    }
    Device.framework_lipc_handle = handle
    W.status.hosted = function()
        check("framework stopped: power service still open throughout Grimoire", at("framework close") == nil)
        return 0
    end
    W.calls = {}
    Device:exit()
    check("framework stopped: framework closed after hosted run",
        at("execute sh ") and at("framework close") and at("execute sh ") < at("framework close"))
    check("framework stopped: field restored", Device.framework_lipc_handle == handle)
    Device.framework_lipc_handle = nil
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

-- ---- Annotate this page in Grimoire ----

local function read_all(path)
    local f = io.open(path, "rb")
    if not f then return nil end
    local s = f:read("*a")
    f:close()
    return s
end
local function request() -- the request written, as key = value
    local text = read_all(W.annotate .. "/request.ini")
    if not text then return nil end
    local r = {}
    for k, v in text:gmatch("([%w_]+)=([^\n]*)") do r[k] = v end
    return r
end
local function annotate_files() -- what the requests' folder holds, sorted, joined by spaces
    local p = io.popen("ls -A " .. q(W.annotate) .. " 2>/dev/null")
    local names = {}
    for name in p:lines() do table.insert(names, name) end
    p:close()
    table.sort(names)
    return table.concat(names, " ")
end
local function drew(prefix) -- the first thing drawn that starts with prefix
    for _, d in ipairs(W.drawn) do
        if d:sub(1, #prefix) == prefix then return d end
    end
    return nil
end
-- Nothing the plugin logged names the book: neither its title nor its file.
local function logs_name_no_book(s)
    for _, line in ipairs(W.logs) do
        if line:find(W.book.title, 1, true) or line:find("book.epub", 1, true) or line:find("Secret", 1, true) then
            print("  logged: " .. line)
            check(s .. ": the log names no book", false)
            return
        end
    end
    check(s .. ": the log names no book", true)
end
-- The picture and the request each written into a temporary file, then renamed into place: no file
-- of the requests' folder ever written under its own name, so Grimoire never reads half of one.
local function written_whole(s)
    local in_place = {}
    for _, path in ipairs(W.wrote) do
        if path:sub(1, #W.annotate + 1) == W.annotate .. "/" and not path:find("%.tmp$") then
            table.insert(in_place, path)
        end
    end
    for _, path in ipairs(in_place) do print("  written in place: " .. path) end
    check(s .. ": nothing written in place", #in_place == 0)
    local function renamed(name)
        local want = W.annotate .. "/" .. name .. ".tmp -> " .. W.annotate .. "/" .. name
        for _, r in ipairs(W.renamed) do
            if r == want then return true end
        end
        return false
    end
    check(s .. ": the picture and the request renamed into place", renamed("page.png") and renamed("request.ini"))
    -- Each renamed only once written and closed: its temporary file opened and closed before the
    -- rename, and neither opened, written nor closed after it (else the name Grimoire reads would
    -- show half a file for a moment).
    local function in_order(name)
        local tmp = W.annotate .. "/" .. name .. ".tmp"
        local rename = "rename " .. tmp .. " -> " .. W.annotate .. "/" .. name
        local opened, closed, at = false, false, nil
        for i, e in ipairs(W.events) do
            if e == rename then
                at = i
                break
            end
            if e == "open " .. tmp then opened, closed = true, false end
            if e == "close " .. tmp then closed = opened end
        end
        if not (at and opened and closed) then return false end
        for i = at + 1, #W.events do
            local e = W.events[i]
            if e == "open " .. tmp or e == "write " .. tmp or e == "close " .. tmp then return false end
        end
        return true
    end
    check(s .. ": each written and closed before it is renamed into place", in_order("page.png") and in_order("request.ini"))
end
-- A page that could not be sent: the message says so, nothing is left in the requests' folder (no
-- picture, no request, nothing half-written), and there is no hand-over.
local function not_sent(s)
    check(s .. ": says so", told("Could not save this page for Grimoire. See koreader/crash.log."))
    check(s .. ": nothing left in the folder", annotate_files() == "")
    check(s .. ": no hand-over", not wrapped() and not is_file(W.marker) and not at("exitOrRestart"))
end
local ANNOTATE_CMD = " --hosted koreader --annotate"

do -- In a book: Tools > Grimoire has Annotate this page in Grimoire, second; the gesture too.
    world("annotate-menu")
    local g = koreader(true)
    local items = {}
    g:addToMainMenu(items)
    local sub = items.grimoire.sub_item_table
    check("annotate: in a PDF, seven entries, Annotate second, Open this book third", #sub == 7
        and sub[1].text == "Open Grimoire" and sub[2].text == "Annotate this page in Grimoire"
        and sub[3].text == "Open this book in Grimoire" and sub[4].text == "Notebook PDFs")
    check("annotate: the menu closes first (kept open by TouchMenu)", sub[2].keep_menu_open == true)
    local a = W.actions.grimoire_annotate
    check("annotate gesture: registered, in books only", a and a.event == "AnnotateInGrimoire"
        and a.title == "Annotate this page in Grimoire" and a.reader == true and not a.general)
    check("annotate: showing the menu runs nothing", calls_of("execute") == 0)
end

do -- From a PDF: the whole page fitted to the Scribe's page, the PNG and the request written whole,
    -- then the hand-over with --annotate; KOReader comes back to the book.
    world("annotate-pdf")
    assert(sh("mkdir -p " .. q(W.annotate)))
    write(W.annotate .. "/old.png", "an older request's picture")
    W.book.file = W.us .. "/documents/Secret Diary.pdf"
    local g = koreader(true)
    local items = {}
    g:addToMainMenu(items)
    local closed = false
    items.grimoire.sub_item_table[2].callback({ closeMenu = function() closed = true end })
    local s = "annotate a PDF"
    check(s .. ": the menu closed", closed)
    check(s .. ": unpacked first, Preparing Grimoire... up", at("execute sh '" .. W.doc .. "' --unpack-only") ~= nil
        and at("show Preparing Grimoire...") ~= nil)
    -- 612 x 792 points fitted to 1860 x 2480: the width decides (zoom 3.0392), centred down the page.
    check(s .. ": the whole page drawn, fitted and centred", drew("drawPage") == "drawPage 0 36 1860x2407 12 3.0392 0 1.5 1")
    check(s .. ": as it is: not reflowed, cleaned up or inverted", drew("settings") == "settings 0 0 0 255 0")
    local c = g.ui.document.configurable
    check(s .. ": the book's own settings back after", c.text_wrap == 1 and c.page_opt == 1 and c.auto_straighten == 5
        and c.white_threshold == 200 and c.nightmode_document == 1)
    check(s .. ": into a page-size picture", read_all(W.annotate .. "/page.png") == "PNG 1860x2480 page 12\n")
    check(s .. ": the folder emptied first, nothing half-written", annotate_files() == "page.png request.ini")
    written_whole(s)
    local r = request() or {}
    check(s .. ": the request names the book by its key", r.book == W.book.md5 and r.version == "1")
    check(s .. ": its title, page and page count", r.title == "Le Petit Prince" and r.page == "12" and r.pages == "96")
    check(s .. ": a paged document, its picture", r.kind == "paged" and r.picture == "page.png")
    check(s .. ": when it was written", tonumber(r.time) and math.abs(tonumber(r.time) - os.time()) <= 5)
    check(s .. ": the BlitBuffer freed", W.freed >= 1)
    check(s .. ": the hand-over waits for the next tick", wrapped() and is_file(W.marker) and at("exitOrRestart") ~= nil)
    check(s .. ": back to the book after", W.settings.start_with == "last")
    run_ticks()
    W.calls = {}
    Device:exit()
    check(s .. ": KOReader's own exit, then grimoire.sh --hosted koreader --annotate",
        at("exit") == 1 and at("execute sh '" .. W.app_sh .. "'" .. ANNOTATE_CMD) == 2)
    check(s .. ": the marker goes once Grimoire ran", not is_file(W.marker))
    logs_name_no_book(s)
    W.calls = {}
    g:onAnnotateInGrimoire()
    check(s .. ": tapped again, nothing more", #W.calls == 0)
    -- drawPage's last argument as the reader passes it: the saturation (KOReader 2024 on), else
    -- the render mode (before).
    world("annotate-pdf-saturation")
    W.book.saturation, W.book.render_mode = 0.8, 0
    koreader(true):onAnnotateInGrimoire()
    check(s .. ", a KOReader with saturation: passed on", drew("drawPage") == "drawPage 0 36 1860x2407 12 3.0392 0 1.5 0.8")
    world("annotate-pdf-render-mode")
    W.book.saturation, W.book.render_mode = nil, 0
    koreader(true):onAnnotateInGrimoire()
    check(s .. ", an older KOReader: its render mode passed on", drew("drawPage") == "drawPage 0 36 1860x2407 12 3.0392 0 1.5 0")
end

do -- From an EPUB: the text as the screen shows it (crengine's own status bar left out), the gesture.
    world("annotate-epub")
    W.book.kind = "reflowable"
    W.book.page, W.book.pages, W.book.header = 31, 340, 46
    local g = koreader(true)
    g:onAnnotateInGrimoire()
    local s = "annotate an EPUB"
    check(s .. ": the page as the screen shows it", drew("drawCurrentViewByPage") == "drawCurrentViewByPage 0 0 1860x2480 31")
    check(s .. ": crengine's status bar left white", drew("paintRect") == "paintRect 0 0 1860 46 255")
    check(s .. ": images as they are, not night mode's", drew("night images") == "night images false"
        and g.ui.document._nightmode_images == true)
    check(s .. ": no scaling (the screen is the page)", not drew("scale"))
    check(s .. ": the picture", read_all(W.annotate .. "/page.png") == "PNG 1860x2480 text 31\n")
    local r = request() or {}
    check(s .. ": a reflowable book, page 31 of 340", r.kind == "reflowable" and r.page == "31" and r.pages == "340")
    written_whole(s)
    run_ticks()
    W.calls = {}
    Device:exit()
    check(s .. ": the hand-over", at("execute sh '" .. W.app_sh .. "'" .. ANNOTATE_CMD) == 2)
    world("annotate-epub-scroll")
    W.book.kind, W.book.view_mode = "reflowable", "scroll"
    koreader(true):onAnnotateInGrimoire()
    check("annotate an EPUB in scroll mode: drawn where it is", drew("drawCurrentViewByPos") == "drawCurrentViewByPos 0 0 1860x2480 4321")
end

do -- Read sideways: the landscape text fitted into the upright page, centred.
    world("annotate-sideways")
    W.book.kind = "reflowable"
    W.screen = { w = 2480, h = 1860 }
    koreader(true):onAnnotateInGrimoire()
    local s = "annotate sideways"
    check(s .. ": drawn at the screen's shape", drew("drawCurrentViewByPage") == "drawCurrentViewByPage 0 0 2480x1860 12")
    check(s .. ": scaled to fit the upright page", drew("scale") == "scale 2480x1860 to 1860x1395")
    check(s .. ": centred on it", drew("blitFrom") == "blitFrom scaled text 12 0 542 0 0 1860 1395")
    check(s .. ": an upright picture", read_all(W.annotate .. "/page.png") == "PNG 1860x2480 scaled text 12\n")
end

do -- A KOReader that cannot draw a page by itself: a screenshot of the reader, the menu closed and
    -- "Preparing Grimoire..." down while it is taken, then up again for the hand-over.
    world("annotate-old")
    W.book.kind = "old"
    local g = koreader(true)
    g:onAnnotateInGrimoire()
    local s = "annotate in an older KOReader"
    local busy = W.shown[1]
    check(s .. ": a screenshot instead", at("shot") ~= nil and read_all(W.annotate .. "/page.png") == "SHOT 1860x2480\n")
    check(s .. ": Preparing Grimoire... taken down, the page drawn again, then the shot",
        W.closed[1] == busy and at("shot") > at("repaint"))
    check(s .. ": then up again for the hand-over", W.shown[#W.shown] == busy and #W.closed == 1)
    local r = request() or {}
    check(s .. ": the request all the same", r.page == "12" and r.picture == "page.png")
    check(s .. ": the hand-over goes ahead", wrapped() and at("exitOrRestart") ~= nil)
    written_whole(s)
    logs_name_no_book(s)
    -- Its Screen:shot answering false (a newer KOReader, its drawing failed): not sent.
    world("annotate-old-shot-fails")
    W.book.kind, W.png_answers = "old", "false"
    koreader(true):onAnnotateInGrimoire()
    not_sent(s .. ", the screenshot not written")
end

do -- No key in the book's settings: worked out from its file (KOReader's partial MD5).
    world("annotate-md5")
    W.book.md5 = nil
    W.md5_of_file = "a1b2c3d4e5f60718293a4b5c6d7e8f90"
    koreader(true):onAnnotateInGrimoire()
    check("annotate, no key kept: the file's partial MD5", at("partialMD5") ~= nil and (request() or {}).book == W.md5_of_file)
    world("annotate-no-key")
    W.book.md5 = nil
    koreader(true):onAnnotateInGrimoire()
    local s = "annotate, no key at all"
    check(s .. ": says so", told("Could not save this page for Grimoire. See koreader/crash.log."))
    check(s .. ": no request, no hand-over", not is_file(W.annotate .. "/request.ini") and not wrapped()
        and not is_file(W.marker))
end

do -- The picture cannot be written (a full disk): no request, no hand-over, the message closed.
    world("annotate-full")
    W.png_error = true
    koreader(true):onAnnotateInGrimoire()
    local s = "annotate, picture not written"
    check(s .. ": Preparing Grimoire... closed", #W.closed == 1 and W.closed[1].text == "Preparing Grimoire...")
    not_sent(s)
    -- writePNG answering false (a newer KOReader), a file cut short left behind: not sent either,
    -- and the log has lodepng's words for why.
    world("annotate-png-false")
    W.png_answers = "false"
    koreader(true):onAnnotateInGrimoire()
    s = "annotate, writePNG says it failed"
    not_sent(s)
    local why = false
    for _, line in ipairs(W.logs) do why = why or line:find("failed to write", 1, true) ~= nil end
    check(s .. ": the log says why", why)
    -- An older KOReader's writePNG answers nothing: a file there and not empty counts as written,
    -- an empty one does not.
    world("annotate-png-nothing")
    W.png_answers = "nothing"
    koreader(true):onAnnotateInGrimoire()
    check("annotate, writePNG answers nothing: sent", annotate_files() == "page.png request.ini" and wrapped())
    world("annotate-png-empty")
    W.png_answers = "empty"
    koreader(true):onAnnotateInGrimoire()
    not_sent("annotate, writePNG answers nothing and writes nothing")
    -- The request cannot be renamed into place (the disk full by then): its picture goes too.
    world("annotate-request-not-written")
    W.rename_fails = "/request.ini"
    koreader(true):onAnnotateInGrimoire()
    not_sent("annotate, request not written")
end

do -- An app/grimoire.sh that cannot annotate (hosted only): Annotate says so, Open still works.
    world("annotate-old-app")
    W.unpack_writes = "#!/bin/sh\n# Capabilities: guard hosted\n"
    local g = koreader(true)
    g:onAnnotateInGrimoire()
    local s = "annotate, app without annotate"
    check(s .. ": update it, says the message", told("This Grimoire cannot annotate pages: copy a newer Grimoire.sh into the Kindle's documents folder."))
    check(s .. ": nothing drawn or written", #W.drawn == 0 and annotate_files() == "")
    check(s .. ": no hand-over", not wrapped() and not is_file(W.marker))
    g:onOpenGrimoire()
    check(s .. ": Open still hands over", wrapped() and at("exitOrRestart") ~= nil)
    world("annotate-refused")
    os.remove(W.koreader_sh)
    koreader(true):onAnnotateInGrimoire()
    check("annotate, no koreader.sh: refused as Open is", told("Grimoire opens only from a KOReader started by koreader.sh (its Library item or KUAL).")
        and calls_of("execute") == 0 and #W.drawn == 0)
end

do -- grimoire.sh refused the hand-over (exit 3): the next start says so, and the page it sent goes
    -- (no Grimoire would open it now).
    world("annotate-not-run")
    W.status.hosted = 768
    local g = koreader(true)
    g:onAnnotateInGrimoire()
    run_ticks()
    Device:exit()
    local s = "annotate, Grimoire never ran"
    check(s .. ": the request was there", annotate_files() == "page.png request.ini" and is_file(W.marker))
    W.shown, W.scheduled = {}, {}
    koreader(true)
    check(s .. ": the next start removes it", annotate_files() == "" and not is_file(W.marker))
    check(s .. ": and says Grimoire could not start", #W.scheduled == 1)
end

do -- The gesture in the file browser: no book to annotate.
    world("annotate-no-book")
    koreader(false):onAnnotateInGrimoire()
    check("annotate without a book: says so", told("Open a book first: Grimoire annotates the page you are reading."))
    untouched("annotate without a book")
end

do -- A notebook's own PDF, in Grimoire's mirror (grimoire/PDFs): refused from the menu and by the
    -- gesture, however its path is written or reached, before anything is run, drawn or written. A
    -- book anywhere else is annotated, in a folder named like the mirror too.
    local OWN = "This PDF is a Grimoire notebook: open that notebook in Grimoire to write on it."
    local function book_at(name, path_of) -- a world whose book is the PDF at path_of()
        world(name)
        assert(sh("mkdir -p " .. q(W.pdfs)))
        local path = path_of()
        assert(sh("mkdir -p " .. q(path:match("^(.*)/"))))
        write(path, "%PDF-1.7\n")
        W.book.file = path
        return koreader(true)
    end
    local function refused(s)
        check(s .. ": says why", told(OWN))
        check(s .. ": nothing drawn or written", #W.drawn == 0 and annotate_files() == "")
        untouched(s)
        logs_name_no_book(s)
    end
    local function annotated(s)
        check(s .. ": annotated", not told(OWN) and annotate_files() == "page.png request.ini"
            and wrapped() and at("exitOrRestart") ~= nil)
    end
    local g = book_at("own-pdf-menu", function() return W.pdfs .. "/Secret Lecture.pdf" end)
    local items = {}
    g:addToMainMenu(items)
    items.grimoire.sub_item_table[2].callback({ closeMenu = function() end })
    refused("own PDF, from the menu")
    g = book_at("own-pdf-gesture", function() return W.pdfs .. "/Secret Lecture.pdf" end)
    g["on" .. W.actions.grimoire_annotate.event](g)
    refused("own PDF, by the gesture")
    book_at("own-pdf-folder", function() return W.pdfs .. "/Work/Secret Lecture.pdf" end):onAnnotateInGrimoire()
    refused("own PDF, in a folder of the mirror")
    -- Doubled slashes, "." and ".." in the book's path, a trailing slash on the plugin's.
    g = book_at("own-pdf-written-oddly", function() return W.us .. "//grimoire/./import/../PDFs/Secret Lecture.pdf" end)
    g.us = W.us .. "/"
    g:onAnnotateInGrimoire()
    refused("own PDF, its path written oddly")
    -- The scratch Kindle's path is relative (from the working folder): the book's made absolute.
    local here = first_line("pwd")
    book_at("own-pdf-absolute", function()
        return (W.pdfs:sub(1, 1) == "/" and "" or here .. "/") .. W.pdfs .. "/Secret Lecture.pdf"
    end):onAnnotateInGrimoire()
    refused("own PDF, by its absolute path")
    book_at("own-pdf-link", function()
        assert(sh("ln -s ../grimoire/PDFs " .. q(W.us .. "/documents/Notebooks")))
        return W.us .. "/documents/Notebooks/Secret Lecture.pdf"
    end):onAnnotateInGrimoire()
    refused("own PDF, through a link to the mirror")
    -- Letter case as the filesystem has it: grimoire/pdfs is the mirror where names ignore case,
    -- else another folder.
    local folds
    book_at("own-pdf-case", function()
        folds = sh("test -d " .. q(W.us .. "/grimoire/pdfs"))
        return W.us .. "/grimoire/pdfs/Secret Lecture.pdf"
    end):onAnnotateInGrimoire()
    if folds then refused("own PDF, by a name in other case on this filesystem")
    else annotated("a folder named pdfs beside the mirror, on a filesystem that keeps case") end
    -- Elsewhere: annotated.
    book_at("own-pdf-elsewhere", function() return W.us .. "/documents/Secret Lecture.pdf" end):onAnnotateInGrimoire()
    annotated("a PDF in documents")
    book_at("own-pdf-named-like", function() return W.us .. "/documents/grimoire/PDFs/Secret Lecture.pdf" end)
        :onAnnotateInGrimoire()
    annotated("a PDF in a grimoire/PDFs folder outside Grimoire's")
    book_at("own-pdf-beside", function() return W.dir .. "/grimoire/PDFs/Secret Lecture.pdf" end):onAnnotateInGrimoire()
    annotated("a PDF in a grimoire/PDFs folder beside the Kindle's")
    book_at("own-pdf-prefix", function() return W.us .. "/grimoire/PDFs old/Secret Lecture.pdf" end):onAnnotateInGrimoire()
    annotated("a PDF in a folder whose name starts like the mirror's")
    book_at("own-pdf-file-named-like", function() return W.us .. "/grimoire/PDFs.pdf" end):onAnnotateInGrimoire()
    annotated("a PDF named like the mirror")
    -- In Spanish.
    book_at("own-pdf-es", function()
        W.settings.language = "es"
        return W.pdfs .. "/Secret Lecture.pdf"
    end):onAnnotateInGrimoire()
    check("own PDF in Spanish: says why", told("Este PDF es un cuaderno de Grimoire: abre ese cuaderno en Grimoire para escribir en él."))
    untouched("own PDF in Spanish")
end

do -- In Spanish.
    world("annotate-es")
    W.settings.language = "es"
    local g = koreader(true)
    local items = {}
    g:addToMainMenu(items)
    check("annotate in Spanish: the menu", items.grimoire.sub_item_table[2].text == "Anotar esta página en Grimoire")
    check("annotate in Spanish: the gesture", W.actions.grimoire_annotate.title == "Anotar esta página en Grimoire")
    world("annotate-es-no-book")
    W.settings.language = "es"
    koreader(false):onAnnotateInGrimoire()
    check("annotate in Spanish: the message", told("Primero abre un libro: Grimoire anota la página que estás leyendo."))
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

do -- A custom wallpaper is exported without ever showing KOReader's sleep screen.
    world("wallpaper")
    local g = koreader(false)
    W.settings.screensaver_type = "random_image"
    local cleaned, painted = false, false
    package.loaded["ui/screensaver"] = {
        setup = function(self)
            self.image_file = "wallpaper.png"
            self.screensaver_background = "white"
        end,
        cleanup = function() cleaned = true end,
    }
    package.loaded["ui/widget/imagewidget"] = { new = function(_, args)
        check("wallpaper: rendered at the screen size", args.width == 1860 and args.height == 2480)
        return {
            paintTo = function(_, bb) painted = true; bb.what = "wallpaper" end,
            free = function() end,
        }
    end }
    g:sendWallpaper()
    local path = W.us .. "/grimoire/koreader-sleep.png"
    check("wallpaper: exported whole", is_file(path) and not is_file(path .. ".tmp"))
    check("wallpaper: widget rendered and resources cleaned", painted and cleaned)
    check("wallpaper: no visible sleep UI or restart", #W.shown == 0 and at("restart") == nil)
    W.settings.screensaver_type = "disable"
    g:sendWallpaper()
    check("wallpaper: disabling it removes the old snapshot", not is_file(path))
    W.settings.screensaver_type = "random_image"
    W.png_answers = "false"
    g:sendWallpaper()
    check("wallpaper: failed write leaves no stale or partial snapshot", not is_file(path) and not is_file(path .. ".tmp"))
    package.loaded["ui/screensaver"], package.loaded["ui/widget/imagewidget"] = nil, nil
end

-- ---- Open this book in Grimoire ----

local BOOK_CMD = " --hosted koreader --book"
local BOOK_CAPS = "#!/bin/sh\n# Capabilities: guard hosted annotate book\n"
local function book_dir() return W.us .. "/grimoire/import/.book" end
local function book_files() -- what the book's folder holds, sorted, joined by spaces
    local p = io.popen("ls -A " .. q(book_dir()) .. " 2>/dev/null")
    local names = {}
    for name in p:lines() do table.insert(names, name) end
    p:close()
    table.sort(names)
    return table.concat(names, " ")
end
local function book_request() -- the book's request, as key = value, with its pictures as a list
    local text = read_all(book_dir() .. "/request.ini")
    if not text then return nil end
    local r = { pictures = {} }
    for k, v in text:gmatch("([%w%.]+)=([^\n]*)") do
        if k:sub(1, 8) == "picture." then table.insert(r.pictures, k:sub(9) .. "=" .. v) else r[k] = v end
    end
    return r
end
-- The book's notebook in Grimoire, holding book pages `held` of `pages` (source.ini only: all the
-- plugin reads).
local function notebook_of(key, pages, held)
    local dir = W.us .. "/grimoire/notebooks/20261026-100000-abcd"
    assert(sh("mkdir -p " .. q(dir)))
    local lines = { "# Grimoire: the book this notebook annotates, from KOReader (docs/FORMAT.md)", "book=" .. key, "kind=paged" }
    for i, p in ipairs(held) do table.insert(lines, string.format("page.%d.%d=%d", p, pages, i)) end
    write(dir .. "/source.ini", table.concat(lines, "\n") .. "\n")
    -- Another book's notebook, which must not count.
    local other = W.us .. "/grimoire/notebooks/20261026-100001-ef01"
    assert(sh("mkdir -p " .. q(other)))
    write(other .. "/source.ini", "book=ffffffffffffffffffffffffffffffff\nkind=paged\npage.1." .. pages .. "=1\n")
end

do -- The menu: in a PDF, Open this book in Grimoire (above); in an EPUB not (its pages change with the font).
    world("book-menu")
    W.book.kind = "reflowable"
    local g = koreader(true)
    local items = {}
    g:addToMainMenu(items)
    local sub = items.grimoire.sub_item_table
    check("book: not in an EPUB's menu", #sub == 6 and sub[2].text == "Annotate this page in Grimoire"
        and sub[3].text == "Notebook PDFs")
    local a = W.actions.grimoire_book
    check("book gesture: registered, in books only", a and a.event == "OpenBookInGrimoire"
        and a.title == "Open this book in Grimoire" and a.reader == true and not a.general)
    check("book: Open with... has Grimoire, never a default", W.aux and W.aux.provider == "grimoire"
        and W.aux.provider_name == "Grimoire" and W.aux.disable_file and W.aux.disable_type)
end

do -- From the reader, in a 3-page PDF on page 2 whose notebook holds page 2 already: pages 1 and 3
    -- drawn whole, each written whole, under a progress message; then the request and --book.
    world("book-reader")
    W.unpack_writes = BOOK_CAPS
    W.book.pages, W.book.page, W.book.whole = 3, 2, true
    W.book.file = W.us .. "/documents/Hanon.pdf"
    notebook_of(W.book.md5, 3, { 2 })
    assert(sh("mkdir -p " .. q(book_dir())))
    write(book_dir() .. "/page-9.png", "an older request's picture")
    local g = koreader(true)
    local items = {}
    g:addToMainMenu(items)
    local closed = false
    items.grimoire.sub_item_table[3].callback({ closeMenu = function() closed = true end })
    local s = "book from the reader"
    check(s .. ": the menu closed", closed)
    check(s .. ": progress for the 2 pages to draw", at("trapper Preparing page 1 of 2 for Grimoire... | Tap to stop.")
        and at("trapper Preparing page 2 of 2 for Grimoire... | Tap to stop.") and calls_of("trapper Preparing") == 2)
    check(s .. ": pages 1 and 3 drawn whole, not page 2", drew("drawPage 0 36 1860x2407 1 ") and drew("drawPage 0 36 1860x2407 3 ")
        and not drew("drawPage 0 36 1860x2407 2 "))
    check(s .. ": into page-size pictures", read_all(book_dir() .. "/page-1.png") == "PNG 1860x2480 page 1\n"
        and read_all(book_dir() .. "/page-3.png") == "PNG 1860x2480 page 3\n")
    check(s .. ": the folder emptied first, nothing half-written", book_files() == "page-1.png page-3.png request.ini")
    local c = g.ui.document.configurable
    check(s .. ": the book's own settings back after", c.text_wrap == 1 and c.page_opt == 1 and c.white_threshold == 200)
    local r = book_request() or {}
    check(s .. ": the request names the book, its pages and the page to open", r.version == "1" and r.book == W.book.md5
        and r.pages == "3" and r.page == "2" and r.kind == "paged" and r.title == "Le Petit Prince")
    check(s .. ": its pictures", table.concat(r.pictures, " ") == "1=page-1.png 3=page-3.png")
    check(s .. ": when it was written", tonumber(r.time) and math.abs(tonumber(r.time) - os.time()) <= 5)
    check(s .. ": the BlitBuffers freed", W.freed >= 2)
    check(s .. ": Preparing Grimoire... up, the hand-over on the next tick", at("show Preparing Grimoire...")
        and wrapped() and is_file(W.marker))
    run_ticks()
    W.calls = {}
    Device:exit()
    check(s .. ": KOReader's own exit, then grimoire.sh --hosted koreader --book",
        at("exit") == 1 and at("execute sh '" .. W.app_sh .. "'" .. BOOK_CMD) == 2)
    logs_name_no_book(s)
end

do -- Every page held already: nothing to draw, the request names none, Grimoire opens the page.
    world("book-all-held")
    W.unpack_writes = BOOK_CAPS
    W.book.pages, W.book.page, W.book.whole = 2, 1, true
    notebook_of(W.book.md5, 2, { 1, 2 })
    koreader(true):onOpenBookInGrimoire()
    local s = "book all held"
    check(s .. ": nothing drawn", calls_of("trapper Preparing") == 0 and not drew("drawPage"))
    local r = book_request() or {}
    check(s .. ": a request with no pictures, the page to open", r.page == "1" and #r.pictures == 0)
    check(s .. ": the hand-over", wrapped())
end

do -- A tap on the progress message stops it: nothing left, nothing sent.
    world("book-stopped")
    W.unpack_writes = BOOK_CAPS
    W.book.pages, W.book.page, W.book.whole = 5, 1, true
    W.stop_at = 3
    koreader(true):onOpenBookInGrimoire()
    local s = "book stopped"
    check(s .. ": says so", told("Stopped: nothing was sent to Grimoire."))
    check(s .. ": nothing left in the folder", book_files() == "")
    check(s .. ": no hand-over", not wrapped() and not is_file(W.marker) and not at("exitOrRestart"))
end

do -- A picture that cannot be written (the disk full): says so, nothing left, no hand-over.
    world("book-disk-full")
    W.unpack_writes = BOOK_CAPS
    W.book.pages, W.book.page, W.book.whole = 3, 1, true
    W.png_answers = "false"
    koreader(true):onOpenBookInGrimoire()
    local s = "book, the disk full"
    check(s .. ": says so", told("Could not save the book's pages for Grimoire. Is the storage full? See koreader/crash.log."))
    check(s .. ": nothing left in the folder", book_files() == "")
    check(s .. ": no hand-over", not wrapped() and not at("exitOrRestart"))
end

do -- A Grimoire that cannot open whole books (its launcher lacks "book"): says so, draws nothing.
    world("book-too-old")
    W.book.pages, W.book.page, W.book.whole = 3, 1, true
    koreader(true):onOpenBookInGrimoire()
    local s = "book, an older Grimoire"
    check(s .. ": says so", told("This Grimoire cannot open whole books: copy a newer Grimoire.sh into the Kindle's documents folder."))
    check(s .. ": nothing drawn or sent", not drew("drawPage") and book_files() == "" and not wrapped())
end

do -- More pages than a notebook holds: refused before anything is drawn.
    world("book-too-big")
    W.unpack_writes = BOOK_CAPS
    W.book.pages, W.book.page, W.book.whole = 2001, 1, true
    koreader(true):onOpenBookInGrimoire()
    check("book of 2001 pages: refused", told("This book has more pages than a Grimoire notebook holds (2000).")
        and not drew("drawPage") and not wrapped())
end

do -- The file browser's Open with... > Grimoire: the files it takes, then a PDF opened for its pages,
    -- at the page it was left at, closed again, and the hand-over.
    world("book-open-with")
    W.unpack_writes = BOOK_CAPS
    local file = W.us .. "/documents/Czerny Op 299.pdf"
    W.files = { [file] = { file = file, pages = 2, md5 = "0123456789abcdef0123456789abcdef", last_page = 2,
        title = "School of Velocity", native = { w = 612, h = 792 } } }
    local g = koreader(false)
    local s = "open with"
    check(s .. ": PDF, DjVu and comics, in any case", g:isFileTypeSupported(file) and g:isFileTypeSupported("/x/a.DJVU")
        and g:isFileTypeSupported("/x/b.cbz"))
    check(s .. ": not an EPUB, nor a notebook's own PDF", not g:isFileTypeSupported("/x/c.epub")
        and not g:isFileTypeSupported(W.pdfs .. "/Notebook.pdf"))
    g:openFile(file)
    local last_preparing -- (it shows while the app unpacks too, before anything is drawn)
    for i, c in ipairs(W.calls) do
        if c == "show Preparing Grimoire..." then last_preparing = i end
    end
    check(s .. ": opened, both pages drawn, closed before the hand-over", at("openDocument")
        and drew("drawPage 0 36 1860x2407 1 ") and drew("drawPage 0 36 1860x2407 2 ") and at("closeDocument")
        and last_preparing and at("closeDocument") < last_preparing)
    check(s .. ": at KOReader's contrast and saturation, a book never set (1, 1)",
        drew("drawPage 0 36 1860x2407 1 ") == "drawPage 0 36 1860x2407 1 3.0392 0 1 1")
    local r = book_request() or {}
    check(s .. ": its key and the page it was left at", r.book == "0123456789abcdef0123456789abcdef" and r.page == "2"
        and r.pages == "2" and r.title == "School of Velocity" and table.concat(r.pictures, " ") == "1=page-1.png 2=page-2.png")
    check(s .. ": the hand-over from the file browser, back to it after", wrapped() and at("restart")
        and W.settings.start_with ~= "last")
    logs_name_no_book(s)
end

do -- Open with draws the pages as the reader would: the book's own contrast and saturation, else the
    -- defaults set for books, else KOReader's (defaults.lua); an older KOReader's render mode.
    local file
    local function open_with(name, settings, set_up)
        world(name)
        W.unpack_writes = BOOK_CAPS
        file = W.us .. "/documents/Czerny Op 299.pdf"
        W.files = { [file] = { file = file, pages = 1, md5 = "0123456789abcdef0123456789abcdef",
            native = { w = 612, h = 792 }, settings = settings } }
        if set_up then set_up() end
        koreader(false):openFile(file)
        return drew("drawPage")
    end
    local s = "open with, the reader's look"
    check(s .. ": the book's own", open_with("book-open-with-own", { kopt_contrast = 1.4, kopt_saturation = 0.6 })
        == "drawPage 0 36 1860x2407 1 3.0392 0 1.4 0.6" and wrapped())
    check(s .. ": the defaults set for books", open_with("book-open-with-defaults", nil, function()
        W.settings.kopt_contrast, W.settings.kopt_saturation = 2, 0.5
    end) == "drawPage 0 36 1860x2407 1 3.0392 0 2 0.5")
    check(s .. ": KOReader's defaults.lua", open_with("book-open-with-defaults-lua", nil, function()
        W.defaults = { DKOPTREADER_CONFIG_CONTRAST = 1.2, DKOPTREADER_CONFIG_SATURATION = 0.8 }
    end) == "drawPage 0 36 1860x2407 1 3.0392 0 1.2 0.8")
    check(s .. ": an older KOReader, the book's render mode", open_with("book-open-with-old", { render_mode = 2 },
        function() W.old_view = true end) == "drawPage 0 36 1860x2407 1 3.0392 0 1 2")
    check(s .. ": an older KOReader, its default render mode", open_with("book-open-with-old-default", nil,
        function() W.old_view = true end) == "drawPage 0 36 1860x2407 1 3.0392 0 1 0")
end

do -- A PDF that needs a password: KOReader cannot draw it here; closed again, nothing sent.
    world("book-open-with-locked")
    W.unpack_writes = BOOK_CAPS
    local file = W.us .. "/documents/Locked.pdf"
    W.files = { [file] = { file = file, pages = 2, md5 = "0123456789abcdef0123456789abcdef", locked = true,
        native = { w = 612, h = 792 } } }
    koreader(false):openFile(file)
    local s = "open with, a locked PDF"
    check(s .. ": says so", told("KOReader could not open this book for Grimoire. See koreader/crash.log."))
    check(s .. ": closed again, nothing drawn or sent", at("closeDocument") and not drew("drawPage") and not wrapped())
end

do -- The plugin uses KOReader's backend once at startup, without replacing network preferences.
    world("wifi-start")
    W.wifi_on,W.wifi_connected=false,false
    local g=koreader(false)
    check("Wi-Fi: deferred until the UI loop",W.wifi_restores==0)
    run_ticks()
    check("Wi-Fi: startup enables and checks the saved connection",W.wifi_on and W.wifi_restores==1 and W.wifi_checks==1)
    g:new{ui=new_ui(true)};run_ticks()
    check("Wi-Fi: only once per KOReader process",W.wifi_restores==1)
    local items={};g:addToMainMenu(items);local controls=items.grimoire.sub_item_table
    controls[4].callback()
    check("Wi-Fi: manual off uses KOReader",not W.wifi_on)
    g:new{ui=new_ui(false)};run_ticks()
    check("Wi-Fi: manual off is not undone in the same process",not W.wifi_on and W.wifi_restores==1)
    controls[4].callback()
    check("Wi-Fi: manual on uses KOReader",W.wifi_on)
    check("Wi-Fi: startup default enabled",controls[5].checked_func())
    controls[5].callback()
    check("Wi-Fi: startup option can be disabled",W.settings.grimoire_wifi_on_start==false)
    world("wifi-opt-out");W.settings.grimoire_wifi_on_start=false
    koreader(false);run_ticks()
    check("Wi-Fi: startup opt-out preserves the radio",W.wifi_restores==0)
    world("wifi-pending");Network.pending_connection=true
    koreader(false);run_ticks()
    check("Wi-Fi: existing connection attempt is not duplicated",W.wifi_restores==0)
    world("wifi-unsupported");W.wifi_supported=false
    koreader(false);run_ticks()
    check("Wi-Fi: unsupported device is left alone",W.wifi_restores==0)
    world("wifi-error");W.wifi_error=true
    koreader(false)
    check("Wi-Fi: backend failure cannot escape the UI loop",pcall(run_ticks) and at("warn")~=nil)
    world("wifi-disabled-before-tick")
    koreader(false);W.settings.grimoire_wifi_on_start=false;run_ticks()
    check("Wi-Fi: opt-out before scheduled task is honored",W.wifi_restores==0)
end

print(string.format("koplugin test: %d of %d checks failed", fails, count))
os.exit(fails == 0 and 0 or 1)
