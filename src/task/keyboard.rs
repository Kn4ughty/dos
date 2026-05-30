// OnceCell is needed isntead of lazy_static becase we need to ensure the interrupt handler does not
//
// perform a heap allocation.
use crate::print;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts};

const QUEUE_SIZE: usize = 100;
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

static WAKER: AtomicWaker = AtomicWaker::new();

use crate::println;

/// Called by the keyboard inetrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: Scancode queue uninitialised");
    }
}

pub struct ScancodeStream {
    // Only constructable in this module
    _private: (),
}

#[expect(clippy::new_without_default)]
impl ScancodeStream {
    /// # Panics
    /// Panics if scancode stream is already created. i.e this func must be called exactly once.
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(QUEUE_SIZE))
            .expect("ScancodeStream::new should only be called once");

        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // this should never fail
        let queue = SCANCODE_QUEUE.try_get().expect("not initialised");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());

        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    // https://wiki.osdev.org/PS/2_Keyboard#Scan_Code_Set_1
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
            && let Some(key) = keyboard.process_keyevent(key_event)
        {
            match key {
                DecodedKey::RawKey(key) => print!("{key:?}"),
                DecodedKey::Unicode(character) => print!("{character}"),
            }
        }
    }
}
