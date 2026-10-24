-- Square corners on the folder tiles of the file browser's mosaic view (the cover browser plugin
-- draws them with rounded corners; book covers are square already). Not part of KOReader: delete
-- this file from koreader/patches to have the rounded tiles back.
local userpatch = require("userpatch")
local logger = require("logger")

userpatch.registerPatchPluginFunc("coverbrowser", function()
    local ok, err = pcall(function()
        local MosaicMenu = require("mosaicmenu")
        local MosaicMenuItem = userpatch.getUpValue(MosaicMenu._updateItemsBuildUI, "MosaicMenuItem")
        if not MosaicMenuItem or MosaicMenuItem._square_corners then return end
        MosaicMenuItem._square_corners = true
        local FrameContainer = require("ui/widget/container/framecontainer")
        local update = MosaicMenuItem.update
        -- While a tile is built, a frame gets no corner radius.
        function MosaicMenuItem:update(...)
            local own = rawget(FrameContainer, "new")
            local new = FrameContainer.new
            FrameContainer.new = function(cls, o)
                if type(o) == "table" then o.radius = nil end
                return new(cls, o)
            end
            local done, e = pcall(update, self, ...)
            FrameContainer.new = own
            if not done then error(e, 0) end
        end
        logger.warn("mosaic square corners patch: on")
    end)
    if not ok then logger.warn("mosaic square corners patch: not applied:", err) end
end)
