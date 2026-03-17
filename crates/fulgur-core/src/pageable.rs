/// Point unit (1/72 inch)
pub type Pt = f32;

#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: Pt,
    pub height: Pt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakBefore {
    Auto,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakAfter {
    Auto,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakInside {
    Auto,
    Avoid,
}

#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub break_before: BreakBefore,
    pub break_after: BreakAfter,
    pub break_inside: BreakInside,
    pub orphans: usize,
    pub widows: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            break_before: BreakBefore::Auto,
            break_after: BreakAfter::Auto,
            break_inside: BreakInside::Auto,
            orphans: 2,
            widows: 2,
        }
    }
}

/// Wrapper around Krilla Surface for drawing commands.
/// This decouples Pageable types from Krilla's concrete Surface type.
pub struct Canvas<'a> {
    pub surface: &'a mut krilla::surface::Surface<'a>,
}

/// Core pagination-aware layout trait.
pub trait Pageable: Send + Sync {
    /// Measure size within available area.
    fn wrap(&mut self, avail_width: Pt, avail_height: Pt) -> Size;

    /// Split at page boundary. Returns None if element fits entirely
    /// or cannot be split.
    fn split(&self, avail_width: Pt, avail_height: Pt)
        -> Option<(Box<dyn Pageable>, Box<dyn Pageable>)>;

    /// Emit drawing commands.
    fn draw(&self, canvas: &mut Canvas, x: Pt, y: Pt, avail_width: Pt, avail_height: Pt);

    /// CSS pagination properties for this element.
    fn pagination(&self) -> Pagination {
        Pagination::default()
    }

    /// Clone this pageable into a boxed trait object.
    fn clone_box(&self) -> Box<dyn Pageable>;

    /// Measured height from last wrap() call.
    fn height(&self) -> Pt;
}

impl Clone for Box<dyn Pageable> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// ─── BlockPageable ───────────────────────────────────────

/// A block container that stacks children vertically.
/// Handles margin/border/padding/background and page splitting.
#[derive(Clone)]
pub struct BlockPageable {
    pub children: Vec<Box<dyn Pageable>>,
    pub pagination: Pagination,
    pub cached_size: Option<Size>,
}

impl BlockPageable {
    pub fn new(children: Vec<Box<dyn Pageable>>) -> Self {
        Self {
            children,
            pagination: Pagination::default(),
            cached_size: None,
        }
    }

    pub fn with_pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = pagination;
        self
    }
}

impl Pageable for BlockPageable {
    fn wrap(&mut self, avail_width: Pt, avail_height: Pt) -> Size {
        let mut total_height: Pt = 0.0;
        for child in &mut self.children {
            let child_size = child.wrap(avail_width, avail_height - total_height);
            total_height += child_size.height;
        }
        let size = Size { width: avail_width, height: total_height };
        self.cached_size = Some(size);
        size
    }

    fn split(&self, avail_width: Pt, avail_height: Pt)
        -> Option<(Box<dyn Pageable>, Box<dyn Pageable>)>
    {
        if self.pagination.break_inside == BreakInside::Avoid {
            return None;
        }

        // Check for forced breaks first (break-before/break-after),
        // even if content fits on one page
        let has_forced_break = self.children.iter().enumerate().any(|(i, child)| {
            (child.pagination().break_before == BreakBefore::Page && i > 0)
                || (child.pagination().break_after == BreakAfter::Page && i < self.children.len() - 1)
        });

        let total_height = self.cached_size.map(|s| s.height).unwrap_or(0.0);
        if total_height <= avail_height && !has_forced_break {
            return None; // Fits entirely and no forced breaks
        }

        let mut consumed: Pt = 0.0;
        let mut split_index = self.children.len();

        for (i, child) in self.children.iter().enumerate() {
            let child_h = child.height();

            // Check break-before
            if child.pagination().break_before == BreakBefore::Page && i > 0 && consumed > 0.0 {
                split_index = i;
                break;
            }

            if consumed + child_h > avail_height {
                // Try to split the child itself
                if let Some((first_part, second_part)) = child.split(avail_width, avail_height - consumed) {
                    let mut first_children: Vec<Box<dyn Pageable>> = self.children[..i].iter().map(|c| c.clone_box()).collect();
                    first_children.push(first_part);

                    let mut second_children = vec![second_part];
                    for c in &self.children[i + 1..] {
                        second_children.push(c.clone_box());
                    }

                    return Some((
                        Box::new(BlockPageable::new(first_children).with_pagination(self.pagination)),
                        Box::new(BlockPageable::new(second_children).with_pagination(self.pagination)),
                    ));
                }
                // Can't split child; put it on the next page
                split_index = i;
                break;
            }

            consumed += child_h;

            // Check break-after
            if child.pagination().break_after == BreakAfter::Page {
                split_index = i + 1;
                break;
            }
        }

        if split_index == 0 || split_index == self.children.len() {
            return None; // Can't split meaningfully
        }

        let first_children: Vec<Box<dyn Pageable>> = self.children[..split_index].iter().map(|c| c.clone_box()).collect();
        let second_children: Vec<Box<dyn Pageable>> = self.children[split_index..].iter().map(|c| c.clone_box()).collect();

        Some((
            Box::new(BlockPageable::new(first_children).with_pagination(self.pagination)),
            Box::new(BlockPageable::new(second_children).with_pagination(self.pagination)),
        ))
    }

    fn draw(&self, canvas: &mut Canvas, x: Pt, y: Pt, avail_width: Pt, avail_height: Pt) {
        let _ = avail_height;
        let mut current_y = y;
        for child in &self.children {
            child.draw(canvas, x, current_y, avail_width, child.height());
            current_y += child.height();
        }
    }

    fn pagination(&self) -> Pagination {
        self.pagination
    }

    fn clone_box(&self) -> Box<dyn Pageable> {
        Box::new(self.clone())
    }

    fn height(&self) -> Pt {
        self.cached_size.map(|s| s.height).unwrap_or(0.0)
    }
}

// ─── SpacerPageable ──────────────────────────────────────

/// Fixed-height vertical space. Cannot be split.
#[derive(Clone)]
pub struct SpacerPageable {
    pub height: Pt,
}

impl SpacerPageable {
    pub fn new(height: Pt) -> Self {
        Self { height }
    }
}

impl Pageable for SpacerPageable {
    fn wrap(&mut self, avail_width: Pt, _avail_height: Pt) -> Size {
        Size { width: avail_width, height: self.height }
    }

    fn split(&self, _avail_width: Pt, _avail_height: Pt)
        -> Option<(Box<dyn Pageable>, Box<dyn Pageable>)>
    {
        None
    }

    fn draw(&self, _canvas: &mut Canvas, _x: Pt, _y: Pt, _avail_width: Pt, _avail_height: Pt) {
        // Spacers are invisible
    }

    fn clone_box(&self) -> Box<dyn Pageable> {
        Box::new(self.clone())
    }

    fn height(&self) -> Pt {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spacer(h: Pt) -> Box<dyn Pageable> {
        let mut s = SpacerPageable::new(h);
        s.wrap(100.0, 1000.0);
        Box::new(s)
    }

    #[test]
    fn test_block_fits_on_one_page() {
        let mut block = BlockPageable::new(vec![
            make_spacer(100.0),
            make_spacer(100.0),
        ]);
        block.wrap(200.0, 300.0);
        assert!(block.split(200.0, 300.0).is_none());
    }

    #[test]
    fn test_block_splits_across_pages() {
        let mut block = BlockPageable::new(vec![
            make_spacer(100.0),
            make_spacer(100.0),
            make_spacer(100.0),
        ]);
        block.wrap(200.0, 1000.0);
        let result = block.split(200.0, 250.0);
        assert!(result.is_some());
        let (first, second) = result.unwrap();
        let mut first = first;
        let mut second = second;
        let s1 = first.wrap(200.0, 250.0);
        let s2 = second.wrap(200.0, 1000.0);
        assert!((s1.height - 200.0).abs() < 0.01);
        assert!((s2.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_break_before_page() {
        let breaking = BlockPageable::new(vec![make_spacer(50.0)])
            .with_pagination(Pagination {
                break_before: BreakBefore::Page,
                ..Pagination::default()
            });
        let mut breaking = breaking;
        breaking.wrap(200.0, 1000.0);

        let mut block = BlockPageable::new(vec![
            make_spacer(50.0),
            make_spacer(50.0),
            Box::new(breaking),
        ]);
        block.wrap(200.0, 1000.0);

        // Even though everything fits in 1000pt, break-before should force split
        let result = block.split(200.0, 1000.0);
        assert!(result.is_some());
    }

    #[test]
    fn test_break_inside_avoid() {
        let block = BlockPageable::new(vec![make_spacer(200.0)])
            .with_pagination(Pagination {
                break_inside: BreakInside::Avoid,
                ..Pagination::default()
            });
        let mut block = block;
        block.wrap(200.0, 1000.0);
        // Even if it doesn't fit, split returns None
        assert!(block.split(200.0, 100.0).is_none());
    }
}
