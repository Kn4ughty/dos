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

/// Cannot use the normal test runner here because it does not support async functions,
/// and we need to grantee that the network loop is running
async fn run_tests() {
    let tests = [send_udp_packet_to_self];
    serial_println!("running socket {:?} tests", tests.len());
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
    // port 2 and 3 are reserved, so they shouldn't be used by anything else
    let mut incoming_socket = SocketHandle::new(2, Ipv4Addr::LOCALHOST, SocketProtocolType::Udp)
        .expect("Can obtain port 2");

    let mut outgoing_socket = SocketHandle::new(3, Ipv4Addr::LOCALHOST, SocketProtocolType::Udp)
        .expect("Can obtain port 3");

    let test_data = b"This is some test bytes that should survive transmission";

    outgoing_socket
        .send_data(Ipv4Addr::LOCALHOST, 2, test_data)
        .await
        .expect("Can send UDP data");
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
