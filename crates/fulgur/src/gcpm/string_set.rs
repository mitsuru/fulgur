//! Named string support for CSS Generated Content for Paged Media (GCPM).
//!
//! Manages string-set values extracted from the DOM via `string-set: name content(text)`.
//! Values are stored with their DOM node IDs for later insertion into the Pageable tree.

/// A single string-set entry extracted from the DOM.
#[derive(Debug, Clone)]
pub struct StringSetEntry {
    /// The named string identifier (e.g. "chapter-title").
    pub name: String,
    /// The resolved text value.
    pub value: String,
    /// Blitz DOM node ID, used to position the marker in the Pageable tree.
    pub node_id: usize,
}

/// Stores string-set entries collected during DOM traversal.
pub struct StringSetStore {
    entries: Vec<StringSetEntry>,
}

impl StringSetStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, entry: StringSetEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[StringSetEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for StringSetStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the text content of a DOM node (recursive).
pub fn extract_text_content(doc: &blitz_dom::BaseDocument, node_id: usize) -> String {
    let mut out = String::new();
    collect_text(doc, node_id, &mut out);
    out
}

fn collect_text(doc: &blitz_dom::BaseDocument, node_id: usize, out: &mut String) {
    let Some(node) = doc.get_node(node_id) else {
        return;
    };
    match &node.data {
        blitz_dom::NodeData::Text(text_data) => out.push_str(&text_data.content),
        _ => {
            for &child_id in &node.children {
                collect_text(doc, child_id, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_set_store_basic() {
        let mut store = StringSetStore::new();
        assert!(store.is_empty());
        store.push(StringSetEntry {
            name: "title".into(),
            value: "Chapter 1".into(),
            node_id: 42,
        });
        assert!(!store.is_empty());
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].name, "title");
    }

    #[test]
    fn test_string_set_store_multiple() {
        let mut store = StringSetStore::new();
        store.push(StringSetEntry {
            name: "title".into(),
            value: "Ch1".into(),
            node_id: 10,
        });
        store.push(StringSetEntry {
            name: "title".into(),
            value: "Ch2".into(),
            node_id: 20,
        });
        store.push(StringSetEntry {
            name: "section".into(),
            value: "Intro".into(),
            node_id: 30,
        });
        assert_eq!(store.entries().len(), 3);
    }
}
