use std::{
    collections::{
        HashMap,
        hash_map::{Drain, Iter},
    },
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use parking_lot::Mutex;

use dashmap::{DashMap, ReadOnlyView};

pub type IndexElement = Mutex<Option<Vec<(u32, u32)>>>;
pub type Index = Vec<IndexElement>;

pub struct IndexMessage {
    pub buffer: Vec<u8>,
    pub index: Arc<Index>,
}

pub struct MergeMessage {
    pub src: Index,
    pub dest: Index,
}

pub enum InMessage {
    Index(IndexMessage),
    Merge(MergeMessage),
    Done,
}

pub enum OutMessage {
    IndexDone(IndexMessage),
    MergeDone(MergeMessage),
    IndexError(IndexMessage),
}

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

pub struct Lexicon {
    dict: DashMap<String, usize>,
    next_id: AtomicUsize,
}

impl Lexicon {
    pub fn new() -> Self {
        Self {
            dict: DashMap::new(),
            next_id: AtomicUsize::new(0),
        }
    }

    pub fn add(&self, term: String) -> usize {
        *self
            .dict
            .entry(term)
            .or_insert_with(|| self.next_id.fetch_add(1, Ordering::Relaxed))
            .value()
    }

    pub fn len(&self) -> usize {
        self.dict.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dict.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.dict.capacity()
    }

    pub fn into_readonly(self) -> ReadOnlyView<String, usize> {
        self.dict.into_read_only()
    }

    pub fn from_readonly(lexicon: ReadOnlyView<String, usize>) -> Self {
        let next_id = lexicon.len();
        Self {
            dict: lexicon.into_inner(),
            next_id: AtomicUsize::new(next_id),
        }
    }
}

impl Default for Lexicon {
    fn default() -> Self {
        Self::new()
    }
}
