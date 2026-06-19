//! # banlex
//!
//! A multilingual content-risk lexicon and classifier for content-moderation
//! pipelines.
//!
//! `banlex` scores input text against configurable risk **categories**
//! (e.g. `spam`, `scam`, `harassment`, `self-harm-keywords`) using a
//! data-driven lexicon mapping each term to a category, a weight, and an
//! optional language tag. It returns per-category scores, the matched terms,
//! and an overall risk verdict against a threshold.
//!
//! The crate is **defensive / moderation scope only**: it is intended to help
//! route or flag content for review, not to generate harmful content.
//!
//! ## Quick start
//!
//! ```
//! use banlex::{default_lexicon, Scanner};
//!
//! let scanner = Scanner::new(default_lexicon(), 1.0);
//! let result = scanner.scan("Please click here and buy now!");
//! assert!(result.total_score > 0.0);
//! for c in &result.category_scores {
//!     println!("{}: {:.2}", c.category, c.score);
//! }
//! ```
//!
//! ## Custom lexicon
//!
//! ```
//! use banlex::{Lexicon, Scanner};
//!
//! let json = r#"{
//!   "name": "demo",
//!   "entries": [
//!     { "term": "free money", "category": "scam", "weight": 2.0, "lang": "en" }
//!   ]
//! }"#;
//! let lex = Lexicon::from_json(json).unwrap();
//! let scanner = Scanner::new(lex, 1.5);
//! let r = scanner.scan("get free money today");
//! assert!(r.flagged);
//! ```

pub mod lexicon;
pub mod scorer;
pub mod tokenize;

pub use lexicon::{default_lexicon, Entry, Lexicon, LexiconError};
pub use scorer::{CategoryScore, Match, ScanResult, Scanner};
pub use tokenize::{normalize_term, tokenize, Token};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn end_to_end_default() {
        let scanner = Scanner::new(default_lexicon(), 1.0);
        let r = scanner.scan("URGENT: verify your account or you have won nothing");
        assert!(r.flagged);
        assert!(!r.matches.is_empty());
        // categories present should be a subset of the lexicon categories.
        let cats = default_lexicon().categories();
        for cs in &r.category_scores {
            assert!(cats.contains(&cs.category));
        }
    }

    #[test]
    fn custom_lexicon_load_and_scan() {
        let json = r#"{
          "name": "tiny",
          "entries": [
            { "term": "promo code", "category": "spam", "weight": 1.0 },
            { "term": "act now", "category": "spam", "weight": 0.5 }
          ]
        }"#;
        let lex = Lexicon::from_json(json).unwrap();
        assert_eq!(lex.categories(), vec!["spam".to_string()]);
        let scanner = Scanner::new(lex, 1.4);
        let r = scanner.scan("Use this promo code and act now");
        assert!((r.total_score - 1.5).abs() < 1e-9);
        assert!(r.flagged);
    }

    #[test]
    fn empty_lexicon_rejected() {
        let json = r#"{ "name": "empty", "entries": [] }"#;
        assert!(Lexicon::from_json(json).is_err());
    }

    #[test]
    fn negative_weight_rejected() {
        let json = r#"{ "entries": [ { "term": "x", "category": "c", "weight": -1.0 } ] }"#;
        assert!(Lexicon::from_json(json).is_err());
    }
}
