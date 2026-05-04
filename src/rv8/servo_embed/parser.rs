use crate::servo_embed::dom::{DomTree, NodeId};
use html5ever::tokenizer::{
    BufferQueue, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use std::cell::RefCell;

/// HTML Parser bridge between html5ever and DomTree
struct DomBuilder<'a> {
    dom: RefCell<&'a mut DomTree>,
    stack: RefCell<Vec<NodeId>>,
}

impl<'a> DomBuilder<'a> {
    fn new(dom: &'a mut DomTree) -> Self {
        let doc_id = dom.document_id();
        DomBuilder {
            dom: RefCell::new(dom),
            stack: RefCell::new(vec![doc_id]),
        }
    }

    fn current_node(&self) -> Option<NodeId> {
        self.stack.borrow().last().copied()
    }
}

impl<'a> TokenSink for DomBuilder<'a> {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        match token {
            Token::TagToken(tag) => {
                // tag.name in tokenizer is likely just the local name (Atom)
                let tag_name = tag.name.to_string();

                match tag.kind {
                    html5ever::tokenizer::TagKind::StartTag => {
                        let node_id = self.dom.borrow_mut().create_element(&tag_name);

                        // Add attributes
                        // attr.name in tokenizer is also likely local name or QualName without prefix resolved?
                        // Actually tokenizer produces tags/attrs where name is LocalName (Atom).
                        if let Some(node) = self.dom.borrow_mut().get_node_mut(node_id) {
                            for attr in tag.attrs {
                                node.set_attribute(&attr.name.local.to_string(), &attr.value);
                            }
                        }

                        if let Some(parent_id) = self.current_node() {
                            self.dom.borrow_mut().append_child(parent_id, node_id);
                        }

                        // Push to stack unless it's a self-closing tag or void element
                        if !tag.self_closing {
                            let void_tags = [
                                "area", "base", "br", "col", "embed", "hr", "img", "input", "link",
                                "meta", "param", "source", "track", "wbr",
                            ];
                            if !void_tags.contains(&tag_name.as_str()) {
                                self.stack.borrow_mut().push(node_id);
                            }
                        }
                    }
                    html5ever::tokenizer::TagKind::EndTag => {
                        let mut stack = self.stack.borrow_mut();
                        // Pop from stack until we find the matching tag
                        // We need access to dom to check tag names of nodes in stack
                        // Borrowing dom here while stack is borrowed is fine (RefCells are separate)
                        let dom = self.dom.borrow();

                        if let Some(pos) = stack.iter().rposition(|&id| {
                            if let Some(node) = dom.get_node(id) {
                                node.tag_name.as_deref() == Some(&tag_name)
                            } else {
                                false
                            }
                        }) {
                            stack.truncate(pos);
                        }
                    }
                }
            }
            Token::CharacterTokens(s) => {
                if let Some(parent_id) = self.current_node() {
                    let node_id = self.dom.borrow_mut().create_text(&s);
                    self.dom.borrow_mut().append_child(parent_id, node_id);
                }
            }
            Token::CommentToken(_s) => {
                // Ignore comments for now
            }
            Token::DoctypeToken(_d) => {
                // Ignore doctype for now
            }
            Token::NullCharacterToken => {}
            Token::EOFToken => {}
            Token::ParseError(_e) => {}
        }
        TokenSinkResult::Continue
    }
}

/// Parse HTML string into DomTree
pub fn parse_html(html: &str, dom: &mut DomTree) {
    let builder = DomBuilder::new(dom);
    let tokenizer = Tokenizer::new(builder, TokenizerOpts::default());

    let mut queue = BufferQueue::default();
    queue.push_back(html.into());

    let _ = tokenizer.feed(&mut queue);
    tokenizer.end();
}
