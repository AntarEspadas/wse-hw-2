use std::{iter::Peekable, vec::IntoIter};

use common::compression::simple9::decompress::{Simple9DecompressIter, decompress};

pub struct PostingReader {
    len: usize,
    max_score: u32,
    doc_ids: Peekable<Simple9DecompressIter<IntoIter<u32>>>,
    scores: Peekable<Simple9DecompressIter<IntoIter<u32>>>,
    advanced: usize,
    prev_doc_id: u32,
}

impl PostingReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        const U32_SIZE: usize = size_of::<u32>();

        let chunks = buffer.as_chunks::<U32_SIZE>().0;

        let len = u32::from_be_bytes(chunks[0]);
        let max_score = u32::from_be_bytes(chunks[1]);
        let doc_ids_len = u32::from_be_bytes(chunks[2]) as usize;
        let scores_len = u32::from_be_bytes(chunks[3]) as usize;

        let doc_ids: Vec<_> = chunks
            .iter()
            .skip(4)
            .take(doc_ids_len / U32_SIZE)
            .map(|x| u32::from_be_bytes(*x))
            .collect();

        let scores: Vec<_> = chunks
            .iter()
            .skip(4 + doc_ids_len / U32_SIZE)
            .take(scores_len / U32_SIZE)
            .map(|x| u32::from_be_bytes(*x))
            .collect();

        let doc_ids = decompress(doc_ids).peekable();
        let scores = decompress(scores).peekable();

        Self {
            len: len as usize,
            max_score,
            doc_ids,
            scores,
            advanced: 0,
            prev_doc_id: 0,
        }
    }

    pub fn get_next_geq(&mut self, other: u32) -> Option<u32> {
        while let Some(doc_id) = self.doc_ids.peek().cloned() {
            let doc_id = doc_id + self.prev_doc_id;
            if doc_id >= other {
                return Some(doc_id);
            }
            self.prev_doc_id = doc_id;
            self.advanced += 1;
            self.doc_ids.next();
        }

        None
    }

    pub fn get_current_score(&mut self) -> u32 {
        for _ in 0..self.advanced {
            self.scores.next();
        }

        self.advanced = 0;

        *self.scores.peek().unwrap()
    }

    #[inline(always)]
    pub fn get_max_score(&self) -> u32 {
        self.max_score
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
