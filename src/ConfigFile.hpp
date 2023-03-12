#pragma once

#include <fstream>
#include <map>
#include <optional>
#include <shared_mutex>
#include <string>

class ConfigFile {
  mutable std::shared_mutex mMutex;
  std::map<std::wstring, std::wstring> mConfigMap;
  std::wstring mFilepath;

  void Load();

public:
  ConfigFile(const std::wstring& filepath);
  ~ConfigFile();

  void Save();

  std::optional<std::wstring> GetStr(const std::wstring& key) const;
  std::optional<int> GetInt(const std::wstring& key) const;
  void Set(const std::wstring& key, const std::wstring& value, bool skipIfExists = false);
  void Set(const std::wstring& key, int value, bool skipIfExists = false);
};
