//! The plug-in window is the desktop app's window: the same [`App`](orb_ui::app::App)
//! panel, parented into the host's view. baseview owns the child view. A winit
//! event loop is not started here, because that replaces the host's application
//! delegate.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use baseview::{
    Event, EventStatus, MouseEvent, PhySize, Window, WindowEvent, WindowHandler, WindowOpenOptions,
};
use eframe::egui::{self, Pos2, Rect, pos2, vec2};
use keyboard_types::{Key, KeyState, KeyboardEvent, Modifiers};
use nih_plug::prelude::{GuiContext, ParamSetter, ParentWindowHandle};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::menu::MENU_H;
#[cfg(target_os = "macos")]
use raw_window_handle::HasRawWindowHandle;

/// Logical size of the app window: the panel plus the menu bar.
pub const W: f64 = PANEL_W as f64;
pub const H: f64 = (PANEL_H + MENU_H) as f64;

/// The view size the host last asked for, in logical pixels.
pub struct FrameSize {
    width: AtomicU32,
    height: AtomicU32,
    /// The child `NSView`, once the window exists. Zero before that.
    view: AtomicUsize,
}

impl FrameSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: AtomicU32::new(width),
            height: AtomicU32::new(height),
            view: AtomicUsize::new(0),
        }
    }

    pub fn get(&self) -> (u32, u32) {
        (
            self.width.load(Ordering::Relaxed),
            self.height.load(Ordering::Relaxed),
        )
    }

    pub fn set(&self, width: u32, height: u32) {
        self.width.store(width, Ordering::Relaxed);
        self.height.store(height, Ordering::Relaxed);
    }

    fn set_view(&self, view: usize) {
        self.view.store(view, Ordering::Relaxed);
    }

    fn view_ptr(&self) -> usize {
        self.view.load(Ordering::Relaxed)
    }

    /// Resize the child view to the size last stored. The host's window is
    /// already that size while `attached` is still running; the next frame is
    /// too late for the first paint.
    pub fn apply(&self) {
        let (width, height) = self.get();
        if width == 0 || height == 0 {
            return;
        }
        apply_view(self.view_ptr(), width, height);
    }
}

thread_local! {
    static OPEN: RefCell<Option<baseview::WindowHandle>> = const { RefCell::new(None) };
}

/// Dropped when the host closes the editor. The window handle is not `Send`,
/// so it stays on this thread and the flag tells a later frame to close.
pub struct Closer {
    flag: Arc<AtomicBool>,
}

impl Drop for Closer {
    fn drop(&mut self) {
        self.flag.store(true, Ordering::Release);
        OPEN.with(|slot| {
            if let Some(mut handle) = slot.borrow_mut().take() {
                handle.close();
            }
        });
    }
}

pub fn open(
    parent: &ParentWindowHandle,
    app: Arc<Mutex<App>>,
    frame: Arc<FrameSize>,
    params: Arc<crate::params::PlugParams>,
    gestures: Arc<crate::params::Gestures>,
    context: Arc<dyn GuiContext>,
) -> Closer {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_ui = flag.clone();
    OPEN.with(|slot| {
        if let Some(mut handle) = slot.borrow_mut().take() {
            handle.close();
        }
    });
    // A panic here crosses `IPlugView::attached` and aborts the host.
    let (width, height) = frame.get();
    let frame_ui = Arc::clone(&frame);
    let opened = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let handle = Window::open_parented(
            parent,
            WindowOpenOptions {
                title: "OpenRB-338".to_owned(),
                size: baseview::Size::new(f64::from(width), f64::from(height)),
                scale: baseview::WindowScalePolicy::SystemScaleFactor,
                gl_config: Some(baseview::gl::GlConfig::default()),
            },
            move |window| Host::new(window, app, flag_ui, frame_ui, params, gestures, context),
        );
        OPEN.with(|slot| *slot.borrow_mut() = Some(handle));
    }));
    if let Err(payload) = opened {
        eprintln!("OpenRB-338: editor: {}", panic_msg(&payload));
    } else {
        // The view is in the host window now. Size it before the window is shown.
        frame.apply();
    }
    Closer { flag }
}

fn panic_msg(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

struct Host {
    app: Arc<Mutex<App>>,
    params: Arc<crate::params::PlugParams>,
    gestures: Arc<crate::params::Gestures>,
    context: Arc<dyn GuiContext>,
    /// A host gesture is open for this parameter, so Ableton records the panel move.
    begun: Vec<bool>,
    frame: Arc<FrameSize>,
    /// Logical size last given to the child view.
    applied: (u32, u32),
    close: Arc<AtomicBool>,
    ctx: egui::Context,
    raw: egui::RawInput,
    start: Instant,
    painter: Option<egui_glow::Painter>,
    glow: Option<Arc<egui_glow::glow::Context>>,
    physical: PhySize,
    pixels_per_point: f32,
    pointer: Option<Pos2>,
    images: bool,
    mods: egui::Modifiers,
}

impl Host {
    fn new(
        window: &mut Window,
        app: Arc<Mutex<App>>,
        close: Arc<AtomicBool>,
        frame: Arc<FrameSize>,
        params: Arc<crate::params::PlugParams>,
        gestures: Arc<crate::params::Gestures>,
        context: Arc<dyn GuiContext>,
    ) -> Self {
        #[cfg(target_os = "macos")]
        if let raw_window_handle::RawWindowHandle::AppKit(handle) = window.raw_window_handle() {
            frame.set_view(handle.ns_view as usize);
        }
        let (width, height) = frame.get();
        // The GL view does not follow the NSView until this runs. Do it before
        // the first paint, not on the next frame.
        window.resize(baseview::Size::new(f64::from(width), f64::from(height)));
        let mut pixels_per_point = 1.0;
        let mut physical = PhySize::new(width, height);
        if let Some(screen) = view_screen(frame.view_ptr()) {
            pixels_per_point = screen.0;
            physical = screen.1;
        }
        let (glow, painter) = match window.gl_context() {
            Some(gl) => unsafe {
                // glow reads GL_VERSION while loading. Without a current context that
                // call panics, and the panic aborts the host at the VST boundary.
                gl.make_current();
                let glow = Arc::new(egui_glow::glow::Context::from_loader_function(|s| {
                    gl.get_proc_address(s)
                }));
                let painter = match egui_glow::Painter::new(glow.clone(), "", None, false) {
                    Ok(painter) => Some(painter),
                    Err(err) => {
                        eprintln!("OpenRB-338: OpenGL painter: {err}");
                        None
                    }
                };
                gl.make_not_current();
                (Some(glow), painter)
            },
            None => (None, None),
        };
        let screen = screen_rect(physical, 1.0 / pixels_per_point);
        let mut raw = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        raw.viewports.insert(
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                native_pixels_per_point: Some(pixels_per_point),
                inner_rect: Some(screen),
                focused: Some(true),
                ..Default::default()
            },
        );
        let applied = (width, height);
        let begun = vec![false; crate::params::initial_sent(&params).len()];
        Host {
            app,
            params,
            gestures,
            context,
            begun,
            frame,
            applied,
            close,
            ctx: egui::Context::default(),
            raw,
            start: Instant::now(),
            painter,
            glow,
            physical,
            pixels_per_point,
            pointer: None,
            images: false,
            mods: egui::Modifiers::default(),
        }
    }

    fn set_modifiers(&mut self, m: Modifiers) {
        self.mods.alt = m.contains(Modifiers::ALT);
        self.mods.shift = m.contains(Modifiers::SHIFT);
        self.mods.ctrl = m.contains(Modifiers::CONTROL);
        self.mods.mac_cmd = m.contains(Modifiers::META);
        self.mods.command = self.mods.ctrl || self.mods.mac_cmd;
    }

    fn paint(&mut self, window: &mut Window, full: &mut egui::FullOutput) {
        let Some(gl) = window.gl_context() else {
            return;
        };
        let Some(painter) = self.painter.as_mut() else {
            return;
        };
        let Some(glow) = self.glow.as_ref() else {
            return;
        };
        unsafe {
            gl.make_current();
        }
        unsafe {
            use egui_glow::glow::HasContext as _;
            glow.clear_color(0.05, 0.07, 0.06, 1.0);
            glow.clear(egui_glow::glow::COLOR_BUFFER_BIT);
        }
        for (id, images) in &full.textures_delta.set {
            for image in images {
                painter.set_texture(*id, image);
            }
        }
        let shapes = std::mem::take(&mut full.shapes);
        let clipped = self.ctx.tessellate(shapes, self.pixels_per_point);
        painter.paint_primitives(
            [self.physical.width, self.physical.height],
            self.pixels_per_point,
            &clipped,
        );
        for id in full.textures_delta.free.drain() {
            painter.free_texture(id);
        }
        gl.swap_buffers();
        unsafe {
            gl.make_not_current();
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(painter) = self.painter.as_mut() {
            painter.destroy();
        }
    }
}

impl WindowHandler for Host {
    fn on_frame(&mut self, window: &mut Window) {
        if self.close.load(Ordering::Acquire) {
            window.close();
            return;
        }
        // The frame timer is an `extern "C"` callback. A panic there aborts the host.
        let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.draw(window)));
        if let Err(payload) = drawn {
            eprintln!("OpenRB-338: frame: {}", panic_msg(&payload));
            window.close();
        }
    }

    fn on_event(&mut self, _window: &mut Window, event: Event) -> EventStatus {
        match event {
            Event::Mouse(ev) => self.on_mouse(ev),
            Event::Keyboard(ev) => self.on_key(ev),
            Event::Window(ev) => self.on_window(ev),
        }
        EventStatus::Captured
    }
}

impl Host {
    fn draw(&mut self, window: &mut Window) {
        let asked = self.frame.get();
        if asked != self.applied && asked.0 > 0 && asked.1 > 0 {
            window.resize(baseview::Size::new(f64::from(asked.0), f64::from(asked.1)));
            self.frame.apply();
            self.applied = asked;
        }
        if let Some(screen) = view_screen(self.frame.view_ptr()) {
            self.pixels_per_point = screen.0;
            self.physical = screen.1;
        } else if asked.0 > 0 && asked.1 > 0 {
            self.physical = PhySize::new(
                (asked.0 as f32 * self.pixels_per_point).round().max(1.0) as u32,
                (asked.1 as f32 * self.pixels_per_point).round().max(1.0) as u32,
            );
        }
        self.raw.time = Some(self.start.elapsed().as_secs_f64());
        let rect = screen_rect(self.physical, 1.0 / self.pixels_per_point);
        self.raw.screen_rect = Some(rect);
        if let Some(info) = self.raw.viewports.get_mut(&egui::ViewportId::ROOT) {
            info.inner_rect = Some(rect);
            info.native_pixels_per_point = Some(self.pixels_per_point);
        }
        if !self.images {
            egui_extras::install_image_loaders(&self.ctx);
            self.ctx.set_theme(egui::ThemePreference::Dark);
            self.images = true;
        }
        let app = Arc::clone(&self.app);
        let mut full = self.ctx.run_ui(self.raw.take(), |ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    if let Ok(mut panel) = app.lock() {
                        panel.show(ui);
                        let held = panel.panel.held.clone();
                        self.gestures.set_held(held.as_deref());
                        if let Some(link) = panel.link.as_mut() {
                            let snap = *link.player().snapshot();
                            let setter = ParamSetter::new(self.context.as_ref());
                            self.params.follow(&setter, &snap, &mut self.begun);
                        }
                        // The project save may run while this frame holds the panel.
                        self.params.song.capture(&panel);
                    }
                });
        });
        self.paint(window, &mut full);
    }

    fn on_mouse(&mut self, ev: MouseEvent) {
        match ev {
            MouseEvent::CursorMoved {
                position,
                modifiers,
            } => {
                self.set_modifiers(modifiers);
                let p = pos2(position.x as f32, position.y as f32);
                self.pointer = Some(p);
                self.raw.events.push(egui::Event::PointerMoved(p));
            }
            MouseEvent::ButtonPressed { button, modifiers } => {
                self.button(button, modifiers, true);
            }
            MouseEvent::ButtonReleased { button, modifiers } => {
                self.button(button, modifiers, false);
            }
            MouseEvent::WheelScrolled { delta, modifiers } => {
                self.set_modifiers(modifiers);
                let (unit, delta) = match delta {
                    baseview::ScrollDelta::Lines { x, y } => {
                        (egui::MouseWheelUnit::Line, vec2(x, y))
                    }
                    baseview::ScrollDelta::Pixels { x, y } => (
                        egui::MouseWheelUnit::Point,
                        vec2(x, y) / self.pixels_per_point,
                    ),
                };
                self.raw.events.push(egui::Event::MouseWheel {
                    unit,
                    delta,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: self.mods,
                });
            }
            MouseEvent::CursorLeft => {
                self.pointer = None;
                self.raw.events.push(egui::Event::PointerGone);
            }
            _ => {}
        }
    }

    fn button(&mut self, button: baseview::MouseButton, modifiers: Modifiers, pressed: bool) {
        self.set_modifiers(modifiers);
        let Some(pos) = self.pointer else {
            return;
        };
        let Some(button) = (match button {
            baseview::MouseButton::Left => Some(egui::PointerButton::Primary),
            baseview::MouseButton::Right => Some(egui::PointerButton::Secondary),
            baseview::MouseButton::Middle => Some(egui::PointerButton::Middle),
            _ => None,
        }) else {
            return;
        };
        self.raw.events.push(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: self.mods,
        });
    }

    fn on_key(&mut self, ev: KeyboardEvent) {
        let pressed = ev.state == KeyState::Down;
        match ev.code {
            keyboard_types::Code::ShiftLeft | keyboard_types::Code::ShiftRight => {
                self.mods.shift = pressed;
            }
            keyboard_types::Code::ControlLeft | keyboard_types::Code::ControlRight => {
                self.mods.ctrl = pressed;
            }
            keyboard_types::Code::AltLeft | keyboard_types::Code::AltRight => {
                self.mods.alt = pressed;
            }
            keyboard_types::Code::MetaLeft | keyboard_types::Code::MetaRight => {
                self.mods.mac_cmd = pressed;
            }
            _ => {}
        }
        self.mods.command = self.mods.ctrl || self.mods.mac_cmd;
        if let Some(key) = egui_key(&ev.key) {
            self.raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: ev.repeat,
                modifiers: self.mods,
            });
        }
        if pressed
            && !self.mods.ctrl
            && !self.mods.command
            && let Key::Character(text) = &ev.key
        {
            self.raw.events.push(egui::Event::Text(text.clone()));
        }
    }

    fn on_window(&mut self, ev: WindowEvent) {
        match ev {
            WindowEvent::Resized(info) => {
                self.pixels_per_point = info.scale() as f32;
                if self.pixels_per_point <= 0.0 {
                    self.pixels_per_point = 1.0;
                }
                self.physical = info.physical_size();
            }
            WindowEvent::Focused => {
                self.raw.events.push(egui::Event::WindowFocused(true));
            }
            WindowEvent::Unfocused => {
                self.raw.events.push(egui::Event::WindowFocused(false));
            }
            WindowEvent::WillClose => {}
        }
    }
}

fn apply_view(view: usize, width: u32, height: u32) {
    #[cfg(target_os = "macos")]
    apply_view_mac(view, width, height);
    #[cfg(not(target_os = "macos"))]
    let _ = (view, width, height);
}

/// Logical scale and the view's physical pixel size, read from the live view.
fn view_screen(view: usize) -> Option<(f32, PhySize)> {
    #[cfg(target_os = "macos")]
    {
        view_screen_mac(view)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = view;
        None
    }
}

#[cfg(target_os = "macos")]
fn apply_view_mac(view: usize, width: u32, height: u32) {
    if view == 0 {
        return;
    }
    unsafe {
        use cocoa::appkit::NSView;
        let view = view as cocoa::base::id;
        let size = cocoa::foundation::NSSize::new(f64::from(width), f64::from(height));
        view.setFrameSize(size);
        resize_subviews(view, size);
    }
}

/// The OpenGL view is a subview. `Window::resize` sizes it too; this covers the
/// resize that arrives from the host before the next frame.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn resize_subviews(view: cocoa::base::id, size: cocoa::foundation::NSSize) {
    use cocoa::appkit::NSView;
    use objc::{sel, sel_impl};
    unsafe {
        let subviews: cocoa::base::id = objc::msg_send![view, subviews];
        if subviews == cocoa::base::nil {
            return;
        }
        let count: usize = objc::msg_send![subviews, count];
        for i in 0..count {
            let sub: cocoa::base::id = objc::msg_send![subviews, objectAtIndex: i];
            sub.setFrameSize(size);
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn view_screen_mac(view: usize) -> Option<(f32, PhySize)> {
    if view == 0 {
        return None;
    }
    use objc::{sel, sel_impl};
    unsafe {
        use cocoa::appkit::{NSView, NSWindow};
        let view = view as cocoa::base::id;
        let window: cocoa::base::id = objc::msg_send![view, window];
        let mut scale = if window == cocoa::base::nil {
            1.0
        } else {
            window.backingScaleFactor()
        };
        if scale <= 0.0 {
            scale = 1.0;
        }
        let bounds = view.bounds();
        if bounds.size.width < 1.0 || bounds.size.height < 1.0 {
            return None;
        }
        Some((
            scale as f32,
            PhySize::new(
                (bounds.size.width * scale).round().max(1.0) as u32,
                (bounds.size.height * scale).round().max(1.0) as u32,
            ),
        ))
    }
}

fn screen_rect(physical: PhySize, points_per_pixel: f32) -> Rect {
    Rect::from_min_size(
        Pos2::ZERO,
        vec2(
            physical.width as f32 * points_per_pixel,
            physical.height as f32 * points_per_pixel,
        ),
    )
}

fn egui_key(key: &Key) -> Option<egui::Key> {
    use egui::Key as K;
    Some(match key {
        Key::ArrowDown => K::ArrowDown,
        Key::ArrowLeft => K::ArrowLeft,
        Key::ArrowRight => K::ArrowRight,
        Key::ArrowUp => K::ArrowUp,
        Key::Escape => K::Escape,
        Key::Tab => K::Tab,
        Key::Backspace => K::Backspace,
        Key::Enter => K::Enter,
        Key::Insert => K::Insert,
        Key::Delete => K::Delete,
        Key::Home => K::Home,
        Key::End => K::End,
        Key::PageUp => K::PageUp,
        Key::PageDown => K::PageDown,
        Key::Character(s) => match s.chars().next()? {
            ' ' => K::Space,
            '0' => K::Num0,
            '1' => K::Num1,
            '2' => K::Num2,
            '3' => K::Num3,
            '4' => K::Num4,
            '5' => K::Num5,
            '6' => K::Num6,
            '7' => K::Num7,
            '8' => K::Num8,
            '9' => K::Num9,
            'a' | 'A' => K::A,
            'b' | 'B' => K::B,
            'c' | 'C' => K::C,
            'd' | 'D' => K::D,
            'e' | 'E' => K::E,
            'f' | 'F' => K::F,
            'g' | 'G' => K::G,
            'h' | 'H' => K::H,
            'i' | 'I' => K::I,
            'j' | 'J' => K::J,
            'k' | 'K' => K::K,
            'l' | 'L' => K::L,
            'm' | 'M' => K::M,
            'n' | 'N' => K::N,
            'o' | 'O' => K::O,
            'p' | 'P' => K::P,
            'q' | 'Q' => K::Q,
            'r' | 'R' => K::R,
            's' | 'S' => K::S,
            't' | 'T' => K::T,
            'u' | 'U' => K::U,
            'v' | 'V' => K::V,
            'w' | 'W' => K::W,
            'x' | 'X' => K::X,
            'y' | 'Y' => K::Y,
            'z' | 'Z' => K::Z,
            _ => return None,
        },
        _ => return None,
    })
}
