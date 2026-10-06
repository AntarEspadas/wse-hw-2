use std::{
    cell::UnsafeCell,
    collections::{
        HashMap, LinkedList,
        hash_map::{Drain, Iter},
    },
};

pub type OwnedIndex = HashMap<String, LinkedList<(u32, u32)>>;
pub type Index<'a> = HashMap<&'a str, LinkedList<(u32, u32)>>;

pub struct MessageData {
    pub buffer: Vec<u8>,
    pub index: OwnedIndex,
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

struct LexiconInner {
    dict: HashMap<String, u32>,
    next_id: u32,
}

pub struct Lexicon {
    inner: UnsafeCell<LexiconInner>,
}

impl Lexicon {
    pub fn new() -> Self {
        Self {
            inner: UnsafeCell::new(LexiconInner {
                dict: HashMap::new(),
                next_id: 0,
            }),
        }
    }

    pub fn add(&self, term: String) -> &str {
        unsafe {
            let inner = &mut *self.inner.get();

            if let Some((key, _)) = inner.dict.get_key_value(term.as_str()) {
                let key_ptr: *const str = key.as_str();

                return &*key_ptr;
            }

            let id = inner.next_id;

            inner.next_id += 1;

            // Point at the allocation owned by `word`.
            let key_ptr: *const str = term.as_str();

            inner.dict.insert(term, id);

            // Moving the String into the HashMap does not move its
            // backing allocation.
            &*key_ptr
        }
    }

    pub fn iter(&self) -> Iter<'_, String, u32> {
        unsafe {
            let inner = &*self.inner.get();
            inner.dict.iter()
        }
    }
}

impl Default for Lexicon {
    fn default() -> Self {
        Self::new()
    }
}
