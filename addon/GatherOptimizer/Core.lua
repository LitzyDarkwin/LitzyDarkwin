-- GatherOptimizer/Core.lua — addon namespace, bootstrap, event bus, slash commands.
-- Lua 5.1 (WoW Retail dialect). No external libraries.
local ADDON_NAME, GO = ...
if GO == nil then
  -- Unit-test harness (tests/lua): files are dofile()'d without varargs.
  GO = _G.__GO_TEST_NS or {}
  _G.__GO_TEST_NS = GO
  ADDON_NAME = ADDON_NAME or "GatherOptimizer"
end

GO.ADDON_NAME = ADDON_NAME
GO.VERSION = "0.1.0"
GO.SCHEMA_VERSION = 1

-- Module registry: filled by each file (GO.Store, GO.Session, ...).
GO.modules = GO.modules or {}

-- Central event frame. Modules subscribe via GO.On(event, handler).
do
  local frame = _G.CreateFrame and _G.CreateFrame("Frame", "GatherOptimizerEvents") or nil
  local handlers = {}

  function GO.On(event, handler)
    handlers[event] = handlers[event] or {}
    table.insert(handlers[event], handler)
    if frame then frame:RegisterEvent(event) end
  end

  function GO.Fire(event, ...)
    local list = handlers[event]
    if list then
      for _, fn in ipairs(list) do
        local ok, err = pcall(fn, event, ...)
        if not ok then GO.Log("handler error for " .. tostring(event) .. ": " .. tostring(err)) end
      end
    end
  end

  if frame then
    frame:SetScript("OnEvent", function(_, event, ...)
      GO.Fire(event, ...)
    end)
  end
  GO._eventFrame = frame
end

--- Print helper (chat frame in game, stdout in the harness).
function GO.Log(msg)
  local line = "|cff7fd4ff[GatherOptimizer]|r " .. tostring(msg)
  if _G.DEFAULT_CHAT_FRAME and _G.DEFAULT_CHAT_FRAME.AddMessage then
    _G.DEFAULT_CHAT_FRAME:AddMessage(line)
  elseif _G.print then
    _G.print(line)
  end
end

--- Current Unix time. GetServerTime() in game; harness-controlled in tests.
function GO.Now()
  if _G.GetServerTime then return _G.GetServerTime() end
  return os.time()
end

function GO.PlayerInfo()
  local name = (_G.UnitName and _G.UnitName("player")) or "Unknown"
  local realm = (_G.GetRealmName and _G.GetRealmName()) or "Unknown"
  return name, realm
end

--- Player map context: mapID, x (0-100), y (0-100), zone name. Nils when unavailable.
function GO.PlayerMapPosition()
  local mapID, x, y, zone
  if _G.C_Map and _G.C_Map.GetBestMapForUnit then
    local ok, id = pcall(_G.C_Map.GetBestMapForUnit, "player")
    if ok then mapID = id end
  end
  if mapID and _G.C_Map and _G.C_Map.GetPlayerMapPosition then
    local ok, pos = pcall(_G.C_Map.GetPlayerMapPosition, mapID, "player")
    if ok and pos and pos.GetXY then
      local okxy, px, py = pcall(pos.GetXY, pos)
      if okxy and type(px) == "number" and type(py) == "number" then
        x, y = px * 100, py * 100
      end
    end
  end
  if _G.GetZoneText then
    local ok, z = pcall(_G.GetZoneText)
    if ok then zone = z end
  end
  return mapID, x, y, zone
end

-- Slash commands: /go, /gopt
local function HandleSlash(input)
  input = (input or ""):gsub("^%s+", ""):gsub("%s+$", "")
  local cmd, rest = input:match("^(%S+)%s*(.*)$")
  cmd = (cmd or "status"):lower()
  if cmd == "status" then
    GO.SessionCmd("status")
  elseif cmd == "ui" then
    if GO.UI then GO.UI.Toggle() else GO.Log("UI module not loaded.") end
  elseif cmd == "start" then
    GO.SessionCmd("start", rest)
  elseif cmd == "stop" then
    GO.SessionCmd("stop")
  elseif cmd == "pause" then
    GO.SessionCmd("pause")
  elseif cmd == "resume" then
    GO.SessionCmd("resume")
  elseif cmd == "import" then
    if GO.Bridge then GO.Bridge.ImportRouteString(rest) else GO.Log("Bridge not loaded.") end
  elseif cmd == "export" then
    if GO.Bridge then GO.Bridge.PrintExportHelp() else GO.Log("Bridge not loaded.") end
  elseif cmd == "nav" then
    if GO.Navigation then GO.Navigation.Slash(rest) else GO.Log("Navigation not loaded.") end
  else
    GO.Log("Commands: status | ui | start [note] | stop | pause | resume | import <code> | export | nav <start|stop|next>")
  end
end

if _G.SlashCmdList then
  _G.SLASH_GATHEROPTIMIZER1 = "/go"
  _G.SLASH_GATHEROPTIMIZER2 = "/gopt"
  _G.SlashCmdList["GATHEROPTIMIZER"] = HandleSlash
end
GO.HandleSlash = HandleSlash -- exposed for the test harness

--- Session command dispatcher (Session.lua registers the real handler).
function GO.SessionCmd(action, arg)
  if GO._sessionCmd then
    GO._sessionCmd(action, arg)
  else
    GO.Log("Session module not loaded.")
  end
end

function GO.OnAddonLoaded(name)
  if name ~= ADDON_NAME then return end
  if GO.Store then GO.Store.Init() end
  if GO.Session then GO.Session.Init() end
  if GO.Bridge then GO.Bridge.Init() end
  if GO.UI then GO.UI.Init() end
  local nav = (GO.Navigation and GO.Navigation.Status) and GO.Navigation.Status() or "navigation n/a"
  GO.Log("v" .. GO.VERSION .. " loaded. " .. tostring(nav))
end

GO.On("ADDON_LOADED", function(_, name) GO.OnAddonLoaded(name) end)

-- Expose namespace globally for adapters/debugging (single global by design).
_G.GatherOptimizer = GO
