//! JavaScript value types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object,
    Array,
    Function,
}
