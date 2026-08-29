// Differential test: the char-keyed TokenSet rewrite must be structurally and
// behaviourally identical to the original String-keyed implementation across a
// broad input set. Compared via (a) to_array() accepted language and (b) an
// id-independent structural fingerprint (handles wildcard/fuzzy cycles).
use lunr::token_set::TokenSet;

mod old {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

    #[derive(Debug, Clone)]
    pub struct TokenSet {
        pub inner: Rc<RefCell<TokenSetInner>>,
    }

    #[derive(Debug)]
    pub struct TokenSetInner {
        pub final_: bool,
        pub edges: HashMap<String, TokenSet>,
        pub id: usize,
        pub _str: Option<String>,
    }

    fn edge_snapshot(inner: &TokenSet) -> Vec<(String, TokenSet)> {
        inner
            .inner
            .borrow()
            .edges
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
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
                let ch_str = ch.to_string();

                if ch == '*' {
                    node.set_edge(ch_str.clone(), node.clone());
                    node.set_final(is_final);
                } else {
                    let next = TokenSet::new();
                    next.set_final(is_final);
                    node.set_edge(ch_str, next.clone());
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

        pub fn from_fuzzy_string(str: &str, edit_distance: usize) -> Self {
            let root = TokenSet::new();
            let mut stack = vec![Frame {
                node: root.clone(),
                edits_remaining: edit_distance,
                str: str.to_string(),
            }];

            while let Some(frame) = stack.pop() {
                if !frame.str.is_empty() {
                    let ch = frame.str.chars().next().unwrap().to_string();
                    let no_edit_node = frame.node.get_edge(&ch).unwrap_or_else(|| {
                        let new_node = TokenSet::new();
                        frame.node.set_edge(ch.clone(), new_node.clone());
                        new_node
                    });

                    if frame.str.len() == 1 {
                        no_edit_node.set_final(true);
                    }

                    stack.push(Frame {
                        node: no_edit_node,
                        edits_remaining: frame.edits_remaining,
                        str: frame.str[1..].to_string(),
                    });
                }

                if frame.edits_remaining == 0 {
                    continue;
                }

                let insertion_node = frame.node.get_edge("*").unwrap_or_else(|| {
                    let new_node = TokenSet::new();
                    frame.node.set_edge("*".to_string(), new_node.clone());
                    new_node
                });

                if frame.str.is_empty() {
                    insertion_node.set_final(true);
                }

                stack.push(Frame {
                    node: insertion_node,
                    edits_remaining: frame.edits_remaining - 1,
                    str: frame.str.clone(),
                });

                if frame.str.len() > 1 {
                    stack.push(Frame {
                        node: frame.node.clone(),
                        edits_remaining: frame.edits_remaining - 1,
                        str: frame.str[1..].to_string(),
                    });
                }

                if frame.str.len() == 1 {
                    frame.node.set_final(true);
                }

                if !frame.str.is_empty() {
                    let substitution_node = frame.node.get_edge("*").unwrap_or_else(|| {
                        let new_node = TokenSet::new();
                        frame.node.set_edge("*".to_string(), new_node.clone());
                        new_node
                    });

                    if frame.str.len() == 1 {
                        substitution_node.set_final(true);
                    }

                    stack.push(Frame {
                        node: substitution_node,
                        edits_remaining: frame.edits_remaining - 1,
                        str: frame.str[1..].to_string(),
                    });
                }

                if frame.str.len() > 1 {
                    let chars: Vec<char> = frame.str.chars().collect();
                    let char_a = chars[0].to_string();
                    let char_b = chars[1].to_string();

                    let transpose_node = frame.node.get_edge(&char_b).unwrap_or_else(|| {
                        let new_node = TokenSet::new();
                        frame.node.set_edge(char_b.clone(), new_node.clone());
                        new_node
                    });

                    if frame.str.len() == 1 {
                        transpose_node.set_final(true);
                    }

                    let remaining: String = chars[2..].iter().collect();
                    stack.push(Frame {
                        node: transpose_node,
                        edits_remaining: frame.edits_remaining - 1,
                        str: format!("{}{}", char_a, remaining),
                    });
                }
            }

            root
        }

        pub fn to_array(&self) -> Vec<String> {
            if self.get_edges().contains_key("*") {
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
                    stack.push(StackFrame {
                        prefix: format!("{}{}", frame.prefix, edge),
                        node,
                    });
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
                        if n_edge == q_edge || q_edge == "*" {
                            let final_ = n_node.is_final() && q_node.is_final();
                            let next = frame.output.get_edge(n_edge).unwrap_or_else(|| {
                                let new_node = TokenSet::new();
                                frame.output.set_edge(n_edge.clone(), new_node.clone());
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

        fn get_edges(&self) -> HashMap<String, TokenSet> {
            self.inner.borrow().edges.clone()
        }

        fn set_edge(&self, key: String, val: TokenSet) {
            self.inner.borrow_mut().edges.insert(key, val);
        }

        fn get_edge(&self, key: &str) -> Option<TokenSet> {
            self.inner.borrow().edges.get(key).cloned()
        }

        pub fn __struct_fingerprint(&self) -> String {
            fn walk(node: &TokenSet, on_path: &mut Vec<usize>) -> String {
                let id = node.inner.borrow().id;
                if on_path.contains(&id) {
                    return "CYCLE".to_string();
                }
                on_path.push(id);
                let mut s = if node.is_final() { "1" } else { "0" }.to_string();
                let mut edges: Vec<(String, TokenSet)> = node
                    .inner
                    .borrow()
                    .edges
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                edges.sort_by(|a, b| a.0.cmp(&b.0));
                for (label, child) in &edges {
                    s.push('L');
                    s.push_str(label);
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

    struct Frame {
        node: TokenSet,
        edits_remaining: usize,
        str: String,
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

    pub struct Builder {
        previous_word: String,
        pub root: TokenSet,
        unchecked_nodes: Vec<UncheckedNode>,
        minimized_nodes: HashMap<String, TokenSet>,
    }

    struct UncheckedNode {
        parent: TokenSet,
        char: String,
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
                let ch = word_chars[i].to_string();
                let next_node = TokenSet::new();
                node.set_edge(ch.clone(), next_node.clone());

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
                    node.parent.set_edge(node.char.clone(), minimized.clone());
                } else {
                    node.child.set_str(child_key.clone());
                    self.minimized_nodes.insert(child_key, node.child.clone());
                }
            }
        }
    }

    impl TokenSet {
        fn str_repr(&self) -> String {
            if let Some(ref s) = self.inner.borrow()._str {
                return s.clone();
            }

            let mut str = if self.is_final() { "1" } else { "0" }.to_string();
            let mut labels: Vec<String> = self.get_edges().keys().cloned().collect();
            labels.sort();

            for label in &labels {
                let node = self.get_edge(label).unwrap();
                str.push_str(label);
                str.push_str(&node.inner.borrow().id.to_string());
            }

            str
        }

        fn set_str(&self, val: String) {
            self.inner.borrow_mut()._str = Some(val);
        }
    }
}

fn words() -> Vec<String> {
    let raw = include_str!("../benches/data/words.txt");
    raw.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn sorted(v: Vec<String>) -> Vec<String> {
    let mut v = v;
    v.sort();
    v
}

#[test]
fn from_string_structurally_identical() {
    let cases = [
        "test", "september", "javascript", "java*cript", "*ber", "a", "ab", "abc",
        "cat", "cats", "running", "typ*", "*", "a*", "foo*bar",
    ];
    for s in cases {
        let a = TokenSet::from_string(s);
        let b = old::TokenSet::from_string(s);
        assert_eq!(a.__struct_fingerprint(), b.__struct_fingerprint(), "mismatch from_string {s:?}");
    }
}

#[test]
fn from_fuzzy_string_structurally_identical() {
    let cases = [
        "javascript", "test", "foo", "cats", "running", "september", "ab",
    ];
    for s in cases {
        for d in 0..=3 {
            let a = TokenSet::from_fuzzy_string(s, d);
            let b = old::TokenSet::from_fuzzy_string(s, d);
            assert_eq!(
                a.__struct_fingerprint(),
                b.__struct_fingerprint(),
                "mismatch fuzzy {s:?} dist={d}"
            );
        }
    }
}

#[test]
fn from_array_language_identical() {
    let w = words();
    let a = TokenSet::from_array(&w);
    let b = old::TokenSet::from_array(&w);
    assert_eq!(a.__struct_fingerprint(), b.__struct_fingerprint(), "from_array structure mismatch");
    let wa = sorted(a.to_array());
    let wb = sorted(b.to_array());
    assert_eq!(wa, wb, "from_array language mismatch");
}

#[test]
fn intersect_identical() {
    for x in ["test", "cats", "running", "september", "foo"] {
        for y in ["test", "cat", "dog", "run", "sept"] {
            let a = TokenSet::from_string(x).intersect(&TokenSet::from_string(y));
            let b = old::TokenSet::from_string(x).intersect(&old::TokenSet::from_string(y));
            assert_eq!(
                a.__struct_fingerprint(),
                b.__struct_fingerprint(),
                "intersect mismatch {x:?} & {y:?}"
            );
            // language must also agree
            let la = sorted(a.to_array());
            let lb = sorted(b.to_array());
            assert_eq!(la, lb, "intersect language mismatch {x:?} & {y:?}");
        }
    }
}

#[test]
fn from_array_subset_intersect() {
    for n in 1..=50 {
        let w = words()[..n].to_vec();
        let a = TokenSet::from_array(&w);
        let b = old::TokenSet::from_array(&w);
        assert_eq!(a.__struct_fingerprint(), b.__struct_fingerprint(), "subset {n} structure");
        assert_eq!(sorted(a.to_array()), sorted(b.to_array()), "subset {n} language");
    }
}
