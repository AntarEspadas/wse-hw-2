use std::collections::{
    HashMap,
    hash_map::{Drain, Iter},
};

pub type Index = HashMap<String, Vec<(usize, usize)>>;

pub struct MessageData {
    pub buffer: Vec<u8>,
    pub index: Index,
}

pub enum InMessage {
    Data(MessageData),
    Done,
}

pub enum OutMessage {
    Data(MessageData),
    Error(MessageData),
}

pub struct TermCounter {
    term_counts: HashMap<String, usize>,
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

    pub fn iter(&self) -> Iter<'_, String, usize> {
        self.term_counts.iter()
    }

    pub fn clear(&mut self) {
        self.term_counts.clear();
    }

    pub fn drain(&mut self) -> Drain<'_, String, usize> {
        self.term_counts.drain()
    }
}

impl Default for TermCounter {
    fn default() -> Self {
        Self::new()
    }
}
