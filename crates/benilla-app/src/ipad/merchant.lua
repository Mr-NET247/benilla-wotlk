-- Vendor controls call the stock buying, selling and confirmation handlers.
local mode, selected = 0, nil
local panel = CreateFrame("Frame", nil, MerchantFrame)
panel:SetSize(360, 115)
panel:SetPoint("TOP", UIParent, "TOP", 0, -65)
panel:SetBackdrop({bgFile = "Interface\\DialogFrame\\UI-DialogBox-Background"})
local hint = panel:CreateFontString(nil, "OVERLAY", "GameFontHighlight")
hint:SetPoint("TOPLEFT", panel, "TOPLEFT", 12, -12)
hint:SetWidth(336)
hint:SetJustifyH("LEFT")
panel:Hide()

local function focus(button)
    if selected == button then return end
    if selected then
        selected:UnlockHighlight()
        if GameTooltip:IsOwned(selected) then GameTooltip:Hide() end
    end
    selected = button
    if not button then return end
    button:LockHighlight()
    if mode == 1 then
        ContainerFrameItemButton_OnEnter(button)
    else
        MerchantItemButton_OnEnter(button)
    end
end

function BenillaControllerMerchant(horizontal, vertical, modeStep, page, confirm, close, active)
    if not active then
        focus(nil)
        panel:Hide()
        mode = 0
        return
    end
    panel:Show()
    for i = 1, 4 do
        local popup = _G["StaticPopup" .. i]
        if popup and popup:IsVisible() then
            local which = popup.which
            if which == "CONFIRM_HIGH_COST_ITEM" or which == "CONFIRM_PURCHASE_TOKEN_ITEM"
                or which == "CONFIRM_REFUND_TOKEN_ITEM" then
                hint:SetText("Confirm this transaction?\nA/Cross: confirm   B/Circle: cancel")
                local button = _G[popup:GetName() .. (close and "Button2" or "Button1")]
                if (close or confirm) and button and button:IsEnabled() == 1 then button:Click() end
            else
                hint:SetText("Finish the open confirmation to return to the vendor.")
            end
            return
        end
    end
    if close then
        focus(nil)
        panel:Hide()
        MerchantFrameCloseButton:Click()
        mode = 0
        return
    end

    if modeStep ~= 0 then
        mode = (mode + modeStep) % 3
        focus(nil)
        local tab = mode == 2 and MerchantFrameTab2 or MerchantFrameTab1
        tab:Click()
        if mode == 1 then OpenAllBags(true) end
    end
    local buttons, index, label = {}, 1
    if mode == 1 then
        for bag = 0, 4 do
            for i = 1, NUM_CONTAINER_FRAMES do
                local frame = _G["ContainerFrame" .. i]
                if frame:IsVisible() and frame:GetID() == bag then
                    local slots = {}
                    for j = 1, frame.size do
                        local button = _G[frame:GetName() .. "Item" .. j]
                        if button:IsVisible() and GetContainerItemLink(bag, button:GetID()) then
                            slots[#slots + 1] = button
                        end
                    end
                    table.sort(slots, function(a, b) return a:GetID() < b:GetID() end)
                    for _, button in ipairs(slots) do buttons[#buttons + 1] = button end
                end
            end
        end
        label = "SELL from your bags"
    else
        if page ~= 0 and mode == 0 then
            local button = page > 0 and MerchantNextPageButton or MerchantPrevPageButton
            if button:IsVisible() and button:IsEnabled() == 1 then button:Click(); focus(nil) end
        end
        for i = 1, mode == 2 and BUYBACK_ITEMS_PER_PAGE or MERCHANT_ITEMS_PER_PAGE do
            local button = _G["MerchantItem" .. i .. "ItemButton"]
            if button:IsVisible() and button:IsEnabled() == 1 then buttons[#buttons + 1] = button end
        end
        label = mode == 2 and "BUY BACK a sold item" or "BUY from the shop"
    end
    for i, button in ipairs(buttons) do if selected == button then index = i end end
    if #buttons > 0 then index = (index - 1 + horizontal + vertical * (mode == 1 and 4 or 2)) % #buttons + 1 end
    local button = buttons[index]
    focus(button)
    local name, price, quantity, _
    if button and mode == 1 then
        local bag, slot = button:GetParent():GetID(), button:GetID()
        local link = GetContainerItemLink(bag, slot)
        local _, count = GetContainerItemInfo(bag, slot)
        local itemName, _, _, _, _, _, _, _, _, _, sellPrice = GetItemInfo(link)
        name, price, quantity = itemName or link, sellPrice and sellPrice * count, count
    elseif button and mode == 2 then
        name, _, price, quantity = GetBuybackItemInfo(button:GetID())
    elseif button then
        name, _, price, quantity = GetMerchantItemInfo(button:GetID())
    end
    local money = price and ("  " .. GetCoinText(price, " ")) or ""
    hint:SetText(label .. "\n" .. (name or "No items") .. money
        .. (mode == 1 and quantity and (" (whole stack: " .. quantity .. ")") or "")
        .. "\nD-pad: item   LB/RB: buy / sell / buyback\nA/Cross: confirm   B/Circle: close"
        .. (mode == 0 and "\nX/Y or Square/Triangle: shop page" or ""))
    if not confirm or not button or GetCursorInfo() then return end
    if mode == 1 then
        local _, _, locked = GetContainerItemInfo(button:GetParent():GetID(), button:GetID())
        if not locked then ContainerFrameItemButton_OnClick(button, "RightButton") end
    else
        MerchantItemButton_OnClick(button, "RightButton")
    end
end
