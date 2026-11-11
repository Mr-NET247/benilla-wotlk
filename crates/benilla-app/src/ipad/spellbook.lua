-- Controller assignment uses the stock spell pickup and action placement APIs.
local selected, spellSlot, spellName, destination = nil, nil, nil, 1
local panel = CreateFrame("Frame", nil, SpellBookFrame)
panel:SetSize(290, 115)
panel:SetPoint("TOPLEFT", SpellBookFrame, "TOPRIGHT", 12, -35)
panel:SetBackdrop({bgFile = "Interface\\DialogFrame\\UI-DialogBox-Background"})
local hint = panel:CreateFontString(nil, "OVERLAY", "GameFontHighlight")
hint:SetPoint("TOPLEFT", panel, "TOPLEFT", 12, -12)
hint:SetWidth(266)
hint:SetJustifyH("LEFT")
panel:Hide()

local function focus(button)
    if selected == button then return end
    if selected then selected:UnlockHighlight() end
    selected = button
    if selected then selected:LockHighlight() end
end

local function slotButton(index)
    if index <= 12 then return _G["ActionButton" .. index] end
    return _G["MultiBarBottomLeftButton" .. (index - 12)]
end

local function slotLabel(index)
    if index <= 12 then return "Main bar slot " .. index end
    return "Bottom-left bar slot " .. (index - 12)
end

function BenillaControllerSpellbook(direction, page, tab, confirm, cancel, active)
    if not active then
        focus(nil)
        spellSlot, spellName = nil, nil
        panel:Hide()
        return
    end
    panel:Show()
    if cancel then
        focus(nil)
        if spellSlot then
            spellSlot, spellName = nil, nil
        else
            HideUIPanel(SpellBookFrame)
            panel:Hide()
            return
        end
    end

    if spellSlot then
        destination = (destination - 1 + direction + page * 4) % 16 + 1
        local button = slotButton(destination)
        focus(button)
        local action = button and ActionButton_GetPagedID(button)
        local kind, id
        if action then kind, id = GetActionInfo(action) end
        local existing = kind == "spell" and GetSpellInfo(id) or kind
        hint:SetText(spellName .. "\n" .. slotLabel(destination)
            .. (existing and (" replaces " .. existing) or " (empty)")
            .. "\nD-pad: slot   A/Cross: assign\nB/Circle: back")
        if confirm and action then
            -- Leave an existing cursor operation for its owner to finish.
            if GetCursorInfo() then
                hint:SetText("Finish moving the item on the cursor, then press A/Cross.")
                return
            end
            PickupSpell(spellSlot, BOOKTYPE_SPELL)
            if GetCursorInfo() == "spell" then
                PlaceAction(action)
                ClearCursor()
                spellSlot, spellName = nil, nil
                focus(nil)
                hint:SetText("Spell assigned.\nD-pad: choose another spell\nLT/L2: close spellbook")
            end
        end
        return
    end

    if SpellBookFrame.bookType ~= BOOKTYPE_SPELL then
        focus(nil)
        hint:SetText("Open the Spells tab to assign a spell.")
        return
    end
    if tab ~= 0 then
        local count = GetNumSpellTabs()
        if count > 0 then
            local nextTab = ((SpellBookFrame.selectedSkillLine or 1) - 1 + tab) % count + 1
            SpellBookSkillLineTab_OnClick(_G["SpellBookSkillLineTab" .. nextTab])
            focus(nil)
        end
    end
    if page ~= 0 then
        local button = page > 0 and SpellBookNextPageButton or SpellBookPrevPageButton
        if button:IsEnabled() == 1 then button:Click(); focus(nil) end
    end
    local buttons, index = {}, 1
    for i = 1, SPELLS_PER_PAGE do
        local button = _G["SpellButton" .. i]
        local slot = SpellBook_GetSpellID(button:GetID())
        if button:IsVisible() and button:IsEnabled() == 1 and GetSpellName(slot, BOOKTYPE_SPELL)
            and not IsPassiveSpell(slot, BOOKTYPE_SPELL) then
            buttons[#buttons + 1] = button
        end
    end
    table.sort(buttons, function(a, b) return a:GetID() < b:GetID() end)
    for i, button in ipairs(buttons) do
        if selected == button then index = i end
    end
    if #buttons > 0 then index = (index - 1 + direction) % #buttons + 1 end
    local button = buttons[index]
    focus(button)
    hint:SetText("D-pad up/down: spell\nLeft/right: page   LB/RB: category\nA/Cross: choose   B/Circle: close")
    if confirm and button then
        spellSlot = SpellBook_GetSpellID(button:GetID())
        spellName = GetSpellName(spellSlot, BOOKTYPE_SPELL)
        destination = 1
        focus(slotButton(destination))
        hint:SetText(spellName .. "\n" .. slotLabel(destination)
            .. "\nD-pad: choose slot\nA/Cross: assign   B/Circle: back")
    end
end
