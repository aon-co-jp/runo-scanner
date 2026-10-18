# runo-scanner

ディレクトリ配下を再帰的にスキャンし、プロジェクト(リポジトリ/ドメイン名のフォルダ)の一覧をJSONで出力するRust製CLI。
`Cargo.toml` などのマーカーファイルでプロジェクトルートを判定し、前回から更新日時が変わっていないフォルダは再スキャンを省きます(差分更新)。

A Rust CLI that recursively scans a directory tree and prints the list of projects (repo / domain-named folders) as JSON.
A folder containing a marker file (e.g. `Cargo.toml`) is treated as a project root; folders whose modification time has not changed since the last run are skipped (incremental cache).

## 使い方 / Usage

```bash
cargo run --release -- <スキャン対象ディレクトリ / root> [設定ファイル / config] [--update-ecosystem-map]
```

- 設定ファイルを省略すると `<root>/runo-scan.txt` を読みます。同梱の [`runo-scan.example.txt`](runo-scan.example.txt)(TOML形式)をコピーして編集してください。
  If the config is omitted, `<root>/runo-scan.txt` is read. Copy and edit the bundled [`runo-scan.example.txt`](runo-scan.example.txt) (TOML).
- 結果は標準出力にJSON(`path` / `kind` / `marker`)で出力され、キャッシュは設定の `cache.index_file` に保存されます。
  Results are printed as JSON (`path` / `kind` / `marker`); the cache is stored in `cache.index_file`.
- `--update-ecosystem-map` は、`<root>/open-raid-z/CLAUDE.md` の「## 関連プロジェクト」節に未掲載のリポジトリを自動生成ブロックとして追記します(実行前に `CLAUDE.md.bak` を作成)。該当ファイルが無ければ何もしません。通常は不要なオプションです。
  `--update-ecosystem-map` appends repos missing from the "## 関連プロジェクト" section of `<root>/open-raid-z/CLAUDE.md` (a `CLAUDE.md.bak` backup is written first; it does nothing if the file is absent). Most users do not need it.

## 制約 / Limitations

- キャッシュはフォルダの更新日時のみを見ます。内容のハッシュ検証はしません。 / The cache only compares folder modification times, not content hashes.
- 設定ファイルの `max_depth` を超える深さは探索しません。 / Folders deeper than `max_depth` are not scanned.

## テスト / Tests

```bash
cargo test
```

## ライセンス / License

Apache-2.0 または MIT(どちらか選択)。 / Apache-2.0 OR MIT, at your option. See `LICENSE-APACHE`, `LICENSE-MIT`.

[open-raid-z](https://github.com/aon-co-jp/open-raid-z) を中心とする aon-co-jp エコシステムの一部です。 / Part of the aon-co-jp ecosystem.
