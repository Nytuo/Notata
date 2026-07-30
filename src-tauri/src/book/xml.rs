use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{NotataError, Result};

/// One element's tag name, attributes, and text content.
pub struct XmlElement {
    pub name: String,
    pub text: String,
    pub attributes: Vec<(String, String)>,
}

/// Read a flat list of every element with its text and attributes.
///
/// Text is accumulated per element rather than per event: quick-xml emits
/// entity references as separate events, so reading only `Event::Text` would
/// truncate any value containing `&`, `<`, or `>`.
pub fn read_all(xml: &str) -> Result<Vec<XmlElement>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut out = Vec::new();
    let mut buf = Vec::new();
    let mut stack: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                stack.push((name, read_attributes(&e)));
                text_buf.clear();
            }
            Ok(Event::Empty(e)) => {
                out.push(XmlElement {
                    name: local_name(e.name().as_ref()),
                    text: String::new(),
                    attributes: read_attributes(&e),
                });
            }
            Ok(Event::End(_)) => {
                if let Some((name, attributes)) = stack.pop() {
                    out.push(XmlElement {
                        name,
                        text: text_buf.trim().to_string(),
                        attributes,
                    });
                }
                text_buf.clear();
            }
            Ok(Event::Text(e)) => {
                text_buf.push_str(
                    &e.xml_content(quick_xml::XmlVersion::Implicit1_0)
                        .unwrap_or_default(),
                );
            }
            Ok(Event::GeneralRef(e)) => {
                let name = String::from_utf8_lossy(e.as_ref()).to_string();
                if let Some(ch) = resolve_entity(&name) {
                    text_buf.push(ch);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(NotataError::Custom(format!(
                    "Malformed XML at position {}: {}",
                    reader.buffer_position(),
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

/// Convenience for flat schemas: element name paired with its text.
pub fn read_elements(xml: &str) -> Result<Vec<(String, String)>> {
    Ok(read_all(xml)?
        .into_iter()
        .map(|e| (e.name, e.text))
        .collect())
}

/// Strip any namespace prefix — OPF documents use `dc:title`, `opf:role`.
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => name.to_string(),
    }
}

fn read_attributes(e: &quick_xml::events::BytesStart) -> Vec<(String, String)> {
    e.attributes()
        .filter_map(|a| a.ok())
        .map(|a| {
            (
                local_name(a.key.as_ref()),
                String::from_utf8_lossy(&a.value).to_string(),
            )
        })
        .collect()
}

fn resolve_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        other => {
            let digits = other.strip_prefix('#')?;
            let code = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

pub fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_entity_text_intact() {
        let elements = read_elements("<r><a>Tom &amp; Jerry</a></r>").unwrap();
        let a = elements.iter().find(|(n, _)| n == "a").unwrap();
        assert_eq!(a.1, "Tom & Jerry");
    }

    #[test]
    fn strips_namespace_prefixes() {
        let elements = read_elements("<p><dc:title>Dune</dc:title></p>").unwrap();
        assert!(elements.iter().any(|(n, v)| n == "title" && v == "Dune"));
    }

    #[test]
    fn exposes_attributes() {
        let all = read_all(r#"<p><item id="cover" href="c.jpg"/></p>"#).unwrap();
        let item = all.iter().find(|e| e.name == "item").unwrap();
        assert_eq!(
            item.attributes
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.as_str()),
            Some("c.jpg")
        );
    }

    #[test]
    fn reports_malformed_documents() {
        assert!(read_elements("<a><b></a>").is_err());
    }
}
