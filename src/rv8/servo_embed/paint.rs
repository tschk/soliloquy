//! Software document painter for the in-process embed path.
//!
//! Renders parsed DOM into RGBA pixels until Servo's compositor feeds real surfaces.

use crate::renderer::RenderFrame;
use crate::servo_embed::dom::{DomTree, NodeId, NodeType};

const PAGE_BG: [u8; 4] = [248, 250, 252, 255];
const TEXT: [u8; 4] = [24, 24, 27, 255];
const MUTED: [u8; 4] = [113, 113, 122, 255];
const LINK: [u8; 4] = [37, 99, 235, 255];
const BORDER: [u8; 4] = [228, 228, 231, 255];
const LOADING: [u8; 4] = [234, 179, 8, 255];

const MARGIN_X: i32 = 28;
const MARGIN_TOP: i32 = 20;
const LINE_H: i32 = 18;
const CHAR_W: i32 = 7;

pub struct PaintContext<'a> {
    pub url: &'a str,
    pub title: &'a str,
    pub loading: bool,
}

pub fn paint_document_frame(frame: &mut RenderFrame, dom: &DomTree, ctx: &PaintContext<'_>) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }

    fill(frame, 0, 0, w, h, PAGE_BG);

    let host = display_host(ctx.url);
    let header = if ctx.title.is_empty() {
        host.clone()
    } else {
        format!("{} — {}", ctx.title, host)
    };
    draw_text(frame, MARGIN_X, MARGIN_TOP, &header, MUTED, w);
    fill(frame, MARGIN_X, MARGIN_TOP + LINE_H + 4, w - MARGIN_X * 2, 1, BORDER);

    let mut y = MARGIN_TOP + LINE_H + 16;
    let max_w = w - MARGIN_X * 2;

    if ctx.loading {
        fill(frame, MARGIN_X, y, max_w, 3, LOADING);
        y += 10;
    }

    let body = dom.body_root().unwrap_or_else(|| dom.document_id());
    y = paint_subtree(frame, dom, body, MARGIN_X, y, max_w, w, h, false);

    if y <= MARGIN_TOP + LINE_H + 20 {
        draw_text(
            frame,
            MARGIN_X,
            y + 8,
            "(no visible body content)",
            MUTED,
            w,
        );
    }
}

fn paint_subtree(
    frame: &mut RenderFrame,
    dom: &DomTree,
    node_id: NodeId,
    x: i32,
    mut y: i32,
    max_w: i32,
    frame_w: i32,
    frame_h: i32,
    in_head: bool,
) -> i32 {
    let Some(node) = dom.get_node(node_id) else {
        return y;
    };

    match node.node_type {
        NodeType::Text => {
            if in_head {
                return y;
            }
            if let Some(text) = &node.text_content {
                let trimmed = collapse_ws(text);
                if !trimmed.is_empty() {
                    y = draw_wrapped_text(frame, x, y, &trimmed, TEXT, max_w, frame_w, frame_h);
                }
            }
            return y;
        }
        NodeType::Comment | NodeType::DocumentFragment => return y,
        NodeType::Document => {
            for &child in &node.children {
                y = paint_subtree(frame, dom, child, x, y, max_w, frame_w, frame_h, in_head);
            }
            return y;
        }
        NodeType::Element => {
            let tag = node.tag_name.as_deref().unwrap_or("");
            let head = in_head || tag == "head";
            if matches!(tag, "script" | "style" | "meta" | "link" | "noscript" | "template") {
                return y;
            }
            if tag == "title" {
                return y;
            }
            if tag == "br" && !head {
                return y + LINE_H;
            }

            if !head {
                let (color, gap_before, gap_after) = element_style(tag);
                y += gap_before;
                for &child in &node.children {
                    if dom.get_node(child).is_some_and(|n| n.node_type == NodeType::Text) {
                        if let Some(text) = dom.get_node(child).and_then(|n| n.text_content.clone()) {
                            let trimmed = collapse_ws(&text);
                            if !trimmed.is_empty() {
                                y = draw_wrapped_text(
                                    frame, x, y, &trimmed, color, max_w, frame_w, frame_h,
                                );
                            }
                        }
                    } else {
                        y = paint_subtree(
                            frame, dom, child, x, y, max_w, frame_w, frame_h, head,
                        );
                    }
                }
                y += gap_after;
                return y;
            }

            for &child in &node.children {
                y = paint_subtree(frame, dom, child, x, y, max_w, frame_w, frame_h, head);
            }
            return y;
        }
    }
}

fn element_style(tag: &str) -> ([u8; 4], i32, i32) {
    match tag {
        "h1" => (TEXT, 8, 12),
        "h2" => (TEXT, 6, 10),
        "h3" | "h4" => (TEXT, 4, 8),
        "a" => (LINK, 2, 6),
        "code" | "pre" => ([39, 39, 42, 255], 4, 8),
        "blockquote" => (MUTED, 6, 10),
        "li" => (TEXT, 2, 4),
        "p" | "div" | "section" | "article" | "main" | "header" | "footer" | "nav" => {
            (TEXT, 6, 8)
        }
        _ => (TEXT, 2, 4),
    }
}

fn draw_wrapped_text(
    frame: &mut RenderFrame,
    x: i32,
    mut y: i32,
    text: &str,
    color: [u8; 4],
    max_w: i32,
    frame_w: i32,
    frame_h: i32,
) -> i32 {
    let max_chars = (max_w / CHAR_W).max(8) as usize;
    for word in text.split_whitespace() {
        if y + LINE_H >= frame_h - 8 {
            break;
        }
        let mut line = String::new();
        for piece in chunk_word(word, max_chars) {
            if line.is_empty() {
                line = piece;
            } else if line.len() + 1 + piece.len() <= max_chars {
                line.push(' ');
                line.push_str(&piece);
            } else {
                draw_text(frame, x, y, &line, color, frame_w);
                y += LINE_H;
                line = piece;
            }
        }
        if !line.is_empty() {
            draw_text(frame, x, y, &line, color, frame_w);
            y += LINE_H;
        }
    }
    y
}

fn chunk_word(word: &str, max_chars: usize) -> Vec<String> {
    if word.len() <= max_chars {
        return vec![word.to_string()];
    }
    word.as_bytes()
        .chunks(max_chars)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect()
}

fn draw_text(frame: &mut RenderFrame, x: i32, y: i32, text: &str, color: [u8; 4], frame_w: i32) {
    let mut cx = x;
    for ch in text.chars().take(512) {
        if cx + CHAR_W >= frame_w - 4 {
            break;
        }
        draw_char(frame, cx, y, ch, color);
        cx += CHAR_W;
    }
}

fn draw_char(frame: &mut RenderFrame, x: i32, y: i32, ch: char, color: [u8; 4]) {
    let glyph = glyph_rows(ch);
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) != 0 {
                fill(frame, x + col, y + row as i32, 1, 1, color);
            }
        }
    }
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        ' ' => [0; 7],
        '0'..='9' => digit_glyph(ch),
        'a'..='z' | 'A'..='Z' => letter_glyph(ch.to_ascii_uppercase()),
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x06],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x06, 0x06, 0x04],
        ':' => [0x00, 0x06, 0x06, 0x00, 0x06, 0x06, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '/' => [0x01, 0x02, 0x04, 0x08, 0x10, 0x00, 0x00],
        '?' => [0x0E, 0x11, 0x02, 0x04, 0x00, 0x04, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F],
    }
}

fn letter_glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x0A, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        _ => [0x1F, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1F],
    }
}

fn digit_glyph(ch: char) -> [u8; 7] {
    match ch {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x06, 0x08, 0x10, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x06, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        _ => [0; 7],
    }
}

fn fill(frame: &mut RenderFrame, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    let fw = frame.width as i32;
    let fh = frame.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + w).min(fw);
    let y1 = (y + h).min(fh);
    for py in y0..y1 {
        for px in x0..x1 {
            let i = ((py * fw + px) * 4) as usize;
            if i + 3 < frame.pixels.len() {
                frame.pixels[i..i + 4].copy_from_slice(&color);
            }
        }
    }
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

pub fn display_host(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            return host.to_string();
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::servo_embed::dom::DomTree;

    #[test]
    fn paints_non_empty_frame() {
        let mut dom = DomTree::new();
        let p = dom.create_element("p");
        let text = dom.create_text("Hello RV8");
        dom.append_child(dom.document_id(), p);
        dom.append_child(p, text);

        let mut frame = RenderFrame::new(400, 300);
        paint_document_frame(
            &mut frame,
            &dom,
            &PaintContext {
                url: "https://example.com",
                title: "Example",
                loading: false,
            },
        );

        let non_bg = frame
            .pixels
            .chunks_exact(4)
            .any(|px| px != PAGE_BG && px != BORDER);
        assert!(non_bg);
    }
}
