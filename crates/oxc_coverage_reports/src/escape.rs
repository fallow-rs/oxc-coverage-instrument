//! Context-specific escaping for the HTML and Cobertura emitters.

/// Escape for an HTML text node. Quotes are left alone; use [`html_attr`]
/// inside an attribute value.
#[cfg(feature = "html")]
pub fn html_text(s: &str) -> String {
    escape(s, EscapeMode::HtmlText)
}

/// Escape for an HTML attribute value, including both quote characters.
#[cfg(feature = "html")]
pub fn html_attr(s: &str) -> String {
    escape(s, EscapeMode::HtmlAttr)
}

/// Escape for an XML attribute value. Emits `&apos;` for the apostrophe where
/// the HTML attribute escape emits `&#39;`, and maps characters XML 1.0
/// forbids to U+FFFD.
pub fn xml_attr(s: &str) -> String {
    escape(s, EscapeMode::XmlAttr)
}

#[derive(Clone, Copy)]
enum EscapeMode {
    #[cfg(feature = "html")]
    HtmlText,
    #[cfg(feature = "html")]
    HtmlAttr,
    XmlAttr,
}

fn escape(s: &str, mode: EscapeMode) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if matches!(mode, EscapeMode::XmlAttr) => out.push_str("&quot;"),
            #[cfg(feature = "html")]
            '"' if matches!(mode, EscapeMode::HtmlAttr) => out.push_str("&quot;"),
            #[cfg(feature = "html")]
            '\'' if matches!(mode, EscapeMode::HtmlAttr) => out.push_str("&#39;"),
            '\'' if matches!(mode, EscapeMode::XmlAttr) => out.push_str("&apos;"),
            // XML 1.0 forbids most control characters; strict parsers
            // reject the document when they appear in coverage-derived
            // function names or paths, so map disallowed chars to U+FFFD.
            c if matches!(mode, EscapeMode::XmlAttr) && !is_xml10_char(c) => out.push('\u{FFFD}'),
            _ => out.push(c),
        }
    }
    out
}

fn is_xml10_char(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\r'
            | '\u{0020}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::xml_attr;
    #[cfg(feature = "html")]
    use super::{html_attr, html_text};

    #[cfg(feature = "html")]
    #[test]
    fn html_text_escapes_text_nodes() {
        assert_eq!(html_text("<a & b>"), "&lt;a &amp; b&gt;");
    }

    #[cfg(feature = "html")]
    #[test]
    fn html_attr_escapes_quotes_with_html_entity() {
        assert_eq!(html_attr(r#"'a" & <b>"#), "&#39;a&quot; &amp; &lt;b&gt;");
    }

    #[test]
    fn xml_attr_escapes_quotes_and_replaces_illegal_control_chars() {
        assert_eq!(xml_attr("a\u{0} ' \" & < >"), "a\u{FFFD} &apos; &quot; &amp; &lt; &gt;");
    }
}
