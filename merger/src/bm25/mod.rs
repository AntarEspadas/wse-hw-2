use crate::page_table::PageTable;

pub fn pre_compute(
    doc_id: u32,
    term_frequency: f64,
    document_frequency: f64,
    page_table: &PageTable,
) -> f32 {
    let k1 = 1.2;
    let b = 0.75;

    let doc_len = page_table.get_doc_len(doc_id).unwrap() as f64;
    let average_len = page_table.get_average_doc_len() as f64;
    let n = page_table.len() as f64;

    let k = k(k1, b, doc_len, average_len);

    let score = ((n - document_frequency + 0.5) / (document_frequency + 0.5)).log2()
        * (((k1 + 1f64) * term_frequency) / (k + term_frequency));
    score as f32
}

fn k(k1: f64, b: f64, doc_len: f64, average_len: f64) -> f64 {
    k1 * ((1f64 - b) + b * (doc_len / average_len))
}
