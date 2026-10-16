use std::ops::Range;

use common::{compression::simple9::Simple9Reader, index::BlockMetadata};

struct PostingBlock {
    last_doc_id: u32,
    doc_ids: Range<usize>,
    scores: Range<usize>,
}

pub struct PostingReader {
    len: usize,
    max_score: u32,
    buffer: Vec<u8>,
    blocks: Vec<PostingBlock>,
    current_block: usize,
    doc_ids: Simple9Reader,
    scores: Simple9Reader,
    advanced: usize,
    prev_doc_id: u32,
}

impl PostingReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        const U32_SIZE: usize = size_of::<u32>();

        let metadata = BlockMetadata::from_bytes(&buffer);
        let metadata_len = U32_SIZE * (3 + 3 * metadata.posting_metadata.len());

        // All document blocks precede all score blocks in the encoded posting list.
        let mut doc_ids_offset = metadata_len;
        let mut scores_offset = metadata_len
            + metadata
                .posting_metadata
                .iter()
                .map(|block| block.doc_ids_len as usize)
                .sum::<usize>();

        let mut blocks = Vec::with_capacity(metadata.posting_metadata.len());

        for posting_metadata in metadata.posting_metadata {
            let doc_ids_end = doc_ids_offset + posting_metadata.doc_ids_len as usize;
            let scores_end = scores_offset + posting_metadata.scores_len as usize;
            let posting_block = PostingBlock {
                last_doc_id: posting_metadata.last_doc_id,
                doc_ids: doc_ids_offset..doc_ids_end,
                scores: scores_offset..scores_end,
            };
            doc_ids_offset = doc_ids_end;
            scores_offset = scores_end;
            blocks.push(posting_block);
        }

        let first_block = blocks.first().unwrap();

        let doc_ids = Simple9Reader::new(first_block.doc_ids.clone());
        let scores = Simple9Reader::new(first_block.scores.clone());

        Self {
            len: metadata.posting_count as usize,
            max_score: metadata.max_score,
            buffer,
            blocks,
            current_block: 0,
            doc_ids,
            scores,
            advanced: 0,
            prev_doc_id: 0,
        }
    }

    pub fn get_next_geq(&mut self, other: u32) -> Option<u32> {
        if self.blocks.get(self.current_block)?.last_doc_id < other {
            let skipped = self.blocks[self.current_block + 1..]
                .partition_point(|block| block.last_doc_id < other);
            self.current_block += skipped + 1;
            let block = self.blocks.get(self.current_block)?;
            self.doc_ids.reset(block.doc_ids.clone());
            self.scores.reset(block.scores.clone());
            // Each block starts with an absolute document ID, followed by gaps.
            self.prev_doc_id = 0;
            self.advanced = 0;
        }

        while let Some(doc_id) = self.doc_ids.peek(&self.buffer) {
            let doc_id = doc_id + self.prev_doc_id;
            if doc_id >= other {
                return Some(doc_id);
            }
            self.prev_doc_id = doc_id;
            self.advanced += 1;
            self.doc_ids.next(&self.buffer);
        }

        None
    }

    pub fn get_current_score(&mut self) -> u32 {
        for _ in 0..self.advanced {
            self.scores.next(&self.buffer);
        }

        self.advanced = 0;

        self.scores.peek(&self.buffer).unwrap()
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

#[cfg(test)]
mod tests {
    use common::{compression::simple9::compress, index::PostingMetadata};

    use super::*;

    fn encode_postings(postings: &[(u32, u32)], block_size: usize) -> Vec<u8> {
        let mut metadata = BlockMetadata {
            posting_count: postings.len() as u32,
            max_score: postings.iter().map(|p| p.1).max().unwrap_or(0),
            posting_metadata: Vec::new(),
        };
        let mut doc_ids = Vec::new();
        let mut scores = Vec::new();

        for block in postings.chunks(block_size) {
            let mut previous = 0;
            let gaps: Vec<_> = block
                .iter()
                .map(|&(doc_id, _)| {
                    let gap = doc_id - previous;
                    previous = doc_id;
                    gap
                })
                .collect();
            let block_scores: Vec<_> = block.iter().map(|p| p.1).collect();
            let compressed_doc_ids: Vec<_> = compress(&gaps).flat_map(u32::to_be_bytes).collect();
            let compressed_scores: Vec<_> =
                compress(&block_scores).flat_map(u32::to_be_bytes).collect();

            metadata.posting_metadata.push(PostingMetadata {
                last_doc_id: previous,
                doc_ids_len: compressed_doc_ids.len() as u32,
                scores_len: compressed_scores.len() as u32,
            });
            doc_ids.extend(compressed_doc_ids);
            scores.extend(compressed_scores);
        }

        let mut result = metadata.to_bytes();
        result.extend(doc_ids);
        result.extend(scores);
        result
    }

    #[test]
    fn reads_known_block_layout() {
        // Header, two block descriptors, all document words, then all score words.
        let words: [u32; 14] = [
            5,
            7000,
            2,
            10,
            4,
            8,
            30,
            4,
            4,
            (2 << 28) | (2 << 9) | (8 << 18), // gaps: 0, 2, 8
            (1 << 28) | 20 | (10 << 14),      // gaps: 20, 10
            (1 << 28) | 6000 | (7000 << 14),
            2,
            (1 << 28) | 300 | (400 << 14),
        ];
        let mut reader = PostingReader::new(words.into_iter().flat_map(u32::to_be_bytes).collect());
        assert_eq!(reader.len(), 5);
        assert_eq!(reader.get_max_score(), 7000);
        assert!(!reader.is_empty());

        for (target, doc_id, score) in [
            (0, 0, 6000),
            (1, 2, 7000),
            (3, 10, 2),
            (11, 20, 300),
            (21, 30, 400),
        ] {
            assert_eq!(reader.get_next_geq(target), Some(doc_id));
            assert_eq!(reader.get_current_score(), score);
        }
        assert_eq!(reader.get_next_geq(31), None);
    }

    #[test]
    fn empty_posting_list_is_exhausted() {
        let mut reader = PostingReader::new(encode_postings(&[], 128));
        assert_eq!(reader.len(), 0);
        assert_eq!(reader.get_max_score(), 0);
        assert!(reader.is_empty());
        assert_eq!(reader.get_next_geq(0), None);
        assert_eq!(reader.get_next_geq(u32::MAX), None);
    }

    #[test]
    fn reads_every_posting_across_words_blocks_and_short_tails() {
        let scores = [0, 1, 31, 127, 511, 16_383, 0x0fff_ffff];
        for len in [1, 27, 28, 29, 127, 128, 129, 256, 263] {
            let postings: Vec<_> = (0..len)
                .map(|i| ((i * (i + 1) * 3 / 2) as u32, scores[i % scores.len()]))
                .collect();
            let mut reader = PostingReader::new(encode_postings(&postings, 128));
            assert_eq!(reader.len(), len);
            assert_eq!(
                reader.get_max_score(),
                *scores.iter().take(len).max().unwrap()
            );

            for &(doc_id, score) in &postings {
                assert_eq!(reader.get_next_geq(doc_id), Some(doc_id));
                assert_eq!(reader.get_current_score(), score);
                assert_eq!(reader.get_next_geq(doc_id), Some(doc_id));
                assert_eq!(reader.get_current_score(), score);
            }
            assert_eq!(reader.get_next_geq(postings.last().unwrap().0 + 1), None);
            assert_eq!(reader.get_next_geq(0), None);
        }
    }

    #[test]
    fn seeks_match_forward_scan_with_lazy_scores() {
        let postings: Vec<_> = (0..800)
            .map(|i| (i * 7 + i % 5, (i * 997) % 20_000))
            .collect();
        let mut reader = PostingReader::new(encode_postings(&postings, 128));
        let mut position = 0;
        let mut targets = vec![0, 0, 1, 50, 49];
        // Exercise block ends, gaps between blocks, and jumps over several blocks.
        for i in [127, 128, 255, 256, 650, 799] {
            targets.extend([postings[i].0 - 1, postings[i].0, postings[i].0 + 1]);
        }
        targets.push(u32::MAX);

        for (i, target) in targets.into_iter().enumerate() {
            while position < postings.len() && postings[position].0 < target {
                position += 1;
            }
            let expected = postings.get(position).copied();
            assert_eq!(reader.get_next_geq(target), expected.map(|p| p.0));
            // Leave scores unread on some seeks to check deferred alignment.
            if i % 3 != 0
                && let Some((_, score)) = expected
            {
                assert_eq!(reader.get_current_score(), score);
                assert_eq!(reader.get_current_score(), score);
            }
        }
    }

    #[test]
    fn skips_blocks_without_decoding_their_payloads() {
        let postings: Vec<_> = (0..520).map(|i| (i * 3, i * 17)).collect();
        let mut buffer = encode_postings(&postings, 128);
        let metadata = BlockMetadata::from_bytes(&buffer);
        let doc_ids_start = 12 + 12 * metadata.posting_metadata.len();
        let scores_start = doc_ids_start
            + metadata
                .posting_metadata
                .iter()
                .map(|b| b.doc_ids_len as usize)
                .sum::<usize>();

        // Invalid Simple9 selectors make any attempt to decode skipped blocks fail.
        let skipped_doc_bytes: usize = metadata.posting_metadata[..3]
            .iter()
            .map(|b| b.doc_ids_len as usize)
            .sum();
        let skipped_score_bytes: usize = metadata.posting_metadata[..3]
            .iter()
            .map(|b| b.scores_len as usize)
            .sum();
        buffer[doc_ids_start..doc_ids_start + skipped_doc_bytes].fill(0xff);
        buffer[scores_start..scores_start + skipped_score_bytes].fill(0xff);

        let mut reader = PostingReader::new(buffer);
        assert_eq!(reader.get_next_geq(postings[384].0), Some(postings[384].0));
        assert_eq!(reader.get_current_score(), postings[384].1);
        assert_eq!(reader.get_next_geq(postings[519].0), Some(postings[519].0));
        assert_eq!(reader.get_current_score(), postings[519].1);
        assert_eq!(reader.get_next_geq(u32::MAX), None);
        assert_eq!(reader.get_next_geq(0), None);
    }
}
