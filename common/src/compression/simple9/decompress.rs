use super::get_mode;

pub fn decompress(data: &[u32]) -> Vec<u32> {
    let mut result = Vec::new();

    for word in data {
        let selector = get_selector(*word);
        match selector {
            0 => decompress_single::<0>(*word, &mut result),
            1 => decompress_single::<1>(*word, &mut result),
            2 => decompress_single::<2>(*word, &mut result),
            3 => decompress_single::<3>(*word, &mut result),
            4 => decompress_single::<4>(*word, &mut result),
            5 => decompress_single::<5>(*word, &mut result),
            6 => decompress_single::<6>(*word, &mut result),
            7 => decompress_single::<7>(*word, &mut result),
            8 => decompress_single::<8>(*word, &mut result),
            _ => panic!("Invalid selector"),
        };
    }

    result
}

fn decompress_single<const SELECTOR: u32>(word: u32, output: &mut Vec<u32>) {
    let (n, width) = const { get_mode(SELECTOR) };

    let mask = (1u32 << width) - 1;

    for i in 0..n {
        output.push((word >> (i * width)) & mask);
    }
}

fn get_selector(word: u32) -> u32 {
    word >> (u32::BITS - 4)
}
