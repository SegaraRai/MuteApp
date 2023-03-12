#pragma once

#include <Windows.h>

class ToggleMuteError {
public:
  enum class ErrorCode {
    NoAudioSessions,
    TooManyAudioSessions,
  };

private:
  ErrorCode mErrorCode;

public:
  ToggleMuteError(ErrorCode errorCode);

  ErrorCode GetErrorCode() const;
};

bool ToggleMuteByProcessId(DWORD processId);
