#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod components;
mod highlighting;
mod state;
mod theme;

#[cfg(not(target_arch = "wasm32"))]
mod storage;

pub use app::PromptGenApp;
