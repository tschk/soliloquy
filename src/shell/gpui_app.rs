use gpui::*;

struct SoliloquyWindow;

impl Render for SoliloquyWindow {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().flex().flex_col().size_full().bg(rgb(0xFFFFFF)).child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size_full()
                .child("Soliloquy Browser (GPUI)"),
        )
    }
}

pub fn run() {
    App::new().run(|cx: &mut AppContext| {
        cx.open_window(WindowOptions::default(), |cx| {
            cx.new_view(|_cx| SoliloquyWindow)
        });
    });
}
