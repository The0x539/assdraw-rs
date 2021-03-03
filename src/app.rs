use std::cell::{Cell, RefCell};
use std::rc::Rc;

use byte_set::ByteSet;
use once_cell::unsync::OnceCell;

use native_windows_gui as nwg;
use nwg::Event;

type Canvas = crate::gl::OpenGlCanvas;
use crate::drawing::{Command, CommandKind};
use crate::point::Point;

fn change_scale(mut scale: f32, factor: i32) -> f32 {
    assert!(scale > 0.0);
    if scale < 1.0 {
        scale = scale.recip().round();
        scale = -scale + 2.0;
    }

    scale += factor as f32;

    if scale < 1.0 {
        scale = (scale - 2.0).abs();
        scale = scale.recip();
    }
    scale
}

#[derive(Default)]
pub struct AppBuilder;

pub struct AppInner {
    window: nwg::Window,
    pub canvas: OnceCell<Canvas>,
    canvas_handler: OnceCell<nwg::EventHandler>,
    paste_image_btn: nwg::Button,
    clear_drawing_btn: nwg::Button,
    copy_drawing_btn: nwg::Button,
    drawing_color_btn: nwg::Button,
    shape_color_btn: nwg::Button,
    shape_alpha_slider: nwg::TrackBar,
    move_mode_btn: nwg::RadioButton,
    line_mode_btn: nwg::RadioButton,
    bezier_mode_btn: nwg::RadioButton,
    color_dialog: nwg::ColorDialog,

    left_dragging: Cell<bool>,
    right_dragging: Cell<bool>,
    dragged_point: Cell<Option<usize>>,
    pre_drag_pos: Cell<Point<f32>>,
    drag_start_pos: Cell<Point<i32>>,
    draw_mode: Cell<CommandKind>,
    keys: RefCell<Keys>,
}

#[derive(Debug, Default, Copy, Clone)]
struct Keys {
    keys: ByteSet,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum KeyState {
    Pressed,
    Released,
    Up,
    Down,
}

impl Keys {
    pub fn update(&mut self, ev: nwg::Event, key: u32) -> KeyState {
        let key = key as u8;
        let old = self.keys.contains(key);
        match ev {
            nwg::Event::OnKeyPress => self.keys.insert(key),
            nwg::Event::OnKeyRelease => self.keys.remove(key),
            _ => (),
        }
        match (old, self.keys.contains(key)) {
            (false, true) => KeyState::Pressed,
            (true, false) => KeyState::Released,
            (false, false) => KeyState::Up,
            (true, true) => KeyState::Down,
        }
    }

    pub fn pressed(&self, key: u32) -> bool {
        self.keys.contains(key as u8)
    }
}

impl AppInner {
    fn get_canvas(&self) -> &Canvas {
        self.canvas.get().unwrap()
    }

    fn cursor_pos(&self) -> Point<i32> {
        nwg::GlobalCursor::local_position(self.get_canvas().handle(), None).into()
    }

    fn is_dragging(&self) -> bool {
        self.left_dragging.get() || self.right_dragging.get()
    }

    fn get_point_at_cursor(&self) -> Point<f32> {
        let dims = self.get_canvas().get_dimensions();
        let cursor_pos = self.cursor_pos().cast::<f32>();
        dims.scene_pos + (cursor_pos / dims.scale)
    }

    fn add_point_at_cursor(&self) {
        let point = self.get_point_at_cursor();
        self.get_canvas().with_drawing(|drawing| {
            let cmd = if drawing.points().is_empty() {
                Command::Move(point)
            } else {
                match self.draw_mode.get() {
                    CommandKind::Move => Command::Move(point),
                    CommandKind::Line => Command::Line(point),
                    CommandKind::Bezier => {
                        let p0 = *drawing.points().last().unwrap();
                        let p3 = point;
                        let p1 = p0.lerp(p3, 0.3333);
                        let p2 = p0.lerp(p3, 0.6667);
                        Command::Bezier(p1, p2, p3)
                    }
                }
            };
            drawing.push(cmd);
        });
    }

    fn clear_drawing(&self) {
        self.get_canvas().clear_drawing();
    }

    fn copy_drawing(&self) -> std::fmt::Result {
        let text = self.get_canvas().with_drawing(|drawing| {
            let mut data = Vec::new();
            let mut last_kind = None;
            for cmd in drawing.commands() {
                let element = match cmd {
                    Command::Move(p) | Command::Line(p) if last_kind == Some(cmd.kind()) => {
                        format!("{} {}", p.x, p.y)
                    }
                    Command::Move(p) => format!("m {} {}", p.x, p.y),
                    Command::Line(p) => format!("l {} {}", p.x, p.y),
                    Command::Bezier(p1, p2, p3) => {
                        if last_kind == Some(CommandKind::Bezier) {
                            format!("{} {} {} {} {} {}", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)
                        } else {
                            format!("b {} {} {} {} {} {}", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)
                        }
                    }
                };
                last_kind = Some(cmd.kind());
                data.push(element);
            }
            data.join(" ")
        });
        clipboard_win::set_clipboard_string(&text).unwrap_or((/* ignore */));
        Ok(())
    }

    fn show(self: Rc<Self>) {
        self.window.set_visible(true);
        self.window.set_focus();

        let canvas = Canvas::new(&self.window);

        let ui = Rc::downgrade(&self);

        let f = move |evt, evt_data, handle| {
            let ui = ui.upgrade().unwrap();
            if &handle != ui.canvas.get().unwrap().handle() {
                return;
            }
            match evt {
                Event::OnMouseMove => ui.mouse_move(),
                Event::OnMouseWheel => ui.zoom(evt_data),
                Event::OnMousePress(mouse_evt) => ui.mouse_press(mouse_evt),
                _ => (),
            }
        };

        let handler = nwg::full_bind_event_handler(&self.window.handle, f);
        self.canvas_handler
            .set(handler)
            .ok()
            .expect("canvas event handler was already initialized");

        self.canvas
            .set(canvas)
            .ok()
            .expect("canvas was already initialized");

        self.handle_resize();
    }

    fn handle_resize(&self) {
        if let Some(canvas) = self.canvas.get() {
            let (x, y) = self.window.size();
            canvas.nwg_canvas().set_size(x - 101, y - 1);
            canvas.resize();
            canvas.render();
        }
    }
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
    fn zoom(&self, data: nwg::EventData) {
        let factor = match data {
            nwg::EventData::OnMouseWheel(i) => i / 120,
            _ => panic!(),
        };
        self.get_canvas().update_dimensions(|dims| {
            // this is the same code as get_point_at_cursor
            // TODO: figure out how to avoid RefCell rules preventing the use of that function here
            let mouse_pos = self.cursor_pos().cast::<f32>();
            let mouse_scene_pos = dims.scene_pos + (mouse_pos / dims.scale);

            let new_scale = change_scale(dims.scale, factor);

            let new_scene_pos = mouse_scene_pos - (mouse_pos / new_scale);

            if self.right_dragging.get() {
                self.pre_drag_pos.set(new_scene_pos);
                self.drag_start_pos.set(self.cursor_pos());
            }

            dims.scale = new_scale;
            dims.scene_pos = new_scene_pos;
        })
    }
    fn mouse_move(&self) {
        if !self.is_dragging() {
            return;
        }

        let xy0 = self.pre_drag_pos.get();
        let dxy0 = self.drag_start_pos.get();
        let dxy1 = self.cursor_pos();
        let dxy = dxy1 - dxy0;

        if self.right_dragging.get() {
            self.get_canvas().update_dimensions(|dims| {
                dims.scene_pos = xy0 - (dxy.cast::<f32>() / dims.scale);
            })
        }
        if self.left_dragging.get() {
            if let Some(i) = self.dragged_point.get() {
                self.get_canvas()
                    .with_drawing(|drawing| drawing.points_mut()[i] = self.get_point_at_cursor());
            }
        }
    }
    fn mouse_press(&self, event: nwg::MousePressEvent) {
        let was_dragging = self.is_dragging();
        match event {
            nwg::MousePressEvent::MousePressRightDown => {
                self.right_dragging.set(true);
            }
            nwg::MousePressEvent::MousePressRightUp => {
                self.right_dragging.set(false);
            }
            nwg::MousePressEvent::MousePressLeftDown => {
                let mut drag_idx = None;
                let cursor_pos = self.get_point_at_cursor();
                let canvas = self.get_canvas();
                let scale = canvas.get_dimensions().scale;
                canvas.with_drawing(|drawing| {
                    for (i, point) in drawing.points().iter().enumerate() {
                        let dx = cursor_pos.x - point.x;
                        let dy = cursor_pos.y - point.y;
                        if f32::max(dx.abs(), dy.abs()) <= 5.0 / scale {
                            drag_idx = Some(i);
                            break;
                        }
                    }
                });
                if drag_idx.is_none() {
                    self.add_point_at_cursor();
                    drag_idx = Some(canvas.with_drawing(|d| d.points().len() - 1));
                }
                self.dragged_point.set(drag_idx);

                self.left_dragging.set(true);
            }
            nwg::MousePressEvent::MousePressLeftUp => {
                self.left_dragging.set(false);
            }
        }
        match (was_dragging, self.is_dragging()) {
            (false, true) => {
                nwg::GlobalCursor::set_capture(self.get_canvas().handle());
                self.drag_start_pos.set(self.cursor_pos());
                self.pre_drag_pos
                    .set(self.get_canvas().get_dimensions().scene_pos);
            }
            (true, false) => {
                nwg::GlobalCursor::release();
                if self.dragged_point.take().is_some() {
                    self.get_canvas().commit_drawing();
                }
            }
            _ => (),
        }
    }
    fn paste_image(&self) {
        let buf = match clipboard_win::get_clipboard(clipboard_win::formats::Bitmap) {
            Ok(buf) => buf,
            e => {
                println!("{:?}", e);
                return;
            }
        };
        let cursor = std::io::Cursor::new(&buf[..]);
        let img = image::codecs::bmp::BmpDecoder::new(cursor).unwrap();
        self.get_canvas().set_image(img);
    }

    fn choose_color(&self, for_drawing: bool) {
        if !self.color_dialog.run(Some(&self.window)) {
            return;
        }
        let rgb = self.color_dialog.color();
        if for_drawing {
            self.get_canvas().recolor_drawing(rgb);
        } else {
            self.get_canvas().recolor_shape(rgb);
        }
    }

    fn update_shape_alpha(&self) {
        self.get_canvas()
            .set_shape_alpha(self.shape_alpha_slider.pos() as u8);
    }
}

pub struct App {
    inner: Rc<AppInner>,
    handler: nwg::EventHandler,
}

impl nwg::NativeUi<App> for AppBuilder {
    fn build_ui(_data: Self) -> Result<App, nwg::NwgError> {
        let mut window = Default::default();
        nwg::Window::builder()
            .size((600, 500))
            .position((300, 300))
            .title("nwg")
            .flags(nwg::WindowFlags::MAIN_WINDOW)
            .build(&mut window)?;

        // we'll initialize this later, eh?
        let canvas = OnceCell::new();
        /*
        nwg::ExternCanvas::builder()
            .parent(Some(&window))
            .build(&mut *canvas)?;
        */

        let make_button = |text, x, y| {
            let mut btn = Default::default();
            nwg::Button::builder()
                .parent(&window)
                .text(text)
                .position((x, y))
                .build(&mut btn)?;
            Ok(btn)
        };

        let paste_image_btn = make_button("bg", 0, 0)?;
        let clear_drawing_btn = make_button("clear", 0, 25)?;
        let copy_drawing_btn = make_button("copy", 0, 50)?;
        let drawing_color_btn = make_button("drawing color", 0, 75)?;
        let shape_color_btn = make_button("shape color", 0, 100)?;

        let make_radio_button = |text, x, y| {
            let mut btn = Default::default();
            nwg::RadioButton::builder()
                .parent(&window)
                .text(text)
                .position((x, y))
                .build(&mut btn)?;
            Ok(btn)
        };

        let move_mode_btn = make_radio_button("move", 0, 150)?;
        let line_mode_btn = make_radio_button("line", 0, 175)?;
        let bezier_mode_btn = make_radio_button("bezier", 0, 200)?;

        let mut shape_alpha_slider = Default::default();
        nwg::TrackBar::builder()
            .parent(&window)
            .position((0, 125))
            .build(&mut shape_alpha_slider)?;
        shape_alpha_slider.set_pos(50);

        let mut color_dialog = Default::default();
        nwg::ColorDialog::builder().build(&mut color_dialog)?;

        let inner = Rc::new(AppInner {
            window,
            canvas,
            canvas_handler: OnceCell::new(),
            paste_image_btn,
            clear_drawing_btn,
            copy_drawing_btn,
            drawing_color_btn,
            shape_color_btn,
            shape_alpha_slider,
            move_mode_btn,
            line_mode_btn,
            bezier_mode_btn,
            color_dialog,

            left_dragging: Default::default(),
            right_dragging: Default::default(),
            dragged_point: Default::default(),
            pre_drag_pos: Default::default(),
            drag_start_pos: Default::default(),
            draw_mode: Cell::new(CommandKind::Line),
            keys: Default::default(),
        });

        let ui = Rc::downgrade(&inner);
        let handle_fn = move |evt, evt_data: nwg::EventData, handle| {
            let ui = ui.upgrade().unwrap();
            if matches!(evt, Event::OnKeyPress | Event::OnKeyRelease) {
                ui.window.set_focus();
            }
            if handle == ui.window.handle {
                match evt {
                    Event::OnInit => AppInner::show(ui),
                    Event::OnResize | Event::OnWindowMaximize | Event::OnResizeEnd => {
                        ui.handle_resize()
                    }
                    Event::OnWindowClose => ui.exit(),
                    Event::OnKeyPress | Event::OnKeyRelease => {
                        let key = evt_data.on_key();
                        let mut keys = ui.keys.borrow_mut();
                        let state = keys.update(evt, key);
                        if keys.pressed(nwg::keys::CONTROL) && state == KeyState::Pressed {
                            match key {
                                nwg::keys::_Z => {
                                    ui.dragged_point.take();
                                    if keys.pressed(nwg::keys::SHIFT) {
                                        ui.get_canvas().redo();
                                    } else {
                                        ui.get_canvas().undo();
                                    }
                                }
                                nwg::keys::_Y => ui.get_canvas().redo(),
                                nwg::keys::_C => ui.copy_drawing().unwrap(),
                                nwg::keys::_V => {
                                    // TODO: paste either a drawing or the image depending on cb
                                    // careful: pasting the image should be a history entry
                                    ui.paste_image();
                                }
                                _ => (),
                            }
                        }
                    }
                    Event::OnMinMaxInfo => {
                        let min_max_info = evt_data.on_min_max();
                        min_max_info.set_min_size(200, 200);
                    }
                    _ => (),
                }
            } else if evt == Event::OnButtonClick {
                if handle == ui.paste_image_btn {
                    ui.paste_image();
                } else if handle == ui.clear_drawing_btn {
                    ui.clear_drawing();
                } else if handle == ui.copy_drawing_btn {
                    ui.copy_drawing().unwrap();
                } else if handle == ui.drawing_color_btn {
                    ui.choose_color(true);
                } else if handle == ui.shape_color_btn {
                    ui.choose_color(false);
                } else if handle == ui.move_mode_btn {
                    ui.draw_mode.set(CommandKind::Move);
                } else if handle == ui.line_mode_btn {
                    ui.draw_mode.set(CommandKind::Line);
                } else if handle == ui.bezier_mode_btn {
                    ui.draw_mode.set(CommandKind::Bezier);
                }
            } else if evt == Event::OnHorizontalScroll {
                if handle == ui.shape_alpha_slider {
                    ui.update_shape_alpha();
                }
            }
        };
        let handler = nwg::full_bind_event_handler(&inner.window.handle, handle_fn);

        Ok(App { inner, handler })
    }
}

impl std::ops::Drop for App {
    fn drop(&mut self) {
        nwg::unbind_event_handler(&self.handler);
        if let Some(canvas_handler) = self.inner.canvas_handler.get() {
            nwg::unbind_event_handler(canvas_handler);
        }
    }
}

impl std::ops::Deref for App {
    type Target = AppInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
