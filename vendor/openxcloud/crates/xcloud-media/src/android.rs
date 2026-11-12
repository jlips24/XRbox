//! Android NDK H.264 surface decoder. Pixels remain in ImageReader GPU buffers.
use crate::{
    ColorInfo, Error, Frame, FrameSink, NativeBuffer, Result, VideoDecoder,
    annexb::{self, ParameterSets},
};
use std::{
    ffi::{c_char, c_void},
    ptr,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicI32, Ordering},
    },
    time::Instant,
};

static STATUS: AtomicI32 = AtomicI32::new(0);
pub fn status() -> i32 {
    STATUS.load(Ordering::Relaxed)
}
pub fn selected_codec() -> String {
    DECODER_NAME.get().cloned().unwrap_or_default()
}
static DECODER_NAME: OnceLock<String> = OnceLock::new();
/// Name selected by Android MediaCodecInfo.isHardwareAccelerated (API 29+).
pub fn set_decoder_name(name: String) {
    let _ = DECODER_NAME.set(name);
}

#[link(name = "mediandk")]
unsafe extern "C" {
    fn AMediaCodec_createCodecByName(name: *const c_char) -> *mut c_void;
    fn AMediaCodec_delete(codec: *mut c_void) -> i32;
    fn AMediaCodec_configure(
        codec: *mut c_void,
        format: *const c_void,
        window: *mut c_void,
        crypto: *mut c_void,
        flags: u32,
    ) -> i32;
    fn AMediaCodec_start(codec: *mut c_void) -> i32;
    fn AMediaCodec_stop(codec: *mut c_void) -> i32;
    fn AMediaCodec_dequeueInputBuffer(codec: *mut c_void, timeout: i64) -> isize;
    fn AMediaCodec_getInputBuffer(codec: *mut c_void, index: usize, size: *mut usize) -> *mut u8;
    fn AMediaCodec_queueInputBuffer(
        codec: *mut c_void,
        index: usize,
        offset: usize,
        size: usize,
        time: u64,
        flags: u32,
    ) -> i32;
    fn AMediaCodec_dequeueOutputBuffer(
        codec: *mut c_void,
        info: *mut BufferInfo,
        timeout: i64,
    ) -> isize;
    fn AMediaCodec_releaseOutputBuffer(codec: *mut c_void, index: usize, render: bool) -> i32;
    fn AMediaFormat_new() -> *mut c_void;
    fn AMediaFormat_delete(format: *mut c_void);
    fn AMediaFormat_setString(format: *mut c_void, name: *const c_char, value: *const c_char);
    fn AMediaFormat_setInt32(format: *mut c_void, name: *const c_char, value: i32);
    fn AMediaFormat_setBuffer(
        format: *mut c_void,
        name: *const c_char,
        data: *const c_void,
        size: usize,
    );
    fn AImageReader_newWithUsage(
        width: i32,
        height: i32,
        format: i32,
        usage: u64,
        max: i32,
        reader: *mut *mut c_void,
    ) -> i32;
    fn AImageReader_getWindow(reader: *mut c_void, window: *mut *mut c_void) -> i32;
    fn AImageReader_acquireLatestImage(reader: *mut c_void, image: *mut *mut c_void) -> i32;
    fn AImageReader_delete(reader: *mut c_void);
    fn AImage_delete(image: *mut c_void);
    fn AImage_getHardwareBuffer(image: *const c_void, buffer: *mut *mut c_void) -> i32;
    fn AImage_getTimestamp(image: *const c_void, timestamp: *mut i64) -> i32;
}
#[repr(C)]
struct BufferInfo {
    offset: i32,
    size: i32,
    time: i64,
    flags: u32,
}
struct Reader(*mut c_void);
// SAFETY: NDK image reader operations are serialized by the decoder; retained
// images may be released by another thread. Arc keeps the reader alive until then.
unsafe impl Send for Reader {}
unsafe impl Sync for Reader {}
impl Drop for Reader {
    fn drop(&mut self) {
        unsafe { AImageReader_delete(self.0) }
    }
}
/// Retained image lease. Never recycle while the renderer can still sample it.
pub struct Image {
    raw: *mut c_void,
    _reader: Arc<Reader>,
    pub backend: String,
}
unsafe impl Send for Image {}
unsafe impl Sync for Image {}
impl Image {
    pub fn hardware_buffer(&self) -> *mut c_void {
        let mut p = ptr::null_mut();
        unsafe { AImage_getHardwareBuffer(self.raw, &mut p) };
        p
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        unsafe { AImage_delete(self.raw) }
    }
}

pub struct AndroidDecoder {
    codec: *mut c_void,
    reader: Option<Arc<Reader>>,
    sets: ParameterSets,
    sink: FrameSink,
    width: u32,
    height: u32,
    epoch: Instant,
    pending: std::collections::VecDeque<(i64, Instant, Instant)>,
}
// The codec is created/used/destroyed by DecodeThread, never concurrently.
unsafe impl Send for AndroidDecoder {}
impl AndroidDecoder {
    pub fn new(sink: FrameSink) -> Self {
        Self {
            codec: ptr::null_mut(),
            reader: None,
            sets: Default::default(),
            sink,
            width: 0,
            height: 0,
            epoch: Instant::now(),
            pending: Default::default(),
        }
    }
    fn reset(&mut self) {
        STATUS.store(0, Ordering::Relaxed);
        if !self.codec.is_null() {
            unsafe {
                AMediaCodec_stop(self.codec);
                AMediaCodec_delete(self.codec);
            }
            self.codec = ptr::null_mut();
        }
        self.reader = None;
        self.pending.clear();
    }
    fn create(&mut self) -> Result<()> {
        self.reset();
        let dimensions = annexb::parse_sps(&self.sets.sps)?;
        if dimensions.width == 0
            || dimensions.height == 0
            || dimensions.width > 4096
            || dimensions.height > 2160
        {
            return Err(Error::Unsupported("invalid Android stream dimensions"));
        }
        let name = DECODER_NAME
            .get()
            .ok_or_else(|| Error::Unavailable("Android hardware decoder not initialized".into()))?;
        let name = std::ffi::CString::new(name.as_str()).map_err(|_| Error::BadData)?;
        unsafe {
            let mut reader = ptr::null_mut();
            check(
                AImageReader_newWithUsage(
                    dimensions.width as i32,
                    dimensions.height as i32,
                    34,
                    1 << 8,
                    8,
                    &mut reader,
                ),
                "AImageReader_newWithUsage",
            )?;
            if reader.is_null() {
                return Err(Error::Unavailable("null ImageReader".into()));
            }
            self.reader = Some(Arc::new(Reader(reader)));
            let mut window = ptr::null_mut();
            check(
                AImageReader_getWindow(reader, &mut window),
                "AImageReader_getWindow",
            )?;
            self.codec = AMediaCodec_createCodecByName(name.as_ptr());
            if self.codec.is_null() {
                return Err(Error::Unavailable(
                    "hardware H264 decoder unavailable".into(),
                ));
            }
            let format = AMediaFormat_new();
            if format.is_null() {
                return Err(Error::Unavailable("media format allocation failed".into()));
            }
            AMediaFormat_setString(format, c"mime".as_ptr(), c"video/avc".as_ptr());
            AMediaFormat_setInt32(format, c"width".as_ptr(), dimensions.width as i32);
            AMediaFormat_setInt32(format, c"height".as_ptr(), dimensions.height as i32);
            AMediaFormat_setInt32(format, c"max-input-size".as_ptr(), 4 * 1024 * 1024);
            AMediaFormat_setInt32(format, c"low-latency".as_ptr(), 1);
            for (key, nal) in [(c"csd-0", &self.sets.sps), (c"csd-1", &self.sets.pps)] {
                let mut csd = vec![0, 0, 0, 1];
                csd.extend(nal);
                AMediaFormat_setBuffer(format, key.as_ptr(), csd.as_ptr().cast(), csd.len());
            }
            let result = AMediaCodec_configure(self.codec, format, window, ptr::null_mut(), 0);
            AMediaFormat_delete(format);
            check(result, "AMediaCodec_configure")?;
            check(AMediaCodec_start(self.codec), "AMediaCodec_start")?;
        }
        STATUS.store(1, Ordering::Relaxed);
        self.width = dimensions.width;
        self.height = dimensions.height;
        Ok(())
    }
    fn drain(&mut self) -> Result<()> {
        unsafe {
            for _ in 0..8 {
                let mut info = BufferInfo {
                    offset: 0,
                    size: 0,
                    time: 0,
                    flags: 0,
                };
                let index = AMediaCodec_dequeueOutputBuffer(self.codec, &mut info, 1000);
                if index < 0 {
                    if index == -2 {
                        continue;
                    }
                    break;
                }
                check(
                    AMediaCodec_releaseOutputBuffer(
                        self.codec,
                        index as usize,
                        crate::surface_output::render_surface_output(info.size, info.flags),
                    ),
                    "AMediaCodec_releaseOutputBuffer",
                )?;
            }
            let reader = self.reader.as_ref().ok_or(Error::InvalidSession)?;
            let mut image = ptr::null_mut();
            if AImageReader_acquireLatestImage(reader.0, &mut image) == 0 && !image.is_null() {
                let mut ns = 0;
                AImage_getTimestamp(image, &mut ns);
                let pts = ns / 1000;
                let position = self.pending.iter().position(|(time, _, _)| *time == pts);
                let times = position.and_then(|p| {
                    let value = self.pending.get(p).copied();
                    self.pending.drain(..=p);
                    value
                });
                let now = Instant::now();
                let (_, received, submitted) = times.unwrap_or((pts, now, now));
                (self.sink)(Frame {
                    buf: NativeBuffer::AndroidImage(Image {
                        raw: image,
                        _reader: reader.clone(),
                        backend: DECODER_NAME.get().cloned().unwrap_or_default(),
                    }),
                    width: self.width,
                    height: self.height,
                    color: ColorInfo::default(),
                    t_received: received,
                    t_submitted: submitted,
                    t_decoded: now,
                });
            }
        }
        Ok(())
    }
}
impl VideoDecoder for AndroidDecoder {
    fn decode(&mut self, au: &[u8], received: Instant) -> Result<()> {
        let changed = self.sets.update_from(au);
        if !self.sets.is_complete() {
            return Err(Error::WaitingForParameterSets);
        }
        if (self.codec.is_null() || changed)
            && let Err(error) = self.create()
        {
            self.reset();
            return Err(error);
        }
        self.drain()?;
        unsafe {
            let index = AMediaCodec_dequeueInputBuffer(self.codec, 3000);
            if index < 0 {
                return Err(Error::BadData);
            }
            let mut capacity = 0;
            let input = AMediaCodec_getInputBuffer(self.codec, index as usize, &mut capacity);
            if input.is_null() || au.len() > capacity {
                AMediaCodec_queueInputBuffer(self.codec, index as usize, 0, 0, 0, 0);
                return Err(Error::BadData);
            }
            ptr::copy_nonoverlapping(au.as_ptr(), input, au.len());
            let pts = self.epoch.elapsed().as_micros().min(i64::MAX as u128) as i64;
            check(
                AMediaCodec_queueInputBuffer(
                    self.codec,
                    index as usize,
                    0,
                    au.len(),
                    pts as u64,
                    0,
                ),
                "AMediaCodec_queueInputBuffer",
            )?;
            if self.pending.len() == 32 {
                self.pending.pop_front();
            }
            self.pending.push_back((pts, received, Instant::now()));
        }
        self.drain()
    }
    fn flush(&mut self) {
        if !self.codec.is_null() {
            let _ = self.drain();
        }
        self.reset();
        self.sets = Default::default();
    }
}
impl Drop for AndroidDecoder {
    fn drop(&mut self) {
        self.reset();
    }
}
fn check(status: i32, what: &'static str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        STATUS.store(status, Ordering::Relaxed);
        Err(Error::Os { what, status })
    }
}
