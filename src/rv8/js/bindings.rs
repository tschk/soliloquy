//! V8 bindings for DOM and Web APIs
//!
//! This module implements the mapping between V8 JavaScript objects
//! and the Rust implementation of DOM nodes and Web APIs.

use parking_lot::RwLock;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

use crate::servo_embed::dom::{DomTree, NodeId, NodeType};
use crate::servo_embed::web_apis::{ConsoleApi, StorageApi, TimerManager};

const CONTEXT_DATA_KEY: &str = "__rv8_context_data";
const NODE_ID_KEY: &str = "__rv8_node_id";
const STORAGE_TYPE_KEY: &str = "__rv8_storage_type";

/// Data stored in V8 context embedder data
pub struct V8ContextData {
    pub dom_tree: Arc<RwLock<DomTree>>,
    pub console_api: Arc<RwLock<ConsoleApi>>,
    pub timer_manager: Arc<RwLock<TimerManager>>,
    pub local_storage: Arc<RwLock<StorageApi>>,
    pub session_storage: Arc<RwLock<StorageApi>>,
    pub timer_callbacks: RwLock<HashMap<u64, v8::Global<v8::Function>>>,
}

impl V8ContextData {
    pub fn new(
        dom_tree: Arc<RwLock<DomTree>>,
        console_api: Arc<RwLock<ConsoleApi>>,
        timer_manager: Arc<RwLock<TimerManager>>,
        local_storage: Arc<RwLock<StorageApi>>,
        session_storage: Arc<RwLock<StorageApi>>,
    ) -> Self {
        Self {
            dom_tree,
            console_api,
            timer_manager,
            local_storage,
            session_storage,
            timer_callbacks: RwLock::new(HashMap::new()),
        }
    }
}

/// Initialize a V8 context with DOM and Web APIs
pub fn initialize_context<'s>(
    scope: &mut v8::HandleScope<'s, ()>,
    data: V8ContextData,
) -> v8::Local<'s, v8::Context> {
    let global_template = v8::ObjectTemplate::new(scope);

    let context = v8::Context::new_from_template(scope, global_template);
    let scope = &mut v8::ContextScope::new(scope, context);

    let data_ptr = Box::into_raw(Box::new(data));
    set_context_data(scope, data_ptr);

    // Set up DOM and Storage on the context
    setup_console(scope, context);
    setup_timers(scope, context);
    setup_dom(scope, context);
    setup_storage(scope, context);

    context
}

/// Remove and free the Rust data attached to the current V8 context.
pub fn take_context_data(scope: &mut v8::HandleScope) -> Option<Box<V8ContextData>> {
    let ptr = context_data_ptr(scope)?;
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, CONTEXT_DATA_KEY)?;
    let undefined = v8::undefined(scope);
    let _ = global.set(scope, key.into(), undefined.into());

    // SAFETY: `ptr` was created with `Box::into_raw` in `initialize_context`.
    Some(unsafe { Box::from_raw(ptr) })
}

fn set_context_data<'s>(scope: &mut v8::HandleScope<'s>, data_ptr: *mut V8ContextData) {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, CONTEXT_DATA_KEY).expect("static V8 key should allocate");
    let external = v8::External::new(scope, data_ptr.cast::<c_void>());
    let _ = global.set(scope, key.into(), external.into());
}

fn context_data_ptr(scope: &mut v8::HandleScope) -> Option<*mut V8ContextData> {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, CONTEXT_DATA_KEY)?;
    let value = global.get(scope, key.into())?;
    let external = v8::Local::<v8::External>::try_from(value).ok()?;
    Some(external.value().cast::<V8ContextData>())
}

fn set_property<'s>(
    scope: &mut v8::HandleScope<'s>,
    object: v8::Local<v8::Object>,
    name: &str,
    value: v8::Local<v8::Value>,
) {
    let key = v8::String::new(scope, name).expect("static V8 key should allocate");
    let _ = object.set(scope, key.into(), value);
}

fn set_number_property<'s>(
    scope: &mut v8::HandleScope<'s>,
    object: v8::Local<v8::Object>,
    name: &str,
    value: u64,
) {
    let number = v8::Number::new(scope, value as f64);
    set_property(scope, object, name, number.into());
}

fn get_number_property(
    scope: &mut v8::HandleScope,
    object: v8::Local<v8::Object>,
    name: &str,
) -> Option<u64> {
    let key = v8::String::new(scope, name)?;
    object
        .get(scope, key.into())?
        .integer_value(scope)
        .map(|value| value as u64)
}

fn setup_console<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    let global = context.global(scope);
    let console = v8::Object::new(scope);

    let log_callback = v8::Function::new(scope, console_log).expect("console.log function");
    set_property(scope, console, "log", log_callback.into());

    let info_callback = v8::Function::new(scope, console_info).expect("console.info function");
    set_property(scope, console, "info", info_callback.into());

    let warn_callback = v8::Function::new(scope, console_warn).expect("console.warn function");
    set_property(scope, console, "warn", warn_callback.into());

    let error_callback = v8::Function::new(scope, console_error).expect("console.error function");
    set_property(scope, console, "error", error_callback.into());

    set_property(scope, global, "console", console.into());
}

fn setup_timers<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    let global = context.global(scope);

    let set_timeout = v8::Function::new(scope, set_timeout_callback).expect("setTimeout function");
    set_property(scope, global, "setTimeout", set_timeout.into());

    let clear_timeout =
        v8::Function::new(scope, clear_timer_callback).expect("clearTimeout function");
    set_property(scope, global, "clearTimeout", clear_timeout.into());

    let set_interval =
        v8::Function::new(scope, set_interval_callback).expect("setInterval function");
    set_property(scope, global, "setInterval", set_interval.into());

    let clear_interval =
        v8::Function::new(scope, clear_timer_callback).expect("clearInterval function");
    set_property(scope, global, "clearInterval", clear_interval.into());
}

fn setup_storage<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    let global = context.global(scope);
    let local_storage_obj = create_storage_object(scope, 0);
    let session_storage_obj = create_storage_object(scope, 1);

    set_property(scope, global, "localStorage", local_storage_obj.into());
    set_property(scope, global, "sessionStorage", session_storage_obj.into());
}

fn create_storage_object<'s>(
    scope: &mut v8::HandleScope<'s>,
    storage_type: u64,
) -> v8::Local<'s, v8::Object> {
    let object = v8::Object::new(scope);
    set_number_property(scope, object, STORAGE_TYPE_KEY, storage_type);

    let get_item_fn = v8::Function::new(scope, storage_get_item).expect("storage getItem function");
    set_property(scope, object, "getItem", get_item_fn.into());

    let set_item_fn = v8::Function::new(scope, storage_set_item).expect("storage setItem function");
    set_property(scope, object, "setItem", set_item_fn.into());

    let remove_item_fn =
        v8::Function::new(scope, storage_remove_item).expect("storage removeItem function");
    set_property(scope, object, "removeItem", remove_item_fn.into());

    let clear_fn = v8::Function::new(scope, storage_clear).expect("storage clear function");
    set_property(scope, object, "clear", clear_fn.into());

    object
}

fn setup_dom<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    let global = context.global(scope);

    let doc_id = get_context_data(scope).dom_tree.read().document_id();
    let doc_obj = create_node_object(scope, doc_id);
    let create_element_fn =
        v8::Function::new(scope, create_element_callback).expect("document.createElement function");
    set_property(scope, doc_obj, "createElement", create_element_fn.into());
    set_property(scope, global, "document", doc_obj.into());

    let node_ctor = v8::Function::new(scope, empty_constructor).expect("Node constructor");
    let element_ctor = v8::Function::new(scope, empty_constructor).expect("Element constructor");
    let document_ctor = v8::Function::new(scope, empty_constructor).expect("Document constructor");
    set_property(scope, global, "Node", node_ctor.into());
    set_property(scope, global, "Element", element_ctor.into());
    set_property(scope, global, "Document", document_ctor.into());
}

/// Wrap a NodeId into a JS object
pub fn wrap_node<'s>(
    scope: &mut v8::HandleScope<'s>,
    node_id: NodeId,
) -> v8::Local<'s, v8::Object> {
    create_node_object(scope, node_id)
}

fn empty_constructor(
    _scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
}

fn create_node_object<'s>(
    scope: &mut v8::HandleScope<'s>,
    node_id: NodeId,
) -> v8::Local<'s, v8::Object> {
    let object = v8::Object::new(scope);
    set_number_property(scope, object, NODE_ID_KEY, node_id);

    let node_type_name = v8::String::new(scope, "nodeType").expect("nodeType property");
    let _ = object.set_accessor(scope, node_type_name.into(), node_type_getter);

    let node_name = v8::String::new(scope, "nodeName").expect("nodeName property");
    let _ = object.set_accessor(scope, node_name.into(), node_name_getter);

    let tag_name = v8::String::new(scope, "tagName").expect("tagName property");
    let _ = object.set_accessor(scope, tag_name.into(), tag_name_getter);

    object
}

// --- Callbacks (Console, Timers, DOM, Storage) ---

fn console_log(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().log(&message);
}

fn console_info(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().info(&message);
}

fn console_warn(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().warn(&message);
}

fn console_error(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let message = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().error(&message);
}

fn set_timeout_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let delay = args.get(1).integer_value(scope).unwrap_or(0) as u64;

    let data = get_context_data(scope);
    let timer_id = data.timer_manager.write().set_timeout(0, delay);

    // Store the callback
    data.timer_callbacks
        .write()
        .insert(timer_id, v8::Global::new(scope, callback));

    rv.set(v8::Number::new(scope, timer_id as f64).into());
}

fn set_interval_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let interval = args.get(1).integer_value(scope).unwrap_or(0) as u64;

    let data = get_context_data(scope);
    let timer_id = data.timer_manager.write().set_interval(0, interval);

    // Store the callback
    data.timer_callbacks
        .write()
        .insert(timer_id, v8::Global::new(scope, callback));

    rv.set(v8::Number::new(scope, timer_id as f64).into());
}

fn clear_timer_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let timer_id = args.get(0).integer_value(scope).unwrap_or(0) as u64;
    let data = get_context_data(scope);
    data.timer_manager.write().clear_timer(timer_id);
    data.timer_callbacks.write().remove(&timer_id);
}

fn node_type_getter(
    scope: &mut v8::HandleScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(node_id) = get_number_property(scope, args.this(), NODE_ID_KEY) else {
        return;
    };
    let type_val = {
        let dom_tree = get_context_data(scope).dom_tree.read();
        dom_tree.get_node(node_id).map(|node| match node.node_type {
            NodeType::Element => 1,
            NodeType::Text => 3,
            NodeType::Comment => 8,
            NodeType::Document => 9,
            NodeType::DocumentFragment => 11,
        })
    };
    if let Some(type_val) = type_val {
        rv.set(v8::Integer::new(scope, type_val).into());
    }
}

fn node_name_getter(
    scope: &mut v8::HandleScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(node_id) = get_number_property(scope, args.this(), NODE_ID_KEY) else {
        return;
    };
    let name = {
        let dom_tree = get_context_data(scope).dom_tree.read();
        dom_tree
            .get_node(node_id)
            .map(|node| node.tag_name.clone().unwrap_or_else(|| "#text".to_string()))
    };
    if let Some(name) = name {
        rv.set(v8::String::new(scope, &name).unwrap().into());
    }
}

fn tag_name_getter(
    scope: &mut v8::HandleScope,
    _name: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(node_id) = get_number_property(scope, args.this(), NODE_ID_KEY) else {
        return;
    };
    let tag_name = {
        let dom_tree = get_context_data(scope).dom_tree.read();
        dom_tree
            .get_node(node_id)
            .and_then(|node| node.tag_name.as_ref().map(|tag| tag.to_uppercase()))
    };
    if let Some(tag_name) = tag_name {
        rv.set(v8::String::new(scope, &tag_name).unwrap().into());
    }
}

fn create_element_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let tag = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let new_id = get_context_data(scope)
        .dom_tree
        .write()
        .create_element(&tag);
    rv.set(wrap_node(scope, new_id).into());
}

fn get_storage(
    scope: &mut v8::HandleScope,
    this: v8::Local<v8::Object>,
) -> Arc<RwLock<StorageApi>> {
    let data = get_context_data(scope);
    let storage_type = get_number_property(scope, this, STORAGE_TYPE_KEY).unwrap_or(0);
    if storage_type == 0 {
        data.local_storage.clone()
    } else {
        data.session_storage.clone()
    }
}

fn storage_get_item(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let key = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let storage = get_storage(scope, args.this());
    let value = storage.read().get_item(&key).map(str::to_owned);
    if let Some(value) = value {
        rv.set(v8::String::new(scope, &value).unwrap().into());
    } else {
        rv.set(v8::null(scope).into());
    }
}

fn storage_set_item(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let key = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let value = args
        .get(1)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let storage = get_storage(scope, args.this());
    let _ = storage.write().set_item(&key, &value);
}

fn storage_remove_item(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let key = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    let storage = get_storage(scope, args.this());
    storage.write().remove_item(&key);
}

fn storage_clear(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let storage = get_storage(scope, args.this());
    storage.write().clear();
}

pub(crate) fn get_context_data(scope: &mut v8::HandleScope) -> &'static V8ContextData {
    let ptr = context_data_ptr(scope).expect("V8 context data should be installed");
    // SAFETY: `ptr` is owned by the current V8 context and freed by `take_context_data`.
    unsafe { &*ptr }
}
