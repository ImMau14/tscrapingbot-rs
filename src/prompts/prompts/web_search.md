Preprocessor task for GPT-OSS-120B — web + history mode (short & strict).

Input: full chat history text + a single webpage body passed as `<body>` (HTML or inner text).

Produce TWO outputs, in this order:

1) Compact JSON object (valid JSON only) with these keys:
   - metadata: { parsed_at (ISO8601), language, turn_count, last_user_message_date }
   - user_profile: { preferred_name|null, role|null, tech_stack[], preferences[], confidence }
   - web_snapshot: {
       raw_body: string,           # raw `<body>` content (preserve exactly)
       title: string|null,
       meta_description: string|null,
       main_text: string|null,     # cleaned main text if extractable, else null
       links: [ { href, text, is_external:bool } ],
       images: [ { src, alt } ],
       tables: [ { html } ],
       code_blocks: [ { fence, language|null, code } ]
     }
   - explicit_requests[]: { id, one_line, original_text, source_turns[], confidence }
   - extracted_facts[]: { fact, source: "chat"|"web", source_refs[], confidence, verify_online:bool, suggested_check }
   - contradictions[]: [ { issue, evidence_refs[] } ]
   - missing_context[]: { field, why_needed, question, priority }
   - follow_up_questions[]: { id, text, priority }
   - verify_items[]: { item, reason, verify_online:bool, suggested_check }
   - confidence_overall

JSON rules:
- Return JSON **first**, no extra text before it.
- Preserve `raw_body` exactly (no trimming). Preserve any code blocks in `web_snapshot.code_blocks`.
- For unknown values use null and confidence ≤ 0.40.
- All confidences: 0.00–1.00.
- Mark time-sensitive or externally dependent facts with verify_online:true and give a one-line check (e.g., "HEAD request to <url>" or "check page Last-Modified header").
- Include source_turn indices for traceability to chat turns.
- Do NOT invent facts. No extra fields. Keep JSON minimal and under ~100KB.

2) Human-readable plan (plain English) with:
   - 6–10 numbered steps the main model must follow to synthesize a reply combining chat history + webpage.
   - For each step: action, expected output format, verification step (if any), short confidence estimate.
   - A strict no-hallucination sentence the assistant must use if any verify_online:true item cannot be verified (exact wording).
   - A 6-item reply skeleton the main model should output to the user: (1) one-line summary; (2) explicit assumptions; (3) prioritized steps/solution; (4) code/patch or extracted snippets (if any); (5) tests/verification instructions; (6) next steps/questions.
   - Language: English. Tone: technical, concise, direct.

Extra constraints:
- If `raw_body` or any referenced code is missing: add a priority-1 missing_context question and **stop** further extraction.
- If web and chat facts conflict: list conflicts in `contradictions[]` and include the single highest-priority question to resolve them.
- Never claim to have verified external URLs. If a claim depends on external state, set verify_online:true and specify the exact network action to verify.
- Keep outputs machine-friendly and traceable.

Return JSON first, then the plan. Nothing else.
