#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = zifile_core::safe_relative_path(input, 128);
});
