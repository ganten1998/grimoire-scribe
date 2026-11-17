[English](Building-from-Source) | **日本語** | [Polski](Building-from-Source-pl)

# ソースからビルド

アプリは Rust です。`rust/` に 1 つのクレートがあり（`rust/Cargo.toml`。最上位に
`Cargo.toml` はありません）、最上位の Makefile がビルドとチェックを行います。cargo は `rust/`
の中で実行するか、make を使ってください。cargo のコマンドとコードのルールは
`rust/PORTING.md` にあります。

## 必要なもの

バージョンはリリースワークフローの固定値（`.github/workflows/release.yml`）です。

- **Rust 1.97.1** を rustup で、Kindle ターゲット付きでインストールし、Makefile がそれを指すように
  します。

  ```sh
  rustup toolchain install 1.97.1 --profile minimal --target armv7-unknown-linux-musleabihf
  export RUST_BIN="$(dirname "$(rustup which --toolchain 1.97.1 rustc)")"
  ```

  `RUST_BIN` がない場合、Makefile は Apple Silicon 向けの rustup の `stable` ツールチェーンを
  PATH の先頭に置きます（Rust のリリースごとに変わり、追加しない限り Kindle ターゲットを持ち
  ません）。Intel Mac や Linux ではそのフォルダーが存在しないため、make は PATH 上で最初に見つ
  かった `cargo` を使います。
- **Zig 0.16.0**（ziglang.org）と **cargo-zigbuild 0.23.0**
  （`cargo install cargo-zigbuild --version 0.23.0 --locked`）。Kindle ビルド用です。新しい
  Zig でもチェックは通りますが、リリースと同じバイト列になるのは固定バージョンだけです。
- C コンパイラー（`clang`。Linux では gcc）、make、curl、unzip。`build.rs` が同梱の C を
  コンパイルします。
- **Python 3.12 以降**（標準ライブラリのみ）、**Node.js 18 以降**、**LuaJIT**。チェックが
  これらを実行します。
- 任意。なくてもメッセージ付きでスキップされます: shellcheck、luacheck、dash、busybox
  （ランチャーと KOReader プラグイン）、ImageMagick 7 の `magick`（エフェクトスタジオの画像
  チェック）、GNU gettext の `msgfmt`（言語の `.mo` 比較。`make l10n` に必要）。`make sim`
  には SDL3 と pkg-config が必要です（Mac では `brew install sdl3 pkgconf`）。

## make のターゲット

```sh
make check     # Kindle なしで確認できるすべて: ユニットテスト、状態チェック付きの
               # スクリプト UI シナリオ、すべての画面のすべての言語、共有サーバー、
               # ライブビュー、Bluetooth、エクスポーター、ARM ビルド、ランチャーと
               # KOReader プラグインを含む zip、バイト単位で比較する 2 回のクリーンビルド、
               # KPM パッケージ。最後に CHECK PASSED
make test      # ユニットテスト（rust/tests）。ホストとヘッドレスビルドの両方
make scenarios # ヘッドレスランナーで tests/scenarios/*.qs を実行
make kindle    # build/kindle/grimoire: 静的 ARMv7 ハードフロート（musl）バイナリ
make dist      # dist/Grimoire-VERSION-COMMIT/ に完全なインストール可能バンドル
make package   # build/dist/Grimoire.zip
make kpm-package # build/kpm/*.kpkg と preview/stable リポジトリマニフェスト
make tools     # build/host/grimoire-export（tools/grimoire-pull が使用）
make tablet    # build/host/scribe-tablet: Mac でペンをタブレットにする（osu! 用）
make l10n      # ソースから po/grimoire.pot を作り、各 po/<lang>.po を更新してコンパイル
make sim       # C ツリーのシミュレーター: SDL3 ウィンドウ、Rust 移植前のアプリ
```

15 コアの Mac では、ビルドキャッシュが温まった状態で `make -j15 check SCEN_JOBS=8` に約 7 分
かかります。初回ビルドや、より小さく混んでいるマシンではもっとかかります。`build/` は数 GB に
なり、cargo が古いビルドを `build/cargo` に保持するため時間とともに増えます。2 つのチェック
アウトで、異なるポート（`GRIMOIRE_TEST_PORT`）を使って同時に実行できます。`make check` 全体は
macOS で動作します。Linux ではユニットテストとシナリオが動作し（既知の失敗が 2 つ、
`docs/BACKLOG.md`）、それ以外は試されていません。共有のチェックには macOS の `sips` が必要
です。

残す、またはインストールするビルドには `make dist` を使ってください。インストーラー ZIP、
任意の KOReader パッチ ZIP、KPM パッケージとマニフェスト、チェックサム、ソースリビジョンを
`dist/` に集めます。バージョン／コミットごとのフォルダーで異なるビルドを分け、コレクターは
異なるファイルへの上書きを拒否します。`DIST=/path/to/dist` でワークツリー間の共有出力
フォルダーを選べます。ビルドの中間ファイルは `build/` に残ります。ビルドを集めても公開はされ
ません。

## 開発用ドキュメント

- `docs/DESIGN.md` — アーキテクチャ、リフレッシュモデル、ブラシモデル、言語、ファイル形式
- `docs/FORMAT.md` — ファイルをバイト単位で
- `docs/KINDLE.md` — Kindle と KOReader への組み込み
- `docs/TESTING.md` — 実機テストと CI の構成
- `docs/SIM.md` — シミュレーターとヘッドレスシナリオランナー
- `docs/TRANSLATING.md` — 翻訳の改善または追加
- `docs/TABLET.md` — SSH 経由でペンを Mac のタブレットにするツール
