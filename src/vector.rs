use std::cell::Cell;
use std::collections::BTreeMap;

use serde::ser::{Serialize, Serializer, SerializeSeq};

#[derive(Debug, Default, Clone)]
pub struct Vector {
    elements: BTreeMap<u32, f64>,
    magnitude_cache: Cell<Option<f64>>,
}

impl Vector {
    pub fn new() -> Self {
        Vector {
            elements: BTreeMap::new(),
            magnitude_cache: Cell::new(None),
        }
    }

    pub fn from_elements(elements: Vec<f64>) -> Self {
        let mut map = BTreeMap::new();
        let mut i = 0;
        while i + 1 < elements.len() {
            let index = elements[i] as u32;
            let val = elements[i + 1];
            map.insert(index, val);
            i += 2;
        }
        Vector {
            elements: map,
            magnitude_cache: Cell::new(None),
        }
    }

    pub fn insert(&mut self, index: u32, val: f64) {
        self.upsert(index, val, |_, _| {
            panic!("duplicate index")
        });
    }

    pub fn upsert<F>(&mut self, index: u32, val: f64, merge_fn: F)
    where
        F: FnOnce(f64, f64) -> f64,
    {
        self.magnitude_cache.set(None);
        if let Some(existing) = self.elements.get(&index) {
            let merged = merge_fn(*existing, val);
            self.elements.insert(index, merged);
        } else {
            self.elements.insert(index, val);
        }
    }

    pub fn magnitude(&self) -> f64 {
        if let Some(m) = self.magnitude_cache.get() {
            return m;
        }
        let sum: f64 = self.elements.values().map(|v| v * v).sum();
        let m = sum.sqrt();
        self.magnitude_cache.set(Some(m));
        m
    }

    pub fn dot(&self, other: &Vector) -> f64 {
        let mut dot_product = 0.0;
        let mut a_iter = self.elements.iter();
        let mut b_iter = other.elements.iter();
        let mut a_next = a_iter.next();
        let mut b_next = b_iter.next();

        while let (Some((&a_idx, &a_val)), Some((&b_idx, &b_val))) = (a_next, b_next) {
            if a_idx < b_idx {
                a_next = a_iter.next();
            } else if a_idx > b_idx {
                b_next = b_iter.next();
            } else {
                dot_product += a_val * b_val;
                a_next = a_iter.next();
                b_next = b_iter.next();
            }
        }

        dot_product
    }

    pub fn similarity(&self, other: &Vector) -> f64 {
        let d = self.dot(other);
        let m = self.magnitude();
        if m == 0.0 {
            0.0
        } else {
            d / m
        }
    }

    pub fn to_array(&self) -> Vec<f64> {
        self.elements.values().copied().collect()
    }
}

impl Serialize for Vector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.elements.len() * 2))?;

        for (index, score) in &self.elements {
            seq.serialize_element(index)?;
            seq.serialize_element(score)?;
        }

        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_serialize() {
        let mut v = Vector::new();
        v.insert(0, 1.5);
        v.insert(2, 3.0);
        let arr = v.to_array();
        assert_eq!(vec![1.5, 3.0], arr);
    }

    #[test]
    fn upsert_merges() {
        let mut v = Vector::new();
        v.upsert(0, 1.0, |a, b| a + b);
        v.upsert(0, 2.0, |a, b| a + b);
        assert_eq!(3.0, v.elements[&0]);
    }

    #[test]
    fn magnitude_empty() {
        let v = Vector::new();
        assert_eq!(0.0, v.magnitude());
    }

    #[test]
    fn magnitude_basic() {
        let mut v = Vector::new();
        v.insert(0, 3.0);
        v.insert(1, 4.0);
        assert_eq!(5.0, v.magnitude());
    }

    #[test]
    fn dot_orthogonal() {
        let mut a = Vector::new();
        let mut b = Vector::new();
        a.insert(0, 1.0);
        b.insert(1, 1.0);
        assert_eq!(0.0, a.dot(&b));
    }

    #[test]
    fn dot_shared() {
        let mut a = Vector::new();
        let mut b = Vector::new();
        a.insert(0, 2.0);
        a.insert(1, 3.0);
        b.insert(0, 4.0);
        b.insert(1, 5.0);
        assert_eq!(23.0, a.dot(&b));
    }

    #[test]
    fn similarity_basic() {
        let mut a = Vector::new();
        let mut b = Vector::new();
        a.insert(0, 1.0);
        a.insert(1, 1.0);
        b.insert(0, 1.0);
        let sim = a.similarity(&b);
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((sim - expected).abs() < 1e-10);
    }

    #[test]
    fn similarity_zero_magnitude() {
        let a = Vector::new();
        let b = Vector::new();
        assert_eq!(0.0, a.similarity(&b));
    }

    #[test]
    fn from_elements() {
        let v = Vector::from_elements(vec![0.0, 1.5, 2.0, 3.0]);
        assert_eq!(2, v.elements.len());
        assert_eq!(1.5, v.elements[&0]);
        assert_eq!(3.0, v.elements[&2]);
    }
}
