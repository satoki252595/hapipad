fn build_existing_comments_prompt(doc: &DocumentState, comments: &[Block]) -> String {
    let mut output = String::from(
        "以下は Markdown ブロックエディタに残っている指摘です。未解決の点を直してください。\n\n",
    );
    output.push_str(&format!(
        "# 文書\n- ファイル: {}\n- タイトル: {}\n\n# 指摘\n",
        doc.file_name, doc.title
    ));
    for (index, comment) in comments.iter().enumerate() {
        let (quotes, body) = review_comment_parts(&comment.text);
        output.push_str(&format!("## {}. 指摘ブロック #{}\n", index + 1, comment.id));
        for (label, quote) in quotes {
            output.push_str(&format!("対象: {label}\n"));
            for line in quote.lines() {
                output.push_str(&format!("> {line}\n"));
            }
        }
        output.push_str(&format!("{}\n\n", body));
    }
    output.push_str("引用箇所だけを直し、変更後の Markdown を返してください。\n");
    output
}

fn render_editor_tree(
    block: Block,
    index: usize,
    all_blocks: Vec<Block>,
    document: Signal<DocumentState>,
    slash_for: Signal<Option<u32>>,
    slash_index: Signal<usize>,
    dragged: Signal<Option<usize>>,
    drop_target: Signal<Option<usize>>,
    notice: Signal<String>,
    review: ReviewUi,
    is_find_match: bool,
    is_active_find_match: bool,
) -> Element {
    let id = block.id;
    let kind = block.kind;
    let children = all_blocks
        .iter()
        .cloned()
        .enumerate()
        .filter(|(_, child)| child.parent == Some(id))
        .collect::<Vec<_>>();
    let is_container = kind.can_have_children();
    let column_count = children
        .iter()
        .map(|(_, child)| child.column as usize + 1)
        .max()
        .unwrap_or(2)
        .clamp(2, 3);

    if !is_container {
        return render_block_editor(
            block,
            index,
            document,
            slash_for,
            slash_index,
            dragged,
            drop_target,
            notice,
            review,
            is_find_match,
            is_active_find_match,
        );
    }

    let container_class = if kind == BlockKind::Columns {
        "editor-group editor-columns"
    } else {
        "editor-group"
    };
    rsx! {
        section { class: "{container_class}",
            {render_block_editor(
                block.clone(),
                index,
                document,
                slash_for,
                slash_index,
                dragged,
                drop_target,
                notice,
                review,
                is_find_match,
                is_active_find_match,
            )}
            if kind == BlockKind::Columns {
                div { class: "columns-editor",
                    for column in 0..column_count {
                        div { class: "editor-column",
                            div { class: "column-label", "{column + 1} 列目" }
                            for (child_index, child) in children.iter().filter(|(_, child)| child.column as usize == column) {
                                {render_editor_tree(
                                    child.clone(),
                                    *child_index,
                                    all_blocks.clone(),
                                    document,
                                    slash_for,
                                    slash_index,
                                    dragged,
                                    drop_target,
                                    notice,
                                    review,
                                    is_find_match,
                                    is_active_find_match,
                                )}
                            }
                            button {
                                class: "add-child",
                                onclick: move |_| {
                                    let mut document = document;
                                    let child_id = document.with_mut(|doc| doc.add_child(id, column as u8));
                                    focus_editor_block(child_id);
                                },
                                "+ この列に追加"
                            }
                            if column == column_count - 1 && column_count < 3 {
                                button {
                                    class: "add-child add-column",
                                    onclick: move |_| {
                                        let mut document = document;
                                        let child_id = document.with_mut(|doc| doc.add_child(id, (column + 1) as u8));
                                        focus_editor_block(child_id);
                                    },
                                    "+ 3 列目を追加"
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "group-children",
                    for (child_index, child) in children {
                        {render_editor_tree(
                            child,
                            child_index,
                            all_blocks.clone(),
                            document,
                            slash_for,
                            slash_index,
                            dragged,
                            drop_target,
                            notice,
                            review,
                            is_find_match,
                            is_active_find_match,
                        )}
                    }
                    button {
                        class: "add-child",
                        onclick: move |_| {
                            let mut document = document;
                            let child_id = document.with_mut(|doc| doc.add_child(id, 0));
                            focus_editor_block(child_id);
                        },
                        "+ 子ブロックを追加"
                    }
                }
            }
        }
    }
}

fn render_block_editor(
    block: Block,
    index: usize,
    mut document: Signal<DocumentState>,
    mut slash_for: Signal<Option<u32>>,
    mut slash_index: Signal<usize>,
    mut dragged: Signal<Option<usize>>,
    mut drop_target: Signal<Option<usize>>,
    mut notice: Signal<String>,
    mut review: ReviewUi,
    is_find_match: bool,
    is_active_find_match: bool,
) -> Element {
    let id = block.id;
    let kind = block.kind;
    let text = block.text.clone();
    let mermaid_source = text.clone();
    let is_checked = block.checked;
    let mut class = if dragged() == Some(index) {
        format!("block-row {} dragging", kind.class_name())
    } else {
        format!("block-row {}", kind.class_name())
    };
    if drop_target() == Some(index) {
        class.push_str(" drop-target");
    }
    if is_active_find_match {
        class.push_str(" search-current");
    } else if is_find_match {
        class.push_str(" search-match");
    }
    if (review.targets)()
        .iter()
        .any(|target| target.block_id == id)
    {
        class.push_str(" review-target");
    }
    let input_id = format!("editor-block-{id}");
    let block_anchor = format!("block-{id}");

    rsx! {
        div {
            class: "{class}",
            draggable: true,
            ondragstart: move |_| dragged.set(Some(index)),
            ondragover: move |event| {
                event.prevent_default();
                drop_target.set(Some(index));
            },
            ondragleave: move |_| drop_target.set(None),
            ondrop: move |event| {
                event.prevent_default();
                let from = dragged();
                if let Some(from) = from {
                    if from != index {
                        let moved_into_group = document.with_mut(|doc| {
                            let source = doc.blocks.get(from).map(|block| block.id);
                            source.is_some_and(|source| drop_block_on(doc, source, id))
                        });
                        if moved_into_group {
                            let target_is_group = document()
                                .blocks
                                .iter()
                                .find(|block| block.id == id)
                                .is_some_and(|block| block.kind.can_have_children());
                            notice.set(if target_is_group {
                                "ブロックをグループへ移動しました".to_string()
                            } else {
                                "ブロックを並べ替えました".to_string()
                            });
                        }
                    }
                }
                dragged.set(None);
                drop_target.set(None);
            },
            ondragend: move |_| {
                dragged.set(None);
                drop_target.set(None);
            },
            if dragged().is_some() && dragged() != Some(index) {
                div { class: "column-drop-zones",
                    div {
                        class: "column-drop-zone left",
                        title: "ここへドロップして左カラムを作る",
                        ondragover: move |event| event.prevent_default(),
                        ondrop: move |event| {
                            event.prevent_default();
                            event.stop_propagation();
                            if let Some(from) = dragged() {
                                let changed = document.with_mut(|doc| {
                                    let source = doc.blocks.get(from).map(|block| block.id);
                                    source.and_then(|source| create_columns(doc, source, id, false)).is_some()
                                });
                                notice.set(if changed {
                                    "左カラムを作成しました".to_string()
                                } else {
                                    "カラムは最大 3 列です".to_string()
                                });
                            }
                            dragged.set(None);
                            drop_target.set(None);
                        }
                    }
                    div {
                        class: "column-drop-zone right",
                        title: "ここへドロップして右カラムを作る",
                        ondragover: move |event| event.prevent_default(),
                        ondrop: move |event| {
                            event.prevent_default();
                            event.stop_propagation();
                            if let Some(from) = dragged() {
                                let changed = document.with_mut(|doc| {
                                    let source = doc.blocks.get(from).map(|block| block.id);
                                    source.and_then(|source| create_columns(doc, source, id, true)).is_some()
                                });
                                notice.set(if changed {
                                    "右カラムを作成しました".to_string()
                                } else {
                                    "カラムは最大 3 列です".to_string()
                                });
                            }
                            dragged.set(None);
                            drop_target.set(None);
                        }
                    }
                }
            }
            button {
                class: "drag-handle",
                title: "ドラッグして並べ替える",
                aria_label: "ドラッグしてブロックを並べ替える",
                "⠿"
            }
            div { class: "input-wrap",
                if kind == BlockKind::Mermaid {
                    div { class: "block-kind-hint", "Mermaid ソース · 変更すると右側の図にすぐ反映されます" }
                }
                if kind == BlockKind::Checklist {
                    div { class: if is_checked { "check-wrap checked" } else { "check-wrap" },
                        input {
                            class: "checkbox",
                            r#type: "checkbox",
                            checked: is_checked,
                            aria_label: "チェックリストを完了にする",
                            onchange: move |_| {
                                document.with_mut(|doc| {
                                    if let Some(item) = doc.blocks.iter_mut().find(|item| item.id == id) {
                                        item.checked = !item.checked;
                                    }
                                });
                            }
                        }
                        textarea {
                            id: "{input_id}",
                            class: "block-input",
                            rows: 1,
                            value: "{text}",
                            placeholder: "{placeholder(kind)}",
                            oncompositionstart: move |_| review.composing.set(Some(id)),
                            oncompositionend: move |_| {
                                if (review.composing)() == Some(id) {
                                    review.composing.set(None);
                                }
                            },
                            onmouseup: move |event| {
                                adopt_text_selection(
                                    document,
                                    review,
                                    notice,
                                    id,
                                    event.modifiers().shift(),
                                );
                            },
                            oninput: move |event| {
                                let value = event.value();
                                document.with_mut(|doc| {
                                    if let Some(item) = doc.blocks.iter_mut().find(|item| item.id == id) {
                                        item.text = value.clone();
                                    }
                                });
                                if value.starts_with('/') {
                                    slash_for.set(Some(id));
                                    slash_index.set(0);
                                } else if slash_for() == Some(id) {
                                    slash_for.set(None);
                                }
                            },
                            onkeydown: move |event| handle_keydown(
                                event,
                                id,
                                index,
                                document,
                                slash_for,
                                slash_index,
                                notice,
                                review,
                            ),
                        }
                    }
                } else {
                    textarea {
                        id: "{input_id}",
                        class: "block-input",
                        rows: if matches!(kind, BlockKind::Code | BlockKind::Table | BlockKind::Mermaid) { 5 } else { 1 },
                        value: "{text}",
                        placeholder: "{placeholder(kind)}",
                        oncompositionstart: move |_| review.composing.set(Some(id)),
                        oncompositionend: move |_| {
                            if (review.composing)() == Some(id) {
                                review.composing.set(None);
                            }
                        },
                        onmouseup: move |event| {
                            adopt_text_selection(
                                document,
                                review,
                                notice,
                                id,
                                event.modifiers().shift(),
                            );
                        },
                        oninput: move |event| {
                            let value = event.value();
                            document.with_mut(|doc| {
                                if let Some(item) = doc.blocks.iter_mut().find(|item| item.id == id) {
                                    item.text = value.clone();
                                }
                            });
                            if kind == BlockKind::Mermaid {
                                render_mermaid_when_ready(id, &value);
                            }
                            if value.starts_with('/') {
                                slash_for.set(Some(id));
                                slash_index.set(0);
                            } else if slash_for() == Some(id) {
                                slash_for.set(None);
                            }
                        },
                        onkeydown: move |event| handle_keydown(
                            event,
                            id,
                            index,
                            document,
                            slash_for,
                            slash_index,
                            notice,
                            review,
                        ),
                    }
                }
                if slash_for() == Some(id) {
                    {render_slash_menu(id, &text, document, slash_for, slash_index)}
                }
            }
            div { class: "block-actions",
                select {
                    class: "mini-button",
                    title: "ブロックの種類を変更",
                    value: "{kind.label()}",
                    onchange: move |event| {
                        let next = BlockKind::from_label(&event.value());
                        document.with_mut(|doc| {
                            if let Some(item) = doc.blocks.iter_mut().find(|item| item.id == id) {
                                item.kind = next;
                            }
                        });
                        if next == BlockKind::Mermaid {
                            render_mermaid_when_ready(id, &mermaid_source);
                        }
                    },
                    for (_, label) in BlockKind::MENU {
                        option { value: "{label}", "{label}" }
                    }
                }
                button {
                    class: "mini-button",
                    title: "このブロックをレビュー対象にする",
                    aria_label: "このブロックをレビュー対象にする",
                    onclick: move |event| {
                        if (review.composing)() == Some(id) {
                            return;
                        }
                        adopt_whole_block(
                            document,
                            review,
                            notice,
                            id,
                            event.modifiers().shift(),
                        );
                    },
                    "レビュー"
                }
                button {
                    class: "mini-button",
                    title: "プレビューの固定アンカーへ移動",
                    aria_label: "プレビューの固定アンカーへ移動",
                    onclick: move |_| {
                        visit_block_anchor(&block_anchor);
                        notice.set(format!("#{block_anchor} へ移動しました"));
                    },
                    "↗"
                }
                button {
                    class: "mini-button",
                    title: "ブロックを複製",
                    aria_label: "ブロックを複製",
                    onclick: move |_| {
                        let mut document = document;
                        let next = document.with_mut(|doc| {
                            let mut copy = doc.blocks.iter().find(|item| item.id == id).cloned().unwrap_or(Block {
                                id: 0,
                                kind: BlockKind::Paragraph,
                                text: String::new(),
                                checked: false,
                                parent: None,
                                column: 0,
                            });
                            copy.id = doc.next_id;
                            doc.next_id += 1;
                            let new_id = copy.id;
                            doc.blocks.insert(index + 1, copy);
                            new_id
                        });
                        focus_editor_block(next);
                        notice.set("ブロックを複製しました".to_string());
                    },
                    "⧉"
                }
            }
        }
    }
}

fn render_slash_menu(
    id: u32,
    text: &str,
    mut document: Signal<DocumentState>,
    mut slash_for: Signal<Option<u32>>,
    mut slash_index: Signal<usize>,
) -> Element {
    let options = slash_options(text);
    let query = text.trim_start().trim_start_matches('/').trim().to_string();
    rsx! {
        div { class: "slash-menu",
            div { class: "slash-label",
                if query.is_empty() {
                    "ブロックを選択 · ↑↓ と Enter で決定"
                } else {
                    "「{query}」に一致するブロック"
                }
            }
            div { class: "slash-options",
                if options.is_empty() {
                    div { class: "slash-empty", "一致するブロックがありません。別のキーワードを入力してください。" }
                }
                for (option_index, kind) in options.into_iter().enumerate() {
                    {
                        let class = if option_index == slash_index() {
                            "slash-option selected"
                        } else {
                            "slash-option"
                        };
                        rsx! {
                            button {
                                class: "{class}",
                                onmouseenter: move |_| slash_index.set(option_index),
                                onclick: move |_| {
                                    document.with_mut(|doc| convert_block(doc, id, kind));
                                    slash_for.set(None);
                                    if kind == BlockKind::Mermaid {
                                        render_mermaid_when_ready(id, "flowchart LR\n  A[開始] --> B[完了]");
                                    }
                                    focus_editor_block(id);
                                },
                                "{kind.label()}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_preview_tree(block: Block, all_blocks: Vec<Block>) -> Element {
    let id = block.id;
    let children = all_blocks
        .iter()
        .filter(|child| child.parent == Some(id))
        .cloned()
        .collect::<Vec<_>>();
    if !block.kind.can_have_children() {
        return render_preview_block(block);
    }

    let anchor = format!("block-{id}");
    let text = block.text.clone();
    match block.kind {
        BlockKind::Toggle => {
            let (summary, legacy_body) = toggle_parts(&text);
            rsx! {
                details { id: "{anchor}", class: "group-preview toggle-preview",
                    summary { "{summary}" }
                    if !legacy_body.is_empty() {
                        p { "{legacy_body}" }
                    }
                    div { class: "nested-preview",
                        for child in children {
                            {render_preview_tree(child, all_blocks.clone())}
                        }
                    }
                }
            }
        }
        BlockKind::Callout => rsx! {
            aside { id: "{anchor}", class: "callout-preview group-preview",
                strong { "✦" }
                div {
                    p { "{text}" }
                    div { class: "nested-preview",
                        for child in children {
                            {render_preview_tree(child, all_blocks.clone())}
                        }
                    }
                }
            }
        },
        BlockKind::Group => rsx! {
            section { id: "{anchor}", class: "group-preview generic-group-preview",
                div { class: "group-preview-title", "{text}" }
                div { class: "nested-preview",
                    for child in children {
                        {render_preview_tree(child, all_blocks.clone())}
                    }
                }
            }
        },
        BlockKind::Columns => {
            let count = children
                .iter()
                .map(|child| child.column as usize + 1)
                .max()
                .unwrap_or(2)
                .clamp(2, 3);
            rsx! {
                section { id: "{anchor}", class: "columns-preview",
                    if !text.trim().is_empty() {
                        div { class: "group-preview-title", "{text}" }
                    }
                    div { class: "columns-preview-grid",
                        for column in 0..count {
                            div { class: "preview-column",
                                for child in children.iter().filter(|child| child.column as usize == column) {
                                    {render_preview_tree(child.clone(), all_blocks.clone())}
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => render_preview_block(block),
    }
}

fn render_preview_block(block: Block) -> Element {
    let id = block.id;
    let anchor = format!("block-{id}");
    let text = block.text;
    match block.kind {
        BlockKind::HeadingOne => rsx! {
            h1 { id: "{anchor}", "{text}" a { class: "anchor-link", href: "#{anchor}", " # " } }
        },
        BlockKind::HeadingTwo => rsx! {
            h2 { id: "{anchor}", "{text}" a { class: "anchor-link", href: "#{anchor}", " # " } }
        },
        BlockKind::HeadingThree => rsx! {
            h3 { id: "{anchor}", "{text}" a { class: "anchor-link", href: "#{anchor}", " # " } }
        },
        BlockKind::Bullet => rsx! {
            div { id: "{anchor}", class: "preview-list", span { class: "list-marker", "•" } span { "{text}" } }
        },
        BlockKind::Numbered => rsx! {
            div { id: "{anchor}", class: "preview-list", span { class: "list-marker", "1." } span { "{text}" } }
        },
        BlockKind::Checklist => rsx! {
            div { id: "{anchor}", class: "preview-list",
                span { class: "list-marker", if block.checked { "✓" } else { "○" } }
                span { class: if block.checked { "checked" } else { "" }, "{text}" }
            }
        },
        BlockKind::Code => rsx! { pre { id: "{anchor}", code { "{text}" } } },
        BlockKind::Quote => rsx! { blockquote { id: "{anchor}", "{text}" } },
        BlockKind::Callout => rsx! {
            div { id: "{anchor}", class: "callout-preview",
                strong { "✦" }
                span { "{text}" }
            }
        },
        BlockKind::ReviewComment => {
            let (quotes, body) = review_comment_parts(&text);
            rsx! {
                aside { id: "{anchor}", class: "review-comment-preview",
                    span { class: "review-comment-label", "指摘" }
                    if !quotes.is_empty() {
                        for (label, quote) in quotes {
                            div { class: "review-quote-preview",
                                span { class: "review-quote-label", "{label}" }
                                blockquote { "{quote}" }
                            }
                        }
                    }
                    span { "{body}" }
                }
            }
        },
        BlockKind::Mermaid => {
            let source = text.clone();
            rsx! {
                div {
                    id: "{anchor}",
                    key: "{source}",
                    class: "mermaid-preview",
                    onmounted: move |_| render_mermaid_when_ready(id, &source),
                }
            }
        }
        BlockKind::Toggle => {
            let (summary, body) = toggle_parts(&text);
            rsx! {
                details { id: "{anchor}",
                    summary { "{summary}" }
                    div { "{body}" }
                }
            }
        }
        BlockKind::Divider => rsx! { hr { id: "{anchor}" } },
        BlockKind::Table => rsx! {
            table { id: "{anchor}", class: "preview-table",
                tbody {
                    for row in table_rows(&text) {
                        tr {
                            for cell in row {
                                td { "{cell}" }
                            }
                        }
                    }
                }
            }
        },
        BlockKind::Image => rsx! {
            img { id: "{anchor}", class: "embed", src: "{text}", alt: "画像の埋め込み" }
        },
        BlockKind::Video => rsx! {
            video { id: "{anchor}", class: "embed", controls: true, src: "{text}", "このブラウザでは動画を再生できません。" }
        },
        BlockKind::Link => {
            let (label, url) = link_parts(&text);
            rsx! {
                p { id: "{anchor}", class: "link-preview",
                    "↗ "
                    a { href: "{url}", target: "_blank", rel: "noreferrer", "{label}" }
                }
            }
        }
        BlockKind::Figure => {
            let (url, caption) = figure_parts(&text);
            rsx! {
                figure { id: "{anchor}", class: "figure-preview",
                    img { class: "embed", src: "{url}", alt: "{caption}" }
                    if !caption.is_empty() {
                        figcaption { "{caption}" }
                    }
                }
            }
        }
        BlockKind::Semantic => {
            let (tag, content) = semantic_parts(&text);
            let tag_label = format!("<{tag}>");
            rsx! {
                div { id: "{anchor}", class: "semantic-preview",
                    span { class: "semantic-tag", "{tag_label}" }
                    span { "{content}" }
                }
            }
        }
        BlockKind::Group | BlockKind::Columns => rsx! { div { id: "{anchor}" } },
        BlockKind::Paragraph => rsx! { p { id: "{anchor}", "{text}" } },
    }
}

fn handle_keydown(
    event: KeyboardEvent,
    id: u32,
    index: usize,
    mut document: Signal<DocumentState>,
    mut slash_for: Signal<Option<u32>>,
    mut slash_index: Signal<usize>,
    mut notice: Signal<String>,
    review: ReviewUi,
) {
    if event.is_composing() {
        return;
    }
    let key = event.key().to_string();
    let modifiers = event.modifiers();
    let command = modifiers.ctrl() || modifiers.meta();

    if slash_for() == Some(id) {
        let query = document()
            .blocks
            .iter()
            .find(|block| block.id == id)
            .map(|block| block.text.clone())
            .unwrap_or_default();
        let options = slash_options(&query);
        match key.as_str() {
            "ArrowDown" if !options.is_empty() => {
                event.prevent_default();
                slash_index.set((slash_index() + 1) % options.len());
                return;
            }
            "ArrowUp" if !options.is_empty() => {
                event.prevent_default();
                slash_index.set((slash_index() + options.len() - 1) % options.len());
                return;
            }
            "Enter" if !options.is_empty() => {
                event.prevent_default();
                let kind = options[slash_index() % options.len()];
                document.with_mut(|doc| convert_block(doc, id, kind));
                slash_for.set(None);
                if kind == BlockKind::Mermaid {
                    render_mermaid_when_ready(id, "flowchart LR\n  A[開始] --> B[完了]");
                }
                notice.set(format!("{} ブロックを追加しました", kind.label()));
                focus_editor_block(id);
                return;
            }
            "Escape" => {
                event.prevent_default();
                document.with_mut(|doc| {
                    if let Some(block) = doc.blocks.iter_mut().find(|block| block.id == id) {
                        block.text.clear();
                    }
                });
                slash_for.set(None);
                notice.set("スラッシュメニューを閉じました".to_string());
                return;
            }
            _ => {}
        }
    }

    if command && key.eq_ignore_ascii_case("b") {
        event.prevent_default();
        wrap_selected_text(document, id, "**");
        notice.set("太字を適用しました".to_string());
        return;
    }
    if command && key.eq_ignore_ascii_case("i") {
        event.prevent_default();
        wrap_selected_text(document, id, "_");
        notice.set("斜体を適用しました".to_string());
        return;
    }
    if command && modifiers.shift() && key.eq_ignore_ascii_case("r") {
        event.prevent_default();
        if (review.composing)() == Some(id) {
            return;
        }
        let selected = current_block_selection(id);
        if selected.is_some() {
            adopt_text_selection(document, review, notice, id, (review.append)());
        } else {
            adopt_whole_block(document, review, notice, id, (review.append)());
        }
        return;
    }
    if key == "Tab" {
        event.prevent_default();
        let changed = document.with_mut(|doc| {
            if modifiers.shift() {
                outdent_block(doc, id)
            } else {
                indent_block(doc, id)
            }
        });
        notice.set(if changed {
            if modifiers.shift() {
                "グループの外へ移動しました".to_string()
            } else {
                "直前のグループへ入れました".to_string()
            }
        } else if modifiers.shift() {
            "これ以上外へ移動できません".to_string()
        } else {
            "直前のトグル・コールアウト・グループの後で Tab を押してください".to_string()
        });
        return;
    }
    if modifiers.alt() && (key == "ArrowUp" || key == "ArrowDown") {
        event.prevent_default();
        let offset: isize = if key == "ArrowUp" { -1 } else { 1 };
        document.with_mut(|doc| {
            let destination = index as isize + offset;
            if destination >= 0 && destination < doc.blocks.len() as isize {
                doc.blocks.swap(index, destination as usize);
            }
        });
        notice.set("ブロックを移動しました".to_string());
        return;
    }
    if key == "Enter" && !modifiers.shift() {
        event.prevent_default();
        let mut document = document;
        let new_id = document.with_mut(|doc| {
            let placement = doc
                .blocks
                .get(index)
                .map(|block| (block.parent, block.column))
                .unwrap_or((None, 0));
            let new_id = doc.insert_after(index, BlockKind::Paragraph);
            if let Some(block) = doc.blocks.iter_mut().find(|block| block.id == new_id) {
                block.parent = placement.0;
                block.column = placement.1;
            }
            new_id
        });
        slash_for.set(None);
        focus_editor_block(new_id);
        return;
    }
    if key == "Backspace" {
        let empty = document()
            .blocks
            .iter()
            .find(|block| block.id == id)
            .is_some_and(|block| block.text.is_empty());
        if empty {
            event.prevent_default();
            document.with_mut(|doc| {
                if doc.blocks.len() > 1 {
                    let mut removed = Vec::new();
                    collect_subtree_ids(&doc.blocks, id, &mut removed);
                    doc.blocks.retain(|block| !removed.contains(&block.id));
                } else if let Some(block) = doc.blocks.first_mut() {
                    block.kind = BlockKind::Paragraph;
                }
            });
            notice.set("空のブロックを削除しました".to_string());
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn wrap_selected_text(_document: Signal<DocumentState>, id: u32, marker: &str) {
    wrap_selection(id, marker);
}

#[cfg(not(target_arch = "wasm32"))]
fn wrap_selected_text(mut document: Signal<DocumentState>, id: u32, marker: &str) {
    document.with_mut(|doc| {
        if let Some(block) = doc.blocks.iter_mut().find(|block| block.id == id) {
            block.text = format!("{marker}{}{marker}", block.text);
        }
    });
}

fn current_block_selection(id: u32) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let raw = read_block_selection(id);
        let mut parts = raw.split('\u{0000}');
        let _start = parts.next()?;
        let _end = parts.next()?;
        let quoted = parts.next().unwrap_or("");
        if quoted.is_empty() {
            None
        } else {
            Some(quoted.to_string())
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        None
    }
}

fn push_review_target(mut review: ReviewUi, target: ReviewTarget, append: bool) {
    let mut targets = if append {
        (review.targets)()
    } else {
        Vec::new()
    };
    targets.retain(|existing| {
        !(existing.block_id == target.block_id && existing.quoted == target.quoted)
    });
    targets.push(target);
    review.targets.set(targets);
    review.open.set(true);
    review.error.set(None);
    review.copied.set(None);
}

fn adopt_text_selection(
    document: Signal<DocumentState>,
    review: ReviewUi,
    mut notice: Signal<String>,
    id: u32,
    shift: bool,
) {
    if (review.composing)() == Some(id) {
        return;
    }
    let Some(quoted) = current_block_selection(id) else {
        return;
    };
    let Some(block) = document()
        .blocks
        .iter()
        .find(|block| block.id == id)
        .cloned()
    else {
        return;
    };
    let append = shift || (review.append)();
    let whole_block = quoted == block.text;
    push_review_target(
        review,
        ReviewTarget {
            block_id: block.id,
            kind: block.kind,
            quoted,
            whole_block,
        },
        append,
    );
    notice.set("選択範囲をレビュー対象にしました".to_string());
}

fn adopt_whole_block(
    document: Signal<DocumentState>,
    mut review: ReviewUi,
    mut notice: Signal<String>,
    id: u32,
    shift: bool,
) {
    if (review.composing)() == Some(id) {
        return;
    }
    let Some(block) = document()
        .blocks
        .iter()
        .find(|block| block.id == id)
        .cloned()
    else {
        return;
    };
    if block.text.trim().is_empty() {
        review.open.set(true);
        review.error.set(Some(
            "このブロックは空です。本文を書いてからレビューしてください。".to_string(),
        ));
        notice.set("空のブロックはレビューできません".to_string());
        return;
    }
    let append = shift || (review.append)();
    push_review_target(
        review,
        ReviewTarget {
            block_id: block.id,
            kind: block.kind,
            quoted: block.text.clone(),
            whole_block: true,
        },
        append,
    );
    notice.set(format!(
        "#{} {} をレビュー対象にしました",
        block.id,
        block.kind.label()
    ));
}

fn generate_review_draft(kind: BlockKind, quoted: &str) -> String {
    let trimmed = quoted.trim();
    if trimmed.is_empty() {
        return "対象が空です。直してほしい箇所と、直したあとの状態を一文で書いてください。"
            .to_string();
    }
    let mut points = Vec::new();
    match kind {
        BlockKind::HeadingOne | BlockKind::HeadingTwo | BlockKind::HeadingThree => {
            points.push(
                "この見出しは節の結論になっていますか。読者が次に判断すべきことを補ってください。"
                    .to_string(),
            );
        }
        BlockKind::Checklist => {
            points.push(
                "完了条件は検証できますか。「できた」ではなく、観測できる状態にしてください。"
                    .to_string(),
            );
        }
        BlockKind::Mermaid => {
            points.push("分岐の漏れと、失敗したときの戻り先を確認してください。".to_string());
        }
        BlockKind::Table => {
            points.push(
                "判断列に数値・期限・担当のどれかがありますか。無い行は根拠を足してください。"
                    .to_string(),
            );
        }
        BlockKind::Quote => {
            points.push(
                "引用は主張の根拠ですか。出典または「誰の判断か」を明示してください。".to_string(),
            );
        }
        BlockKind::Code => {
            points.push(
                "このコードが失敗する入力と、そのときの表示を一文で書いてください。".to_string(),
            );
        }
        BlockKind::ReviewComment => {
            points.push(
                "指摘が依頼になっていません。直す場所と、直したあとの文面を指定してください。"
                    .to_string(),
            );
        }
        BlockKind::Columns | BlockKind::Group => {
            points.push(
                "比較の軸が揃っていますか。違う軸を並べている列は分けてください。".to_string(),
            );
        }
        _ => {
            points.push(
                "この箇所の次の行動は何か、利用者の言葉で書いてください。".to_string(),
            );
        }
    }
    if trimmed.chars().count() > 72 {
        points.push(
            "一文が長いです。結論を先に、根拠と例外を後ろへ分けてください。".to_string(),
        );
    }
    if matches!(kind, BlockKind::Paragraph | BlockKind::Quote) && !trimmed.contains('。') {
        points.push(
            "句点がなく、観察と依頼が混ざっています。事実と、直してほしい点を分けてください。"
                .to_string(),
        );
    }
    points.push("引用した箇所だけを直し、変更後の Markdown を返してください。".to_string());
    points.join("\n")
}

fn generate_review_drafts(targets: &[ReviewTarget]) -> String {
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let header = if targets.len() > 1 {
                format!(
                    "## 対象 {}（#{} {}）\n",
                    index + 1,
                    target.block_id,
                    target.kind.label()
                )
            } else {
                String::new()
            };
            format!(
                "{}{}",
                header,
                generate_review_draft(target.kind, &target.quoted)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn json_escape(value: &str) -> String {
    let mut out = String::from('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn target_scope_label(target: &ReviewTarget) -> &'static str {
    if target.whole_block {
        "ブロック全体"
    } else {
        "部分選択"
    }
}

fn build_llm_prompt(doc: &DocumentState, targets: &[ReviewTarget], review: &str) -> String {
    let mut output = String::from(
        "以下は Markdown ブロックエディタからのレビューです。引用箇所を直し、指摘に答えてください。\n\n",
    );
    output.push_str(&format!(
        "# 文書\n- ファイル: {}\n- タイトル: {}\n\n# 対象\n",
        doc.file_name, doc.title
    ));
    if targets.is_empty() {
        output.push_str("（対象が選ばれていません）\n");
    }
    for target in targets {
        output.push_str(&format!(
            "## #{} {}（{}）\n",
            target.block_id,
            target.kind.label(),
            target_scope_label(target)
        ));
        for line in target.quoted.lines() {
            output.push_str(&format!("> {line}\n"));
        }
        output.push('\n');
    }
    output.push_str("# 指摘\n");
    output.push_str(review.trim());
    output.push_str(
        "\n\n# お願い\n1. 引用した箇所だけを直す\n2. 指摘への対応を短く説明する\n3. 変更後の Markdown を返す\n",
    );
    output
}

fn build_llm_json(doc: &DocumentState, targets: &[ReviewTarget], review: &str) -> String {
    let mut output = String::from("{\n");
    output.push_str("  \"source\": \"md-block-editor\",\n");
    output.push_str(&format!("  \"file\": {},\n", json_escape(&doc.file_name)));
    output.push_str(&format!("  \"title\": {},\n", json_escape(&doc.title)));
    output.push_str("  \"targets\": [\n");
    for (index, target) in targets.iter().enumerate() {
        output.push_str("    {\n");
        output.push_str(&format!("      \"block_id\": {},\n", target.block_id));
        output.push_str(&format!(
            "      \"kind\": {},\n",
            json_escape(target.kind.label())
        ));
        output.push_str(&format!(
            "      \"quoted\": {},\n",
            json_escape(&target.quoted)
        ));
        output.push_str(&format!(
            "      \"whole_block\": {}\n",
            if target.whole_block { "true" } else { "false" }
        ));
        output.push_str("    }");
        if index + 1 != targets.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("  ],\n");
    output.push_str(&format!("  \"review\": {},\n", json_escape(review.trim())));
    output.push_str(&format!(
        "  \"instruction\": {}\n",
        json_escape("引用箇所を直し、指摘に答えてください。")
    ));
    output.push_str("}\n");
    output
}

fn format_review_comment_text(targets: &[ReviewTarget], body: &str) -> String {
    let mut lines = Vec::new();
    for target in targets {
        lines.push(format!("対象: #{} {}", target.block_id, target.kind.label()));
        if target.quoted.lines().count() <= 1 {
            lines.push(format!("引用: {}", target.quoted.replace('\n', " ")));
        } else {
            lines.push("引用:".to_string());
            lines.extend(target.quoted.lines().map(|line| format!("  {line}")));
        }
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(body.trim().to_string());
    lines.join("\n")
}

fn insert_review_comment(
    document: &mut DocumentState,
    targets: &[ReviewTarget],
    body: &str,
) -> Option<u32> {
    let last_id = targets.last()?.block_id;
    let index = document.blocks.iter().position(|block| block.id == last_id)?;
    let parent = document.blocks[index].parent;
    let column = document.blocks[index].column;
    let text = format_review_comment_text(targets, body);
    let id = document.insert_after(index, BlockKind::ReviewComment);
    if let Some(block) = document.blocks.iter_mut().find(|block| block.id == id) {
        block.text = text;
        block.parent = parent;
        block.column = column;
    }
    Some(id)
}

fn review_comment_parts(text: &str) -> (Vec<(String, String)>, String) {
    let mut quotes = Vec::new();
    let mut body_lines = Vec::new();
    let mut pending_label = None::<String>;
    let mut collecting_quote = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("対象:") {
            pending_label = Some(rest.trim().to_string());
            collecting_quote = false;
        } else if let Some(rest) = line.strip_prefix("引用:") {
            let label = pending_label.take().unwrap_or_else(|| "引用".to_string());
            let rest = rest.trim();
            if rest.is_empty() {
                quotes.push((label, String::new()));
                collecting_quote = true;
            } else {
                quotes.push((label, rest.to_string()));
                collecting_quote = false;
            }
        } else if collecting_quote && (line.starts_with("  ") || line.starts_with('\t')) {
            if let Some((_, quote)) = quotes.last_mut() {
                if !quote.is_empty() {
                    quote.push('\n');
                }
                quote.push_str(line.trim());
            }
        } else {
            collecting_quote = false;
            body_lines.push(line);
        }
    }
    let body = body_lines.join("\n").trim().to_string();
    if quotes.is_empty() && body.is_empty() {
        (quotes, text.trim().to_string())
    } else {
        (quotes, body)
    }
}

fn review_export_error(targets: &[ReviewTarget], body: &str) -> Option<String> {
    if targets.is_empty() {
        Some("対象がありません。本文をドラッグして選ぶか、ブロックの「レビュー」を押してください。".to_string())
    } else if body.trim().is_empty() {
        Some("指摘が空です。書いてからコピーするか、「下書きを生成」してください。".to_string())
    } else {
        None
    }
}

fn placeholder(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::HeadingOne => "文書の主題",
        BlockKind::HeadingTwo | BlockKind::HeadingThree => "セクションの見出し",
        BlockKind::Bullet | BlockKind::Numbered => "リスト項目",
        BlockKind::Checklist => "確認する項目",
        BlockKind::Code => "コードまたはコマンド",
        BlockKind::Quote => "参照したい発言や根拠",
        BlockKind::Table => "論点 | 詳細\n別の論点 | 補足",
        BlockKind::Image => "https://…/image.jpg",
        BlockKind::Video => "https://…/video.mp4",
        BlockKind::Callout => "共有したい補足",
        BlockKind::Toggle => "要約 | 詳細",
        BlockKind::Divider => "",
        BlockKind::Mermaid => "flowchart LR\n  A[開始] --> B[完了]",
        BlockKind::ReviewComment => "レビューで確認したい点",
        BlockKind::Link => "リンク名 | https://example.com",
        BlockKind::Figure => "https://…/diagram.png | 図の説明",
        BlockKind::Semantic => "mark | 強調したい設計上の注意",
        BlockKind::Group => "グループの見出し",
        BlockKind::Columns => "比較カラム",
        BlockKind::Paragraph => "入力するか、/ でブロックを選びます",
    }
}

fn output_name(doc: &DocumentState) -> String {
    let base = doc
        .file_name
        .rsplit_once('.')
        .map(|(base, _)| base)
        .filter(|base| !base.trim().is_empty())
        .unwrap_or_else(|| doc.title.trim());
    let clean = if base.is_empty() { "untitled" } else { base };
    format!("{clean}.{}", doc.source.extension())
}

fn serialize_document(doc: &DocumentState) -> String {
    match doc.source {
        SourceType::Markdown => serialize_markdown(doc),
        SourceType::Html => serialize_html(doc),
    }
}

fn serialize_markdown(doc: &DocumentState) -> String {
    let mut output = if doc
        .blocks
        .iter()
        .any(|block| block.parent.is_none() && block.kind == BlockKind::HeadingOne)
    {
        String::new()
    } else {
        format!("# {}\n\n", doc.title.trim())
    };
    for block in doc.blocks.iter().filter(|block| block.parent.is_none()) {
        output.push_str(&serialize_markdown_block(doc, block));
        output.push_str("\n\n");
    }
    output.trim_end().to_string() + "\n"
}

fn serialize_markdown_children(doc: &DocumentState, parent: u32) -> String {
    doc.blocks
        .iter()
        .filter(|block| block.parent == Some(parent))
        .map(|child| serialize_markdown_block(doc, child))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn serialize_markdown_block(doc: &DocumentState, block: &Block) -> String {
    let value = block.text.trim_end();
    let children = serialize_markdown_children(doc, block.id);
    match block.kind {
        BlockKind::HeadingOne => format!("# {value}"),
        BlockKind::HeadingTwo => format!("## {value}"),
        BlockKind::HeadingThree => format!("### {value}"),
        BlockKind::Bullet => format!("- {value}"),
        BlockKind::Numbered => format!("1. {value}"),
        BlockKind::Checklist => format!("- [{}] {value}", if block.checked { "x" } else { " " }),
        BlockKind::Code => format!("```\n{value}\n```"),
        BlockKind::Mermaid => format!("```mermaid\n{value}\n```"),
        BlockKind::Quote => format!("> {value}"),
        BlockKind::Table => table_markdown(value),
        BlockKind::Image => format!("![]({value})"),
        BlockKind::Video => format!("<video controls src=\"{}\"></video>", escape_html(value)),
        BlockKind::ReviewComment => {
            let mut lines = vec!["> [!COMMENT]".to_string()];
            if value.is_empty() {
                lines.push("> ".to_string());
            } else {
                for line in value.lines() {
                    lines.push(format!("> {line}"));
                }
            }
            lines.join("\n")
        }
        BlockKind::Toggle => {
            let (summary, legacy_body) = toggle_parts(value);
            let content = [legacy_body, children.trim()]
                .iter()
                .filter(|part| !part.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join("\n\n");
            format!(
                "<details data-block=\"toggle\">\n<summary>{}</summary>\n\n{}\n</details>",
                escape_html(summary),
                content
            )
        }
        BlockKind::Callout => format!(
            "<aside class=\"callout\" data-block=\"callout\">\n<p>{}</p>\n{}\n</aside>",
            escape_html(value),
            children
        ),
        BlockKind::Group => format!(
            "<section class=\"blockpad-group\" data-block=\"group\"><p class=\"group-title\">{}</p>\n{}\n</section>",
            escape_html(value),
            children
        ),
        BlockKind::Columns => {
            let count = direct_column_count(doc, block.id);
            let mut columns = Vec::new();
            for column in 0..count {
                let column_children = doc
                    .blocks
                    .iter()
                    .filter(|child| child.parent == Some(block.id) && child.column as usize == column)
                    .map(|child| serialize_markdown_block(doc, child))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                columns.push(format!(
                    "<div class=\"blockpad-column\" data-column=\"{column}\">\n{column_children}\n</div>"
                ));
            }
            format!(
                "<section class=\"blockpad-columns\" data-block=\"columns\">\n<p class=\"group-title\">{}</p>\n{}\n</section>",
                escape_html(value),
                columns.join("\n")
            )
        }
        BlockKind::Divider => "---".to_string(),
        BlockKind::Link => {
            let (label, url) = link_parts(value);
            format!("[{label}]({url})")
        }
        BlockKind::Figure => {
            let (url, caption) = figure_parts(value);
            format!("![{caption}]({url})")
        }
        BlockKind::Semantic => {
            let (tag, content) = semantic_parts(value);
            format!("<{tag}>{content}</{tag}>")
        }
        BlockKind::Paragraph => value.to_string(),
    }
}

fn serialize_html(doc: &DocumentState) -> String {
    let mut body = String::new();
    for block in doc.blocks.iter().filter(|block| block.parent.is_none()) {
        body.push_str("    ");
        body.push_str(&serialize_html_block(doc, block));
        body.push('\n');
    }
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(&doc.title),
        body.trim_end()
    )
}

fn serialize_html_children(doc: &DocumentState, parent: u32) -> String {
    doc.blocks
        .iter()
        .filter(|block| block.parent == Some(parent))
        .map(|child| serialize_html_block(doc, child))
        .collect::<Vec<_>>()
        .join("\n")
}

fn serialize_html_block(doc: &DocumentState, block: &Block) -> String {
    let id = format!("block-{}", block.id);
    let value = escape_html(block.text.trim_end());
    let children = serialize_html_children(doc, block.id);
    match block.kind {
        BlockKind::HeadingOne => format!("<h1 id=\"{id}\">{value}</h1>"),
        BlockKind::HeadingTwo => format!("<h2 id=\"{id}\">{value}</h2>"),
        BlockKind::HeadingThree => format!("<h3 id=\"{id}\">{value}</h3>"),
        BlockKind::Paragraph => format!("<p id=\"{id}\">{value}</p>"),
        BlockKind::Bullet => format!("<ul id=\"{id}\"><li>{value}</li></ul>"),
        BlockKind::Numbered => format!("<ol id=\"{id}\"><li>{value}</li></ol>"),
        BlockKind::Checklist => format!(
            "<p id=\"{id}\"><input type=\"checkbox\" disabled {}> {value}</p>",
            if block.checked { "checked" } else { "" }
        ),
        BlockKind::Code => format!("<pre id=\"{id}\"><code>{value}</code></pre>"),
        BlockKind::Mermaid => {
            format!("<pre id=\"{id}\" class=\"mermaid\" data-block=\"mermaid\">{value}</pre>")
        }
        BlockKind::Quote => format!("<blockquote id=\"{id}\">{value}</blockquote>"),
        BlockKind::Image => format!("<img id=\"{id}\" src=\"{value}\" alt=\"画像の埋め込み\">"),
        BlockKind::Video => format!("<video id=\"{id}\" controls src=\"{value}\"></video>"),
        BlockKind::Callout => {
            format!("<aside id=\"{id}\" class=\"callout\" data-block=\"callout\"><p>{value}</p>{children}</aside>")
        }
        BlockKind::ReviewComment => {
            format!("<aside id=\"{id}\" class=\"review-comment\" data-block=\"comment\">{value}</aside>")
        }
        BlockKind::Toggle => {
            let (summary, detail) = toggle_parts(&block.text);
            format!("<details id=\"{id}\" data-block=\"toggle\"><summary>{}</summary><p>{}</p>{children}</details>", escape_html(summary), escape_html(detail))
        }
        BlockKind::Divider => format!("<hr id=\"{id}\">"),
        BlockKind::Table => table_html(&id, &block.text),
        BlockKind::Link => {
            let (label, url) = link_parts(&block.text);
            format!(
                "<p id=\"{id}\"><a href=\"{}\">{}</a></p>",
                escape_html(url),
                escape_html(label)
            )
        }
        BlockKind::Figure => {
            let (url, caption) = figure_parts(&block.text);
            format!(
                    "<figure id=\"{id}\"><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>",
                    escape_html(url),
                    escape_html(caption),
                    escape_html(caption)
                )
        }
        BlockKind::Semantic => {
            let (tag, content) = semantic_parts(&block.text);
            format!("<{tag} id=\"{id}\">{}</{tag}>", escape_html(content))
        }
        BlockKind::Group => {
            format!("<section id=\"{id}\" class=\"blockpad-group\" data-block=\"group\"><p class=\"group-title\">{value}</p>{children}</section>")
        }
        BlockKind::Columns => {
            let count = direct_column_count(doc, block.id);
            let columns = (0..count)
                    .map(|column| {
                        let column_children = doc
                            .blocks
                            .iter()
                            .filter(|child| child.parent == Some(block.id) && child.column as usize == column)
                            .map(|child| serialize_html_block(doc, child))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("<div class=\"blockpad-column\" data-column=\"{column}\">{column_children}</div>")
                    })
                    .collect::<String>();
            format!("<section id=\"{id}\" class=\"blockpad-columns\" data-block=\"columns\"><p class=\"group-title\">{value}</p>{columns}</section>")
        }
    }
}

fn direct_column_count(doc: &DocumentState, parent: u32) -> usize {
    doc.blocks
        .iter()
        .filter(|block| block.parent == Some(parent))
        .map(|block| block.column as usize + 1)
        .max()
        .unwrap_or(2)
        .clamp(2, 3)
}

