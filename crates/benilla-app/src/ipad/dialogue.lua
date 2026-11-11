-- The iPad controller drives the stock gossip and quest buttons.
local currentPanel, selected

local function focus(button, scroll)
    if selected == button then return end
    if selected then selected:UnlockHighlight() end
    selected = button
    if not button then return end

    button:LockHighlight()
    if not scroll then return end
    local top, bottom = button:GetTop(), button:GetBottom()
    local viewTop, viewBottom = scroll:GetTop(), scroll:GetBottom()
    if not (top and bottom and viewTop and viewBottom) then return end

    local offset = scroll:GetVerticalScroll()
    if top > viewTop then
        offset = offset - (top - viewTop)
    elseif bottom < viewBottom then
        offset = offset + (viewBottom - bottom)
    else
        return
    end
    scroll:SetVerticalScroll(math.max(0, math.min(offset, scroll:GetVerticalScrollRange())))
end

local function visible(frame)
    return frame and frame:IsVisible()
end

local function add(buttons, button)
    if visible(button) and button:IsEnabled() == 1 then
        buttons[#buttons + 1] = button
    end
end

local function addRows(buttons, prefix, count)
    for index = 1, count do
        add(buttons, _G[prefix .. index])
    end
end

function BenillaControllerDialogue(direction, confirm, close, active)
    local panel, scroll, cancel, complete
    local buttons = {}
    if active and visible(QuestFrameDetailPanel) then
        panel, scroll, cancel = QuestFrameDetailPanel, QuestDetailScrollFrame, QuestFrameCloseButton
        add(buttons, QuestFrameAcceptButton)
    elseif active and visible(QuestFrameProgressPanel) then
        panel, scroll, cancel = QuestFrameProgressPanel, QuestProgressScrollFrame, QuestFrameCloseButton
        add(buttons, QuestFrameCompleteButton)
    elseif active and visible(QuestFrameRewardPanel) then
        panel, scroll, cancel = QuestFrameRewardPanel, QuestRewardScrollFrame, QuestFrameCloseButton
        local choices = GetNumQuestChoices()
        addRows(buttons, "QuestInfoItem", choices)
        if choices == 0 or (QuestInfoFrame.itemChoice or 0) > 0 then
            complete = QuestFrameCompleteQuestButton
            add(buttons, complete)
        end
    elseif active and visible(QuestFrameGreetingPanel) then
        panel, scroll, cancel = QuestFrameGreetingPanel, QuestGreetingScrollFrame, QuestFrameCloseButton
        addRows(buttons, "QuestTitleButton", 32)
        if #buttons == 0 then add(buttons, QuestFrameGreetingGoodbyeButton) end
    elseif active and visible(GossipFrame) then
        panel, scroll, cancel = GossipFrame, GossipGreetingScrollFrame, GossipFrameCloseButton
        addRows(buttons, "GossipTitleButton", NUMGOSSIPBUTTONS or 32)
        if #buttons == 0 then add(buttons, GossipFrameGreetingGoodbyeButton) end
    end

    if currentPanel ~= panel then
        currentPanel = panel
        focus(nil)
    end
    if not panel then return end
    if close then
        focus(nil)
        if cancel then cancel:Click() end
        return
    end

    local index = 1
    for i, button in ipairs(buttons) do
        if button == selected then index = i; break end
    end
    if #buttons > 0 then
        index = (index - 1 + direction) % #buttons + 1
    end
    local button = buttons[index]
    local scrollButton = button and button ~= QuestFrameAcceptButton
        and button ~= QuestFrameCompleteButton and button ~= QuestFrameCompleteQuestButton
        and button ~= QuestFrameGreetingGoodbyeButton and button ~= GossipFrameGreetingGoodbyeButton
    focus(button, scrollButton and scroll or nil)
    if not confirm or not button then return end

    focus(nil)
    button:Click()
    if panel == QuestFrameRewardPanel and button ~= QuestFrameCompleteQuestButton
        and visible(QuestFrameCompleteQuestButton) and (QuestInfoFrame.itemChoice or 0) > 0 then
        focus(QuestFrameCompleteQuestButton)
    end
end
