[English](Home) | **日本語** | [Polski](Home-pl)

# Grimoire

**脱獄した Kindle Scribe のための手書きノート。** 罫線、ドット、方眼、コーネル式の用紙、
楽譜、ギター／ベースタブを自由に組み合わせて重ねられ、Scribe の筆圧と傾きに合わせた 9 種類の
ペン、フォルダ、そしてページを PDF や PNG としてパソコンに取り出す 3 つの方法があります。
すべては Kindle 上のオープンなファイルに保存されます。アカウントもクラウドも追跡もありません。

見た目は昔のコンソールゲームの城のようです。ピクセル風のブラックレターの見出し、ピクセル
フォント、石を彫ったような枠、尖ったアーチ（設定でワンタップすれば素朴な **Classic** スタイルに
なります）。

GPL-3.0 の自由ソフトウェアです。Amazon とは提携しておらず、承認も受けていません。

## 自分の Kindle で動きますか？

**Kindle Scribe** と、スクリプトレット（ライブラリからタップして実行する `.sh` ファイル）が
動く脱獄が必要です。現在の Scribe 向け脱獄（Vera、SpiderCat、WinterBreak、Sanctuary）はすべて
対応しています。脱獄の方法は [kindlemodding.org](https://kindlemodding.org) を参照してください。
スクリプトレットが動かない古い脱獄では、代わりに KOReader から Grimoire を開けます（表を参照）。

| Kindle | 状態 |
|---|---|
| Kindle Scribe（2024、第 2 世代）、ファームウェア 5.17〜5.20.1 | **動作します。** 5.20.1.0.2 で日常使用しており、5.20.1（Vera）でも動作します |
| Kindle Scribe（2022、第 1 世代）、ファームウェア 5.16.3 以降 | **動作します。** 5.20.1.0.2 で日常使用しており、5.20.1（Vera）でも動作します。画面が上下逆になる場合は「困ったときは」を参照してください。 |
| Kindle Scribe（2022）でファームウェア 5.16.3 より古いもの | 動作するはずですが、古い脱獄向けのライブラリランチャーはまだありません。KOReader を使う場合は、プラグインを手動でコピーすれば KOReader の Tools > Grimoire から開けるはずです（「KOReader で動きますか？」を参照）。 |
| Kindle Scribe 3 と Scribe Colorsoft（2025） | **未検証。** 公開された脱獄はまだありません。Grimoire はこれらの大きな画面（1986x2648）をすでに認識しており、ノートのページサイズは旧 Scribe との間で保たれます（「バージョン間の移動」を参照）。Colorsoft のカラー画面では、ペンのインク以外の素早い更新のたびに画面ドライバーが読み取るまで待つため、ボタンに縞模様が出ないはずです。 |
| Paperwhite、Oasis、Basic などペンのない Kindle | 実用になりません。ペンがないと書けません。 |

ファームウェア 5.20.1 は 2026 年 10 月時点で最新です。更新する前に、脱獄がそれに耐えるか確認して
ください。多くの人は OTA 更新をブロックしたままにしています。

## ドキュメント

| ページ | 内容 |
|---|---|
| [インストール](Install-ja) | 必要なもの、リリース ZIP、KPM、更新とアンインストール |
| [Grimoire の使い方](Using-Grimoire-ja) | ライブラリ、用紙、ペン、ツールバー、インデックス、リンク、ライブビュー、Wi-Fi、スリープ、パスコード |
| [フォントと言語](Fonts-and-Language-ja) | 英語・スペイン語・日本語・ポーランド語、Classic のフォント、自分のフォント |
| [取り込み](Importing-ja) | テンプレート、写真、PDF、すでにあるページ |
| [Kindle からページを取り出す](Exporting-ja) | Wi-Fi 共有、エフェクト、USB の PDF、SSH |
| [KOReader](KOReader-ja) | KOReader から Grimoire を開く、本への書き込み、任意のプラグイン |
| [ショートカット](Shortcuts-ja) | キーボードとコントローラー、`shortcuts.ini` |
| [困ったときは](Troubleshooting-ja) | 既知の問題、よくある対処、バージョン間の移動 |
| [プライバシー](Privacy-ja) | Kindle 内に留まるもの、共有が公開するもの |
| [コントリビュート](Contributing-ja) | バグ、機能要望、プルリクエスト、LLM の使用、wiki |
| [ソースからビルド](Building-from-Source-ja) | ツールチェーン、make のターゲット、テスト |
| [翻訳](Translating-ja) | スペイン語・日本語・ポーランド語の改善、言語の追加 |

## Grimoire を支援する

Grimoire は無料のオープンソースで、一人が心を込めて作っています。あなたのノートの一冊になったなら、
Ko-fi で開発を支援できます。

<a href="https://ko-fi.com/B0B7EF4TJ"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Ko-fi で Grimoire を支援" height="36"></a>

フィードバックや機能要望は issue でも歓迎します。

## ライセンス

Grimoire は自由ソフトウェアです。GPL-3.0-or-later（`LICENSE` を参照）。電子ペーパー画面の駆動に
使っている NiLuJe の [FBInk](https://github.com/NiLuJe/FBInk) と同じライセンスです。Kindle に
ついて Grimoire が知っていることの多くは [KOReader](https://koreader.rocks) と
[kindlemodding.org](https://kindlemodding.org) から来ています。

Kindle と Kindle Scribe は Amazon.com, Inc. またはその関連会社の商標です。Grimoire は独立した
プロジェクトであり、Amazon と提携しておらず、承認もサポートも受けていません。
