-- GatherOptimizer/UI.lua — lightweight status panel (/go ui).
-- Refreshes at 1 Hz only while visible; no background OnUpdate loops.
local _, GO = ...
if GO == nil then GO = _G.__GO_TEST_NS or {} end

local UI = {}
GO.UI = UI
GO.modules["UI"] = UI

UI._frame = nil
UI._lines = {}
UI._elapsed = 0

local WIDTH, HEIGHT = 320, 210

local function setLines()
  local status = "Session module not loaded."
  if GO.Session then status = GO.Session.Status() end
  local nav = ""
  if GO.Navigation then
    local st = GO.Navigation.State()
    nav = string.format("Nav: %s (%d/%d)%s", st.active and "active" or "idle",
      st.index, st.total, st.tomtom and "" or " — TomTom missing")
  end
  local gm = (GO.GatherMate and GO.GatherMate.Status()) or ""
  local rt = (GO.Routes and GO.Routes.Status()) or ""
  local text = table.concat({
    "Status: " .. status,
    nav,
    gm,
    rt,
    " ",
    "Desktop sync: log out or /reload,",
    "then press Sync in wowgoldfarm.",
  }, "\n")
  for i, fs in ipairs(UI._lines) do
    if i == 1 and fs.SetText then fs:SetText(text) end
  end
end

local function makeButton(parent, label, x, onClick)
  local b = _G.CreateFrame("Button", nil, parent, "UIPanelButtonTemplate")
  b:SetSize(96, 24)
  b:SetPoint("BOTTOMLEFT", parent, "BOTTOMLEFT", x, 12)
  b:SetText(label)
  b:SetScript("OnClick", onClick)
  return b
end

function UI.Build()
  if UI._frame or not _G.CreateFrame then return UI._frame end
  local f = _G.CreateFrame("Frame", "GatherOptimizerPanel", _G.UIParent)
  f:SetSize(WIDTH, HEIGHT)
  f:SetPoint("CENTER")
  f:SetMovable(true)
  f:EnableMouse(true)
  f:RegisterForDrag("LeftButton")
  f:SetScript("OnDragStart", function(self) self:StartMoving() end)
  f:SetScript("OnDragStop", function(self) self:StopMovingOrSizing() end)
  if f.SetBackdrop then
    f:SetBackdrop({ bgFile = "Interface\\DialogFrame\\UI-DialogBox-Background" })
  end
  f:SetScript("OnUpdate", function(_, elapsed)
    UI._elapsed = UI._elapsed + (elapsed or 0)
    if UI._elapsed >= 1 then
      UI._elapsed = 0
      setLines()
    end
  end)

  local title = f:CreateFontString(nil, "OVERLAY", "GameFontNormal")
  title:SetPoint("TOP", f, "TOP", 0, -10)
  title:SetText("GatherOptimizer")

  local body = f:CreateFontString(nil, "OVERLAY", "GameFontHighlightSmall")
  body:SetPoint("TOPLEFT", f, "TOPLEFT", 12, -34)
  body:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", -12, 44)
  body:SetJustifyH("LEFT")
  body:SetJustifyV("TOP")
  UI._lines = { body }

  makeButton(f, "Start", 12, function() GO.SessionCmd("start") setLines() end)
  makeButton(f, "Stop", 114, function() GO.SessionCmd("stop") setLines() end)
  makeButton(f, "Close", 216, function() UI.Hide() end)

  f:Hide()
  UI._frame = f
  return f
end

function UI.Init()
  UI.Build()
end

function UI.Show()
  local f = UI._frame or UI.Build()
  if not f then GO.Log("UI unavailable.") return end
  setLines()
  f:Show()
end

function UI.Hide()
  if UI._frame then UI._frame:Hide() end
end

function UI.Toggle()
  if UI._frame and UI._frame.IsShown and UI._frame:IsShown() then
    UI.Hide()
  else
    UI.Show()
  end
end

function UI.IsShown()
  return UI._frame and UI._frame.IsShown and UI._frame:IsShown() or false
end
