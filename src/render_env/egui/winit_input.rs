use egui::{Key, RawInput};
use egui::math::{pos2, vec2};
use winit::event::{ModifiersState, VirtualKeyCode, WindowEvent};

pub(crate) struct WinitInput {
    scale_factor: f64,
    pub(super) raw_input: egui::RawInput,

    mouse_pos: egui::Pos2,
    modifiers_state: ModifiersState,
}

impl WinitInput {
    pub fn new(init_input: RawInput) -> WinitInput {
        WinitInput {
            scale_factor: 1.0,
            raw_input: init_input,
            mouse_pos: egui::Pos2::new(0.0, 0.0),
            modifiers_state: ModifiersState::default(),
        }
    }

    pub fn handle_event(&mut self, context: egui::CtxRef, window_event: &WindowEvent) {
        match window_event {
            // window size changed
            WindowEvent::Resized(physical_size) => {
                let pixels_per_point = self
                    .raw_input
                    .pixels_per_point
                    .unwrap_or_else(|| context.pixels_per_point());
                self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
                    Default::default(),
                    vec2(physical_size.width as f32, physical_size.height as f32)
                        / pixels_per_point,
                ));
            }
            // dpi changed
            WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
                self.scale_factor = *scale_factor;
                self.raw_input.pixels_per_point = Some(*scale_factor as f32);
                let pixels_per_point = self
                    .raw_input
                    .pixels_per_point
                    .unwrap_or_else(|| context.pixels_per_point());
                self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
                    Default::default(),
                    vec2(new_inner_size.width as f32, new_inner_size.height as f32)
                        / pixels_per_point,
                ));
            }
            // mouse click
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = winit_to_egui_mouse_button(*button) {
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos: self.mouse_pos,
                        button,
                        pressed: *state == winit::event::ElementState::Pressed,
                        modifiers: winit_to_egui_modifiers(self.modifiers_state),
                    });
                }
            }
            // mouse wheel
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    let line_height = 24.0;
                    self.raw_input.scroll_delta = vec2(*x, *y) * line_height;
                }
                winit::event::MouseScrollDelta::PixelDelta(delta) => {
                    self.raw_input.scroll_delta = vec2(delta.x as f32, delta.y as f32);
                }
            },
            // mouse move
            WindowEvent::CursorMoved { position, .. } => {
                let pixels_per_point = self
                    .raw_input
                    .pixels_per_point
                    .unwrap_or_else(|| context.pixels_per_point());
                let pos = pos2(
                    position.x as f32 / pixels_per_point,
                    position.y as f32 / pixels_per_point,
                );
                self.raw_input.events.push(egui::Event::PointerMoved(pos));
                self.mouse_pos = pos;
            }
            // mouse out
            WindowEvent::CursorLeft { .. } => {
                self.raw_input.events.push(egui::Event::PointerGone);
            }
            // modifier keys
            WindowEvent::ModifiersChanged(input) => self.modifiers_state = *input,
            // keyboard inputs
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(virtual_keycode) = input.virtual_keycode {
                    let pressed = input.state == winit::event::ElementState::Pressed;
                    if pressed {
                        if let Some(key) = winit_to_egui_key_code(virtual_keycode)
                        {
                            self.raw_input.events.push(egui::Event::Key {
                                key,
                                pressed: input.state == winit::event::ElementState::Pressed,
                                modifiers: winit_to_egui_modifiers(self.modifiers_state),
                            })
                        }
                    }
                }
            }
            // receive character
            WindowEvent::ReceivedCharacter(ch) => {
                // remove control character
                if ch.is_ascii_control() {
                    return;
                }
                self.raw_input
                    .events
                    .push(egui::Event::Text(ch.to_string()));
            }
            _ => (),
        }
    }
}


fn winit_to_egui_key_code(key: VirtualKeyCode) -> Option<egui::Key> {
    Some(match key {
        VirtualKeyCode::Down => Key::ArrowDown,
        VirtualKeyCode::Left => Key::ArrowLeft,
        VirtualKeyCode::Right => Key::ArrowRight,
        VirtualKeyCode::Up => Key::ArrowUp,
        VirtualKeyCode::Escape => Key::Escape,
        VirtualKeyCode::Tab => Key::Tab,
        VirtualKeyCode::Back => Key::Backspace,
        VirtualKeyCode::Return => Key::Enter,
        VirtualKeyCode::Space => Key::Space,
        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::PageDown => Key::PageDown,
        VirtualKeyCode::Key0 => Key::Num0,
        VirtualKeyCode::Key1 => Key::Num1,
        VirtualKeyCode::Key2 => Key::Num2,
        VirtualKeyCode::Key3 => Key::Num3,
        VirtualKeyCode::Key4 => Key::Num4,
        VirtualKeyCode::Key5 => Key::Num5,
        VirtualKeyCode::Key6 => Key::Num6,
        VirtualKeyCode::Key7 => Key::Num7,
        VirtualKeyCode::Key8 => Key::Num8,
        VirtualKeyCode::Key9 => Key::Num9,
        VirtualKeyCode::A => Key::A,
        VirtualKeyCode::B => Key::B,
        VirtualKeyCode::C => Key::C,
        VirtualKeyCode::D => Key::D,
        VirtualKeyCode::E => Key::E,
        VirtualKeyCode::F => Key::F,
        VirtualKeyCode::G => Key::G,
        VirtualKeyCode::H => Key::H,
        VirtualKeyCode::I => Key::I,
        VirtualKeyCode::J => Key::J,
        VirtualKeyCode::K => Key::K,
        VirtualKeyCode::L => Key::L,
        VirtualKeyCode::M => Key::M,
        VirtualKeyCode::N => Key::N,
        VirtualKeyCode::O => Key::O,
        VirtualKeyCode::P => Key::P,
        VirtualKeyCode::Q => Key::Q,
        VirtualKeyCode::R => Key::R,
        VirtualKeyCode::S => Key::S,
        VirtualKeyCode::T => Key::T,
        VirtualKeyCode::U => Key::U,
        VirtualKeyCode::V => Key::V,
        VirtualKeyCode::W => Key::W,
        VirtualKeyCode::X => Key::X,
        VirtualKeyCode::Y => Key::Y,
        VirtualKeyCode::Z => Key::Z,
        _ => return None,
    })
}

fn winit_to_egui_modifiers(modifiers: ModifiersState) -> egui::Modifiers {
    #[cfg(target_os = "macos")]
        let mac_cmd = modifiers.logo();
    #[cfg(target_os = "macos")]
        let command = modifiers.logo();
    #[cfg(not(target_os = "macos"))]
        let mac_cmd = false;
    #[cfg(not(target_os = "macos"))]
        let command = modifiers.ctrl();

    egui::Modifiers {
        alt: modifiers.alt(),
        ctrl: modifiers.ctrl(),
        shift: modifiers.shift(),
        mac_cmd,
        command,
    }
}

fn winit_to_egui_mouse_button(
    button: winit::event::MouseButton,
) -> Option<egui::PointerButton> {
    Some(match button {
        winit::event::MouseButton::Left => egui::PointerButton::Primary,
        winit::event::MouseButton::Right => egui::PointerButton::Secondary,
        winit::event::MouseButton::Middle => egui::PointerButton::Middle,
        _ => return None,
    })
}

/// Convert from [`egui::CursorIcon`] to [`winit::window::CursorIcon`].
pub fn egui_to_winit_cursor_icon(
    cursor_icon: egui::CursorIcon,
) -> Option<winit::window::CursorIcon> {
    Some(match cursor_icon {
        egui::CursorIcon::Default => winit::window::CursorIcon::Default,
        egui::CursorIcon::PointingHand => winit::window::CursorIcon::Hand,
        egui::CursorIcon::ResizeHorizontal => winit::window::CursorIcon::ColResize,
        egui::CursorIcon::ResizeNeSw => winit::window::CursorIcon::NeResize,
        egui::CursorIcon::ResizeNwSe => winit::window::CursorIcon::NwResize,
        egui::CursorIcon::ResizeVertical => winit::window::CursorIcon::RowResize,
        egui::CursorIcon::Text => winit::window::CursorIcon::Text,
        egui::CursorIcon::Grab => winit::window::CursorIcon::Grab,
        egui::CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
        egui::CursorIcon::None => return None,
        egui::CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
        egui::CursorIcon::Help => winit::window::CursorIcon::Help,
        egui::CursorIcon::Progress => winit::window::CursorIcon::Progress,
        egui::CursorIcon::Wait => winit::window::CursorIcon::Wait,
        egui::CursorIcon::Cell => winit::window::CursorIcon::Cell,
        egui::CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        egui::CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
        egui::CursorIcon::Alias => winit::window::CursorIcon::Alias,
        egui::CursorIcon::Copy => winit::window::CursorIcon::Copy,
        egui::CursorIcon::Move => winit::window::CursorIcon::Move,
        egui::CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
        egui::CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
        egui::CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
        egui::CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
        egui::CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
    })
}
