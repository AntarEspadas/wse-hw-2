#[derive(Clone, Copy)]
pub struct Posting {
    pub doc_id: u32,
    pub score: f32,
}

pub struct PostingReader {
    // buffer: Vec<u8>,
    postings: Vec<Posting>,
    max_score: f32,
    current: usize,
}

impl PostingReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        const U32_SIZE: usize = size_of::<u32>();

        let len = (buffer.len() - U32_SIZE) / (U32_SIZE * 2);

        let mut parsed_doc_ids: Vec<u32> = Vec::with_capacity(len);
        let mut accumulator = 0u32;

        for chunk in buffer.as_chunks::<4>().0.iter().skip(1).take(len) {
            accumulator += u32::from_be_bytes(*chunk);
            parsed_doc_ids.push(accumulator);
        }

        let scores: Vec<_> = buffer
            .as_chunks::<4>()
            .0
            .iter()
            .skip(len + 1)
            .map(|buf| f32::from_be_bytes(*buf))
            .collect();

        let max_score = &buffer[..U32_SIZE];
        let max_score = f32::from_be_bytes(max_score.try_into().unwrap());

        let postings: Vec<_> = parsed_doc_ids
            .into_iter()
            .zip(scores)
            .map(|(doc_id, score)| Posting { doc_id, score })
            .collect();

        Self {
            postings,
            max_score,
            current: 0,
        }
    }

    pub fn get_next_geq(&mut self, other: u32) -> Option<u32> {
        let result = self
            .postings
            .iter()
            .skip(self.current)
            .enumerate()
            .find(|x| x.1.doc_id >= other);

        let Some((i, posting)) = result else {
            self.current = self.postings.len();
            return None;
        };

        self.current += i;
        Some(posting.doc_id)
    }

    pub fn get_current_score(&self) -> f32 {
        self.postings[self.current].score
    }

    #[inline(always)]
    pub fn get_max_score(&self) -> f32 {
        self.max_score
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.postings.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
