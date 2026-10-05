use alloc::{format, string::String, vec, vec::Vec};
use psp::sys::{self, IoOpenFlags as Flags, IoWhence};
fn path(value: &str) -> Vec<u8> {
    let mut v = value.as_bytes().to_vec();
    v.push(0);
    v
}
pub struct File(pub sys::SceUid);
impl File {
    pub fn open(name: &str) -> Result<Self, i32> {
        let p = path(name);
        let fd = unsafe { sys::sceIoOpen(p.as_ptr(), Flags::RD_ONLY, 0) };
        if fd.0 < 0 {
            Err(fd.0)
        } else {
            Ok(Self(fd))
        }
    }
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, i32> {
        let n = unsafe { sys::sceIoRead(self.0, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
        if n < 0 {
            Err(n)
        } else {
            Ok(n as usize)
        }
    }
    pub fn rewind(&mut self) {
        unsafe {
            sys::sceIoLseek(self.0, 0, IoWhence::Set);
        }
    }
}
impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            sys::sceIoClose(self.0);
        }
    }
}
pub fn read(name: impl AsRef<str>) -> Result<Vec<u8>, i32> {
    let mut f = File::open(name.as_ref())?;
    let len = unsafe { sys::sceIoLseek(f.0, 0, IoWhence::End) };
    if len < 0 || len > 8 * 1024 * 1024 {
        return Err(-1);
    }
    f.rewind();
    let mut data = vec![0; len as usize];
    let mut done = 0;
    while done < data.len() {
        let n = f.read(&mut data[done..])?;
        if n == 0 {
            return Err(-1);
        }
        done += n;
    }
    Ok(data)
}
pub fn exists(name: &str) -> bool {
    File::open(name).is_ok()
}
pub fn mkdir(name: &str) {
    let p = path(name);
    unsafe {
        sys::sceIoMkdir(p.as_ptr(), 0o777);
    }
}
pub fn write(name: &str, data: &[u8]) -> Result<(), i32> {
    let p = path(name);
    let fd = unsafe {
        sys::sceIoOpen(
            p.as_ptr(),
            Flags::WR_ONLY | Flags::CREAT | Flags::TRUNC,
            0o666,
        )
    };
    if fd.0 < 0 {
        return Err(fd.0);
    }
    let f = File(fd);
    let mut done = 0;
    while done < data.len() {
        let n = unsafe { sys::sceIoWrite(f.0, data[done..].as_ptr().cast(), data.len() - done) };
        if n <= 0 {
            return Err(n);
        }
        done += n as usize;
    }
    Ok(())
}
pub fn rename(old: &str, new: &str) -> bool {
    let a = path(old);
    let b = path(new);
    unsafe { sys::sceIoRename(a.as_ptr(), b.as_ptr()) >= 0 }
}
pub fn remove(name: &str) {
    let p = path(name);
    unsafe {
        sys::sceIoRemove(p.as_ptr());
    }
}
pub fn replace(temp: &str, target: &str) -> bool {
    let backup = format!("{target}.BAK");
    let had_target = exists(target);
    if had_target {
        if exists(&backup) {
            remove(&backup);
        }
        if !rename(target, &backup) {
            return false;
        }
    }
    if rename(temp, target) {
        if exists(&backup) {
            remove(&backup);
        }
        true
    } else {
        if had_target {
            rename(&backup, target);
        }
        false
    }
}
pub fn save_dir() -> String {
    mkdir("ms0:/PSP");
    mkdir("ms0:/PSP/SAVEDATA");
    let p = format!("ms0:/PSP/SAVEDATA/STARRYFLOWERS");
    mkdir(&p);
    p
}
