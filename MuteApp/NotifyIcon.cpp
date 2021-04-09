#include "pch.h"

#include "NotifyIcon.hpp"

#include <Windows.h>

#include <shellapi.h>

#pragma comment(lib, "Shell32.lib")

NotifyIcon::NotifyIcon(const NOTIFYICONDATAW& notifyIconData, bool setVersion) :
  mNotifyIconData(notifyIconData), mSetVersion(setVersion) {}

BOOL NotifyIcon::Register() {
  do {
    NOTIFYICONDATAW notifyIconData = mNotifyIconData;
    if (!Shell_NotifyIconW(NIM_ADD, &notifyIconData)) {
      if (GetLastError() == ERROR_TIMEOUT) {
        Sleep(1500);
        continue;
      }
      // TODO: return TRUE if notify icon is already registered
      return FALSE;
    }
  } while (false);

  if (mSetVersion) {
    NOTIFYICONDATAW notifyIconData = mNotifyIconData;
    if (!Shell_NotifyIconW(NIM_SETVERSION, &notifyIconData)) {
      return FALSE;
    }
  }

  return TRUE;
}

BOOL NotifyIcon::Unregister() {
  NOTIFYICONDATAW notifyIconData = mNotifyIconData;
  return Shell_NotifyIconW(NIM_DELETE, &notifyIconData);
}

BOOL NotifyIcon::SetIcon(HICON hIcon) {
  mNotifyIconData.hIcon = hIcon;
  NOTIFYICONDATAW notifyIconData = mNotifyIconData;
  return Shell_NotifyIconW(NIM_MODIFY, &notifyIconData);
}
