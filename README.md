# Otosu

YouTubeの動画やプレイリストをデスクトップアプリへ共有し、ダウンロードや管理を行うためのTauriアプリケーションです。
専用のFirefox拡張機能と連携して動作します。

## プロジェクト構成

- **`src/` & `src-tauri/`**: Tauri + Vue 3 + TypeScript によるデスクトップアプリの本体
- **`firefox-extension/`**: YouTubeからOtosuへURLを送るためのFirefox拡張機能

---

## デスクトップアプリ (Tauri)

### 必要条件

- [Rust](https://www.rust-lang.org/) のインストール
- Node.js / [Bun](https://bun.sh/) パッケージマネージャのインストール

### 開発コマンド

依存関係のインストール:
```bash
bun install
```

開発用サーバーの起動 (Vite + Tauri デバッグ):
```bash
bun tauri dev
```

プロダクションビルドの作成:
```bash
bun tauri build
```

---

## Firefox 拡張機能

専用の拡張機能を使用することで、ブラウザからワンクリック、またはコンテキストメニュー（右クリック）から動画をOtosuへ送ることができます。

### 開発・ローカルでの動作確認手順

1. Firefoxのアドレスバーに `about:debugging#/runtime/this-firefox` を入力して開きます。
2. **「一時的なアドオンを読み込む...」** (Load Temporary Add-on...) をクリックします。
3. `firefox-extension/` フォルダ内にある `manifest.json` を選択します。
4. YouTubeの動画ページを開き、ツールバーのアイコンまたは右クリックメニュー「Otosuでダウンロード」から動作を確認できます。

*※ あらかじめOtosuデスクトップアプリを一度起動し、OSに `otosu://` ディープリンクプロトコルを登録しておく必要があります。*

### GitHub Actions による自動ビルド

本リポジトリには以下の自動化ワークフローが設定されています。

- **Tauri アプリのビルド・リリース (`publish`):** Tag `v*` (例: `v1.0.0`) をプッシュすると、Tauriバイナリが自動ビルドされ、GitHub Releaseの下書きとしてアップロードされます。
- **Firefox 拡張機能のパッケージング (`Pack Firefox Extension`):** `firefox-extension/` 配下の変更が `main` にプッシュされた際、コードの検証（Lint）が行われ、成果物としてそのまま手動インストールに利用できる zip ファイルが GitHub Artifacts に保存されます。

## ライセンス

[MIT License](LICENSE)
