use crate::simple9::get_mode;

pub fn compress(mut data: &[u32]) -> Vec<u32> {
    let mut result = Vec::new();

    while !data.is_empty() {
        let selector = get_selector(data);
        let (n, word) = match selector {
            0 => compress_single::<0>(data),
            1 => compress_single::<1>(data),
            2 => compress_single::<2>(data),
            3 => compress_single::<3>(data),
            4 => compress_single::<4>(data),
            5 => compress_single::<5>(data),
            6 => compress_single::<6>(data),
            7 => compress_single::<7>(data),
            8 => compress_single::<8>(data),
            _ => panic!("Invalid selector"),
        };
        result.push(word);
        data = &data[n..];
    }

    result
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
