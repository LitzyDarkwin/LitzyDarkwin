-- GatherOptimizer/Bridge.lua — desktop synchronization boundary.
-- OUT (addon -> desktop): versioned SavedVariables snapshot. The desktop app reads
--   the SV file; WoW only flushes it on logout/reload, so freshness is explicit.
-- IN (desktop -> addon): short import string pasted via /go import <code>.
--
-- Import codec v1 ("GO1"):
--   GO1.<mapID>.<n>.<coords>.<check>
--   coords = n x 6 decimal digits, per waypoint XXXYYY where XXX=floor(x*10)
--            clamped to 000..999 (0.1 precision on the 0-100 map scale).
--   check  = weighted digit sum mod 997, zero-padded to 3 digits.
-- Example: GO1.2346.2.042137500500.123
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local Bridge = {}
GO.Bridge = Bridge
GO.modules["Bridge"] = Bridge

Bridge.EXPORT_VERSION = 1
Bridge.CODEC_PREFIX = "GO1"

--- Build the export snapshot (also stored at DB.export.payloadMeta for the desktop
-- to validate without scanning whole tables). Cheap: counts + newest timestamps.
function Bridge.BuildExport()
  local db = GO.Store.DB()
  local newestSession, newestObs = 0, 0
  for _, s in ipairs(db.sessions) do
    newestSession = math.max(newestSession, s.endedAt or s.startedAt or 0)
  end
  for _, o in ipairs(db.observations) do
    newestObs = math.max(newestObs, o.t or 0)
  end
  local player, realm = GO.PlayerInfo()
  local export = {
    version = Bridge.EXPORT_VERSION,
    schemaVersion = GO.SCHEMA_VERSION,
    exportedAt = GO.Now(),
    character = player,
    realm = realm,
    sessionCount = #db.sessions,
    observationCount = #db.observations,
    newestSessionAt = newestSession,
    newestObservationAt = newestObs,
    hasActiveRoute = db.activeRoute ~= nil,
  }
  db.export = export
  return export
end

function Bridge.Init()
  -- Re-export marker on login so a fresh SV flush always carries valid metadata.
  -- Registered once: Init re-runs on every ADDON_LOADED.
  if not Bridge._loginHooked then
    Bridge._loginHooked = true
    GO.On("PLAYER_LOGIN", function() Bridge.BuildExport() end)
  end
end

function Bridge.PrintExportHelp()
  local e = Bridge.BuildExport()
  GO.Log(string.format(
    "Export v%d ready: %d session(s), %d observation(s). " ..
    "Log out or type /reload, then press Sync in the desktop app.",
    e.version, e.sessionCount, e.observationCount))
end

-- Import codec ---------------------------------------------------------------

local function checksum(mapID, n, coords)
  local s = tostring(mapID) .. "." .. tostring(n) .. "." .. coords
  local acc = 0
  for i = 1, #s do
    local b = s:byte(i)
    if b >= 48 and b <= 57 then -- digits only
      acc = (acc + (b - 48) * i) % 997
    end
  end
  return acc
end

--- Encode waypoints into a GO1 string. Returns code or nil, err.
function Bridge.EncodeRoute(mapID, waypoints, maxPoints)
  maxPoints = maxPoints or 40
  if type(mapID) ~= "number" then return nil, "mapID must be a number" end
  if type(waypoints) ~= "table" or #waypoints < 2 then
    return nil, "need at least 2 waypoints"
  end
  local parts = {}
  local count = 0
  for _, w in ipairs(waypoints) do
    if count >= maxPoints then break end
    if type(w.x) == "number" and type(w.y) == "number" then
      local ix = math.max(0, math.min(999, math.floor(w.x * 10 + 0.5)))
      local iy = math.max(0, math.min(999, math.floor(w.y * 10 + 0.5)))
      table.insert(parts, string.format("%03d%03d", ix, iy))
      count = count + 1
    end
  end
  if count < 2 then return nil, "need at least 2 valid waypoints" end
  local coords = table.concat(parts)
  local check = checksum(mapID, count, coords)
  return string.format("%s.%d.%d.%s.%03d", Bridge.CODEC_PREFIX, mapID, count, coords, check)
end

--- Decode a GO1 string. Returns { mapID, waypoints={{x,y}...} } or nil, err.
function Bridge.DecodeRouteString(code)
  if type(code) ~= "string" then return nil, "empty code" end
  code = code:gsub("%s+", "")
  local prefix, mapID, n, coords, check =
    code:match("^([A-Z]+%d+)%.(%d+)%.(%d+)%.([0-9]+)%.(%d%d%d)$")
  if not prefix then return nil, "unrecognized format (want GO1.<map>.<n>.<coords>.<check>)" end
  if prefix ~= Bridge.CODEC_PREFIX then return nil, "unsupported version: " .. tostring(prefix) end
  mapID, n, check = tonumber(mapID), tonumber(n), tonumber(check)
  if #coords ~= n * 6 then
    return nil, string.format("length mismatch: %d coords for %d waypoints", #coords / 6, n)
  end
  if n < 2 or n > 64 then return nil, "waypoint count out of range (2-64)" end
  if checksum(mapID, n, coords) ~= check then
    return nil, "checksum mismatch — code was mistyped or truncated"
  end
  local waypoints = {}
  for i = 1, n do
    local chunk = coords:sub((i - 1) * 6 + 1, i * 6)
    local ix, iy = tonumber(chunk:sub(1, 3)), tonumber(chunk:sub(4, 6))
    table.insert(waypoints, { x = ix / 10, y = iy / 10 })
  end
  return { mapID = mapID, waypoints = waypoints }
end

--- Handle "/go import <code>": decode, store, load into navigation.
function Bridge.ImportRouteString(code)
  if not code or code == "" then
    GO.Log("Usage: /go import <code>  (paste the code from the desktop app)")
    return false
  end
  local route, err = Bridge.DecodeRouteString(code)
  if not route then
    GO.Log("Import failed: " .. tostring(err))
    return false
  end
  local db = GO.Store.DB()
  db.activeRoute = {
    name = "Desktop route",
    mapID = route.mapID,
    waypoints = route.waypoints,
    importedAt = GO.Now(),
  }
  local loaded = 0
  if GO.Navigation then
    loaded = GO.Navigation.SetWaypoints(route.mapID, route.waypoints, "Route")
  end
  GO.Log(string.format("Route imported: %d waypoints (map %d). Type /go nav start to navigate.",
    loaded, route.mapID))
  return true
end

--- Re-load the stored route into navigation (e.g. after /reload).
function Bridge.RestoreActiveRoute()
  local db = GO.Store.DB()
  local r = db.activeRoute
  if not r or not GO.Navigation then return 0 end
  return GO.Navigation.SetWaypoints(r.mapID, r.waypoints, r.name or "Route")
end
