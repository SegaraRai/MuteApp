# MuteApp

Mute or unmute the focused application with a global hotkey.

![Demo](https://user-images.githubusercontent.com/29276700/115992195-55d5f800-a607-11eb-9096-bb18a7054be5.gif)

## Usage

Download the latest build from [Releases](https://github.com/SegaraRai/MuteApp/releases) and run `MuteApp.exe`.

Focus the application you want to mute or unmute, then press the hotkey. The default hotkey is <kbd>Ctrl+Shift+F8</kbd>.

MuteApp runs in the notification area. Use the tray menu to quit the app.

For regular use, add MuteApp to your startup apps.

## Build

Install Rust 1.95 or later, then run:

```powershell
cargo build --release
```

The executable is generated at `target/release/MuteApp.exe`.

## Configuration

MuteApp creates `MuteApp.cfg` next to the executable. The available settings are:

| Key                             | Default       | Description                                      |
| ------------------------------- | ------------- | ------------------------------------------------ |
| hotkey                          | Ctrl+Shift+F8 | Global hotkey used to toggle mute                |
| indicatorDuration               | 1000          | Overlay duration in milliseconds; `0` disables it |
| indicatorSize                   | 240           | Overlay size in pixels; `0` disables it          |
| indicatorTransparency           | 200           | Overlay background opacity from `0` to `255`     |
| indicatorForegroundTransparency | 255           | Overlay icon opacity from `0` to `255`           |

## Known Limitations

### Some applications may not work

MuteApp looks up the audio session for the process that owns the foreground window, then toggles that session's mute state.

Some multi-process applications may play audio from a different process than the focused window. In those cases, MuteApp may not be able to toggle the expected audio session.

Known unsupported applications:

- Google Chrome and other Chromium-based apps

### Mute state can persist after restarting apps or Windows

This is standard Windows audio session behavior. See [`ISimpleAudioVolume`](https://learn.microsoft.com/windows/win32/api/audioclient/nn-audioclient-isimpleaudiovolume).

Use MuteApp again, or use the Windows volume mixer, to change the mute state.
