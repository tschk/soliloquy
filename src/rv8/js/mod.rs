//! JavaScript engine module - V8 based

mod engine;
mod value;
pub mod bindings;

pub use engine::JsEngine;
pub use value::JsValue;
