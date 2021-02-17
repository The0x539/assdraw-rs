use std::cell::Cell;
use std::fmt::Write;
use std::rc::Rc;

use once_cell::unsync::OnceCell;

use native_windows_gui as nwg;
use nwg::Event;

type Canvas = crate::gl::OpenGlCanvas;
use crate::gl::Point;

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
    grid: nwg::GridLayout,
    pub canvas: OnceCell<Canvas>,
    canvas_handler: OnceCell<nwg::EventHandler>,
    paste_image_btn: nwg::Button,
    clear_drawing_btn: nwg::Button,
    copy_drawing_btn: nwg::Button,
    drawing_color_btn: nwg::Button,
    shape_color_btn: nwg::Button,
    shape_alpha_slider: nwg::TrackBar,
    color_dialog: nwg::ColorDialog,

    left_dragging: Cell<bool>,
    right_dragging: Cell<bool>,
    dragged_point: Cell<Option<usize>>,
    pre_drag_pos: Cell<Point>,
    drag_start_pos: Cell<(i32, i32)>,
}

impl AppInner {
    fn get_canvas(&self) -> &Canvas {
        self.canvas.get().unwrap()
    }

    fn cursor_pos(&self) -> (i32, i32) {
        nwg::GlobalCursor::local_position(self.get_canvas().handle(), None)
    }

    fn is_dragging(&self) -> bool {
        self.left_dragging.get() || self.right_dragging.get()
    }

    fn get_point_at_cursor(&self) -> Point {
        let (x, y) = self.cursor_pos();
        let dims = self.get_canvas().get_dimensions();
        Point {
            x: dims.scene_pos.x + x as f32 / dims.scale,
            y: dims.scene_pos.y + y as f32 / dims.scale,
        }
    }

    fn add_point_at_cursor(&self) {
        let point = self.get_point_at_cursor();
        self.get_canvas().add_point(point);
    }

    fn clear_drawing(&self) {
        self.get_canvas().clear_drawing();
    }

    fn copy_drawing(&self) -> std::fmt::Result {
        let points = self.get_canvas().drawing_points();
        if points.len() == 0 {
            return Ok(());
        }
        let mut data = format!("m {} {}", points[0].x, points[0].y);
        if points.len() > 2 {
            write!(data, " l")?;
            for xy in &points[1..] {
                write!(data, " {} {}", xy.x, xy.y)?;
            }
        }
        clipboard_win::set_clipboard_string(&data).unwrap_or((/* ignore */));
        Ok(())
    }

    fn show(self: Rc<Self>) {
        self.window.set_visible(true);
        self.window.set_focus();

        let canvas = Canvas::new(&self.window);

        self.grid
            .add_child_item(nwg::GridLayoutItem::new(canvas.handle(), 0, 0, 3, 8));

        canvas.resize();

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
    }

    fn resize_canvas(&self) {
        if let Some(canvas) = self.canvas.get() {
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
            let (mouse_x, mouse_y) = self.cursor_pos();
            let [mouse_x, mouse_y] = [mouse_x as f32, mouse_y as f32];
            let [mouse_scene_x, mouse_scene_y] = [
                dims.scene_pos.x + mouse_x / dims.scale,
                dims.scene_pos.y + mouse_y / dims.scale,
            ];

            let new_scale = change_scale(dims.scale, factor);

            let new_scene_pos = Point {
                x: mouse_scene_x - (mouse_x / new_scale),
                y: mouse_scene_y - (mouse_y / new_scale),
            };

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
        let (x0, y0) = (xy0.x, xy0.y);
        let (dx0, dy0) = self.drag_start_pos.get();
        let (dx1, dy1) = self.cursor_pos();
        let (dx, dy) = (dx1 - dx0, dy1 - dy0);

        if self.right_dragging.get() {
            self.get_canvas().update_dimensions(|dims| {
                let x = x0 - (dx as f32) / dims.scale;
                let y = y0 - (dy as f32) / dims.scale;
                dims.scene_pos = [x, y].into();
            })
        }
        if self.left_dragging.get() {
            if let Some(i) = self.dragged_point.get() {
                self.get_canvas()
                    .update_point(i, self.get_point_at_cursor());
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
                for (i, point) in canvas.drawing_points().iter().enumerate() {
                    let dx = cursor_pos.x - point.x;
                    let dy = cursor_pos.y - point.y;
                    if f32::max(dx.abs(), dy.abs()) <= 5.0 / scale {
                        drag_idx = Some(i);
                        break;
                    }
                }
                if drag_idx.is_none() {
                    self.add_point_at_cursor();
                    drag_idx = Some(canvas.drawing_points().len() - 1);
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
                self.dragged_point.take();
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

        let make_button = |text| {
            let mut btn = Default::default();
            nwg::Button::builder()
                .parent(&window)
                .text(text)
                .build(&mut btn)?;
            Ok(btn)
        };

        let paste_image_btn = make_button("bg")?;
        let clear_drawing_btn = make_button("clear")?;
        let copy_drawing_btn = make_button("copy")?;
        let drawing_color_btn = make_button("drawing color")?;
        let shape_color_btn = make_button("shape color")?;

        let mut shape_alpha_slider = Default::default();
        nwg::TrackBar::builder()
            .parent(&window)
            .build(&mut shape_alpha_slider)?;
        shape_alpha_slider.set_pos(50);

        let mut grid = Default::default();
        nwg::GridLayout::builder()
            .parent(&window)
            .max_column(Some(4))
            .max_row(Some(8))
            //.child_item(nwg::GridLayoutItem::new(&canvas, 0, 0, 3, 8))
            .child_item(nwg::GridLayoutItem::new(&paste_image_btn, 3, 0, 1, 1))
            .child_item(nwg::GridLayoutItem::new(&clear_drawing_btn, 3, 1, 1, 1))
            .child_item(nwg::GridLayoutItem::new(&copy_drawing_btn, 3, 2, 1, 1))
            .child_item(nwg::GridLayoutItem::new(&drawing_color_btn, 3, 3, 1, 1))
            .child_item(nwg::GridLayoutItem::new(&shape_color_btn, 3, 4, 1, 1))
            .child_item(nwg::GridLayoutItem::new(&shape_alpha_slider, 3, 5, 1, 1))
            .build(&mut grid)?;

        let mut color_dialog = Default::default();
        nwg::ColorDialog::builder().build(&mut color_dialog)?;

        let inner = Rc::new(AppInner {
            window,
            grid,
            canvas,
            canvas_handler: OnceCell::new(),
            paste_image_btn,
            clear_drawing_btn,
            copy_drawing_btn,
            drawing_color_btn,
            shape_color_btn,
            shape_alpha_slider,
            color_dialog,

            left_dragging: Default::default(),
            right_dragging: Default::default(),
            dragged_point: Default::default(),
            pre_drag_pos: Default::default(),
            drag_start_pos: Default::default(),
        });

        let ui = Rc::downgrade(&inner);
        let handle_fn = move |evt, _evt_data, handle| {
            let ui = ui.upgrade().unwrap();
            if handle == ui.window.handle {
                match evt {
                    Event::OnInit => AppInner::show(ui),
                    Event::OnResize | Event::OnWindowMaximize | Event::OnResizeEnd => {
                        ui.resize_canvas()
                    }
                    Event::OnWindowClose => ui.exit(),
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
