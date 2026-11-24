mod app;
mod audio;
mod pages;
mod platform;
mod state;
mod storage;
mod ui;

fn main() {
    dioxus::launch(app::App);
}
