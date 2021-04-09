#include "pch.h"

#include "Audio.hpp"
#include "ConfigFile.hpp"
#include "HotKey.hpp"
#include "NotifyIcon.hpp"

#include "resource.h"

#include <cstddef>
#include <optional>
#include <stdexcept>
#include <string>
#include <system_error>

#include <Windows.h>

#include <shellapi.h>
#include <windowsx.h>

#pragma comment(lib, "Gdi32.lib")

using namespace std::literals;

namespace {
  constexpr std::size_t PathBufferSize = 65600;
  constexpr std::size_t MenuPathLength = 32;
  constexpr auto MutexName = L"MuteAppMutex";
  constexpr auto ClassNameMain = L"MuteApp";
  constexpr auto ClassNameIndicator = L"MuteAppIndicator";
  constexpr auto WindowNameMain = L"MuteApp";
  constexpr auto WindowNameIndicator = L"MuteApp";
  constexpr UINT NotifyIconId = 0x0001;
  constexpr UINT NotifyIconCallbackMessageId = WM_APP + 0x1101;
  constexpr UINT HotKeyId = 0x0001;
  constexpr UINT_PTR TimerId = 0x0001;

  const UINT gTaskbarCreatedMessage = RegisterWindowMessageW(L"TaskbarCreated");
  std::optional<ConfigFile> gConfigFile;
  std::optional<NotifyIcon> gNotifyIcon;
  HINSTANCE gHInstance = NULL;
  HICON gHIcon = NULL;
  HICON gHIconDisabled = NULL;
  UINT gTimerEventId = 0;
  HWND gHWndIndicator = NULL;
  bool gMuted = false;

  std::wstring GetModuleFilepath(HMODULE hModule) {
    constexpr std::size_t BufferSize = 65600;

    auto buffer = std::make_unique<wchar_t[]>(BufferSize);
    GetModuleFileNameW(hModule, buffer.get(), BufferSize);
    if (GetLastError() != ERROR_SUCCESS) {
      throw std::system_error(std::error_code(GetLastError(), std::system_category()), "GetModukeFileNameW failed");
    }

    return std::wstring(buffer.get());
  }
} // namespace

LRESULT CALLBACK MainWindowProc(HWND hWnd, UINT uMsg, WPARAM wParam, LPARAM lParam) {
  if (gTaskbarCreatedMessage != 0 && uMsg == gTaskbarCreatedMessage) {
    // not including in switch block because gTaskbarCreatedMessage is not a constexpr
    // NOTE: To receive "TaskbarCreated" message, the window must be a top level window. A message-only window cannot receive this message.
    if (gNotifyIcon) {
      if (!gNotifyIcon.value().Register()) {
        const std::wstring message =
          L"Failed to register notify icon (code "s + std::to_wstring(GetLastError()) + L")"s;
        MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
        DestroyWindow(hWnd);
      }
    }
    return DefWindowProcW(hWnd, uMsg, wParam, lParam);
  }

  switch (uMsg) {
    case WM_CREATE: {
      SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
      break;
    }

    // hotkey
    case WM_HOTKEY: {
      if (wParam != HotKeyId) {
        break;
      }

      const auto hForegroundWnd = GetForegroundWindow();
      if (hForegroundWnd == NULL) {
        MessageBeep(MB_OK);
        break;
      }

      DWORD pid;
      GetWindowThreadProcessId(hForegroundWnd, &pid);
      if (pid == 0) {
        MessageBeep(MB_OK);
        break;
      }

      bool muted;
      try {
        muted = !ToggleMuteByProcessId(pid);
        gMuted = muted;
      } catch (...) {
        MessageBeep(MB_OK);
        break;
      }

      const auto duration = gConfigFile.value().GetInt(L"indicatorDuration").value();
      const auto baseSize = gConfigFile.value().GetInt(L"indicatorSize").value();
      const auto transparency = gConfigFile.value().GetInt(L"indicatorTransparency").value();

      if (duration <= 0 || baseSize <= 0 || transparency <= 0) {
        break;
      }

      RECT rect{};
      GetWindowRect(hForegroundWnd, &rect);

      const LONG cx = (rect.left + rect.right) / 2;
      const LONG cy = (rect.top + rect.bottom) / 2;

      SetWindowPos(hWnd, NULL, cx, cy, 1, 1, SWP_HIDEWINDOW);
      const auto dpi = GetDpiForWindow(hWnd);

      const long size = baseSize * dpi / 96;
      SetWindowPos(gHWndIndicator, NULL, cx - size / 2, cy - size / 2, size, size, SWP_SHOWWINDOW | SWP_NOACTIVATE);
      InvalidateRect(gHWndIndicator, NULL, FALSE);
      UpdateWindow(gHWndIndicator);

      SetTimer(hWnd, TimerId, duration, NULL);

      break;
    }

    case WM_TIMER: {
      if (wParam != TimerId) {
        break;
      }

      ShowWindow(gHWndIndicator, SW_HIDE);

      break;
    }

    // menu
    case WM_COMMAND: {
      if (lParam != 0 || HIWORD(wParam) != 0) {
        break;
      }

      switch (LOWORD(wParam)) {
        case ID_CONTEXTMENU_QUIT:
          PostMessageW(hWnd, WM_CLOSE, 0, 0);
          return 0;
      }

      break;
    }

    // notify icon
    case NotifyIconCallbackMessageId:
      if (HIWORD(lParam) != NotifyIconId) {
        break;
      }

      switch (LOWORD(lParam)) {
        case NIN_SELECT:
        case NIN_KEYSELECT:
        case WM_CONTEXTMENU: {
          HMENU hMenu = NULL;
          do {
            hMenu = LoadMenuW(gHInstance, MAKEINTRESOURCEW(IDR_MENU1));
            if (hMenu == NULL) {
              break;
            }

            const HMENU hSubMenu = GetSubMenu(hMenu, 0);
            if (hMenu == NULL) {
              break;
            }

            SetForegroundWindow(hWnd);

            const auto x = GET_X_LPARAM(wParam);
            const auto y = GET_Y_LPARAM(wParam);

            const UINT flags = TPM_LEFTALIGN;
            TrackPopupMenuEx(hSubMenu, flags, x, y, hWnd, NULL);
          } while (false);

          if (hMenu != NULL) {
            DestroyMenu(hMenu);
            hMenu = NULL;
          }

          return 0;
        }
      }
      break;

    // on exit
    case WM_CLOSE:
      DestroyWindow(hWnd);
      return 0;

    case WM_DESTROY:
      PostQuitMessage(0);
      return 0;
  }

  return DefWindowProcW(hWnd, uMsg, wParam, lParam);
}

LRESULT CALLBACK IndicatorWindowProc(HWND hWnd, UINT uMsg, WPARAM wParam, LPARAM lParam) {
  switch (uMsg) {
    case WM_CREATE: {
      SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
      break;
    }

    case WM_PAINT: {
      winrt::Windows::UI::ViewManagement::UISettings settings;
      const auto background = settings.GetColorValue(winrt::Windows::UI::ViewManagement::UIColorType::Background);
      const auto foreground = settings.GetColorValue(winrt::Windows::UI::ViewManagement::UIColorType::Foreground);

      RECT rect{};
      GetClientRect(hWnd, &rect);

      // assume right == bottom
      const LONG size = rect.right;

      PAINTSTRUCT ps{};
      const auto hDC = BeginPaint(hWnd, &ps);

      SetBkColor(hDC, RGB(background.R, background.G, background.B));
      SetTextColor(hDC, RGB(foreground.R, foreground.G, foreground.B));

      const auto hBrushBG = CreateSolidBrush(RGB(background.R, background.G, background.B));
      FillRect(hDC, &rect, hBrushBG);
      DeleteObject(hBrushBG);

      const auto hFont = CreateFontW(size / 2,
                                     0,
                                     0,
                                     0,
                                     FW_DONTCARE,
                                     FALSE,
                                     FALSE,
                                     FALSE,
                                     DEFAULT_CHARSET,
                                     OUT_OUTLINE_PRECIS,
                                     CLIP_DEFAULT_PRECIS,
                                     CLEARTYPE_QUALITY,
                                     DEFAULT_PITCH,
                                     L"Segoe MDL2 Assets");
      const auto hOldFont = SelectObject(hDC, hFont);

      RECT textRect{ size / 4, size / 4, size * 3 / 4, size * 3 / 4 };
      wchar_t text[2]{};
      text[0] = gMuted ? L'\uE74F' : L'\uE767';
      DrawTextExW(hDC, text, 1, &textRect, DT_CENTER | DT_VCENTER, NULL);

      SelectObject(hDC, hOldFont);
      DeleteObject(hFont);

      EndPaint(hWnd, &ps);

      break;
    }

    // on exit
    case WM_CLOSE:
      return 0;

    case WM_DESTROY:
      return 0;
  }

  return DefWindowProcW(hWnd, uMsg, wParam, lParam);
}

int WINAPI wWinMain(HINSTANCE hInstance,
                    [[maybe_unused]] HINSTANCE hPrevInstance,
                    [[maybe_unused]] LPWSTR lpCmdLine,
                    [[maybe_unused]] int nShowCmd) {
  winrt::init_apartment();

  gHInstance = hInstance;

  // multiple instance check
  SetLastError(ERROR_SUCCESS);
  auto hMutex = CreateMutexW(NULL, TRUE, MutexName);
  if (const auto error = GetLastError(); error != ERROR_SUCCESS) {
    if (hMutex != NULL) {
      ReleaseMutex(hMutex);
      hMutex = NULL;
    }

    // unexpected error
    if (error != ERROR_ALREADY_EXISTS) {
      const std::wstring message = L"Initialization error: CreateMutexW failed with code "s + std::to_wstring(error);
      MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
      return 1;
    }

    return 0;
  }

  // instantiate ConfigFile
  const auto exeFilepath = GetModuleFilepath(NULL);
  const auto bsPos = exeFilepath.find_last_of(L'\\');
  const auto configFilepath =
    bsPos == std::wstring::npos ? exeFilepath + L".cfg"s : exeFilepath.substr(0, bsPos + 1) + L"MuteApp.cfg"s;
  gConfigFile.emplace(configFilepath);

  auto& configFile = gConfigFile.value();
  configFile.Set(L"hotkey"s, L"Ctrl+Shift+F8"s, true);
  configFile.Set(L"hotkeyRepeat"s, 0, true);
  configFile.Set(L"indicatorDuration"s, 1000, true);
  configFile.Set(L"indicatorSize"s, 200, true);
  configFile.Set(L"indicatorTransparency"s, 200, true);
  configFile.Save();

  // open icon
  gHIcon = LoadIconW(hInstance, MAKEINTRESOURCEW(IDI_ICON1));
  if (gHIcon == NULL) {
    const std::wstring message =
      L"Initialization error: LoadIconW failed with code "s + std::to_wstring(GetLastError());
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    return 1;
  }

  // register window class (main)
  const WNDCLASSEXW wndClassExWMain{
    sizeof(wndClassExWMain),
    CS_HREDRAW | CS_VREDRAW | CS_NOCLOSE,
    MainWindowProc,
    0,
    0,
    hInstance,
    gHIcon,
    LoadCursorW(NULL, IDC_ARROW),
    reinterpret_cast<HBRUSH>(GetStockObject(WHITE_BRUSH)),
    NULL,
    ClassNameMain,
    gHIcon,
  };
  if (!RegisterClassExW(&wndClassExWMain)) {
    const std::wstring message =
      L"Initialization error: RegisterClassExW failed with code "s + std::to_wstring(GetLastError());
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    return 1;
  }

  // register window class (indicator)
  const WNDCLASSEXW wndClassExWIndicator{
    sizeof(wndClassExWIndicator),
    CS_HREDRAW | CS_VREDRAW | CS_NOCLOSE,
    IndicatorWindowProc,
    0,
    0,
    hInstance,
    gHIcon,
    LoadCursorW(NULL, IDC_ARROW),
    reinterpret_cast<HBRUSH>(GetStockObject(WHITE_BRUSH)),
    NULL,
    ClassNameIndicator,
    gHIcon,
  };
  if (!RegisterClassExW(&wndClassExWIndicator)) {
    const std::wstring message =
      L"Initialization error: RegisterClassExW failed with code "s + std::to_wstring(GetLastError());
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    return 1;
  }

  // create indicator window
  gHWndIndicator = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                                   ClassNameIndicator,
                                   WindowNameIndicator,
                                   WS_POPUP,
                                   CW_USEDEFAULT,
                                   CW_USEDEFAULT,
                                   CW_USEDEFAULT,
                                   CW_USEDEFAULT,
                                   NULL,
                                   NULL,
                                   hInstance,
                                   NULL);
  if (gHWndIndicator == NULL) {
    const std::wstring message =
      L"Initialization error: CreateWindowExW failed with code "s + std::to_wstring(GetLastError());
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    return 1;
  }

  const auto transparency = static_cast<BYTE>(std::clamp(configFile.GetInt(L"indicatorTransparency"s).value(), 0, 255));
  SetLayeredWindowAttributes(gHWndIndicator, 0, transparency, LWA_ALPHA);

  // create main window
  HWND hWndMain = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                                  ClassNameMain,
                                  WindowNameMain,
                                  WS_POPUP,
                                  CW_USEDEFAULT,
                                  CW_USEDEFAULT,
                                  CW_USEDEFAULT,
                                  CW_USEDEFAULT,
                                  NULL,
                                  NULL,
                                  hInstance,
                                  NULL);
  if (hWndMain == NULL) {
    const std::wstring message =
      L"Initialization error: CreateWindowExW failed with code "s + std::to_wstring(GetLastError());
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    return 1;
  }

  SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

  // register notify icon
  gNotifyIcon.emplace(
    NOTIFYICONDATAW{
      sizeof(NOTIFYICONDATAW),
      hWndMain,
      NotifyIconId,
      NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
      NotifyIconCallbackMessageId,
      gHIcon,
      L"MuteApp",
      0,
      0,
      {},
      {
        NOTIFYICON_VERSION_4,
      },
      {},
      NIIF_NONE,
      {},
      NULL,
    },
    true);

  if (!gNotifyIcon.value().Register()) {
    ReleaseMutex(hMutex);
    const std::wstring message =
      L"Initialization error: Failed to register notify icon (code "s + std::to_wstring(GetLastError()) + L")"s;
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    DestroyWindow(hWndMain);
    DestroyWindow(gHWndIndicator);
    return 1;
  }

  // register hotkey
  const auto hotKey = ParseHotKey(configFile.GetStr(L"hotkey").value());
  if (hotKey.vKey == 0) {
    ReleaseMutex(hMutex);
    MessageBoxW(
      NULL, L"Initialization error: Failed to parse hotkey", L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    DestroyWindow(hWndMain);
    DestroyWindow(gHWndIndicator);
    return 1;
  }
  if (RegisterHotKey(hWndMain,
                     HotKeyId,
                     (hotKey.alt ? MOD_ALT : 0) | (hotKey.control ? MOD_CONTROL : 0) | (hotKey.shift ? MOD_SHIFT : 0) |
                       (hotKey.win ? MOD_WIN : 0) | (configFile.GetInt(L"hotkeyRepeat"s).value() ? 0 : MOD_NOREPEAT),
                     hotKey.vKey) == 0) {
    ReleaseMutex(hMutex);
    const std::wstring message =
      L"Initialization error: Failed to register hotkey (code "s + std::to_wstring(GetLastError()) + L")"s;
    MessageBoxW(NULL, message.c_str(), L"MuteApp", MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
    DestroyWindow(hWndMain);
    DestroyWindow(gHWndIndicator);
    return 1;
  }

  // message loop
  MSG msg;
  BOOL gmResult;
  while (true) {
    gmResult = GetMessageW(&msg, hWndMain, 0, 0);
    if (gmResult == 0 || gmResult == -1) {
      break;
    }
    TranslateMessage(&msg);
    DispatchMessageW(&msg);
  }

  DestroyWindow(gHWndIndicator);

  ReleaseMutex(hMutex);

  return msg.message == WM_QUIT ? static_cast<int>(msg.wParam) : 0;
}
