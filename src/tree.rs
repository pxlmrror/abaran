use anyhow::Result;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    Copy,
    Cut,
}

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

fn icon_for(name: &str, is_dir: bool) -> &str {
    let base = if is_dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };

    if is_dir {
        if base.eq_ignore_ascii_case(".git") {
            "\u{e5fb} "
        } else if base.eq_ignore_ascii_case(".github") {
            "\u{e5fd} "
        } else if base.eq_ignore_ascii_case(".gitlab") {
            "\u{f0fa5} "
        } else if base.eq_ignore_ascii_case(".svn") {
            "\u{f064f} "
        } else if base.eq_ignore_ascii_case("node_modules") || base.eq_ignore_ascii_case(".npm") {
            "\u{e5fa} "
        } else if base.eq_ignore_ascii_case("src")
            || base.eq_ignore_ascii_case("source")
            || base.eq_ignore_ascii_case("lib")
            || base.eq_ignore_ascii_case("libs")
            || base.eq_ignore_ascii_case("app")
            || base.eq_ignore_ascii_case("pkg")
        {
            "\u{f08de} "
        } else if base.eq_ignore_ascii_case("test")
            || base.eq_ignore_ascii_case("tests")
            || base.eq_ignore_ascii_case("spec")
            || base.eq_ignore_ascii_case("specs")
            || base.eq_ignore_ascii_case("bench")
            || base.eq_ignore_ascii_case("benches")
        {
            "\u{f09b8} "
        } else if base.eq_ignore_ascii_case("examples")
            || base.eq_ignore_ascii_case("example")
            || base.eq_ignore_ascii_case("samples")
            || base.eq_ignore_ascii_case("sample")
        {
            "\u{f024e} "
        } else if base.eq_ignore_ascii_case("include")
            || base.eq_ignore_ascii_case("includes")
            || base.eq_ignore_ascii_case("headers")
        {
            "\u{e60b} "
        } else if base.eq_ignore_ascii_case("target")
            || base.eq_ignore_ascii_case("build")
            || base.eq_ignore_ascii_case("dist")
            || base.eq_ignore_ascii_case("out")
            || base.eq_ignore_ascii_case(".build")
            || base.eq_ignore_ascii_case("obj")
            || base.eq_ignore_ascii_case(".next")
            || base.eq_ignore_ascii_case(".nuxt")
            || base.eq_ignore_ascii_case(".output")
            || base.eq_ignore_ascii_case(".cache")
            || base.eq_ignore_ascii_case(".parcel-cache")
        {
            "\u{f19fc} "
        } else if base.eq_ignore_ascii_case(".config")
            || base.eq_ignore_ascii_case("config")
            || base.eq_ignore_ascii_case("conf")
            || base.eq_ignore_ascii_case("etc")
            || base.eq_ignore_ascii_case(".settings")
        {
            "\u{e5fc} "
        } else if base.eq_ignore_ascii_case("bin") || base.eq_ignore_ascii_case("binaries") {
            "\u{f0a01} "
        } else if base.eq_ignore_ascii_case("docs")
            || base.eq_ignore_ascii_case("doc")
            || base.eq_ignore_ascii_case("documentation")
            || base.eq_ignore_ascii_case("man")
            || base.eq_ignore_ascii_case(".documentation")
        {
            "\u{f02d0} "
        } else if base.eq_ignore_ascii_case("assets")
            || base.eq_ignore_ascii_case("public")
            || base.eq_ignore_ascii_case("static")
            || base.eq_ignore_ascii_case("resources")
            || base.eq_ignore_ascii_case("res")
            || base.eq_ignore_ascii_case("images")
            || base.eq_ignore_ascii_case("img")
            || base.eq_ignore_ascii_case("media")
        {
            "\u{f024f} "
        } else if base.eq_ignore_ascii_case("fonts")
            || base.eq_ignore_ascii_case("font")
            || base.eq_ignore_ascii_case("typefaces")
        {
            "\u{f031} "
        } else if base.eq_ignore_ascii_case("data")
            || base.eq_ignore_ascii_case("database")
            || base.eq_ignore_ascii_case("db")
            || base.eq_ignore_ascii_case("migrations")
            || base.eq_ignore_ascii_case("migration")
            || base.eq_ignore_ascii_case("seeds")
        {
            "\u{f1c0} "
        } else if base.eq_ignore_ascii_case(".ssh") || base.eq_ignore_ascii_case("ssh") {
            "\u{f08ac} "
        } else if base.eq_ignore_ascii_case(".vscode")
            || base.eq_ignore_ascii_case(".idea")
            || base.eq_ignore_ascii_case(".devcontainer")
            || base.eq_ignore_ascii_case(".cursor")
        {
            "\u{f06e8} "
        } else if base.eq_ignore_ascii_case("venv")
            || base.eq_ignore_ascii_case(".venv")
            || base.eq_ignore_ascii_case(".env")
            || base.eq_ignore_ascii_case(".virtualenv")
            || base.eq_ignore_ascii_case(".tox")
            || base.eq_ignore_ascii_case("__pycache__")
            || base.eq_ignore_ascii_case(".mypy_cache")
            || base.eq_ignore_ascii_case(".pytest_cache")
            || base.eq_ignore_ascii_case(".ruff_cache")
        {
            "\u{e606} "
        } else if base.eq_ignore_ascii_case(".cargo") {
            "\u{e68b} "
        } else if base.eq_ignore_ascii_case(".terraform")
            || base.eq_ignore_ascii_case("terraform")
        {
            "\u{f1061} "
        } else if base.eq_ignore_ascii_case(".docker") || base.eq_ignore_ascii_case("docker") {
            "\u{e650} "
        } else if base.eq_ignore_ascii_case(".circleci") {
            "\u{f0fd9} "
        } else if base.eq_ignore_ascii_case("vendor") || base.eq_ignore_ascii_case(".bundle") {
            "\u{e739} "
        } else if base.eq_ignore_ascii_case("Downloads") {
            "\u{f024d} "
        } else if base.eq_ignore_ascii_case("Documents") {
            "\u{f0c82} "
        } else if base.eq_ignore_ascii_case("Pictures") || base.eq_ignore_ascii_case("Photos") {
            "\u{f024f} "
        } else if base.eq_ignore_ascii_case("Music") || base.eq_ignore_ascii_case("Audio") {
            "\u{f1359} "
        } else if base.eq_ignore_ascii_case("Videos") || base.eq_ignore_ascii_case("Movies") {
            "\u{f1795} "
        } else if base.eq_ignore_ascii_case("Desktop") {
            "\u{f108} "
        } else if base.eq_ignore_ascii_case("Templates") {
            "\u{f0eb6} "
        } else if base.eq_ignore_ascii_case("Public") {
            "\u{f0256} "
        } else if base.eq_ignore_ascii_case("log") || base.eq_ignore_ascii_case("logs") {
            "\u{f18d} "
        } else if base.eq_ignore_ascii_case("tmp")
            || base.eq_ignore_ascii_case("temp")
            || base.eq_ignore_ascii_case(".tmp")
            || base.eq_ignore_ascii_case(".temp")
            || base.eq_ignore_ascii_case("cache")
        {
            "\u{f017b} "
        } else if base.eq_ignore_ascii_case("locale")
            || base.eq_ignore_ascii_case("locales")
            || base.eq_ignore_ascii_case("i18n")
            || base.eq_ignore_ascii_case("lang")
        {
            "\u{f05ca} "
        } else if base.starts_with('.') {
            "\u{f179e} "
        } else {
            "\u{e5ff} "
        }
    } else {
        let lower = base.to_lowercase();

        if lower == "makefile" || lower == "gnumakefile" {
            "\u{e68f} "
        } else if lower.starts_with("dockerfile")
            || lower == "docker-compose.yml"
            || lower == "docker-compose.yaml"
        {
            "\u{e650} "
        } else if lower == "license"
            || lower == "licence"
            || lower == "copying"
            || lower == "copyright"
            || lower == "notice"
            || lower == "unlicense"
        {
            "\u{f02d} "
        } else if lower.starts_with("readme") || lower.starts_with("changelog") {
            "\u{f00ba} "
        } else if lower.starts_with(".git") {
            "\u{f02a2} "
        } else if lower == ".dockerignore" {
            "\u{e650} "
        } else if lower == ".editorconfig" {
            "\u{f107b} "
        } else if lower.starts_with(".env") || lower == ".envrc" {
            "\u{f0462} "
        } else if lower.starts_with(".eslint")
            || lower.starts_with(".prettier")
        {
            "\u{e74e} "
        } else if lower == "cargo.toml"
            || lower == "cargo.lock"
            || lower == "rust-toolchain"
            || lower == "rust-toolchain.toml"
        {
            "\u{e68b} "
        } else if lower == "go.mod" || lower == "go.sum" || lower == "go.work" || lower == "go.work.sum" {
            "\u{e65e} "
        } else if lower == "package.json"
            || lower == "package-lock.json"
            || lower == "npm-shrinkwrap.json"
            || lower == "yarn.lock"
            || lower == "pnpm-lock.yaml"
            || lower == ".npmrc"
        {
            "\u{e5fa} "
        } else if lower.starts_with("tsconfig") && (lower.ends_with(".json") || lower == "tsconfig") {
            "\u{e628} "
        } else if lower == "jsconfig.json" {
            "\u{e74e} "
        } else if lower.starts_with("gemfile") || lower == "gems.rb" || lower == "gems.locked" {
            "\u{e739} "
        } else if lower == "mix.exs" || lower == "mix.lock" {
            "\u{e62d} "
        } else if lower == "cmakelists.txt" {
            "\u{f0fe4} "
        } else if lower == "meson.build" || lower == "meson_options.txt" {
            "\u{f14cc} "
        } else if lower == "build" || lower == "workspace" {
            "\u{eb0f} "
        } else if lower == "rebar.config" {
            "\u{e7b1} "
        } else if lower == "composer.json" || lower == "composer.lock" {
            "\u{e73d} "
        } else if lower.starts_with("requirements")
            || lower == "pipfile"
            || lower == "pipfile.lock"
            || lower == "pyproject.toml"
            || lower == "setup.py"
            || lower == "setup.cfg"
            || lower == ".python-version"
        {
            "\u{e606} "
        } else if lower == "rakefile" {
            "\u{e739} "
        } else if lower == "vagrantfile" {
            "\u{f0f27} "
        } else if lower == "procfile" || lower.starts_with("procfile.") {
            "\u{f0ae5} "
        } else if lower == ".luacheckrc" || lower == ".stylua.toml" {
            "\u{e620} "
        } else if let Some((_, ext)) = lower.rsplit_once('.') {
            match ext {
                "rs" => "\u{e68b} ",
                "py" | "pyw" | "pyi" | "pyt" => "\u{e606} ",
                "js" | "jsx" | "mjs" | "cjs" => "\u{e74e} ",
                "ts" | "tsx" | "mts" | "cts" => "\u{e628} ",
                "go" => "\u{e65e} ",
                "c" => "\u{e61e} ",
                "h" => "\u{e61e} ",
                "cpp" | "cc" | "cxx" | "c++" => "\u{e61d} ",
                "hpp" | "hh" | "hxx" | "h++" => "\u{e61d} ",
                "cs" | "csx" => "\u{f031b} ",
                "java" | "jav" => "\u{e256} ",
                "kt" | "kts" => "\u{e634} ",
                "scala" | "sc" => "\u{e737} ",
                "rb" | "rake" | "gemspec" | "ru" => "\u{e739} ",
                "php" | "phtml" | "phps" | "phpt" => "\u{e73d} ",
                "lua" | "luau" => "\u{e620} ",
                "swift" => "\u{e755} ",
                "nim" | "nims" => "\u{e677} ",
                "zig" => "\u{f06ba} ",
                "hs" | "lhs" => "\u{e777} ",
                "ex" | "exs" => "\u{e62d} ",
                "erl" | "hrl" => "\u{e7b1} ",
                "clj" | "cljs" | "cljc" | "edn" => "\u{e76a} ",
                "dart" => "\u{e798} ",
                "r" | "rmd" | "rproj" => "\u{f25d} ",
                "jl" => "\u{e624} ",
                "pl" | "pm" => "\u{e769} ",
                "sql" | "sqlt" | "sqlite" | "db" | "sqlite3" => "\u{f1c0} ",
                "graphql" | "gql" => "\u{e284} ",
                "proto" => "\u{f068d} ",
                "md" | "mdx" | "markdown" | "mdown" | "mkdn" | "mkd" => "\u{f48a} ",
                "rst" | "rest" => "\u{f01e4} ",
                "org" => "\u{f022c} ",
                "toml" => "\u{e6b2} ",
                "yaml" | "yml" => "\u{e8eb} ",
                "json" | "jsonc" | "json5" => "\u{e60b} ",
                "xml" | "xsl" | "xsd" | "rss" | "atom" | "plist" => "\u{eb97} ",
                "ini" | "cfg" | "conf" | "config" | "properties" | "desktop" => "\u{f107b} ",
                "html" | "htm" | "xhtml" | "shtml" => "\u{f13b} ",
                "css" => "\u{e749} ",
                "scss" | "sass" => "\u{e74b} ",
                "less" => "\u{e758} ",
                "styl" | "stylus" => "\u{f1f62} ",
                "sh" | "bash" => "\u{f489} ",
                "zsh" => "\u{f1183} ",
                "fish" => "\u{f020a} ",
                "ps1" | "psm1" | "psd1" => "\u{e7b8} ",
                "bat" | "cmd" => "\u{f17a} ",
                "nix" => "\u{f313} ",
                "tf" | "tfvars" | "tfstate" => "\u{f1061} ",
                "hcl" => "\u{f1061} ",
                "wasm" | "wat" => "\u{e6a1} ",
                "lock" => "\u{f023} ",
                "log" => "\u{f18d} ",
                "vim" | "vimrc" | "gvimrc" | "vimbak" => "\u{e62b} ",
                "el" | "elc" | "emacs" => "\u{e632} ",
                "diff" | "patch" | "rej" => "\u{f0440} ",
                "zip" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "xz" | "txz" | "7z" | "rar"
                | "zst" | "lz" | "lz4" | "lzma" | "br" => "\u{f410} ",
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tif" | "tiff"
                | "heic" | "heif" | "avif" => "\u{f1c5} ",
                "svg" => "\u{fc1d} ",
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "wma" | "m4a" | "opus" | "aiff" => {
                    "\u{f001} "
                }
                "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "flv" | "m4v" | "3gp" => {
                    "\u{f03d} "
                }
                "ttf" | "otf" | "woff" | "woff2" | "eot" => "\u{f031} ",
                "pdf" => "\u{f1c1} ",
                "doc" | "docx" => "\u{f1c2} ",
                "xls" | "xlsx" => "\u{f1c3} ",
                "ppt" | "pptx" => "\u{f1c4} ",
                "csv" | "tsv" => "\u{f1c3} ",
                "pem" | "crt" | "key" | "p12" | "pfx" | "cer" | "der" | "csr" => "\u{f09a1} ",
                "gpg" | "pgp" | "asc" | "sig" => "\u{f099d} ",
                "so" | "o" | "a" | "dll" | "dylib" | "lib" => "\u{eb9c} ",
                "exe" | "msi" | "app" => "\u{f17a} ",
                "ipynb" => "\u{e606} ",
                "cmake" => "\u{f0fe4} ",
                "sage" => "\u{f1c2} ",
                "sbt" => "\u{e737} ",
                "gradle" => "\u{e256} ",
                _ => "\u{f15b} ",
            }
        } else {
            "\u{f15b} "
        }
    }
}

fn entry_ignored(path: &Path, is_dir: bool, root_path: &Path, gi: &Option<Gitignore>) -> bool {
    let git_dir = root_path.join(".git");
    if path == git_dir || path.starts_with(&git_dir) {
        return true;
    }
    let gi = match gi {
        Some(g) => g,
        None => return false,
    };

    let mut p = path.parent();
    while let Some(parent) = p {
        if parent == root_path {
            break;
        }
        let rel = parent.strip_prefix(root_path).unwrap_or(parent);
        if gi.matched(rel, true).is_ignore() {
            return true;
        }
        p = parent.parent();
    }

    let rel = path.strip_prefix(root_path).unwrap_or(path);
    gi.matched(rel, is_dir).is_ignore()
}

pub struct FileTree {
    pub root: Entry,
    pub selected: usize,
    search_query: String,
    search_matches: Vec<usize>,
    gitignore: Option<Gitignore>,
}

impl FileTree {
    pub fn new(root_path: PathBuf) -> Result<Self> {
        let root = {
            let mut r = Entry {
                name: root_path.to_string_lossy().to_string(),
                path: root_path.clone(),
                is_dir: true,
                expanded: true,
                children: Vec::new(),
                loaded: false,
            };
            r.children = load_entries(&r.path);
            r.loaded = true;
            r
        };
        let gitignore = {
            let mut builder = GitignoreBuilder::new(&root_path);
            builder.add(root_path.join(".gitignore"));
            builder.build().ok()
        };
        Ok(FileTree {
            root,
            selected: 0,
            search_query: String::new(),
            search_matches: Vec::new(),
            gitignore,
        })
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
        if !self.root.expanded {
            return 1;
        }
        1 + {
            let mut count = 0;
            for child in &self.root.children {
                count += count_visible(child);
            }
            count
        }
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
        if self.selected == 0 {
            return Some(self.root.path.clone());
        }
        fn walk<'a>(e: &'a Entry, target: usize, idx: &mut usize) -> Option<&'a Path> {
            for child in &e.children {
                if *idx == target {
                    return Some(&child.path);
                }
                *idx += 1;
                if child.is_dir && child.expanded
                    && let Some(found) = walk(child, target, idx)
                {
                    return Some(found);
                }
            }
            None
        }
        let mut idx = 0;
        walk(&self.root, self.selected - 1, &mut idx).map(|p| p.to_path_buf())
    }

    pub fn is_selected_dir(&self) -> bool {
        if self.selected == 0 {
            return true;
        }
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

    pub fn is_root_selected(&self) -> bool {
        self.selected == 0
    }

    fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        entry_ignored(path, is_dir, &self.root.path, &self.gitignore)
    }

    pub fn any_expanded(&self) -> bool {
        fn walk(e: &Entry) -> bool {
            for child in &e.children {
                if child.is_dir && child.expanded {
                    return true;
                }
                if walk(child) {
                    return true;
                }
            }
            false
        }
        walk(&self.root)
    }

    pub fn collapse_all(&mut self) {
        let mut ignored_dirs = HashSet::new();
        for child in &self.root.children {
            if self.is_ignored(&child.path, child.is_dir) {
                ignored_dirs.insert(child.path.clone());
            }
        }
        fn walk(e: &mut Entry, ignored: &HashSet<PathBuf>) {
            if e.is_dir {
                e.expanded = false;
                for child in &mut e.children {
                    if !ignored.contains(&child.path) {
                        walk(child, ignored);
                    }
                }
            }
        }
        for child in &mut self.root.children {
            if !ignored_dirs.contains(&child.path) {
                walk(child, &ignored_dirs);
            }
        }
    }

    pub fn expand_all(&mut self) {
        let mut ignored_dirs = HashSet::new();
        fn collect(
            e: &Entry,
            root: &Path,
            gitignore: &Option<Gitignore>,
            set: &mut HashSet<PathBuf>,
        ) {
            for child in &e.children {
                if child.is_dir {
                    let ign = entry_ignored(&child.path, true, root, gitignore);
                    if ign {
                        set.insert(child.path.clone());
                    }
                    if child.expanded {
                        collect(child, root, gitignore, set);
                    }
                }
            }
        }
        collect(&self.root, &self.root.path, &self.gitignore, &mut ignored_dirs);

        fn walk(e: &mut Entry, ignored: &HashSet<PathBuf>) {
            e.expanded = true;
            if !e.loaded {
                e.children = load_entries(&e.path);
                e.loaded = true;
            }
            for child in &mut e.children {
                if child.is_dir && !ignored.contains(&child.path) {
                    walk(child, ignored);
                }
            }
        }
        for child in &mut self.root.children {
            if child.is_dir && !ignored_dirs.contains(&child.path) {
                walk(child, &ignored_dirs);
            }
        }
    }

    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty()
    }

    pub fn set_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        if query.is_empty() {
            self.search_matches.clear();
            return;
        }
        self.search_matches = self.build_search_matches(query);
        if !self.search_matches.is_empty() {
            self.selected = self.search_matches[0];
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_matches.clear();
    }

    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        match self
            .search_matches
            .iter()
            .position(|&m| m > self.selected)
        {
            Some(pos) => self.selected = self.search_matches[pos],
            None => self.selected = self.search_matches[0],
        }
    }

    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        match self
            .search_matches
            .iter()
            .rposition(|&m| m < self.selected)
        {
            Some(pos) => self.selected = self.search_matches[pos],
            None => {
                self.selected = self.search_matches[self.search_matches.len() - 1]
            }
        }
    }

    fn build_search_matches(&self, query: &str) -> Vec<usize> {
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

        if self
            .root
            .name
            .to_lowercase()
            .contains(&query_lower)
        {
            matches.push(0);
        }

        fn walk(
            e: &Entry,
            query: &str,
            idx: &mut usize,
            matches: &mut Vec<usize>,
        ) {
            for child in &e.children {
                *idx += 1;
                if child.name.to_lowercase().contains(query) {
                    matches.push(*idx);
                }
                if child.is_dir && child.expanded {
                    walk(child, query, idx, matches);
                }
            }
        }

        let mut idx = 0;
        walk(&self.root, &query_lower, &mut idx, &mut matches);
        matches
    }

    pub fn reload_children(&mut self, dir_path: &Path) {
        walk_mut(&mut self.root, dir_path, &mut |e: &mut Entry| {
            if e.is_dir {
                e.loaded = false;
                if e.expanded {
                    e.children = load_entries(&e.path);
                    e.loaded = true;
                }
            }
        });
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        clipboard: &[PathBuf],
        clip_mode: Option<ClipMode>,
    ) {
        let total = self.visible_count();
        if total > 0 && self.selected >= total {
            self.selected = total - 1;
        }

        let clip_set: HashSet<&PathBuf> = clipboard.iter().collect();
        let match_set: HashSet<usize> = self.search_matches.iter().copied().collect();

        #[allow(clippy::too_many_arguments)]
        fn walk_items<'a>(
            e: &'a Entry,
            ancestors: &[bool],
            items: &mut Vec<ListItem<'a>>,
            clip_set: &HashSet<&PathBuf>,
            clip_mode: Option<ClipMode>,
            match_set: &HashSet<usize>,
            idx: &mut usize,
            selected: usize,
            gitignore: &Option<Gitignore>,
            root_path: &Path,
        ) {
            let count = e.children.len();
            for (i, child) in e.children.iter().enumerate() {
                *idx += 1;
                let is_last = i == count - 1;

                let mut connector = String::with_capacity(ancestors.len() * 4 + 4);
                for &cont in ancestors {
                    if cont {
                        connector.push_str("│  ");
                    } else {
                        connector.push_str("   ");
                    }
                }
                if is_last {
                    connector.push_str("└── ");
                } else {
                    connector.push_str("├── ");
                }

                let ignored = entry_ignored(
                    &child.path,
                    child.is_dir,
                    root_path,
                    gitignore,
                );

                let icon = icon_for(&child.name, child.is_dir);
                let in_clip = clip_set.contains(&child.path);
                let clip_tag = match (in_clip, clip_mode) {
                    (true, Some(ClipMode::Copy)) => "[copy] ",
                    (true, Some(ClipMode::Cut)) => "[cut] ",
                    _ => "",
                };

                let mut style = if ignored {
                    Style::default().fg(Color::DarkGray)
                } else if child.is_dir {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                if match_set.contains(idx) {
                    style = Style::default().fg(Color::Black).bg(Color::Yellow);
                }
                if *idx == selected {
                    style = Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD);
                }

                let tree_style = Style::default().fg(Color::DarkGray);
                let line = Line::from(vec![
                    Span::styled(connector, tree_style),
                    Span::styled(format!("{}{}{}", clip_tag, icon, child.name), style),
                ]);
                items.push(ListItem::new(line));

                if child.is_dir && child.expanded {
                    let mut next_ancestors = ancestors.to_vec();
                    next_ancestors.push(!is_last);
                    walk_items(
                        child,
                        &next_ancestors,
                        items,
                        clip_set,
                        clip_mode,
                        match_set,
                        idx,
                        selected,
                        gitignore,
                        root_path,
                    );
                }
            }
        }

        let mut items = Vec::new();
        let mut root_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        if match_set.contains(&0) {
            root_style = Style::default().fg(Color::Black).bg(Color::Yellow);
        }
        if self.selected == 0 {
            root_style = Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD);
        }
        items.push(ListItem::new(Span::styled(
            format!("\u{e5ff} {}", self.root.path.display()),
            root_style,
        )));
        let mut idx = 0;
        walk_items(
            &self.root,
            &[],
            &mut items,
            &clip_set,
            clip_mode,
            &match_set,
            &mut idx,
            self.selected,
            &self.gitignore,
            &self.root.path,
        );

        let list = List::new(items);

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
    fn test_gitignore_builder() {
        use std::fs;
        let tmp = std::path::PathBuf::from("/tmp/gi_test_repo_simple");
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::write(tmp.join(".gitignore"), "/target\n*.log\nbuild/\n").unwrap();

        let mut builder = GitignoreBuilder::new(&tmp);
        builder.add(tmp.join(".gitignore"));
        let gi = builder.build().unwrap();

        for (p, is_dir, expected) in [
            ("target", true, true),
            ("target/sub", true, true),
            ("target/deep/nested", true, true),
            ("src", true, false),
            ("debug.log", false, true),
            ("build", true, true),
            ("build/output", true, true),
            ("Cargo.toml", false, false),
        ] {
            let full = tmp.join(p);
            let actual = entry_ignored(&full, is_dir, &tmp, &Some(gi.clone()));
            assert_eq!(
                actual, expected,
                "path={} dir={}: expected is_ignore={} got {}",
                p, is_dir, expected, actual
            );
        }
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_toggle_directory() {
        let cwd = std::env::current_dir().unwrap();
        let mut tree = FileTree::new(cwd).unwrap();
        let before = tree.visible_count();
        if tree.is_selected_dir() {
            tree.toggle_selected();
            let after = tree.visible_count();
            assert!(after != before);
        }
    }
}

