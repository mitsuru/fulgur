//! Convert a Blitz DOM (after style resolution + layout) into a Pageable tree.

use crate::pageable::{BlockPageable, Pageable, SpacerPageable};
use crate::paragraph::{ParagraphPageable, ShapedGlyph, ShapedGlyphRun, ShapedLine};
use blitz_dom::{Node, NodeData};
use blitz_html::HtmlDocument;
use std::ops::Deref;
use std::sync::Arc;

/// Convert a resolved Blitz document into a Pageable tree.
pub fn dom_to_pageable(doc: &HtmlDocument) -> Box<dyn Pageable> {
    let root = doc.root_element();
    convert_node(doc.deref(), root.id)
}

fn convert_node(doc: &blitz_dom::BaseDocument, node_id: usize) -> Box<dyn Pageable> {
    let node = doc.get_node(node_id).unwrap();
    let layout = node.final_layout;
    let height = layout.size.height;
    let width = layout.size.width;

    // Check if this is an inline root (contains text layout)
    if node.flags.is_inline_root() {
        if let Some(paragraph) = extract_paragraph(doc, node) {
            return Box::new(paragraph);
        }
    }

    let children: &[usize] = &node.children;

    if children.is_empty() {
        // Leaf node — create a spacer with the computed height
        let mut spacer = SpacerPageable::new(height);
        spacer.wrap(width, height);
        return Box::new(spacer);
    }

    // Container node — recurse into children
    let child_pageables: Vec<Box<dyn Pageable>> = children
        .iter()
        .filter_map(|&child_id| {
            let child = doc.get_node(child_id)?;
            // Skip comment nodes
            if matches!(&child.data, NodeData::Comment) {
                return None;
            }
            Some(convert_node(doc, child_id))
        })
        .collect();

    let mut block = BlockPageable::new(child_pageables);
    block.wrap(width, 10000.0);
    Box::new(block)
}

/// Extract a ParagraphPageable from an inline root node.
fn extract_paragraph(doc: &blitz_dom::BaseDocument, node: &Node) -> Option<ParagraphPageable> {
    let elem_data = node.element_data()?;
    let text_layout = elem_data.inline_layout_data.as_ref()?;

    let parley_layout = &text_layout.layout;
    let text = &text_layout.text;

    let mut shaped_lines = Vec::new();

    for line in parley_layout.lines() {
        let metrics = line.metrics();
        let mut glyph_runs = Vec::new();

        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let font_data = run.font();
                let font_bytes: Vec<u8> = font_data.data.data().to_vec();
                let font_index = font_data.index;
                let font_size = run.font_size();

                // Get text color from the brush (node ID) → computed styles
                let brush = &glyph_run.style().brush;
                let color = get_text_color(doc, brush.id);

                // Extract positioned glyphs
                let mut glyphs = Vec::new();
                for g in glyph_run.positioned_glyphs() {
                    glyphs.push(ShapedGlyph {
                        id: g.id,
                        x_advance: g.advance / font_size,
                        x_offset: g.x / font_size,
                        y_offset: g.y / font_size,
                        text_range: 0..1, // Simplified range
                    });
                }

                if !glyphs.is_empty() {
                    // Extract the text segment for this run
                    let run_text = text.clone();

                    glyph_runs.push(ShapedGlyphRun {
                        font_data: Arc::new(font_bytes),
                        font_index,
                        font_size,
                        color,
                        glyphs,
                        text: run_text,
                        x_offset: 0.0,
                    });
                }
            }
        }

        shaped_lines.push(ShapedLine {
            height: metrics.line_height,
            baseline: metrics.baseline,
            glyph_runs,
        });
    }

    if shaped_lines.is_empty() {
        return None;
    }

    Some(ParagraphPageable::new(shaped_lines))
}

/// Get text color from a DOM node's computed styles.
fn get_text_color(doc: &blitz_dom::BaseDocument, node_id: usize) -> [u8; 4] {
    if let Some(node) = doc.get_node(node_id) {
        if let Some(styles) = node.primary_styles() {
            let color = styles.clone_color();
            // AbsoluteColor has components (f32, f32, f32) in 0.0-1.0 range and alpha as f32
            let r = (color.components.0.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.components.1.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.components.2.clamp(0.0, 1.0) * 255.0) as u8;
            let a = (color.alpha.clamp(0.0, 1.0) * 255.0) as u8;
            return [r, g, b, a];
        }
    }
    [0, 0, 0, 255] // Default: black
}
