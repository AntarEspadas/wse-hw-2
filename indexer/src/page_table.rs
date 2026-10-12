use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
};

pub struct Document {
    pub id: u32,
    pub len: usize,
}

pub struct PageTable {
    documents: Vec<Document>,
    first_doc_id: u32,
    total_length: usize,
}

impl PageTable {
    pub fn new(first_doc_id: u32) -> Self {
        Self {
            documents: Vec::new(),
            first_doc_id,
            total_length: 0,
        }
    }

    pub fn add(&mut self, document: Document) {
        self.total_length += document.len;
        self.documents.push(document);
    }

    pub fn write(self, writer: &mut BufWriter<File>) -> Result<(), Box<dyn Error>> {
        for document in self.documents {
            let len = (document.len as u32).to_be_bytes();

            writer.write_all(&len)?;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn first_doc_id(&self) -> u32 {
        self.first_doc_id
    }

    #[inline(always)]
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    #[inline(always)]
    pub fn total_length(&self) -> usize {
        self.total_length
    }
}
