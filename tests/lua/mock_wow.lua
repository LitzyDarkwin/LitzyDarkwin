-- mock_wow.lua — minimal WoW API mock for headless addon tests (Lua 5.1).
local MOCK = {
  time = 1700000000,
  mapID = 2346,
  x = 0.42, -- 0..1 scale (C_Map native)
  y = 0.37,
  zone = "Test Zone",
  player = "Testtoon",
  realm = "Testrealm",
  log = {}, -- captured chat lines
}
_G.MOCK = MOCK

function MOCK.advance(secs)
  MOCK.time = MOCK.time + (secs or 0)
end

function MOCK.lootLink(itemID, name)
  return string.format("|cffffffff|Hitem:%d:::::::::|h[%s]|h|r", itemID, name)
end

function MOCK.reset()
  MOCK.time = 1700000000
  MOCK.log = {}
  _G.TomTom = nil
  _G.GatherMate2 = nil
  _G.GatherMate = nil
  _G.Routes = nil
end

-- Time / player ---------------------------------------------------------------
_G.GetServerTime = function() return MOCK.time end
_G.UnitName = function() return MOCK.player end
_G.GetRealmName = function() return MOCK.realm end
_G.GetZoneText = function() return MOCK.zone end

_G.C_Map = {
  GetBestMapForUnit = function() return MOCK.mapID end,
  GetPlayerMapPosition = function(mapID)
    return {
      GetXY = function() return MOCK.x, MOCK.y end,
    }
  end,
}

-- Chat -------------------------------------------------------------------------
_G.DEFAULT_CHAT_FRAME = {
  AddMessage = function(_, msg) table.insert(MOCK.log, tostring(msg)) end,
}
_G.SlashCmdList = {}

-- Frames ------------------------------------------------------------------------
local function newFontString()
  local fs = { _text = "" }
  function fs:SetText(t) self._text = tostring(t) end
  function fs:GetText() return self._text end
  function fs:SetPoint() end
  function fs:SetJustifyH() end
  function fs:SetJustifyV() end
  return fs
end

local function newFrame(ftype, name, parent, template)
  local f = {
    _type = ftype, _name = name, _shown = false,
    _scripts = {}, _events = {}, _children = {},
    _text = "",
  }
  function f:RegisterEvent(e) self._events[e] = true end
  function f:UnregisterEvent(e) self._events[e] = nil end
  function f:SetScript(which, fn) self._scripts[which] = fn end
  function f:GetScript(which) return self._scripts[which] end
  function f:Show() self._shown = true end
  function f:Hide() self._shown = false end
  function f:IsShown() return self._shown end
  function f:SetSize() end
  function f:SetPoint() end
  function f:SetMovable() end
  function f:EnableMouse() end
  function f:RegisterForDrag() end
  function f:StartMoving() end
  function f:StopMovingOrSizing() end
  function f:SetBackdrop() end
  function f:SetText(t) self._text = tostring(t) end
  function f:GetText() return self._text end
  function f:Click()
    local fn = self._scripts["OnClick"]
    if fn then fn(self) end
  end
  function f:CreateFontString()
    local fs = newFontString()
    table.insert(self._children, fs)
    return fs
  end
  return f
end

_G.CreateFrame = function(ftype, name, parent, template)
  return newFrame(ftype, name, parent, template)
end
_G.UIParent = newFrame("Frame", "UIParent", nil, nil)

return MOCK
