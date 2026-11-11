-- Navigate the stock player bags and use their normal right-click handler.
local selected, selectedBag = nil, 0
local lastLink
local panel = CreateFrame("Frame", nil, UIParent)
panel:SetSize(340, 100)
panel:SetPoint("TOP", UIParent, "TOP", 0, -65)
panel:SetBackdrop({bgFile = "Interface\\DialogFrame\\UI-DialogBox-Background"})
local hint = panel:CreateFontString(nil, "OVERLAY", "GameFontHighlight")
hint:SetPoint("TOPLEFT", panel, "TOPLEFT", 12, -12)
hint:SetWidth(316)
hint:SetJustifyH("LEFT")
panel:Hide()

local function reset()
    if selected then
        selected:UnlockHighlight()
        if GameTooltip:IsOwned(selected) then GameTooltip:Hide() end
    end
    selected, lastLink = nil, nil
    panel:Hide()
end

function BenillaControllerInventory(horizontal, vertical, bagStep, confirm, close, active)
    if not active then reset(); return end
    if close then
        reset()
        CloseAllBags()
        return
    end

    local bags = {}
    for i = 1, NUM_CONTAINER_FRAMES do
        local frame = _G["ContainerFrame" .. i]
        if frame:IsVisible() and frame:GetID() >= 0 and frame:GetID() <= 4 then
            bags[#bags + 1] = frame
        end
    end
    table.sort(bags, function(a, b) return a:GetID() < b:GetID() end)
    if #bags == 0 then reset(); return end

    local bagIndex = 1
    for i, frame in ipairs(bags) do
        if frame:GetID() == selectedBag then bagIndex = i end
    end
    bagIndex = (bagIndex - 1 + bagStep) % #bags + 1
    local bag = bags[bagIndex]
    selectedBag = bag:GetID()
    local buttons, index = {}, 1
    for i = 1, bag.size do
        local button = _G[bag:GetName() .. "Item" .. i]
        if button and button:IsVisible() then buttons[#buttons + 1] = button end
    end
    table.sort(buttons, function(a, b) return a:GetID() < b:GetID() end)
    for i, button in ipairs(buttons) do
        if selected == button then index = i end
    end
    if #buttons == 0 then reset(); return end
    index = (index - 1 + horizontal + vertical * 4) % #buttons + 1
    local button = buttons[index]
    local slot = button:GetID()
    local link = GetContainerItemLink(selectedBag, slot)
    if selected ~= button or lastLink ~= link then
        reset()
        selected, lastLink = button, link
        button:LockHighlight()
        if link then ContainerFrameItemButton_OnEnter(button) end
    end

    panel:Show()
    local bagName = selectedBag == 0 and "Backpack" or ("Bag " .. selectedBag)
    local name = link and GetItemInfo(link) or "Empty slot"
    hint:SetText(bagName .. " - " .. (name or link or "Item") .. "\n"
        .. "D-pad: item   LB/RB: bag\nA/Cross: equip or use   B/Circle: close")
    if not confirm or not link then return end

    -- The shortcut is for personal use, while the stock right-click has service-window meanings.
    for _, name in ipairs({"MerchantFrame", "MailFrame", "BankFrame", "TradeFrame", "AuctionFrame"}) do
        local frame = _G[name]
        if frame and frame:IsVisible() then
            hint:SetText("Close the shop, mail, bank or trade window to equip or use items.")
            return
        end
    end
    if GetCursorInfo() then
        hint:SetText("Finish moving the item on the cursor, then press A/Cross.")
        return
    end
    local _, _, locked = GetContainerItemInfo(selectedBag, slot)
    if locked then return end
    ContainerFrameItemButton_OnClick(button, "RightButton")
end
