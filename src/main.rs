#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

#[allow(unused_imports)]
#[allow(clippy::single_component_path_imports)]
use os;
