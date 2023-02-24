use crate::args::{Args, PositionArg};
use crate::translation::Translator;
use clap::Parser;
use std::{borrow::Cow, str::FromStr};
use wry::http::response::Builder;
use wry::http::{header, Response};
use wry::{
    application::window::WindowId,
    application::{
        accelerator::Accelerator,
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        global_shortcut::ShortcutManager,
        window::WindowBuilder,
    },
    webview::WebView,
    webview::WebViewBuilder,
};

mod args;
mod clipboard;
mod translation;

#[warn(unused_assignments)]
fn main() -> wry::Result<()> {
    let args  = Args::parse();
    let mut _current_webview = None;
    let event_loop = EventLoop::new();

    let mut hotkey_manager = ShortcutManager::new(&event_loop);
    let shortcut_show = Accelerator::from_str(args.show()).unwrap();

    if args.run_once() {
        let (_id, webview) = show(&event_loop, args.translator(), args.text(), args.position());
        _current_webview = Some(webview);
    } else {
        hotkey_manager.register(shortcut_show.clone()).unwrap();
    }

    event_loop.run(move |event, event_loop, control_flow| {
        let args = args.clone();

        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("Popup translation has started!")
            }
            Event::GlobalShortcutEvent(hotkey_id) => {
                if hotkey_id == shortcut_show.clone().id() {
                    let (_id, webview) =
                        show(event_loop, args.translator(), args.text(), args.position());
                    _current_webview = Some(webview);
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    _current_webview = None;
                    if args.run_once() {
                        *control_flow = ControlFlow::Exit
                    }
                }
                _ => (),
            },
            _ => (),
        }
    });
}

const EMPTY: &[u8] = b"HELLO";

fn show<T: 'static>(
    event_loop: &EventLoopWindowTarget<T>,
    translator: Translator,
    text: String,
    position: PositionArg,
) -> (WindowId, WebView) {
    #[cfg(target_os = "macos")]
        let user_agent_string = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.1 Safari/605.1.15";
    #[cfg(target_os = "windows")]
        let user_agent_string = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";
    #[cfg(target_os = "linux")]
        let user_agent_string = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36";

    let window = WindowBuilder::new()
        .with_title(translator.name())
        .with_inner_size(translator.inner_size())
        .with_decorations(false)
        .with_resizable(false)
        .with_focused(true)
        .with_visible(false)
        .build(event_loop)
        .unwrap();

    let size_and_pos = if let Some(monitor) = window.current_monitor() {
        let size = monitor.size();
        let pos = monitor.position();

        println!("{:?}", pos);
        ((size.width as i32, size.height as i32), (pos.x, pos.y))
    } else {
        ((0, 0), (0, 0))
    };

    window.set_outer_position(position.to_wry_position(
        || event_loop.cursor_position(),
        size_and_pos.0,
        translator.size(),
        size_and_pos.1,
    ));

    let window_id = window.id();
    let icon = translator.icon();

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_user_agent(user_agent_string)
        .with_devtools(true)
        .with_custom_protocol("wry".into(), move |request| {
            let uri = request.uri().to_string();
            let url = uri.strip_prefix("wry://dev/").unwrap();

            let (content, resp) = match url {
                "icon" => (icon, common_resp("image/png")),
                _ => (EMPTY, common_resp("text/plain")),
            };

            resp.body(Cow::from(content)).map_err(Into::into)
        });

    let web_view = translator.build_webview(webview, text);
    web_view.window().set_visible(true);

    (window_id, web_view)
}

fn common_resp(content_type: &str) -> Builder {
    Response::builder()
        .header("Origin", "http://localhost/")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CONTENT_TYPE, content_type)
        .header("Access-Control-Request-Method", "*")
        .header("Access-Control-Allow-Methods", "*")
        .header("Access-Control-Allow-Headers", "*")
        .status(200)
}
