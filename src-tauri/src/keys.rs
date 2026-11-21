use crate::{state::Action, SharedStore};
use std::sync::mpsc::{self, SyncSender};
use tauri::{Emitter, Manager, WebviewWindow};

pub struct Keys(SyncSender<i64>);

impl Keys {
    pub fn new(window: &WebviewWindow) -> Self {
        let (send, receive) = mpsc::sync_channel(64);
        let watched = window.clone();
        std::thread::spawn(move || {
            while let Ok(step) = receive.recv() {
                let window = watched.clone();
                let _ = watched.run_on_main_thread(move || {
                    if window.is_focused().unwrap_or(true) {
                        return;
                    }
                    let store = window.state::<SharedStore>();
                    let mut store = store.lock().unwrap();
                    if !store.settings().pinned {
                        return;
                    }
                    if let Ok(update) = store.apply(Action::Step { step }) {
                        let _ = window.emit("counter-changed", &update["data"]);
                    }
                });
            }
        });
        Self(send)
    }

    pub fn enable(&self) -> bool {
        platform::enable(self.0.clone())
    }
}

pub use platform::{disable, request_access};

#[cfg(target_os = "macos")]
mod platform {
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop, CFRunLoopSource};
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult, EventField, KeyCode,
    };
    use std::{cell::RefCell, sync::mpsc::SyncSender};

    unsafe extern "C" {
        fn CGPreflightListenEventAccess() -> bool;
        fn CGRequestListenEventAccess() -> bool;
    }

    thread_local! {
        static MONITOR: RefCell<Option<(CGEventTap<'static>, CFRunLoopSource)>> = const { RefCell::new(None) };
    }

    fn arrow_step(key: u16, flags: CGEventFlags) -> Option<i64> {
        let modifiers = CGEventFlags::CGEventFlagShift
            | CGEventFlags::CGEventFlagControl
            | CGEventFlags::CGEventFlagAlternate
            | CGEventFlags::CGEventFlagCommand;
        if flags.intersects(modifiers) {
            return None;
        }
        match key {
            KeyCode::UP_ARROW | KeyCode::RIGHT_ARROW => Some(1),
            KeyCode::DOWN_ARROW | KeyCode::LEFT_ARROW => Some(-1),
            _ => None,
        }
    }

    pub fn enable(send: SyncSender<i64>) -> bool {
        MONITOR.with(|slot| {
            if !unsafe { CGPreflightListenEventAccess() } {
                disable();
                return false;
            }
            if slot.borrow().is_some() {
                return true;
            }
            let tap = CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::TailAppendEventTap,
                CGEventTapOptions::ListenOnly,
                vec![CGEventType::KeyDown],
                move |_, kind, event| {
                    if matches!(kind, CGEventType::KeyDown) {
                        let key = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
                            as u16;
                        if let Some(step) = arrow_step(key, event.get_flags()) {
                            let _ = send.try_send(step);
                        }
                    }
                    CallbackResult::Keep
                },
            );
            let Ok(tap) = tap else { return false };
            let Ok(source) = tap.mach_port().create_runloop_source(0) else {
                return false;
            };
            CFRunLoop::get_main().add_source(&source, unsafe { kCFRunLoopCommonModes });
            tap.enable();
            slot.replace(Some((tap, source)));
            true
        })
    }

    pub fn disable() {
        MONITOR.with(|slot| {
            if let Some((_tap, source)) = slot.take() {
                CFRunLoop::get_main().remove_source(&source, unsafe { kCFRunLoopCommonModes });
            }
        });
    }

    pub fn request_access() {
        if !unsafe { CGPreflightListenEventAccess() } {
            unsafe { CGRequestListenEventAccess() };
        }
    }

    #[cfg(test)]
    #[test]
    fn navigation_flags_allow_arrows_but_control_shortcuts_are_ignored() {
        let navigation = CGEventFlags::CGEventFlagNumericPad | CGEventFlags::CGEventFlagSecondaryFn;
        assert_eq!(arrow_step(KeyCode::UP_ARROW, navigation), Some(1));
        assert_eq!(arrow_step(KeyCode::DOWN_ARROW, navigation), Some(-1));
        assert_eq!(arrow_step(KeyCode::RIGHT_ARROW, navigation), Some(1));
        assert_eq!(arrow_step(KeyCode::LEFT_ARROW, navigation), Some(-1));
        assert_eq!(
            arrow_step(
                KeyCode::RIGHT_ARROW,
                navigation | CGEventFlags::CGEventFlagControl
            ),
            None
        );
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::{cell::RefCell, ptr::null_mut, sync::mpsc::SyncSender};
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_CONTROL, VK_DOWN, VK_LEFT, VK_LWIN, VK_MENU, VK_RIGHT,
                VK_RWIN, VK_SHIFT, VK_UP,
            },
            WindowsAndMessaging::{
                CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
                WH_KEYBOARD_LL, WM_KEYDOWN,
            },
        },
    };

    thread_local! {
        static MONITOR: RefCell<Option<(HHOOK, SyncSender<i64>)>> = const { RefCell::new(None) };
    }

    unsafe extern "system" fn observe(code: i32, kind: usize, data: isize) -> isize {
        if code == 0 && kind == WM_KEYDOWN as usize {
            let key = unsafe { (*(data as *const KBDLLHOOKSTRUCT)).vkCode };
            let step = match key as u16 {
                VK_UP | VK_RIGHT => Some(1),
                VK_DOWN | VK_LEFT => Some(-1),
                _ => None,
            };
            if let Some(step) = step {
                let modified = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                    .iter()
                    .any(|key| unsafe { GetAsyncKeyState(i32::from(*key)) < 0 });
                if !modified {
                    MONITOR.with(|slot| {
                        if let Ok(monitor) = slot.try_borrow() {
                            if let Some((_, send)) = monitor.as_ref() {
                                let _ = send.try_send(step);
                            }
                        }
                    });
                }
            }
        }
        // The focused application always receives the original key event.
        unsafe { CallNextHookEx(null_mut(), code, kind, data) }
    }

    pub fn enable(send: SyncSender<i64>) -> bool {
        MONITOR.with(|slot| {
            if slot.borrow().is_some() {
                return true;
            }
            let hook = unsafe {
                SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(observe),
                    GetModuleHandleW(std::ptr::null()),
                    0,
                )
            };
            if hook.is_null() {
                return false;
            }
            slot.replace(Some((hook, send)));
            true
        })
    }

    pub fn disable() {
        MONITOR.with(|slot| {
            if let Some((hook, _)) = slot.take() {
                unsafe { UnhookWindowsHookEx(hook) };
            }
        });
    }

    pub fn request_access() {}
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use std::sync::mpsc::SyncSender;
    pub fn enable(_: SyncSender<i64>) -> bool {
        false
    }
    pub fn disable() {}
    pub fn request_access() {}
}
