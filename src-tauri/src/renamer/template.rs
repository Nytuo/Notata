use std::collections::HashMap;

/// One parsed piece of a rename template.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Literal(String),
    /// `{name}` or `{name:03}` — the spec zero-pads numeric values.
    Field { name: String, pad: Option<usize> },
    /// `[...]` — dropped entirely if any field inside resolves to empty.
    Optional(Vec<Token>),
}

/// Parse a template string into tokens.
///
/// Syntax:
/// - `{field}` substitutes a value, `{field:03}` zero-pads it to 3 digits
/// - `[ ... ]` is an optional group, omitted when a field inside is empty
/// - `/` separates path components
/// - `%` escapes the next character, so `%{` yields a literal brace
pub fn parse(template: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = template.chars().collect();
    let (tokens, consumed) = parse_until(&chars, 0, None)?;
    if consumed != chars.len() {
        return Err(format!("Unexpected ']' at position {}", consumed));
    }
    Ok(tokens)
}

fn parse_until(
    chars: &[char],
    mut i: usize,
    terminator: Option<char>,
) -> Result<(Vec<Token>, usize), String> {
    let mut tokens = Vec::new();
    let mut literal = String::new();

    while i < chars.len() {
        let c = chars[i];

        if Some(c) == terminator {
            if !literal.is_empty() {
                tokens.push(Token::Literal(literal));
            }
            return Ok((tokens, i));
        }

        match c {
            '%' => {
                // Escape: take the next character verbatim.
                if i + 1 < chars.len() {
                    literal.push(chars[i + 1]);
                    i += 2;
                } else {
                    literal.push('%');
                    i += 1;
                }
            }
            '{' => {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                let close = find_char(chars, i + 1, '}')
                    .ok_or_else(|| format!("Unclosed '{{' at position {}", i))?;
                let inner: String = chars[i + 1..close].iter().collect();
                tokens.push(parse_field(&inner)?);
                i = close + 1;
            }
            '[' => {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                let (inner, end) = parse_until(chars, i + 1, Some(']'))?;
                if end >= chars.len() {
                    return Err(format!("Unclosed '[' at position {}", i));
                }
                tokens.push(Token::Optional(inner));
                i = end + 1;
            }
            ']' => return Err(format!("Unmatched ']' at position {}", i)),
            _ => {
                literal.push(c);
                i += 1;
            }
        }
    }

    if terminator.is_some() {
        return Err("Unclosed '['".to_string());
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    Ok((tokens, i))
}

fn find_char(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == target)
}

fn parse_field(inner: &str) -> Result<Token, String> {
    if inner.trim().is_empty() {
        return Err("Empty field name in '{}'".to_string());
    }

    match inner.split_once(':') {
        Some((name, spec)) => {
            let pad = spec
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid pad width '{}' for field '{}'", spec, name))?;
            Ok(Token::Field {
                name: name.trim().to_lowercase(),
                pad: Some(pad),
            })
        }
        None => Ok(Token::Field {
            name: inner.trim().to_lowercase(),
            pad: None,
        }),
    }
}

/// Characters that are illegal in a path component on at least one supported
/// platform. Replaced rather than stripped so words stay separated.
const ILLEGAL: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Make a substituted value safe to sit inside a single path component.
///
/// Applied at substitution time so a value containing `/` cannot silently
/// introduce a directory level the template did not ask for.
pub fn sanitize_value(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if ILLEGAL.contains(&c) {
                '_'
            } else if c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned.trim().to_string()
}

/// Trailing dots and spaces are not addressable on Windows.
fn sanitize_component(component: &str) -> String {
    component.trim_end_matches([' ', '.']).trim().to_string()
}

fn format_value(raw: &str, pad: Option<usize>) -> String {
    match pad {
        // Padding only means something for numbers; leave other values alone.
        Some(width) => match raw.parse::<i64>() {
            Ok(n) => format!("{:0>width$}", n, width = width),
            Err(_) => raw.to_string(),
        },
        None => raw.to_string(),
    }
}

/// How a render went, beyond the text it produced.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOutcome {
    pub text: String,
    /// How many `{field}` tokens the template referenced.
    pub fields_seen: usize,
    /// How many of those actually had a value.
    pub fields_resolved: usize,
}

impl RenderOutcome {
    /// True when the template asked for fields but none of them existed.
    ///
    /// Distinguishes a genuinely empty result from one where only the
    /// template's literal punctuation survived — the case that produces
    /// names like `() ()`.
    pub fn is_unresolved(&self) -> bool {
        self.fields_seen > 0 && self.fields_resolved == 0
    }
}

struct RenderState {
    seen: usize,
    resolved: usize,
}

fn render_tokens(
    tokens: &[Token],
    values: &HashMap<String, String>,
    out: &mut String,
    state: &mut RenderState,
) -> bool {
    let mut all_present = true;

    for token in tokens {
        match token {
            Token::Literal(text) => out.push_str(text),
            Token::Field { name, pad } => {
                state.seen += 1;
                let raw = values.get(name).map(|s| s.as_str()).unwrap_or("");
                if raw.trim().is_empty() {
                    all_present = false;
                } else {
                    state.resolved += 1;
                    out.push_str(&format_value(&sanitize_value(raw), *pad));
                }
            }
            Token::Optional(inner) => {
                let mut buf = String::new();
                // Count the group's fields separately so a dropped group does
                // not inflate the resolved tally.
                let mut inner_state = RenderState {
                    seen: 0,
                    resolved: 0,
                };
                if render_tokens(inner, values, &mut buf, &mut inner_state) {
                    out.push_str(&buf);
                    state.seen += inner_state.seen;
                    state.resolved += inner_state.resolved;
                } else {
                    state.seen += inner_state.seen;
                }
            }
        }
    }

    all_present
}

/// Render a parsed template into a relative path.
///
/// Path components are sanitized individually and empty components are
/// dropped, so an optional group that collapses cannot leave `//` behind.
pub fn render_detailed(
    tokens: &[Token],
    values: &HashMap<String, String>,
) -> RenderOutcome {
    let mut raw = String::new();
    let mut state = RenderState {
        seen: 0,
        resolved: 0,
    };
    render_tokens(tokens, values, &mut raw, &mut state);

    let text = raw
        .split('/')
        .map(sanitize_component)
        .filter(|c| !c.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    RenderOutcome {
        text,
        fields_seen: state.seen,
        fields_resolved: state.resolved,
    }
}

pub fn render(tokens: &[Token], values: &HashMap<String, String>) -> String {
    render_detailed(tokens, values).text
}

/// Convenience: parse and render in one step.
pub fn render_template(
    template: &str,
    values: &HashMap<String, String>,
) -> Result<String, String> {
    let tokens = parse(template)?;
    Ok(render(&tokens, values))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn substitutes_fields_and_pads_numbers() {
        let v = values(&[("track", "7"), ("title", "Blue Monday")]);
        assert_eq!(
            render_template("{track:02} - {title}", &v).unwrap(),
            "07 - Blue Monday"
        );
    }

    #[test]
    fn padding_leaves_non_numeric_values_alone() {
        let v = values(&[("title", "Intro")]);
        assert_eq!(render_template("{title:05}", &v).unwrap(), "Intro");
    }

    #[test]
    fn optional_group_is_dropped_when_a_field_is_empty() {
        let v = values(&[("album", "Power")]);
        assert_eq!(render_template("{album}[ ({year})]", &v).unwrap(), "Power");

        let v = values(&[("album", "Power"), ("year", "1983")]);
        assert_eq!(
            render_template("{album}[ ({year})]", &v).unwrap(),
            "Power (1983)"
        );
    }

    #[test]
    fn slashes_inside_values_do_not_create_directories() {
        let v = values(&[("artist", "AC/DC"), ("title", "T.N.T")]);
        assert_eq!(
            render_template("{artist}/{title}", &v).unwrap(),
            "AC_DC/T.N.T"
        );
    }

    #[test]
    fn collapsed_components_do_not_leave_empty_path_segments() {
        let v = values(&[("artist", "Nine Inch Nails"), ("title", "Closer")]);
        assert_eq!(
            render_template("{artist}/[{album}]/{title}", &v).unwrap(),
            "Nine Inch Nails/Closer"
        );
    }

    #[test]
    fn strips_trailing_dots_and_spaces_from_components() {
        let v = values(&[("album", "Wish You Were Here.")]);
        assert_eq!(
            render_template("{album}/x", &v).unwrap(),
            "Wish You Were Here/x"
        );
    }

    #[test]
    fn escape_yields_literal_braces() {
        let v = values(&[("title", "x")]);
        assert_eq!(render_template("%{{title}%}", &v).unwrap(), "{x}");
    }

    #[test]
    fn reports_malformed_templates() {
        assert!(parse("{title").is_err());
        assert!(parse("[{title}").is_err());
        assert!(parse("{title}]").is_err());
        assert!(parse("{}").is_err());
        assert!(parse("{track:xx}").is_err());
    }
}
