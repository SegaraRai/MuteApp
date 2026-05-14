use anyhow::{Context, Result, bail};
use std::ffi::c_void;
use std::time::Instant;
use windows::Win32::Foundation::{
    COLORREF, CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, POINT, RECT,
    SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HGDIOBJ,
    SelectObject,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId, ReleaseMutex};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWL_EXSTYLE, GetForegroundWindow, GetMessageW, GetWindowLongPtrW,
    GetWindowRect, GetWindowThreadProcessId, HWND_TOPMOST, KillTimer, MB_ICONERROR, MB_OK,
    MB_SETFOREGROUND, MSG, MessageBoxW, PM_NOREMOVE, PeekMessageW, PostQuitMessage,
    PostThreadMessageW, RegisterClassExW, SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_SHOWWINDOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage,
    ULW_ALPHA, UpdateLayeredWindow, WM_DESTROY, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{BOOL, PCWSTR, w};

pub const EXIT_MESSAGE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;
pub const TIMER_MESSAGE: u32 = WM_TIMER;
const HIDE_TIMER_ID: usize = 1;
const ANIMATION_TIMER_ID: usize = 2;
const ANIMATION_FRAME_MS: u32 = 16;
const FADE_IN_DURATION_MS: u64 = 60;
const FADE_OUT_DURATION_MS: u64 = 120;
const MAX_WINDOW_ALPHA: u8 = 255;
const MIN_ANIMATION_PROGRESS: f32 = 0.0;
const MAX_ANIMATION_PROGRESS: f32 = 1.0;

pub struct ComApartment;

pub fn enable_per_monitor_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

fn system_animations_enabled() -> bool {
    let mut enabled = BOOL(0);
    let result = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    result.is_ok() && enabled.as_bool()
}

impl ComApartment {
    pub fn init() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub enum InstanceLock {
    Acquired(HANDLE),
    AlreadyRunning,
}

impl InstanceLock {
    pub fn acquire() -> Result<Self> {
        let handle = unsafe { CreateMutexW(None, true, w!("MuteAppMutex"))? };
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle).ok();
            }
            return Ok(Self::AlreadyRunning);
        }
        Ok(Self::Acquired(handle))
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        if let Self::Acquired(handle) = *self {
            unsafe {
                ReleaseMutex(handle).ok();
                CloseHandle(handle).ok();
            }
        }
    }
}

pub struct OverlayWindow {
    hwnd: HWND,
    animation: Option<FadeAnimation>,
    current_progress: f32,
}

impl OverlayWindow {
    pub fn new() -> Result<Self> {
        register_window_class(w!("MuteAppOverlay"))?;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW,
                w!("MuteAppOverlay"),
                w!("MuteApp"),
                WS_POPUP,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1,
                1,
                None,
                None,
                Some(module_instance()?),
                None,
            )?
        };

        unsafe {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                ex_style
                    | WS_EX_TOOLWINDOW.0 as isize
                    | WS_EX_NOACTIVATE.0 as isize
                    | WS_EX_TRANSPARENT.0 as isize,
            );
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        Ok(Self {
            hwnd,
            animation: None,
            current_progress: MIN_ANIMATION_PROGRESS,
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn show(
        &mut self,
        center_x: i32,
        center_y: i32,
        size: u32,
        rgba: &[u8],
        duration_ms: u32,
    ) -> Result<()> {
        let animations_enabled = system_animations_enabled();
        let start_progress = if animations_enabled {
            self.current_animation_progress(Instant::now())
        } else {
            MAX_ANIMATION_PROGRESS
        };

        unsafe {
            let _ = KillTimer(Some(self.hwnd), HIDE_TIMER_ID);
            let _ = KillTimer(Some(self.hwnd), ANIMATION_TIMER_ID);
        }
        self.animation = None;
        self.current_progress = start_progress;

        update_layered_window(
            self.hwnd,
            center_x,
            center_y,
            size,
            rgba,
            opacity_from_progress(start_progress),
        )?;
        unsafe {
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                center_x - size as i32 / 2,
                center_y - size as i32 / 2,
                size as i32,
                size as i32,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )?;
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            if animations_enabled {
                self.animation = Some(FadeAnimation {
                    center_x,
                    center_y,
                    size,
                    rgba: rgba.to_vec(),
                    start_progress,
                    hold_duration_ms: u64::from(duration_ms),
                    started_at: Instant::now(),
                });
                SetTimer(
                    Some(self.hwnd),
                    ANIMATION_TIMER_ID,
                    ANIMATION_FRAME_MS,
                    None,
                );
            } else {
                SetTimer(Some(self.hwnd), HIDE_TIMER_ID, duration_ms, None);
            }
        }
        Ok(())
    }

    pub fn handle_timer(&mut self, timer_id: usize) {
        match timer_id {
            HIDE_TIMER_ID => self.hide(),
            ANIMATION_TIMER_ID => match self.update_animation() {
                Ok(()) => {}
                Err(_) => self.hide(),
            },
            _ => {}
        }
    }

    pub fn hide(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), HIDE_TIMER_ID);
            let _ = KillTimer(Some(self.hwnd), ANIMATION_TIMER_ID);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.animation = None;
        self.current_progress = MIN_ANIMATION_PROGRESS;
    }

    fn update_animation(&mut self) -> Result<()> {
        let Some(animation) = &self.animation else {
            return Ok(());
        };
        let now = Instant::now();
        if animation.is_finished(now) {
            self.hide();
            return Ok(());
        }

        let progress = animation.progress(now);
        update_layered_window(
            self.hwnd,
            animation.center_x,
            animation.center_y,
            animation.size,
            &animation.rgba,
            opacity_from_progress(progress),
        )?;
        self.current_progress = progress;
        Ok(())
    }

    fn current_animation_progress(&self, now: Instant) -> f32 {
        match &self.animation {
            Some(animation) if !animation.is_finished(now) => animation.progress(now),
            _ => self.current_progress,
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        self.hide();
        unsafe {
            DestroyWindow(self.hwnd).ok();
        }
    }
}

struct FadeAnimation {
    center_x: i32,
    center_y: i32,
    size: u32,
    rgba: Vec<u8>,
    start_progress: f32,
    hold_duration_ms: u64,
    started_at: Instant,
}

impl FadeAnimation {
    fn progress(&self, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.started_at);
        let elapsed_ms = elapsed.as_millis() as u64;
        let fade_in_duration = self.fade_in_duration_ms();
        if elapsed_ms < fade_in_duration {
            return interpolate_progress(
                self.start_progress,
                MAX_ANIMATION_PROGRESS,
                elapsed_ms,
                fade_in_duration,
            );
        }

        let fade_out_start = fade_in_duration + self.hold_duration_ms;
        if elapsed_ms < fade_out_start {
            return MAX_ANIMATION_PROGRESS;
        }

        let fade_out_elapsed = elapsed_ms.saturating_sub(fade_out_start);
        interpolate_progress(
            MAX_ANIMATION_PROGRESS,
            MIN_ANIMATION_PROGRESS,
            fade_out_elapsed.min(FADE_OUT_DURATION_MS),
            FADE_OUT_DURATION_MS,
        )
    }

    fn is_finished(&self, now: Instant) -> bool {
        let elapsed = now.saturating_duration_since(self.started_at);
        elapsed.as_millis() as u64
            >= self.fade_in_duration_ms() + self.hold_duration_ms + FADE_OUT_DURATION_MS
    }

    fn fade_in_duration_ms(&self) -> u64 {
        if self.start_progress >= MAX_ANIMATION_PROGRESS {
            return 0;
        }
        (FADE_IN_DURATION_MS as f32 * (MAX_ANIMATION_PROGRESS - self.start_progress)).ceil() as u64
    }
}

fn interpolate_progress(start: f32, end: f32, elapsed_ms: u64, duration_ms: u64) -> f32 {
    if duration_ms == 0 {
        return end;
    }
    let elapsed = elapsed_ms.min(duration_ms) as f32 / duration_ms as f32;
    (start + (end - start) * elapsed).clamp(MIN_ANIMATION_PROGRESS, MAX_ANIMATION_PROGRESS)
}

fn opacity_from_progress(progress: f32) -> u8 {
    (progress.clamp(MIN_ANIMATION_PROGRESS, MAX_ANIMATION_PROGRESS) * f32::from(MAX_WINDOW_ALPHA))
        .round() as u8
}

pub fn attach_parent_console_if_any() {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

pub fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

pub fn post_exit_message(thread_id: u32) {
    unsafe {
        let _ = PostThreadMessageW(thread_id, EXIT_MESSAGE, WPARAM(0), LPARAM(0));
    }
}

pub fn ensure_message_queue() {
    let mut msg = MSG::default();
    unsafe {
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
    }
}

pub fn next_message() -> Option<MSG> {
    let mut msg = MSG::default();
    let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
    (result.0 > 0).then_some(msg)
}

pub fn translate_and_dispatch(message: &MSG) {
    unsafe {
        let _ = TranslateMessage(message);
        DispatchMessageW(message);
    }
}

pub fn foreground_process() -> Option<(HWND, u32)> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }

    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    (process_id != 0).then_some((hwnd, process_id))
}

pub fn foreground_center_and_dpi(hwnd: HWND) -> Option<(i32, i32, u32)> {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect).is_err() } {
        return None;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    Some((
        (rect.left + rect.right) / 2,
        (rect.top + rect.bottom) / 2,
        dpi,
    ))
}

pub fn beep() {
    unsafe {
        MessageBeep(MB_OK).ok();
    }
}

pub fn error_box(message: &str) {
    let wide_message = message.encode_utf16().chain([0]).collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(wide_message.as_ptr()),
            w!("MuteApp"),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

pub fn exe_config_path() -> Result<std::path::PathBuf> {
    let exe = std::env::current_exe().context("failed to resolve executable path")?;
    let directory = exe
        .parent()
        .context("executable path did not have a parent directory")?;
    Ok(directory.join("MuteApp.cfg"))
}

fn register_window_class(class_name: PCWSTR) -> Result<()> {
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: module_instance()?,
        lpszClassName: class_name,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassExW(&class) };
    if atom == 0 {
        bail!("failed to register overlay window class");
    }
    Ok(())
}

fn module_instance() -> Result<windows::Win32::Foundation::HINSTANCE> {
    let module = unsafe { GetModuleHandleW(None)? };
    Ok(windows::Win32::Foundation::HINSTANCE(module.0))
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if msg == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return windows::Win32::Foundation::LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn update_layered_window(
    hwnd: HWND,
    center_x: i32,
    center_y: i32,
    size: u32,
    rgba: &[u8],
    opacity: u8,
) -> Result<()> {
    if rgba.len() != (size * size * 4) as usize {
        bail!("invalid overlay image size");
    }

    let mut bgra = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        let alpha = pixel[3] as u16;
        let r = ((pixel[0] as u16 * alpha) / 255) as u8;
        let g = ((pixel[1] as u16 * alpha) / 255) as u8;
        let b = ((pixel[2] as u16 * alpha) / 255) as u8;
        bgra.extend_from_slice(&[b, g, r, pixel[3]]);
    }

    let hdc = unsafe { CreateCompatibleDC(None) };
    if hdc.0.is_null() {
        bail!("failed to create memory DC");
    }
    let bitmap = DibBitmap::new(size, &bgra)?;
    let old_object = unsafe { SelectObject(hdc, HGDIOBJ(bitmap.handle.0)) };

    let dst = POINT {
        x: center_x - size as i32 / 2,
        y: center_y - size as i32 / 2,
    };
    let src = POINT { x: 0, y: 0 };
    let extent = SIZE {
        cx: size as i32,
        cy: size as i32,
    };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: opacity,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    let result = unsafe {
        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&extent),
            Some(hdc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    };

    unsafe {
        SelectObject(hdc, old_object);
        let _ = DeleteDC(hdc);
    }

    result.context("failed to update layered overlay window")
}

struct DibBitmap {
    handle: HBITMAP,
}

impl DibBitmap {
    fn new(size: u32, bgra: &[u8]) -> Result<Self> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size as i32,
                biHeight: -(size as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let handle = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
        if bits.is_null() {
            bail!("failed to allocate DIB bits");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), bgra.len());
        }
        Ok(Self { handle })
    }
}

impl Drop for DibBitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.handle.0));
        }
    }
}
