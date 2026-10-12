use crate::simple9::get_mode;

pub struct Simple9CompressIter<'a> {
    data: &'a [u32],
}

impl<'a> Iterator for Simple9CompressIter<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        let selector = get_selector(self.data);
        let (n, word) = match selector {
            0 => compress_single::<0>(self.data),
            1 => compress_single::<1>(self.data),
            2 => compress_single::<2>(self.data),
            3 => compress_single::<3>(self.data),
            4 => compress_single::<4>(self.data),
            5 => compress_single::<5>(self.data),
            6 => compress_single::<6>(self.data),
            7 => compress_single::<7>(self.data),
            8 => compress_single::<8>(self.data),
            _ => panic!("Invalid selector"),
        };
        self.data = &self.data[n..];
        Some(word)
    }
}

pub fn compress(data: &[u32]) -> Simple9CompressIter<'_> {
    Simple9CompressIter { data }
}

fn get_selector(data: &[u32]) -> u32 {
    for selector in (0..9).rev() {
        let (n, width) = get_mode(selector);
        if has_n_of_width_k(data, n, width as u32) {
            return selector;
        }
    }
    panic!("Invalid selector")
}

fn compress_single<const SELECTOR: u32>(data: &[u32]) -> (usize, u32) {
    let (n, width) = const { get_mode(SELECTOR) };
    let mut result = SELECTOR << 28;

    for (i, datum) in data[..n].iter().enumerate() {
        result |= datum << (i * width);
    }
    (n, result)
}

#[inline(always)]
fn has_n_of_width_k(slice: &[u32], n: usize, k: u32) -> bool {
    if slice.len() < n {
        return false;
    }
    slice.iter().take(n).all(|x| x.bit_width() <= k)
}
