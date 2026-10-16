use std::collections::HashSet;

use common::index::normalize;

pub struct Query {
    terms: HashSet<String>,
}

impl Query {
    pub fn new(query: &str) -> Self {
        let terms: HashSet<_> = query.split_whitespace().filter_map(normalize).collect();

        Self { terms }
    }

    #[inline(always)]
    pub fn terms(&self) -> &HashSet<String> {
        &self.terms
    }

    #[inline]
    pub fn contains(&self, token: &str) -> bool {
        let Some(term) = normalize(token) else {
            return false;
        };

        self.terms.contains(&term)
    }
}
