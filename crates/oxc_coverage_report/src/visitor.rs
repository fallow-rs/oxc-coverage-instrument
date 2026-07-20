//! Tree-walking visitor protocol.

use std::io;

use crate::tree::{NodeKind, ReportNode};

/// Callbacks invoked by [`walk`] as it traverses a [`ReportNode`] tree
/// depth-first.
///
/// Calling order for a tree with one folder containing one file:
///
/// 1. [`Visitor::on_start`] on the root.
/// 2. [`Visitor::on_summary`] on the root folder (entry).
/// 3. [`Visitor::on_detail`] on the file.
/// 4. [`Visitor::on_summary_end`] on the root folder (exit).
/// 5. [`Visitor::on_end`] on the root.
///
/// All methods default to no-ops returning `Ok(())`. Override only what you
/// need; future callbacks added to this trait will not break implementations
/// that rely on defaults.
pub trait Visitor {
    /// Called once at the start of traversal with the root node.
    ///
    /// # Errors
    /// Returns any error the implementation hits while writing output.
    fn on_start(&mut self, _root: &ReportNode) -> io::Result<()> {
        Ok(())
    }

    /// Called when entering a folder node (after `on_start` for the root).
    ///
    /// # Errors
    /// Returns any error the implementation hits while writing output.
    fn on_summary(&mut self, _node: &ReportNode) -> io::Result<()> {
        Ok(())
    }

    /// Called when leaving a folder node (after all its children have been visited).
    ///
    /// # Errors
    /// Returns any error the implementation hits while writing output.
    fn on_summary_end(&mut self, _node: &ReportNode) -> io::Result<()> {
        Ok(())
    }

    /// Called once per file node.
    ///
    /// # Errors
    /// Returns any error the implementation hits while writing output.
    fn on_detail(&mut self, _node: &ReportNode) -> io::Result<()> {
        Ok(())
    }

    /// Called once at the end of traversal with the root node.
    ///
    /// # Errors
    /// Returns any error the implementation hits while writing output.
    fn on_end(&mut self, _root: &ReportNode) -> io::Result<()> {
        Ok(())
    }
}

/// Walk a tree, invoking the visitor callbacks in depth-first order.
///
/// # Errors
/// Returns the first error a callback returns. Traversal stops there and
/// [`Visitor::on_end`] is not called, so a visitor that buffers output has to
/// handle its own cleanup.
pub fn walk<V: Visitor + ?Sized>(root: &ReportNode, visitor: &mut V) -> io::Result<()> {
    visitor.on_start(root)?;
    walk_node(root, visitor)?;
    visitor.on_end(root)?;
    Ok(())
}

fn walk_node<V: Visitor + ?Sized>(node: &ReportNode, visitor: &mut V) -> io::Result<()> {
    match &node.kind {
        NodeKind::Folder { children } => {
            visitor.on_summary(node)?;
            for child in children {
                walk_node(child, visitor)?;
            }
            visitor.on_summary_end(node)?;
        }
        NodeKind::File { .. } => {
            visitor.on_detail(node)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use oxc_coverage_types::parse_coverage_map;

    use super::*;
    use crate::summarize;

    #[derive(Default)]
    struct CallLog(Vec<String>);
    impl Visitor for CallLog {
        fn on_start(&mut self, _root: &ReportNode) -> io::Result<()> {
            self.0.push("start".into());
            Ok(())
        }
        fn on_summary(&mut self, node: &ReportNode) -> io::Result<()> {
            self.0.push(format!("enter:{}", node.relative_path));
            Ok(())
        }
        fn on_summary_end(&mut self, node: &ReportNode) -> io::Result<()> {
            self.0.push(format!("leave:{}", node.relative_path));
            Ok(())
        }
        fn on_detail(&mut self, node: &ReportNode) -> io::Result<()> {
            self.0.push(format!("file:{}", node.relative_path));
            Ok(())
        }
        fn on_end(&mut self, _root: &ReportNode) -> io::Result<()> {
            self.0.push("end".into());
            Ok(())
        }
    }

    #[test]
    fn callbacks_fire_in_expected_order() {
        let json = r#"{"src/a/x.js":{"path":"src/a/x.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}},"src/b.js":{"path":"src/b.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#;
        let map = parse_coverage_map(json).unwrap();
        let root = summarize(&map);
        let mut log = CallLog::default();
        walk(&root, &mut log).unwrap();
        assert_eq!(
            log.0,
            vec![
                "start",
                "enter:",
                "enter:a",
                "file:a/x.js",
                "leave:a",
                "file:b.js",
                "leave:",
                "end",
            ]
        );
    }

    #[test]
    fn walk_succeeds_with_all_default_callbacks() {
        struct Empty;
        impl Visitor for Empty {}
        let map = parse_coverage_map(r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{},"s":{},"f":{},"b":{}}}"#).unwrap();
        let root = summarize(&map);
        let mut empty = Empty;
        assert!(walk(&root, &mut empty).is_ok());
    }
}
