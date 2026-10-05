use crate::sys::{self, SceSysMemBlockTypes, SceSysMemPartitionId};
use alloc::alloc::Layout;
use core::ptr;

#[global_allocator]
static ALLOC: linked_list_allocator::LockedHeap = linked_list_allocator::LockedHeap::empty();

/// Initialize a single arena before any allocating game code runs.
/// This avoids using one scarce kernel object for each Rust allocation.
pub unsafe fn init_heap(size: usize) {
    let id = sys::sceKernelAllocPartitionMemory(
        SceSysMemPartitionId::SceKernelPrimaryUserPartition,
        b"StarryFlowersHeap\0".as_ptr(), SceSysMemBlockTypes::Low,
        size as u32, ptr::null_mut());
    assert!(id.0 > 0, "Unable to allocate PSP heap");
    let base = sys::sceKernelGetBlockHeadAddr(id).cast::<u8>();
    ALLOC.lock().init(base, size);
}

#[cfg(not(feature = "std"))]
#[alloc_error_handler]
fn aeh(_: Layout) -> ! {
    loop {
        core::hint::spin_loop()
    }
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memset(ptr: *mut u8, value: u32, num: usize) -> *mut u8 {
    let mut i = 0;

    while i < num {
        *((ptr as usize + i) as *mut u8) = value as u8;
        i += 1;
    }

    ptr
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, num: isize) -> *mut u8 {
    let mut i = 0;

    while i < num {
        *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
        i += 1;
    }

    dst
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memcmp(ptr1: *mut u8, ptr2: *mut u8, num: usize) -> i32 {
    let mut i = 0;

    while i < num {
        let val1 = *((ptr1 as usize + i) as *mut u8);
        let val2 = *((ptr2 as usize + i) as *mut u8);
        let diff = val1 as i32 - val2 as i32;

        if diff != 0 {
            return diff;
        }

        i += 1;
    }

    0
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn memmove(dst: *mut u8, src: *mut u8, num: isize) -> *mut u8 {
    if dst < src {
        let mut i = 0;

        while i < num {
            *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
            i += 1;
        }
    } else {
        let mut i = num - 1;

        while i >= 0 {
            *((dst as isize + i) as *mut u8) = *((src as isize + i) as *mut u8);
            i -= 1;
        }
    }

    dst
}

#[no_mangle]
#[cfg(not(feature = "stub-only"))]
unsafe extern "C" fn strlen(s: *mut u8) -> usize {
    let mut len = 0;

    while *s.add(len) != 0 {
        len += 1;
    }

    len
}
