use crepuscularity_gpui::prelude::*;
use std::process::{Child, Command, Stdio};

struct SoliloquyWindow {
    address: SharedString,
    mode: SharedString,
    tab_title: SharedString,
    servo_child: Option<Child>,
}

impl SoliloquyWindow {
    fn new(_cx: &mut Context<Self>) -> Self {
        let address =
            std::env::var("SOL_START_URL").unwrap_or_else(|_| "https://example.com".to_string());
        let servo_child = spawn_servo_renderer(&address);

        Self {
            address: address.into(),
            mode: "Zen".into(),
            tab_title: "Servo".into(),
            servo_child,
        }
    }
}

impl Drop for SoliloquyWindow {
    fn drop(&mut self) {
        if let Some(child) = self.servo_child.as_mut() {
            let _ = child.kill();
        }
    }
}

fn spawn_servo_renderer(address: &str) -> Option<Child> {
    if std::env::var("SOL_SERVO_CHILD_DISABLED").ok().as_deref() == Some("1") {
        return None;
    }

    let servo_bin = std::env::var("SERVO_BIN")
        .unwrap_or_else(|_| "third_party/servo/target/release/servoshell".to_string());

    let bin_name = std::path::Path::new(&servo_bin)
        .file_name()
        .and_then(|n| n.to_str());

    if bin_name != Some("servo") && bin_name != Some("servoshell") {
        eprintln!("Security error: Invalid SERVO_BIN path. Must be an executable named 'servo' or 'servoshell'.");
        return None;
    }

    let window_size = std::env::var("SOL_WINDOW_SIZE").unwrap_or_else(|_| "1280x820".to_string());
    let js_engine =
        std::env::var("SOLILOQUY_JS_ENGINE").unwrap_or_else(|_| "v8-experimental".to_string());

    Command::new(servo_bin)
        .arg("--no-browser-chrome")
        .arg("--window-size")
        .arg(window_size)
        .arg(address)
        .env("SOLILOQUY_JS_ENGINE", js_engine)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

impl Render for SoliloquyWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let address = self.address.clone();
        let mode = self.mode.clone();
        let tab_title = self.tab_title.clone();

        view! {r#"
            div w-full h-full bg-[#070807] text-white flex flex-col font-['Instrument_Sans']
                div h-[38px] px-4 border-b border-white/10 flex items-center justify-between bg-[#11120f]
                    div flex gap-2 items-center
                        div w-[11px] h-[11px] rounded-full border border-white/20 bg-[#2a2b26]
                        div w-[11px] h-[11px] rounded-full border border-white/20 bg-[#2a2b26]
                        div w-[11px] h-[11px] rounded-full border border-white/20 bg-[#2a2b26]
                    div text-xs font-bold text-white/70
                        "{mode}"
                    div text-xs font-bold text-white/60
                        "macOS"
                div h-[56px] px-4 border-b border-white/10 bg-[#11120f] flex items-center gap-3
                    div w-[32px] h-[32px] rounded-[7px] border border-white/10 flex items-center justify-center text-white/70
                        "‹"
                    div w-[32px] h-[32px] rounded-[7px] border border-white/10 flex items-center justify-center text-white/70
                        "›"
                    div w-[32px] h-[32px] rounded-[7px] border border-white/10 flex items-center justify-center text-white/70
                        "↻"
                    div flex-1 h-[36px] rounded-[8px] border border-white/10 bg-black flex items-center px-3 text-sm font-bold text-white
                        "{address}"
                    div w-[32px] h-[32px] rounded-[7px] border border-[#80b9a4]/40 bg-white/10 flex items-center justify-center text-white
                        "⌘"
                    div w-[32px] h-[32px] rounded-[7px] border border-white/10 flex items-center justify-center text-white/70
                        "+"
                div flex-1 min-h-0 flex
                    div w-[58px] bg-[#0d0e0c] border-r border-white/10 flex flex-col items-center gap-3 py-3
                        div w-[36px] h-[36px] rounded-[8px] border border-white/10 bg-white/10 flex items-center justify-center text-white
                            "›_"
                        div w-[36px] h-[36px] rounded-[8px] flex items-center justify-center text-white/70
                            "□"
                        div w-[36px] h-[36px] rounded-[8px] flex items-center justify-center text-white/70
                            "⚙"
                        div flex-1
                        div w-[36px] h-[36px] rounded-[8px] flex items-center justify-center text-white/70
                            "+"
                    div flex-1 bg-black p-4
                        div w-full h-full rounded-[8px] border border-white/10 bg-black flex flex-col font-['SF_Mono']
                            div flex-1 p-4 text-sm text-emerald-300
                                "[terminal unavailable] start sold to attach a PTY"
                            div h-[45px] border-t border-white/10 px-4 flex items-center gap-2 text-sm
                                div text-emerald-300
                                    "soliloquy%"
                                div text-white/40
                                    "{tab_title}"
        "#}
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_options = WindowOptions {
            titlebar: None,
            app_id: Some("company.atechnology.soliloquy".to_string()),
            ..WindowOptions::default()
        };

        cx.open_window(window_options, |_window, cx| cx.new(SoliloquyWindow::new))
            .unwrap();
    });
}
