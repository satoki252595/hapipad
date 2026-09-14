# Hapipad

Hapipad は、はぴまねのローカル Notion 風エディタです。ブラウザだけで Markdown / HTML を編集・レビューする軽量な Rust/WASM アプリで、ブロック編集、ライブプレビュー、Mermaid 図、指摘ブロックを備え、元のファイル形式へ保存します。

The Cargo package name remains `blockpad` so the existing Dioxus/WASM build keeps working.

## Run locally

The complete `Cargo.lock` and `src/main.rs` are stored as matching slices under `parts/` (GitHub file-write size). Concatenate them once after clone:

```bash
cat parts/Cargo.lock.0{0,1,2,3,4,5,6,7} > Cargo.lock
mkdir -p src
cat parts/main.rs.0{0,1,2,3,4,5,6,7,8,9} parts/main.rs.10 > src/main.rs
```

Install the Rust WASM target and Dioxus CLI once:

```bash
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --locked
```

Then start the browser app:

```bash
dx serve --platform web --addr 127.0.0.1 --port 43123
```

Open `http://127.0.0.1:43123`.

## Local files

- **Open file** uses the File System Access API when the browser supports it, so **Save** writes back to the original `.md` or `.html` file.
- Browsers without that API use a file-picker upload and a safe download fallback.
- The app has no server, account, database, or Node dependency.

Markdown の ` ```mermaid ` フェンス、HTML の `pre.mermaid`、表、ネストしたリスト、リンク、図版、`details`、引用、コード、レビュー用の `> [!COMMENT]` 指摘をブロックとして扱います。

## Editor controls

Type `/` in a block to switch its kind; `/mermaid` creates a Mermaid source block and `/指摘` or `/comment` creates a review note. Drag the handle to reorder blocks, or use `Alt + ↑` / `Alt + ↓`. `Enter` inserts a block; `Shift + Enter` keeps a line break. `Cmd/Ctrl + B` and `Cmd/Ctrl + I` add Markdown emphasis markers. IME composition events do not create or delete blocks.
