//! DOM bindings for V8
//!
//! Provides JavaScript bindings for DOM manipulation that bridge
//! V8 execution with Servo's DOM representation.

use std::collections::HashMap;

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

/// DOM mutation emitted by V8 bindings for Servo-side synchronization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomMutation {
    ChildAppended {
        parent_id: NodeId,
        child_id: NodeId,
    },
    ChildRemoved {
        parent_id: NodeId,
        child_id: NodeId,
    },
    AttributeSet {
        node_id: NodeId,
        name: String,
        value: String,
    },
    TextSet {
        node_id: NodeId,
        text: String,
    },
}

/// Host event routed into V8 listeners.
#[derive(Debug, Clone, PartialEq)]
pub struct DomEvent {
    pub event_type: String,
    pub target_id: NodeId,
    pub client_x: Option<f32>,
    pub client_y: Option<f32>,
    pub button: Option<u8>,
    pub key: Option<String>,
}

impl DomEvent {
    pub fn mouse(
        event_type: &str,
        target_id: NodeId,
        client_x: f32,
        client_y: f32,
        button: crate::servo_embed::MouseButton,
    ) -> Self {
        let button = match button {
            crate::servo_embed::MouseButton::Left => 0,
            crate::servo_embed::MouseButton::Middle => 1,
            crate::servo_embed::MouseButton::Right => 2,
        };

        Self {
            event_type: event_type.to_string(),
            target_id,
            client_x: Some(client_x),
            client_y: Some(client_y),
            button: Some(button),
            key: None,
        }
    }

    pub fn key(event_type: &str, target_id: NodeId, key: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            target_id,
            client_x: None,
            client_y: None,
            button: None,
            key: Some(key.to_string()),
        }
    }
}

/// DOM tree for V8 bindings
pub struct DomTree {
    nodes: HashMap<NodeId, DomNode>,
    next_id: NodeId,
    document_id: NodeId,
    mutations: Vec<DomMutation>,
    events: Vec<DomEvent>,
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
            mutations: Vec::new(),
            events: Vec::new(),
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
        if !self.nodes.contains_key(&parent_id) || !self.nodes.contains_key(&child_id) {
            return false;
        }

        if let Some(old_parent_id) = self.nodes.get(&child_id).and_then(|child| child.parent_id) {
            if old_parent_id != parent_id {
                self.remove_child(old_parent_id, child_id);
            }
        }

        if let Some(child) = self.nodes.get_mut(&child_id) {
            child.parent_id = Some(parent_id);
        }

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.retain(|&id| id != child_id);
            parent.children.push(child_id);
        }

        self.mutations.push(DomMutation::ChildAppended {
            parent_id,
            child_id,
        });
        true
    }

    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) -> bool {
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            let original_len = parent.children.len();
            parent.children.retain(|&id| id != child_id);
            if original_len == parent.children.len() {
                return false;
            }

            if let Some(child) = self.nodes.get_mut(&child_id) {
                child.parent_id = None;
            }
            self.mutations.push(DomMutation::ChildRemoved {
                parent_id,
                child_id,
            });
            true
        } else {
            false
        }
    }

    pub fn set_attribute(&mut self, node_id: NodeId, name: &str, value: &str) -> bool {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return false;
        };
        node.set_attribute(name, value);
        self.mutations.push(DomMutation::AttributeSet {
            node_id,
            name: name.to_string(),
            value: value.to_string(),
        });
        true
    }

    pub fn set_text_content(&mut self, node_id: NodeId, text: &str) -> bool {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return false;
        };
        node.text_content = Some(text.to_string());
        self.mutations.push(DomMutation::TextSet {
            node_id,
            text: text.to_string(),
        });
        true
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

    pub fn document_title(&self) -> Option<String> {
        for (&id, node) in &self.nodes {
            if node.tag_name.as_deref() == Some("title") {
                let text = self.text_content(id);
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        None
    }

    pub fn body_root(&self) -> Option<NodeId> {
        if let Some(id) = self.find_element_tag(self.document_id, "body") {
            return Some(id);
        }
        self.find_element_tag(self.document_id, "html")
    }

    pub fn text_content(&self, node_id: NodeId) -> String {
        let mut out = String::new();
        self.append_text(node_id, &mut out);
        out.trim().to_string()
    }

    fn append_text(&self, node_id: NodeId, out: &mut String) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        if let Some(ref text) = node.text_content {
            out.push_str(text);
        }
        for &child in &node.children {
            self.append_text(child, out);
        }
    }

    fn find_element_tag(&self, start: NodeId, tag: &str) -> Option<NodeId> {
        let Some(node) = self.nodes.get(&start) else {
            return None;
        };
        if node.tag_name.as_deref() == Some(tag) {
            return Some(start);
        }
        for &child in &node.children {
            if let Some(found) = self.find_element_tag(child, tag) {
                return Some(found);
            }
        }
        None
    }

    pub fn take_mutations(&mut self) -> Vec<DomMutation> {
        std::mem::take(&mut self.mutations)
    }

    pub fn record_event(&mut self, event: DomEvent) {
        self.events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<DomEvent> {
        std::mem::take(&mut self.events)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_child_should_record_mutation() {
        let mut tree = DomTree::new();
        let parent = tree.create_element("main");
        let child = tree.create_element("section");

        assert!(tree.append_child(parent, child));
        assert_eq!(
            tree.get_node(child).and_then(|node| node.parent_id),
            Some(parent)
        );
        assert_eq!(
            tree.take_mutations(),
            vec![DomMutation::ChildAppended {
                parent_id: parent,
                child_id: child,
            }]
        );
    }

    #[test]
    fn append_child_should_not_orphan_on_missing_parent() {
        let mut tree = DomTree::new();
        let child = tree.create_element("section");

        assert!(!tree.append_child(999, child));
        assert_eq!(tree.get_node(child).and_then(|node| node.parent_id), None);
        assert!(tree.take_mutations().is_empty());
    }

    #[test]
    fn set_attribute_should_record_mutation() {
        let mut tree = DomTree::new();
        let node_id = tree.create_element("section");

        assert!(tree.set_attribute(node_id, "id", "root"));
        assert_eq!(tree.query_selector("#root"), Some(node_id));
        assert_eq!(
            tree.take_mutations(),
            vec![DomMutation::AttributeSet {
                node_id,
                name: "id".to_string(),
                value: "root".to_string(),
            }]
        );
    }
}
