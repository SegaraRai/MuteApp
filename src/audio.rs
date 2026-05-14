use anyhow::{Result, bail};
use windows::Win32::Media::Audio::{
    AudioSessionStateExpired, DEVICE_STATE_ACTIVE, IAudioSessionControl, IAudioSessionControl2,
    IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::Interface;

fn audio_sessions_for_process(process_id: u32) -> Result<Vec<IAudioSessionControl2>> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let devices = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };
    let device_count = unsafe { devices.GetCount()? };
    let mut sessions = Vec::new();

    for device_index in 0..device_count {
        let device = unsafe { devices.Item(device_index)? };
        let session_manager: IAudioSessionManager2 = unsafe { device.Activate(CLSCTX_ALL, None)? };
        let session_enumerator = unsafe { session_manager.GetSessionEnumerator()? };
        let session_count = unsafe { session_enumerator.GetCount()? };

        for session_index in 0..session_count {
            let session: IAudioSessionControl =
                unsafe { session_enumerator.GetSession(session_index)? };
            let session2: IAudioSessionControl2 = session.cast()?;

            let state = unsafe { session2.GetState()? };
            if state == AudioSessionStateExpired {
                continue;
            }

            let system_session = unsafe { session2.IsSystemSoundsSession() };
            if system_session.0 == 0 {
                continue;
            }
            if system_session.0 < 0 {
                system_session.ok()?;
            }

            let session_pid = unsafe { session2.GetProcessId()? };
            if session_pid == process_id {
                sessions.push(session2);
            }
        }
    }

    Ok(sessions)
}

pub fn toggle_mute_by_process_id(process_id: u32) -> Result<bool> {
    let sessions = audio_sessions_for_process(process_id)?;
    if sessions.is_empty() {
        bail!("no audio sessions for process {process_id}");
    }

    let mut all_muted = true;
    for session in &sessions {
        let volume: ISimpleAudioVolume = session.cast()?;
        let muted = unsafe { volume.GetMute()? };
        if !muted.as_bool() {
            all_muted = false;
            break;
        }
    }

    let new_muted = !all_muted;
    for session in &sessions {
        let volume: ISimpleAudioVolume = session.cast()?;
        unsafe { volume.SetMute(new_muted, std::ptr::null())? };
    }

    Ok(new_muted)
}
