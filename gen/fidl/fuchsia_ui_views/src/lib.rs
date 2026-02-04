//! Fuchsia UI Views FIDL Protocol Implementation
//!
//! This module implements the Views protocol for managing UI views,
//! viewports, and view references in Flatland-based applications.
//!
//! Protocols:
//! - ViewRef: Reference to a view for focus and accessibility
//! - View: The view's server-side representation
//! - Viewport: A hole in the scene graph that hosts a child view

#![allow(unused)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique view identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u64);

static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);

impl ViewId {
    pub fn new() -> Self {
        Self(NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ViewId {
    fn default() -> Self {
        Self::new()
    }
}

/// Viewport identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewportId(pub u64);

static NEXT_VIEWPORT_ID: AtomicU64 = AtomicU64::new(1);

impl ViewportId {
    pub fn new() -> Self {
        Self(NEXT_VIEWPORT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ViewportId {
    fn default() -> Self {
        Self::new()
    }
}

/// View reference - immutable handle for focus/accessibility
#[derive(Debug, Clone)]
pub struct ViewRef {
    pub koid: u64,
    pub related_koid: u64,
}

impl ViewRef {
    pub fn new(koid: u64) -> Self {
        Self {
            koid,
            related_koid: koid + 1,
        }
    }

    pub fn get_koid(&self) -> u64 {
        self.koid
    }
}

/// View reference control - used to invalidate ViewRef
#[derive(Debug)]
pub struct ViewRefControl {
    pub koid: u64,
    pub related_koid: u64,
}

impl ViewRefControl {
    pub fn new(related_koid: u64) -> Self {
        Self {
            koid: related_koid + 1,
            related_koid,
        }
    }
}

/// Create a ViewRef/ViewRefControl pair
pub fn create_view_ref_pair() -> (ViewRef, ViewRefControl) {
    static NEXT_KOID: AtomicU64 = AtomicU64::new(1000);
    let koid = NEXT_KOID.fetch_add(2, Ordering::Relaxed);
    (ViewRef::new(koid), ViewRefControl::new(koid))
}

/// View creation token
#[derive(Debug)]
pub struct ViewCreationToken {
    pub value: u64,
}

/// Viewport creation token
#[derive(Debug)]
pub struct ViewportCreationToken {
    pub value: u64,
}

/// Create linked view/viewport tokens
pub fn create_view_tokens() -> (ViewCreationToken, ViewportCreationToken) {
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let value = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    (
        ViewCreationToken { value },
        ViewportCreationToken { value },
    )
}

/// Viewport properties
#[derive(Debug, Clone, Default)]
pub struct ViewportProperties {
    pub bounds: Option<Rect>,
    pub inset: Option<Inset>,
    pub focusable: bool,
}

/// Rectangle bounds
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width &&
        py >= self.y && py < self.y + self.height
    }
}

/// Inset for safe area
#[derive(Debug, Clone, Copy, Default)]
pub struct Inset {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// View state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewState {
    Created,
    Attached,
    Rendered,
    Hidden,
    Destroyed,
}

/// View focus state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    NotFocusable,
    Unfocused,
    Focused,
}

/// View instance
#[derive(Debug)]
pub struct View {
    pub id: ViewId,
    pub view_ref: ViewRef,
    pub state: ViewState,
    pub focus_state: FocusState,
    pub connected_viewport: Option<ViewportId>,
    pub properties: ViewportProperties,
    pub children: Vec<ViewportId>,
    pub debug_name: Option<String>,
}

impl View {
    pub fn new(view_ref: ViewRef) -> Self {
        Self {
            id: ViewId::new(),
            view_ref,
            state: ViewState::Created,
            focus_state: FocusState::NotFocusable,
            connected_viewport: None,
            properties: ViewportProperties::default(),
            children: Vec::new(),
            debug_name: None,
        }
    }

    pub fn with_debug_name(mut self, name: impl Into<String>) -> Self {
        self.debug_name = Some(name.into());
        self
    }

    pub fn is_focusable(&self) -> bool {
        self.focus_state != FocusState::NotFocusable && self.properties.focusable
    }

    pub fn set_focusable(&mut self, focusable: bool) {
        self.properties.focusable = focusable;
        if focusable && self.focus_state == FocusState::NotFocusable {
            self.focus_state = FocusState::Unfocused;
        }
    }

    pub fn add_child_viewport(&mut self, viewport_id: ViewportId) {
        if !self.children.contains(&viewport_id) {
            self.children.push(viewport_id);
        }
    }

    pub fn remove_child_viewport(&mut self, viewport_id: ViewportId) {
        self.children.retain(|&id| id != viewport_id);
    }
}

/// Viewport instance (hole in parent view that hosts child view)
#[derive(Debug)]
pub struct Viewport {
    pub id: ViewportId,
    pub parent_view: Option<ViewId>,
    pub child_view: Option<ViewId>,
    pub properties: ViewportProperties,
    pub token_value: u64,
}

impl Viewport {
    pub fn new(token: ViewportCreationToken) -> Self {
        Self {
            id: ViewportId::new(),
            parent_view: None,
            child_view: None,
            properties: ViewportProperties::default(),
            token_value: token.value,
        }
    }

    pub fn set_properties(&mut self, props: ViewportProperties) {
        self.properties = props;
    }

    pub fn has_child(&self) -> bool {
        self.child_view.is_some()
    }
}

/// Focus chain - ordered list of ViewRefs from root to focused view
#[derive(Debug, Clone, Default)]
pub struct FocusChain {
    pub view_refs: Vec<ViewRef>,
}

impl FocusChain {
    pub fn new() -> Self {
        Self { view_refs: Vec::new() }
    }

    pub fn push(&mut self, view_ref: ViewRef) {
        self.view_refs.push(view_ref);
    }

    pub fn get_focused(&self) -> Option<&ViewRef> {
        self.view_refs.last()
    }

    pub fn contains(&self, koid: u64) -> bool {
        self.view_refs.iter().any(|vr| vr.koid == koid)
    }
}

/// View tree for managing the hierarchy
pub struct ViewTree {
    views: HashMap<ViewId, View>,
    viewports: HashMap<ViewportId, Viewport>,
    view_ref_to_view: HashMap<u64, ViewId>,
    token_to_viewport: HashMap<u64, ViewportId>,
    root_view: Option<ViewId>,
    focus_chain: FocusChain,
}

impl ViewTree {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
            viewports: HashMap::new(),
            view_ref_to_view: HashMap::new(),
            token_to_viewport: HashMap::new(),
            root_view: None,
            focus_chain: FocusChain::new(),
        }
    }

    /// Create a new view in the tree
    pub fn create_view(&mut self, view_ref: ViewRef) -> ViewId {
        let view = View::new(view_ref.clone());
        let id = view.id;
        self.view_ref_to_view.insert(view_ref.koid, id);
        self.views.insert(id, view);
        id
    }

    /// Create a new viewport in the tree
    pub fn create_viewport(&mut self, token: ViewportCreationToken) -> ViewportId {
        let viewport = Viewport::new(token);
        let id = viewport.id;
        let token_value = viewport.token_value;
        self.token_to_viewport.insert(token_value, id);
        self.viewports.insert(id, viewport);
        id
    }

    /// Connect view to viewport (via matching tokens)
    pub fn connect(&mut self, view_token: &ViewCreationToken, viewport_id: ViewportId) -> bool {
        if let Some(&vp_id) = self.token_to_viewport.get(&view_token.value) {
            if vp_id != viewport_id {
                return false;
            }
        }

        if let Some(_viewport) = self.viewports.get_mut(&viewport_id) {
            true
        } else {
            false
        }
    }

    /// Set the root view
    pub fn set_root(&mut self, view_id: ViewId) {
        self.root_view = Some(view_id);
        if let Some(view) = self.views.get_mut(&view_id) {
            view.state = ViewState::Attached;
        }
    }

    /// Get view by ID
    pub fn get_view(&self, id: ViewId) -> Option<&View> {
        self.views.get(&id)
    }

    /// Get mutable view by ID
    pub fn get_view_mut(&mut self, id: ViewId) -> Option<&mut View> {
        self.views.get_mut(&id)
    }

    /// Get view by ViewRef koid
    pub fn get_view_by_ref(&self, koid: u64) -> Option<&View> {
        self.view_ref_to_view.get(&koid)
            .and_then(|id| self.views.get(id))
    }

    /// Get viewport by ID
    pub fn get_viewport(&self, id: ViewportId) -> Option<&Viewport> {
        self.viewports.get(&id)
    }

    /// Get mutable viewport by ID
    pub fn get_viewport_mut(&mut self, id: ViewportId) -> Option<&mut Viewport> {
        self.viewports.get_mut(&id)
    }

    /// Request focus for a view
    pub fn request_focus(&mut self, view_id: ViewId) -> bool {
        let should_focus = if let Some(view) = self.views.get(&view_id) {
            view.is_focusable() && view.state == ViewState::Attached
        } else {
            false
        };

        if should_focus {
            // Unfocus previously focused view
            for v in self.views.values_mut() {
                if v.focus_state == FocusState::Focused {
                    v.focus_state = FocusState::Unfocused;
                }
            }

            // Focus this view
            if let Some(view) = self.views.get_mut(&view_id) {
                view.focus_state = FocusState::Focused;
            }

            // Rebuild focus chain
            self.rebuild_focus_chain(view_id);
            return true;
        }
        false
    }

    /// Rebuild focus chain from root to focused view
    fn rebuild_focus_chain(&mut self, focused_id: ViewId) {
        self.focus_chain = FocusChain::new();
        
        if let Some(view) = self.views.get(&focused_id) {
            self.focus_chain.push(view.view_ref.clone());
        }
    }

    /// Get the current focus chain
    pub fn get_focus_chain(&self) -> &FocusChain {
        &self.focus_chain
    }

    /// Destroy a view
    pub fn destroy_view(&mut self, view_id: ViewId) -> bool {
        if let Some(view) = self.views.remove(&view_id) {
            self.view_ref_to_view.remove(&view.view_ref.koid);
            
            for vp_id in view.children {
                if let Some(viewport) = self.viewports.get_mut(&vp_id) {
                    viewport.parent_view = None;
                }
            }
            
            if self.root_view == Some(view_id) {
                self.root_view = None;
            }
            
            true
        } else {
            false
        }
    }

    /// Destroy a viewport
    pub fn destroy_viewport(&mut self, viewport_id: ViewportId) -> bool {
        if let Some(viewport) = self.viewports.remove(&viewport_id) {
            self.token_to_viewport.remove(&viewport.token_value);
            
            if let Some(parent_id) = viewport.parent_view {
                if let Some(parent) = self.views.get_mut(&parent_id) {
                    parent.remove_child_viewport(viewport_id);
                }
            }
            
            if let Some(child_id) = viewport.child_view {
                if let Some(child) = self.views.get_mut(&child_id) {
                    child.connected_viewport = None;
                    child.state = ViewState::Created;
                }
            }
            
            true
        } else {
            false
        }
    }
}

impl Default for ViewTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Focuser protocol implementation
pub struct Focuser {
    view_tree: Arc<Mutex<ViewTree>>,
}

impl Focuser {
    pub fn new(view_tree: Arc<Mutex<ViewTree>>) -> Self {
        Self { view_tree }
    }

    /// Request focus for a view
    pub fn request_focus(&self, view_ref: &ViewRef) -> Result<(), &'static str> {
        let mut tree = self.view_tree.lock().unwrap();
        
        if let Some(&view_id) = tree.view_ref_to_view.get(&view_ref.koid) {
            if tree.request_focus(view_id) {
                Ok(())
            } else {
                Err("View is not focusable")
            }
        } else {
            Err("View not found")
        }
    }

    /// Set auto focus behavior
    pub fn set_auto_focus(&self, _enabled: bool) -> Result<(), &'static str> {
        Ok(())
    }
}

/// ViewIdentity for view creation
#[derive(Debug)]
pub struct ViewIdentityOnCreation {
    pub view_ref: ViewRef,
    pub view_ref_control: ViewRefControl,
}

impl ViewIdentityOnCreation {
    pub fn new() -> Self {
        let (view_ref, view_ref_control) = create_view_ref_pair();
        Self { view_ref, view_ref_control }
    }
}

impl Default for ViewIdentityOnCreation {
    fn default() -> Self {
        Self::new()
    }
}

/// View bound protocols configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewBoundProtocols {
    pub view_focuser: bool,
    pub view_ref_focused: bool,
    pub touch_source: bool,
    pub mouse_source: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_view_ref_pair() {
        let (view_ref, control) = create_view_ref_pair();
        assert_eq!(view_ref.related_koid, control.related_koid);
        assert_eq!(view_ref.koid, control.related_koid);
    }

    #[test]
    fn test_create_view_tokens() {
        let (view_token, viewport_token) = create_view_tokens();
        assert_eq!(view_token.value, viewport_token.value);
    }

    #[test]
    fn test_view_tree() {
        let mut tree = ViewTree::new();
        
        let (view_ref, _control) = create_view_ref_pair();
        let view_id = tree.create_view(view_ref.clone());
        
        assert!(tree.get_view(view_id).is_some());
        assert!(tree.get_view_by_ref(view_ref.koid).is_some());
    }

    #[test]
    fn test_viewport() {
        let mut tree = ViewTree::new();
        
        let (_, viewport_token) = create_view_tokens();
        let viewport_id = tree.create_viewport(viewport_token);
        
        let viewport = tree.get_viewport_mut(viewport_id).unwrap();
        viewport.set_properties(ViewportProperties {
            bounds: Some(Rect::new(0.0, 0.0, 800.0, 600.0)),
            inset: None,
            focusable: true,
        });
        
        assert!(tree.get_viewport(viewport_id).unwrap().properties.focusable);
    }

    #[test]
    fn test_focus() {
        let mut tree = ViewTree::new();
        
        let (view_ref, _) = create_view_ref_pair();
        let view_id = tree.create_view(view_ref);
        tree.set_root(view_id);
        
        {
            let view = tree.get_view_mut(view_id).unwrap();
            view.set_focusable(true);
        }
        
        assert!(tree.request_focus(view_id));
        
        let view = tree.get_view(view_id).unwrap();
        assert_eq!(view.focus_state, FocusState::Focused);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        
        assert!(rect.contains(10.0, 20.0));
        assert!(rect.contains(50.0, 40.0));
        assert!(!rect.contains(5.0, 20.0));
        assert!(!rect.contains(120.0, 40.0));
    }

    #[test]
    fn test_destroy_view() {
        let mut tree = ViewTree::new();
        
        let (view_ref, _) = create_view_ref_pair();
        let koid = view_ref.koid;
        let view_id = tree.create_view(view_ref);
        
        assert!(tree.get_view(view_id).is_some());
        assert!(tree.destroy_view(view_id));
        assert!(tree.get_view(view_id).is_none());
        assert!(tree.get_view_by_ref(koid).is_none());
    }
}
