//! Lexicon types and the built-in default lexicon.
//!
//! A lexicon is a collection of *entries*, each mapping a term (single word or
//! multi-word phrase) to a risk category, a weight, and an optional language
//! tag. Lexicons are data-driven: they can be authored as JSON and loaded at
//! runtime, or the small built-in default can be used.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A single lexicon entry: one term tied to a category and a weight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The surface term to match. May contain spaces for multi-word phrases.
    pub term: String,
    /// The risk category this term contributes to (e.g. `spam`, `scam`).
    pub category: String,
    /// Positive weight contributed to the category score on each match.
    pub weight: f64,
    /// Optional BCP-47-ish language tag (e.g. `en`, `es`, `fr`). Informational.
    #[serde(default)]
    pub lang: Option<String>,
}

/// A loaded lexicon plus metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lexicon {
    /// Human-readable name for this lexicon.
    #[serde(default = "default_name")]
    pub name: String,
    /// The list of entries.
    pub entries: Vec<Entry>,
}

fn default_name() -> String {
    "custom".to_string()
}

impl Lexicon {
    /// Build a lexicon directly from a list of entries.
    pub fn new(name: impl Into<String>, entries: Vec<Entry>) -> Self {
        Lexicon {
            name: name.into(),
            entries,
        }
    }

    /// Parse a lexicon from a JSON string.
    ///
    /// The JSON must be an object of the shape:
    /// `{ "name": "...", "entries": [ { "term", "category", "weight", "lang"? } ] }`
    pub fn from_json(json: &str) -> Result<Lexicon, LexiconError> {
        let lex: Lexicon = serde_json::from_str(json).map_err(LexiconError::Parse)?;
        lex.validate()?;
        Ok(lex)
    }

    /// Serialize this lexicon to pretty JSON.
    pub fn to_json(&self) -> Result<String, LexiconError> {
        serde_json::to_string_pretty(self).map_err(LexiconError::Parse)
    }

    /// Validate structural invariants: non-empty terms/categories, finite
    /// non-negative weights, and at least one entry.
    pub fn validate(&self) -> Result<(), LexiconError> {
        if self.entries.is_empty() {
            return Err(LexiconError::Empty);
        }
        for (i, e) in self.entries.iter().enumerate() {
            if e.term.trim().is_empty() {
                return Err(LexiconError::BadEntry(i, "term is empty".into()));
            }
            if e.category.trim().is_empty() {
                return Err(LexiconError::BadEntry(i, "category is empty".into()));
            }
            if !e.weight.is_finite() || e.weight < 0.0 {
                return Err(LexiconError::BadEntry(
                    i,
                    "weight must be finite and non-negative".into(),
                ));
            }
        }
        Ok(())
    }

    /// Return the sorted, de-duplicated set of category names in this lexicon.
    pub fn categories(&self) -> Vec<String> {
        let set: BTreeSet<&str> = self.entries.iter().map(|e| e.category.as_str()).collect();
        set.into_iter().map(|s| s.to_string()).collect()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the lexicon has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Errors that can arise while loading or validating a lexicon.
#[derive(Debug)]
pub enum LexiconError {
    /// The JSON could not be parsed or serialized.
    Parse(serde_json::Error),
    /// The lexicon contained no entries.
    Empty,
    /// An individual entry failed validation (index + reason).
    BadEntry(usize, String),
}

impl std::fmt::Display for LexiconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexiconError::Parse(e) => write!(f, "lexicon JSON error: {e}"),
            LexiconError::Empty => write!(f, "lexicon contains no entries"),
            LexiconError::BadEntry(i, why) => write!(f, "lexicon entry {i} invalid: {why}"),
        }
    }
}

impl std::error::Error for LexiconError {}

/// The built-in default lexicon.
///
/// These terms are an ORIGINAL, hand-authored short list of generic risk
/// *indicators* across a few languages. They are deliberately mild, common
/// indicator phrases chosen to demonstrate the scoring engine — not a
/// comprehensive or harmful wordlist, and not copied from any external source.
///
/// Categories: `spam`, `scam`, `harassment`, `self-harm-keywords`.
pub fn default_lexicon() -> Lexicon {
    // (term, category, weight, lang)
    let raw: &[(&str, &str, f64, &str)] = &[
        // --- spam: promotional / unsolicited-bulk indicators ---
        ("buy now", "spam", 1.0, "en"),
        ("limited offer", "spam", 1.0, "en"),
        ("click here", "spam", 0.8, "en"),
        ("act now", "spam", 0.8, "en"),
        ("subscribe today", "spam", 0.7, "en"),
        ("oferta limitada", "spam", 1.0, "es"),
        ("compra ahora", "spam", 1.0, "es"),
        ("offre limitee", "spam", 1.0, "fr"),
        ("achetez maintenant", "spam", 1.0, "fr"),
        ("jetzt kaufen", "spam", 1.0, "de"),

        // --- scam: fraud / advance-fee / credential-bait indicators ---
        ("wire transfer", "scam", 1.2, "en"),
        ("you have won", "scam", 1.3, "en"),
        ("verify your account", "scam", 1.1, "en"),
        ("send gift card", "scam", 1.4, "en"),
        ("urgent payment", "scam", 1.0, "en"),
        ("has ganado", "scam", 1.3, "es"),
        ("transferencia urgente", "scam", 1.0, "es"),
        ("vous avez gagne", "scam", 1.3, "fr"),
        ("paiement urgent", "scam", 1.0, "fr"),
        ("dringende zahlung", "scam", 1.0, "de"),

        // --- harassment: hostile / threatening tone indicators ---
        ("shut up", "harassment", 0.9, "en"),
        ("you are worthless", "harassment", 1.2, "en"),
        ("get lost", "harassment", 0.7, "en"),
        ("nobody likes you", "harassment", 1.0, "en"),
        ("callate", "harassment", 0.9, "es"),
        ("ferme la", "harassment", 0.9, "fr"),
        ("halt den mund", "harassment", 0.9, "de"),

        // --- self-harm-keywords: distress indicators (route to human/help) ---
        ("hopeless", "self-harm-keywords", 0.8, "en"),
        ("want to disappear", "self-harm-keywords", 1.0, "en"),
        ("end it all", "self-harm-keywords", 1.2, "en"),
        ("no quiero seguir", "self-harm-keywords", 1.0, "es"),
        ("plus envie de vivre", "self-harm-keywords", 1.0, "fr"),
    ];

    let entries = raw
        .iter()
        .map(|(t, c, w, l)| Entry {
            term: t.to_string(),
            category: c.to_string(),
            weight: *w,
            lang: Some(l.to_string()),
        })
        .collect();

    Lexicon::new("banlex-default", entries)
}
