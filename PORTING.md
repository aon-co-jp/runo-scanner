# PORTING.md — runo-scanner お引越しファイル

## 移植パターン

`src/main.rs`は外部プロジェクト固有の依存を持たない自己完結なCLIツール
(`walkdir`/`ignore`/`serde`/`toml`/`regex`/`serde_json`のみ)。他のドライブ
構成・別エコシステムへ移植する場合は、`runo-scan.txt`(スキャンルール、
[open-raid-z](https://github.com/aon-co-jp/open-raid-z)側で管理)の
フォーマット(TOML: `max_depth`/`exclude_dirs`/`project_markers`/
`naming.repo_name_pattern`/`naming.domain_name_pattern`/`cache.index_file`)
に合わせた新しい設定ファイルを用意すればそのまま動く。

## 移植先での注意点

- キャッシュ(`cache.index_file`で指定するJSON)はディレクトリ単位の
  更新時刻のみを記録する軽量な差分検出であり、ファイル内容のハッシュ
  検証等は行わない——高い正確性が必要な用途では検証ロジックの追加を
  検討すること。
