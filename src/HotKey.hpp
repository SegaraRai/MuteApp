#pragma once

#include <string>

struct HotKey {
  bool alt;
  bool control;
  bool shift;
  bool win;
  unsigned int vKey;
};

HotKey ParseHotKey(const std::wstring& hotKey);
