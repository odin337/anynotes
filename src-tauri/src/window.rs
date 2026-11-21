use crate::{
    state::{Frame, Settings},
    SharedStore,
};
use std::{sync::mpsc, time::Duration};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

pub fn pin(window: &WebviewWindow, pinned: bool) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as Behavior};

        window.app_handle().set_dock_visibility(!pinned)?;
        // The command and startup handler run on AppKit's main thread.
        let native = unsafe { &*window.ns_window()?.cast::<NSWindow>() };
        let mut behavior = native.collectionBehavior();
        behavior.set(
            Behavior::CanJoinAllSpaces
                | Behavior::CanJoinAllApplications
                | Behavior::FullScreenAuxiliary,
            pinned,
        );
        if pinned {
            behavior.remove(Behavior::MoveToActiveSpace | Behavior::Primary | Behavior::Auxiliary);
            native.setHidesOnDeactivate(false);
        }
        behavior.set(Behavior::FullScreenPrimary, !pinned);
        native.setCollectionBehavior(behavior);
    }
    window.set_always_on_top(pinned)
}

fn current_frame(window: &WebviewWindow) -> tauri::Result<Frame> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    Ok(Frame {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

fn intersection(frame: Frame, area: Frame) -> i64 {
    fn overlap(start: i32, length: u32, other: i32, other_length: u32) -> i64 {
        let end = i64::from(start) + i64::from(length);
        let other_end = i64::from(other) + i64::from(other_length);
        (end.min(other_end) - i64::from(start.max(other))).max(0)
    }
    overlap(frame.x, frame.width, area.x, area.width).saturating_mul(overlap(
        frame.y,
        frame.height,
        area.y,
        area.height,
    ))
}

fn work_area(monitor: tauri::Monitor) -> Frame {
    let area = monitor.work_area();
    Frame {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
    }
}

fn fit(frame: Frame, area: Frame) -> Frame {
    fn axis(start: i32, size: u32, available_start: i32, available_size: u32) -> (i32, u32) {
        let size = size.min(available_size);
        let maximum = i64::from(available_start) + i64::from(available_size - size);
        (
            i64::from(start).clamp(i64::from(available_start), maximum) as i32,
            size,
        )
    }
    let (x, width) = axis(frame.x, frame.width, area.x, area.width);
    let (y, height) = axis(frame.y, frame.height, area.y, area.height);
    Frame {
        x,
        y,
        width,
        height,
    }
}

fn apply_frame(window: &WebviewWindow, desired: Frame) -> tauri::Result<()> {
    let area = window
        .available_monitors()?
        .into_iter()
        .map(work_area)
        .map(|area| (intersection(desired, area), area))
        .filter(|(overlap, _)| *overlap > 0)
        .max_by_key(|(overlap, _)| *overlap)
        .map(|(_, area)| area)
        .or(window.primary_monitor()?.map(work_area));
    let desired = area.map_or(desired, |area| fit(desired, area));
    let current = current_frame(window)?;

    if current.width != desired.width || current.height != desired.height {
        let inner = window.inner_size()?;
        let chrome_width = current.width.saturating_sub(inner.width);
        let chrome_height = current.height.saturating_sub(inner.height);
        window.set_size(PhysicalSize::new(
            desired.width.saturating_sub(chrome_width).max(1),
            desired.height.saturating_sub(chrome_height).max(1),
        ))?;
    }
    if current.x != desired.x || current.y != desired.y {
        window.set_position(PhysicalPosition::new(desired.x, desired.y))?;
    }
    Ok(())
}

pub fn restore(window: &WebviewWindow, settings: &Settings) -> tauri::Result<()> {
    let frame = if let Some(frame) = settings.frame {
        frame
    } else if let Some(bounds) = settings.bounds {
        let scale = window.scale_factor()?;
        // Older releases saved logical content bounds, without native window chrome.
        let compact_default = matches!((bounds.width, bounds.height), (902, 644) | (548, 416));
        let mut frame = current_frame(window)?;
        frame.x = (f64::from(bounds.x) * scale).round() as i32;
        frame.y = (f64::from(bounds.y) * scale).round() as i32;
        if !compact_default {
            let inner = window.inner_size()?;
            frame.width = frame.width.saturating_sub(inner.width)
                + (f64::from(bounds.width.max(266)) * scale).round() as u32;
            frame.height = frame.height.saturating_sub(inner.height)
                + (f64::from(bounds.height.max(225)) * scale).round() as u32;
        }
        frame
    } else {
        return Ok(());
    };
    apply_frame(window, frame)
}

fn in_normal_mode(window: &WebviewWindow) -> tauri::Result<bool> {
    Ok(!window.is_fullscreen()? && !window.is_maximized()? && !window.is_minimized()?)
}

fn save_current_frame(window: &WebviewWindow) -> Result<(), String> {
    if in_normal_mode(window).map_err(|error| error.to_string())? {
        let frame = current_frame(window).map_err(|error| error.to_string())?;
        window
            .state::<SharedStore>()
            .lock()
            .unwrap()
            .save_frame(frame)?;
    }
    Ok(())
}

pub fn reveal(window: &WebviewWindow) -> tauri::Result<()> {
    window.unminimize()?;
    if in_normal_mode(window)? {
        apply_frame(window, current_frame(window)?)?;
    }
    window.show()?;
    window.set_focus()
}

fn pointer_down() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSEvent::pressedMouseButtons() != 0
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON,
        };
        // Query mouse buttons only, so moving between monitors can finish before clamping.
        unsafe {
            GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0
                || GetAsyncKeyState(i32::from(VK_RBUTTON)) < 0
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

enum Signal {
    Changed,
    Closed,
}

pub fn track_bounds(window: &WebviewWindow) {
    let (send, receive) = mpsc::channel();
    let watched = window.clone();
    let schedule = send.clone();
    std::thread::spawn(move || {
        let settle_delay = Duration::from_millis(180);
        'events: while let Ok(Signal::Changed) = receive.recv() {
            loop {
                match receive.recv_timeout(settle_delay) {
                    Ok(Signal::Changed) => {}
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    _ => break 'events,
                }
            }
            let window = watched.clone();
            let schedule = schedule.clone();
            let _ = watched.run_on_main_thread(move || {
                if pointer_down() {
                    let _ = schedule.send(Signal::Changed);
                    return;
                }
                let result = (|| {
                    if in_normal_mode(&window)? {
                        apply_frame(&window, current_frame(&window)?)?;
                    }
                    Ok::<_, tauri::Error>(())
                })();
                if let Err(error) = result {
                    eprintln!("{error}");
                } else if let Err(error) = save_current_frame(&window) {
                    eprintln!("{error}");
                }
            });
        }
    });

    let watched = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(_)
        | WindowEvent::Resized(_)
        | WindowEvent::ScaleFactorChanged { .. }
        | WindowEvent::Focused(true) => {
            let _ = send.send(Signal::Changed);
        }
        WindowEvent::CloseRequested { .. } => {
            if let Err(error) = save_current_frame(&watched) {
                eprintln!("{error}");
            }
        }
        WindowEvent::Destroyed => {
            let _ = send.send(Signal::Closed);
        }
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::{fit, intersection, Frame};

    #[test]
    fn window_fits_left_monitor_with_negative_coordinates() {
        let left = Frame {
            x: -1280,
            y: 30,
            width: 1280,
            height: 760,
        };
        let right = Frame {
            x: 0,
            y: 30,
            width: 1920,
            height: 1040,
        };
        let frame = Frame {
            x: -1350,
            y: 680,
            width: 420,
            height: 320,
        };
        assert!(intersection(frame, left) > intersection(frame, right));
        assert_eq!(
            fit(frame, left),
            Frame {
                x: -1280,
                y: 470,
                ..frame
            }
        );
        let gone = Frame {
            x: 3210,
            y: -500,
            ..frame
        };
        assert_eq!(
            fit(gone, right),
            Frame {
                x: 1500,
                y: 30,
                ..frame
            }
        );
    }
}
