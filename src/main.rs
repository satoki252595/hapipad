use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const STYLE: Asset = asset!("/assets/style.css");

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r##"
let blockpadFileHandle = null;

const fileTypes = [{
  description: "Blockpad 文書",
  accept: {
    "text/markdown": [".md", ".markdown"],
    "text/html": [".html", ".htm"],
  },
}];

export async function open_local_file() {
  if (window.showOpenFilePicker) {
    try {
      const [handle] = await window.showOpenFilePicker({ types: fileTypes, multiple: false });
      const file = await handle.getFile();
      blockpadFileHandle = handle;
      const source = /\.(html?|htm)$/i.test(file.name) ? "html" : "markdown";
      return `${file.name}\u0000${source}\u0000${await file.text()}`;
    } catch (error) {
      if (error && error.name === "AbortError") return "";
      throw error;
    }
  }

  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".md,.markdown,.html,.htm,text/markdown,text/html";
    input.onchange = async () => {
      const file = input.files && input.files[0];
      if (!file) return resolve("");
      const source = /\.(html?|htm)$/i.test(file.name) ? "html" : "markdown";
      resolve(`${file.name}\u0000${source}\u0000${await file.text()}`);
    };
    input.click();
  });
}

export async function save_local_file(name, content, mime) {
  try {
    if (blockpadFileHandle) {
      const writable = await blockpadFileHandle.createWritable();
      await writable.write(content);
      await writable.close();
      return "original";
    }

    if (window.showSaveFilePicker) {
      const handle = await window.showSaveFilePicker({
        suggestedName: name,
        types: [{
          description: mime === "text/html" ? "HTML 文書" : "Markdown 文書",
          accept: { [mime]: [mime === "text/html" ? ".html" : ".md"] },
        }],
      });
      const writable = await handle.createWritable();
      await writable.write(content);
      await writable.close();
      blockpadFileHandle = handle;
      return "picker";
    }
  } catch (error) {
    if (error && error.name === "AbortError") return "cancelled";
  }

  const blob = new Blob([content], { type: `${mime};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 300);
  return "downloaded";
}

export function apply_theme(isDark) {
  document.documentElement.dataset.theme = isDark ? "dark" : "light";
  document.querySelector('meta[name="theme-color"]').content = isDark ? "#171817" : "#faf9f6";
  document.querySelectorAll("[data-mermaid-source]").forEach((node) => {
    render_mermaid_block(node.id, node.dataset.mermaidSource || "");
  });
}

export function focus_block(id) {
  setTimeout(() => document.getElementById(`editor-block-${id}`)?.focus(), 0);
}

export function wrap_selection(id, marker) {
  const input = document.getElementById(`editor-block-${id}`);
  if (!input) return;
  const start = input.selectionStart ?? 0;
  const end = input.selectionEnd ?? start;
  const selected = input.value.slice(start, end);
  input.value = input.value.slice(0, start) + marker + selected + marker + input.value.slice(end);
  input.setSelectionRange(start + marker.length, end + marker.length);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

export function visit_anchor(id) {
  window.location.hash = id;
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "center" });
}

let mermaidApi = null;
let mermaidTheme = null;

export function render_mermaid_block(id, source) {
  window.setTimeout(async () => {
    const host = document.getElementById(id);
    if (!host) return;
    const scrollY = window.scrollY;
    try {
      host.dataset.mermaidSource = source;
      if (!mermaidApi) {
        mermaidApi = (await import("https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs")).default;
      }
      const theme = document.documentElement.dataset.theme === "dark" ? "dark" : "default";
      if (mermaidTheme !== theme) {
        mermaidApi.initialize({ startOnLoad: false, theme, securityLevel: "strict" });
        mermaidTheme = theme;
      }
      const result = await mermaidApi.render(`mermaid-svg-${id}`, source || "flowchart LR\n  A[図を追加] --> B[プレビュー]");
      host.replaceChildren();
      host.insertAdjacentHTML("afterbegin", result.svg);
      result.bindFunctions?.(host);
      window.requestAnimationFrame(() => window.scrollTo(0, scrollY));
    } catch (error) {
      host.replaceChildren();
      const fallback = document.createElement("pre");
      fallback.className = "mermaid-error";
      fallback.textContent = `Mermaid を描画できません: ${error?.message || error}\n\n${source}`;
      host.appendChild(fallback);
    }
  }, 0);
}

export function find_in_page(query) {
  if (query.trim()) window.find(query, false, false, true, false, false, false);
}
"##)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn open_local_file() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn save_local_file(name: &str, content: &str, mime: &str) -> Result<JsValue, JsValue>;
    fn apply_theme(is_dark: bool);
    fn focus_block(id: u32);
    fn wrap_selection(id: u32, marker: &str);
    fn visit_anchor(id: &str);
    fn render_mermaid_block(id: &str, source: &str);
    fn find_in_page(query: &str);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceType {
    Markdown,
    Html,
}

impl SourceType {
    fn label(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Html => "html",
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Self::Markdown => "text/markdown",
            Self::Html => "text/html",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    Paragraph,
    HeadingOne,
    HeadingTwo,
    HeadingThree,
    Bullet,
    Numbered,
    Checklist,
    Code,
    Quote,
    Table,
    Image,
    Video,
    Callout,
    Toggle,
    Divider,
    Mermaid,
    ReviewComment,
    Link,
    Figure,
    Semantic,
    Group,
    Columns,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MobileView {
    Editor,
    Preview,
}

impl BlockKind {
    const MENU: [(Self, &'static str); 22] = [
        (Self::Paragraph, "テキスト"),
        (Self::HeadingOne, "見出し 1"),
        (Self::HeadingTwo, "見出し 2"),
        (Self::HeadingThree, "見出し 3"),
        (Self::Bullet, "箇条書き"),
        (Self::Numbered, "番号付きリスト"),
        (Self::Checklist, "チェックリスト"),
        (Self::Code, "コード"),
        (Self::Quote, "引用"),
        (Self::Table, "表"),
        (Self::Image, "画像 URL"),
        (Self::Video, "動画 URL"),
        (Self::Callout, "コールアウト"),
        (Self::Toggle, "折りたたみ"),
        (Self::Divider, "区切り線"),
        (Self::Mermaid, "Mermaid 図"),
        (Self::ReviewComment, "指摘"),
        (Self::Link, "リンク"),
        (Self::Figure, "図版"),
        (Self::Semantic, "HTML 要素"),
        (Self::Group, "グループ"),
        (Self::Columns, "カラム"),
    ];

    fn label(self) -> &'static str {
        Self::MENU
            .iter()
            .find(|(kind, _)| *kind == self)
            .map(|(_, label)| *label)
            .unwrap_or("テキスト")
    }

    fn class_name(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::HeadingOne => "heading-one",
            Self::HeadingTwo => "heading-two",
            Self::HeadingThree => "heading-three",
            Self::Bullet => "bullet",
            Self::Numbered => "numbered",
            Self::Checklist => "checklist",
            Self::Code => "code",
            Self::Quote => "quote",
            Self::Table => "table",
            Self::Image => "image",
            Self::Video => "video",
            Self::Callout => "callout",
            Self::Toggle => "toggle",
            Self::Divider => "divider",
            Self::Mermaid => "mermaid",
            Self::ReviewComment => "review-comment",
            Self::Link => "link",
            Self::Figure => "figure",
            Self::Semantic => "semantic",
            Self::Group => "group",
            Self::Columns => "columns",
        }
    }

    fn from_label(value: &str) -> Self {
        Self::MENU
            .iter()
            .find(|(_, label)| *label == value)
            .map(|(kind, _)| *kind)
            .unwrap_or(Self::Paragraph)
    }

    fn aliases(self) -> &'static str {
        match self {
            Self::Paragraph => "text paragraph 本文 文章",
            Self::HeadingOne => "h1 heading title 大見出し",
            Self::HeadingTwo => "h2 heading section 中見出し",
            Self::HeadingThree => "h3 heading 小見出し",
            Self::Bullet => "bullet list 箇条書き リスト",
            Self::Numbered => "number list 番号",
            Self::Checklist => "task todo checkbox タスク チェック",
            Self::Code => "code pre コード",
            Self::Quote => "quote 引用",
            Self::Table => "table 表 テーブル",
            Self::Image => "image img 画像",
            Self::Video => "video 動画",
            Self::Callout => "note callout 補足",
            Self::Toggle => "toggle details 折りたたみ",
            Self::Divider => "divider hr 区切り",
            Self::Mermaid => "mermaid diagram flowchart 図 フローチャート",
            Self::ReviewComment => "comment review 指摘 コメント レビュー",
            Self::Link => "link anchor リンク",
            Self::Figure => "figure 図版",
            Self::Semantic => "html semantic 要素",
            Self::Group => "group container グループ 入れ物",
            Self::Columns => "columns column カラム 列",
        }
    }

    fn matches_query(self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || self.label().to_lowercase().contains(&query)
            || self.aliases().to_lowercase().contains(&query)
    }

    fn can_have_children(self) -> bool {
        matches!(
            self,
            Self::Toggle | Self::Callout | Self::Group | Self::Columns
        )
    }
}

#[derive(Clone, PartialEq)]
struct Block {
    id: u32,
    kind: BlockKind,
    text: String,
    checked: bool,
    parent: Option<u32>,
    column: u8,
}

#[derive(Clone, PartialEq)]
struct DocumentState {
    title: String,
    file_name: String,
    source: SourceType,
    blocks: Vec<Block>,
    next_id: u32,
}

impl DocumentState {
    fn starter() -> Self {
        Self {
            title: "設計レビュー：オンボーディング改善".to_string(),
            file_name: "design-review.md".to_string(),
            source: SourceType::Markdown,
            next_id: 19,
            blocks: vec![
                Block {
                    id: 1,
                    kind: BlockKind::HeadingOne,
                    text: "オンボーディング改善案".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 2,
                    kind: BlockKind::Paragraph,
                    text: "初回体験を短くし、利用者が最初の価値に到達するまでの手順を明確にします。".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 3,
                    kind: BlockKind::ReviewComment,
                    text: "確認：モバイルでの招待フローも同じ導線で検証してください。".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 4,
                    kind: BlockKind::HeadingTwo,
                    text: "提案フロー".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 5,
                    kind: BlockKind::Checklist,
                    text: "目的と成功指標を最初に示す".to_string(),
                    checked: true,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 6,
                    kind: BlockKind::Checklist,
                    text: "招待の完了後に次のアクションを提案する".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 7,
                    kind: BlockKind::Mermaid,
                    text: "flowchart LR\n  A[招待を受け取る] --> B{既存ユーザー?}\n  B -->|はい| C[ワークスペースを開く]\n  B -->|いいえ| D[プロフィールを設定]\n  D --> C".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 8,
                    kind: BlockKind::Table,
                    text: "論点 | 判断\n完了率 | 90% 以上を目標\n計測 | 招待から初回操作まで".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 9,
                    kind: BlockKind::Quote,
                    text: "レビューでは、実装の詳細よりも利用者の意思決定を見失わないことが重要です。".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 10,
                    kind: BlockKind::HeadingTwo,
                    text: "レビュー観点".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 11,
                    kind: BlockKind::Paragraph,
                    text: "各ブロックの ↗ から、プレビュー内の固定アンカーへ移動できます。".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 12,
                    kind: BlockKind::Link,
                    text: "仕様書（Figma） | https://www.figma.com/".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 13,
                    kind: BlockKind::Toggle,
                    text: "未解決の問い".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 14,
                    kind: BlockKind::Paragraph,
                    text: "完了率を上げるために、どの説明を削るべきか？".to_string(),
                    checked: false,
                    parent: Some(13),
                    column: 0,
                },
                Block {
                    id: 15,
                    kind: BlockKind::Columns,
                    text: "比較案".to_string(),
                    checked: false,
                    parent: None,
                    column: 0,
                },
                Block {
                    id: 16,
                    kind: BlockKind::Paragraph,
                    text: "案 A：最初に価値を説明する".to_string(),
                    checked: false,
                    parent: Some(15),
                    column: 0,
                },
                Block {
                    id: 17,
                    kind: BlockKind::Paragraph,
                    text: "案 B：先に操作を始めてもらう".to_string(),
                    checked: false,
                    parent: Some(15),
                    column: 1,
                },
                Block {
                    id: 18,
                    kind: BlockKind::ReviewComment,
                    text: "比較対象は最大 3 案までに絞る".to_string(),
                    checked: false,
                    parent: Some(15),
                    column: 2,
                },
            ],
        }
    }

    fn insert_after(&mut self, index: usize, kind: BlockKind) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.insert(
            index + 1,
            Block {
                id,
                kind,
                text: String::new(),
                checked: false,
                parent: None,
                column: 0,
            },
        );
        id
    }

    fn add_child(&mut self, parent: u32, column: u8) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(Block {
            id,
            kind: BlockKind::Paragraph,
            text: String::new(),
            checked: false,
            parent: Some(parent),
            column,
        });
        id
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut document = use_signal(DocumentState::starter);
    let mut dark_mode = use_signal(|| false);
    let slash_for = use_signal(|| None::<u32>);
    let slash_index = use_signal(|| 0usize);
    let dragged = use_signal(|| None::<usize>);
    let drop_target = use_signal(|| None::<usize>);
    let mut notice = use_signal(|| "ローカルの .md または .html を開けます".to_string());
    let mut find_query = use_signal(String::new);
    let mut find_index = use_signal(|| 0usize);
    let mut active_outline = use_signal(|| None::<u32>);
    let is_loading = use_signal(|| false);
    let file_error = use_signal(|| None::<String>);
    let mut mobile_view = use_signal(|| MobileView::Editor);

    let current = document();
    let preview_blocks = current.blocks.clone();
    let editor_blocks = current.blocks.clone();
    let preview_is_empty = preview_blocks.iter().all(|block| {
        block.text.trim().is_empty()
            && matches!(block.kind, BlockKind::Paragraph | BlockKind::Divider)
    });
    let source = current.source;
    let file_name = current.file_name.clone();
    let title = current.title.clone();
    let status = notice();
    let is_dark = dark_mode();
    let find_matches = find_match_ids(&current, &find_query());
    let active_find = find_matches
        .get(find_index() % find_matches.len().max(1))
        .copied();
    let shell_class = match mobile_view() {
        MobileView::Editor => "app-shell mobile-editor",
        MobileView::Preview => "app-shell mobile-preview",
    };
    let outline = current
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                BlockKind::HeadingOne | BlockKind::HeadingTwo | BlockKind::HeadingThree
            )
        })
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        document::Title { "Blockpad — ローカル設計レビュー" }
        document::Link { rel: "stylesheet", href: STYLE }
        main { class: "{shell_class}",
            header { class: "topbar",
                div { class: "brand",
                    div { class: "brand-mark", "B" }
                    span { "Blockpad" }
                    span { class: "brand-subtitle", "ブラウザだけで完結する設計レビュー" }
                }
                div { class: "top-actions",
                    button {
                        class: "button ghost",
                        onclick: move |_| {
                            let mut document = document;
                            let mut notice = notice;
                            let mut slash_for = slash_for;
                            let mut is_loading = is_loading;
                            let mut file_error = file_error;
                            is_loading.set(true);
                            file_error.set(None);
                            notice.set("ファイルを読み込んでいます…".to_string());
                            spawn(async move {
                                match load_from_browser().await {
                                    Ok(Some((name, source, content))) => {
                                        let parsed = parse_document(&name, source, &content);
                                        document.set(parsed);
                                        slash_for.set(None);
                                        notice.set(format!("{name} を開きました"));
                                    }
                                    Ok(None) => notice.set("開く操作を取り消しました".to_string()),
                                    Err(message) => {
                                        file_error.set(Some(message.clone()));
                                        notice.set(format!("開けませんでした：{message}"));
                                    }
                                }
                                is_loading.set(false);
                            });
                        },
                        "ファイルを開く"
                    }
                    button {
                        class: "icon-button",
                        title: "ダークモードを切り替える",
                        aria_label: "ダークモードを切り替える",
                        onclick: move |_| {
                            let next = !dark_mode();
                            dark_mode.set(next);
                            set_browser_theme(next);
                        },
                        if is_dark { "☀" } else { "☾" }
                    }
                    button {
                        class: "button primary",
                        onclick: move |_| {
                            let doc = document();
                            let mut notice = notice;
                            spawn(async move {
                                let output = serialize_document(&doc);
                                let name = output_name(&doc);
                                match save_to_browser(&name, &output, doc.source.mime()).await {
                                    Ok(message) => notice.set(save_status(&message)),
                                    Err(message) => notice.set(format!("保存できませんでした：{message}")),
                                }
                            });
                        },
                        "{source.label()} を保存"
                    }
                }
            }
            nav { class: "mobile-tabs", aria_label: "モバイル表示の切り替え",
                button {
                    class: if mobile_view() == MobileView::Editor { "mobile-tab active" } else { "mobile-tab" },
                    onclick: move |_| mobile_view.set(MobileView::Editor),
                    "編集"
                }
                button {
                    class: if mobile_view() == MobileView::Preview { "mobile-tab active" } else { "mobile-tab" },
                    onclick: move |_| mobile_view.set(MobileView::Preview),
                    "プレビュー"
                }
            }

            div { class: "workspace",
                section { class: "panel editor-panel",
                    div { class: "editor-header",
                        div { class: "document-meta",
                            span { class: "status", title: "{status}", "{status}" }
                            span { class: "source-badge", "{source.label()}" }
                        }
                        input {
                            class: "document-title",
                            value: "{title}",
                            aria_label: "文書タイトル",
                            oninput: move |event| {
                                let value = event.value();
                                document.with_mut(|doc| doc.title = value);
                            },
                        }
                        if is_loading() {
                            div { class: "file-feedback loading", "ファイルを読み込んでいます…" }
                        }
                        if let Some(error) = file_error() {
                            div { class: "file-feedback error",
                                strong { "ファイルを開けませんでした" }
                                span { "{error}" }
                            }
                        }
                    }
                    div { class: "toolbar",
                        for (kind, label) in [
                            (BlockKind::Paragraph, "テキスト"),
                            (BlockKind::HeadingOne, "見出し"),
                            (BlockKind::Checklist, "タスク"),
                            (BlockKind::Mermaid, "Mermaid"),
                            (BlockKind::ReviewComment, "指摘"),
                        ] {
                            button {
                                class: "kind-button",
                                onclick: move |_| {
                                    let mut document = document;
                                    let id = document.with_mut(|doc| {
                                        let at = doc.blocks.len().saturating_sub(1);
                                        doc.insert_after(at, kind)
                                    });
                                    focus_editor_block(id);
                                },
                                "{label}"
                            }
                        }
                        div { class: "find-bar",
                            input {
                                class: "find-input",
                                value: "{find_query()}",
                                placeholder: "文書内を検索",
                                aria_label: "文書内を検索",
                                oninput: move |event| {
                                    find_query.set(event.value());
                                    find_index.set(0);
                                },
                                onkeydown: move |event| {
                                    if event.key().to_string() == "Enter" && !event.is_composing() {
                                        event.prevent_default();
                                        let matches = find_match_ids(&document(), &find_query());
                                        if let Some(id) = matches.get(find_index() % matches.len().max(1)) {
                                            find_browser_text(&find_query());
                                            visit_block_anchor(&format!("block-{id}"));
                                            notice.set(format!("{} 件中 {} 件目", matches.len(), find_index() + 1));
                                        }
                                    }
                                }
                            }
                            button {
                                class: "kind-button",
                                title: "前の検索結果",
                                onclick: move |_| {
                                    let matches = find_match_ids(&document(), &find_query());
                                    if !matches.is_empty() {
                                        let next = (find_index() + matches.len() - 1) % matches.len();
                                        find_index.set(next);
                                        visit_block_anchor(&format!("block-{}", matches[next]));
                                        notice.set(format!("{} 件中 {} 件目", matches.len(), next + 1));
                                    }
                                },
                                "↑"
                            }
                            button {
                                class: "kind-button",
                                title: "次の検索結果",
                                onclick: move |_| {
                                    let matches = find_match_ids(&document(), &find_query());
                                    if !matches.is_empty() {
                                        let next = (find_index() + 1) % matches.len();
                                        find_index.set(next);
                                        find_browser_text(&find_query());
                                        visit_block_anchor(&format!("block-{}", matches[next]));
                                        notice.set(format!("{} 件中 {} 件目", matches.len(), next + 1));
                                    }
                                },
                                "↓"
                            }
                            if !find_query().trim().is_empty() {
                                span { class: "find-count", "{find_matches.len()} 件" }
                            }
                        }
                    }

                    div { class: "block-list",
                        for (index, block) in editor_blocks.iter().cloned().enumerate().filter(|(_, block)| block.parent.is_none()) {
                            {
                                let block_id = block.id;
                                rsx! {
                                    {render_editor_tree(
                                        block,
                                        index,
                                        editor_blocks.clone(),
                                        document,
                                        slash_for,
                                        slash_index,
                                        dragged,
                                        drop_target,
                                        notice,
                                        find_matches.contains(&block_id),
                                        active_find == Some(block_id),
                                    )}
                                }
                            }
                        }
                    }
                    div { class: "insert-row",
                        button {
                            class: "button ghost",
                            onclick: move |_| {
                                let mut document = document;
                                let id = document.with_mut(|doc| {
                                    let at = doc.blocks.len().saturating_sub(1);
                                    doc.insert_after(at, BlockKind::Paragraph)
                                });
                                focus_editor_block(id);
                            },
                            "+ ブロックを追加"
                        }
                    }
                }

                aside { class: "panel preview-panel",
                    div { class: "preview-header",
                        div { class: "preview-title",
                            span { class: "live-dot" }
                            "ライブプレビュー"
                        }
                        span { class: "source-badge", "{file_name}" }
                    }
                    if !outline.is_empty() {
                        nav { class: "outline",
                            span { class: "outline-title", "見出し" }
                            for heading in outline {
                                {
                                    let anchor = format!("block-{}", heading.id);
                                    let class = match heading.kind {
                                        BlockKind::HeadingOne => "outline-link level-one",
                                        BlockKind::HeadingTwo => "outline-link level-two",
                                        _ => "outline-link level-three",
                                    };
                                    let class = if active_outline() == Some(heading.id) {
                                        format!("{class} active")
                                    } else {
                                        class.to_string()
                                    };
                                    rsx! {
                                        button {
                                            class: "{class}",
                                            onclick: move |_| {
                                                active_outline.set(Some(heading.id));
                                                visit_block_anchor(&anchor);
                                            },
                                            "{heading.text}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    article { class: "preview-body",
                        if preview_is_empty {
                            div { class: "empty-preview", "文書は空です。ブロックを追加して書き始めましょう。" }
                        } else {
                            for block in preview_blocks.iter().cloned().filter(|block| block.parent.is_none()) {
                                {
                                    let match_class = if active_find == Some(block.id) {
                                        "preview-block search-current"
                                    } else if find_matches.contains(&block.id) {
                                        "preview-block search-match"
                                    } else {
                                        "preview-block"
                                    };
                                    rsx! {
                                        div { class: "{match_class}", {render_preview_tree(block, preview_blocks.clone())} }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "shortcuts",
                details {
                    summary { "ショートカット" }
                    div { class: "shortcut-list",
                        span { kbd { "Enter" } " 新しいブロック" }
                        span { kbd { "Shift+Enter" } " 改行" }
                        span { kbd { "↑↓ Enter Esc" } " スラッシュメニュー" }
                        span { kbd { "⌘/Ctrl+B" } " 太字" }
                        span { kbd { "⌘/Ctrl+I" } " 斜体" }
                        span { kbd { "Alt+↑/↓" } " 移動" }
                        span { kbd { "Tab" } " インデント" }
                    }
                }
            }
        }
    }
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
        BlockKind::ReviewComment => rsx! {
            aside { id: "{anchor}", class: "review-comment-preview",
                span { class: "review-comment-label", "指摘" }
                span { "{text}" }
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
        BlockKind::ReviewComment => format!("> [!COMMENT]\n> {value}"),
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
        } else if let Some(value) = line.strip_prefix("> [!COMMENT]") {
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
                .push(new_block(&mut doc.next_id, BlockKind::ReviewComment, text));
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
                    push_html_block_in(
                        doc,
                        BlockKind::ReviewComment,
                        &strip_tags(inner),
                        false,
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

        assert_eq!(mermaid, vec![BlockKind::Mermaid]);
        assert_eq!(review, vec![BlockKind::ReviewComment]);
        assert_eq!(comment, vec![BlockKind::ReviewComment]);
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
}
