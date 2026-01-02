use crate::handlers::types::MessageRow;

// Escape XML entities (safe fallback).
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

// Wrap content in a CDATA safely by splitting any "]]>" sequences.
fn wrap_cdata_safe(s: &str) -> String {
    if s.is_empty() {
        return "<![CDATA[]]>".to_string();
    }
    // Replace all occurrences of "]]>" with "]]]]><![CDATA[>"
    let safe = s.replace("]]>", "]]]]><![CDATA[>");
    format!("<![CDATA[{}]]>", safe)
}

// Format rows to XML.
pub fn format_messages_xml(rows: &[MessageRow], start_id: u64, use_cdata: bool) -> String {
    let total: usize = rows
        .iter()
        .map(|row| {
            let mut cnt = 0;
            if row
                .content
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                cnt += 1;
            }
            if row
                .ia_response
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                cnt += 1;
            }
            cnt
        })
        .sum();

    let mut emitted: usize = 0;
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    xml.push('\n');
    xml.push_str("<messages>\n");

    for row in rows {
        // User message
        if let Some(ref c) = row.content
            && let Some(text) = Some(c.trim()).filter(|s| !s.is_empty())
        {
            let id = start_id + (total - 1 - emitted) as u64;
            xml.push_str(&format!("  <message id=\"{}\" role=\"user\">", id));
            if use_cdata {
                xml.push_str(&wrap_cdata_safe(text));
            } else {
                xml.push_str(&escape_xml(text));
            }
            xml.push_str("</message>\n");
            emitted += 1;
        }

        // Assistant / ia message
        if let Some(ref a) = row.ia_response
            && let Some(text) = Some(a.trim()).filter(|s| !s.is_empty())
        {
            let id = start_id + (total - 1 - emitted) as u64;
            xml.push_str(&format!("  <message id=\"{}\" role=\"assistant\">", id));
            if use_cdata {
                xml.push_str(&wrap_cdata_safe(text));
            } else {
                xml.push_str(&escape_xml(text));
            }
            xml.push_str("</message>\n");
            emitted += 1;
        }
    }

    xml.push_str("</messages>\n");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_vs_cdata_and_ids() {
        let rows = vec![
            MessageRow {
                content: Some("Hola & <mundo>".into()),
                ia_response: Some("Bien > todo".into()),
            },
            MessageRow {
                content: Some("Mensaje con ]]> dentro: ]]>!".into()),
                ia_response: None,
            },
        ];

        let xml_escape = format_messages_xml(&rows, 1, false);
        assert!(xml_escape.contains("&amp;"));
        assert!(xml_escape.contains("&lt;"));

        let xml_cdata = format_messages_xml(&rows, 10, true);

        // comprueba que existen los tres ids
        assert!(xml_cdata.contains(r#"id="12" role="user""#));
        assert!(xml_cdata.contains(r#"id="11" role="assistant""#));
        assert!(xml_cdata.contains(r#"id="10" role="user""#));

        let p12 = xml_cdata
            .find(r#"id="12" role="user""#)
            .expect("falta id=12");
        let p11 = xml_cdata
            .find(r#"id="11" role="assistant""#)
            .expect("falta id=11");
        let p10 = xml_cdata
            .find(r#"id="10" role="user""#)
            .expect("falta id=10");
        assert!(
            p12 < p11 && p11 < p10,
            "orden de ids incorrecto: xml = {}",
            xml_cdata
        );

        assert!(
            xml_cdata.contains("<![CDATA[Mensaje con ]]]><![CDATA[> dentro: ]]>!]]>")
                || xml_cdata.contains("]]]]><![CDATA[>")
        );
    }
}
