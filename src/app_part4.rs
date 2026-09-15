#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffMode {
    Off,
    Unified,
    Split,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffLineKind {
    Equal,
    Delete,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DiffLine {
    kind: DiffLineKind,
    text: String,
}

#[derive(Clone, PartialEq)]
struct ReviewDraftSession {
    file: String,
    round: u32,
    snapshot: Option<String>,
    snapshot_label: String,
    body: String,
    append: bool,
    targets: Vec<ReviewTarget>,
}

#[derive(Clone)]
struct BlockSelection {
    start: usize,
    end: usize,
    quoted: String,
}

fn line_number_at(text: &str, offset: usize) -> u32 {
    let offset = offset.min(text.len());
    let safe = text
        .char_indices()
        .take_while(|(index, _)| *index < offset)
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    (text[..safe].chars().filter(|&ch| ch == '\n').count() + 1) as u32
}

#[cfg(test)]
fn review_target(block_id: u32, kind: BlockKind, quoted: &str, whole_block: bool) -> ReviewTarget {
    let quoted = quoted.to_string();
    let end = quoted.len();
    ReviewTarget {
        block_id,
        kind,
        start: 0,
        end,
        start_line: 1,
        end_line: line_number_at(&quoted, end.saturating_sub(1)),
        quoted,
        whole_block,
    }
}

fn selection_target(block: &Block, quoted: String, start: usize, end: usize) -> ReviewTarget {
    let whole_block = quoted == block.text;
    let start = start.min(block.text.len());
    let end = end.min(block.text.len()).max(start);
    ReviewTarget {
        block_id: block.id,
        kind: block.kind,
        quoted,
        whole_block,
        start,
        end,
        start_line: line_number_at(&block.text, start),
        end_line: line_number_at(&block.text, end.saturating_sub(1).max(start)),
    }
}

fn target_range_label(target: &ReviewTarget) -> String {
    let scope = target_scope_label(target);
    if target.whole_block {
        scope.to_string()
    } else if target.start_line == target.end_line {
        format!("{scope} · L{}", target.start_line)
    } else {
        format!("{scope} · L{}–L{}", target.start_line, target.end_line)
    }
}

fn comment_is_resolved(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line == "状態: 解決済み" || line.eq_ignore_ascii_case("[resolved]")
    })
}

fn with_resolved_marker(text: &str, resolved: bool) -> String {
    let rest = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("状態:") && !trimmed.eq_ignore_ascii_case("[resolved]")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if resolved {
        if rest.is_empty() {
            "状態: 解決済み".to_string()
        } else {
            format!("状態: 解決済み\n{rest}")
        }
    } else {
        rest
    }
}

fn set_comment_resolved(block: &mut Block, resolved: bool) {
    block.checked = resolved;
    block.text = with_resolved_marker(&block.text, resolved);
}

fn toggle_comment_resolved(document: &mut DocumentState, id: u32) -> bool {
    if let Some(block) = document
        .blocks
        .iter_mut()
        .find(|block| block.id == id && block.kind == BlockKind::ReviewComment)
    {
        let next = !block.checked;
        set_comment_resolved(block, next);
        next
    } else {
        false
    }
}

fn comment_target_ids(text: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("対象:") else {
            continue;
        };
        let rest = rest.trim();
        if let Some(rest) = rest.strip_prefix('#') {
            let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            if let Ok(id) = digits.parse() {
                ids.push(id);
            }
        }
    }
    ids
}

fn infer_comment_anchor(doc: &DocumentState, comment: &Block) -> Option<u32> {
    if let Some(id) = comment_target_ids(&comment.text).into_iter().next() {
        return Some(id);
    }
    let index = doc.blocks.iter().position(|block| block.id == comment.id)?;
    doc.blocks[..index]
        .iter()
        .rev()
        .find(|block| {
            block.parent == comment.parent
                && block.column == comment.column
                && block.kind != BlockKind::ReviewComment
        })
        .map(|block| block.id)
}

fn comments_on_block(doc: &DocumentState, block_id: u32) -> Vec<Block> {
    doc.blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment && block.id != block_id)
        .filter(|block| infer_comment_anchor(doc, block) == Some(block_id))
        .cloned()
        .collect()
}

fn unresolved_comments(doc: &DocumentState) -> Vec<Block> {
    doc.blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment && !block.checked)
        .cloned()
        .collect()
}

fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let n = old_lines.len();
    let m = new_lines.len();
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            dp[i + 1][j + 1] = if old_lines[i] == new_lines[j] {
                dp[i][j] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut rev = Vec::new();
    let mut i = n;
    let mut j = m;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            rev.push(DiffLine {
                kind: DiffLineKind::Equal,
                text: old_lines[i - 1].to_string(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            rev.push(DiffLine {
                kind: DiffLineKind::Insert,
                text: new_lines[j - 1].to_string(),
            });
            j -= 1;
        } else if i > 0 {
            rev.push(DiffLine {
                kind: DiffLineKind::Delete,
                text: old_lines[i - 1].to_string(),
            });
            i -= 1;
        } else {
            break;
        }
    }
    rev.reverse();
    rev
}

fn diff_counts(lines: &[DiffLine]) -> (usize, usize) {
    let deletes = lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Delete)
        .count();
    let inserts = lines
        .iter()
        .filter(|line| line.kind == DiffLineKind::Insert)
        .count();
    (deletes, inserts)
}

fn unified_diff_text(lines: &[DiffLine]) -> String {
    let mut output = String::from("```diff\n");
    for line in lines {
        let mark = match line.kind {
            DiffLineKind::Equal => ' ',
            DiffLineKind::Delete => '-',
            DiffLineKind::Insert => '+',
        };
        output.push(mark);
        output.push_str(&line.text);
        output.push('\n');
    }
    output.push_str("```\n");
    output
}

fn review_session_key(file_name: &str) -> String {
    format!("md-block-editor.review.{file_name}")
}

fn serialize_review_session(session: &ReviewDraftSession) -> String {
    let mut parts = vec![
        "mdb-review-1".to_string(),
        session.file.clone(),
        session.round.to_string(),
        session.snapshot_label.clone(),
        if session.append { "1" } else { "0" }.to_string(),
        session.body.clone(),
        session.targets.len().to_string(),
    ];
    for target in &session.targets {
        parts.push(format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            target.block_id,
            target.kind.label(),
            if target.whole_block { 1 } else { 0 },
            target.start,
            target.end,
            target.start_line,
            target.end_line,
            target.quoted
        ));
    }
    parts.push(session.snapshot.clone().unwrap_or_default());
    parts.join("\u{1e}")
}

fn parse_review_session(raw: &str) -> Option<ReviewDraftSession> {
    let mut parts = raw.split('\u{1e}');
    if parts.next()? != "mdb-review-1" {
        return None;
    }
    let file = parts.next()?.to_string();
    let round = parts.next()?.parse().ok()?;
    let snapshot_label = parts.next()?.to_string();
    let append = parts.next()? == "1";
    let body = parts.next()?.to_string();
    let target_count: usize = parts.next()?.parse().ok()?;
    let mut targets = Vec::new();
    for _ in 0..target_count {
        let packed = parts.next()?;
        let mut fields = packed.split('\u{1f}');
        let block_id = fields.next()?.parse().ok()?;
        let kind = BlockKind::from_label(fields.next()?);
        let whole_block = fields.next()? == "1";
        let start = fields.next()?.parse().ok()?;
        let end = fields.next()?.parse().ok()?;
        let start_line = fields.next()?.parse().ok()?;
        let end_line = fields.next()?.parse().ok()?;
        let quoted = fields.next().unwrap_or("").to_string();
        targets.push(ReviewTarget {
            block_id,
            kind,
            quoted,
            whole_block,
            start,
            end,
            start_line,
            end_line,
        });
    }
    let snapshot_text = parts.next().unwrap_or("");
    let snapshot = if snapshot_text.is_empty() {
        None
    } else {
        Some(snapshot_text.to_string())
    };
    Some(ReviewDraftSession {
        file,
        round,
        snapshot,
        snapshot_label,
        body,
        append,
        targets,
    })
}

fn current_review_session(document: Signal<DocumentState>, review: ReviewUi) -> ReviewDraftSession {
    ReviewDraftSession {
        file: document().file_name.clone(),
        round: (review.round)(),
        snapshot: (review.snapshot)(),
        snapshot_label: (review.snapshot_label)(),
        body: (review.body)(),
        append: (review.append)(),
        targets: (review.targets)(),
    }
}

fn persist_review_now(document: Signal<DocumentState>, review: ReviewUi) {
    persist_review_session_browser(&current_review_session(document, review));
}

fn remember_snapshot(
    document: Signal<DocumentState>,
    mut review: ReviewUi,
    label: String,
) {
    review.snapshot.set(Some(serialize_document(&document())));
    review.snapshot_label.set(label);
    persist_review_now(document, review);
}

fn apply_opened_document(
    mut document: Signal<DocumentState>,
    mut review: ReviewUi,
    mut notice: Signal<String>,
    mut slash_for: Signal<Option<u32>>,
    parsed: DocumentState,
    for_review: bool,
) {
    let name = parsed.file_name.clone();
    if let Some(saved) = load_review_session_browser(&name) {
        review.targets.set(saved.targets);
        review.body.set(saved.body);
        review.append.set(saved.append);
        review.round.set(saved.round);
        review.snapshot.set(saved.snapshot);
        review.snapshot_label.set(saved.snapshot_label);
        notice.set(format!(
            "{name} を開き、このブラウザに残していた下書きを戻しました"
        ));
    } else {
        review.targets.set(Vec::new());
        review.body.set(String::new());
        review.append.set(false);
        review.round.set(0);
        review.snapshot.set(Some(serialize_document(&parsed)));
        review.snapshot_label.set("開いたときの文書".to_string());
        notice.set(if for_review {
            format!("{name} をレビュー用に開きました。対象を選んで指摘できます")
        } else {
            format!("{name} を開きました。対象を選んでレビューできます")
        });
    }
    document.set(parsed);
    slash_for.set(None);
    review.error.set(None);
    review.copied.set(None);
    if for_review {
        review.open.set(true);
    }
    persist_review_now(document, review);
}

fn finish_review_error(
    targets: &[ReviewTarget],
    body: &str,
    unresolved: &[Block],
) -> Option<String> {
    if targets.is_empty() && unresolved.is_empty() {
        Some("未解決の指摘がありません。本文を選ぶか、指摘ブロックを残してください。".to_string())
    } else if !targets.is_empty() && body.trim().is_empty() {
        Some("指摘が空です。書いてから終えるか、「下書きを生成」してください。".to_string())
    } else {
        None
    }
}

fn build_finish_prompt(
    doc: &DocumentState,
    targets: &[ReviewTarget],
    draft: &str,
    unresolved: &[Block],
    snapshot: Option<&str>,
    round: u32,
) -> String {
    let mut output = String::from(
        "以下は Markdown ブロックエディタのレビュー結果です。未解決の指摘を直してください。\n\n",
    );
    output.push_str(&format!(
        "# 文書\n- ファイル: {}\n- タイトル: {}\n- ラウンド: {}\n\n",
        doc.file_name, doc.title, round
    ));
    if let Some(old) = snapshot {
        let current = serialize_document(doc);
        let lines = line_diff(old, &current);
        let (deletes, inserts) = diff_counts(&lines);
        if deletes + inserts > 0 {
            output.push_str(&format!(
                "# 前回からの差分\n- 削除 {deletes} 行 / 追加 {inserts} 行\n\n"
            ));
            output.push_str(&unified_diff_text(&lines));
            output.push('\n');
        }
    }
    if !unresolved.is_empty() {
        output.push_str("# 未解決の指摘\n");
        for (index, comment) in unresolved.iter().enumerate() {
            let (quotes, body) = review_comment_parts(&comment.text);
            output.push_str(&format!(
                "## {}. 指摘ブロック #{}\n",
                index + 1,
                comment.id
            ));
            for (label, quote) in quotes {
                output.push_str(&format!("対象: {label}\n"));
                for line in quote.lines() {
                    output.push_str(&format!("> {line}\n"));
                }
            }
            if !body.is_empty() {
                output.push_str(&format!("{body}\n\n"));
            }
        }
    }
    if !targets.is_empty() && !draft.trim().is_empty() {
        output.push_str("# 今回の下書き\n");
        for target in targets {
            output.push_str(&format!(
                "## #{} {}（{}）\n",
                target.block_id,
                target.kind.label(),
                target_range_label(target)
            ));
            for line in target.quoted.lines() {
                output.push_str(&format!("> {line}\n"));
            }
            output.push('\n');
        }
        output.push_str(draft.trim());
        output.push_str("\n\n");
    }
    output.push_str(
        "# お願い\n1. 引用した箇所だけを直す\n2. 指摘への対応を短く説明する\n3. 変更後の Markdown を返す\n4. 直した指摘は解決済みとして報告する\n",
    );
    output
}

fn build_comments_json(
    doc: &DocumentState,
    targets: &[ReviewTarget],
    draft: &str,
    round: u32,
) -> String {
    let comments = doc
        .blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment)
        .cloned()
        .collect::<Vec<_>>();
    let mut output = String::from("{\n");
    output.push_str("  \"source\": \"md-block-editor\",\n");
    output.push_str(&format!("  \"file\": {},\n", json_escape(&doc.file_name)));
    output.push_str(&format!("  \"title\": {},\n", json_escape(&doc.title)));
    output.push_str(&format!("  \"round\": {round},\n"));
    output.push_str("  \"comments\": [\n");
    for (index, comment) in comments.iter().enumerate() {
        let (quotes, body) = review_comment_parts(&comment.text);
        let anchor = infer_comment_anchor(doc, comment);
        output.push_str("    {\n");
        output.push_str(&format!("      \"id\": {},\n", comment.id));
        output.push_str(&format!(
            "      \"anchor_block_id\": {},\n",
            anchor
                .map(|id| id.to_string())
                .unwrap_or_else(|| "null".to_string())
        ));
        output.push_str(&format!(
            "      \"resolved\": {},\n",
            if comment.checked { "true" } else { "false" }
        ));
        output.push_str("      \"quotes\": [\n");
        for (quote_index, (label, quote)) in quotes.iter().enumerate() {
            output.push_str("        {\n");
            output.push_str(&format!("          \"label\": {},\n", json_escape(label)));
            output.push_str(&format!("          \"quoted\": {}\n", json_escape(quote)));
            output.push_str("        }");
            if quote_index + 1 != quotes.len() {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("      ],\n");
        output.push_str(&format!("      \"body\": {}\n", json_escape(&body)));
        output.push_str("    }");
        if index + 1 != comments.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("  ],\n");
    output.push_str("  \"draft\": {\n");
    output.push_str("    \"targets\": [\n");
    for (index, target) in targets.iter().enumerate() {
        output.push_str("      {\n");
        output.push_str(&format!("        \"block_id\": {},\n", target.block_id));
        output.push_str(&format!(
            "        \"kind\": {},\n",
            json_escape(target.kind.label())
        ));
        output.push_str(&format!(
            "        \"quoted\": {},\n",
            json_escape(&target.quoted)
        ));
        output.push_str(&format!(
            "        \"whole_block\": {},\n",
            if target.whole_block { "true" } else { "false" }
        ));
        output.push_str(&format!("        \"start\": {},\n", target.start));
        output.push_str(&format!("        \"end\": {},\n", target.end));
        output.push_str(&format!("        \"start_line\": {},\n", target.start_line));
        output.push_str(&format!("        \"end_line\": {}\n", target.end_line));
        output.push_str("      }");
        if index + 1 != targets.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("    ],\n");
    output.push_str(&format!("    \"body\": {}\n", json_escape(draft.trim())));
    output.push_str("  }\n}\n");
    output
}

fn restore_saved_review(mut review: ReviewUi) {
    if let Some(saved) = load_review_session_browser("design-review.md") {
        review.targets.set(saved.targets);
        review.body.set(saved.body);
        review.append.set(saved.append);
        review.round.set(saved.round);
        review.snapshot.set(saved.snapshot);
        review.snapshot_label.set(saved.snapshot_label);
    } else {
        review.snapshot.set(Some(serialize_document(&DocumentState::starter())));
        review.snapshot_label.set("開いたときの文書".to_string());
    }
}

fn render_inline_comment_threads(
    block_id: u32,
    mut document: Signal<DocumentState>,
    mut review: ReviewUi,
    mut notice: Signal<String>,
) -> Element {
    let threads = comments_on_block(&document(), block_id);
    if threads.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "inline-threads",
            for comment in threads {
                {
                    let id = comment.id;
                    let resolved = comment.checked;
                    let (quotes, body) = review_comment_parts(&comment.text);
                    let quote = quotes
                        .first()
                        .map(|(_, quote)| quote.clone())
                        .unwrap_or_default();
                    let class = if resolved {
                        "inline-thread resolved"
                    } else {
                        "inline-thread"
                    };
                    rsx! {
                        article { class: "{class}",
                            div { class: "inline-thread-head",
                                span {
                                    if resolved { "指摘 · 解決済み" } else { "指摘 · 未解決" }
                                }
                                span { class: "inline-thread-id", "#{id}" }
                            }
                            if !quote.is_empty() {
                                blockquote { "{quote}" }
                            }
                            p { "{body}" }
                            div { class: "inline-thread-actions",
                                button {
                                    class: "mini-button",
                                    title: "この指摘をレビューパネルで開く",
                                    onclick: move |_| {
                                        review.open.set(true);
                                        visit_block_anchor(&format!("block-{id}"));
                                        notice.set(format!("指摘 #{id} を開きました"));
                                    },
                                    "開く"
                                }
                                button {
                                    class: "mini-button",
                                    title: if resolved { "未解決に戻す" } else { "解決する" },
                                    onclick: move |_| {
                                        let resolved_now = document.with_mut(|doc| {
                                            toggle_comment_resolved(doc, id)
                                        });
                                        persist_review_now(document, review);
                                        notice.set(if resolved_now {
                                            format!("指摘 #{id} を解決済みにしました")
                                        } else {
                                            format!("指摘 #{id} を未解決に戻しました")
                                        });
                                    },
                                    if resolved { "未解決へ" } else { "解決" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_diff_section(document: Signal<DocumentState>, mut review: ReviewUi, mut notice: Signal<String>) -> Element {
    let mode = (review.diff_mode)();
    let snapshot = (review.snapshot)();
    let label = (review.snapshot_label)();
    let current = serialize_document(&document());
    let lines = snapshot
        .as_ref()
        .map(|old| line_diff(old, &current))
        .unwrap_or_default();
    let (deletes, inserts) = diff_counts(&lines);
    let has_changes = deletes + inserts > 0;
    rsx! {
        section { class: "review-diff",
            div { class: "review-diff-toolbar",
                span { class: "review-diff-label",
                    if snapshot.is_some() {
                        if has_changes {
                            {format!("{label} との差分 · 削除 {deletes} / 追加 {inserts}")}
                        } else {
                            {format!("{label} から変更はありません")}
                        }
                    } else {
                        "まだ基準がありません。ファイルを開くか、レビューを終えると差分が出ます。"
                    }
                }
                div { class: "review-diff-modes",
                    button {
                        class: if mode == DiffMode::Off { "mini-button active-diff" } else { "mini-button" },
                        onclick: move |_| review.diff_mode.set(DiffMode::Off),
                        "非表示"
                    }
                    button {
                        class: if mode == DiffMode::Unified { "mini-button active-diff" } else { "mini-button" },
                        onclick: move |_| review.diff_mode.set(DiffMode::Unified),
                        "統合"
                    }
                    button {
                        class: if mode == DiffMode::Split { "mini-button active-diff" } else { "mini-button" },
                        onclick: move |_| review.diff_mode.set(DiffMode::Split),
                        "分割"
                    }
                    button {
                        class: "mini-button",
                        title: "今の文書を次の差分の基準にする",
                        onclick: move |_| {
                            remember_snapshot(document, review, "この版を基準".to_string());
                            notice.set("今の文書を差分の基準にしました".to_string());
                        },
                        "基準にする"
                    }
                }
            }
            if mode != DiffMode::Off && snapshot.is_some() && has_changes {
                if mode == DiffMode::Unified {
                    pre { class: "diff-unified",
                        for line in lines.iter() {
                            {
                                let class = match line.kind {
                                    DiffLineKind::Equal => "diff-line",
                                    DiffLineKind::Delete => "diff-line del",
                                    DiffLineKind::Insert => "diff-line add",
                                };
                                let mark = match line.kind {
                                    DiffLineKind::Equal => " ",
                                    DiffLineKind::Delete => "-",
                                    DiffLineKind::Insert => "+",
                                };
                                let text = line.text.clone();
                                rsx! { div { class: "{class}", "{mark}{text}" } }
                            }
                        }
                    }
                } else {
                    div { class: "diff-split",
                        div { class: "diff-pane",
                            span { class: "diff-pane-label", "前回" }
                            for line in lines.iter() {
                                if line.kind != DiffLineKind::Insert {
                                    {
                                        let class = if line.kind == DiffLineKind::Delete {
                                            "diff-line del"
                                        } else {
                                            "diff-line"
                                        };
                                        let text = line.text.clone();
                                        rsx! { div { class: "{class}", "{text}" } }
                                    }
                                }
                            }
                        }
                        div { class: "diff-pane",
                            span { class: "diff-pane-label", "今" }
                            for line in lines.iter() {
                                if line.kind != DiffLineKind::Delete {
                                    {
                                        let class = if line.kind == DiffLineKind::Insert {
                                            "diff-line add"
                                        } else {
                                            "diff-line"
                                        };
                                        let text = line.text.clone();
                                        rsx! { div { class: "{class}", "{text}" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_existing_comment_cards(
    mut document: Signal<DocumentState>,
    mut review: ReviewUi,
    mut notice: Signal<String>,
    comments: Vec<Block>,
) -> Element {
    rsx! {
        div { class: "review-existing",
            div { class: "review-existing-head",
                span {
                    {format!(
                        "文書内の指摘 {} 件（未解決 {}）",
                        comments.len(),
                        comments.iter().filter(|comment| !comment.checked).count()
                    )}
                }
                button {
                    class: "mini-button",
                    onclick: move |_| {
                        let doc = document();
                        let unresolved = unresolved_comments(&doc);
                        if unresolved.is_empty() {
                            review.error.set(Some("まとめてコピーする未解決の指摘がありません。".to_string()));
                            return;
                        }
                        let payload = build_existing_comments_prompt(&doc, &unresolved);
                        spawn(async move {
                            match copy_to_clipboard(&payload).await {
                                Ok(()) => {
                                    review.error.set(None);
                                    review.copied.set(Some("指摘まとめ".to_string()));
                                    remember_snapshot(
                                        document,
                                        review,
                                        "直前の LLM コピー".to_string(),
                                    );
                                    notice.set("未解決の指摘をまとめてコピーしました".to_string());
                                }
                                Err(message) => review.error.set(Some(message)),
                            }
                        });
                    },
                    "未解決をまとめてコピー"
                }
            }
            for comment in comments {
                {
                    let id = comment.id;
                    let resolved = comment.checked;
                    let (quotes, body) = review_comment_parts(&comment.text);
                    let class = if resolved {
                        "review-thread-card resolved"
                    } else {
                        "review-thread-card"
                    };
                    rsx! {
                        article { class: "{class}",
                            div { class: "review-thread-meta",
                                span {
                                    if resolved { "解決済み" } else { "未解決" }
                                    {format!(" · #{id}")}
                                }
                                button {
                                    class: "mini-button",
                                    onclick: move |_| {
                                        let resolved_now = document.with_mut(|doc| {
                                            toggle_comment_resolved(doc, id)
                                        });
                                        persist_review_now(document, review);
                                        notice.set(if resolved_now {
                                            format!("指摘 #{id} を解決済みにしました")
                                        } else {
                                            format!("指摘 #{id} を未解決に戻しました")
                                        });
                                    },
                                    if resolved { "未解決へ" } else { "解決" }
                                }
                            }
                            for (label, quote) in quotes {
                                div { class: "review-quote-preview",
                                    span { class: "review-quote-label", "{label}" }
                                    blockquote { "{quote}" }
                                }
                            }
                            p { class: "review-existing-item", "{body}" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn persist_review_session_browser(session: &ReviewDraftSession) {
    persist_session(
        &review_session_key(&session.file),
        &serialize_review_session(session),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_review_session_browser(_session: &ReviewDraftSession) {}

#[cfg(target_arch = "wasm32")]
fn load_review_session_browser(file_name: &str) -> Option<ReviewDraftSession> {
    let raw = load_session(&review_session_key(file_name));
    if raw.trim().is_empty() {
        None
    } else {
        parse_review_session(&raw)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_review_session_browser(_file_name: &str) -> Option<ReviewDraftSession> {
    None
}

#[cfg(test)]
mod review_loop_tests {
    use super::*;

    #[test]
    fn line_diff_marks_replacements() {
        let lines = line_diff("a\nb\nc\n", "a\nx\nc\n");
        assert_eq!(
            lines
                .iter()
                .map(|line| (line.kind, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (DiffLineKind::Equal, "a"),
                (DiffLineKind::Delete, "b"),
                (DiffLineKind::Insert, "x"),
                (DiffLineKind::Equal, "c"),
            ]
        );
        assert_eq!(diff_counts(&lines), (1, 1));
    }

    #[test]
    fn session_round_trips_draft_and_snapshot() {
        let session = ReviewDraftSession {
            file: "design-review.md".to_string(),
            round: 2,
            snapshot: Some("# 旧\n".to_string()),
            snapshot_label: "ラウンド 1（レビュー完了）".to_string(),
            body: "モバイルも同じ導線で。".to_string(),
            append: true,
            targets: vec![review_target(
                2,
                BlockKind::Paragraph,
                "初回体験を短くする",
                false,
            )],
        };
        let restored = parse_review_session(&serialize_review_session(&session)).unwrap();
        assert_eq!(restored.round, 2);
        assert_eq!(review_session_key("design-review.md"), "md-block-editor.review.design-review.md");
        assert_eq!(restored.body, "モバイルも同じ導線で。");
        assert_eq!(restored.targets[0].block_id, 2);
        assert_eq!(restored.snapshot.as_deref(), Some("# 旧\n"));
        assert!(restored.append);
    }

    #[test]
    fn finish_prompt_includes_unresolved_and_diff() {
        let mut document = DocumentState::starter();
        set_comment_resolved(
            document
                .blocks
                .iter_mut()
                .find(|block| block.id == 18)
                .unwrap(),
            true,
        );
        let old = "# 旧タイトル\n";
        let prompt = build_finish_prompt(
            &document,
            &[],
            "",
            &unresolved_comments(&document),
            Some(old),
            1,
        );
        assert!(prompt.contains("指摘ブロック #3"));
        assert!(prompt.contains("モバイルでの招待フロー"));
        assert!(!prompt.contains("指摘ブロック #18"));
        assert!(prompt.contains("前回からの差分"));
        assert!(prompt.contains("-# 旧タイトル"));
        assert!(prompt.contains("変更後の Markdown を返す"));
    }

    #[test]
    fn comments_json_lists_threads_and_draft_range() {
        let document = DocumentState::starter();
        let targets = vec![selection_target(
            document.blocks.iter().find(|block| block.id == 2).unwrap(),
            "初回体験を短くし".to_string(),
            0,
            "初回体験を短くし".len(),
        )];
        let json = build_comments_json(&document, &targets, "導線を揃える", 3);
        assert!(json.contains("\"source\": \"md-block-editor\""));
        assert!(json.contains("\"round\": 3"));
        assert!(json.contains("\"resolved\": false"));
        assert!(json.contains("\"start_line\": 1"));
        assert!(json.contains("導線を揃える"));
        assert!(json.contains("\"comments\": ["));
    }

    #[test]
    fn resolved_marker_round_trips_in_markdown() {
        let mut document = DocumentState::starter();
        toggle_comment_resolved(&mut document, 3);
        let comment = document
            .blocks
            .iter()
            .find(|block| block.id == 3)
            .unwrap();
        assert!(comment.checked);
        assert!(comment.text.contains("状態: 解決済み"));
        let markdown = serialize_markdown(&document);
        let restored = parse_document("design-review.md", SourceType::Markdown, &markdown);
        let restored_comment = restored
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::ReviewComment && block.text.contains("モバイル"))
            .unwrap();
        assert!(restored_comment.checked);
        assert!(comment_is_resolved(&restored_comment.text));
    }

    #[test]
    fn inline_threads_attach_to_quoted_block() {
        let document = DocumentState::starter();
        let on_intro = comments_on_block(&document, 2);
        assert_eq!(on_intro.len(), 1);
        assert_eq!(on_intro[0].id, 3);
        assert_eq!(comment_target_ids(&on_intro[0].text), vec![2]);
    }

    #[test]
    fn finish_error_requires_unresolved_or_draft() {
        let document = DocumentState::starter();
        assert!(finish_review_error(&[], "", &[]).is_some());
        assert!(finish_review_error(&[], "", &unresolved_comments(&document)).is_none());
        assert!(finish_review_error(
            &[review_target(2, BlockKind::Paragraph, "x", true)],
            "   ",
            &[]
        )
        .is_some());
    }
}
