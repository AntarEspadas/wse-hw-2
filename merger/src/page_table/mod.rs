use std::{error::Error, fs::File, io::Read, path::Path};

pub struct PageTable {
    lengths: Vec<u32>,
    average_len: usize,
}

impl PageTable {
    pub fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(path)?;

        let mut buffer: Vec<u8> = Vec::new();

        file.read_to_end(&mut buffer)?;

        const U32_SIZE: usize = size_of::<u32>();

        let mut total_len = 0usize;

        let lengths: Vec<_> = buffer
            .as_chunks::<U32_SIZE>()
            .0
            .iter()
            .map(|x| {
                let len = u32::from_be_bytes(*x);
                total_len += len as usize;
                len
            })
            .collect();

        let average_len = total_len / lengths.len();

        Ok(Self {
            lengths,
            average_len,
        })
    }

    pub fn get_doc_len(&self, doc_id: u32) -> Option<u32> {
        self.lengths.get(doc_id as usize).copied()
    }

    #[inline(always)]
    pub fn get_average_doc_len(&self) -> usize {
        self.average_len
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.lengths.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
