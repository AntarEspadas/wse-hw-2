use std::collections::VecDeque;

use super::get_mode;

pub struct Simple9DecompressIter<I>
where
    I: Iterator<Item = u32>,
{
    input: I,
    buffer: VecDeque<u32>,
}

impl<I> Iterator for Simple9DecompressIter<I>
where
    I: Iterator<Item = u32>,
{
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(result) = self.buffer.pop_front() {
            return Some(result);
        }

        let word = self.input.next()?;

        decompress_word(word, &mut self.buffer);

        Some(self.buffer.pop_front().unwrap())
    }
}

pub fn decompress<I>(data: I) -> Simple9DecompressIter<I::IntoIter>
where
    I: IntoIterator<Item = u32>,
{
    Simple9DecompressIter {
        input: data.into_iter(),
        buffer: VecDeque::with_capacity(28),
    }
}

pub fn decompress_word(word: u32, buffer: &mut VecDeque<u32>) {
    let selector = get_selector(word);
    match selector {
        0 => decompress_single::<0>(word, buffer),
        1 => decompress_single::<1>(word, buffer),
        2 => decompress_single::<2>(word, buffer),
        3 => decompress_single::<3>(word, buffer),
        4 => decompress_single::<4>(word, buffer),
        5 => decompress_single::<5>(word, buffer),
        6 => decompress_single::<6>(word, buffer),
        7 => decompress_single::<7>(word, buffer),
        8 => decompress_single::<8>(word, buffer),
        _ => panic!("Invalid selector"),
    };
}

fn decompress_single<const SELECTOR: u32>(word: u32, output: &mut VecDeque<u32>) {
    let (n, width) = const { get_mode(SELECTOR) };

    let mask = (1u32 << width) - 1;

    for i in 0..n {
        output.push_back((word >> (i * width)) & mask);
    }
}

fn get_selector(word: u32) -> u32 {
    word >> (u32::BITS - 4)
}
