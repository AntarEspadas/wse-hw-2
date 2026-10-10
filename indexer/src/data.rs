use std::collections::{
    HashMap,
    hash_map::{Drain, Iter},
};

use dashmap::DashMap;

pub type IndexElement = (String, Vec<(u32, u32)>);
pub type Index = DashMap<String, Vec<(u32, u32)>>;

pub struct TermCounter {
    term_counts: HashMap<String, u32>,
}

impl TermCounter {
    pub fn new() -> Self {
        Self {
            term_counts: HashMap::new(),
        }
    }

    pub fn count(&mut self, term: String) {
        let entry = self.term_counts.entry(term);
        entry.and_modify(|x| *x += 1).or_insert(1);
    }

    pub fn iter(&self) -> Iter<'_, String, u32> {
        self.term_counts.iter()
    }

    pub fn clear(&mut self) {
        self.term_counts.clear();
    }

    pub fn drain(&mut self) -> Drain<'_, String, u32> {
        self.term_counts.drain()
    }
}

impl Default for TermCounter {
    fn default() -> Self {
        Self::new()
    }
}
