#pragma once

#include <Windows.h>

class ToggleMuteError {
public:
  enum class ErrorCode {
    NoAudioSessions,
  };

private:
  ErrorCode mErrorCode;

public:
  ToggleMuteError(ErrorCode errorCode);

  ErrorCode GetErrorCode() const;
};

/**
 * @brief Toggle mute for all audio sessions for a given process id
 * @param processId PID
 * @return true if un-muted, false if muted
 */
bool ToggleMuteByProcessId(DWORD processId);
