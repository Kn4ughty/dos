use crate::{vga_print, vga_println};
use alloc::{string::String, vec::Vec};
// OnceCell is needed isntead of lazy_static becase we need to ensure the interrupt handler does not
// perform a heap allocation.
use conquer_once::spin::OnceCell;
use core::{
    char,
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts};

const QUEUE_SIZE: usize = 100;
/// Contains keyboard scancodes that have not been intepretered as keys yet.
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

pub struct Shell {
    text_buffer: Vec<char>,
    scancode_stream: ScancodeStream,
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
}

impl Shell {
    #[must_use]
    pub fn new() -> Self {
        vga_print!("\n> ");
        Shell {
            text_buffer: Vec::new(),
            scancode_stream: ScancodeStream::new(),
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            ),
        }
    }

    pub async fn run(mut self) {
        // https://wiki.osdev.org/PS/2_Keyboard#Scan_Code_Set_1
        while let Some(scancode) = self.scancode_stream.next().await {
            if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode)
                && let Some(key) = self.keyboard.process_keyevent(key_event)
            {
                self.handle_key(key);
            }
        }
    }

    fn handle_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(_) => {
                // pass
            }
            DecodedKey::Unicode(character) => {
                if character == '\n' {
                    vga_println!();
                    Self::handle_command(self.text_buffer.as_slice());
                    self.text_buffer.clear();
                    vga_print!("\n> ");
                } else {
                    self.text_buffer.push(character);
                    vga_print!("{character}");
                }
            }
        }
    }

    fn handle_command(command: &[char]) {
        let command: String = command.iter().collect();
        match command.as_str() {
            "lspci" => {
                crate::pci::lspci();
            }
            "date" => {
                println!("{}", crate::time::rtc::CMOS.lock().get_datetime());
            }
            _ => {
                vga_println!("Unknown command!");
            }
        }
    }
}
