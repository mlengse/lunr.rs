use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

const WILDCARD: char = '*';

fn edge_snapshot(inner: &TokenSet) -> Vec<(char, TokenSet)> {
    inner
        .inner
        .borrow()
        .edges
        .iter()
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    inner: Rc<RefCell<TokenSetInner>>,
}

#[derive(Debug)]
struct TokenSetInner {
    final_: bool,
    edges: HashMap<char, TokenSet>,
    id: usize,
    _str: Option<String>,
}

impl TokenSet {
    pub fn new() -> Self {
        TokenSet {
            inner: Rc::new(RefCell::new(TokenSetInner {
                final_: false,
                edges: HashMap::new(),
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                _str: None,
            })),
        }
    }

    pub fn from_string(str: &str) -> Self {
        let root = TokenSet::new();
        let mut node = root.clone();

        for (i, ch) in str.chars().enumerate() {
            let is_final = i == str.len() - 1;

            if ch == WILDCARD {
                node.set_edge(WILDCARD, node.clone());
                node.set_final(is_final);
            } else {
                let next = TokenSet::new();
                next.set_final(is_final);
                node.set_edge(ch, next.clone());
                node = next;
            }
        }

        root
    }

    pub fn from_array(arr: &[String]) -> Self {
        let mut builder = Builder::new();
        for word in arr {
            builder.insert(word);
        }
        builder.finish();
        builder.root
    }

    pub fn from_clause(clause: &Clause) -> Self {
        if let Some(edit_distance) = clause.edit_distance {
            TokenSet::from_fuzzy_string(&clause.term, edit_distance)
        } else {
            TokenSet::from_string(&clause.term)
        }
    }

    pub fn from_fuzzy_string(str: &str, edit_distance: usize) -> Self {
        let root = TokenSet::new();
        let chars: Vec<char> = str.chars().collect();
        let mut stack = Vec::new();
        stack.push(Frame {
            node: root.clone(),
            edits_remaining: edit_distance,
            chars: chars.clone(),
        });

        while let Some(frame) = stack.pop() {
            // no edit
            if let Some(&ch) = frame.chars.first() {
                let no_edit_node = frame.node.get_edge(ch).unwrap_or_else(|| {
                    let new_node = TokenSet::new();
                    frame.node.set_edge(ch, new_node.clone());
                    new_node
                });

                if frame.chars.len() == 1 {
                    no_edit_node.set_final(true);
                }

                stack.push(Frame {
                    node: no_edit_node,
                    edits_remaining: frame.edits_remaining,
                    chars: frame.chars[1..].to_vec(),
                });
            }

            if frame.edits_remaining == 0 {
                continue;
            }

            // insertion (wildcard)
            let insertion_node = frame.node.get_edge(WILDCARD).unwrap_or_else(|| {
                let new_node = TokenSet::new();
                frame.node.set_edge(WILDCARD, new_node.clone());
                new_node
            });

            if frame.chars.is_empty() {
                insertion_node.set_final(true);
            }

            stack.push(Frame {
                node: insertion_node,
                edits_remaining: frame.edits_remaining - 1,
                chars: frame.chars.clone(),
            });

            // deletion
            if frame.chars.len() > 1 {
                stack.push(Frame {
                    node: frame.node.clone(),
                    edits_remaining: frame.edits_remaining - 1,
                    chars: frame.chars[1..].to_vec(),
                });
            }

            if frame.chars.len() == 1 {
                frame.node.set_final(true);
            }

            // substitution (wildcard)
            if !frame.chars.is_empty() {
                let substitution_node = frame.node.get_edge(WILDCARD).unwrap_or_else(|| {
                    let new_node = TokenSet::new();
                    frame.node.set_edge(WILDCARD, new_node.clone());
                    new_node
                });

                if frame.chars.len() == 1 {
                    substitution_node.set_final(true);
                }

                stack.push(Frame {
                    node: substitution_node,
                    edits_remaining: frame.edits_remaining - 1,
                    chars: frame.chars[1..].to_vec(),
                });
            }

            // transposition
            if frame.chars.len() > 1 {
                let c0 = frame.chars[0];
                let c1 = frame.chars[1];

                let transpose_node = frame.node.get_edge(c1).unwrap_or_else(|| {
                    let new_node = TokenSet::new();
                    frame.node.set_edge(c1, new_node.clone());
                    new_node
                });

                // remaining after swapping first two chars: c0 + rest
                let mut remaining: Vec<char> = Vec::with_capacity(frame.chars.len());
                remaining.push(c0);
                remaining.extend_from_slice(&frame.chars[2..]);
                stack.push(Frame {
                    node: transpose_node,
                    edits_remaining: frame.edits_remaining - 1,
                    chars: remaining,
                });
            }
        }

        root
    }

    pub fn to_array(&self) -> Vec<String> {
        if self.get_edges().contains_key(&WILDCARD) {
            panic!("cannot convert a TokenSet containing wildcards to an array");
        }

        let mut words = Vec::new();
        let mut stack = vec![StackFrame {
            prefix: String::new(),
            node: self.clone(),
        }];

        while let Some(frame) = stack.pop() {
            if frame.node.is_final() {
                words.push(frame.prefix.clone());
            }

            for (edge, node) in edge_snapshot(&frame.node) {
                let mut prefix = frame.prefix.clone();
                prefix.push(edge);
                stack.push(StackFrame { prefix, node });
            }
        }

        words
    }

    pub fn intersect(&self, b: &TokenSet) -> TokenSet {
        let output = TokenSet::new();
        let mut stack = vec![IntersectFrame {
            q_node: b.clone(),
            output: output.clone(),
            node: self.clone(),
        }];

        while let Some(frame) = stack.pop() {
            let q_edges = edge_snapshot(&frame.q_node);
            let n_edges = edge_snapshot(&frame.node);

            for (q_edge, q_node) in &q_edges {
                for (n_edge, n_node) in &n_edges {
                    if n_edge == q_edge || *q_edge == WILDCARD {
                        let final_ = n_node.is_final() && q_node.is_final();
                        let next = frame.output.get_edge(*n_edge).unwrap_or_else(|| {
                            let new_node = TokenSet::new();
                            frame.output.set_edge(*n_edge, new_node.clone());
                            new_node
                        });

                        next.set_final(next.is_final() || final_);

                        stack.push(IntersectFrame {
                            node: n_node.clone(),
                            q_node: q_node.clone(),
                            output: next,
                        });
                    }
                }
            }
        }

        output
    }

    fn is_final(&self) -> bool {
        self.inner.borrow().final_
    }

    fn set_final(&self, val: bool) {
        self.inner.borrow_mut().final_ = val;
    }

    fn get_edges(&self) -> HashMap<char, TokenSet> {
        self.inner.borrow().edges.clone()
    }

    fn set_edge(&self, key: char, val: TokenSet) {
        self.inner.borrow_mut().edges.insert(key, val);
    }

    fn get_edge(&self, key: char) -> Option<TokenSet> {
        self.inner.borrow().edges.get(&key).cloned()
    }

    fn id(&self) -> usize {
        self.inner.borrow().id
    }

    fn str_repr(&self) -> String {
        if let Some(ref s) = self.inner.borrow()._str {
            return s.clone();
        }

        let mut str = if self.is_final() { "1" } else { "0" }.to_string();
        let mut edges: Vec<(char, TokenSet)> =
            self.inner.borrow().edges.iter().map(|(k, v)| (*k, v.clone())).collect();
        edges.sort_by_key(|(k, _)| *k);

        for (label, node) in edges {
            str.push(label);
            str.push_str(&node.id().to_string());
        }

        str
    }

    fn set_str(&self, val: String) {
        self.inner.borrow_mut()._str = Some(val);
    }

    /// Id-independent, cycle-safe structural signature (testing only).
    #[doc(hidden)]
    pub fn __struct_fingerprint(&self) -> String {
        fn walk(node: &TokenSet, on_path: &mut Vec<usize>) -> String {
            let id = node.id();
            if on_path.contains(&id) {
                return "CYCLE".to_string();
            }
            on_path.push(id);
            let mut s = if node.is_final() { "1" } else { "0" }.to_string();
            let mut edges: Vec<(char, TokenSet)> =
                node.inner.borrow().edges.iter().map(|(k, v)| (*k, v.clone())).collect();
            edges.sort_by_key(|(k, _)| *k);
            for (label, child) in &edges {
                s.push('L');
                s.push(*label);
                s.push_str(&walk(child, on_path));
            }
            on_path.pop();
            s
        }
        walk(self, &mut Vec::new())
    }
}

impl Default for TokenSet {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenSet {
    pub fn builder() -> Builder {
        Builder::new()
    }
}

struct Frame {
    node: TokenSet,
    edits_remaining: usize,
    chars: Vec<char>,
}

struct StackFrame {
    prefix: String,
    node: TokenSet,
}

struct IntersectFrame {
    q_node: TokenSet,
    output: TokenSet,
    node: TokenSet,
}

pub struct Clause {
    pub term: String,
    pub edit_distance: Option<usize>,
    pub boost: Option<f64>,
    pub presence: Option<u8>,
    pub wildcard: Option<u8>,
    pub use_pipeline: Option<bool>,
    pub fields: Option<Vec<String>>,
}

pub struct Builder {
    previous_word: String,
    pub root: TokenSet,
    unchecked_nodes: Vec<UncheckedNode>,
    minimized_nodes: HashMap<String, TokenSet>,
}

struct UncheckedNode {
    parent: TokenSet,
    char: char,
    child: TokenSet,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            previous_word: String::new(),
            root: TokenSet::new(),
            unchecked_nodes: Vec::new(),
            minimized_nodes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, word: &str) {
        if word < self.previous_word.as_str() {
            panic!("Out of order word insertion");
        }

        let mut common_prefix = 0;
        let prev_chars: Vec<char> = self.previous_word.chars().collect();
        let word_chars: Vec<char> = word.chars().collect();

        for i in 0..word_chars.len().min(prev_chars.len()) {
            if word_chars[i] != prev_chars[i] {
                break;
            }
            common_prefix += 1;
        }

        self.minimize(common_prefix);

        let mut node = if self.unchecked_nodes.is_empty() {
            self.root.clone()
        } else {
            self.unchecked_nodes.last().unwrap().child.clone()
        };

        for i in common_prefix..word_chars.len() {
            let ch = word_chars[i];
            let next_node = TokenSet::new();
            node.set_edge(ch, next_node.clone());

            self.unchecked_nodes.push(UncheckedNode {
                parent: node.clone(),
                char: ch,
                child: next_node.clone(),
            });

            node = next_node;
        }

        node.set_final(true);
        self.previous_word = word.to_string();
    }

    pub fn finish(&mut self) {
        self.minimize(0);
    }

    fn minimize(&mut self, down_to: usize) {
        while self.unchecked_nodes.len() > down_to {
            let node = self.unchecked_nodes.pop().unwrap();
            let child_key = node.child.str_repr();

            if let Some(minimized) = self.minimized_nodes.get(&child_key) {
                node.parent.set_edge(node.char, minimized.clone());
            } else {
                node.child.set_str(child_key.clone());
                self.minimized_nodes.insert(child_key, node.child.clone());
            }
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TokenSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.str_repr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string() {
        let ts = TokenSet::from_string("test");
        assert!(ts.get_edges().contains_key(&'t'));
        assert!(!ts.is_final());
    }

    #[test]
    fn test_from_array() {
        let arr = vec!["apple".to_string(), "application".to_string()];
        let ts = TokenSet::from_array(&arr);
        let words = ts.to_array();
        assert!(words.contains(&"apple".to_string()));
        assert!(words.contains(&"application".to_string()));
    }

    #[test]
    fn test_intersect() {
        let a = TokenSet::from_string("test");
        let b = TokenSet::from_string("test");
        let result = a.intersect(&b);
        let words = result.to_array();
        assert!(words.contains(&"test".to_string()));
    }
}
