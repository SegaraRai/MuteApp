#include "pch.h"

#include "Audio.hpp"

#include <vector>

#include <Windows.h>

#include <audiopolicy.h>
#include <mmdeviceapi.h>

namespace {
  std::vector<winrt::com_ptr<IAudioSessionControl2>> GetAudioSessionControls(DWORD processId) {
    std::vector<winrt::com_ptr<IAudioSessionControl2>> sessions;

    // enumerate active audio endpoint devices for rendering (playback)
    auto mmDevEnum = winrt::create_instance<IMMDeviceEnumerator>(__uuidof(MMDeviceEnumerator), CLSCTX_ALL);
    winrt::com_ptr<IMMDeviceCollection> mmDevColl;
    winrt::check_hresult(mmDevEnum->EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE, mmDevColl.put()));

    UINT numMMDevices = 0;
    winrt::check_hresult(mmDevColl->GetCount(&numMMDevices));
    for (UINT i = 0; i < numMMDevices; i++) {
      winrt::com_ptr<IMMDevice> mmDevice;
      winrt::check_hresult(mmDevColl->Item(i, mmDevice.put()));

      winrt::com_ptr<IAudioSessionManager2> audioSessMgr2;
      winrt::check_hresult(
        mmDevice->Activate(__uuidof(IAudioSessionManager2), CLSCTX_ALL, NULL, audioSessMgr2.put_void()));

      winrt::com_ptr<IAudioSessionEnumerator> audioSessEnum;
      winrt::check_hresult(audioSessMgr2->GetSessionEnumerator(audioSessEnum.put()));

      int numSessions = 0;
      winrt::check_hresult(audioSessEnum->GetCount(&numSessions));
      for (int j = 0; j < numSessions; j++) {
        winrt::com_ptr<IAudioSessionControl> audioSessCtrl;
        winrt::check_hresult(audioSessEnum->GetSession(j, audioSessCtrl.put()));

        auto audioSessCtrl2 = audioSessCtrl.as<IAudioSessionControl2>();

        // ignore expired sessions
        AudioSessionState audioSessState;
        winrt::check_hresult(audioSessCtrl2->GetState(&audioSessState));
        if (audioSessState == AudioSessionStateExpired) {
          continue;
        }

        // ignore system sounds session
        {
          const auto hr = audioSessCtrl2->IsSystemSoundsSession();
          winrt::check_hresult(hr);
          if (hr == S_OK) {
            continue;
          }
        }

        // filter by process id (if specified)
        if (processId != 0) {
          DWORD pid;
          winrt::check_hresult(audioSessCtrl2->GetProcessId(&pid));
          if (pid != processId) {
            continue;
          }
        }

        sessions.push_back(audioSessCtrl2);
      }
    }

    return sessions;
  }
} // namespace

ToggleMuteError::ToggleMuteError(ErrorCode errorCode) : mErrorCode(errorCode) {}

ToggleMuteError::ErrorCode ToggleMuteError::GetErrorCode() const {
  return mErrorCode;
}

bool ToggleMuteByProcessId(DWORD processId) {
  const auto sessions = GetAudioSessionControls(processId);
  if (sessions.empty()) {
    throw ToggleMuteError(ToggleMuteError::ErrorCode::NoAudioSessions);
  }

  // get mute state
  // if any session is not muted, then mute all sessions
  bool muted = true;
  for (const auto& session : sessions) {
    BOOL sessionMuted = FALSE;
    auto simpleAudioVolume = session.as<ISimpleAudioVolume>();
    winrt::check_hresult(simpleAudioVolume->GetMute(&sessionMuted));
    if (!sessionMuted) {
      muted = false;
      break;
    }
  }

  // toggle mute
  for (const auto& session : sessions) {
    auto simpleAudioVolume = session.as<ISimpleAudioVolume>();
    winrt::check_hresult(simpleAudioVolume->SetMute(muted ? FALSE : TRUE, NULL));
  }

  return muted;
}
