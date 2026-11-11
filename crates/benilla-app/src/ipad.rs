use std::collections::VecDeque;
use std::path::PathBuf;

use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion};
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::input::{ButtonState, InputSystems};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::char_select::ClientState;
use crate::loading_screen::LoadingScreen;

mod controls;
pub(crate) use controls::MobileMovement;
use controls::{Gamepad, TouchStick};

#[link(name = "GameController", kind = "framework")]
extern "C" {}

/// Boot the player build with data and writable state in the iPad's Files folder.
pub fn run_ipad(build: crate::BuildId) -> crate::AppExit {
    let documents =
        PathBuf::from(objc2_foundation_ios::NSHomeDirectory().to_string()).join("Documents");
    std::env::set_var("WOW_DATA", documents.join("Data"));
    std::env::set_var("BENILLA_HOME", documents.join("benilla-config"));
    crate::run_with(build, configure_ipad)
}

fn configure_ipad(app: &mut App) {
    app.init_resource::<Gamepad>()
        .init_resource::<TouchStick>()
        .init_resource::<MobileMovement>()
        .add_systems(
            PreUpdate,
            (
                hardware_keyboard,
                (controls::poll_gamepad, touch_to_mouse).chain(),
            )
                .before(InputSystems),
        )
        .add_systems(
            Update,
            (
                show_keyboard.after(crate::ui_script::UiInput),
                controls::dispatch_controls
                    .in_set(crate::ui_script::UiInput)
                    .before(crate::bindings::BindingSet),
            ),
        );

    controls::add_overlay(app);
    if benilla_formats::wow_data().is_none() {
        app.add_systems(Startup, show_setup);
    }
}

fn show_setup(mut commands: Commands) {
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 100,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        bevy::ui::UiTargetCamera(camera),
        Node {
            position_type: PositionType::Absolute,
            left: percent(8),
            top: percent(12),
            width: percent(84),
            padding: UiRect::all(px(24)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.08, 0.12, 0.19)),
        Text::new(concat!(
            "Benilla-WotLK\n\n",
            "Copy the Data folder from your English WoW 3.3.5a (12340) installation to ",
            "On My iPad → Benilla-WotLK in Files. Then fully close and reopen the app.",
        )),
        TextFont {
            font_size: 24.0,
            ..default()
        },
    ));
}

fn show_keyboard(
    capture: Res<crate::ui_script::UiKeyboardCapture>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if window.ime_enabled != capture.typing {
        window.ime_enabled = capture.typing;
    }
}

// GameController uses USB HID usages. UIKit still supplies text to focused edit boxes.
const LETTERS: [KeyCode; 26] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
];
const DIGITS: [KeyCode; 10] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Digit0,
];
const CONTROLS: [(isize, KeyCode, Key); 13] = [
    (0x28, KeyCode::Enter, Key::Enter),
    (0x29, KeyCode::Escape, Key::Escape),
    (0x2b, KeyCode::Tab, Key::Tab),
    (0x2c, KeyCode::Space, Key::Space),
    (0x4f, KeyCode::ArrowRight, Key::ArrowRight),
    (0x50, KeyCode::ArrowLeft, Key::ArrowLeft),
    (0x51, KeyCode::ArrowDown, Key::ArrowDown),
    (0x52, KeyCode::ArrowUp, Key::ArrowUp),
    (0xe0, KeyCode::ControlLeft, Key::Control),
    (0xe1, KeyCode::ShiftLeft, Key::Shift),
    (0xe2, KeyCode::AltLeft, Key::Alt),
    (0xe4, KeyCode::ControlRight, Key::Control),
    (0xe5, KeyCode::ShiftRight, Key::Shift),
];

fn hardware_keyboard(
    primary: Single<Entity, With<PrimaryWindow>>,
    capture: Res<crate::ui_script::UiKeyboardCapture>,
    mut events: MessageWriter<KeyboardInput>,
    mut was_down: Local<bevy::input::ButtonInput<KeyCode>>,
    _main: bevy::ecs::system::NonSendMarker,
) {
    use objc2_ios::{
        class, msg_send,
        rc::Retained,
        runtime::{AnyObject, Bool},
    };

    // These selectors are on GCKeyboard, GCKeyboardInput and GCControllerButtonInput.
    let keyboard: Option<Retained<AnyObject>> =
        unsafe { msg_send![class!(GCKeyboard), coalescedKeyboard] };
    let input: Option<Retained<AnyObject>> = keyboard
        .as_deref()
        .and_then(|keyboard| unsafe { msg_send![keyboard, keyboardInput] });
    let any_pressed = input.as_deref().is_some_and(|input| {
        let pressed: Bool = unsafe { msg_send![input, isAnyKeyPressed] };
        pressed.as_bool()
    });
    let mut emit = |usage: isize, key_code, logical_key: Key, text_key: bool| {
        let delivered_by_uikit = capture.typing && text_key;
        let down = any_pressed
            && !delivered_by_uikit
            && input.as_deref().is_some_and(|input| {
                let button: Option<Retained<AnyObject>> =
                    unsafe { msg_send![input, buttonForKeyCode: usage] };
                button.as_deref().is_some_and(|button| {
                    let pressed: Bool = unsafe { msg_send![button, isPressed] };
                    pressed.as_bool()
                })
            });
        if was_down.pressed(key_code) == down {
            return;
        }
        let state = if down {
            was_down.press(key_code);
            ButtonState::Pressed
        } else {
            was_down.release(key_code);
            ButtonState::Released
        };
        events.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window: *primary,
        });
    };
    for (index, key_code) in LETTERS.into_iter().enumerate() {
        emit(
            4 + index as isize,
            key_code,
            Key::Unidentified(NativeKey::Unidentified),
            true,
        );
    }
    for (index, key_code) in DIGITS.into_iter().enumerate() {
        emit(
            0x1e + index as isize,
            key_code,
            Key::Unidentified(NativeKey::Unidentified),
            true,
        );
    }
    for (usage, key_code, logical_key) in CONTROLS {
        let text_key = matches!(logical_key, Key::Space | Key::Enter);
        emit(usage, key_code, logical_key, text_key);
    }
}

#[derive(Default)]
struct TouchPointer {
    contact: Option<(u64, MouseButton)>,
    stick_contact: Option<u64>,
    queued: VecDeque<TouchInput>,
}

fn touch_to_mouse(
    mut touches: MessageReader<TouchInput>,
    mut clicks: MessageWriter<MouseButtonInput>,
    mut motion: MessageWriter<MouseMotion>,
    mut primary: Single<(Entity, &mut Window), With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut pointer: Local<TouchPointer>,
    mut stick: ResMut<TouchStick>,
    pad: Res<Gamepad>,
    state: Res<State<ClientState>>,
    cover: Option<Res<LoadingScreen>>,
    script: Option<NonSend<benilla_ui::script::UiScript>>,
) {
    let (entity, window) = &mut *primary;
    stick.visible = window.focused
        && !pad.connected
        && controls::world_ready(&state, cover.as_deref())
        && !controls::movement_ui_open(script.as_deref());
    if !stick.visible {
        stick.offset = Vec2::ZERO;
    }
    if !window.focused {
        pointer.stick_contact = None;
    }
    let center = TouchStick::center(window);
    let click = |button, state| MouseButtonInput {
        button,
        state,
        window: *entity,
    };
    // Bevy consumes button edges once per frame. Keep a fast tap's press and
    // release in separate frames and preserve their positions for hit testing.
    pointer.queued.extend(
        touches
            .read()
            .filter(|touch| touch.window == *entity)
            .copied(),
    );
    while let Some(touch) = pointer.queued.pop_front() {
        if pointer.stick_contact == Some(touch.id) {
            match touch.phase {
                TouchPhase::Moved if stick.visible => stick.update(touch.position, center),
                TouchPhase::Ended | TouchPhase::Canceled => {
                    pointer.stick_contact = None;
                    stick.offset = Vec2::ZERO;
                }
                _ => (),
            }
            continue;
        }
        match touch.phase {
            TouchPhase::Started
                if stick.visible
                    && pointer.stick_contact.is_none()
                    && touch.position.distance_squared(center)
                        <= controls::STICK_RADIUS_SQUARED =>
            {
                pointer.stick_contact = Some(touch.id);
                stick.update(touch.position, center);
            }
            TouchPhase::Started if pointer.contact.is_none() => {
                let button = if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
                    MouseButton::Right
                } else {
                    MouseButton::Left
                };
                pointer.contact = Some((touch.id, button));
                window.set_cursor_position(Some(touch.position));
                clicks.write(click(button, ButtonState::Pressed));
                break;
            }
            TouchPhase::Moved if pointer.contact.is_some_and(|(id, _)| id == touch.id) => {
                if let Some(last) = window.cursor_position() {
                    motion.write(MouseMotion {
                        delta: touch.position - last,
                    });
                }
                window.set_cursor_position(Some(touch.position));
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                if let Some((_, button)) = pointer.contact.filter(|(id, _)| *id == touch.id) {
                    pointer.contact = None;
                    window.set_cursor_position(Some(touch.position));
                    clicks.write(click(button, ButtonState::Released));
                    break;
                }
            }
            _ => (),
        }
    }
}
