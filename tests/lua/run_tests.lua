-- run_tests.lua — headless functional tests for the GatherOptimizer addon.
-- Usage: lua5.1 tests/lua/run_tests.lua   (run from repo root)
local ROOT = arg and arg[0] and arg[0]:match("^(.*/)tests/lua/run_tests%.lua$") or "./"
if ROOT == "" then ROOT = "./" end

dofile(ROOT .. "tests/lua/mock_wow.lua")
local MOCK = _G.MOCK

-- Fresh addon namespace, then load modules in TOC order.
_G.__GO_TEST_NS = {}
_G.GatherOptimizerDB = nil
local ADDON = ROOT .. "addon/GatherOptimizer/"
for _, f in ipairs({ "Core.lua", "Store.lua", "Session.lua", "GatherMate.lua",
                     "Routes.lua", "Navigation.lua", "Bridge.lua", "UI.lua" }) do
  local chunk, err = loadfile(ADDON .. f)
  assert(chunk, "load " .. f .. ": " .. tostring(err))
  chunk("GatherOptimizer", _G.__GO_TEST_NS)
end
local GO = _G.__GO_TEST_NS

-- Mini framework -----------------------------------------------------------------
local passed, failed = 0, 0
local failures = {}
local function test(name, fn)
  local ok, err = pcall(fn)
  if ok then
    passed = passed + 1
    print("ok  " .. name)
  else
    failed = failed + 1
    table.insert(failures, name .. ": " .. tostring(err))
    print("FAIL  " .. name .. ": " .. tostring(err))
  end
end
local function eq(a, b, msg)
  if a ~= b then error((msg or "mismatch") .. " (got " .. tostring(a) .. ", want " .. tostring(b) .. ")", 2) end
end
local function isTrue(v, msg) if not v then error(msg or "expected true", 2) end end
local function isFalse(v, msg) if v then error(msg or "expected false", 2) end end
local function approx(a, b, tol, msg)
  if math.abs(a - b) > (tol or 1e-9) then error((msg or "approx") .. " (" .. tostring(a) .. " vs " .. tostring(b) .. ")", 2) end
end

local function boot()
  MOCK.reset()
  _G.GatherOptimizerDB = nil
  GO.Session._active = nil
  GO.Navigation.Stop(true)
  GO.Fire("ADDON_LOADED", "GatherOptimizer")
  GO.Fire("PLAYER_LOGIN")
end

-- Bootstrap -----------------------------------------------------------------------
test("boot creates versioned SavedVariables", function()
  boot()
  local db = _G.GatherOptimizerDB
  eq(db.schemaVersion, 1)
  eq(db.export.version, 1)
  isTrue(db.export.exportedAt > 0)
  eq(#db.sessions, 0)
  eq(#db.observations, 0)
end)

test("non-addon ADDON_LOADED is ignored", function()
  boot()
  _G.GatherOptimizerDB = nil
  GO.Fire("ADDON_LOADED", "SomeOtherAddon")
  eq(_G.GatherOptimizerDB, nil)
end)

test("load message printed", function()
  boot()
  isTrue(#MOCK.log >= 1)
  isTrue(MOCK.log[1]:find("GatherOptimizer") ~= nil)
end)

-- Store ------------------------------------------------------------------------------
test("migrate fills missing keys", function()
  local db = GO.Store.Migrate({ schemaVersion = 1, sessions = {} })
  eq(db.schemaVersion, 1)
  isTrue(type(db.observations) == "table")
  isTrue(type(db.settings) == "table")
end)

test("migrate repairs unknown versions, keeps sane data", function()
  local db = GO.Store.Migrate({ schemaVersion = 99, sessions = { { id = "x" } }, settings = { maxSessions = 7 } })
  eq(db.schemaVersion, 1)
  eq(#db.sessions, 1)
  eq(db.settings.maxSessions, 7)
end)

test("migrate handles nil", function()
  local db = GO.Store.Migrate(nil)
  eq(db.schemaVersion, 1)
end)

test("prune enforces caps", function()
  boot()
  local db = GO.Store.DB()
  db.settings.maxObservations = 5
  for i = 1, 10 do GO.Store.AddObservation({ t = i }) end
  eq(#db.observations, 5)
end)

-- Loot parsing -------------------------------------------------------------------------
test("ParseLootMessage handles single + stacked loot", function()
  local id, name, qty = GO.Session.ParseLootMessage("You receive loot: " .. MOCK.lootLink(12345, "Ghost Iron Ore") .. ".")
  eq(id, 12345) eq(name, "Ghost Iron Ore") eq(qty, 1)
  local _, _, q3 = GO.Session.ParseLootMessage("You receive loot: " .. MOCK.lootLink(9, "Mycobloom") .. "x3.")
  eq(q3, 3)
end)

test("ParseLootMessage rejects foreign loot + garbage", function()
  eq(GO.Session.ParseLootMessage("Someone receives loot: [x]."), nil)
  eq(GO.Session.ParseLootMessage("You receive currency: 10g."), nil)
  eq(GO.Session.ParseLootMessage(nil), nil)
  eq(GO.Session.ParseLootMessage(""), nil)
end)

test("ClassifyResource mining/herbalism/unknown", function()
  eq(GO.Session.ClassifyResource("Ghost Iron Ore", 12345), "mining")
  eq(GO.Session.ClassifyResource("Mycobloom", 9), "herbalism")
  eq(GO.Session.ClassifyResource("Linen Cloth", 7), "unknown")
end)

-- Session lifecycle -----------------------------------------------------------------------
test("start/pause/resume/stop accrue active time (pause excluded)", function()
  boot()
  GO.HandleSlash("start test-note")
  MOCK.advance(100)
  GO.Fire("CHAT_MSG_LOOT", "You receive loot: " .. MOCK.lootLink(1, "Ghost Iron Ore") .. ".")
  GO.HandleSlash("pause")
  MOCK.advance(50) -- paused: must not accrue
  GO.HandleSlash("resume")
  MOCK.advance(20)
  local s = GO.Session.Stop()
  eq(s.gatherCount, 1)
  eq(s.zone, "Test Zone")
  eq(s.activeSeconds, 120)
  eq(s.state, "ended")
  isTrue(s.endedAt > s.startedAt)
end)

test("gather records map context + observation ring", function()
  boot()
  GO.HandleSlash("start")
  MOCK.x, MOCK.y = 0.5, 0.25
  GO.Fire("CHAT_MSG_LOOT", "You receive loot: " .. MOCK.lootLink(2, "Mycobloom") .. "x2.")
  local s = GO.Session.Stop()
  eq(#s.gathers, 1)
  approx(s.gathers[1].x, 50, 1e-9)
  approx(s.gathers[1].y, 25, 1e-9)
  eq(s.gathers[1].qty, 2)
  eq(s.gathers[1].cat, "herbalism")
  eq(#GO.Store.DB().observations, 1)
end)

test("observations recorded even when idle", function()
  boot()
  GO.Fire("CHAT_MSG_LOOT", "You receive loot: " .. MOCK.lootLink(3, "Bismuth") .. ".")
  eq(#GO.Store.DB().observations, 1)
  eq(#GO.Store.DB().sessions, 0)
end)

test("double start is rejected", function()
  boot()
  GO.HandleSlash("start")
  local n = #MOCK.log
  GO.HandleSlash("start")
  isTrue(#MOCK.log > n)
  GO.HandleSlash("stop")
end)

test("leftover recording session is closed on reload", function()
  boot()
  GO.HandleSlash("start")
  GO.Session.Init() -- simulate second ADDON_LOADED
  local db = GO.Store.DB()
  eq(db.sessions[1].state, "ended")
end)

test("status strings for idle + active", function()
  boot()
  isTrue(GO.Session.Status():find("Idle") ~= nil)
  GO.HandleSlash("start")
  isTrue(GO.Session.Status():find("Recording") ~= nil)
  GO.HandleSlash("stop")
end)

-- Bridge codec ------------------------------------------------------------------------------
test("GO1 encode/decode roundtrip at 0.1 precision", function()
  boot()
  local wps = { { x = 42.04, y = 13.06 }, { x = 50.0, y = 50.0 }, { x = 99.95, y = 0.02 } }
  local code, err = GO.Bridge.EncodeRoute(2346, wps)
  isTrue(code ~= nil, tostring(err))
  isTrue(code:find("^GO1%.2346%.3%.") ~= nil, code)
  local r, derr = GO.Bridge.DecodeRouteString(code)
  isTrue(r ~= nil, tostring(derr))
  eq(r.mapID, 2346)
  eq(#r.waypoints, 3)
  approx(r.waypoints[1].x, 42.0, 1e-9)
  approx(r.waypoints[3].x, 99.9, 1e-9) -- clamped
end)

test("GO1 rejects tampered checksum + bad shapes", function()
  boot()
  local code = GO.Bridge.EncodeRoute(1, { { x = 1, y = 2 }, { x = 3, y = 4 } })
  local bad = code:sub(1, -4) .. "999"
  local r, err = GO.Bridge.DecodeRouteString(bad)
  eq(r, nil)
  isTrue(err:find("checksum") ~= nil)
  eq(GO.Bridge.DecodeRouteString("GO2.1.2.000000000000.000"), nil)
  eq(GO.Bridge.DecodeRouteString("not a code"), nil)
  eq(GO.Bridge.DecodeRouteString("GO1.1.3.000000000000.000"), nil) -- length mismatch
end)

test("import stores route + loads navigation", function()
  boot()
  local code = GO.Bridge.EncodeRoute(2346, { { x = 10, y = 10 }, { x = 20, y = 20 } })
  isTrue(GO.Bridge.ImportRouteString(code))
  local db = GO.Store.DB()
  eq(db.activeRoute.mapID, 2346)
  eq(#db.activeRoute.waypoints, 2)
  eq(GO.Navigation.State().total, 2)
end)

test("import rejects garbage", function()
  boot()
  isFalse(GO.Bridge.ImportRouteString("garbage"))
  eq(GO.Store.DB().activeRoute, nil)
end)

test("BuildExport carries counts + timestamps", function()
  boot()
  GO.HandleSlash("start")
  GO.Fire("CHAT_MSG_LOOT", "You receive loot: " .. MOCK.lootLink(1, "Ore") .. ".")
  GO.HandleSlash("stop")
  local e = GO.Bridge.BuildExport()
  eq(e.version, 1)
  eq(e.sessionCount, 1)
  eq(e.observationCount, 1)
  eq(e.character, "Testtoon")
  isTrue(e.newestSessionAt > 0)
end)

-- Navigation -----------------------------------------------------------------------------
test("navigation works without TomTom (text mode)", function()
  boot()
  isFalse(GO.Navigation.IsTomTomAvailable())
  eq(GO.Navigation.SetWaypoints(2346, { { x = 1, y = 1 }, { x = 2, y = 2 } }), 2)
  isTrue(GO.Navigation.Start())
  local st = GO.Navigation.State()
  eq(st.active, true)
  GO.Navigation.Next()
  eq(GO.Navigation.State().index, 2)
  GO.Navigation.Next() -- completes route
  eq(GO.Navigation.State().active, false)
end)

test("navigation drives TomTom when present", function()
  boot()
  local calls = { add = {}, arrow = {}, removed = {} }
  _G.TomTom = {
    AddWaypoint = function(self, map, x, y, opts)
      table.insert(calls.add, { map = map, x = x, y = y, opts = opts })
      return "uid" .. #calls.add
    end,
    SetCrazyArrow = function() table.insert(calls.arrow, true) end,
    RemoveWaypoint = function(self, uid) table.insert(calls.removed, uid) end,
    ClearCrazyArrow = function() end,
  }
  isTrue(GO.Navigation.IsTomTomAvailable())
  GO.Navigation.SetWaypoints(2346, { { x = 10, y = 20 }, { x = 30, y = 40 } })
  GO.Navigation.Start()
  eq(#calls.add, 2)
  approx(calls.add[1].x, 0.1, 1e-9)
  eq(#calls.arrow, 1)
  GO.Navigation.Next()
  eq(#calls.removed, 1)
  GO.Navigation.Stop(false)
end)

test("start with no route fails gracefully", function()
  boot()
  GO.Navigation.SetWaypoints(nil, {}) -- clear route left by earlier tests
  isFalse(GO.Navigation.Start())
end)

-- Optional adapters ----------------------------------------------------------------------------
test("GatherMate adapter degrades + normalizes when present", function()
  boot()
  isFalse(GO.GatherMate.IsAvailable())
  eq(#GO.GatherMate.GetZoneNodes("Zone"), 0)
  isTrue(GO.GatherMate.Status():find("not installed") ~= nil)
  _G.GatherMate2 = {
    GetNodesForZone = function(self, zone)
      return { { x = 10, y = 20, kind = "herb" }, { x = 30, y = 40 } }
    end,
  }
  isTrue(GO.GatherMate.IsAvailable())
  local nodes = GO.GatherMate.GetZoneNodes("Zone")
  eq(#nodes, 2)
  eq(nodes[1].kind, "herb")
end)

test("Routes adapter degrades + pushes when present", function()
  boot()
  isFalse(GO.Routes.IsAvailable())
  local ok = GO.Routes.PushRoute("r", "z", { { x = 1, y = 1 }, { x = 2, y = 2 } })
  isFalse(ok)
  local pushed = nil
  _G.Routes = {
    AddRoute = function(self, name, zone, pts) pushed = { name = name, zone = zone, pts = pts } return true end,
  }
  local ok2, msg = GO.Routes.PushRoute("myroute", "Test Zone", { { x = 50, y = 25 }, { x = 60, y = 30 } })
  isTrue(ok2, tostring(msg))
  eq(pushed.name, "myroute")
  approx(pushed.pts[1].x, 0.5, 1e-9)
end)

-- UI ------------------------------------------------------------------------------------------------
test("UI panel toggles + refreshes on 1Hz tick", function()
  boot()
  GO.HandleSlash("start")
  GO.UI.Toggle()
  isTrue(GO.UI.IsShown())
  local onUpdate = GO.UI._frame:GetScript("OnUpdate")
  isTrue(type(onUpdate) == "function")
  onUpdate(GO.UI._frame, 1.2) -- >1s triggers refresh
  local text = GO.UI._lines[1]:GetText()
  isTrue(text:find("Recording") ~= nil, text)
  GO.UI.Toggle()
  isFalse(GO.UI.IsShown())
  GO.HandleSlash("stop")
end)

-- Summary ----------------------------------------------------------------------------------------------
print(string.format("\n%d passed, %d failed", passed, failed))
if failed > 0 then
  print("failures:")
  for _, f in ipairs(failures) do print("  - " .. f) end
  os.exit(1)
end
