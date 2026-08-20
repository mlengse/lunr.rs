use std::collections::HashSet;

pub enum Set {
    Empty,
    Complete,
    Elements {
        elements: HashSet<String>,
        length: usize,
    },
}

impl Set {
    pub fn new(elements: Vec<String>) -> Self {
        let length = elements.len();
        let elements: HashSet<String> = elements.into_iter().collect();
        Set::Elements { elements, length }
    }

    pub fn contains(&self, element: &str) -> bool {
        match self {
            Set::Empty => false,
            Set::Complete => true,
            Set::Elements { elements, .. } => elements.contains(element),
        }
    }

    pub fn intersect(&self, other: &Set) -> Set {
        match (self, other) {
            (_, Set::Complete) => Set::Elements {
                elements: self.elements_set().clone(),
                length: self.length(),
            },
            (_, Set::Empty) => Set::Empty,
            (Set::Complete, _) => Set::Elements {
                elements: other.elements_set().clone(),
                length: other.length(),
            },
            (Set::Empty, _) => Set::Empty,
            (Set::Elements { .. }, Set::Elements { .. }) => {
                let (a, b) = if self.length() < other.length() {
                    (self, other)
                } else {
                    (other, self)
                };
                let intersection: HashSet<String> = a.elements_set()
                    .iter()
                    .filter(|e| b.elements_set().contains(*e))
                    .cloned()
                    .collect();
                let length = intersection.len();
                Set::Elements { elements: intersection, length }
            }
        }
    }

    pub fn union(&self, other: &Set) -> Set {
        match (self, other) {
            (_, Set::Complete) | (Set::Complete, _) => Set::Complete,
            (_, Set::Empty) => Set::Elements {
                elements: self.elements_set().clone(),
                length: self.length(),
            },
            (Set::Empty, _) => Set::Elements {
                elements: other.elements_set().clone(),
                length: other.length(),
            },
            (Set::Elements { .. }, Set::Elements { .. }) => {
                let union: HashSet<String> = self.elements_set()
                    .iter()
                    .chain(other.elements_set().iter())
                    .cloned()
                    .collect();
                let length = union.len();
                Set::Elements { elements: union, length }
            }
        }
    }

    fn elements_set(&self) -> &HashSet<String> {
        match self {
            Set::Elements { elements, .. } => elements,
            _ => panic!("elements_set called on non-Elements set"),
        }
    }

    fn length(&self) -> usize {
        match self {
            Set::Empty => 0,
            Set::Complete => panic!("length called on Complete set"),
            Set::Elements { length, .. } => *length,
        }
    }
}
