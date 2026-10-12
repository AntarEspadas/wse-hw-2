#[derive(Debug)]
pub struct DocScore {
    pub id: u32,
    pub score: f32,
}

impl Ord for DocScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.total_cmp(&other.score)
    }
}

impl PartialOrd for DocScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DocScore {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for DocScore {}
