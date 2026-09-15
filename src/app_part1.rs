use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const STYLE: Asset = asset!("/assets/style.css");

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r##"
let blockpadFileHandle = null;

const fileTypes = [{
  description: "Markdown / HTML 文書",
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

export function read_block_selection(id) {
  const input = document.getElementById(`editor-block-${id}`);
  if (!input) return "";
  const start = input.selectionStart ?? 0;
  const end = input.selectionEnd ?? start;
  if (end <= start) return "";
  return `${start}\u0000${end}\u0000${input.value.slice(start, end)}`;
}

export async function copy_text(text) {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return "copied";
    }
  } catch (_) {}
  const area = document.createElement("textarea");
  area.value = text;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.focus();
  area.select();
  const ok = document.execCommand("copy");
  area.remove();
  if (!ok) throw new Error("クリップボードへコピーできませんでした");
  return "copied";
}

export function persist_session(key, json) {
  try {
    localStorage.setItem(key, json);
    return "ok";
  } catch (_) {
    return "fail";
  }
}

export function load_session(key) {
  try {
    return localStorage.getItem(key) || "";
  } catch (_) {
    return "";
  }
}
"##)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn open_local_file() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn save_local_file(name: &str, content: &str, mime: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn copy_text(text: &str) -> Result<JsValue, JsValue>;
    fn apply_theme(is_dark: bool);
    fn focus_block(id: u32);
    fn wrap_selection(id: u32, marker: &str);
    fn visit_anchor(id: &str);
    fn render_mermaid_block(id: &str, source: &str);
    fn find_in_page(query: &str);
    fn read_block_selection(id: u32) -> String;
    fn persist_session(key: &str, json: &str) -> String;
    fn load_session(key: &str) -> String;
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
    Review,
}

#[derive(Clone, PartialEq)]
struct ReviewTarget {
    block_id: u32,
    kind: BlockKind,
    quoted: String,
    whole_block: bool,
    start: usize,
    end: usize,
    start_line: u32,
    end_line: u32,
}

#[derive(Clone, Copy)]
struct ReviewUi {
    open: Signal<bool>,
    targets: Signal<Vec<ReviewTarget>>,
    body: Signal<String>,
    generating: Signal<bool>,
    error: Signal<Option<String>>,
    copied: Signal<Option<String>>,
    composing: Signal<Option<u32>>,
    append: Signal<bool>,
    round: Signal<u32>,
    snapshot: Signal<Option<String>>,
    snapshot_label: Signal<String>,
    diff_mode: Signal<DiffMode>,
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
            Self::ReviewComment => "comment review 指摘 コメント レビュー llm 返す",
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
                    text: "対象: #2 テキスト\n引用: 初回体験を短くし、利用者が最初の価値に到達するまでの手順を明確にします。\n\n確認：モバイルでの招待フローも同じ導線で検証してください。".to_string(),
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
    let mut notice = use_signal(|| "本文を選んでレビューし、LLM に返せます".to_string());
    let mut find_query = use_signal(String::new);
    let mut find_index = use_signal(|| 0usize);
    let mut active_outline = use_signal(|| None::<u32>);
    let is_loading = use_signal(|| false);
    let file_error = use_signal(|| None::<String>);
    let mut mobile_view = use_signal(|| MobileView::Editor);
    let mut review = ReviewUi {
        open: use_signal(|| true),
        targets: use_signal(Vec::new),
        body: use_signal(String::new),
        generating: use_signal(|| false),
        error: use_signal(|| None::<String>),
        copied: use_signal(|| None::<String>),
        composing: use_signal(|| None::<u32>),
        append: use_signal(|| false),
        round: use_signal(|| 0u32),
        snapshot: use_signal(|| None::<String>),
        snapshot_label: use_signal(|| "開いたときの文書".to_string()),
        diff_mode: use_signal(|| DiffMode::Off),
    };
    use_hook(move || restore_saved_review(review));

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
        MobileView::Review => "app-shell mobile-review",
    };
    let workspace_class = if (review.open)() {
        "workspace with-review"
    } else {
        "workspace"
    };
    let review_count = current
        .blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment)
        .count();
    let review_unresolved = current
        .blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment && !block.checked)
        .count();
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
        document::Title { "Markdownブロックエディタ — MD Block Editor" }
        document::Link { rel: "stylesheet", href: STYLE }
        main { class: "{shell_class}",
            header { class: "topbar",
                div { class: "brand",
                    div { class: "brand-mark", "B" }
                    span { "MD Block Editor" }
                    span { class: "brand-subtitle", "選択してレビューし、LLMに返す" }
                }
                div { class: "top-actions",
                    button {
                        class: "button ghost",
                        onclick: move |_| {
                            let document = document;
                            let mut notice = notice;
                            let slash_for = slash_for;
                            let mut is_loading = is_loading;
                            let mut file_error = file_error;
                            is_loading.set(true);
                            file_error.set(None);
                            notice.set("ファイルを読み込んでいます…".to_string());
                            spawn(async move {
                                match load_from_browser().await {
                                    Ok(Some((name, source, content))) => {
                                        let parsed = parse_document(&name, source, &content);
                                        apply_opened_document(
                                            document,
                                            review,
                                            notice,
                                            slash_for,
                                            parsed,
                                            false,
                                        );
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
                        class: "button ghost",
                        title: "同じファイル選択で開き、レビューパネルを前面にする",
                        onclick: move |_| {
                            let document = document;
                            let mut notice = notice;
                            let slash_for = slash_for;
                            let mut is_loading = is_loading;
                            let mut file_error = file_error;
                            is_loading.set(true);
                            file_error.set(None);
                            notice.set("レビューするファイルを選んでいます…".to_string());
                            spawn(async move {
                                match load_from_browser().await {
                                    Ok(Some((name, source, content))) => {
                                        let parsed = parse_document(&name, source, &content);
                                        apply_opened_document(
                                            document,
                                            review,
                                            notice,
                                            slash_for,
                                            parsed,
                                            true,
                                        );
                                        mobile_view.set(MobileView::Review);
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
                        "レビューで開く"
                    }
                    button {
                        class: if (review.open)() { "button ghost active-review" } else { "button ghost" },
                        title: "レビューパネルを開く",
                        aria_label: "レビューパネルを開く",
                        onclick: move |_| {
                            let next = !(review.open)();
                            review.open.set(next);
                            if next {
                                mobile_view.set(MobileView::Review);
                                notice.set("対象を選んで指摘を書き、LLM にコピーできます".to_string());
                            } else if mobile_view() == MobileView::Review {
                                mobile_view.set(MobileView::Editor);
                            }
                        },
                        if review_count == 0 {
                            "レビュー"
                        } else if review_unresolved == 0 {
                            {format!("レビュー {review_count}")}
                        } else {
                            {format!("レビュー {review_unresolved}/{review_count}")}
                        }
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
                button {
                    class: if mobile_view() == MobileView::Review { "mobile-tab active" } else { "mobile-tab" },
                    onclick: move |_| {
                        mobile_view.set(MobileView::Review);
                        review.open.set(true);
                    },
                    "レビュー"
                }
            }

            div { class: "{workspace_class}",
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
                                        review,
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

                if (review.open)() {
                    {render_review_panel(document, review, notice, mobile_view)}
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
                        span { kbd { "⌘/Ctrl+Shift+R" } " 選択をレビュー" }
                        span { kbd { "Alt+↑/↓" } " 移動" }
                        span { kbd { "Tab" } " インデント" }
                    }
                }
            }
        }
    }
}

fn render_review_panel(
    mut document: Signal<DocumentState>,
    mut review: ReviewUi,
    mut notice: Signal<String>,
    mut mobile_view: Signal<MobileView>,
) -> Element {
    let targets = (review.targets)();
    let body = (review.body)();
    let generating = (review.generating)();
    let copied = (review.copied)();
    let error = (review.error)();
    let prompt_preview = if targets.is_empty() {
        String::new()
    } else if body.trim().is_empty() {
        String::new()
    } else {
        build_llm_prompt(&document(), &targets, &body)
    };
    let existing_comments = document()
        .blocks
        .iter()
        .filter(|block| block.kind == BlockKind::ReviewComment)
        .cloned()
        .collect::<Vec<_>>();
    rsx! {
        aside { class: "panel review-panel",
            div { class: "review-header",
                div { class: "preview-title",
                    span { class: "live-dot review-dot" }
                    "選択してレビュー"
                }
                button {
                    class: "icon-button",
                    title: "レビューパネルを閉じる",
                    aria_label: "レビューパネルを閉じる",
                    onclick: move |_| {
                        review.open.set(false);
                        if mobile_view() == MobileView::Review {
                            mobile_view.set(MobileView::Editor);
                        }
                    },
                    "×"
                }
            }
            p { class: "review-lead",
                "本文をドラッグして選ぶか、ブロックの「レビュー」を押します。指摘は対象の下にスレッドとして出ます。終えると LLM に貼る全文をコピーします。"
            }
            p { class: "review-autosave", "指摘の下書きはこのブラウザに自動保存します。サーバーは使いません。" }
            {render_diff_section(document, review, notice)}
            label { class: "review-append",
                input {
                    r#type: "checkbox",
                    checked: (review.append)(),
                    onchange: move |_| {
                        let next = !(review.append)();
                        review.append.set(next);
                        persist_review_now(document, review);
                    },
                }
                "次の選択を範囲に追加（Shift でも可）"
            }

            if generating {
                div { class: "file-feedback loading", "下書きを組み立てています…" }
            }
            if let Some(message) = error.clone() {
                div { class: "file-feedback error",
                    strong { "コピーできません" }
                    span { "{message}" }
                }
            }
            if let Some(kind) = copied.clone() {
                div { class: "file-feedback success",
                    strong { "コピーしました" }
                    span { "{kind} をクリップボードに入れました。LLM の入力へ貼ってください。" }
                }
            }

            if targets.is_empty() {
                div { class: "review-empty",
                    strong { "まだ対象がありません" }
                    span { "編集中の文をドラッグするか、行末の「レビュー」でブロック全体を選べます。IME 変換中は取り込みません。" }
                }
            } else {
                div { class: "review-targets",
                    for (index, target) in targets.iter().cloned().enumerate() {
                        {
                            let quoted = target.quoted.clone();
                            let label = format!(
                                "#{} {} · {}",
                                target.block_id,
                                target.kind.label(),
                                target_range_label(&target)
                            );
                            rsx! {
                                article { class: "review-target-card",
                                    div { class: "review-target-meta",
                                        span { "{label}" }
                                        button {
                                            class: "mini-button",
                                            title: "この対象を外す",
                                            aria_label: "この対象を外す",
                                            onclick: move |_| {
                                                let mut next = (review.targets)();
                                                if index < next.len() {
                                                    next.remove(index);
                                                }
                                                review.targets.set(next);
                                                review.copied.set(None);
                                                persist_review_now(document, review);
                                            },
                                            "外す"
                                        }
                                    }
                                    blockquote { "{quoted}" }
                                }
                            }
                        }
                    }
                }
            }

            label { class: "review-body-label",
                "指摘"
                textarea {
                    class: "review-body",
                    rows: 7,
                    value: "{body}",
                    placeholder: "直してほしい点を書いてください。空なら下書きを生成できます。",
                    oncompositionstart: move |_| review.composing.set(Some(0)),
                    oncompositionend: move |_| {
                        if (review.composing)() == Some(0) {
                            review.composing.set(None);
                        }
                    },
                    oninput: move |event| {
                        review.body.set(event.value());
                        review.copied.set(None);
                        review.error.set(None);
                        persist_review_now(document, review);
                    },
                }
            }

            div { class: "review-actions",
                button {
                    class: "button ghost",
                    disabled: generating || targets.is_empty(),
                    onclick: move |_| {
                        if (review.generating)() {
                            return;
                        }
                        let current_targets = (review.targets)();
                        if current_targets.is_empty() {
                            review.error.set(Some(
                                "対象がありません。本文をドラッグして選ぶか、ブロックの「レビュー」を押してください。".to_string(),
                            ));
                            return;
                        }
                        review.generating.set(true);
                        review.error.set(None);
                        review.copied.set(None);
                        spawn(async move {
                            sleep_ms(220).await;
                            review.body.set(generate_review_drafts(&current_targets));
                            review.generating.set(false);
                            persist_review_now(document, review);
                            notice.set("下書きを入れました。直してから LLM にコピーしてください".to_string());
                        });
                    },
                    "下書きを生成"
                }
                button {
                    class: "button ghost",
                    disabled: generating,
                    onclick: move |_| {
                        let current_targets = (review.targets)();
                        let current_body = (review.body)();
                        if let Some(message) = review_export_error(&current_targets, &current_body) {
                            review.error.set(Some(message));
                            return;
                        }
                        let inserted = document.with_mut(|doc| {
                            insert_review_comment(doc, &current_targets, &current_body)
                        });
                        if let Some(id) = inserted {
                            focus_editor_block(id);
                            notice.set("指摘ブロックを追加しました".to_string());
                            review.error.set(None);
                            persist_review_now(document, review);
                        }
                    },
                    "指摘にする"
                }
                button {
                    class: "button ghost",
                    disabled: generating,
                    onclick: move |_| {
                        let doc = document();
                        let current_targets = (review.targets)();
                        let current_body = (review.body)();
                        if let Some(message) = review_export_error(&current_targets, &current_body) {
                            review.error.set(Some(message));
                            return;
                        }
                        let payload = build_llm_prompt(&doc, &current_targets, &current_body);
                        spawn(async move {
                            match copy_to_clipboard(&payload).await {
                                Ok(()) => {
                                    review.error.set(None);
                                    review.copied.set(Some("プロンプト".to_string()));
                                    remember_snapshot(
                                        document,
                                        review,
                                        "直前の LLM コピー".to_string(),
                                    );
                                    notice.set("LLM に貼るプロンプトをコピーしました".to_string());
                                }
                                Err(message) => review.error.set(Some(message)),
                            }
                        });
                    },
                    "LLMにコピー"
                }
                button {
                    class: "button ghost",
                    disabled: generating,
                    onclick: move |_| {
                        let doc = document();
                        let current_targets = (review.targets)();
                        let current_body = (review.body)();
                        if let Some(message) = review_export_error(&current_targets, &current_body) {
                            review.error.set(Some(message));
                            return;
                        }
                        let payload = build_llm_json(&doc, &current_targets, &current_body);
                        spawn(async move {
                            match copy_to_clipboard(&payload).await {
                                Ok(()) => {
                                    review.error.set(None);
                                    review.copied.set(Some("JSON".to_string()));
                                    notice.set("構造化 JSON をコピーしました".to_string());
                                }
                                Err(message) => review.error.set(Some(message)),
                            }
                        });
                    },
                    "JSONをコピー"
                }
                button {
                    class: "button ghost",
                    onclick: move |_| {
                        let doc = document();
                        let payload = build_comments_json(
                            &doc,
                            &(review.targets)(),
                            &(review.body)(),
                            (review.round)(),
                        );
                        spawn(async move {
                            match copy_to_clipboard(&payload).await {
                                Ok(()) => {
                                    review.error.set(None);
                                    review.copied.set(Some("コメントJSON".to_string()));
                                    notice.set("指摘スレッドの JSON をコピーしました".to_string());
                                }
                                Err(message) => review.error.set(Some(message)),
                            }
                        });
                    },
                    "コメントJSONをコピー"
                }
                button {
                    class: "button ghost",
                    disabled: generating,
                    onclick: move |_| {
                        let doc = document();
                        let current_targets = (review.targets)();
                        let current_body = (review.body)();
                        if let Some(message) = review_export_error(&current_targets, &current_body) {
                            review.error.set(Some(message));
                            return;
                        }
                        let payload = build_llm_prompt(&doc, &current_targets, &current_body);
                        spawn(async move {
                            match save_to_browser("review-for-llm.md", &payload, "text/markdown").await {
                                Ok(message) => {
                                    review.error.set(None);
                                    notice.set(format!("プロンプトを書き出しました（{}）", save_status(&message)));
                                }
                                Err(message) => review.error.set(Some(format!("書き出せませんでした：{message}"))),
                            }
                        });
                    },
                    "プロンプトを保存"
                }
                button {
                    class: "button ghost",
                    onclick: move |_| {
                        let doc = document();
                        let payload = build_comments_json(
                            &doc,
                            &(review.targets)(),
                            &(review.body)(),
                            (review.round)(),
                        );
                        spawn(async move {
                            match save_to_browser("md-block-editor-comments.json", &payload, "application/json").await {
                                Ok(message) => {
                                    review.error.set(None);
                                    notice.set(format!("コメント JSON を書き出しました（{}）", save_status(&message)));
                                }
                                Err(message) => review.error.set(Some(format!("書き出せませんでした：{message}"))),
                            }
                        });
                    },
                    "コメントJSONを保存"
                }
                button {
                    class: "button primary",
                    disabled: generating,
                    onclick: move |_| {
                        let doc = document();
                        let current_targets = (review.targets)();
                        let current_body = (review.body)();
                        let unresolved = unresolved_comments(&doc);
                        if let Some(message) = finish_review_error(&current_targets, &current_body, &unresolved) {
                            review.error.set(Some(message));
                            return;
                        }
                        let next_round = (review.round)() + 1;
                        let payload = build_finish_prompt(
                            &doc,
                            &current_targets,
                            &current_body,
                            &unresolved,
                            (review.snapshot)().as_deref(),
                            next_round,
                        );
                        spawn(async move {
                            match copy_to_clipboard(&payload).await {
                                Ok(()) => {
                                    review.round.set(next_round);
                                    review.error.set(None);
                                    review.copied.set(Some("レビュー全文".to_string()));
                                    remember_snapshot(
                                        document,
                                        review,
                                        format!("ラウンド {next_round}（レビュー完了）"),
                                    );
                                    notice.set("レビューを終え、エージェントへ貼る全文をコピーしました".to_string());
                                }
                                Err(message) => review.error.set(Some(message)),
                            }
                        });
                    },
                    "レビューを終えてコピー"
                }
            }

            if !prompt_preview.is_empty() {
                details { class: "review-preview",
                    summary { "今回の下書き（LLM に貼る文面）" }
                    pre { class: "review-prompt", "{prompt_preview}" }
                }
            }

            if existing_comments.is_empty() {
                div { class: "review-empty compact",
                    "文書内の指摘はまだありません。/指摘 でも同じブロックを置けます。"
                }
            } else {
                {render_existing_comment_cards(document, review, notice, existing_comments)}
            }
        }
    }
}
