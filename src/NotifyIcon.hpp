#pragma once

#include <Windows.h>

#include <shellapi.h>

class NotifyIcon {
  NOTIFYICONDATAW mNotifyIconData;
  bool mSetVersion;

public:
  NotifyIcon(const NOTIFYICONDATAW& notifyIconData, bool setVersion);

  BOOL Register();
  BOOL Unregister();
  BOOL SetIcon(HICON hIcon);
};
