use crate::inverted_index::Posting;

pub fn idf(posting: &Posting, document_count: usize) -> f64 {
    let documents_with_term = posting.document_count();
    let x = (document_count as f64 - documents_with_term as f64 + 0.5)
          / (documents_with_term as f64 + 0.5);
    (1.0 + x.abs()).ln()
}
