//! iPad input adapter for the stock movement and binding commands.
use benilla_ui::script::UiScript;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use objc2_ios::{
    class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};

use crate::bindings::Input;
use crate::char_select::ClientState;
use crate::loading_screen::LoadingScreen;
use crate::target::{self, Hovered, HoveredObject, Interact, Selection};

const SIZE: f32 = 112.0;
pub(super) const STICK_RADIUS_SQUARED: f32 = SIZE * SIZE / 4.0;
const LEFT: f32 = 30.0;
const BOTTOM: f32 = 104.0;
const KNOB: f32 = 44.0;
const TRAVEL: f32 = (SIZE - KNOB) / 2.0;

#[derive(Resource, Default)]
pub(super) struct TouchStick {
    pub(super) visible: bool,
    pub(super) offset: Vec2,
}

impl TouchStick {
    pub(super) fn center(window: &Window) -> Vec2 {
        Vec2::new(LEFT + SIZE / 2.0, window.height() - BOTTOM - SIZE / 2.0)
    }

    pub(super) fn update(&mut self, point: Vec2, center: Vec2) {
        self.offset = (point - center).clamp_length_max(TRAVEL);
    }
}

#[derive(Component)]
struct StickBase;

#[derive(Component)]
struct StickKnob;

pub(super) fn add_overlay(app: &mut App) {
    app.add_systems(Startup, spawn_stick)
        .add_systems(Update, draw_stick);
}

fn spawn_stick(mut commands: Commands) {
    commands.spawn((
        Name::new("iPad movement joystick"),
        StickBase,
        Node {
            position_type: PositionType::Absolute,
            left: px(LEFT),
            bottom: px(BOTTOM),
            width: px(SIZE),
            height: px(SIZE),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(2.0)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BorderColor::all(Color::srgba(0.8, 0.87, 0.94, 0.45)),
        BackgroundColor(Color::srgba(0.09, 0.13, 0.19, 0.44)),
        GlobalZIndex(20),
        Visibility::Hidden,
        children![(
            StickKnob,
            Node {
                width: px(KNOB),
                height: px(KNOB),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            UiTransform::default(),
            BackgroundColor(Color::srgba(0.78, 0.85, 0.94, 0.65)),
        )],
    ));
}

fn draw_stick(
    stick: Res<TouchStick>,
    mut base: Single<&mut Visibility, With<StickBase>>,
    mut knob: Single<&mut UiTransform, With<StickKnob>>,
) {
    **base = if stick.visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    knob.translation = bevy::ui::Val2::px(stick.offset.x, stick.offset.y);
}

pub(super) fn world_ready(state: &State<ClientState>, cover: Option<&LoadingScreen>) -> bool {
    *state.get() == ClientState::InWorld && !cover.is_some_and(LoadingScreen::covering)
}

fn panel_open(script: &UiScript) -> bool {
    [
        "GameMenuFrame",
        "WorldMapFrame",
        "StaticPopup1",
        "StaticPopup2",
        "StaticPopup3",
        "StaticPopup4",
    ]
    .iter()
    .any(|name| script.frame_visible(name))
}

pub(super) fn movement_ui_open(script: Option<&UiScript>) -> bool {
    script.is_some_and(|script| {
        !script.world_ui_visible() || script.has_keyboard_focus() || panel_open(script)
    })
}

#[derive(Resource, Default)]
pub(super) struct Gamepad {
    pub(super) connected: bool,
    left: Vec2,
    right: Vec2,
    buttons: u16,
    pressed: u16,
    jump_blocked: bool,
}

const JUMP: usize = 0;
const INTERACT: usize = 1;
const MENU: usize = 2;
const TARGET: usize = 3;
const BAGS: usize = 4;
const DPAD_LEFT: usize = 5;
const DPAD_RIGHT: usize = 6;
const DPAD_UP: usize = 7;
const DPAD_DOWN: usize = 8;
const PRIMARY_ACTION: usize = 9;
const SECONDARY_ACTION: usize = 10;
const SPELLBOOK: usize = 11;
const FACE_BUTTONS: [usize; 4] = [JUMP, PRIMARY_ACTION, INTERACT, SECONDARY_ACTION]; // A, X, B, Y
const DPAD: u16 = (1 << DPAD_LEFT) | (1 << DPAD_RIGHT) | (1 << DPAD_UP) | (1 << DPAD_DOWN);
const ACTION_BANKS: [[&str; 4]; 4] = [
    [
        "ACTIONBUTTON1",
        "ACTIONBUTTON2",
        "ACTIONBUTTON3",
        "ACTIONBUTTON4",
    ],
    [
        "ACTIONBUTTON5",
        "ACTIONBUTTON6",
        "ACTIONBUTTON7",
        "ACTIONBUTTON8",
    ],
    [
        "ACTIONBUTTON9",
        "ACTIONBUTTON10",
        "ACTIONBUTTON11",
        "ACTIONBUTTON12",
    ],
    [
        "MULTIACTIONBAR1BUTTON1",
        "MULTIACTIONBAR1BUTTON2",
        "MULTIACTIONBAR1BUTTON3",
        "MULTIACTIONBAR1BUTTON4",
    ],
];

impl Gamepad {
    fn down(&self, button: usize) -> bool {
        self.buttons & (1 << button) != 0
    }

    fn just_pressed(&self, button: usize) -> bool {
        self.pressed & (1 << button) != 0
    }

    fn dpad(&self) -> u16 {
        self.buttons & DPAD
    }

    fn action_bank(&self) -> Option<&'static [&'static str; 4]> {
        let directions = self.dpad();
        // A diagonal is ambiguous. Keep the modifier active but fire neither bank.
        if directions.is_power_of_two() {
            Some(&ACTION_BANKS[directions.trailing_zeros() as usize - DPAD_LEFT])
        } else {
            None
        }
    }
}

pub(super) fn poll_gamepad(
    window: Single<&Window, With<PrimaryWindow>>,
    mut pad: ResMut<Gamepad>,
    _main: bevy::ecs::system::NonSendMarker,
) {
    // GameController supplies the same extended profile for supported Xbox,
    // PlayStation and MFi pads. Fall back to the first connected pad until it is used.
    let controller: Option<Retained<AnyObject>> =
        unsafe { msg_send![class!(GCController), current] };
    let controller = controller.or_else(|| {
        let controllers: Retained<AnyObject> =
            unsafe { msg_send![class!(GCController), controllers] };
        unsafe { msg_send![&*controllers, firstObject] }
    });
    let profile: Option<Retained<AnyObject>> = controller
        .as_deref()
        .and_then(|controller| unsafe { msg_send![controller, extendedGamepad] });
    let Some(profile) = profile else {
        *pad = Gamepad::default();
        return;
    };
    pad.connected = true;
    if !window.focused {
        pad.left = Vec2::ZERO;
        pad.right = Vec2::ZERO;
        pad.pressed = 0;
        return;
    }

    // Selectors below return GameController elements, and axes are normalized floats.
    let profile = &*profile;
    let left: Retained<AnyObject> = unsafe { msg_send![profile, leftThumbstick] };
    let right: Retained<AnyObject> = unsafe { msg_send![profile, rightThumbstick] };
    let dpad: Retained<AnyObject> = unsafe { msg_send![profile, dpad] };
    pad.left = stick_axes(&left);
    pad.right = stick_axes(&right);
    let old = pad.buttons;
    pad.buttons = [
        unsafe { msg_send![profile, buttonA] },
        unsafe { msg_send![profile, buttonB] },
        unsafe { msg_send![profile, buttonMenu] },
        unsafe { msg_send![profile, leftShoulder] },
        unsafe { msg_send![profile, rightShoulder] },
        unsafe { msg_send![&*dpad, left] },
        unsafe { msg_send![&*dpad, right] },
        unsafe { msg_send![&*dpad, up] },
        unsafe { msg_send![&*dpad, down] },
        unsafe { msg_send![profile, buttonX] },
        unsafe { msg_send![profile, buttonY] },
        unsafe { msg_send![profile, leftTrigger] },
    ]
    .into_iter()
    .enumerate()
    .fold(0, |bits, (index, button): (_, Retained<AnyObject>)| {
        let down: Bool = unsafe { msg_send![&*button, isPressed] };
        bits | (u16::from(down.as_bool()) << index)
    });
    pad.pressed = pad.buttons & !old;
    // Releasing the direction before the face button must not produce an extra jump.
    pad.jump_blocked = pad.down(JUMP) && (pad.jump_blocked || pad.dpad() != 0);
}

fn stick_axes(stick: &AnyObject) -> Vec2 {
    let x: Retained<AnyObject> = unsafe { msg_send![stick, xAxis] };
    let y: Retained<AnyObject> = unsafe { msg_send![stick, yAxis] };
    let x: f32 = unsafe { msg_send![&*x, value] };
    let y: f32 = unsafe { msg_send![&*y, value] };
    Vec2::new(x, y).clamp_length_max(1.0)
}

/// Like keyboard movement, the 3.3.5a mover consumes direction bits. Keep this
/// source separate so lifting a thumb cannot release a direction held on a keyboard.
#[derive(Resource, Default)]
pub(crate) struct MobileMovement {
    pub(crate) held: Vec<Input>,
    pub(crate) look: Vec2,
}

pub(super) fn dispatch_controls(
    stick: Res<TouchStick>,
    mut pad: ResMut<Gamepad>,
    mut movement: ResMut<MobileMovement>,
    window: Single<&Window, With<PrimaryWindow>>,
    state: Res<State<ClientState>>,
    cover: Option<Res<LoadingScreen>>,
    script: Option<NonSendMut<UiScript>>,
    picks: (
        target::click::NearbyLoot,
        Res<Selection>,
        Res<Hovered>,
        Res<HoveredObject>,
        Query<&crate::net::ObjectStore>,
    ),
    mut interact: MessageWriter<Interact>,
    mut dialogue: Local<(crate::ui_script::VmMemo<bool>, bool)>,
    mut spellbook: Local<(crate::ui_script::VmMemo<bool>, bool)>,
    mut inventory: Local<(crate::ui_script::VmMemo<bool>, bool)>,
    mut merchant: Local<(crate::ui_script::VmMemo<bool>, bool)>,
) {
    movement.held.clear();
    movement.look = Vec2::ZERO;
    if !window.focused || !world_ready(&state, cover.as_deref()) {
        return;
    }
    let Some(mut script) = script else { return };
    if pad.just_pressed(MENU) {
        if script.has_keyboard_focus() {
            script.key_input("ESCAPE");
        } else if !script.frame_key_input("ESCAPE") {
            run_binding(&script, "TOGGLEGAMEMENU");
        }
        return;
    }
    if inventory.1
        && (movement_ui_open(Some(&script))
            || script.frame_visible("SpellBookFrame")
            || script.frame_visible("GossipFrame")
            || script.frame_visible("QuestFrame")
            || script.frame_visible("MerchantFrame"))
    {
        if let Err(error) = script.run("if BenillaControllerInventory then BenillaControllerInventory(0, 0, 0, false, false, false) end") {
            warn!("iPad controller inventory reset: {error}");
        }
        inventory.1 = false;
    }
    let spellbook_open =
        pad.connected && !movement_ui_open(Some(&script)) && script.frame_visible("SpellBookFrame");
    if spellbook_open || spellbook.1 {
        if spellbook.0.claim(&script) {
            if let Err(error) = script.run(include_str!("spellbook.lua")) {
                warn!("iPad controller spellbook setup: {error}");
                return;
            }
        }
        spellbook.1 = spellbook_open;
        let direction =
            i32::from(pad.just_pressed(DPAD_DOWN)) - i32::from(pad.just_pressed(DPAD_UP));
        let page = i32::from(pad.just_pressed(DPAD_RIGHT)) - i32::from(pad.just_pressed(DPAD_LEFT));
        let tab = i32::from(pad.just_pressed(BAGS)) - i32::from(pad.just_pressed(TARGET));
        let confirm = pad.just_pressed(JUMP);
        let cancel = pad.just_pressed(INTERACT);
        if let Err(error) = script.run(&format!(
            "BenillaControllerSpellbook({direction}, {page}, {tab}, {confirm}, {cancel}, {spellbook_open})"
        )) {
            warn!("iPad controller spellbook: {error}");
        }
        if spellbook_open {
            let jump_held = pad.down(JUMP);
            pad.jump_blocked |= jump_held;
            if pad.just_pressed(SPELLBOOK) {
                run_binding(&script, "TOGGLESPELLBOOK");
            }
            return;
        }
    }
    let dialogue_open = pad.connected
        && !movement_ui_open(Some(&script))
        && (script.frame_visible("GossipFrame") || script.frame_visible("QuestFrame"));
    if dialogue_open || dialogue.1 {
        if dialogue.0.claim(&script) {
            if let Err(error) = script.run(include_str!("dialogue.lua")) {
                warn!("iPad controller dialogue setup: {error}");
                return;
            }
        }
        dialogue.1 = dialogue_open;
        let next = pad.just_pressed(DPAD_DOWN) || pad.just_pressed(DPAD_RIGHT);
        let previous = pad.just_pressed(DPAD_UP) || pad.just_pressed(DPAD_LEFT);
        let direction = i32::from(next) - i32::from(previous);
        let confirm = pad.just_pressed(JUMP);
        let close = pad.just_pressed(INTERACT);
        if let Err(error) = script.run(&format!(
            "BenillaControllerDialogue({direction}, {confirm}, {close}, {dialogue_open})"
        )) {
            warn!("iPad controller dialogue: {error}");
        }
        if dialogue_open {
            // Keep a confirm press from becoming a jump when its panel closes.
            let jump_held = pad.down(JUMP);
            pad.jump_blocked |= jump_held;
            return;
        }
    }
    let merchant_open = pad.connected
        && !script.has_keyboard_focus()
        && !script.frame_visible("GameMenuFrame")
        && !script.frame_visible("WorldMapFrame")
        && script.frame_visible("MerchantFrame");
    if merchant_open || merchant.1 {
        if merchant.0.claim(&script) {
            if let Err(error) = script.run(include_str!("merchant.lua")) {
                warn!("iPad controller merchant setup: {error}");
                return;
            }
        }
        merchant.1 = merchant_open;
        let horizontal =
            i32::from(pad.just_pressed(DPAD_RIGHT)) - i32::from(pad.just_pressed(DPAD_LEFT));
        let vertical =
            i32::from(pad.just_pressed(DPAD_DOWN)) - i32::from(pad.just_pressed(DPAD_UP));
        let mode = i32::from(pad.just_pressed(BAGS)) - i32::from(pad.just_pressed(TARGET));
        let page = i32::from(pad.just_pressed(SECONDARY_ACTION))
            - i32::from(pad.just_pressed(PRIMARY_ACTION));
        let confirm = pad.just_pressed(JUMP);
        let close = pad.just_pressed(INTERACT);
        if let Err(error) = script.run(&format!(
            "BenillaControllerMerchant({horizontal}, {vertical}, {mode}, {page}, {confirm}, {close}, {merchant_open})"
        )) {
            warn!("iPad controller merchant: {error}");
        }
        if merchant_open {
            let jump_held = pad.down(JUMP);
            pad.jump_blocked |= jump_held;
            return;
        }
    }
    let inventory_open = pad.connected
        && !movement_ui_open(Some(&script))
        && (1..=13).any(|index| script.frame_visible(&format!("ContainerFrame{index}")));
    if inventory_open || inventory.1 {
        if inventory.0.claim(&script) {
            if let Err(error) = script.run(include_str!("inventory.lua")) {
                warn!("iPad controller inventory setup: {error}");
                return;
            }
        }
        inventory.1 = inventory_open;
        let horizontal =
            i32::from(pad.just_pressed(DPAD_RIGHT)) - i32::from(pad.just_pressed(DPAD_LEFT));
        let vertical =
            i32::from(pad.just_pressed(DPAD_DOWN)) - i32::from(pad.just_pressed(DPAD_UP));
        let bag = i32::from(pad.just_pressed(BAGS)) - i32::from(pad.just_pressed(TARGET));
        let confirm = pad.just_pressed(JUMP);
        let close = pad.just_pressed(INTERACT);
        if let Err(error) = script.run(&format!(
            "BenillaControllerInventory({horizontal}, {vertical}, {bag}, {confirm}, {close}, {inventory_open})"
        )) {
            warn!("iPad controller inventory: {error}");
        }
        if inventory_open {
            let jump_held = pad.down(JUMP);
            pad.jump_blocked |= jump_held;
            if pad.just_pressed(SPELLBOOK) {
                run_binding(&script, "TOGGLESPELLBOOK");
            }
            return;
        }
    }
    if movement_ui_open(Some(&script)) {
        return;
    }

    let touch = if stick.visible {
        stick.offset / TRAVEL
    } else {
        Vec2::ZERO
    };
    let stick_axes = Vec2::new(pad.left.x, -pad.left.y);
    for (active, input) in [
        (touch.y < -0.28 || stick_axes.y < -0.28, Input::MoveForward),
        (touch.y > 0.28 || stick_axes.y > 0.28, Input::MoveBackward),
        (touch.x < -0.28 || stick_axes.x < -0.28, Input::TurnLeft),
        (touch.x > 0.28 || stick_axes.x > 0.28, Input::TurnRight),
        (pad.down(JUMP) && !pad.jump_blocked, Input::Ascend),
    ] {
        if active {
            movement.held.push(input);
        }
    }
    let magnitude = pad.right.length();
    if magnitude > 0.18 {
        movement.look = pad.right / magnitude * ((magnitude - 0.18) / 0.82);
    }
    let (nearby_loot, selection, hovered, object, stores) = &picks;
    if let Some(bank) = pad.action_bank() {
        for (button, command) in FACE_BUTTONS.into_iter().zip(bank) {
            if pad.just_pressed(button) {
                run_binding(&script, command);
            }
        }
    } else if pad.dpad() == 0 {
        if pad.just_pressed(PRIMARY_ACTION) {
            let live_target = selection
                .target
                .and_then(|entity| stores.get(entity).ok())
                .is_some_and(|store| !store.0.unit_is_dead());
            if !live_target {
                // The script call queue applies the target change before the slot press.
                run_binding(&script, "TARGETNEARESTENEMY");
            }
            run_binding(&script, "ACTIONBUTTON1");
        }
        if pad.just_pressed(SECONDARY_ACTION) {
            run_binding(&script, "ACTIONBUTTON2");
        }
    }
    for (button, command) in [
        (TARGET, "TARGETNEARESTENEMY"),
        (BAGS, "OPENALLBAGS"),
        (SPELLBOOK, "TOGGLESPELLBOOK"),
    ] {
        if pad.just_pressed(button) {
            run_binding(&script, command);
        }
    }
    if pad.dpad() == 0 && pad.just_pressed(INTERACT) {
        let under_cursor = if target::go_is_nearest(hovered, object) {
            object.target
        } else {
            hovered.any()
        };
        if let Some(entity) = nearby_loot
            .nearest()
            .or_else(|| nearby_loot.nearest_dialogue(selection.target.or(under_cursor)))
            .or(selection.target)
            .or(under_cursor)
        {
            interact.write(Interact(entity));
        }
    }
}

fn run_binding(script: &UiScript, command: &str) {
    for down in [true, false] {
        if let Err(error) = script.execute_binding(command, down) {
            warn!("iPad controller binding {command}: {error}");
        }
    }
}
