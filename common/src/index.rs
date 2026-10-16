pub const SCORE_SCALE: f32 = 100.0;
pub const SCORE_OFFSET: f32 = 64.0;

pub fn encode_score(score: f32) -> u32 {
    ((score + SCORE_OFFSET) * SCORE_SCALE).round() as u32
}

pub fn decode_score(score: u32) -> i64 {
    i64::from(score) - (SCORE_OFFSET * SCORE_SCALE) as i64
}

pub struct BlockMetadata {
    pub posting_count: u32,
    pub max_score: u32,
    pub posting_metadata: Vec<PostingMetadata>,
}

pub struct PostingMetadata {
    pub last_doc_id: u32,
    pub doc_ids_len: u32,
    pub scores_len: u32,
}

impl BlockMetadata {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        let blocks = self.posting_metadata.len() as u32;

        result.extend(self.posting_count.to_be_bytes());
        result.extend(self.max_score.to_be_bytes());
        result.extend(blocks.to_be_bytes());

        for metadata in self.posting_metadata.iter() {
            metadata.to_bytes(&mut result);
        }

        result
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        const U32_SIZE: usize = size_of::<u32>();
        let words = bytes.as_chunks::<U32_SIZE>().0;

        let posting_count = u32::from_be_bytes(words[0]);
        let max_score = u32::from_be_bytes(words[1]);
        let blocks = u32::from_be_bytes(words[2]);

        let bytes = &bytes[U32_SIZE * 3..];

        let posting_metadata: Vec<_> = bytes
            .as_chunks::<12>()
            .0
            .iter()
            .take(blocks as usize)
            .map(|x| PostingMetadata::from_bytes(*x))
            .collect();

        Self {
            posting_count,
            max_score,
            posting_metadata,
        }
    }
}

impl PostingMetadata {
    pub fn to_bytes(&self, buffer: &mut Vec<u8>) {
        buffer.extend(self.last_doc_id.to_be_bytes());
        buffer.extend(self.doc_ids_len.to_be_bytes());
        buffer.extend(self.scores_len.to_be_bytes());
    }

    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        const U32_SIZE: usize = size_of::<u32>();
        let last_doc_id = u32::from_be_bytes(bytes[..U32_SIZE].try_into().unwrap());
        let doc_ids_len = u32::from_be_bytes(bytes[U32_SIZE..U32_SIZE * 2].try_into().unwrap());
        let scores_len = u32::from_be_bytes(bytes[U32_SIZE * 2..U32_SIZE * 3].try_into().unwrap());
        Self {
            last_doc_id,
            doc_ids_len,
            scores_len,
        }
    }
}
