#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::net::Ipv4Addr;

use bootloader::{BootInfo, entry_point};
use futures_util::StreamExt;

use os::net::socket::*;
use os::serial_println;
use os::task::Task;
use os::task::executor::Executor;
use os::{exit_qemu, net};

use core::panic::PanicInfo;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    os::init();
    os::memory_init(boot_info);
    os::post_memory_init();

    let mut executor = Executor::new();
    executor.spawn(Task::new(net::loop_networking(), "network"));
    executor.spawn(Task::new(run_tests(), "test_main"));

    executor.run();
}

async fn run_tests() {
    let tests = [send_udp_packet_to_self];
    serial_println!("runnning socket {:?} tests", tests.len());
    for test in tests {
        serial_println!(
            "running socket test: {}",
            core::any::type_name_of_val(&test)
        );
        test().await;
        serial_println!("[ok]",);
    }
    exit_qemu(os::QemuExitCode::Success)
}

async fn send_udp_packet_to_self() {
    // port 2 and 3 are reserved, so they wont be used by anything else
    let mut incoming_socket =
        SocketHandle::new(2.into(), Ipv4Addr::LOCALHOST).expect("Can obtain port 2");

    let mut outgoing_socket =
        SocketHandle::new(3.into(), Ipv4Addr::LOCALHOST).expect("Can obtain port 3");

    let test_data = b"This is some test bytes that should survive transmission";

    let interface = crate::net::INTERFACE
        .read()
        .await
        .expect("network card exists");

    outgoing_socket
        .send_data(Ipv4Addr::LOCALHOST, 2.into(), test_data, interface)
        .await
        .expect("Can send udp data");
    log::debug!("Sent packet on port 2");

    let response = incoming_socket
        .next()
        .await
        .expect("socket not force closed ");

    assert_eq!(response.data.as_slice(), test_data);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    os::test_panic_handler(info)
}
