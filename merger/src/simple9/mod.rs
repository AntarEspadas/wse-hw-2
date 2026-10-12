mod compress;
mod decompress;

pub use compress::compress;
pub use decompress::decompress;

#[inline(always)]
const fn get_mode(selector: u32) -> (usize, usize) {
    match selector {
        0 => (1, 28),
        1 => (2, 14),
        2 => (3, 9),
        3 => (4, 7),
        4 => (5, 5),
        5 => (7, 4),
        6 => (9, 3),
        7 => (14, 2),
        8 => (28, 1),
        _ => panic!("Invalid selector"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (maximum field value, number of fields, encoded word)
    const WIDTH_BOUNDARY_CASES: [(u32, usize, u32); 9] = [
        (268_435_455, 1, 0x0fff_ffff),
        (16_383, 2, 0x1fff_ffff),
        (511, 3, 0x27ff_ffff),
        (127, 4, 0x3fff_ffff),
        (31, 5, 0x41ff_ffff),
        (15, 7, 0x5fff_ffff),
        (7, 9, 0x67ff_ffff),
        (3, 14, 0x7fff_ffff),
        (1, 28, 0x8fff_ffff),
    ];

    fn assert_roundtrip(data: &[u32]) {
        let compressed: Vec<u32> = compress(data).collect();
        let decompressed = decompress(&compressed);

        assert_eq!(data, &decompressed);
    }

    #[test]
    fn empty_data_roundtrips() {
        assert_roundtrip(&[]);
    }

    #[test]
    fn small_increasing_sequence_roundtrips() {
        assert_roundtrip(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn repeated_values_roundtrip() {
        assert_roundtrip(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2,
        ]);
    }

    #[test]
    fn values_at_each_width_boundary_roundtrip() {
        let cases: [&[u32]; 9] = [
            &[268_435_455],
            &[16_383, 16_383],
            &[511, 511, 511],
            &[127, 127, 127, 127],
            &[31, 31, 31, 31, 31],
            &[15, 15, 15, 15, 15, 15, 15],
            &[7, 7, 7, 7, 7, 7, 7, 7, 7],
            &[3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
            &[1; 28],
        ];

        for data in cases {
            assert_roundtrip(data);
        }
    }

    #[test]
    fn mixed_sizes_roundtrip() {
        let data = vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 63, 64, 65, 127, 128, 255, 256, 511, 1023, 2047, 4095,
            8191, 16383, 32767,
        ];

        assert_roundtrip(&data);
    }

    #[test]
    fn single_values_use_one_field_without_padding() {
        let values = [
            0,
            1,
            2,
            3,
            4,
            7,
            8,
            15,
            16,
            31,
            32,
            127,
            128,
            511,
            512,
            16_383,
            16_384,
            268_435_454,
            268_435_455,
        ];

        for value in values {
            assert_eq!(compress(&[value]).collect::<Vec<_>>(), vec![value]);
            assert_eq!(decompress(&[value]), vec![value]);
        }
    }

    #[test]
    fn each_mode_compresses_to_a_known_word() {
        for (value, count, word) in WIDTH_BOUNDARY_CASES {
            let data = vec![value; count];

            assert_eq!(
                compress(&data).collect::<Vec<_>>(),
                vec![word],
                "value {value} repeated {count} times should fit in one word"
            );
        }
    }

    #[test]
    fn each_mode_decompresses_a_known_word() {
        for (value, count, word) in WIDTH_BOUNDARY_CASES {
            assert_eq!(
                decompress(&[word]),
                vec![value; count],
                "incorrect decoding of word {word:#010x}"
            );
        }
    }

    #[test]
    fn distinct_packed_fields_keep_their_order() {
        let cases: &[(u32, &[u32])] = &[
            (0x0000_0001, &[1]),
            (0x1000_8001, &[1, 2]),
            (0x200c_0401, &[1, 2, 3]),
            (0x3080_c101, &[1, 2, 3, 4]),
            (0x4052_0c41, &[1, 2, 3, 4, 5]),
            (0x5765_4321, &[1, 2, 3, 4, 5, 6, 7]),
            (0x60fa_c688, &[0, 1, 2, 3, 4, 5, 6, 7, 0]),
            (0x74e4_e4e4, &[0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1]),
            (
                0x8aaa_aaaa,
                &[
                    0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1,
                    0, 1,
                ],
            ),
        ];

        for &(word, data) in cases {
            assert_eq!(
                compress(data).collect::<Vec<_>>(),
                vec![word],
                "incorrect packing of {data:?}"
            );
            assert_eq!(
                decompress(&[word]).as_slice(),
                data,
                "incorrect field order in word {word:#010x}"
            );
        }
    }

    #[test]
    fn every_small_input_length_roundtrips_without_padding() {
        for len in 0..=57 {
            assert_roundtrip(&vec![0; len]);
            assert_roundtrip(&vec![1; len]);
        }
    }

    #[test]
    fn full_blocks_and_tails_use_expected_selectors() {
        let cases: &[(usize, &[u32])] = &[
            (27, &[0x7000_0000, 0x6000_0000, 0x3000_0000]),
            (28, &[0x8000_0000]),
            (29, &[0x8000_0000, 0x0000_0000]),
            (56, &[0x8000_0000, 0x8000_0000]),
            (57, &[0x8000_0000, 0x8000_0000, 0x0000_0000]),
        ];

        for &(len, words) in cases {
            let data = vec![0; len];

            assert_eq!(
                compress(&data).collect::<Vec<_>>().as_slice(),
                words,
                "incorrect packing for input length {len}"
            );
            assert_eq!(decompress(words), data);
        }
    }

    #[test]
    fn wider_values_in_any_field_prevent_narrow_packing() {
        for (max_value, count, word) in WIDTH_BOUNDARY_CASES.into_iter().skip(1) {
            for position in 0..count {
                let mut data = vec![max_value; count];
                data[position] = max_value + 1;
                let compressed: Vec<_> = compress(&data).collect();

                assert!(
                    (compressed[0] >> 28) < (word >> 28),
                    "selector {} cannot hold value {} at position {position}",
                    word >> 28,
                    max_value + 1
                );
                assert_eq!(
                    decompress(&compressed),
                    data,
                    "value above {max_value} at position {position} must be preserved"
                );
            }
        }
    }

    #[test]
    fn packed_blocks_can_surround_a_wide_value() {
        let mut data = vec![0; 28];
        data.push(268_435_455);
        data.extend_from_slice(&[1; 28]);
        let words = [0x8000_0000, 0x0fff_ffff, 0x8fff_ffff];

        assert_eq!(compress(&data).collect::<Vec<_>>().as_slice(), &words);
        assert_eq!(decompress(&words), data);
    }

    #[test]
    fn mixed_mode_stream_decodes_in_order() {
        let mut words = Vec::new();
        let mut expected = Vec::new();

        for (value, count, word) in WIDTH_BOUNDARY_CASES {
            words.push(word);
            expected.extend(std::iter::repeat_n(value, count));
        }

        assert_eq!(decompress(&words), expected);
    }

    #[test]
    fn unused_payload_bits_do_not_leak_into_values() {
        let cases = [(0x2800_0000, 3), (0x4e00_0000, 5), (0x6800_0000, 9)];

        for (word, count) in cases {
            assert_eq!(decompress(&[word]), vec![0; count]);
        }
    }

    #[test]
    fn compress_rejects_values_above_twenty_eight_bits() {
        for value in [1u32 << 28, 1u32 << 31, u32::MAX] {
            for position in [0, 14, 28] {
                let mut data = vec![0; 29];
                data[position] = value;

                assert!(
                    std::panic::catch_unwind(|| compress(&data).collect::<Vec<_>>()).is_err(),
                    "unsupported value {value} at position {position} must be rejected"
                );
            }
        }
    }

    #[test]
    fn decompress_rejects_every_invalid_selector() {
        for selector in 9..=15 {
            for payload in [0, 0x0fff_ffff] {
                let word = (selector << 28) | payload;

                assert!(
                    std::panic::catch_unwind(|| decompress(&[0x8000_0000, word])).is_err(),
                    "invalid selector {selector} must be rejected"
                );
            }
        }
    }
}
