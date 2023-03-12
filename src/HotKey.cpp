#include "pch.h"

#include "HotKey.hpp"

#include <algorithm>
#include <cwctype>
#include <regex>
#include <string>
#include <unordered_map>

#include <Windows.h>

using namespace std::literals;

namespace {
  // see https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
  const std::unordered_map<std::wstring, unsigned int> gKeyNameMap{
    { L"BACKSPACE"s, VK_BACK },
    { L"TAB"s, VK_TAB },
    { L"ENTER"s, VK_RETURN },
    { L"SPACE"s, VK_SPACE },
    { L"PAGEUP"s, VK_PRIOR },
    { L"PAGEDOWN"s, VK_NEXT },
    { L"END"s, VK_END },
    { L"HOME"s, VK_HOME },
    { L"LEFT"s, VK_LEFT },
    { L"UP"s, VK_UP },
    { L"RIGHT"s, VK_RIGHT },
    { L"INSERT"s, VK_INSERT },
    { L"DELETE"s, VK_DELETE },
    { L"0"s, 0x30 },
    { L"1"s, 0x31 },
    { L"2"s, 0x32 },
    { L"3"s, 0x33 },
    { L"4"s, 0x34 },
    { L"5"s, 0x35 },
    { L"6"s, 0x36 },
    { L"7"s, 0x37 },
    { L"8"s, 0x38 },
    { L"9"s, 0x39 },
    { L"A"s, 0x41 },
    { L"B"s, 0x42 },
    { L"C"s, 0x43 },
    { L"D"s, 0x44 },
    { L"E"s, 0x45 },
    { L"F"s, 0x46 },
    { L"G"s, 0x47 },
    { L"H"s, 0x48 },
    { L"I"s, 0x49 },
    { L"J"s, 0x4A },
    { L"K"s, 0x4B },
    { L"L"s, 0x4C },
    { L"M"s, 0x4D },
    { L"N"s, 0x4E },
    { L"O"s, 0x4F },
    { L"P"s, 0x50 },
    { L"Q"s, 0x51 },
    { L"R"s, 0x52 },
    { L"S"s, 0x53 },
    { L"T"s, 0x54 },
    { L"U"s, 0x55 },
    { L"V"s, 0x56 },
    { L"W"s, 0x57 },
    { L"X"s, 0x58 },
    { L"Y"s, 0x59 },
    { L"Z"s, 0x5A },
    { L"NUMPAD0"s, VK_NUMPAD0 },
    { L"NUMPAD1"s, VK_NUMPAD1 },
    { L"NUMPAD2"s, VK_NUMPAD2 },
    { L"NUMPAD3"s, VK_NUMPAD3 },
    { L"NUMPAD4"s, VK_NUMPAD4 },
    { L"NUMPAD5"s, VK_NUMPAD5 },
    { L"NUMPAD6"s, VK_NUMPAD6 },
    { L"NUMPAD7"s, VK_NUMPAD7 },
    { L"NUMPAD8"s, VK_NUMPAD8 },
    { L"NUMPAD9"s, VK_NUMPAD9 },
    { L"MUL"s, VK_MULTIPLY },
    { L"ADD"s, VK_ADD },
    { L"SEP"s, VK_SEPARATOR },
    { L"SUB"s, VK_SUBTRACT },
    { L"DEC"s, VK_DECIMAL },
    { L"DIV"s, VK_DIVIDE },
    { L"F1"s, VK_F1 },
    { L"F2"s, VK_F2 },
    { L"F3"s, VK_F3 },
    { L"F4"s, VK_F4 },
    { L"F5"s, VK_F5 },
    { L"F6"s, VK_F6 },
    { L"F7"s, VK_F7 },
    { L"F8"s, VK_F8 },
    { L"F9"s, VK_F9 },
    { L"F10"s, VK_F10 },
    { L"F11"s, VK_F11 },
    { L"F12"s, VK_F12 },
  };
} // namespace

HotKey ParseHotKey(const std::wstring& hotKey) {
  HotKey hk{};

  std::wregex separator(L"\\s*\\+\\s*");
  auto itr = std::wsregex_token_iterator(hotKey.begin(), hotKey.end(), separator, -1);
  auto end = std::wsregex_token_iterator();
  while (itr != end) {
    std::wstring part(*itr++);
    std::transform(part.begin(), part.end(), part.begin(), [](wchar_t c) {
      return static_cast<wchar_t>(std::towupper(c));
    });
    if (part == L"ALT"s) {
      hk.alt = true;
    } else if (part == L"CTRL"s || part == L"CONTROL"s) {
      hk.control = true;
    } else if (part == L"SHIFT"s) {
      hk.shift = true;
    } else if (part == L"WIN"s) {
      hk.win = true;
    } else {
      const auto itr2 = gKeyNameMap.find(part);
      if (itr2 != gKeyNameMap.end()) {
        hk.vKey = itr2->second;
      }
    }
  }
  return hk;
}
