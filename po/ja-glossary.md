# Japanese glossary (po/ja.po)

Grimoire's Japanese is one `ja`. This page lists the choices it made; `make l10n-check` shows
every screen in `build/shots/l10n/ja-gothic/` and `ja-classic/`, and lists any text that no longer
fits as `fit[ja]: ...`.

## The pen and the drawing tools

| English | Japanese | Notes |
| --- | --- | --- |
| pen (the Scribe's stylus) | **ペン** | *タッチペン* only where the device itself is meant. |
| Pencil (tool) | **鉛筆** | |
| Mechanical pencil | **シャープペンシル** | The tool picker uses the same word. |
| Ballpoint | **ボールペン** | |
| Fountain pen | **万年筆** | |
| Fineliner | **細書きペン** | |
| Marker | **マーカー** | |
| Highlighter | **蛍光ペン** | |
| Eraser | **消しゴム** | |
| Charcoal | **木炭** | |
| Calligraphy pen | **カリグラフィーペン** | |
| nib | **ペン先** | |
| Pen feel | **ペンの感触** | |
| pressure, tilt | **筆圧**, **傾き** | |
| deadzone | **デッドゾーン** | |
| grain, smoothing | **粒子**, **スムージング** | |
| test pad | **試し書き** | |
| white ink | **白インク** | |
| Whole strokes / Precise | **ストローク全体** / **精密** | |
| stroke | **ストローク** | *線* would read as a drawn line. |

## Notebooks, papers and music

| English | Japanese | Notes |
| --- | --- | --- |
| notebook | **ノート** | *ノートブック* only where the longer word is clearer. |
| Library | **ライブラリ** | |
| folder | **フォルダー** | |
| Untitled | **無題** | |
| Paper | **紙** | |
| Blank | **白紙** | |
| Narrow / Medium / Wide rule | **細罫 / 中罫 / 太罫** | With no margin: **細罫（余白なし）**. |
| Dot grid | **ドット方眼** | |
| Graph paper | **方眼紙** | |
| Cornell notes | **コーネルノート** | The printed Cues / Notes / Summary stay English on the page. |
| Sheet music | **五線譜** | |
| Grand staff | **大譜表** | |
| Guitar tab | **ギタータブ** | |
| Custom music | **カスタム楽譜** | |
| Treble / Bass / Alto / Tenor staff | **ト音譜表 / ヘ音譜表 / ハ音譜表 / テノール譜表** | |
| Drum staff | **ドラム譜** | |
| Plain staff | **無記号譜表** | |
| Tab, 4 to 8 lines | **タブ 4線** ... **タブ 8線** | |
| Lyric line | **歌詞行** | |
| system (of staves) | **段** | |
| Template | **テンプレート** | |
| Imported page | **読み込んだページ** | |
| Newer paper | **新しい紙** | |

## Screens, actions and settings

| English | Japanese | Notes |
| --- | --- | --- |
| Settings | **設定** | |
| Share | **共有** | |
| New / Folder | **新規 / フォルダー** | |
| Exit | **終了** | |
| Back to KOReader | **KOReaderに戻る** | |
| Done / Cancel | **完了 / キャンセル** | |
| Save / Delete / Clear | **保存 / 削除 / 消去** | Clear takes the ink; Delete removes the page or notebook. |
| Rename | **名前を変更** | |
| Back | **戻る** | |
| Language | **言語** | |
| Automatic | **自動** | The languages' own names stay: **English**, **Español**, **日本語**. |
| Display | **画面** | |
| Brightness / Warmth / Invert | **明るさ / 色温度 / 反転** | |
| Sleep | **スリープ** | |
| screensaver | **スクリーンセーバー** | |
| Controller | **コントローラー** | |
| pair / forget | **ペアリング / 解除** | A toast says what failed: **コントローラーをペアリングできませんでした**, **コントローラーを解除できませんでした**. |
| shortcuts | **ショートカット** | Settings > Shortcuts: **ショートカット: キーとコントローラーのボタン**. |
| Reset | **…をリセット** | The verb, not a noun: **ボタンをリセット**, **ジェスチャーをリセット**. |
| Link | **リンク** | |
| Index | **索引** | |
| Pin | **固定** | |
| Live view | **ライブビュー** | |
| Dismiss (What's new) | **閉じる** | It closes the card. |
| Disable | **無効にする** | The record card's button. |
| unreleased / This version | **未リリース / このバージョン** | What's new's list of versions. |

## Files, the PDF import and fonts

| English | Japanese | Notes |
| --- | --- | --- |
| Import | **インポート** | The file picker's first row, and the verb: **インポート中**, **インポート済み**. |
| Documents / Downloads | **ドキュメント / ダウンロード** | The file picker's rows. |
| All storage | **すべてのストレージ** | |
| Look again | **もう一度探す** | |
| %d picture | **%d枚の画像** | Pictures are counted in 枚. |
| Waiting / Imported | **待機中 / インポート済み** | A row of the Import place. |
| has notes | **ノートあり** | A PDF's tag in the file picker. |
| document links | **文書のリンク** | The links a PDF brings (a planner's tabs). |
| done | **done** | The folder's name: kept. |
| Font | **フォント** | Settings > Font, and the chip **フォント: %s**. |
| Built in | **内蔵フォント** | The heading over Grimoire's own fonts. |
| Your fonts | **自分のフォント** | The heading over the owner's fonts: **自分のフォント (%s)**. |
| merge / keep both | **統合する / 両方残す** | The folder dialog's answers; **両方残す（「…」として）**. |

## Dates, counters and numbers

- Dates: **10月10日 14:32**; a new notebook is **ノート 10月10日**. The catalog's month names
  are **1月** to **12月**; the date formats are `%1$s%2$d日, %3$02d:%4$02d` and `%1$s%2$d日`.
- The 12-hour clock puts **午前** or **午後** before the hour: **10月10日 午後2:32**, its format
  `%1$s%2$d日 %5$s%3$d:%4$02d` (all five arguments numbered, as they are moved).
- Counters follow what is counted: **%dページ**, **%dノート**, **%d冊**, **%d枚** for pictures.
  Japanese has one plural form, so `msgstr[0]` serves 0, 1 and every other count.
- Percent keeps the English's space: **150 %**. Angles: **45°**. Decimals: **0.30 mm**, as the
  code writes them.
- Quotes: **「」** in prose around a notebook's name; the English's straight quotes may stay in a
  format or a path.

## Fonts and characters

The Font sheet's sample sentence is **今日はいい天気ですね。コーヒーでも飲みながら、本を読みましょう。**

Every character must be in DotGothic16 or one of the Latin faces. DotGothic16 covers the kana,
the JIS kanji and the punctuation used here; a rare kanji outside it is left out where a name is
drawn, as the Latin faces already leave out characters they lack. Do not add emoji, rare CJK
ideographs, or a character the font check reports missing.
