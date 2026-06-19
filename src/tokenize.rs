//! Simple, dependency-free, unicode-aware tokenization and normalization.
//!
//! The goal is robust matching for a lexicon classifier, not linguistic
//! perfection. We:
//!   * lowercase using Unicode `to_lowercase`,
//!   * split on any character that is not alphanumeric (Unicode-aware),
//!   * keep each remaining run of alphanumerics as one token.
//!
//! Multi-word phrase matching is handled in the scorer by sliding a window
//! over the resulting token stream, so the tokenizer only needs to produce a
//! clean, normalized sequence of word tokens.

/// A single normalized token together with its position (token index) in the
/// original stream. The index lets callers report *where* matches occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Normalized (lowercased) token text.
    pub text: String,
    /// Zero-based position of this token in the produced stream.
    pub index: usize,
}

/// Tokenize and normalize `input` into a vector of [`Token`]s.
///
/// Word boundaries are any non-alphanumeric Unicode character. Tokens are
/// lowercased. Empty runs are dropped.
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buf = String::new();

    for ch in input.chars() {
        if ch.is_alphanumeric() {
            // Lowercase per-char (handles multi-char lowercase mappings).
            for low in ch.to_lowercase() {
                buf.push(low);
            }
        } else if !buf.is_empty() {
            tokens.push(buf.clone());
            buf.clear();
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }

    tokens
        .into_iter()
        .enumerate()
        .map(|(index, text)| Token { text, index })
        .collect()
}

/// Normalize a lexicon term into its sequence of normalized token strings.
///
/// Uses the same rules as [`tokenize`] so a phrase like `"Buy  Now!"` and the
/// term `"buy now"` normalize to the identical token sequence `["buy","now"]`.
pub fn normalize_term(term: &str) -> Vec<String> {
    tokenize(term).into_iter().map(|t| t.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_punctuation_and_lowercases() {
        let toks: Vec<String> = tokenize("Buy NOW! Click... here?")
            .into_iter()
            .map(|t| t.text)
            .collect();
        assert_eq!(toks, vec!["buy", "now", "click", "here"]);
    }

    #[test]
    fn unicode_alphanumerics_are_kept() {
        // Spanish accents and German eszett survive as part of tokens.
        let toks: Vec<String> = tokenize("¡Oferta LIMITADA! Straße")
            .into_iter()
            .map(|t| t.text)
            .collect();
        assert_eq!(toks, vec!["oferta", "limitada", "straße"]);
    }

    #[test]
    fn normalize_term_matches_tokenize() {
        assert_eq!(normalize_term("Buy  Now!"), vec!["buy", "now"]);
    }

    #[test]
    fn empty_input_yields_no_tokens() {
        assert!(tokenize("   \n\t  ").is_empty());
    }

    #[test]
    fn indices_are_sequential() {
        let toks = tokenize("a b c");
        assert_eq!(toks[0].index, 0);
        assert_eq!(toks[2].index, 2);
    }
}
