//! Scoring engine: match a lexicon against tokenized text and produce
//! per-category scores, matched terms, and an overall verdict.

use crate::lexicon::{Entry, Lexicon};
use crate::tokenize::{normalize_term, tokenize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single matched term occurrence in the scanned text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Match {
    /// The lexicon term (as authored) that matched.
    pub term: String,
    /// The category the term belongs to.
    pub category: String,
    /// The weight contributed by this match.
    pub weight: f64,
    /// Token index at which the matched phrase begins.
    pub start_token: usize,
    /// Number of tokens the phrase spans.
    pub token_len: usize,
    /// Optional language tag from the lexicon entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

/// Score for one category: summed weight and the matches that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryScore {
    /// Category name.
    pub category: String,
    /// Summed weight across all matches in this category.
    pub score: f64,
    /// Number of matches in this category.
    pub match_count: usize,
}

/// The full result of scanning a piece of text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanResult {
    /// Total number of tokens in the scanned text.
    pub token_count: usize,
    /// Per-category scores, sorted by descending score then category name.
    pub category_scores: Vec<CategoryScore>,
    /// Every individual match, in order of appearance.
    pub matches: Vec<Match>,
    /// The single highest category score across all categories.
    pub total_score: f64,
    /// The threshold the result was evaluated against.
    pub threshold: f64,
    /// `true` when `total_score >= threshold`.
    pub flagged: bool,
}

impl ScanResult {
    /// Human verdict string derived from `flagged`.
    pub fn verdict(&self) -> &'static str {
        if self.flagged {
            "FLAGGED"
        } else {
            "CLEAR"
        }
    }
}

/// A scanner binds a lexicon to a threshold and performs scans.
#[derive(Debug, Clone)]
pub struct Scanner {
    lexicon: Lexicon,
    threshold: f64,
    // Pre-normalized phrase tokens kept alongside their source entry, sorted
    // longest-phrase-first so longer phrases win at a given position.
    normalized: Vec<(Vec<String>, Entry)>,
}

impl Scanner {
    /// Create a scanner from a lexicon and a flag threshold.
    pub fn new(lexicon: Lexicon, threshold: f64) -> Self {
        let mut normalized: Vec<(Vec<String>, Entry)> = lexicon
            .entries
            .iter()
            .filter_map(|e| {
                let toks = normalize_term(&e.term);
                if toks.is_empty() {
                    None
                } else {
                    Some((toks, e.clone()))
                }
            })
            .collect();
        // Longest phrase first so multi-word phrases take priority over any
        // single-word term that might share a prefix token.
        normalized.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Scanner {
            lexicon,
            threshold,
            normalized,
        }
    }

    /// The bound flag threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// The bound lexicon.
    pub fn lexicon(&self) -> &Lexicon {
        &self.lexicon
    }

    /// Scan `text` and return a full [`ScanResult`].
    ///
    /// Matching is non-overlapping per position: at each token position the
    /// longest matching phrase is taken, and matching resumes after it. This
    /// prevents a phrase and its sub-words from double-counting at the same
    /// span. The same phrase recurring later in the text counts each time.
    pub fn scan(&self, text: &str) -> ScanResult {
        let tokens: Vec<String> = tokenize(text).into_iter().map(|t| t.text).collect();
        let n = tokens.len();
        let mut matches: Vec<Match> = Vec::new();

        let mut i = 0usize;
        while i < n {
            // Find the longest phrase (already sorted longest-first) that
            // matches starting at position i.
            let mut consumed = 0usize;
            for (phrase, entry) in &self.normalized {
                let plen = phrase.len();
                if plen == 0 || i + plen > n {
                    continue;
                }
                if tokens[i..i + plen] == phrase[..] {
                    matches.push(Match {
                        term: entry.term.clone(),
                        category: entry.category.clone(),
                        weight: entry.weight,
                        start_token: i,
                        token_len: plen,
                        lang: entry.lang.clone(),
                    });
                    consumed = plen;
                    break;
                }
            }
            if consumed > 0 {
                i += consumed;
            } else {
                i += 1;
            }
        }

        // Aggregate per-category.
        let mut agg: BTreeMap<String, (f64, usize)> = BTreeMap::new();
        for m in &matches {
            let e = agg.entry(m.category.clone()).or_insert((0.0, 0));
            e.0 += m.weight;
            e.1 += 1;
        }

        let mut category_scores: Vec<CategoryScore> = agg
            .into_iter()
            .map(|(category, (score, match_count))| CategoryScore {
                category,
                score,
                match_count,
            })
            .collect();
        // Sort by descending score, then category name for stable output.
        category_scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.category.cmp(&b.category))
        });

        let total_score = category_scores
            .iter()
            .map(|c| c.score)
            .fold(0.0_f64, f64::max);
        let flagged = total_score >= self.threshold;

        ScanResult {
            token_count: n,
            category_scores,
            matches,
            total_score,
            threshold: self.threshold,
            flagged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::default_lexicon;

    fn scanner() -> Scanner {
        Scanner::new(default_lexicon(), 1.0)
    }

    #[test]
    fn scores_single_word_and_phrase() {
        let r = scanner().scan("Please click here and buy now!");
        // "click here" (0.8) + "buy now" (1.0) both spam.
        let spam = r
            .category_scores
            .iter()
            .find(|c| c.category == "spam")
            .unwrap();
        assert!((spam.score - 1.8).abs() < 1e-9);
        assert_eq!(spam.match_count, 2);
    }

    #[test]
    fn multiword_phrase_does_not_double_count_subwords() {
        // "send gift card" is a single scam phrase (1.4); "card" alone is not
        // a term, so total scam matches should be exactly 1.
        let r = scanner().scan("Please send gift card now");
        let scam = r
            .category_scores
            .iter()
            .find(|c| c.category == "scam")
            .unwrap();
        assert_eq!(scam.match_count, 1);
        assert!((scam.score - 1.4).abs() < 1e-9);
    }

    #[test]
    fn threshold_gate_flags_and_clears() {
        let high = Scanner::new(default_lexicon(), 100.0);
        assert!(!high.scan("you have won a prize, send gift card").flagged);

        let low = Scanner::new(default_lexicon(), 0.5);
        let r = low.scan("you have won");
        assert!(r.flagged);
        assert_eq!(r.verdict(), "FLAGGED");
    }

    #[test]
    fn clean_text_scores_zero() {
        let r = scanner().scan("The quick brown fox jumps over the lazy dog");
        assert_eq!(r.matches.len(), 0);
        assert_eq!(r.total_score, 0.0);
        assert_eq!(r.verdict(), "CLEAR");
    }

    #[test]
    fn repeated_phrase_counts_each_occurrence() {
        let r = scanner().scan("buy now buy now");
        let spam = r
            .category_scores
            .iter()
            .find(|c| c.category == "spam")
            .unwrap();
        assert_eq!(spam.match_count, 2);
    }

    #[test]
    fn multilingual_match() {
        let r = scanner().scan("¡Oferta limitada! Compra ahora");
        let spam = r
            .category_scores
            .iter()
            .find(|c| c.category == "spam")
            .unwrap();
        assert_eq!(spam.match_count, 2);
    }

    #[test]
    fn total_score_is_max_category() {
        // scam phrase weight (1.3) should dominate a lone 0.8 spam match.
        let r = scanner().scan("click here, you have won");
        assert!((r.total_score - 1.3).abs() < 1e-9);
    }
}
