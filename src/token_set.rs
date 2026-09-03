use std::collections::HashMap;

const WILDCARD: char = '*';

/// Compact child edge list: kept sorted by `char` so lookups are a binary
/// search and iteration is deterministic. Node degrees are tiny in practice
/// (letter tries), so a `Vec` beats a per-node `HashMap` on both allocation
/// and hashing cost.
#[derive(Debug, Clone)]
struct Edges {
    pairs: Vec<(char, usize)>,
}

impl Edges {
    #[inline]
    fn new() -> Self {
        Edges { pairs: Vec::new() }
    }

    #[inline]
    fn get(&self, ch: char) -> Option<usize> {
        self.pairs
            .binary_search_by(|&(c, _)| c.cmp(&ch))
            .ok()
            .map(|i| self.pairs[i].1)
    }

    /// Insert an edge; if `ch` is already present its target is returned and
    /// the caller should treat the edge as existing.
    fn insert(&mut self, ch: char, target: usize) {
        match self.pairs.binary_search_by(|&(c, _)| c.cmp(&ch)) {
            Ok(i) => self.pairs[i].1 = target,
            Err(i) => self.pairs.insert(i, (ch, target)),
        }
    }

    fn contains(&self, ch: char) -> bool {
        self.get(ch).is_some()
    }
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    root: usize,
    arena: Vec<TokenSetNode>,
}

#[derive(Debug, Clone)]
struct TokenSetNode {
    final_: bool,
    edges: Edges,
}

fn collect_words(
    arena: &[TokenSetNode],
    node: usize,
    prefix: &mut String,
    words: &mut Vec<String>,
) {
    if arena[node].final_ {
        words.push(prefix.clone());
    }
    for &(edge, next) in &arena[node].edges.pairs {
        prefix.push(edge);
        collect_words(arena, next, prefix, words);
        prefix.pop();
    }
}

fn get_or_insert_edge(arena: &mut Vec<TokenSetNode>, node: usize, ch: char) -> usize {
    if let Some(target) = arena[node].edges.get(ch) {
        target
    } else {
        let next = arena.len();
        arena.push(TokenSetNode {
            final_: false,
            edges: Edges::new(),
        });
        arena[node].edges.insert(ch, next);
        next
    }
}

impl TokenSet {
    pub fn builder() -> Builder {
        Builder::new()
    }

    fn new() -> Self {
        let arena = vec![TokenSetNode {
            final_: false,
            edges: Edges::new(),
        }];
        TokenSet { root: 0, arena }
    }

    pub fn from_string(str: &str) -> Self {
        let mut ts = TokenSet::new();
        let mut node = ts.root;
        let arena = &mut ts.arena;

        for (i, ch) in str.chars().enumerate() {
            let is_final = i == str.len() - 1;

            if ch == WILDCARD {
                if !arena[node].edges.contains(WILDCARD) {
                    arena[node].edges.insert(WILDCARD, node);
                }
                arena[node].final_ = is_final;
            } else {
                node = get_or_insert_edge(arena, node, ch);
                arena[node].final_ = is_final;
            }
        }
        ts
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
        let mut ts = TokenSet::new();
        let chars: Vec<char> = str.chars().collect();
        let mut stack = vec![Frame {
            node: ts.root,
            edits_remaining: edit_distance,
            chars: chars.clone(),
        }];

        while let Some(frame) = stack.pop() {
            let arena = &mut ts.arena;

            if let Some(&ch) = frame.chars.first() {
                let no_edit_node = get_or_insert_edge(arena, frame.node, ch);

                if frame.chars.len() == 1 {
                    arena[no_edit_node].final_ = true;
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

            let insertion_node = get_or_insert_edge(arena, frame.node, WILDCARD);

            if frame.chars.is_empty() {
                arena[insertion_node].final_ = true;
            }

            stack.push(Frame {
                node: insertion_node,
                edits_remaining: frame.edits_remaining - 1,
                chars: frame.chars.clone(),
            });

            if frame.chars.len() > 1 {
                stack.push(Frame {
                    node: frame.node,
                    edits_remaining: frame.edits_remaining - 1,
                    chars: frame.chars[1..].to_vec(),
                });
            }

            if frame.chars.len() == 1 {
                arena[frame.node].final_ = true;
            }

            if !frame.chars.is_empty() {
                let substitution_node = get_or_insert_edge(arena, frame.node, WILDCARD);

                if frame.chars.len() == 1 {
                    arena[substitution_node].final_ = true;
                }

                stack.push(Frame {
                    node: substitution_node,
                    edits_remaining: frame.edits_remaining - 1,
                    chars: frame.chars[1..].to_vec(),
                });
            }

            if frame.chars.len() > 1 {
                let c0 = frame.chars[0];
                let c1 = frame.chars[1];

                let transpose_node = get_or_insert_edge(arena, frame.node, c1);

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

        ts
    }

    pub fn to_array(&self) -> Vec<String> {
        let arena = &self.arena;
        if arena[self.root].edges.contains(WILDCARD) {
            panic!("cannot convert a TokenSet containing wildcards to an array");
        }

        let mut words = Vec::new();
        let mut prefix = String::new();
        collect_words(arena, self.root, &mut prefix, &mut words);
        words
    }

    pub fn intersect(&self, b: &TokenSet) -> TokenSet {
        let mut output = TokenSet::new();
        let mut stack = vec![IntersectFrame {
            q_node: b.root,
            output: output.root,
            node: self.root,
        }];

        // All three arenas are distinct; borrow them once for the whole walk
        // instead of per frame.
        let q_arena = &b.arena;
        let n_arena = &self.arena;
        let o_arena = &mut output.arena;

        while let Some(frame) = stack.pop() {
            for &(q_edge, q_next) in &q_arena[frame.q_node].edges.pairs {
                for &(n_edge, n_next) in &n_arena[frame.node].edges.pairs {
                    if n_edge == q_edge || q_edge == WILDCARD {
                        let final_ = n_arena[n_next].final_ && q_arena[q_next].final_;
                        let next = get_or_insert_edge(o_arena, frame.output, n_edge);

                        o_arena[next].final_ = o_arena[next].final_ || final_;

                        stack.push(IntersectFrame {
                            node: n_next,
                            q_node: q_next,
                            output: next,
                        });
                    }
                }
            }
        }

        output
    }

    /// Id-independent, cycle-safe structural fingerprint (testing only).
    #[doc(hidden)]
    pub fn __struct_fingerprint(&self) -> String {
        let arena = &self.arena;
        let mut visited = std::collections::HashSet::new();
        fn walk(
            node: usize,
            arena: &[TokenSetNode],
            visited: &mut std::collections::HashSet<usize>,
        ) -> String {
            if !visited.insert(node) {
                return "CYCLE".to_string();
            }
            let mut s = if arena[node].final_ { "1" } else { "0" }.to_string();
            for &(label, child) in &arena[node].edges.pairs {
                s.push('L');
                s.push(label);
                s.push_str(&walk(child, arena, visited));
            }
            visited.remove(&node);
            s
        }
        walk(self.root, arena, &mut visited)
    }
}

impl Default for TokenSet {
    fn default() -> Self {
        Self::new()
    }
}

struct Frame {
    node: usize,
    edits_remaining: usize,
    chars: Vec<char>,
}

struct IntersectFrame {
    q_node: usize,
    output: usize,
    node: usize,
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
    minimized_nodes: HashMap<Vec<(char, usize)>, usize>,
}

struct UncheckedNode {
    parent: usize,
    char: char,
    child: usize,
}

impl Builder {
    pub fn new() -> Self {
        let root = TokenSet::new();
        Builder {
            previous_word: String::new(),
            root,
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
            self.root.root
        } else {
            self.unchecked_nodes.last().unwrap().child
        };

        let arena = &mut self.root.arena;
        for i in common_prefix..word_chars.len() {
            let ch = word_chars[i];
            let parent = node;
            node = get_or_insert_edge(arena, parent, ch);

            self.unchecked_nodes.push(UncheckedNode {
                parent,
                char: ch,
                child: node,
            });
        }

        arena[node].final_ = true;
        self.previous_word = word.to_string();
    }

    pub fn finish(&mut self) {
        self.minimize(0);
    }

    fn minimize(&mut self, down_to: usize) {
        let arena = &mut self.root.arena;
        while self.unchecked_nodes.len() > down_to {
            let unchecked = self.unchecked_nodes.pop().unwrap();
            let child_min_key = {
                let child = &arena[unchecked.child];
                let mut key: Vec<(char, usize)> = child.edges.pairs.clone();
                key.sort_by_key(|(k, _)| *k);
                let mut full_key = Vec::with_capacity(1 + key.len());
                full_key.push((if child.final_ { '\x01' } else { '\x00' }, 0));
                full_key.extend(key);
                full_key
            };

            if let Some(&minimized) = self.minimized_nodes.get(&child_min_key) {
                arena[unchecked.parent].edges.insert(unchecked.char, minimized);
            } else {
                self.minimized_nodes
                    .insert(child_min_key, unchecked.child);
            }
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_string() {
        let ts = TokenSet::from_string("test");
        let arena = &ts.arena;
        assert!(arena[ts.root].edges.contains('t'));
        assert!(!arena[ts.root].final_);
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
