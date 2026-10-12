use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

#[derive(Clone, Copy)]
pub struct LexiconEntry {
    pub offset: usize,
    pub len: usize,
}
pub struct Lexicon {
    entries: HashMap<String, LexiconEntry>,
}

impl Lexicon {
    pub fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;

        let mut reader = BufReader::new(file);

        let mut entries = HashMap::new();

        loop {
            let mut term: Vec<u8> = Vec::new();
            let read = reader.read_until(b'\0', &mut term)?;

            if read == 0 {
                break;
            }

            // Remove null byte from end
            term.truncate(term.len() - 1);

            let term = String::from_utf8(term)?;

            let mut offset = [0u8; 4];
            let mut length = [0u8; 4];

            reader.read_exact(&mut offset)?;
            reader.read_exact(&mut length)?;

            entries.insert(
                term,
                LexiconEntry {
                    offset: u32::from_be_bytes(offset) as usize,
                    len: u32::from_be_bytes(length) as usize,
                },
            );
        }

        Ok(Self { entries })
    }

    pub fn get(&self, term: &str) -> Option<&LexiconEntry> {
        self.entries.get(term)
    }
}
