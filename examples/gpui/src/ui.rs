use std::sync::Arc;

use gpui::{
    div, img, prelude::*, px, size, App, Application, Bounds, Context, ImageSource, Render, Window,
    WindowBounds, WindowOptions,
};
use tokio::sync::oneshot;

use crate::video::SharedFrame;

pub fn run_video_window(shared_frame: SharedFrame, shutdown_tx: oneshot::Sender<()>) {
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(540.0)), cx);
        let mut shutdown_tx = Some(shutdown_tx);
        cx.on_window_closed(move |cx| {
            if let Some(tx) = shutdown_tx.take() {
                let _ = tx.send(());
            }
            cx.shutdown();
        })
        .detach();

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| VideoWindow::new(Arc::clone(&shared_frame))),
        )
        .unwrap();
        cx.activate(true);
    });
}

struct VideoWindow {
    shared_frame: SharedFrame,
    last_frame_size: Option<(u32, u32)>,
}

impl VideoWindow {
    fn new(shared_frame: SharedFrame) -> Self {
        Self {
            shared_frame,
            last_frame_size: None,
        }
    }
}

impl Render for VideoWindow {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        let (current, frame_size) = {
            let state = self.shared_frame.lock().unwrap();
            (state.image.clone(), state.size)
        };

        if let Some(frame) = current {
            let source: ImageSource = frame.into();
            let bounds = window.bounds();
            let window_w: f32 = bounds.size.width.into();
            let window_h: f32 = bounds.size.height.into();
            let (target_w, target_h) = if window_w > 0.0 && window_h > 0.0 {
                if let Some((w, h)) = frame_size {
                    if self.last_frame_size != Some((w, h)) {
                        window.resize(size(px(w as f32), px(h as f32)));
                        self.last_frame_size = Some((w, h));
                    }
                    let ratio = w as f32 / h as f32;
                    if window_w / window_h > ratio {
                        (window_h * ratio, window_h)
                    } else {
                        (window_w, window_w / ratio)
                    }
                } else {
                    (window_w, window_h)
                }
            } else {
                (0.0, 0.0)
            };

            div()
                .size_full()
                .bg(gpui::black())
                .items_center()
                .justify_center()
                .child(
                    img(source)
                        .w(px(target_w))
                        .h(px(target_h))
                        .object_fit(gpui::ObjectFit::Contain),
                )
        } else {
            div()
                .size_full()
                .bg(gpui::black())
                .items_center()
                .justify_center()
                .child("Waiting for video...")
        }
    }
}
