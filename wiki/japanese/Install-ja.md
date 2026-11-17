[English](../english/Install.md) | **日本語**

# インストール

Grimoire は、スクリプトレット（ライブラリからタップして実行する `.sh` ファイル）が動く脱獄済みの
**Kindle Scribe** で動作します。現在の Scribe 向け脱獄（Vera、SpiderCat、WinterBreak、
Sanctuary）はすべて対応しています。脱獄の方法は
[kindlemodding.org](https://kindlemodding.org) を参照してください。スクリプトレットのない古い
脱獄では、代わりに KOReader から Grimoire を開けます（[KOReader](KOReader-ja) を参照）。

## リリース ZIP から

1. [Releases](https://github.com/ganten1998/grimoire-scribe/releases) ページから
   `Grimoire-<version>.zip` をダウンロードして解凍します。`documents` フォルダに 2 つの
   ファイルが入っています。ほかに、手動インストール用の KOReader プラグインとパッチの
   フォルダも任意で入っています。
2. Kindle をパソコンに接続します。Scribe は MTP を使うため、Mac では
   [OpenMTP](https://openmtp.ganeshrvel.com)（または `brew install --cask openmtp`）を使い、
   Windows と Linux ではデバイスとして表示されます。
3. **両方のファイル**を Kindle の `documents` フォルダにコピーします。
   - `Grimoire.sh` — アプリ本体（すべてを中に含んでいます）
   - `Grimoire - reset pen calibration.sh` — 万一のときのレスキュー用
4. Kindle を取り出します。ライブラリにゴシック調の表紙の **Grimoire** が現れます（インデックス
   作成に 1 分ほどかかることがあります）。タップして開きます。

初回起動では、`grimoire` フォルダに自分を展開するのに数秒かかります。ノートと設定はそこに
保存されます（Kindle 上では `/mnt/us/grimoire`）。

## 更新

新しい `Grimoire.sh` を古いものに上書きコピーします。ノートと設定はそのまま残ります。古い
バージョンに戻すのも同じ方法です（[困ったときは](Troubleshooting-ja#バージョン間の移動) を参照）。

アプリから更新することもできます。**ライブラリ > 設定 > アップデート** で Grimoire のフィードを
KPM 経由で確認し、新しい Grimoire をインストールします。Grimoire が終了し、KPM が画面に進行状況を
出しながらインストールし、Grimoire が再起動します。ベータ版にはベータ版とリリース版、リリース版には
リリース版だけが届きます。そこでタップするとリリース版のみに切り替わります。KPM と Wi-Fi が
必要です。

## Quire から移行する

Grimoire は以前 Quire という名前でした。新しいファイルをコピーして Grimoire を一度タップすると、
ノートと設定が `quire/` から `grimoire/` に移され、古い Quire ランチャーが削除されます。

## KPM でインストールする

リリースには `kindlehf` の Scribe 向け KPM パッケージも含まれています。`.kpkg` を Kindle に
コピーし、Grimoire を閉じてから、Kindle のシェルで次を実行します。

```sh
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_2.0.1001_kindlehf.kpkg
/var/local/kmc/bin/kpm launch grimoire
```

ファイル名は実際にダウンロードしたものに置き換えてください。ライブラリのランチャーは
`kpm launch grimoire` を呼び出します。アプリ本体は KPM のパッケージディレクトリに置かれ、初回
起動時に `grimoire/app` へ展開されます。ペンキャリブレーションをリセットするライブラリ項目も
含まれます。通常のライブラリからの起動の前に KOReader を終了してください。

KPM でのインストールは、公式の手動 Grimoire ランチャー 2 つを移行できます。アンインストールで
削除されるのは変更されていない KPM のライブラリランチャーだけです。**`grimoire/` のデータは
すべて残ります**（ノート、設定、PDF、マッピングなど）。

## アンインストール

`documents` から `Grimoire.sh` と `Grimoire - reset pen calibration.sh` を削除し、次に
`grimoire` フォルダを削除します（ページを残したい場合は先に `grimoire/notebooks` や
`grimoire/PDFs` をどこかへコピーしてください）。KOReader を使っている場合は
`koreader/plugins/grimoire.koplugin` も削除します。

KPM の場合: `kpm -y uninstall grimoire`。変更されていない KPM のライブラリランチャーだけが
削除され、`grimoire/` のデータはすべて残ります。データフォルダは、本当に削除するときだけ自分で
消してください。
