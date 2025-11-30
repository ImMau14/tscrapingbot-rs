// Escapes HTML entities for Telegram messages while preserving <code> blocks and valid HTML entities.

use regex::Regex;

pub fn escape_telegram_code_entities(input: &str) -> String {
    // Attrs: (?:[^"'<>]|"[^"]*"|'[^']*')*
    let code_re = Regex::new(r#"(?is)<code\b((?:[^"'<>]|"[^"]*"|'[^']*')*)>(.*?)</code>"#).unwrap();
    let tag_re = Regex::new(r#"(?s)<[A-Za-z/](?:[^"'<>]|"[^"]*"|'[^']*')*>"#).unwrap();
    let entity_re = Regex::new(r#"&(?:#x[0-9A-Fa-f]+|#\d+|[A-Za-z][A-Za-z0-9]*);"#).unwrap();

    // Placeholder helper (BEL char to reduce collisions)
    fn ph(prefix: &str, idx: usize) -> String {
        format!("\x07{}{}\x07", prefix, idx)
    }

    // 1) Extract and process all <code ...>...</code> blocks first.
    let mut code_blocks: Vec<String> = Vec::new();
    let s_after_code = code_re
        .replace_all(input, |caps: &regex::Captures| {
            let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let inner = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            // Protect entities inside the inner content with local placeholders
            let mut local_entities: Vec<String> = Vec::new();
            let inner_protected = entity_re
                .replace_all(inner, |ec: &regex::Captures| {
                    let ent = ec.get(0).unwrap().as_str().to_string();
                    let id = local_entities.len();
                    local_entities.push(ent);
                    ph("ENTC", id)
                })
                .into_owned();

            // Escape the remaining characters
            let mut inner_escaped = inner_protected.replace('&', "&amp;");
            inner_escaped = inner_escaped.replace('<', "&lt;");
            inner_escaped = inner_escaped.replace('>', "&gt;");

            // Restore local entities
            for (i, ent) in local_entities.iter().enumerate() {
                let placeholder = ph("ENTC", i);
                inner_escaped = inner_escaped.replace(&placeholder, ent);
            }

            // Recreate code tag with attributes as-is
            let code_html = format!("<code{}>{}</code>", attrs, inner_escaped);
            let id = code_blocks.len();
            code_blocks.push(code_html);
            ph("CODE", id)
        })
        .into_owned();

    // 2) Replace tags (outside code placeholders) with placeholders.
    // If a <code> tag without its closing tag exists in the entire resulting document, we leave it as text
    let has_closing_code = s_after_code.to_lowercase().contains("</code>");
    let mut tag_map: Vec<String> = Vec::new();
    let s_after_tags = tag_re
        .replace_all(&s_after_code, |caps: &regex::Captures| {
            let tag = caps.get(0).unwrap().as_str().to_string();

            // Detect if the tag is a <code ...> or </code>
            let tag_lower = tag.to_lowercase();
            let is_code_opening = tag_lower.starts_with("<code") && !tag_lower.starts_with("</");
            let is_code_closing = tag_lower.starts_with("</code");

            // If it's an opening/closing code tag and no closing tag exists in the whole doc -> don't treat it as a tag
            if (is_code_opening || is_code_closing) && !has_closing_code {
                return tag;
            }

            let id = tag_map.len();
            tag_map.push(tag);
            ph("TAG", id)
        })
        .into_owned();

    // 3) Protect global entities (outside tags) with placeholders
    let mut global_entities: Vec<String> = Vec::new();
    let s_after_entities = entity_re
        .replace_all(&s_after_tags, |caps: &regex::Captures| {
            let ent = caps.get(0).unwrap().as_str().to_string();
            let id = global_entities.len();
            global_entities.push(ent);
            ph("ENTG", id)
        })
        .into_owned();

    // 4) Escape &, <, > in the remaining text
    let mut escaped = s_after_entities.replace('&', "&amp;");
    escaped = escaped.replace('<', "&lt;");
    escaped = escaped.replace('>', "&gt;");

    // 5) Restore global entities
    for (i, ent) in global_entities.iter().enumerate() {
        let placeholder = ph("ENTG", i);
        escaped = escaped.replace(&placeholder, ent);
    }

    // 6) Restore tags (valid tags)
    for (i, tag) in tag_map.iter().enumerate() {
        let placeholder = ph("TAG", i);
        escaped = escaped.replace(&placeholder, tag);
    }

    // 7) Restore code blocks (final step)
    for (i, code_html) in code_blocks.iter().enumerate() {
        let placeholder = ph("CODE", i);
        escaped = escaped.replace(&placeholder, code_html);
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_telegram_code_entities;

    #[test]
    fn escapes_inside_code_tags() {
        let input = r#"Antes <code>let x: Option<i32> = Some(5); // <i32> / & test</code> después"#;
        let out = escape_telegram_code_entities(input);
        assert!(out.contains(
            "<code>let x: Option&lt;i32&gt; = Some(5); // &lt;i32&gt; / &amp; test</code>"
        ));
    }

    #[test]
    fn preserves_other_html() {
        let input = r#"Texto <b>bold</b> <code><a href="x">x</a> & <  ></code>"#;
        let out = escape_telegram_code_entities(input);
        assert!(out.contains("<b>bold</b>"));
        assert!(out.contains("<code>&lt;a href=\"x\"&gt;x&lt;/a&gt; &amp; &lt;  &gt;</code>"));
    }

    #[test]
    fn preserves_valid_named_and_numeric_entities_inside_code() {
        let input = r#"Antes <code>&amp; &unknown; &#65; &#x41; &</code> Después"#;
        let out = escape_telegram_code_entities(input);
        assert!(out.contains("<code>&amp; &unknown; &#65; &#x41; &amp;</code>"));
    }

    #[test]
    fn escapes_angle_brackets_outside_tags() {
        let input = r#"Comparación: 1 < 2 y 3 > 2"#;
        let out = escape_telegram_code_entities(input);
        assert_eq!(out, "Comparación: 1 &lt; 2 y 3 &gt; 2");
    }

    #[test]
    fn preserves_tag_with_greater_than_inside_attribute_quotes() {
        let input = r#"Link: <a href="http://example.com?q=1>2">click</a> & test"#;
        let out = escape_telegram_code_entities(input);
        assert!(out.starts_with("Link: <a href=\"http://example.com?q=1>2\">click</a>"));
        assert!(out.ends_with("&amp; test"));
    }

    #[test]
    fn handles_malformed_tag_without_closing_gt() {
        let input = r#"Mal formado: <div attr="x"#;
        let out = escape_telegram_code_entities(input);
        assert_eq!(out, "Mal formado: &lt;div attr=\"x");
    }

    #[test]
    fn code_tag_with_attributes_escapes_inner_but_preserves_tag_attrs() {
        let input = r#"Start <code class="rust">let s = "<test>" &amp; 1;</code> End"#;
        let out = escape_telegram_code_entities(input);
        assert!(
            out.contains(r#"Start <code class="rust">let s = "&lt;test&gt;" &amp; 1;</code> End"#)
        );
    }

    #[test]
    fn multiple_code_blocks_and_mixed_entities() {
        let input = r#"A <code>1 < 2</code> B <code>&amp; &</code> C"#;
        let out = escape_telegram_code_entities(input);
        assert!(out.contains("<code>1 &lt; 2</code>"));
        assert!(out.contains("<code>&amp; &amp;</code>"));
    }

    #[test]
    fn numeric_and_hex_entities_and_incomplete_ampersand_outside() {
        let input = r#"Nums &#65; &#x41; &badness; &incomplete"#;
        let out = escape_telegram_code_entities(input);
        assert_eq!(out, "Nums &#65; &#x41; &badness; &amp;incomplete");
    }

    #[test]
    fn stray_greater_and_less_outside_tags() {
        let input = r#"> <"#;
        let out = escape_telegram_code_entities(input);
        assert_eq!(out, "&gt; &lt;");
    }

    #[test]
    fn html_like_content_inside_code_is_escaped_fully() {
        let input = r#"<code><em>x</em></code>"#;
        let out = escape_telegram_code_entities(input);
        assert_eq!(out, "<code>&lt;em&gt;x&lt;/em&gt;</code>");
    }

    #[test]
    fn unclosed_code_tag_escapes_until_end_of_input() {
        let input = r#"Open <code>1 < 2"#;
        let out = escape_telegram_code_entities(input);
        // We expect the unclosed <code> tag to be treated as text and therefore escaped.
        assert_eq!(out, "Open &lt;code&gt;1 &lt; 2");
    }
}
