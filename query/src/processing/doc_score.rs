pub struct DocScore {
    pub id: u32,
    pub score: u32,
}

impl DocScore {
    pub fn original_score(&self) -> f32 {
        (self.score as f32) / 100f32 - 64f32
    }
}

impl Ord for DocScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score.cmp(&other.score)
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
