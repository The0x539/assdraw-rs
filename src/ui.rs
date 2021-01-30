use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use native_windows_gui as nwg;

use crate::canvas::Canvas;

pub struct CanvasUi {
    inner: Rc<Canvas>,
    default_handler: RefCell<Vec<nwg::EventHandler>>,
}

impl nwg::NativeUi<CanvasUi> for Canvas {
    fn build_ui(mut data: Self) -> Result<CanvasUi, nwg::NwgError> {
        nwg::ColorDialog::builder().build(&mut data.color_dialog)?;

        nwg::Window::builder()
            .flags(nwg::WindowFlags::MAIN_WINDOW)
            .size((600, 500))
            .position((300, 300))
            .title("nwg/ogl")
            .build(&mut data.window)?;

        nwg::ExternCanvas::builder()
            .parent(Some(&data.window))
            .build(&mut data.canvas)?;

        nwg::Button::builder()
            .text("bg")
            .parent(&data.window)
            .build(&mut data.choose_color_btn1)?;

        nwg::Button::builder()
            .text("tri")
            .parent(&data.window)
            .build(&mut data.choose_color_btn2)?;

        let ui = CanvasUi {
            inner: Rc::new(data),
            default_handler: RefCell::new(Vec::new()),
        };

        let window_handles = [&ui.window.handle]; // ???
        for handle in &window_handles {
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                use nwg::Event as E;
                let evt_ui = match evt_ui.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };
                match evt {
                    E::OnResize => {
                        if &handle == &evt_ui.canvas {
                            evt_ui.resize_canvas();
                        }
                    }
                    E::OnButtonClick => {
                        if &handle == &evt_ui.choose_color_btn1 {
                            evt_ui.select_bg_color();
                        } else if &handle == &evt_ui.choose_color_btn2 {
                            evt_ui.select_tri_color();
                        }
                    }
                    E::OnWindowClose => {
                        if &handle == &evt_ui.window {
                            evt_ui.exit();
                        }
                    }
                    E::OnInit => {
                        if &handle == &evt_ui.window {
                            evt_ui.show();
                        }
                    }
                    _ => (),
                }
            };

            ui.default_handler
                .borrow_mut()
                .push(nwg::full_bind_event_handler(handle, handle_events));
        }

        nwg::GridLayout::builder()
            .parent(&ui.window)
            .max_column(Some(4))
            .max_row(Some(8))
            .child_item(nwg::GridLayoutItem::new(&ui.canvas, 0, 0, 3, 8))
            .child(3, 0, &ui.choose_color_btn1)
            .child(3, 1, &ui.choose_color_btn2)
            .build(&ui.layout)?;

        Ok(ui)
    }
}

impl Drop for CanvasUi {
    fn drop(&mut self) {
        let mut handlers = self.default_handler.borrow_mut();
        for handler in handlers.drain(0..) {
            nwg::unbind_event_handler(&handler);
        }
    }
}

impl Deref for CanvasUi {
    type Target = Canvas;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
