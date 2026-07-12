use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Vec<Entry>,
    loaded: bool,
}

fn load_entries(path: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = WalkDir::new(path)
        .max_depth(1)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter()
        .filter_map(|e| e.ok())
        .skip(1)
        .map(|e| {
            let is_dir = e.file_type().is_dir();
            let path = e.path().to_path_buf();
            let name = e.file_name().to_string_lossy().to_string();
            Entry {
                name: if is_dir { format!("{}/", name) } else { name },
                path,
                is_dir,
                expanded: false,
                children: Vec::new(),
                loaded: false,
            }
        })
        .collect();

    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    entries
}

pub struct FileTree {
    pub root: Entry,
    pub selected: usize,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Result<Self> {
        let mut root = Entry {
            name: root_path.to_string_lossy().to_string(),
            path: root_path,
            is_dir: true,
            expanded: true,
            children: Vec::new(),
            loaded: false,
        };
        root.children = load_entries(&root.path);
        root.loaded = true;
        Ok(FileTree { root, selected: 0 })
    }

    pub fn visible_count(&self) -> usize {
        fn count_visible(e: &Entry) -> usize {
            if !e.is_dir || !e.expanded {
                return 1;
            }
            let mut count = 1;
            for child in &e.children {
                count += count_visible(child);
            }
            count
        }
        let mut count = 0;
        for child in &self.root.children {
            count += count_visible(child);
        }
        count
    }

    pub fn navigate_down(&mut self) {
        let total = self.visible_count();
        if total == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(total - 1);
    }

    pub fn navigate_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn toggle_selected(&mut self) {
        let path = self.selected_path();
        if let Some(path) = path {
            walk_mut(&mut self.root, &path, &mut |e: &mut Entry| {
                if e.is_dir {
                    e.expanded = !e.expanded;
                    if e.expanded && !e.loaded {
                        e.children = load_entries(&e.path);
                        e.loaded = true;
                    }
                }
            });
            let total = self.visible_count();
            if total > 0 && self.selected >= total {
                self.selected = total - 1;
            }
        }
    }

    pub fn expand_selected(&mut self) {
        let path = self.selected_path();
        if let Some(path) = path {
            walk_mut(&mut self.root, &path, &mut |e: &mut Entry| {
                if e.is_dir && !e.expanded {
                    e.expanded = true;
                    if !e.loaded {
                        e.children = load_entries(&e.path);
                        e.loaded = true;
                    }
                }
            });
            let total = self.visible_count();
            if total > 0 && self.selected >= total {
                self.selected = total - 1;
            }
        }
    }

    pub fn collapse_selected(&mut self) {
        let path = self.selected_path();
        if let Some(path) = path {
            walk_mut(&mut self.root, &path, &mut |e: &mut Entry| {
                if e.is_dir && e.expanded {
                    e.expanded = false;
                }
            });
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        fn walk<'a>(e: &'a Entry, target: usize, idx: &mut usize) -> Option<&'a Path> {
            for child in &e.children {
                if *idx == target {
                    return Some(&child.path);
                }
                *idx += 1;
                if child.is_dir && child.expanded {
                    if let Some(found) = walk(child, target, idx) {
                        return Some(found);
                    }
                }
            }
            None
        }
        let mut idx = 0;
        walk(&self.root, self.selected, &mut idx).map(|p| p.to_path_buf())
    }

    pub fn is_selected_dir(&self) -> bool {
        if let Some(path) = self.selected_path() {
            fn find(e: &Entry, target: &Path) -> Option<bool> {
                if e.path == target {
                    return Some(e.is_dir);
                }
                for child in &e.children {
                    if let Some(result) = find(child, target) {
                        return Some(result);
                    }
                }
                None
            }
            find(&self.root, &path).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let total = self.visible_count();
        if total > 0 && self.selected >= total {
            self.selected = total - 1;
        }

        fn walk_items<'a>(e: &'a Entry, depth: usize, items: &mut Vec<ListItem<'a>>) {
            for child in &e.children {
                let indent = "  ".repeat(depth.saturating_sub(1));
                let line = format!("{}{}", indent, child.name);
                let style = if child.is_dir {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                items.push(ListItem::new(Span::styled(line, style)));
                if child.is_dir && child.expanded {
                    walk_items(child, depth + 1, items);
                }
            }
        }

        let mut items = Vec::new();
        walk_items(&self.root, 1, &mut items);

        let list = List::new(items)
            .block(Block::default().borders(Borders::TOP).title(" abaran "))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if total > 0 {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, area, &mut state);
    }
}

fn walk_mut<F>(entry: &mut Entry, target: &Path, f: &mut F)
where
    F: FnMut(&mut Entry),
{
    if entry.path == target {
        f(entry);
        return;
    }
    for child in &mut entry.children {
        if child.path == target {
            f(child);
            return;
        }
        if child.is_dir {
            walk_mut(child, target, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_lists_files() {
        let cwd = std::env::current_dir().unwrap();
        let tree = FileTree::new(cwd.clone()).unwrap();
        assert!(tree.visible_count() > 0, "should see files in {}", cwd.display());
    }

    #[test]
    fn test_select_path() {
        let cwd = std::env::current_dir().unwrap();
        let tree = FileTree::new(cwd.clone()).unwrap();
        let path = tree.selected_path();
        assert!(path.is_some(), "should have a selected path");
        assert!(path.unwrap().starts_with(&cwd));
    }

    #[test]
    fn test_navigate_down() {
        let cwd = std::env::current_dir().unwrap();
        let mut tree = FileTree::new(cwd).unwrap();
        let before = tree.selected;
        tree.navigate_down();
        assert!(tree.selected > before || tree.visible_count() == 0);
    }

    #[test]
    fn test_toggle_directory() {
        let cwd = std::env::current_dir().unwrap();
        let mut tree = FileTree::new(cwd).unwrap();
        let before = tree.visible_count();
        // Find a directory and toggle it
        if tree.is_selected_dir() {
            tree.toggle_selected();
            // After expand, visible count should change
            let after = tree.visible_count();
            assert!(after != before);
        }
    }
}
