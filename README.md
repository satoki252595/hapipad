# MD Block Editor（Markdownブロックエディタ）

ローカルの `.md` / `.html` を、ブラウザだけで Notion 風ブロック編集するエディタです。サーバー・アカウント・データベースは使いません。

Cargo パッケージ名は `blockpad` のままです。公開名は **MD Block Editor / Markdownブロックエディタ** です。

## すぐ使う

ビルド済みファイルは Git には入れていません。[最新リリース](https://github.com/satoki252595/md-block-editor/releases/latest) から取ってください。ブラウザですぐ開くなら [GitHub Pages](https://satoki252595.github.io/md-block-editor/) です。

- Windows: `md-block-editor-windows-x64-setup.exe`
- Linux: `md-block-editor-linux-x86_64.AppImage`
- macOS (Apple Silicon): `md-block-editor-macos-arm64.dmg`
- どの OS でも: `md-block-editor-web.zip`（静的配信）または Pages

## できること

- `/` でスラッシュコマンド（見出し、リスト、コード、表、コールアウト、カラム、Mermaid、指摘など）
- ドラッグハンドル、または `Alt + ↑` / `Alt + ↓` で並べ替え
- 右側のライブプレビュー（見出しアウトライン付き）
- トップバーの ☾ / ☀ でダークモード
- 日本語 IME の変換中にブロックを増やしたり消したりしない
- 見出し、箇条書き、番号付き、チェックリスト、コード、引用、表、コールアウト、折りたたみ、グループ、カラム
- 対応ブラウザは File System Access で元ファイルへ上書き保存。非対応時はファイル選択 + ダウンロード

## 画面

編集とライブプレビュー:

![エディタとライブプレビュー](docs/screenshots/editor.svg)

スラッシュメニュー:

![スラッシュメニュー](docs/screenshots/slash.svg)

見出し・リスト・コード・コールアウト・カラム:

![ブロック種別](docs/screenshots/blocks.svg)

ダークモード:

![ダークモード](docs/screenshots/dark.svg)

ライブプレビュー（Mermaid・表・カラム）:

![ライブプレビュー](docs/screenshots/preview.svg)

## 起動

```bash
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --locked
dx serve --platform web --addr 127.0.0.1 --port 43123
```

`http://127.0.0.1:43123` を開きます。

操作: `Enter` でブロック追加、`Shift + Enter` で改行。`Cmd/Ctrl + B` / `I` で強調。`/mermaid` で図、`/指摘` または `/comment` でレビュー指摘。
