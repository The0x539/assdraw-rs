use native_windows_gui as nwg;

use nwg::NativeUi;

mod app;
mod ass_outline;
mod canvas;
mod drawing;
mod gl;
mod vk;

pub use crate::gl::abstraction;

fn othermain() {
    nwg::init().expect("Failed to init NWG");
    use canvas::Canvas;
    let app = Canvas::build_ui(Canvas::default()).expect("Failed to build UI");
    app.canvas.create_context();
    app.canvas.render();

    nwg::dispatch_thread_events_with_callback(move || {
        app.canvas.render();
    });
}

fn main() {
    nwg::init().unwrap();
    let app = app::AppBuilder::build_ui(Default::default()).unwrap();
    app.canvas.create_context();
    app.canvas.render();
    nwg::dispatch_thread_events_with_callback(move || {
        app.canvas.render();
    });
}
