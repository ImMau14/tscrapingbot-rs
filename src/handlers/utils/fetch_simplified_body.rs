use html_escape::encode_text;
use kuchiki::NodeRef;
use kuchiki::traits::*;
use reqwest;

pub async fn fetch_simplified_body(url: &str) -> Result<String, String> {
    // Map reqwest errors to string descriptions
    let raw = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let document = kuchiki::parse_html().one(raw);

    let root: NodeRef = match document.select_first("body") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document.clone(),
    };

    fn walk(node: &NodeRef, out: &mut String) {
        if let Some(el) = node.as_element() {
            let tag = el.name.local.as_ref().to_ascii_lowercase();

            if tag == "script" || tag == "style" {
                return;
            }

            const KEEP_TAGS: &[&str] = &[
                "p",
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "ul",
                "ol",
                "li",
                "strong",
                "b",
                "em",
                "i",
                "br",
                "blockquote",
                "pre",
                "code",
            ];

            if KEEP_TAGS.contains(&tag.as_str()) {
                out.push('<');
                out.push_str(&tag);
                out.push('>');

                for child in node.children() {
                    walk(&child, out);
                }

                if tag != "br" {
                    out.push_str("</");
                    out.push_str(&tag);
                    out.push('>');
                }
            } else {
                for child in node.children() {
                    walk(&child, out);
                }
            }
        } else if let Some(text_rc) = node.as_text() {
            let text = text_rc.borrow();
            let s = text.trim();
            if !s.is_empty() {
                out.push_str(&encode_text(s));
                out.push(' ');
            }
        } else {
            for child in node.children() {
                walk(&child, out);
            }
        }
    }

    let mut simplified = String::with_capacity(4096);
    walk(&root, &mut simplified);

    let result = format!("<body>{}</body>", simplified.trim());

    Ok(result)
}
