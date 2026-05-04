//! V8 bindings for DOM and Web APIs
//!
//! This module implements the mapping between V8 JavaScript objects
//! and the Rust implementation of DOM nodes and Web APIs.

use rusty_v8 as v8;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use log::{debug, warn};

use crate::servo_embed::dom::{DomTree, NodeId, NodeType};
use crate::servo_embed::web_apis::{ConsoleApi, TimerManager, StorageApi};

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
    scope: &mut v8::HandleScope<'s>,
    data: V8ContextData,
) -> v8::Local<'s, v8::Context> {
    let global_template = v8::ObjectTemplate::new(scope);

    // Set up Global Web APIs
    setup_console(scope, global_template);
    setup_timers(scope, global_template);

    let context = v8::Context::new_from_template(scope, global_template);
    let scope = &mut v8::ContextScope::new(scope, context);

    // Store context data
    let data_ptr = Box::into_raw(Box::new(data));
    context.set_aligned_pointer_in_embedder_data(0, data_ptr as *mut std::ffi::c_void);

    // Set up DOM and Storage on the context
    setup_dom(scope, context);
    setup_storage(scope, context);

    context
}

fn setup_console<'s>(scope: &mut v8::HandleScope<'s>, global: v8::Local<v8::ObjectTemplate>) {
    let console_template = v8::ObjectTemplate::new(scope);

    let log_callback = v8::FunctionTemplate::new(scope, console_log);
    let log_name = v8::String::new(scope, "log").unwrap();
    console_template.set(log_name.into(), log_callback.into());

    let info_callback = v8::FunctionTemplate::new(scope, console_info);
    let info_name = v8::String::new(scope, "info").unwrap();
    console_template.set(info_name.into(), info_callback.into());

    let warn_callback = v8::FunctionTemplate::new(scope, console_warn);
    let warn_name = v8::String::new(scope, "warn").unwrap();
    console_template.set(warn_name.into(), warn_callback.into());

    let error_callback = v8::FunctionTemplate::new(scope, console_error);
    let error_name = v8::String::new(scope, "error").unwrap();
    console_template.set(error_name.into(), error_callback.into());

    let console_name = v8::String::new(scope, "console").unwrap();
    global.set(console_name.into(), console_template.into());
}

fn setup_timers<'s>(scope: &mut v8::HandleScope<'s>, global: v8::Local<v8::ObjectTemplate>) {
    let set_timeout = v8::FunctionTemplate::new(scope, set_timeout_callback);
    let set_timeout_name = v8::String::new(scope, "setTimeout").unwrap();
    global.set(set_timeout_name.into(), set_timeout.into());

    let clear_timeout = v8::FunctionTemplate::new(scope, clear_timer_callback);
    let clear_timeout_name = v8::String::new(scope, "clearTimeout").unwrap();
    global.set(clear_timeout_name.into(), clear_timeout.into());

    let set_interval = v8::FunctionTemplate::new(scope, set_interval_callback);
    let set_interval_name = v8::String::new(scope, "setInterval").unwrap();
    global.set(set_interval_name.into(), set_interval.into());

    let clear_interval = v8::FunctionTemplate::new(scope, clear_timer_callback);
    let clear_interval_name = v8::String::new(scope, "clearInterval").unwrap();
    global.set(clear_interval_name.into(), clear_interval.into());
}

fn setup_storage<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    let storage_tmpl = v8::FunctionTemplate::new(scope, |_, _, _| {});
    storage_tmpl.set_class_name(v8::String::new(scope, "Storage").unwrap());
    let storage_inst = storage_tmpl.instance_template(scope);
    storage_inst.set_internal_field_count(1);

    let get_item_fn = v8::FunctionTemplate::new(scope, storage_get_item);
    storage_inst.set(v8::String::new(scope, "getItem").unwrap().into(), get_item_fn.into());

    let set_item_fn = v8::FunctionTemplate::new(scope, storage_set_item);
    storage_inst.set(v8::String::new(scope, "setItem").unwrap().into(), set_item_fn.into());

    let remove_item_fn = v8::FunctionTemplate::new(scope, storage_remove_item);
    storage_inst.set(v8::String::new(scope, "removeItem").unwrap().into(), remove_item_fn.into());

    let clear_fn = v8::FunctionTemplate::new(scope, storage_clear);
    storage_inst.set(v8::String::new(scope, "clear").unwrap().into(), clear_fn.into());

    let global = context.global(scope);
    let storage_ctor = storage_tmpl.get_function(scope).unwrap();

    // Local Storage (type 0)
    let local_storage_obj = storage_ctor.new_instance(scope, &[]).unwrap();
    local_storage_obj.set_internal_field(0, v8::Integer::new(scope, 0).into());
    global.set(scope, v8::String::new(scope, "localStorage").unwrap().into(), local_storage_obj.into()).unwrap();

    // Session Storage (type 1)
    let session_storage_obj = storage_ctor.new_instance(scope, &[]).unwrap();
    session_storage_obj.set_internal_field(0, v8::Integer::new(scope, 1).into());
    global.set(scope, v8::String::new(scope, "sessionStorage").unwrap().into(), session_storage_obj.into()).unwrap();
}

fn setup_dom<'s>(scope: &mut v8::HandleScope<'s>, context: v8::Local<v8::Context>) {
    // Define Node, Element, Document templates
    let node_tmpl = v8::FunctionTemplate::new(scope, |_, _, _| {});
    node_tmpl.set_class_name(v8::String::new(scope, "Node").unwrap());
    let node_inst = node_tmpl.instance_template(scope);
    node_inst.set_internal_field_count(1);

    node_inst.set_accessor(v8::String::new(scope, "nodeType").unwrap().into(), node_type_getter);
    node_inst.set_accessor(v8::String::new(scope, "nodeName").unwrap().into(), node_name_getter);

    let element_tmpl = v8::FunctionTemplate::new(scope, |_, _, _| {});
    element_tmpl.inherit(node_tmpl);
    element_tmpl.set_class_name(v8::String::new(scope, "Element").unwrap());
    let element_inst = element_tmpl.instance_template(scope);
    element_inst.set_accessor(v8::String::new(scope, "tagName").unwrap().into(), tag_name_getter);

    let doc_tmpl = v8::FunctionTemplate::new(scope, |_, _, _| {});
    doc_tmpl.inherit(node_tmpl);
    doc_tmpl.set_class_name(v8::String::new(scope, "Document").unwrap());
    let doc_inst = doc_tmpl.instance_template(scope);
    
    let create_element_fn = v8::FunctionTemplate::new(scope, create_element_callback);
    doc_inst.set(v8::String::new(scope, "createElement").unwrap().into(), create_element_fn.into());

    // Store templates in the context for later use (simplified)
    let global = context.global(scope);
    
    // Create 'document' object
    let data = unsafe {
        let ptr = context.get_aligned_pointer_in_embedder_data(0);
        &*(ptr as *const V8ContextData)
    };

    let doc_id = data.dom_tree.read().document_id();
    let doc_obj = doc_tmpl.get_function(scope).unwrap().new_instance(scope, &[]).unwrap();
    doc_obj.set_aligned_pointer_in_internal_field(0, doc_id as *mut std::ffi::c_void);
    
    global.set(scope, v8::String::new(scope, "document").unwrap().into(), doc_obj.into()).unwrap();

    // Also expose constructors
    global.set(scope, v8::String::new(scope, "Node").unwrap().into(), node_tmpl.get_function(scope).unwrap().into()).unwrap();
    global.set(scope, v8::String::new(scope, "Element").unwrap().into(), element_tmpl.get_function(scope).unwrap().into()).unwrap();
    global.set(scope, v8::String::new(scope, "Document").unwrap().into(), doc_tmpl.get_function(scope).unwrap().into()).unwrap();
}

/// Wrap a NodeId into a JS object
pub fn wrap_node<'s>(
    scope: &mut v8::HandleScope<'s>,
    node_id: NodeId,
) -> v8::Local<'s, v8::Object> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    
    let data = unsafe {
        let ptr = context.get_aligned_pointer_in_embedder_data(0);
        &*(ptr as *const V8ContextData)
    };

    let node_type = data.dom_tree.read().get_node(node_id).map(|n| n.node_type.clone()).unwrap_or(NodeType::Element);

    let constructor_name = match node_type {
        NodeType::Document => "Document",
        NodeType::Element => "Element",
        _ => "Node",
    };

    let constructor_val = global.get(scope, v8::String::new(scope, constructor_name).unwrap().into()).unwrap();
    let constructor = v8::Local::<v8::Function>::try_from(constructor_val).unwrap();
    let obj = constructor.new_instance(scope, &[]).unwrap();
    obj.set_aligned_pointer_in_internal_field(0, node_id as *mut std::ffi::c_void);
    obj
}

// --- Callbacks (Console, Timers, DOM, Storage) ---

fn console_log(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let message = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().log(&message);
}

fn console_info(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let message = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().info(&message);
}

fn console_warn(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let message = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().warn(&message);
}

fn console_error(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let message = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    get_context_data(scope).console_api.write().error(&message);
}

fn set_timeout_callback(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let delay = args.get(1).integer_value(scope).unwrap_or(0) as u64;
    
    let data = get_context_data(scope);
    let timer_id = data.timer_manager.write().set_timeout(0, delay);
    
    // Store the callback
    data.timer_callbacks.write().insert(timer_id, v8::Global::new(scope, callback));
    
    rv.set(v8::Number::new(scope, timer_id as f64).into());
}

fn set_interval_callback(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
    let interval = args.get(1).integer_value(scope).unwrap_or(0) as u64;
    
    let data = get_context_data(scope);
    let timer_id = data.timer_manager.write().set_interval(0, interval);
    
    // Store the callback
    data.timer_callbacks.write().insert(timer_id, v8::Global::new(scope, callback));
    
    rv.set(v8::Number::new(scope, timer_id as f64).into());
}

fn clear_timer_callback(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let timer_id = args.get(0).integer_value(scope).unwrap_or(0) as u64;
    let data = get_context_data(scope);
    data.timer_manager.write().clear_timer(timer_id);
    data.timer_callbacks.write().remove(&timer_id);
}

fn node_type_getter(scope: &mut v8::HandleScope, _name: v8::Local<v8::Name>, args: v8::PropertyCallbackArguments, mut rv: v8::ReturnValue) {
    let node_id = args.this().get_aligned_pointer_in_internal_field(0) as u64;
    if let Some(node) = get_context_data(scope).dom_tree.read().get_node(node_id) {
        let type_val = match node.node_type {
            NodeType::Element => 1,
            NodeType::Text => 3,
            NodeType::Comment => 8,
            NodeType::Document => 9,
            NodeType::DocumentFragment => 11,
        };
        rv.set(v8::Integer::new(scope, type_val).into());
    }
}

fn node_name_getter(scope: &mut v8::HandleScope, _name: v8::Local<v8::Name>, args: v8::PropertyCallbackArguments, mut rv: v8::ReturnValue) {
    let node_id = args.this().get_aligned_pointer_in_internal_field(0) as u64;
    if let Some(node) = get_context_data(scope).dom_tree.read().get_node(node_id) {
        let name = node.tag_name.clone().unwrap_or_else(|| "#text".to_string());
        rv.set(v8::String::new(scope, &name).unwrap().into());
    }
}

fn tag_name_getter(scope: &mut v8::HandleScope, _name: v8::Local<v8::Name>, args: v8::PropertyCallbackArguments, mut rv: v8::ReturnValue) {
    let node_id = args.this().get_aligned_pointer_in_internal_field(0) as u64;
    if let Some(node) = get_context_data(scope).dom_tree.read().get_node(node_id) {
        if let Some(ref tag) = node.tag_name {
            rv.set(v8::String::new(scope, &tag.to_uppercase()).unwrap().into());
        }
    }
}

fn create_element_callback(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let tag = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    let new_id = get_context_data(scope).dom_tree.write().create_element(&tag);
    rv.set(wrap_node(scope, new_id).into());
}

fn get_storage<'s>(scope: &mut v8::HandleScope<'s>, args: &v8::FunctionCallbackArguments<'s>) -> Arc<RwLock<StorageApi>> {
    let data = get_context_data(scope);
    let storage_type = args.this().get_internal_field(scope, 0).unwrap().integer_value(scope).unwrap_or(0);
    if storage_type == 0 {
        data.local_storage.clone()
    } else {
        data.session_storage.clone()
    }
}

fn storage_get_item(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue) {
    let key = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    let storage = get_storage(scope, &args);
    if let Some(val) = storage.read().get_item(&key) {
        rv.set(v8::String::new(scope, val).unwrap().into());
    } else {
        rv.set(v8::null(scope).into());
    }
}

fn storage_set_item(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let key = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    let value = args.get(1).to_string(scope).unwrap().to_rust_string_lossy(scope);
    let storage = get_storage(scope, &args);
    let _ = storage.write().set_item(&key, &value);
}

fn storage_remove_item(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let key = args.get(0).to_string(scope).unwrap().to_rust_string_lossy(scope);
    let storage = get_storage(scope, &args);
    storage.write().remove_item(&key);
}

fn storage_clear(scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue) {
    let storage = get_storage(scope, &args);
    storage.write().clear();
}

fn get_context_data(scope: &mut v8::HandleScope) -> &'static V8ContextData {
    unsafe {
        let ptr = scope.get_current_context().get_aligned_pointer_in_embedder_data(0);
        &*(ptr as *const V8ContextData)
    }
}
