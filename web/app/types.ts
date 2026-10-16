export interface SearchResults {
    results: DocumentResult[],
    time_taken: string,
}

export interface DocumentResult {
    doc_id: number,
    score: number,
    snippet: string,
    term_indices: number[],
}
