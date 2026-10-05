//! Native blocking audio workers; no allocation or spin locks on audio threads.
use alloc::{collections::BTreeMap, format, string::String, sync::Arc, vec::Vec};
use core::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicI32, AtomicU32, Ordering},
};
use psp::sys;

struct Shared {
    paths: Vec<Vec<u8>>,
    names: Vec<String>,
    looping: bool,
    selected: AtomicU32,
    generation: AtomicU32,
    volume: AtomicI32,
    status: AtomicI32,
    blocks: AtomicU32,
}
pub struct Audio {
    shared: Arc<Shared>,
}
impl Audio {
    pub fn new(root: &str, files: &BTreeMap<String, String>, looping: bool) -> Self {
        let names: Vec<_> = files.values().cloned().collect();
        let paths = names
            .iter()
            .map(|name| {
                let mut path = format!("{root}/DATA/{name}").into_bytes();
                path.push(0);
                path
            })
            .collect();
        let shared = Arc::new(Shared {
            paths,
            names,
            looping,
            selected: AtomicU32::new(u32::MAX),
            generation: AtomicU32::new(0),
            volume: AtomicI32::new(32768),
            status: AtomicI32::new(-1),
            blocks: AtomicU32::new(0),
        });
        unsafe {
            let thread = sys::sceKernelCreateThread(
                b"StarryAudio\0".as_ptr(),
                worker,
                0x18,
                16 * 1024,
                sys::ThreadAttributes::USER,
                ptr::null_mut(),
            );
            if thread.0 > 0 {
                let mut arg = Arc::into_raw(shared.clone());
                let result = sys::sceKernelStartThread(
                    thread,
                    core::mem::size_of_val(&arg),
                    (&mut arg as *mut *const Shared).cast(),
                );
                if result < 0 {
                    drop(Arc::from_raw(arg));
                    shared.status.store(result, Ordering::Relaxed);
                }
            } else {
                shared.status.store(thread.0, Ordering::Relaxed);
            }
        }
        Self { shared }
    }
    pub fn update(&self, filename: &str, volume: f32, restart: bool) {
        self.shared
            .volume
            .store((volume.clamp(0., 1.) * 32768.) as i32, Ordering::Relaxed);
        // An empty sound event changes volume without cutting the previous sound short.
        if filename.is_empty() && !self.shared.looping {
            return;
        }
        let selected = self
            .shared
            .names
            .iter()
            .position(|s| s == filename)
            .map(|i| i as u32)
            .unwrap_or(u32::MAX);
        if restart || selected != self.shared.selected.load(Ordering::Relaxed) {
            self.shared.selected.store(selected, Ordering::Relaxed);
            self.shared.generation.fetch_add(1, Ordering::Release);
        }
    }
    pub fn blocks(&self) -> u32 {
        self.shared.blocks.load(Ordering::Relaxed)
    }
}
unsafe extern "C" fn worker(_: usize, arg: *mut c_void) -> i32 {
    let shared = Arc::from_raw(*(arg as *const *const Shared));
    let channel = sys::sceAudioChReserve(-1, 1024, sys::AudioFormat::Mono);
    shared.status.store(channel, Ordering::Relaxed);
    if channel < 0 {
        return channel;
    }
    let mut fd = sys::SceUid(-1);
    let mut generation = u32::MAX;
    let mut samples = [0i16; 1024];
    loop {
        let next = shared.generation.load(Ordering::Acquire);
        if next != generation {
            generation = next;
            if fd.0 >= 0 {
                sys::sceIoClose(fd);
            }
            fd = shared
                .paths
                .get(shared.selected.load(Ordering::Relaxed) as usize)
                .map(|p| sys::sceIoOpen(p.as_ptr(), sys::IoOpenFlags::RD_ONLY, 0))
                .unwrap_or(sys::SceUid(-1));
        }
        if fd.0 < 0 {
            sys::sceKernelDelayThread(10_000);
            continue;
        }
        samples.fill(0);
        let n = sys::sceIoRead(fd, samples.as_mut_ptr().cast(), 2048);
        if n <= 0 {
            if shared.looping && n == 0 {
                sys::sceIoLseek(fd, 0, sys::IoWhence::Set);
            } else {
                sys::sceIoClose(fd);
                fd = sys::SceUid(-1);
            }
            continue;
        }
        let result = sys::sceAudioOutputBlocking(
            channel,
            shared.volume.load(Ordering::Relaxed),
            samples.as_mut_ptr().cast(),
        );
        if result >= 0 {
            shared.blocks.fetch_add(1, Ordering::Relaxed);
        } else {
            sys::sceKernelDelayThread(10_000);
        }
    }
}
