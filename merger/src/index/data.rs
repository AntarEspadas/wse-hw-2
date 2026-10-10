pub struct Posting {
    pub doc_id: u32,
    pub frequency: u32,
}

pub struct IndexEntry {
    pub term: String,
    pub postings: Vec<Posting>,
}

pub struct LexiconEntry {
    pub term: String,
    pub offset: usize,
    pub len: usize,
}
