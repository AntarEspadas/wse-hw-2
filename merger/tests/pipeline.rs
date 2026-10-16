use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use merger::{index::IndexReader, merge};
use query::{
    index::{Index, Lexicon},
    processing::{Query, QueryProcessor, QueryType},
};

type RawEntry = (String, Vec<(u32, u32)>);

struct Fixture {
    parent: PathBuf,
    root: PathBuf,
    input: PathBuf,
    output: PathBuf,
}

impl Fixture {
    fn new(documents: usize) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = std::env::temp_dir().canonicalize().unwrap();
        let root = parent.join(format!(
            "wse-merger-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let input = root.join("input");
        let output = root.join("output");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&output).unwrap();
        let lengths: Vec<_> = (0..documents).flat_map(|_| 100u32.to_be_bytes()).collect();
        fs::write(input.join("page-table.bin"), lengths).unwrap();
        Self {
            parent,
            root,
            input,
            output,
        }
    }

    fn write_partial(&self, number: usize, entries: &[RawEntry]) {
        let mut index = Vec::new();
        let mut lexicon = Vec::new();
        for (term, postings) in entries {
            lexicon.extend_from_slice(term.as_bytes());
            lexicon.push(0);
            lexicon.extend_from_slice(&(index.len() as u32).to_be_bytes());
            lexicon.extend_from_slice(&(postings.len() as u32).to_be_bytes());
            for &(id, _) in postings {
                index.extend_from_slice(&id.to_be_bytes());
            }
            for &(_, frequency) in postings {
                index.extend_from_slice(&frequency.to_be_bytes());
            }
        }
        fs::write(self.input.join(format!("index-{number}.bin")), index).unwrap();
        fs::write(self.input.join(format!("lexicon-{number}.bin")), lexicon).unwrap();
    }

    fn index(&self) -> Index {
        Index::new(
            self.output.join("index.bin"),
            Lexicon::read(&self.output.join("lexicon.bin")).unwrap(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only recursively remove the unique directory this fixture created
        // directly underneath the resolved temporary directory.
        assert_eq!(self.root.parent(), Some(self.parent.as_path()));
        assert!(
            self.root
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("wse-merger-test-")
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn postings(index: &Index, term: &str) -> Vec<(u32, u32)> {
    let mut reader = index.get_postings(term).unwrap().unwrap();
    let mut result = Vec::new();
    let mut target = 0;
    while let Some(id) = reader.get_next_geq(target) {
        result.push((id, reader.get_current_score()));
        target = id + 1;
    }
    result
}

// Reference BM25 for fixtures with equal document lengths, quantized exactly
// as specified by the on-disk format. This does not call the merger's scorer.
fn reference_score(documents: usize, document_frequency: usize, frequency: u32) -> i64 {
    let n = documents as f64;
    let df = document_frequency as f64;
    let tf = frequency as f64;
    let score = (((n - df + 0.5) / (df + 0.5)).log2() * (2.2 * tf / (1.2 + tf))) as f32;
    ((score + 64.0) * 100.0).round() as i64 - 6400
}

#[test]
fn reader_returns_a_partial_final_buffer_before_exhaustion() {
    let fixture = Fixture::new(1);
    let entries: Vec<_> = (0..105)
        .map(|i| (format!("term{i:03}"), vec![(0, 1)]))
        .collect();
    fixture.write_partial(0, &entries);
    for buffer_size in [1, 2, 100, 200] {
        let mut reader = IndexReader::open(
            &fixture.input.join("index-0.bin"),
            &fixture.input.join("lexicon-0.bin"),
            buffer_size,
        )
        .unwrap();
        for (term, expected) in &entries {
            let entry = reader.next_entry().unwrap().unwrap();
            assert_eq!(&entry.term, term);
            let actual: Vec<_> = entry
                .postings
                .iter()
                .map(|posting| (posting.doc_id, posting.frequency))
                .collect();
            assert_eq!(&actual, expected);
        }
        assert!(reader.next_entry().unwrap().is_none());
        assert!(reader.next_entry().unwrap().is_none());
    }
}

#[test]
fn merges_single_small_and_short_final_groups_without_losing_postings() {
    for partial_count in 1..=8 {
        let fixture = Fixture::new(partial_count);
        for id in 0..partial_count {
            fixture.write_partial(id, &[("alpha".into(), vec![(id as u32, 1)])]);
        }
        merge(&fixture.input, &fixture.output, 4, 100).unwrap();
        let actual = postings(&fixture.index(), "alpha");
        assert_eq!(actual.len(), partial_count);
        assert_eq!(
            actual.iter().map(|p| p.0).collect::<Vec<_>>(),
            (0..partial_count as u32).collect::<Vec<_>>()
        );
    }
}

#[test]
fn merge_output_does_not_depend_on_buffer_size() {
    let fixture = Fixture::new(5);
    for id in 0..5 {
        let entries: Vec<_> = (0..105)
            .map(|i| (format!("term{i:03}"), vec![(id, 1)]))
            .collect();
        fixture.write_partial(id as usize, &entries);
    }
    merge(&fixture.input, &fixture.output, 4, 100).unwrap();
    let one = fixture.root.join("buffer-one");
    fs::create_dir(&one).unwrap();
    merge(&fixture.input, &one, 4, 1).unwrap();
    for filename in ["index.bin", "lexicon.bin"] {
        assert_eq!(
            fs::read(fixture.output.join(filename)).unwrap(),
            fs::read(one.join(filename)).unwrap()
        );
    }
    let index = fixture.index();
    for i in 0..105 {
        assert_eq!(postings(&index, &format!("term{i:03}")).len(), 5);
    }
}

#[test]
fn document_frequency_is_constant_across_full_blocks_and_short_tails() {
    let fixture = Fixture::new(1024);
    fixture.write_partial(0, &[("alpha".into(), (0..260).map(|id| (id, 1)).collect())]);
    merge(&fixture.input, &fixture.output, 4, 100).unwrap();
    let actual = postings(&fixture.index(), "alpha");
    assert_eq!(actual.len(), 260);
    let expected = reference_score(1024, 260, 1);
    for (id, encoded) in actual {
        assert_eq!(i64::from(encoded) - 6400, expected, "document {id}");
    }
}

#[test]
fn ranking_thresholds_and_pruning_use_signed_unshifted_scores() {
    let fixture = Fixture::new(1000);
    fixture.write_partial(
        0,
        &[
            ("alpha".into(), vec![(0, 1), (1, 1), (900, 1)]),
            ("beta".into(), vec![(1, 1), (2, 1), (900, 1)]),
            ("common".into(), (0..800).map(|id| (id, 1)).collect()),
        ],
    );
    merge(&fixture.input, &fixture.output, 4, 100).unwrap();
    let processor = QueryProcessor::new(fixture.index());
    let rare = reference_score(1000, 3, 1);
    let common = reference_score(1000, 800, 1);
    assert!(rare > 0 && common < 0);

    let search = |text, mode, threshold| {
        let mut results: Vec<_> = processor
            .perform_query(&Query::new(text), &mode, 1000, threshold)
            .unwrap()
            .into_iter()
            .map(|result| {
                assert_eq!(result.original_score(), result.score as f32 / 100.0);
                (result.id, result.score)
            })
            .collect();
        results.sort_unstable();
        results
    };

    assert_eq!(
        search("alpha beta", QueryType::CONJUNCTIVE, 5.0),
        vec![(1, 2 * rare), (900, 2 * rare)]
    );
    assert_eq!(
        search("alpha beta", QueryType::DISJUNCTIVE, 5.0),
        vec![(0, rare), (1, 2 * rare), (2, rare), (900, 2 * rare)]
    );
    assert_eq!(
        search("alpha common", QueryType::DISJUNCTIVE, 7.0),
        vec![(900, rare)]
    );
    assert!(search("alpha common", QueryType::CONJUNCTIVE, 7.0).is_empty());

    // A negative-only list must have an upper bound of zero: a candidate
    // matching both positive terms can omit the negative term entirely.
    assert_eq!(
        search("alpha beta common", QueryType::DISJUNCTIVE, 15.0),
        vec![(900, 2 * rare)]
    );
    assert!(search("alpha beta common", QueryType::CONJUNCTIVE, 15.0).is_empty());
    let negative = search("common", QueryType::DISJUNCTIVE, -3.0);
    assert_eq!(negative.len(), 800);
    assert!(negative.iter().all(|p| p.1 == common));
    assert!(search("common", QueryType::DISJUNCTIVE, 0.0).is_empty());

    let just_above_sum = 2.0 * rare as f32 / 100.0 + 0.005;
    assert!(search("alpha beta", QueryType::CONJUNCTIVE, just_above_sum).is_empty());
    let exact_sum = 2.0 * rare as f32 / 100.0;
    assert_eq!(
        search("alpha beta", QueryType::CONJUNCTIVE, exact_sum),
        vec![(1, 2 * rare), (900, 2 * rare)]
    );
    assert!(search("absent", QueryType::DISJUNCTIVE, -3.0).is_empty());
    assert!(search("alpha absent", QueryType::CONJUNCTIVE, -3.0).is_empty());
}

#[test]
fn threshold_equal_to_the_displayed_score_is_inclusive() {
    let fixture = Fixture::new(1000);
    let entries: Vec<_> = (480..520)
        .map(|frequency| {
            (
                format!("term{frequency}"),
                (0..frequency).map(|id| (id, 1)).collect(),
            )
        })
        .collect();
    fixture.write_partial(0, &entries);
    merge(&fixture.input, &fixture.output, 4, 100).unwrap();
    let processor = QueryProcessor::new(fixture.index());
    for frequency in 480..520 {
        let query = Query::new(&format!("term{frequency}"));
        let exact = reference_score(1000, frequency, 1) as f32 / 100.0;
        for mode in [QueryType::CONJUNCTIVE, QueryType::DISJUNCTIVE] {
            let results = processor.perform_query(&query, &mode, 1000, exact).unwrap();
            assert_eq!(results.len(), frequency);
            assert!(
                results
                    .iter()
                    .all(|result| result.original_score() == exact)
            );
            assert!(
                processor
                    .perform_query(&query, &mode, 1000, exact.next_up())
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[test]
fn invalid_merge_parameters_and_missing_inputs_return_errors() {
    let fixture = Fixture::new(2);
    assert!(merge(&fixture.input, &fixture.output, 4, 100).is_err());
    fixture.write_partial(0, &[("alpha".into(), vec![(0, 1)])]);
    fixture.write_partial(1, &[("alpha".into(), vec![(1, 1)])]);
    for (fan_in, buffer_size) in [(0, 100), (1, 100), (4, 0)] {
        assert!(merge(&fixture.input, &fixture.output, fan_in, buffer_size).is_err());
    }
}
