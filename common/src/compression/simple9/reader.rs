use std::{collections::VecDeque, ops::Range};

use crate::compression::simple9::decompress::decompress_word;

pub struct Simple9Reader {
    remaining: Range<usize>,
    buffer: VecDeque<u32>,
}

impl Simple9Reader {
    pub fn new(remaining: Range<usize>) -> Self {
        Self {
            remaining,
            buffer: VecDeque::with_capacity(28),
        }
    }

    pub fn reset(&mut self, remaining: Range<usize>) {
        self.remaining = remaining;
        self.buffer.clear();
    }

    pub fn peek(&mut self, input: &[u8]) -> Option<u32> {
        if self.buffer.is_empty() {
            if self.remaining.is_empty() {
                return None;
            }

            let end = self.remaining.start + size_of::<u32>();
            let word = u32::from_be_bytes(input[self.remaining.start..end].try_into().unwrap());
            self.remaining.start = end;
            decompress_word(word, &mut self.buffer);
        }

        self.buffer.front().copied()
    }

    pub fn next(&mut self, input: &[u8]) -> Option<u32> {
        self.peek(input)?;
        self.buffer.pop_front()
    }
}
