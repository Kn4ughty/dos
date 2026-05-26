use core::arch::asm;
use core::marker::PhantomData;

pub trait PortReadAccess {}
pub trait PortWriteAccess {}

pub struct ReadOnlyAccess(());
impl PortReadAccess for ReadOnlyAccess {}

pub struct WriteOnlyAccess(());
impl PortWriteAccess for WriteOnlyAccess {}

pub struct ReadWriteAccess(());
impl PortReadAccess for ReadWriteAccess {}
impl PortWriteAccess for ReadWriteAccess {}

// T is output type, A is access
pub struct PortGeneric<T, A> {
    port: u16,
    phantom: PhantomData<(T, A)>,
}

pub type Port<T> = PortGeneric<T, ReadWriteAccess>;
pub type PortReadOnly<T> = PortGeneric<T, ReadOnlyAccess>;
pub type PortWriteOnly<T> = PortGeneric<T, WriteOnlyAccess>;

impl<T, A> PortGeneric<T, A> {
    pub const fn new(port: u16) -> PortGeneric<T, A> {
        PortGeneric {
            port,
            phantom: PhantomData,
        }
    }
}

impl<T: PortRead, A: PortReadAccess> PortGeneric<T, A> {
    /// # Safety
    /// The port could have side effects that violate memory safety
    #[inline]
    pub unsafe fn read(&mut self) -> T {
        unsafe { T::read_from_port(self.port) }
    }
}

impl<T: PortWrite, A: PortWriteAccess> PortGeneric<T, A> {
    /// ## Safety
    /// The port could have side effects that violate memory safety
    pub unsafe fn write(&mut self, value: T) {
        unsafe { T::write_to_port(self.port, value) }
    }
}
pub trait PortRead {
    /// # Safety
    /// Implementation must actually read from port
    unsafe fn read_from_port(port: u16) -> Self;
}

pub trait PortWrite {
    /// # Safety
    /// Implementation must actually write to port correctly
    unsafe fn write_to_port(port: u16, val: Self);
}

impl PortWrite for u8 {
    #[inline]
    unsafe fn write_to_port(port: u16, val: u8) {
        unsafe {
            asm!("out dx, al", in("al") val, in("dx") port,
            options(nomem, preserves_flags, nostack));
        }
    }
}

impl PortRead for u8 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> u8 {
        let mut ret: u8 = 0;
        unsafe {
            asm!("in al, dx", out("al") ret, in("dx") port,
            options(nomem, preserves_flags, nostack));
        }
        ret
    }
}

impl PortWrite for u32 {
    #[inline]
    unsafe fn write_to_port(port: u16, val: u32) {
        unsafe {
            asm!("out dx, eax", in("eax") val, in("dx") port,
            options(nomem, preserves_flags, nostack));
        }
    }
}

impl PortRead for u32 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> u32 {
        let mut ret: u32 = 0;
        unsafe {
            asm!("in eax, dx", out("eax") ret, in("dx") port,
            options(nomem, preserves_flags, nostack));
        }
        ret
    }
}
