//! Render-thread EGL image import. No Godot objects or CPU pixels cross threads.
use godot::obj::Singleton;
use godot::{builtin::Callable, classes::RenderingServer};
use parking_lot::Mutex;
use std::{
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use xcloud_media::{Frame, NativeBuffer};
#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetCurrentDisplay() -> *mut c_void;
    fn eglGetProcAddress(name: *const std::ffi::c_char) -> *mut c_void;
}
#[link(name = "GLESv3")]
unsafe extern "C" {
    fn glBindTexture(target: u32, texture: u32);
    fn glGetIntegerv(name: u32, value: *mut i32);
    fn glFenceSync(condition: u32, flags: u32) -> *mut c_void;
    fn glClientWaitSync(sync: *mut c_void, flags: u32, timeout: u64) -> u32;
    fn glDeleteSync(sync: *mut c_void);
    fn glFinish();
    fn glGetError() -> u32;
}
struct Lease {
    frame: Arc<Frame>,
    image: usize,
    fence: usize,
}
#[derive(Default)]
struct State {
    current: Option<Lease>,
    retired: crate::retirement::Retired<Lease>,
}
unsafe impl Send for State {}
pub struct Presenter {
    state: Mutex<State>,
    pub closed: AtomicBool,
    pub queued: AtomicBool,
    pub imported: AtomicU64,
    pub failed: AtomicBool,
}
impl Presenter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Default::default(),
            closed: AtomicBool::new(false),
            queued: AtomicBool::new(false),
            imported: AtomicU64::new(0),
            failed: AtomicBool::new(false),
        })
    }
    pub fn submit(self: &Arc<Self>, frame: Arc<Frame>, texture: u32) {
        if self.closed.load(Ordering::Acquire) || self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let this = self.clone();
        let callback =
            Callable::from_sync_fn("xrbox_import", move |_: &[&godot::builtin::Variant]| {
                if !this.closed.load(Ordering::Acquire)
                    && unsafe { this.import(frame.clone(), texture) }.is_err()
                {
                    this.failed.store(true, Ordering::Release);
                }
                this.queued.store(false, Ordering::Release);
            });
        RenderingServer::singleton().call_on_render_thread(&callback);
    }
    pub fn close(self: &Arc<Self>) {
        self.closed.store(true, Ordering::Release);
        let this = self.clone();
        let callback = Callable::from_sync_fn(
            "xrbox_release",
            move |_: &[&godot::builtin::Variant]| unsafe {
                glFinish();
                let mut state = this.state.lock();
                if let Some(current) = state.current.take() {
                    destroy(current);
                }
                state.retired.clear(|| {}, |old| destroy(old));
            },
        );
        RenderingServer::singleton().call_on_render_thread(&callback);
    }
    unsafe fn import(&self, frame: Arc<Frame>, texture: u32) -> Result<(), ()> {
        unsafe {
            let NativeBuffer::AndroidImage(ref image) = frame.buf else {
                return Err(());
            };
            let display = eglGetCurrentDisplay();
            if display.is_null() {
                return Err(());
            }
            let client_fn = eglGetProcAddress(c"eglGetNativeClientBufferANDROID".as_ptr());
            let create_fn = eglGetProcAddress(c"eglCreateImageKHR".as_ptr());
            let target_fn = eglGetProcAddress(c"glEGLImageTargetTexture2DOES".as_ptr());
            if client_fn.is_null() || create_fn.is_null() || target_fn.is_null() {
                return Err(());
            }
            let client: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
                std::mem::transmute(client_fn);
            let create: unsafe extern "C" fn(
                *mut c_void,
                *mut c_void,
                u32,
                *mut c_void,
                *const i32,
            ) -> *mut c_void = std::mem::transmute(create_fn);
            let target: unsafe extern "C" fn(u32, *mut c_void) = std::mem::transmute(target_fn);
            let buffer = image.hardware_buffer();
            if buffer.is_null() {
                return Err(());
            }
            let imported = create(
                display,
                std::ptr::null_mut(),
                0x3140,
                client(buffer),
                [0x30d2, 1, 0x3038].as_ptr(),
            );
            if imported.is_null() {
                return Err(());
            }
            let mut binding = 0;
            glGetIntegerv(0x8d67, &mut binding);
            glBindTexture(0x8d65, texture);
            target(0x8d65, imported);
            let error = glGetError();
            glBindTexture(0x8d65, binding as u32);
            if error != 0 {
                glFinish();
                destroy(Lease {
                    frame,
                    image: imported as usize,
                    fence: 0,
                });
                return Err(());
            }
            let mut state = self.state.lock();
            if let Some(mut old) = state.current.take() {
                old.fence = glFenceSync(0x9117, 0) as usize;
                state.retired.push(
                    old,
                    |lease| {
                        if lease.fence == 0 {
                            return false;
                        }
                        matches!(
                            glClientWaitSync(lease.fence as *mut c_void, 0, 0),
                            0x911a | 0x911c
                        )
                    },
                    || glFinish(),
                    |lease| destroy(lease),
                );
            }
            state.current = Some(Lease {
                frame,
                image: imported as usize,
                fence: 0,
            });
            self.imported.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
}
unsafe fn destroy(lease: Lease) {
    unsafe {
        let p = eglGetProcAddress(c"eglDestroyImageKHR".as_ptr());
        if !p.is_null() {
            let destroy: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u32 =
                std::mem::transmute(p);
            destroy(eglGetCurrentDisplay(), lease.image as *mut c_void);
        }
        if lease.fence != 0 {
            glDeleteSync(lease.fence as *mut c_void);
        }
        drop(lease.frame);
    }
}
