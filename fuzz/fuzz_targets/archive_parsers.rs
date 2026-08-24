#![no_main]

use std::io::Write;

use libfuzzer_sys::fuzz_target;
use tempfile::Builder;
use zifile_core::{SafetyLimits, list_archive_with_limits, test_archive_with_limits};

const FUZZ_LIMITS: SafetyLimits = SafetyLimits {
    max_entries: 256,
    max_expanded_bytes: 4 * 1024 * 1024,
    max_expansion_ratio: 64,
    max_path_depth: 32,
};

struct FormatCase {
    suffix: &'static str,
    magic: &'static [u8],
    tar_header: bool,
}

const CASES: [FormatCase; 13] = [
    FormatCase {
        suffix: ".zip",
        magic: b"PK\x03\x04",
        tar_header: false,
    },
    FormatCase {
        suffix: ".7z",
        magic: b"7z\xBC\xAF\x27\x1C",
        tar_header: false,
    },
    FormatCase {
        suffix: ".tar",
        magic: b"",
        tar_header: true,
    },
    FormatCase {
        suffix: ".tar.gz",
        magic: b"\x1F\x8B",
        tar_header: false,
    },
    FormatCase {
        suffix: ".tar.zst",
        magic: b"\x28\xB5\x2F\xFD",
        tar_header: false,
    },
    FormatCase {
        suffix: ".tar.xz",
        magic: b"\xFD7zXZ\x00",
        tar_header: false,
    },
    FormatCase {
        suffix: ".tar.bz2",
        magic: b"BZh",
        tar_header: false,
    },
    FormatCase {
        suffix: ".gz",
        magic: b"\x1F\x8B",
        tar_header: false,
    },
    FormatCase {
        suffix: ".zst",
        magic: b"\x28\xB5\x2F\xFD",
        tar_header: false,
    },
    FormatCase {
        suffix: ".xz",
        magic: b"\xFD7zXZ\x00",
        tar_header: false,
    },
    FormatCase {
        suffix: ".bz2",
        magic: b"BZh",
        tar_header: false,
    },
    FormatCase {
        suffix: ".lz4",
        magic: b"\x04\x22\x4D\x18",
        tar_header: false,
    },
    FormatCase {
        suffix: ".br",
        magic: b"",
        tar_header: false,
    },
];

fn parser_input(case: &FormatCase, payload: &[u8]) -> Vec<u8> {
    if case.tar_header {
        let mut bytes = payload.to_vec();
        bytes.resize(bytes.len().max(512), 0);
        bytes[257..262].copy_from_slice(b"ustar");
        return bytes;
    }
    if case.magic.is_empty() || payload.starts_with(case.magic) {
        return payload.to_vec();
    }
    let mut bytes = Vec::with_capacity(case.magic.len() + payload.len());
    bytes.extend_from_slice(case.magic);
    bytes.extend_from_slice(payload);
    bytes
}

fuzz_target!(|input: &[u8]| {
    let Some((&selector, payload)) = input.split_first() else {
        return;
    };
    let case = &CASES[usize::from(selector) % CASES.len()];
    let bytes = parser_input(case, payload);
    let Ok(mut archive) = Builder::new().suffix(case.suffix).tempfile() else {
        return;
    };
    if archive.write_all(&bytes).is_err() || archive.flush().is_err() {
        return;
    }
    let password = (selector & 0x80 != 0).then_some("fuzz-password");
    if list_archive_with_limits(archive.path(), password, FUZZ_LIMITS).is_ok() {
        let _ = test_archive_with_limits(archive.path(), password, FUZZ_LIMITS);
    }
});
