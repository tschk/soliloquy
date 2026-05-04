//! JavaScript engine module - V8 based

pub mod bindings;
mod engine;
mod value;

pub use engine::JsEngine;
pub use value::JsValue;
