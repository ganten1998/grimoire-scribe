[English](../english/Translating.md) | **日本語**

# Grimoire の翻訳

Grimoire は英語、スペイン語、日本語に対応しています。このページでは、翻訳の改善方法と新しい
言語の追加方法を説明します。完全なルールはリポジトリの `docs/TRANSLATING.md` にあります。

## 用意されているもの

```
po/grimoire.pot      翻訳するすべての文字列。make l10n がソースから作成
po/<lang>.po         1 つの言語の翻訳（po/es.po）。翻訳者が編集するファイル
po/<lang>.mo         msgfmt が .po をコンパイルしたもの。コミットされ、バイナリに埋め込まれる
po/es-glossary.md    スペイン語が選んだ語と、検討した代替案
po/ja-glossary.md    日本語が選んだ語、フォントと日付の注意
```

- 英語はコード（`src/*.c` と `web/studio.js`）にあります。カタログにない文字列、または空の
  文字列は英語で表示されます。
- `po/grimoire.pot` は画面に表示されるすべての文字列とエフェクトスタジオの文字列を、それぞれの
  `TRANSLATORS` コメントと出典ファイル付きで保持します。
- ビルドに gettext は不要です。バイナリがコミット済みの `.mo` ファイルを埋め込みます。GNU
  gettext が必要な `make l10n` は、ソースから `po/grimoire.pot` を作り直し、各
  `po/<lang>.po` にマージし、それぞれを `.mo` にコンパイルします。`make check` の一部である
  `make l10n-check` は、すべての画面をすべての言語でチェックします。
- KOReader の Grimoire プラグインには、`po/` にない独自の語が少しあります。
  `kindle/koreader/grimoire.koplugin/main.lua` の `STRINGS` テーブルと `_meta.lua` の説明
  です。KOReader 自身の言語設定で選ばれます。
- 翻訳は `po/es.po` のヘッダーにあるように、Grimoire のライセンス GPL-3.0-or-later の下に
  あります。

## スペイン語を改善する

スペイン語はスペインとラテンアメリカ向けの 1 つの中立な `es` で、読者に *tú* で話しかけます。
`po/es-glossary.md` にすべての語の選択と検討した代替案があります。そこから始めてください。

## 日本語を改善する

日本語は 1 つの `ja` で、文は丁寧なです・ます調、ボタンは名詞ラベルで話しかけます。
`po/ja-glossary.md` に主な選択と日付・助数詞の慣例があります。そこから始めてください。かなと
漢字は、日本語フォールバックとして埋め込まれたピクセルフォント DotGothic16 から表示されます
（かなと常用漢字にサブセット化）。これによりゴシック調のスタイルが保たれます。

## 翻訳のルール

- **余白。** ほとんどの文字列は固定幅のウィジェットに収まり、`TRANSLATORS` コメントがテキスト
  サイズに対するピクセル数で余白を示します。「284 px at 34 px」は、高さ 34 px で描かれ、幅
  284 px の余白があるという意味です。`make l10n-check` が短く切られたすべてのテキストを一覧
  表示します。それでも収まらない語は短くしてください。
- **上部バーのタップ。** 上部バーの語が Gothic で 153 px（New 以外は 159 px）、Classic で
  175 px より広いと、ボタンが広がってシナリオのタップ位置から New がずれます。その語はその幅に
  収めてください。
- **引数。** `%s`、`%d` などは英語と同じ種類を同じ数だけ残してください。順序を変える場合は
  すべてに番号を付けます（`%2$d %1$s`）。番号付きと番号なしを混在させず、すべての引数を使い、
  パーセント記号は英語と同じ場所で `%%` と書きます。
- **文全体。** 各文字列は 1 つの文またはラベルで、値は `%s` や `%d` で入ります。タップで
  切り替わる行は値ごとに 1 つの文なので、それぞれ全体を翻訳します。
- **複数形。** `msgid_plural` を持つ文字列には、その言語のすべての形（`msgstr[0]`、
  `msgstr[1]`...）を Plural-Forms ルールの順に書きます。
- **コンテキスト。** `msgctxt` は同じ英語の 2 つの意味（5 月の "May"、用紙の "Blank"）を
  区別します。それぞれの意味で翻訳してください。コンテキストは `paper`、`tool`、
  `music part`、`month`、`date`、`key`、`keyboard`、`share link`、`folder`、
  `sleep after`、`studio` です。
- **文字。** すべての文字は、いずれかのフォントが描けるものでなければなりません。Jersey 10 と
  Jacquard 24（Gothic）、Terminess の regular と bold（Classic）、日本語なら DotGothic16 です。
  設定 > フォントの Classic フォントも、日本語自身の文字以外はすべて持っている必要があります。
- **フォント名**（Terminess、Atkinson Hyperlegible Next、Noto Sans、Literata）は翻訳しません。
- **そのままにするもの**: Grimoire、Kindle、Kindle Scribe、KOReader、Wi-Fi、PDF、PNG、ZIP、
  ImageMagick。Kindle 上のファイル名とフォルダー名（`grimoire/PDFs`、`grimoire/trash`、
  `grimoire.log`）、コマンド `tools/grimoire-pull --http`。コーネル用紙に印刷される見出しは
  ページの一部で、カタログには含まれません。

## 言語を追加する

フランス語 `fr` を例にします。

1. テンプレートからカタログを作ります。

   ```
   msginit --input=po/grimoire.pot --locale=fr --output-file=po/fr.po --no-translator
   ```

   `msginit` がヘッダーを埋めます（フランス語なら Plural-Forms は
   `nplurals=2; plural=(n > 1);`）。ヘッダーコメントに `po/es.po` と同じライセンス行を入れて
   ください。
2. `src/i18n.c` の `k_langs` の最後に、その言語の行を追加します。コード、その言語自身での
   名前（「Français」）、複数形の数、そして Plural-Forms に合うように書いた、数から形の
   インデックスへの複数形ルール関数です。
3. `tests/test_i18n.c` の `k_rules` に、カタログのヘッダーと同じ Plural-Forms 行を追加し、
   そのファイルの `plurals` テストに関数のチェックを追加します（`NG_` の 0、1、2 と、
   カタログが示す形）。
4. 翻訳してから `make l10n` を実行します。テンプレートが `po/fr.po` にマージされ、
   `po/fr.mo` がコンパイルされます。`make l10n-check` がすべての画面をチェックし、
   `make test` が `tests/test_i18n.c` を実行します。

## ビルドせずに変更を試す

デスクトップシミュレーターで、ビルドし直さずに変更を試すには、カタログを手動でコンパイルして
シミュレーターに指定します。

```sh
mkdir -p /tmp/l10n && msgfmt --check -o /tmp/l10n/es.mo po/es.po
GRIMOIRE_L10N_DIR=/tmp/l10n GRIMOIRE_LANG=es ./build/host/grimoire-sim
```

ログに `i18n: (simulator) the catalog for es comes from /tmp/l10n/es.mo` と表示されます。

gettext がなくても `po/es.po` や `po/ja.po` を編集して送ることはできます。ビルドする人が
`make l10n` を実行します。`po/<lang>.po` と `po/<lang>.mo` は一緒にコミットし、
`po/grimoire.pot` が変わっていればそれもコミットしてください。`make l10n-check` は `.mo` が
`.po` から msgfmt で作ったものと違うと失敗します。
