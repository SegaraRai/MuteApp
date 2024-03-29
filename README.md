# MuteApp

ホットキーを押下して任意のアプリをミュート/ミュート解除できます。

![動作画面](https://user-images.githubusercontent.com/29276700/115992195-55d5f800-a607-11eb-9096-bb18a7054be5.gif)

## 使い方

[Releases](https://github.com/SegaraRai/MuteApp/releases)からダウンロードして、実行します。

ミュートを設定/解除したいアプリにフォーカスを移してホットキー（規定では<kbd>Ctrl+Shift+F8</kbd>）を押下すると、ミュートが設定または解除されます。

通知エリアから終了できます。

スタートアップに登録しておくと便利です。

## 既知の不具合

### 一部のアプリで動作しない

本アプリはフォアグラウンドウィンドウのプロセスに対応するオーディオセッションを取得し、ミュート/ミュート解除を行います。  
そのため、一部のマルチプロセスアプリケーションでは動作しないことがあります。

現在確認している未対応のアプリは以下の通りです。

- Google Chrome等Chromium系のアプリ

### OSやアプリの再起動後もミュート設定が残る

[Windowsの仕様](https://docs.microsoft.com/en-us/windows/win32/api/audioclient/nn-audioclient-isimpleaudiovolume)です。

MuteAppまたはOS標準の音量ミキサーから再設定してください。

## 設定ファイル

アプリの設定ファイルに記述される項目を以下に記します。

| キー                  | 既定値        | 値の意味                                           |
| --------------------- | ------------- | -------------------------------------------------- |
| hotkey                | Ctrl+Shift+F8 | ホットキー                                         |
| hotkeyRepeat          | 0             | キーリピートを許容するか（0/1）                    |
| indicatorDuration     | 1000          | インジケータの表示時間（ミリ秒）、0 で表示しない   |
| indicatorSize         | 200           | インジケータの大きさ（px）、0 で表示しない         |
| indicatorTransparency | 200           | インジケータの不透明度（0 ～ 255）、0 で表示しない |
