use std::cell::Cell;

use native_windows_derive as nwd;
use native_windows_gui as nwg;

use nwd::NwgUi;

use crate::gl::OpenGlCanvas;

#[derive(Default, NwgUi)]
pub struct Canvas {
    #[nwg_control(size: (600, 500), position: (300, 300), title: "nwg/ogl", flags: "MAIN_WINDOW")]
    #[nwg_events(
        OnInit: [Canvas::show],
        OnWindowClose: [Canvas::exit],
        OnResize: [Canvas::resize_canvas],
        OnWindowMaximize: [Canvas::resize_canvas],
    )]
    pub window: nwg::Window,

    #[nwg_layout(parent: window, max_column: Some(4), max_row: Some(8))]
    pub grid: nwg::GridLayout,

    #[nwg_control(ty: nwg::ExternCanvas, parent: Some(&data.window))]
    #[nwg_events(
        OnMouseMove: [Canvas::mouse_move],
        OnMousePress: [Canvas::mouse_press(SELF, EVT)],
        OnMouseWheel: [Canvas::zoom(SELF, EVT_DATA)],
    )]
    #[nwg_layout_item(layout: grid, col: 0, row: 0, col_span: 3, row_span: 8)]
    pub canvas: OpenGlCanvas,

    #[nwg_resource]
    pub color_dialog: nwg::ColorDialog,

    #[nwg_control(parent: window, text: "bg")]
    #[nwg_events(OnButtonClick: [Canvas::select_bg_color])]
    #[nwg_layout_item(layout: grid, col: 3, row: 0)]
    pub choose_color_btn2: nwg::Button,

    #[nwg_control(parent: window, text: "tri")]
    #[nwg_events(OnButtonClick: [Canvas::select_tri_color])]
    #[nwg_layout_item(layout: grid, col: 3, row: 1)]
    pub choose_color_btn1: nwg::Button,

    #[nwg_control(parent: window, text: "paste image")]
    #[nwg_events(OnButtonClick: [Canvas::paste_image])]
    #[nwg_layout_item(layout: grid, col: 3, row: 2)]
    pub paste_image_btn: nwg::Button,

    dragging: Cell<bool>,
    pre_drag_pos: Cell<[f32; 2]>,
    drag_start_pos: Cell<(i32, i32)>,
}

pub fn change_scale(mut scale: f32, factor: i32) -> f32 {
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

impl Canvas {
    fn cursor_pos(&self) -> (i32, i32) {
        nwg::GlobalCursor::local_position(&self.canvas, None)
    }

    pub fn mouse_move(&self) {
        if self.dragging.get() {
            let [x0, y0] = self.pre_drag_pos.get();
            let (dx0, dy0) = self.drag_start_pos.get();
            let (dx1, dy1) = self.cursor_pos();
            let (dx, dy) = (dx1 - dx0, dy1 - dy0);
            self.canvas.update_dimensions(|dims| {
                let x = x0 - (dx as f32) / dims.scale;
                let y = y0 - (dy as f32) / dims.scale;
                dims.scene_pos = [x, y];
            })
        }
    }

    pub fn mouse_press(&self, event: nwg::Event) {
        let ev = match event {
            nwg::Event::OnMousePress(ev) => ev,
            _ => return, // should be unreachable
        };
        match ev {
            nwg::MousePressEvent::MousePressRightDown => {
                nwg::GlobalCursor::set_capture(&self.canvas.handle);
                self.dragging.set(true);
                self.drag_start_pos.set(self.cursor_pos());
                self.pre_drag_pos
                    .set(self.canvas.get_dimensions().scene_pos);
            }
            nwg::MousePressEvent::MousePressRightUp => {
                nwg::GlobalCursor::release();
                self.dragging.set(false);
            }
            nwg::MousePressEvent::MousePressLeftDown => {
                let (x, y) = self.cursor_pos();
                let dims = self.canvas.get_dimensions();
                let (scene_x, scene_y) = (
                    dims.scene_pos[0] + x as f32 / dims.scale,
                    dims.scene_pos[1] + y as f32 / dims.scale,
                );
                self.canvas.add_point(scene_x, scene_y);
            }
            _ => (),
        }
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.set_focus();
    }

    pub fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    pub fn resize_canvas(&self) {
        self.canvas.resize();
    }

    pub fn select_bg_color(&self) {
        if self.color_dialog.run(Some(&self.window)) {
            let [r, g, b] = self.color_dialog.color();
            let [r, g, b] = [r as f32 / 225.0, g as f32 / 225.0, b as f32 / 225.0];

            unsafe {
                gl::ClearColor(r, g, b, 1.0);
            }
        }
        self.window.invalidate();
    }

    pub fn select_tri_color(&self) {
        if self.color_dialog.run(Some(&self.window)) {
            let [r, g, b] = self.color_dialog.color();
            let [r, g, b] = [r as f32 / 225.0, g as f32 / 225.0, b as f32 / 225.0];

            #[rustfmt::skip]
            let vertex_data: &[f32] = &[
                0.0,  1.0,   r, g, b,
               -1.0, -1.0,   r, g, b,
                1.0, -1.0,   r, g, b,
            ];
            let vertex_size = vertex_data.len() * std::mem::size_of::<f32>();

            unsafe {
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    vertex_size as _,
                    vertex_data.as_ptr().cast(),
                );
            }
        }
        self.window.invalidate();
    }

    pub fn paste_image(&self) {
        let buf = match clipboard_win::get_clipboard(clipboard_win::formats::Bitmap) {
            Ok(buf) => buf,
            e => {
                println!("{:?}", e);
                return;
            }
        };
        let cursor = std::io::Cursor::new(&buf[..]);
        let img = image::codecs::bmp::BmpDecoder::new(cursor).unwrap();
        self.canvas.set_image(img);
    }

    pub fn zoom(&self, data: &nwg::EventData) {
        let factor = match data {
            nwg::EventData::OnMouseWheel(i) => *i / 120,
            _ => panic!(),
        };
        self.canvas.update_dimensions(|dims| {
            let (mouse_x, mouse_y) = self.cursor_pos();
            let [mouse_x, mouse_y] = [mouse_x as f32, mouse_y as f32];
            let [mouse_scene_x, mouse_scene_y] = [
                dims.scene_pos[0] + mouse_x / dims.scale,
                dims.scene_pos[1] + mouse_y / dims.scale,
            ];

            let new_scale = change_scale(dims.scale, factor);

            let new_scene_pos = [
                mouse_scene_x - (mouse_x / new_scale),
                mouse_scene_y - (mouse_y / new_scale),
            ];

            if self.dragging.get() {
                self.pre_drag_pos.set(new_scene_pos);
                self.drag_start_pos.set(self.cursor_pos());
            }

            dims.scale = new_scale;
            dims.scene_pos = new_scene_pos;
        })
    }
}
