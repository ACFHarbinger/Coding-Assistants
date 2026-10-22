//! Built-in predicate guardrails (P16 / #360). Pure string checks, no
//! network, no LLM, no shell — the cheap tier the roadmap asks for first.
//! Details never quote the screened content: they name the rule, the
//! phrase or shape that matched, and sizes.

use super::{bounded, Guardrail, GuardrailResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuardrailSpec {
    /// Trip when any phrase occurs in the content.
    DenyPhrases {
        phrases: Vec<String>,
        #[serde(default)]
        case_insensitive: bool,
    },
    /// Trip unless every phrase occurs (an output must say "DONE", say).
    RequirePhrases {
        phrases: Vec<String>,
        #[serde(default)]
        case_insensitive: bool,
    },
    /// Trip when the content is longer than `max` characters.
    MaxChars { max: usize },
    /// Trip on common credential shapes (API-key prefixes, PEM private
    /// keys). Reports only the shape and length, never the token.
    NoSecrets,
}

impl GuardrailSpec {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::DenyPhrases { .. } => "deny_phrases",
            Self::RequirePhrases { .. } => "require_phrases",
            Self::MaxChars { .. } => "max_chars",
            Self::NoSecrets => "no_secrets",
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::DenyPhrases { phrases, .. } | Self::RequirePhrases { phrases, .. } => {
                if phrases.iter().all(|phrase| phrase.trim().is_empty()) {
                    return Err(format!(
                        "{} needs at least one non-empty phrase",
                        self.kind()
                    ));
                }
                Ok(())
            }
            Self::MaxChars { max } => {
                if *max == 0 {
                    return Err("max_chars needs max > 0".into());
                }
                Ok(())
            }
            Self::NoSecrets => Ok(()),
        }
    }

    pub fn build(&self, name: &str) -> Box<dyn Guardrail> {
        Box::new(BuiltinGuardrail {
            name: name.to_string(),
            spec: self.clone(),
        })
    }
}

struct BuiltinGuardrail {
    name: String,
    spec: GuardrailSpec,
}

fn phrases_present<'a>(
    content: &str,
    phrases: &'a [String],
    case_insensitive: bool,
) -> (Vec<&'a str>, Vec<&'a str>) {
    let haystack = if case_insensitive {
        content.to_lowercase()
    } else {
        content.to_string()
    };
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for phrase in phrases
        .iter()
        .map(String::as_str)
        .filter(|p| !p.trim().is_empty())
    {
        let needle = if case_insensitive {
            phrase.to_lowercase()
        } else {
            phrase.to_string()
        };
        if haystack.contains(&needle) {
            present.push(phrase);
        } else {
            missing.push(phrase);
        }
    }
    (present, missing)
}

/// `(prefix, minimum total length)` of token shapes that are credentials
/// far more often than prose.
const SECRET_PREFIXES: &[(&str, usize)] = &[
    ("sk-", 20),
    ("sk_live_", 20),
    ("sk_test_", 20),
    ("ghp_", 30),
    ("gho_", 30),
    ("ghu_", 30),
    ("ghs_", 30),
    ("github_pat_", 40),
    ("xoxb-", 20),
    ("xoxp-", 20),
    ("xapp-", 20),
    ("AKIA", 20),
    ("ASIA", 20),
    ("AIza", 35),
    ("glpat-", 20),
    ("npm_", 36),
];

fn secret_shape(content: &str) -> Option<String> {
    if content.contains("-----BEGIN ") && content.contains("PRIVATE KEY-----") {
        return Some("PEM private key block".into());
    }
    for token in content.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\''
                    | '`'
                    | ','
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '='
                    | ':'
            )
    }) {
        for (prefix, min_len) in SECRET_PREFIXES {
            if token.starts_with(prefix)
                && token.len() >= *min_len
                && token[prefix.len()..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Some(format!(
                    "token with prefix {prefix:?} ({} chars)",
                    token.len()
                ));
            }
        }
    }
    None
}

impl Guardrail for BuiltinGuardrail {
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self, content: &str) -> GuardrailResult {
        match &self.spec {
            GuardrailSpec::DenyPhrases {
                phrases,
                case_insensitive,
            } => {
                let (present, _) = phrases_present(content, phrases, *case_insensitive);
                if present.is_empty() {
                    GuardrailResult::pass("no denied phrase present")
                } else {
                    GuardrailResult::trip(bounded(format!(
                        "denied phrase present: {}",
                        present
                            .iter()
                            .map(|p| format!("{p:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )))
                }
            }
            GuardrailSpec::RequirePhrases {
                phrases,
                case_insensitive,
            } => {
                let (_, missing) = phrases_present(content, phrases, *case_insensitive);
                if missing.is_empty() {
                    GuardrailResult::pass("every required phrase present")
                } else {
                    GuardrailResult::trip(bounded(format!(
                        "required phrase missing: {}",
                        missing
                            .iter()
                            .map(|p| format!("{p:?}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )))
                }
            }
            GuardrailSpec::MaxChars { max } => {
                let len = content.chars().count();
                if len <= *max {
                    GuardrailResult::pass(format!("{len} chars <= {max}"))
                } else {
                    GuardrailResult::trip(format!("{len} chars exceeds the {max}-char limit"))
                }
            }
            GuardrailSpec::NoSecrets => match secret_shape(content) {
                None => GuardrailResult::pass("no credential shape found"),
                Some(shape) => GuardrailResult::trip(format!("credential-shaped {shape}")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(spec: GuardrailSpec, content: &str) -> GuardrailResult {
        spec.build("t").check(content)
    }

    #[test]
    fn deny_and_require_phrases_respect_case_flag() {
        let deny = GuardrailSpec::DenyPhrases {
            phrases: vec!["force-push".into()],
            case_insensitive: false,
        };
        assert!(!check(deny.clone(), "please Force-Push").tripwire_triggered);
        assert!(check(deny, "please force-push").tripwire_triggered);

        let require = GuardrailSpec::RequirePhrases {
            phrases: vec!["done".into(), "tests pass".into()],
            case_insensitive: true,
        };
        let partial = check(require.clone(), "DONE but no suite ran");
        assert!(partial.tripwire_triggered);
        assert!(partial.detail.contains("tests pass"), "{}", partial.detail);
        assert!(!check(require, "Done. Tests Pass.").tripwire_triggered);
    }

    #[test]
    fn max_chars_counts_characters_not_bytes() {
        let spec = GuardrailSpec::MaxChars { max: 3 };
        assert!(!check(spec.clone(), "héé").tripwire_triggered);
        assert!(check(spec, "hééé").tripwire_triggered);
    }

    #[test]
    fn no_secrets_names_the_shape_but_never_the_token() {
        let token = "sk-abcdefghijklmnopqrstuvwxyz0123456789";
        let result = check(GuardrailSpec::NoSecrets, &format!("use key={token} please"));
        assert!(result.tripwire_triggered);
        assert!(!result.detail.contains("abcdefghij"), "{}", result.detail);
        assert!(result.detail.contains("sk-"));

        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----";
        assert!(check(GuardrailSpec::NoSecrets, pem).tripwire_triggered);

        assert!(!check(GuardrailSpec::NoSecrets, "sk-short and AKIA prose").tripwire_triggered);
        assert!(!check(GuardrailSpec::NoSecrets, "ordinary instruction text").tripwire_triggered);
    }

    #[test]
    fn specs_validate_their_shape_and_round_trip_json() {
        assert!(GuardrailSpec::MaxChars { max: 0 }.validate().is_err());
        assert!(GuardrailSpec::DenyPhrases {
            phrases: vec![" ".into()],
            case_insensitive: false
        }
        .validate()
        .is_err());
        let parsed: GuardrailSpec =
            serde_json::from_str(r#"{"kind":"deny_phrases","phrases":["x"]}"#).unwrap();
        assert!(parsed.validate().is_ok());
        assert_eq!(parsed.kind(), "deny_phrases");
        assert_eq!(
            serde_json::to_value(GuardrailSpec::NoSecrets).unwrap(),
            serde_json::json!({ "kind": "no_secrets" })
        );
    }
}
