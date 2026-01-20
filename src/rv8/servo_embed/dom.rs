//! DOM bindings for V8
//!
//! Provides JavaScript bindings for DOM manipulation that bridge
//! V8 execution with Servo's DOM representation.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// DOM node identifier
pub type NodeId = u64;

/// DOM node types
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Element,
    Text,
    Comment,
    Document,
    DocumentFragment,
}

/// DOM node representation for V8 bindings
#[derive(Debug, Clone)]
pub struct DomNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub tag_name: Option<String>,
    pub text_content: Option<String>,
    pub parent_id: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub attributes: HashMap<String, String>,
}

impl DomNode {
    pub fn new_element(id: NodeId, tag: &str) -> Self {
        DomNode {
            id,
            node_type: NodeType::Element,
            tag_name: Some(tag.to_lowercase()),
            text_content: None,
            parent_id: None,
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn new_text(id: NodeId, text: &str) -> Self {
        DomNode {
            id,
            node_type: NodeType::Text,
            tag_name: None,
            text_content: Some(text.to_string()),
            parent_id: None,
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        self.attributes.insert(name.to_string(), value.to_string());
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }
}

/// DOM tree for V8 bindings
pub struct DomTree {
    nodes: HashMap<NodeId, DomNode>,
    next_id: NodeId,
    document_id: NodeId,
}

impl DomTree {
    pub fn new() -> Self {
        let doc_id = 1;
        let mut nodes = HashMap::new();

        nodes.insert(
            doc_id,
            DomNode {
                id: doc_id,
                node_type: NodeType::Document,
                tag_name: Some("#document".to_string()),
                text_content: None,
                parent_id: None,
                children: Vec::new(),
                attributes: HashMap::new(),
            },
        );

        DomTree {
            nodes,
            next_id: 2,
            document_id: doc_id,
        }
    }

    pub fn create_element(&mut self, tag: &str) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, DomNode::new_element(id, tag));
        id
    }

    pub fn create_text(&mut self, text: &str) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, DomNode::new_text(id, text));
        id
    }

    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) -> bool {
        // Set child's parent
        if let Some(child) = self.nodes.get_mut(&child_id) {
            child.parent_id = Some(parent_id);
        } else {
            return false;
        }

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(child_id);
            true
        } else {
            false
        }
    }

    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) -> bool {
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.retain(|&id| id != child_id);

            if let Some(child) = self.nodes.get_mut(&child_id) {
                child.parent_id = None;
            }
            true
        } else {
            false
        }
    }

    pub fn get_node(&self, id: NodeId) -> Option<&DomNode> {
        self.nodes.get(&id)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut DomNode> {
        self.nodes.get_mut(&id)
    }

    pub fn document(&self) -> &DomNode {
        self.nodes.get(&self.document_id).unwrap()
    }

    pub fn document_id(&self) -> NodeId {
        self.document_id
    }

    /// Query selector (simplified)
    pub fn query_selector(&self, selector: &str) -> Option<NodeId> {
        // Very simplified selector matching
        // Real implementation would use a proper CSS selector parser

        for (&id, node) in &self.nodes {
            if let Some(ref tag) = node.tag_name {
                // Tag selector
                if selector == tag {
                    return Some(id);
                }
                // ID selector
                if selector.starts_with('#') {
                    if let Some(id_attr) = node.get_attribute("id") {
                        if &format!("#{}", id_attr) == selector {
                            return Some(id);
                        }
                    }
                }
                // Class selector
                if selector.starts_with('.') {
                    if let Some(class_attr) = node.get_attribute("class") {
                        let class_name = &selector[1..];
                        if class_attr.split_whitespace().any(|c| c == class_name) {
                            return Some(id);
                        }
                    }
                }
            }
        }

        None
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}
