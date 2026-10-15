use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TrySendError;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use pipewire as pw;
use pw::spa;
use spa::pod::Pod;

use crate::capture::traits::CaptureError;
use crate::storage::RawFrame;

/// Target object property key for PipeWire streams.
pub const PW_KEY_TARGET_OBJECT: &str = "target.object";

/// Thread-safe single-frame slot storing the freshest incoming frame.
///
/// Ensures zero-lag queue semantics: when a new frame arrives, it overwrites
/// and drops any unconsumed older frame.
#[derive(Clone, Default, Debug)]
pub struct FrameSlot(Arc<Mutex<Option<RawFrame>>>);

impl FrameSlot {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    pub fn put(&self, frame: RawFrame) {
        if let Ok(mut lock) = self.0.lock() {
            *lock = Some(frame);
        }
    }

    pub fn take(&self) -> Option<RawFrame> {
        self.0.lock().ok().and_then(|mut lock| lock.take())
    }

    pub fn flush(&self) {
        if let Ok(mut lock) = self.0.lock() {
            *lock = None;
        }
    }
}

/// Sender handle for pushing frames into the consumer's freshest-frame slot.
#[derive(Clone, Debug)]
pub struct FrameSender {
    slot: FrameSlot,
}

impl FrameSender {
    pub fn try_send(&self, frame: RawFrame) -> Result<(), TrySendError<RawFrame>> {
        self.slot.put(frame);
        Ok(())
    }

    pub fn send(&self, frame: RawFrame) -> Result<(), TrySendError<RawFrame>> {
        self.try_send(frame)
    }
}

/// Normalizes SPA video format pixel data into 32-bit BGRA in-place.
///
/// Handles `VideoFormat::RGBA` (swapping R & B channels), `VideoFormat::BGRx`
/// (setting alpha to 255), and `VideoFormat::RGBx` (swapping R & B channels and setting alpha to 255).
pub fn convert_format_to_bgra(bgra: &mut [u8], fmt: spa::param::video::VideoFormat) {
    if fmt == spa::param::video::VideoFormat::RGBA {
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
    } else if fmt == spa::param::video::VideoFormat::BGRx {
        for px in bgra.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    } else if fmt == spa::param::video::VideoFormat::RGBx {
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }
    }
}

/// Background PipeWire video stream consumer.
///
/// Runs a dedicated OS worker thread driving a `pipewire::MainLoop`, negotiating
/// SPA video format (`BGRx` / `BGRA` / `RGBA` / `RGBx`), dequeuing incoming stream buffers,
/// and storing the freshest 32-bit BGRA `RawFrame` into a zero-lag `FrameSlot`.
pub struct PipeWireStreamConsumer {
    frame_slot: FrameSlot,
    stop_signal: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    pw_sender: Option<pw::channel::Sender<()>>,
}

impl std::fmt::Debug for PipeWireStreamConsumer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeWireStreamConsumer")
            .field("stopped", &self.is_stopped())
            .finish()
    }
}

struct UserData {
    format: spa::param::video::VideoInfoRaw,
    frame_slot: FrameSlot,
    stop_signal: Arc<AtomicBool>,
    start_time: Instant,
}

impl PipeWireStreamConsumer {
    /// Connect to a PipeWire node using an existing screencast file descriptor.
    ///
    /// Spawns a background worker thread that initializes PipeWire, attaches to the
    /// node, negotiates video formats, and starts streaming buffers.
    pub fn connect(fd: OwnedFd, node_id: u32) -> Result<Self, CaptureError> {
        pw::init();

        let frame_slot = FrameSlot::new();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let (pw_sender, pw_receiver) = pw::channel::channel::<()>();
        let (startup_tx, startup_rx) = std::sync::mpsc::sync_channel::<Result<(), CaptureError>>(1);

        let thread_slot = frame_slot.clone();
        let thread_stop = Arc::clone(&stop_signal);

        let worker_handle = std::thread::Builder::new()
            .name("pipewire-stream-consumer".to_string())
            .spawn(move || {
                run_pipewire_loop(
                    fd,
                    node_id,
                    thread_slot,
                    thread_stop,
                    pw_receiver,
                    startup_tx,
                );
            })
            .map_err(|e| {
                CaptureError::BackendError(format!("Failed to spawn PipeWire thread: {e}"))
            })?;

        match startup_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                frame_slot,
                stop_signal,
                worker_handle: Some(worker_handle),
                pw_sender: Some(pw_sender),
            }),
            Ok(Err(err)) => {
                let _ = worker_handle.join();
                Err(err)
            }
            Err(_) => {
                let _ = worker_handle.join();
                Err(CaptureError::BackendError(
                    "PipeWire worker thread terminated unexpectedly during startup".to_string(),
                ))
            }
        }
    }

    /// Create an in-memory mock consumer for testing without hardware PipeWire.
    pub fn new_mock() -> (Self, FrameSender) {
        let frame_slot = FrameSlot::new();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let test_sender = FrameSender {
            slot: frame_slot.clone(),
        };
        let consumer = Self {
            frame_slot,
            stop_signal,
            worker_handle: None,
            pw_sender: None,
        };
        (consumer, test_sender)
    }

    /// Retrieve the freshest frame from the consumer slot, clearing it so subsequent calls return None
    /// until a new frame arrives.
    pub fn recv_frame(&self) -> Option<RawFrame> {
        self.frame_slot.take()
    }

    /// Flush/drain any queued frame in the slot.
    pub fn flush(&self) {
        self.frame_slot.flush();
    }

    /// Send a synthetic frame into the slot (replacing any older frame).
    pub fn send_synthetic_frame(&self, frame: RawFrame) -> Result<(), TrySendError<RawFrame>> {
        self.frame_slot.put(frame);
        Ok(())
    }

    /// Check whether the consumer has been signaled to stop.
    pub fn is_stopped(&self) -> bool {
        self.stop_signal.load(Ordering::SeqCst)
    }

    /// Signal the background stream worker to terminate and join the thread.
    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        if let Some(pw_sender) = self.pw_sender.take() {
            let _ = pw_sender.send(());
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for PipeWireStreamConsumer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_pipewire_loop(
    fd: OwnedFd,
    node_id: u32,
    frame_slot: FrameSlot,
    stop_signal: Arc<AtomicBool>,
    pw_receiver: pw::channel::Receiver<()>,
    startup_tx: std::sync::mpsc::SyncSender<Result<(), CaptureError>>,
) {
    let main_loop = match pw::main_loop::MainLoopRc::new(None) {
        Ok(l) => l,
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to create PipeWire MainLoop: {e}"
            ))));
            return;
        }
    };

    let context = match pw::context::ContextRc::new(&main_loop, None) {
        Ok(c) => c,
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to create PipeWire Context: {e}"
            ))));
            return;
        }
    };

    let core = match context.connect_fd_rc(fd, None) {
        Ok(c) => c,
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to connect PipeWire core fd: {e}"
            ))));
            return;
        }
    };

    let main_loop_clone = main_loop.clone();
    let _term_listener = pw_receiver.attach(main_loop.loop_(), move |_| {
        main_loop_clone.quit();
    });

    let props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Video",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Screen",
        PW_KEY_TARGET_OBJECT => node_id.to_string(),
    };

    let stream = match pw::stream::StreamBox::new(&core, "carve-screencast", props) {
        Ok(s) => s,
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to create PipeWire StreamBox: {e}"
            ))));
            return;
        }
    };

    let user_data = UserData {
        format: Default::default(),
        frame_slot,
        stop_signal,
        start_time: Instant::now(),
    };

    let _listener = match stream
        .add_local_listener_with_user_data(user_data)
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            let (media_type, media_subtype) =
                match pw::spa::param::format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(_) => return,
                };
            if media_type != pw::spa::param::format::MediaType::Video
                || media_subtype != pw::spa::param::format::MediaSubtype::Raw
            {
                return;
            }
            let _ = user_data.format.parse(param);
        })
        .process(|stream, user_data| {
            if user_data.stop_signal.load(Ordering::Relaxed) {
                return;
            }
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }
            let data = &mut datas[0];
            let width = user_data.format.size().width;
            let height = user_data.format.size().height;
            if width == 0 || height == 0 {
                return;
            }
            let chunk = data.chunk();
            let offset = chunk.offset() as usize;
            let size = chunk.size() as usize;
            let stride = chunk.stride();

            let Some(slice) = data.data() else {
                return;
            };
            if offset + size > slice.len() {
                return;
            }
            let src_bytes = &slice[offset..offset + size];
            let expected_len = (width as usize) * (height as usize) * 4;

            let mut bgra = if stride <= 0 || stride as usize == (width as usize) * 4 {
                if src_bytes.len() < expected_len {
                    return;
                }
                src_bytes[..expected_len].to_vec()
            } else {
                let row_bytes = (width as usize) * 4;
                let stride_bytes = stride as usize;
                let mut buf = vec![0u8; expected_len];
                for row in 0..(height as usize) {
                    let s_start = row * stride_bytes;
                    let s_end = s_start + row_bytes;
                    let d_start = row * row_bytes;
                    let d_end = d_start + row_bytes;
                    if s_end <= src_bytes.len() {
                        buf[d_start..d_end].copy_from_slice(&src_bytes[s_start..s_end]);
                    }
                }
                buf
            };

            let fmt = user_data.format.format();
            convert_format_to_bgra(&mut bgra, fmt);

            let timestamp_ms = user_data.start_time.elapsed().as_millis() as u64;
            let raw_frame = RawFrame::new(timestamp_ms, width, height, bgra);
            user_data.frame_slot.put(raw_frame);
        })
        .register()
    {
        Ok(l) => l,
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to register PipeWire stream listener: {e}"
            ))));
            return;
        }
    };

    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::BGRA,
            pw::spa::param::video::VideoFormat::RGBA,
            pw::spa::param::video::VideoFormat::RGBx,
        ),
    );

    let values: Vec<u8> = match pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    ) {
        Ok((cursor, _)) => cursor.into_inner(),
        Err(e) => {
            let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
                "Failed to serialize SPA format POD: {e:?}"
            ))));
            return;
        }
    };

    let Some(param_pod) = Pod::from_bytes(&values) else {
        let _ = startup_tx.send(Err(CaptureError::BackendError(
            "Failed to deserialize Pod from bytes".to_string(),
        )));
        return;
    };
    let mut params = [param_pod];

    if let Err(e) = stream.connect(
        spa::utils::Direction::Input,
        Some(node_id),
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    ) {
        let _ = startup_tx.send(Err(CaptureError::BackendError(format!(
            "Failed to connect PipeWire stream: {e}"
        ))));
        return;
    }

    let _ = startup_tx.send(Ok(()));
    main_loop.run();
}
