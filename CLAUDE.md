# 開発方針・開発環境ルール(runo-scanner)

作業ドライブは`F:\runo`。この節は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`を
正本とし、各プロジェクトへコピーして同期する方針に準じる。
GitHubリポジトリ: [aon-co-jp/runo-scanner](https://github.com/aon-co-jp/runo-scanner)。

## このプロジェクトの役割

`F:\runo`配下の複数プロジェクトを自動認識するスキャナー(`src/main.rs`、
`walkdir`+`ignore`crateでの再帰探索、`regex`でのリポジトリ名/ドメイン名
判定、ディレクトリ更新時刻ベースのキャッシュによる差分スキャン)。
設定ファイル(`runo-scan.txt`、探索深度・除外ディレクトリ・プロジェクト
判定マーカー等)は`open-raid-z`側で管理する——このリポジトリは実装のみを
持ち、スキャンルールそのものの正本ではない。

## 復旧ポイント(2026-07-23追加、どのリポジトリからでも作業を再開できるように)

エコシステム全体の開発方針・運用ルール・全プロジェクトの現況・直近の
セッションチェックポイントは
[`open-raid-z`の`CLAUDE.md`](https://github.com/aon-co-jp/open-raid-z/blob/main/CLAUDE.md)
を正本として一元管理している。このリポジトリから作業を再開する場合も、
まず`open-raid-z/CLAUDE.md`のHANDOFF節(特に直近の「セッション末尾
チェックポイント」エントリ)を読んでから着手すること。

## 関連プロジェクト

- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — 開発ルールの正本、`runo-scan.txt`の管理元。

## HANDOFF

- **2026-07-23 README/CLAUDE.md新規作成**: それまでこのリポジトリには
  ドキュメントが一切無かった(実装〈`src/main.rs`〉のみ)。エコシステム
  全体のドキュメント整備の一環として作成、他リポジトリへの復旧ポイント
  導線を追加。

- **2026-10-18 v0.1.0公開**: 単体テスト4件・設定例(runo-scan.example.txt)・日英README・LICENSE(Apache-2.0 OR MIT)を追加、全履歴の機密検査後にリポジトリを公開、Windows版バイナリ付きRelease。

