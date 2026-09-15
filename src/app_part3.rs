fn parse_document(name: &str, source: SourceType, content: &str) -> DocumentState {
    let mut doc = DocumentState {
        title: name
            .rsplit_once('.')
            .map(|(base, _)| base)
            .unwrap_or(name)
            .to_string(),
        file_name: name.to_string(),
        source,
        blocks: Vec::new(),
        next_id: 1,
    };
    if source == SourceType::Markdown {
        parse_markdown(&mut doc, content);
    } else {
        parse_html(&mut doc, content);
    }
    if doc.blocks.is_empty() {
        doc.blocks
            .push(new_block(&mut doc.next_id, BlockKind::Paragraph, content));
    }
    doc
}

fn parse_markdown(doc: &mut DocumentState, content: &str) {
    let lines: Vec<&str> = content.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_end();
        if line.trim().is_empty() {
            index += 1;
            continue;
        }
        if line.starts_with("```") {
            let kind = if line
                .trim_start_matches('`')
                .trim()
                .eq_ignore_ascii_case("mermaid")
            {
                BlockKind::Mermaid
            } else {
                BlockKind::Code
            };
            index += 1;
            let mut code = Vec::new();
            while index < lines.len() && !lines[index].starts_with("```") {
                code.push(lines[index]);
                index += 1;
            }
            doc.blocks
                .push(new_block(&mut doc.next_id, kind, &code.join("\n")));
        } else if let Some(value) = line.strip_prefix("# ") {
            if doc.blocks.is_empty() {
                doc.title = value.to_string();
            }
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::HeadingOne, value));
        } else if let Some(value) = line.strip_prefix("## ") {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::HeadingTwo, value));
        } else if let Some(value) = line.strip_prefix("### ") {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::HeadingThree, value));
        } else if let Some(value) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            let mut block = new_block(&mut doc.next_id, BlockKind::Checklist, value);
            block.checked = true;
            doc.blocks.push(block);
        } else if let Some(value) = line.strip_prefix("- [ ] ") {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Checklist, value));
        } else if let Some(value) = line.strip_prefix("- ") {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Bullet, value));
        } else if let Some(value) = numbered_item(line) {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Numbered, value));
        } else if line == "---" || line == "***" {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Divider, ""));
        } else if line.starts_with("> [!COMMENT]") {
            let mut parts = Vec::new();
            let rest = line
                .strip_prefix("> [!COMMENT]")
                .unwrap_or("")
                .trim();
            if !rest.is_empty() {
                parts.push(rest.to_string());
            }
            while index + 1 < lines.len() {
                let next = lines[index + 1];
                if let Some(body) = next.strip_prefix("> ") {
                    parts.push(body.to_string());
                    index += 1;
                } else if next.trim() == ">" {
                    parts.push(String::new());
                    index += 1;
                } else {
                    break;
                }
            }
            doc.blocks.push({
                let text = parts.join("\n");
                let mut block = new_block(
                    &mut doc.next_id,
                    BlockKind::ReviewComment,
                    &text,
                );
                block.checked = comment_is_resolved(&text);
                block
            });
        } else if let Some(value) = line.strip_prefix("> [!NOTE]") {
            let text = if value.trim().is_empty() && index + 1 < lines.len() {
                index += 1;
                lines[index]
                    .trim_start()
                    .trim_start_matches('>')
                    .trim_start()
            } else {
                value.trim()
            };
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Callout, text));
        } else if let Some(value) = line.strip_prefix("> ") {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Quote, value));
        } else if let Some(url) = markdown_image_url(line) {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Image, url));
        } else if let Some((label, url)) = markdown_link_parts(line) {
            doc.blocks.push(new_block(
                &mut doc.next_id,
                BlockKind::Link,
                &format!("{label} | {url}"),
            ));
        } else if line.starts_with("<video") {
            let url = html_attribute(line, "src").unwrap_or_default();
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Video, &url));
        } else if line.starts_with("<details") {
            let mut raw = line.to_string();
            while !raw.contains("</details>") && index + 1 < lines.len() {
                index += 1;
                raw.push('\n');
                raw.push_str(lines[index]);
            }
            let summary = between(&raw, "<summary>", "</summary>").unwrap_or("Details");
            let body = raw
                .split("</summary>")
                .nth(1)
                .unwrap_or("")
                .replace("</details>", "")
                .trim()
                .to_string();
            let toggle_id = doc.next_id;
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Toggle, summary));
            if body.contains("<section") || body.contains("<aside") {
                let start = doc.blocks.len();
                parse_html(doc, &body);
                for child in &mut doc.blocks[start..] {
                    if child.parent.is_none() {
                        child.parent = Some(toggle_id);
                    }
                }
            } else if !body.is_empty() {
                parse_markdown_children(doc, &body, toggle_id, 0);
            }
        } else if line.starts_with("<section") || line.starts_with("<aside") {
            let closing = if line.starts_with("<section") {
                "</section>"
            } else {
                "</aside>"
            };
            let mut raw = line.to_string();
            while !raw.contains(closing) && index + 1 < lines.len() {
                index += 1;
                raw.push('\n');
                raw.push_str(lines[index]);
            }
            parse_html(doc, &raw);
        } else if line.contains('|') {
            let mut rows = vec![line.trim_matches('|').trim().to_string()];
            while index + 1 < lines.len() && lines[index + 1].contains('|') {
                index += 1;
                let next = lines[index].trim();
                if next
                    .chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace())
                {
                    continue;
                }
                rows.push(next.trim_matches('|').trim().to_string());
            }
            doc.blocks.push(new_block(
                &mut doc.next_id,
                BlockKind::Table,
                &rows.join("\n"),
            ));
        } else {
            doc.blocks
                .push(new_block(&mut doc.next_id, BlockKind::Paragraph, line));
        }
        index += 1;
    }
}

fn parse_markdown_children(doc: &mut DocumentState, content: &str, parent: u32, column: u8) {
    let mut temporary = DocumentState {
        title: String::new(),
        file_name: String::new(),
        source: SourceType::Markdown,
        blocks: Vec::new(),
        next_id: 1,
    };
    parse_markdown(&mut temporary, content);
    let mut id_map = Vec::new();
    for child in &temporary.blocks {
        let next = doc.next_id;
        doc.next_id += 1;
        id_map.push((child.id, next));
    }
    for mut child in temporary.blocks {
        let old_parent = child.parent;
        child.id = id_map
            .iter()
            .find(|(old, _)| *old == child.id)
            .map(|(_, next)| *next)
            .unwrap_or(child.id);
        child.parent = old_parent
            .and_then(|old| id_map.iter().find(|(existing, _)| *existing == old))
            .map(|(_, mapped)| *mapped)
            .or(Some(parent));
        if old_parent.is_none() {
            child.column = column;
        }
        doc.blocks.push(child);
    }
}

fn parse_html(doc: &mut DocumentState, content: &str) {
    parse_html_fragment(doc, content, 0);
}

fn parse_html_fragment(doc: &mut DocumentState, source: &str, list_depth: usize) {
    parse_html_fragment_in(doc, source, list_depth, None, 0);
}

fn parse_html_fragment_in(
    doc: &mut DocumentState,
    source: &str,
    list_depth: usize,
    parent: Option<u32>,
    column: u8,
) {
    let mut cursor = 0;
    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let plain = html_unescape(source[cursor..start].trim());
        if !plain.is_empty() {
            push_html_block(doc, BlockKind::Paragraph, &plain, false);
        }
        let Some(tag) = html_tag_at(source, start) else {
            break;
        };
        cursor = tag.end;
        if tag.closing {
            continue;
        }

        if matches!(tag.name.as_str(), "img" | "hr" | "br" | "input") || tag.self_closing {
            match tag.name.as_str() {
                "img" => push_html_block(
                    doc,
                    BlockKind::Image,
                    &html_attribute(&tag.attributes, "src").unwrap_or_default(),
                    false,
                ),
                "hr" => push_html_block(doc, BlockKind::Divider, "", false),
                _ => {}
            }
            continue;
        }

        let Some((inner_end, next)) = matching_html_close(source, &tag.name, cursor) else {
            continue;
        };
        let inner = &source[cursor..inner_end];
        cursor = next;

        match tag.name.as_str() {
            "section" if tag.attributes.contains("blockpad-columns") => {
                let title = element_inner(inner, "p")
                    .map(|value| strip_tags(&value))
                    .unwrap_or_else(|| "比較カラム".to_string());
                let columns_id =
                    push_html_block_in(doc, BlockKind::Columns, &title, false, parent, column);
                for (column, fragment) in column_fragments(inner) {
                    parse_html_children(doc, &fragment, columns_id, column);
                }
            }
            "section" if tag.attributes.contains("blockpad-group") => {
                let title = element_inner(inner, "p")
                    .map(|value| strip_tags(&value))
                    .unwrap_or_else(|| "グループ".to_string());
                let group_id =
                    push_html_block_in(doc, BlockKind::Group, &title, false, parent, column);
                parse_html_children(doc, &remove_first_element(inner, "p"), group_id, 0);
            }
            "html" | "head" | "body" | "main" | "article" | "section" | "header" | "footer"
            | "div" => parse_html_fragment_in(doc, inner, list_depth, parent, column),
            "h1" => {
                let text = strip_tags(inner);
                if doc.blocks.is_empty() {
                    doc.title = text.clone();
                }
                push_html_block(doc, BlockKind::HeadingOne, &text, false);
            }
            "h2" => push_html_block(doc, BlockKind::HeadingTwo, &strip_tags(inner), false),
            "h3" => push_html_block(doc, BlockKind::HeadingThree, &strip_tags(inner), false),
            "h4" | "h5" | "h6" => push_html_block(
                doc,
                BlockKind::Semantic,
                &format!("{} | {}", tag.name, strip_tags(inner)),
                false,
            ),
            "p" => {
                let checked = inner.contains("checkbox") || tag.attributes.contains("checkbox");
                let only_link = if inner.trim().starts_with("<a") && inner.trim().ends_with("</a>")
                {
                    html_tag_at(inner.trim(), 0).and_then(|link| {
                        matching_html_close(inner.trim(), "a", link.end).map(|(end, _)| {
                            (
                                strip_tags(&inner.trim()[link.end..end]),
                                html_attribute(&link.attributes, "href").unwrap_or_default(),
                            )
                        })
                    })
                } else {
                    None
                };
                if let Some((label, url)) = only_link {
                    push_html_block(doc, BlockKind::Link, &format!("{label} | {url}"), false);
                } else {
                    let text = strip_tags(inner);
                    push_html_block(
                        doc,
                        if checked {
                            BlockKind::Checklist
                        } else {
                            BlockKind::Paragraph
                        },
                        &text,
                        inner.contains("checked") || tag.attributes.contains("checked"),
                    );
                }
            }
            "blockquote" => push_html_block(doc, BlockKind::Quote, &strip_tags(inner), false),
            "pre" => push_html_block(
                doc,
                if tag.attributes.contains("mermaid") {
                    BlockKind::Mermaid
                } else {
                    BlockKind::Code
                },
                &strip_tags(inner),
                false,
            ),
            "ul" => parse_html_list(doc, inner, BlockKind::Bullet, list_depth),
            "ol" => parse_html_list(doc, inner, BlockKind::Numbered, list_depth),
            "table" => push_html_block(doc, BlockKind::Table, &table_from_html(inner), false),
            "figure" => push_html_block(doc, BlockKind::Figure, &figure_from_html(inner), false),
            "details" => {
                let summary = element_inner(inner, "summary").unwrap_or_else(|| "詳細".to_string());
                let toggle_id = push_html_block_in(
                    doc,
                    BlockKind::Toggle,
                    &strip_tags(&summary),
                    false,
                    parent,
                    column,
                );
                parse_html_children(doc, &remove_first_element(inner, "summary"), toggle_id, 0);
            }
            "aside" => {
                if tag.attributes.contains("review-comment")
                    || tag.attributes.contains("data-block=\"comment\"")
                {
                    let text = strip_tags(inner);
                    let resolved = tag.attributes.contains("data-resolved=\"true\"")
                        || comment_is_resolved(&text);
                    push_html_block_in(
                        doc,
                        BlockKind::ReviewComment,
                        &text,
                        resolved,
                        parent,
                        column,
                    );
                } else {
                    let title = element_inner(inner, "p")
                        .map(|value| strip_tags(&value))
                        .unwrap_or_else(|| strip_tags(inner));
                    let callout_id =
                        push_html_block_in(doc, BlockKind::Callout, &title, false, parent, column);
                    parse_html_children(doc, &remove_first_element(inner, "p"), callout_id, 0);
                }
            }
            "a" => push_html_block(
                doc,
                BlockKind::Link,
                &format!(
                    "{} | {}",
                    strip_tags(inner),
                    html_attribute(&tag.attributes, "href").unwrap_or_default()
                ),
                false,
            ),
            "video" => push_html_block(
                doc,
                BlockKind::Video,
                &html_attribute(&tag.attributes, "src").unwrap_or_default(),
                false,
            ),
            "strong" | "em" | "mark" | "small" | "kbd" | "del" | "ins" | "dl" | "dt" | "dd" => {
                push_html_block(
                    doc,
                    BlockKind::Semantic,
                    &format!("{} | {}", tag.name, strip_tags(inner)),
                    false,
                )
            }
            _ => parse_html_fragment(doc, inner, list_depth),
        }
    }
    let tail = html_unescape(source[cursor..].trim());
    if !tail.is_empty() {
        push_html_block(doc, BlockKind::Paragraph, &tail, false);
    }
}

fn parse_html_list(doc: &mut DocumentState, source: &str, kind: BlockKind, depth: usize) {
    let mut cursor = 0;
    while let Some(relative_start) = source[cursor..].find("<li") {
        let start = cursor + relative_start;
        let Some(tag) = html_tag_at(source, start) else {
            break;
        };
        let Some((inner_end, next)) = matching_html_close(source, "li", tag.end) else {
            break;
        };
        let inner = &source[tag.end..inner_end];
        let nested_start = ["<ul", "<ol"]
            .iter()
            .filter_map(|needle| inner.find(needle))
            .min()
            .unwrap_or(inner.len());
        let text = strip_tags(&inner[..nested_start]);
        if !text.is_empty() {
            push_html_block(doc, kind, &format!("{}{}", "  ".repeat(depth), text), false);
        }
        parse_html_fragment(doc, &inner[nested_start..], depth + 1);
        cursor = next;
    }
}

fn push_html_block(doc: &mut DocumentState, kind: BlockKind, text: &str, checked: bool) {
    push_html_block_in(doc, kind, text, checked, None, 0);
}

fn push_html_block_in(
    doc: &mut DocumentState,
    kind: BlockKind,
    text: &str,
    checked: bool,
    parent: Option<u32>,
    column: u8,
) -> u32 {
    let mut block = new_block(&mut doc.next_id, kind, text);
    block.checked = checked;
    block.parent = parent;
    block.column = column;
    let id = block.id;
    doc.blocks.push(block);
    id
}

fn parse_html_children(doc: &mut DocumentState, source: &str, parent: u32, column: u8) {
    let start = doc.blocks.len();
    parse_html_fragment(doc, source, 0);
    for child in &mut doc.blocks[start..] {
        if child.parent.is_none() {
            child.parent = Some(parent);
            child.column = column;
        }
    }
}

fn remove_first_element(source: &str, name: &str) -> String {
    let Some(start) = source.find(&format!("<{name}")) else {
        return source.to_string();
    };
    let Some(tag) = html_tag_at(source, start) else {
        return source.to_string();
    };
    let Some((_, end)) = matching_html_close(source, name, tag.end) else {
        return source.to_string();
    };
    format!("{}{}", &source[..start], &source[end..])
}

fn column_fragments(source: &str) -> Vec<(u8, String)> {
    let mut columns = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("<div") {
        let start = cursor + relative;
        let Some(tag) = html_tag_at(source, start) else {
            break;
        };
        let Some((inner_end, next)) = matching_html_close(source, "div", tag.end) else {
            break;
        };
        if tag.attributes.contains("blockpad-column") {
            let column = html_attribute(&tag.attributes, "data-column")
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(columns.len() as u8);
            columns.push((column.min(2), source[tag.end..inner_end].to_string()));
        }
        cursor = next;
    }
    columns
}

struct HtmlTag {
    name: String,
    attributes: String,
    end: usize,
    closing: bool,
    self_closing: bool,
}

fn html_tag_at(source: &str, start: usize) -> Option<HtmlTag> {
    let remainder = &source[start..];
    let close_offset = remainder.find('>')?;
    let raw = &remainder[1..close_offset];
    let trimmed = raw.trim();
    if trimmed.starts_with('!') || trimmed.starts_with('?') {
        return Some(HtmlTag {
            name: String::new(),
            attributes: String::new(),
            end: start + close_offset + 1,
            closing: true,
            self_closing: true,
        });
    }
    let closing = trimmed.starts_with('/');
    let without_slash = trimmed.trim_start_matches('/').trim();
    let name_end = without_slash
        .find(|character: char| character.is_whitespace() || character == '/')
        .unwrap_or(without_slash.len());
    let name = without_slash[..name_end].to_ascii_lowercase();
    Some(HtmlTag {
        name,
        attributes: without_slash[name_end..].trim().to_string(),
        end: start + close_offset + 1,
        closing,
        self_closing: trimmed.ends_with('/'),
    })
}

fn matching_html_close(source: &str, name: &str, mut cursor: usize) -> Option<(usize, usize)> {
    let mut depth = 1;
    while let Some(relative_start) = source[cursor..].find('<') {
        let start = cursor + relative_start;
        let tag = html_tag_at(source, start)?;
        cursor = tag.end;
        if tag.name != name {
            continue;
        }
        if tag.closing {
            depth -= 1;
            if depth == 0 {
                return Some((start, tag.end));
            }
        } else if !tag.self_closing {
            depth += 1;
        }
    }
    None
}

fn element_inner(source: &str, name: &str) -> Option<String> {
    let start = source.find(&format!("<{name}"))?;
    let tag = html_tag_at(source, start)?;
    let (inner_end, _) = matching_html_close(source, name, tag.end)?;
    Some(source[tag.end..inner_end].to_string())
}

fn table_from_html(source: &str) -> String {
    let mut rows = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = source[cursor..].find("<tr") {
        let start = cursor + relative_start;
        let Some(tag) = html_tag_at(source, start) else {
            break;
        };
        let Some((inner_end, next)) = matching_html_close(source, "tr", tag.end) else {
            break;
        };
        let row_source = &source[tag.end..inner_end];
        let mut cells = Vec::new();
        for cell_tag in ["th", "td"] {
            let mut cell_cursor = 0;
            while let Some(relative_cell) = row_source[cell_cursor..].find(&format!("<{cell_tag}"))
            {
                let cell_start = cell_cursor + relative_cell;
                let Some(cell_open) = html_tag_at(row_source, cell_start) else {
                    break;
                };
                let Some((cell_end, cell_next)) =
                    matching_html_close(row_source, cell_tag, cell_open.end)
                else {
                    break;
                };
                cells.push(strip_tags(&row_source[cell_open.end..cell_end]));
                cell_cursor = cell_next;
            }
            if !cells.is_empty() {
                break;
            }
        }
        if !cells.is_empty() {
            rows.push(cells.join(" | "));
        }
        cursor = next;
    }
    rows.join("\n")
}

fn figure_from_html(source: &str) -> String {
    let image_start = source.find("<img");
    let url = image_start
        .and_then(|start| html_tag_at(source, start))
        .and_then(|tag| html_attribute(&tag.attributes, "src"))
        .unwrap_or_default();
    let caption = element_inner(source, "figcaption")
        .map(|caption| strip_tags(&caption))
        .unwrap_or_default();
    format!("{url} | {caption}")
}

fn new_block(next_id: &mut u32, kind: BlockKind, text: &str) -> Block {
    let id = *next_id;
    *next_id += 1;
    Block {
        id,
        kind,
        text: text.to_string(),
        checked: false,
        parent: None,
        column: 0,
    }
}

fn numbered_item(line: &str) -> Option<&str> {
    let dot = line.find(". ")?;
    if line[..dot].chars().all(|c| c.is_ascii_digit()) {
        Some(&line[dot + 2..])
    } else {
        None
    }
}

fn markdown_image_url(line: &str) -> Option<&str> {
    if !line.starts_with("![") {
        return None;
    }
    line.rsplit_once('(')
        .and_then(|(_, rest)| rest.strip_suffix(')'))
}

fn markdown_link_parts(line: &str) -> Option<(&str, &str)> {
    if line.starts_with("![") || !line.starts_with('[') {
        return None;
    }
    let (label, remainder) = line.strip_prefix('[')?.split_once("](")?;
    Some((label, remainder.strip_suffix(')')?))
}

fn html_attribute(input: &str, attribute: &str) -> Option<String> {
    let marker = format!("{attribute}=\"");
    input
        .split_once(&marker)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| html_unescape(value))
}

fn between<'a>(input: &'a str, start: &str, end: &str) -> Option<&'a str> {
    input
        .split_once(start)?
        .1
        .split_once(end)
        .map(|(value, _)| value)
}

fn strip_tags(input: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for character in input.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(character),
            _ => {}
        }
    }
    html_unescape(result.trim())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn toggle_parts(value: &str) -> (&str, &str) {
    value
        .split_once('|')
        .map(|(title, body)| (title.trim(), body.trim()))
        .unwrap_or((value.trim(), ""))
}

fn link_parts(value: &str) -> (&str, &str) {
    value
        .split_once('|')
        .map(|(label, url)| (label.trim(), url.trim()))
        .unwrap_or((value.trim(), ""))
}

fn figure_parts(value: &str) -> (&str, &str) {
    value
        .split_once('|')
        .map(|(url, caption)| (url.trim(), caption.trim()))
        .unwrap_or((value.trim(), ""))
}

fn semantic_parts(value: &str) -> (&str, &str) {
    let (candidate, content) = value
        .split_once('|')
        .map(|(tag, content)| (tag.trim(), content.trim()))
        .unwrap_or(("span", value.trim()));
    let allowed = [
        "strong", "em", "mark", "small", "kbd", "del", "ins", "h4", "h5", "h6", "dl", "dt", "dd",
    ];
    if allowed.contains(&candidate) {
        (candidate, content)
    } else {
        ("span", content)
    }
}

fn slash_options(value: &str) -> Vec<BlockKind> {
    let query = value.trim_start().trim_start_matches('/').trim();
    BlockKind::MENU
        .iter()
        .map(|(kind, _)| *kind)
        .filter(|kind| kind.matches_query(query))
        .collect()
}

fn convert_block(document: &mut DocumentState, id: u32, kind: BlockKind) {
    let previous_text = document
        .blocks
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.text.clone())
        .unwrap_or_default();
    if let Some(item) = document.blocks.iter_mut().find(|item| item.id == id) {
        item.kind = kind;
        item.text = match kind {
            BlockKind::Mermaid => "flowchart LR\n  A[開始] --> B[完了]".to_string(),
            BlockKind::ReviewComment => "確認したい点を入力".to_string(),
            BlockKind::Toggle => "折りたたみの見出し".to_string(),
            BlockKind::Callout => "補足のグループ".to_string(),
            BlockKind::Group => "新しいグループ".to_string(),
            BlockKind::Columns => "比較カラム".to_string(),
            _ => String::new(),
        };
        item.checked = false;
    }
    if kind == BlockKind::Columns {
        let left_id = document.next_id;
        let right_id = left_id + 1;
        document.next_id += 2;
        document.blocks.push(Block {
            id: left_id,
            kind: BlockKind::Paragraph,
            text: if previous_text.starts_with('/') || previous_text.is_empty() {
                "左のカラム".to_string()
            } else {
                previous_text
            },
            checked: false,
            parent: Some(id),
            column: 0,
        });
        document.blocks.push(Block {
            id: right_id,
            kind: BlockKind::Paragraph,
            text: "右のカラム".to_string(),
            checked: false,
            parent: Some(id),
            column: 1,
        });
    }
}

fn indent_block(document: &mut DocumentState, id: u32) -> bool {
    let Some(index) = document.blocks.iter().position(|block| block.id == id) else {
        return false;
    };
    let parent = document.blocks[index].parent;
    let column = document.blocks[index].column;
    let previous_group = document.blocks[..index]
        .iter()
        .rev()
        .find(|block| {
            block.parent == parent && block.column == column && block.kind.can_have_children()
        })
        .map(|block| block.id);
    if let Some(group_id) = previous_group {
        document.blocks[index].parent = Some(group_id);
        document.blocks[index].column = 0;
        true
    } else {
        false
    }
}

fn outdent_block(document: &mut DocumentState, id: u32) -> bool {
    let Some(index) = document.blocks.iter().position(|block| block.id == id) else {
        return false;
    };
    let Some(parent_id) = document.blocks[index].parent else {
        return false;
    };
    let Some((grandparent, column)) = document
        .blocks
        .iter()
        .find(|block| block.id == parent_id)
        .map(|parent| (parent.parent, parent.column))
    else {
        return false;
    };
    document.blocks[index].parent = grandparent;
    document.blocks[index].column = column;
    true
}

fn collect_subtree_ids(blocks: &[Block], id: u32, ids: &mut Vec<u32>) {
    if ids.contains(&id) {
        return;
    }
    ids.push(id);
    for child in blocks.iter().filter(|block| block.parent == Some(id)) {
        collect_subtree_ids(blocks, child.id, ids);
    }
}

fn move_subtree_before(document: &mut DocumentState, id: u32, target_id: u32) -> bool {
    if id == target_id {
        return false;
    }
    let mut subtree_ids = Vec::new();
    collect_subtree_ids(&document.blocks, id, &mut subtree_ids);
    if subtree_ids.contains(&target_id) {
        return false;
    }
    let mut moved = Vec::new();
    document.blocks.retain(|block| {
        if subtree_ids.contains(&block.id) {
            moved.push(block.clone());
            false
        } else {
            true
        }
    });
    let destination = document
        .blocks
        .iter()
        .position(|block| block.id == target_id)
        .unwrap_or(document.blocks.len());
    document.blocks.splice(destination..destination, moved);
    true
}

fn drop_block_on(document: &mut DocumentState, id: u32, target_id: u32) -> bool {
    let Some(target) = document
        .blocks
        .iter()
        .find(|block| block.id == target_id)
        .cloned()
    else {
        return false;
    };
    if target.kind.can_have_children() && target.id != id {
        if let Some(block) = document.blocks.iter_mut().find(|block| block.id == id) {
            block.parent = Some(target.id);
            block.column = 0;
            return true;
        }
        return false;
    }
    if let Some(block) = document.blocks.iter_mut().find(|block| block.id == id) {
        block.parent = target.parent;
        block.column = target.column;
    }
    move_subtree_before(document, id, target_id)
}

fn create_columns(
    document: &mut DocumentState,
    dragged_id: u32,
    target_id: u32,
    place_after: bool,
) -> Option<u32> {
    if dragged_id == target_id {
        return None;
    }
    let target = document
        .blocks
        .iter()
        .find(|block| block.id == target_id)
        .cloned()?;
    if target.kind == BlockKind::Columns {
        return None;
    }
    if let Some(columns_id) = target.parent.filter(|parent| {
        document
            .blocks
            .iter()
            .find(|block| block.id == *parent)
            .is_some_and(|block| block.kind == BlockKind::Columns)
    }) {
        let highest_column = document
            .blocks
            .iter()
            .filter(|block| block.parent == Some(columns_id))
            .map(|block| block.column)
            .max()
            .unwrap_or(0);
        let requested = if place_after {
            target.column.saturating_add(1)
        } else {
            target.column
        };
        if requested > 2 || (highest_column >= 2 && requested > highest_column) {
            return None;
        }
        if let Some(block) = document
            .blocks
            .iter_mut()
            .find(|block| block.id == dragged_id)
        {
            block.parent = Some(columns_id);
            block.column = requested;
        }
        return Some(columns_id);
    }

    let columns_id = document.next_id;
    document.next_id += 1;
    let parent = target.parent;
    let column = target.column;
    let target_index = document
        .blocks
        .iter()
        .position(|block| block.id == target_id)
        .unwrap_or(document.blocks.len());
    document.blocks.insert(
        target_index,
        Block {
            id: columns_id,
            kind: BlockKind::Columns,
            text: "比較カラム".to_string(),
            checked: false,
            parent,
            column,
        },
    );
    if let Some(block) = document
        .blocks
        .iter_mut()
        .find(|block| block.id == target_id)
    {
        block.parent = Some(columns_id);
        block.column = if place_after { 0 } else { 1 };
    }
    if let Some(block) = document
        .blocks
        .iter_mut()
        .find(|block| block.id == dragged_id)
    {
        block.parent = Some(columns_id);
        block.column = if place_after { 1 } else { 0 };
    }
    Some(columns_id)
}

fn find_match_ids(document: &DocumentState, query: &str) -> Vec<u32> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    document
        .blocks
        .iter()
        .filter(|block| block.text.to_lowercase().contains(&query))
        .map(|block| block.id)
        .collect()
}

fn save_status(status: &str) -> String {
    match status {
        "original" => "元のローカルファイルへ保存しました".to_string(),
        "picker" => "ブラウザの保存先を選んで保存しました".to_string(),
        "downloaded" => "ローカルコピーをダウンロードしました".to_string(),
        "cancelled" => "保存を取り消しました".to_string(),
        _ => "保存しました".to_string(),
    }
}

fn table_rows(value: &str) -> Vec<Vec<&str>> {
    value
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect()
        })
        .collect()
}

fn table_markdown(value: &str) -> String {
    let rows = table_rows(value);
    if rows.is_empty() {
        return "| Column | Detail |\n| --- | --- |".to_string();
    }
    let mut output = rows
        .iter()
        .map(|row| format!("| {} |", row.join(" | ")))
        .collect::<Vec<_>>();
    if rows.len() == 1 {
        output.insert(
            1,
            format!(
                "| {} |",
                rows[0]
                    .iter()
                    .map(|_| "---")
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        );
    }
    output.join("\n")
}

fn table_html(id: &str, value: &str) -> String {
    let rows = table_rows(value);
    let rendered = rows
        .iter()
        .map(|row| {
            format!(
                "<tr>{}</tr>",
                row.iter()
                    .map(|cell| format!("<td>{}</td>", escape_html(cell)))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    format!("<table id=\"{id}\"><tbody>{rendered}</tbody></table>")
}

#[cfg(target_arch = "wasm32")]
async fn load_from_browser() -> Result<Option<(String, SourceType, String)>, String> {
    let value = open_local_file()
        .await
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| "The browser did not return file contents".to_string())?;
    if value.is_empty() {
        return Ok(None);
    }
    let mut fields = value.splitn(3, '\0');
    let name = fields.next().unwrap_or("untitled.md").to_string();
    let source = match fields.next().unwrap_or("markdown") {
        "html" => SourceType::Html,
        _ => SourceType::Markdown,
    };
    let content = fields.next().unwrap_or_default().to_string();
    Ok(Some((name, source, content)))
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_from_browser() -> Result<Option<(String, SourceType, String)>, String> {
    Err("File access is available in the browser build".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn save_to_browser(name: &str, content: &str, mime: &str) -> Result<String, String> {
    save_local_file(name, content, mime)
        .await
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| "The browser did not return save status".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn save_to_browser(_name: &str, _content: &str, _mime: &str) -> Result<String, String> {
    Err("File saving is available in the browser build".to_string())
}

#[cfg(target_arch = "wasm32")]
fn set_browser_theme(is_dark: bool) {
    apply_theme(is_dark);
}

#[cfg(not(target_arch = "wasm32"))]
fn set_browser_theme(_is_dark: bool) {}

#[cfg(target_arch = "wasm32")]
fn focus_editor_block(id: u32) {
    focus_block(id);
}

#[cfg(not(target_arch = "wasm32"))]
fn focus_editor_block(_id: u32) {}

#[cfg(target_arch = "wasm32")]
fn visit_block_anchor(anchor: &str) {
    visit_anchor(anchor);
}

#[cfg(not(target_arch = "wasm32"))]
fn visit_block_anchor(_anchor: &str) {}

#[cfg(target_arch = "wasm32")]
fn render_mermaid_when_ready(id: u32, source: &str) {
    render_mermaid_block(&format!("block-{id}"), source);
}

#[cfg(not(target_arch = "wasm32"))]
fn render_mermaid_when_ready(_id: u32, _source: &str) {}

#[cfg(target_arch = "wasm32")]
fn find_browser_text(query: &str) {
    find_in_page(query);
}

#[cfg(not(target_arch = "wasm32"))]
fn find_browser_text(_query: &str) {}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        } else {
            let _ = resolve.call0(&wasm_bindgen::JsValue::UNDEFINED);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(_ms: i32) {}

#[cfg(target_arch = "wasm32")]
async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    copy_text(text)
        .await
        .map(|_| ())
        .map_err(js_error)
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_to_clipboard(_text: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "ブラウザのファイル API でエラーが発生しました".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_export_keeps_block_syntax() {
        let document = DocumentState {
            title: "Release notes".to_string(),
            file_name: "release.md".to_string(),
            source: SourceType::Markdown,
            next_id: 4,
            blocks: vec![
                Block {
                    id: 1,
                    kind: BlockKind::Checklist,
                    text: "Review the export".to_string(),
                    checked: true,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 2,
                    kind: BlockKind::Code,
                    text: "cargo test".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 3,
                    kind: BlockKind::Image,
                    text: "https://example.test/diagram.png".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
            ],
        };

        let output = serialize_markdown(&document);

        assert!(output.starts_with("# Release notes\n"));
        assert!(output.contains("- [x] Review the export"));
        assert!(output.contains("```\ncargo test\n```"));
        assert!(output.contains("![](https://example.test/diagram.png)"));
    }

    #[test]
    fn markdown_export_does_not_duplicate_first_heading() {
        let document = DocumentState {
            title: "Brief".to_string(),
            file_name: "brief.md".to_string(),
            source: SourceType::Markdown,
            next_id: 2,
            blocks: vec![Block {
                id: 1,
                kind: BlockKind::HeadingOne,
                text: "Brief".to_string(),
                checked: false,
                parent: None,
                column: 0,
            }],
        };

        assert_eq!(serialize_markdown(&document), "# Brief\n");
    }

    #[test]
    fn review_markdown_round_trips_mermaid_and_comments() {
        let source = "# 画面遷移\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n> [!COMMENT]\n> モバイル表示を確認";
        let document = parse_document("review.md", SourceType::Markdown, source);

        assert_eq!(document.blocks[1].kind, BlockKind::Mermaid);
        assert_eq!(document.blocks[2].kind, BlockKind::ReviewComment);

        let output = serialize_markdown(&document);
        assert!(output.contains("```mermaid\nflowchart LR\n  A --> B\n```"));
        assert!(output.contains("> [!COMMENT]\n> モバイル表示を確認"));
    }

    #[test]
    fn html_export_is_structured_and_escapes_text() {
        let document = DocumentState {
            title: "Safety < first".to_string(),
            file_name: "safety.html".to_string(),
            source: SourceType::Html,
            next_id: 3,
            blocks: vec![
                Block {
                    id: 1,
                    kind: BlockKind::HeadingTwo,
                    text: "A & B".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 2,
                    kind: BlockKind::Paragraph,
                    text: "Use <strong> only as text".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
            ],
        };

        let output = serialize_html(&document);

        assert!(output.starts_with("<!doctype html>"));
        assert!(output.contains("<title>Safety &lt; first</title>"));
        assert!(output.contains("<h2 id=\"block-1\">A &amp; B</h2>"));
        assert!(output.contains("<p id=\"block-2\">Use &lt;strong&gt; only as text</p>"));
        assert!(output.ends_with("</html>\n"));
    }

    #[test]
    fn html_import_handles_compact_document_markup() {
        let document = parse_document(
            "compact.html",
            SourceType::Html,
            "<!doctype html><html><body><h1>Compact note</h1><p>Works without source line breaks.</p><img src=\"https://example.test/a.png\"></body></html>",
        );

        assert_eq!(document.title, "Compact note");
        assert_eq!(document.blocks.len(), 3);
        assert_eq!(document.blocks[1].text, "Works without source line breaks.");
        assert_eq!(document.blocks[2].kind, BlockKind::Image);
    }

    #[test]
    fn html_import_preserves_review_structures() {
        let source = r#"<article><h1>設計レビュー</h1><table><tr><th>論点</th><th>状態</th></tr><tr><td>計測</td><td>確認中</td></tr></table><ul><li>親<ul><li>子</li></ul></li></ul><figure><img src="diagram.png"><figcaption>画面遷移</figcaption></figure><p><a href="https://example.test">仕様書</a></p><details><summary>補足</summary><p>詳細</p></details></article>"#;
        let document = parse_document("review.html", SourceType::Html, source);

        assert!(document
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Table));
        assert!(document
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Bullet));
        assert!(document
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Figure));
        assert!(document
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Link));
        assert!(document
            .blocks
            .iter()
            .any(|block| block.kind == BlockKind::Toggle));
        let output = serialize_html(&document);
        assert!(output.contains("<table"));
        assert!(output.contains("<figure"));
        assert!(output.contains("<a href=\"https://example.test\">仕様書</a>"));
    }

    #[test]
    fn mermaid_html_round_trips_as_a_diagram_block() {
        let document = parse_document(
            "flow.html",
            SourceType::Html,
            r#"<pre class="mermaid">flowchart LR
  A[開始] --> B[完了]</pre>"#,
        );

        assert_eq!(document.blocks[0].kind, BlockKind::Mermaid);
        assert!(serialize_html(&document).contains("class=\"mermaid\" data-block=\"mermaid\""));
    }

    #[test]
    fn slash_typeahead_supports_japanese_and_english_aliases() {
        let mermaid = slash_options("/merm");
        let review = slash_options("/指摘");
        let comment = slash_options("/comment");
        let llm = slash_options("/llm");
        let reply = slash_options("/返す");

        assert_eq!(mermaid, vec![BlockKind::Mermaid]);
        assert_eq!(review, vec![BlockKind::ReviewComment]);
        assert_eq!(comment, vec![BlockKind::ReviewComment]);
        assert_eq!(llm, vec![BlockKind::ReviewComment]);
        assert_eq!(reply, vec![BlockKind::ReviewComment]);
    }

    #[test]
    fn find_returns_matching_block_ids_in_document_order() {
        let document = DocumentState::starter();
        let matches = find_match_ids(&document, "レビュー");

        assert_eq!(matches, vec![9, 10, 11]);
    }

    #[test]
    fn tab_nests_into_group_and_shift_tab_outdents() {
        let mut document = DocumentState::starter();

        assert!(outdent_block(&mut document, 14));
        assert_eq!(
            document
                .blocks
                .iter()
                .find(|block| block.id == 14)
                .unwrap()
                .parent,
            None
        );
        assert!(indent_block(&mut document, 14));
        assert_eq!(
            document
                .blocks
                .iter()
                .find(|block| block.id == 14)
                .unwrap()
                .parent,
            Some(13)
        );
    }

    #[test]
    fn grouped_markdown_round_trips_children_and_columns() {
        let document = DocumentState::starter();
        let markdown = serialize_markdown(&document);
        let restored = parse_document("review.md", SourceType::Markdown, &markdown);

        let toggle = restored
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Toggle)
            .unwrap();
        assert!(restored
            .blocks
            .iter()
            .any(|block| block.parent == Some(toggle.id)));
        let columns = restored
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::Columns)
            .unwrap();
        assert!(restored
            .blocks
            .iter()
            .any(|block| block.parent == Some(columns.id) && block.column == 1));
    }

    #[test]
    fn columns_keep_children_together_when_moved() {
        let mut document = DocumentState::starter();
        let columns_id = create_columns(&mut document, 2, 3, true).unwrap();
        assert_eq!(
            document
                .blocks
                .iter()
                .find(|block| block.id == 2)
                .unwrap()
                .parent,
            Some(columns_id)
        );
        assert!(move_subtree_before(&mut document, columns_id, 1));
        let mut subtree = Vec::new();
        collect_subtree_ids(&document.blocks, columns_id, &mut subtree);
        assert!(subtree.contains(&2));
        assert!(subtree.contains(&3));
    }

    #[test]
    fn markdown_import_recognizes_editor_blocks() {
        let document = parse_document(
            "brief.md",
            SourceType::Markdown,
            "# Brief\n\n- [ ] Write intro\n\n> [!NOTE]\n> Verify local saving\n\n---",
        );

        assert_eq!(document.title, "Brief");
        assert_eq!(document.blocks[0].kind, BlockKind::HeadingOne);
        assert_eq!(document.blocks[1].kind, BlockKind::Checklist);
        assert_eq!(document.blocks[2].kind, BlockKind::Callout);
        assert_eq!(document.blocks[3].kind, BlockKind::Divider);
    }

    #[test]
    fn review_prompt_includes_quote_and_instruction() {
        let document = DocumentState::starter();
        let targets = vec![review_target(
            2,
            BlockKind::Paragraph,
            "初回体験を短くし、利用者が最初の価値に到達するまでの手順を明確にします。",
            false,
        )];
        let review = "モバイルの招待も同じ導線で確認してください。";
        let prompt = build_llm_prompt(&document, &targets, review);
        let json = build_llm_json(&document, &targets, review);

        assert!(prompt.contains("Markdown ブロックエディタからのレビュー"));
        assert!(prompt.contains("部分選択 · L1"));
        assert!(prompt.contains("初回体験を短くし"));
        assert!(prompt.contains("モバイルの招待も同じ導線で確認してください。"));
        assert!(prompt.contains("変更後の Markdown を返す"));
        assert!(json.contains("\"source\": \"md-block-editor\""));
        assert!(json.contains("\"whole_block\": false"));
        assert!(json.contains("\\u") || json.contains("モバイル"));
    }

    #[test]
    fn review_comment_round_trips_quote_and_body() {
        let mut document = DocumentState::starter();
        let targets = vec![
            review_target(2, BlockKind::Paragraph, "初回体験を短くする", false),
            review_target(7, BlockKind::Mermaid, "flowchart LR\n  A --> B", true),
        ];
        let id = insert_review_comment(&mut document, &targets, "分岐の失敗時を足してください。")
            .unwrap();
        let comment = document
            .blocks
            .iter()
            .find(|block| block.id == id)
            .unwrap();
        assert_eq!(comment.kind, BlockKind::ReviewComment);
        let (quotes, body) = review_comment_parts(&comment.text);
        assert_eq!(quotes.len(), 2);
        assert_eq!(quotes[0].1, "初回体験を短くする");
        assert!(quotes[1].1.contains("A --> B"));
        assert_eq!(body, "分岐の失敗時を足してください。");

        let markdown = serialize_markdown(&document);
        let restored = parse_document("design-review.md", SourceType::Markdown, &markdown);
        assert!(restored.blocks.iter().any(|block| {
            block.kind == BlockKind::ReviewComment
                && block.text.contains("分岐の失敗時を足してください。")
                && block.text.contains("対象: #2 テキスト")
        }));
    }

    #[test]
    fn generate_review_draft_uses_kind_and_length() {
        let heading = generate_review_draft(BlockKind::HeadingTwo, "提案フロー");
        let long = generate_review_draft(
            BlockKind::Paragraph,
            "初回体験を短くし、利用者が最初の価値に到達するまでの手順を明確にします。さらに説明を足して一文を長くし、結論と根拠が混ざる文章にして、どこを直すのか分かりにくくします。",
        );
        let empty = generate_review_draft(BlockKind::Paragraph, "   ");

        assert!(heading.contains("見出し"));
        assert!(
            long.contains("一文が長い"),
            "draft was: {long:?} (chars={})",
            long.chars().count()
        );
        assert!(empty.contains("対象が空"));
        assert!(review_export_error(&[], "本文").is_some());
        assert!(review_export_error(
            &[review_target(1, BlockKind::Paragraph, "x", true)],
            "   "
        )
        .is_some());
        assert!(review_export_error(
            &[review_target(1, BlockKind::Paragraph, "x", true)],
            "直してください"
        )
        .is_none());
    }
}

