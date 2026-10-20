-- Grimoire for KOReader: its name and description in KOReader's plugin list (main.lua does the
-- work and says how).
--
-- Copyright (C) 2026 the Grimoire authors (https://github.com/ganten1998/grimoire-scribe)
-- SPDX-License-Identifier: GPL-3.0-or-later
-- This program is free software: you can redistribute it and/or modify it under the terms of the
-- GNU General Public License as published by the Free Software Foundation, either version 3 of
-- the License, or (at your option) any later version. It is distributed WITHOUT ANY WARRANTY;
-- see the GNU General Public License for details: <https://www.gnu.org/licenses/>.

local description = {
    en = "Opens Grimoire, the handwriting notebook, from KOReader. Leaving Grimoire comes back here.",
    es = "Abre Grimoire, el cuaderno de escritura a mano, desde KOReader. Al salir de Grimoire, vuelves aquí.",
}
local lang = G_reader_settings and G_reader_settings:readSetting("language")
return {
    fullname = "Grimoire",
    description = description[type(lang) == "string" and lang:sub(1, 2) or "en"] or description.en,
}
