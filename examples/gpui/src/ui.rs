use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use gpui::{
    div, img, prelude::*, px, size, App, Application, Bounds, Context, ImageSource, Render, Window,
    WindowBounds, WindowHandle, WindowOptions,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::video::SharedFrame;
use example_common::playback;

#[derive(Clone, Copy, Debug)]
pub enum UiEvent {
    Open,
    Close,
}

pub fn run_video_window(
    shared_frame: SharedFrame,
    ui_rx: UnboundedReceiver<UiEvent>,
    restart_tx: std::sync::mpsc::Sender<()>,
) {
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(540.0)), cx);
        let shared_frame = Arc::clone(&shared_frame);
        let close_guard = Arc::new(AtomicUsize::new(0));
        let guard_for_observer = Arc::clone(&close_guard);
        let restart_tx = restart_tx.clone();

        cx.on_window_closed(move |_cx| {
            if guard_for_observer.load(Ordering::Acquire) > 0 {
                guard_for_observer.fetch_sub(1, Ordering::AcqRel);
                return;
            }
            playback::close_current_streams();
            let _ = restart_tx.send(());
        })
        .detach();

        cx.spawn(async move |cx| {
            let mut window_handle: Option<WindowHandle<VideoWindow>> = None;
            let mut ui_rx = ui_rx;

            while let Some(event) = ui_rx.recv().await {
                match event {
                    UiEvent::Open => {
                        let shared_frame = Arc::clone(&shared_frame);
                        let previous = window_handle.take();
                        if let Some(handle) = previous {
                            close_guard.fetch_add(1, Ordering::AcqRel);
                            let _ = handle.update(cx, |_, window, _| window.remove_window());
                        }
                        if let Ok(handle) = cx.open_window(
                            WindowOptions {
                                window_bounds: Some(WindowBounds::Windowed(bounds.clone())),
                                ..Default::default()
                            },
                            move |_, cx| cx.new(|_| VideoWindow::new(Arc::clone(&shared_frame))),
                        ) {
                            window_handle = Some(handle);
                            let _ = cx.update(|app| app.activate(true));
                        }
                    }
                    UiEvent::Close => {
                        let previous = window_handle.take();
                        if let Some(handle) = previous {
                            close_guard.fetch_add(1, Ordering::AcqRel);
                            let _ = handle.update(cx, |_, window, _| window.remove_window());
                        }
                    }
                }
            }
        })
        .detach();
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
