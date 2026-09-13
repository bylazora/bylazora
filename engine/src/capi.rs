// SPDX-License-Identifier: AGPL-3.0-or-later
// C ABI for bylazora-core: stable entry points for C, C++, and FFI callers.
//
// Every function is panic-free across the FFI boundary (panics are caught and
// reported as -1 with a message), errbuf writes are NUL-terminated and
// truncated safely, and no function performs network I/O. Paths are UTF-8 and
// are passed as ordinary NUL-terminated char pointers.
use std::ffi::{c_char, CStr};
use std::path::PathBuf;

fn write_err(errbuf: *mut c_char, errbuf_len: usize, msg: &str) {
    if errbuf.is_null() || errbuf_len == 0 {
        return;
    }
    let bytes = msg.as_bytes();
    let n = bytes.len().min(errbuf_len - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), errbuf.cast::<u8>(), n);
        *errbuf.add(n) = 0;
    }
}

unsafe fn cstr_string(p: *const c_char) -> Result<String, String> {
    if p.is_null() {
        return Err("null string pointer".to_string());
    }
    let c = unsafe { CStr::from_ptr(p) };
    c.to_str()
        .map(str::to_string)
        .map_err(|_| "string is not valid UTF-8".to_string())
}

unsafe fn cstr_path(p: *const c_char) -> Result<PathBuf, String> {
    Ok(PathBuf::from(cstr_string(p)?))
}

#[no_mangle]
pub extern "C" fn bylazora_version() -> *const c_char {
    b"0.4.0\0".as_ptr().cast()
}

fn validate_impl(ref_dir: *const c_char, other_dir: *const c_char) -> Result<i32, String> {
    let ref_dir = unsafe { cstr_path(ref_dir)? };
    let other_dir = unsafe { cstr_path(other_dir)? };
    let errs = crate::validator::compare_outputs(&ref_dir, &other_dir)
        .map_err(|e| format!("{e:#}"))?;
    if errs.is_empty() { Ok(0) } else { Ok(1) }
}

#[no_mangle]
pub unsafe extern "C" fn bylazora_validate(
    ref_dir: *const c_char,
    other_dir: *const c_char,
    errbuf: *mut c_char,
    errbuf_len: usize,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        validate_impl(ref_dir, other_dir)
    })) {
        Ok(Ok(code)) => code,
        Ok(Err(msg)) => { write_err(errbuf, errbuf_len, &msg); -1 }
        Err(_) => { write_err(errbuf, errbuf_len, "internal panic"); -1 }
    }
}

fn bench_impl(
    backend: *const c_char,
    input_dir: *const c_char,
    output_dir: *const c_char,
) -> Result<(), String> {
    let backend = if backend.is_null() {
        "cpu".to_string()
    } else {
        unsafe { cstr_string(backend)? }
    };
    let backend = if backend.is_empty() { "cpu".to_string() } else { backend };
    let input_dir = unsafe { cstr_path(input_dir)? };
    let output_dir = unsafe { cstr_path(output_dir)? };
    match backend.as_str() {
        "cpu" => crate::cpu::run(&input_dir, &output_dir),
        "gpu" => crate::gpu::run(&input_dir, &output_dir, 0),
        "cuda" => crate::cuda::run(&input_dir, &output_dir, 0),
        other => return Err(format!("unknown backend: {other} (expected cpu, gpu, or cuda)")),
    }
    .map_err(|e| format!("{e:#}"))
}

#[no_mangle]
pub unsafe extern "C" fn bylazora_bench(
    backend: *const c_char,
    input_dir: *const c_char,
    output_dir: *const c_char,
    errbuf: *mut c_char,
    errbuf_len: usize,
) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bench_impl(backend, input_dir, output_dir)
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(msg)) => { write_err(errbuf, errbuf_len, &msg); -1 }
        Err(_) => { write_err(errbuf, errbuf_len, "internal panic"); -1 }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bylazora_licence_check(errbuf: *mut c_char, errbuf_len: usize) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::key::cmd_check())) {
        Ok(Ok(())) => 0,
        Ok(Err(msg)) => { write_err(errbuf, errbuf_len, &msg); -1 }
        Err(_) => { write_err(errbuf, errbuf_len, "internal panic"); -1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bylazora-capi-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn err_msg(buf: &[u8]) -> String {
        unsafe { CStr::from_ptr(buf.as_ptr().cast()) }.to_string_lossy().into_owned()
    }

    fn write_dataset(dir: &std::path::Path) {
        fs::create_dir_all(dir).unwrap();
        let mut balances = String::from("ACCOUNT_ID,BALANCE_CENTS\n");
        for i in 0..10 {
            balances.push_str(&format!("{},{}\n", i, (i + 1) * 1000));
        }
        fs::write(dir.join("balances.csv"), balances).unwrap();

        let mut txns = String::from("ACCOUNT_ID,AMOUNT_CENTS,TXN_TYPE\n");
        for i in 0..50 {
            let account = i % 10;
            let amount = (i as i64 + 1) * 7;
            let kind = if i % 2 == 0 { "D" } else { "W" };
            txns.push_str(&format!("{account},{amount},{kind}\n"));
        }
        fs::write(dir.join("transactions.csv"), txns).unwrap();
    }

    #[test]
    fn version_is_0_4_0() {
        let s = unsafe { CStr::from_ptr(bylazora_version()) }.to_str().unwrap();
        assert_eq!(s, "0.4.0");
    }

    #[test]
    fn licence_check_obeys_contract() {
        let mut errbuf = [0u8; 512];
        let rc = unsafe { bylazora_licence_check(errbuf.as_mut_ptr().cast(), errbuf.len()) };
        assert!(rc == 0 || rc == -1, "licence check returned {rc}");
        if rc == -1 {
            let msg = err_msg(&errbuf);
            assert!(!msg.is_empty(), "licence error message must be non-empty");
        }
    }

    #[test]
    fn bench_and_validate_roundtrip() {
        let root = temp_dir("roundtrip");
        write_dataset(&root);
        let out = root.join("out");

        let backend_c = CString::new("cpu").unwrap();
        let input_c = CString::new(root.to_str().unwrap()).unwrap();
        let out_c = CString::new(out.to_str().unwrap()).unwrap();

        let mut errbuf = [0u8; 512];
        let rc = unsafe {
            bylazora_bench(
                backend_c.as_ptr(),
                input_c.as_ptr(),
                out_c.as_ptr(),
                errbuf.as_mut_ptr().cast(),
                errbuf.len(),
            )
        };
        assert_eq!(rc, 0, "cpu bench failed: {}", err_msg(&errbuf));
        assert!(out.join("final_balances.csv").exists(), "final_balances.csv missing");
        assert!(out.join("summary_report.csv").exists(), "summary_report.csv missing");

        let nonsense_c = CString::new("nonsense").unwrap();
        let mut errbuf2 = [0u8; 512];
        let rc = unsafe {
            bylazora_bench(
                nonsense_c.as_ptr(),
                input_c.as_ptr(),
                out_c.as_ptr(),
                errbuf2.as_mut_ptr().cast(),
                errbuf2.len(),
            )
        };
        assert_eq!(rc, -1, "unknown backend must error");
        assert!(!err_msg(&errbuf2).is_empty(), "unknown backend error must fill errbuf");

        let same = root.join("same");
        fs::create_dir_all(&same).unwrap();
        fs::copy(out.join("final_balances.csv"), same.join("final_balances.csv")).unwrap();
        fs::copy(out.join("summary_report.csv"), same.join("summary_report.csv")).unwrap();
        let same_c = CString::new(same.to_str().unwrap()).unwrap();
        let mut errbuf3 = [0u8; 512];
        let rc = unsafe {
            bylazora_validate(out_c.as_ptr(), same_c.as_ptr(), errbuf3.as_mut_ptr().cast(), errbuf3.len())
        };
        assert_eq!(rc, 0, "identical outputs must validate as 0");

        let tampered = root.join("tampered");
        fs::create_dir_all(&tampered).unwrap();
        fs::copy(out.join("final_balances.csv"), tampered.join("final_balances.csv")).unwrap();
        fs::copy(out.join("summary_report.csv"), tampered.join("summary_report.csv")).unwrap();
        fs::write(tampered.join("final_balances.csv"), "ACCOUNT_ID,BALANCE_CENTS\n0,1\n").unwrap();
        let tampered_c = CString::new(tampered.to_str().unwrap()).unwrap();
        let mut errbuf4 = [0u8; 512];
        let rc = unsafe {
            bylazora_validate(out_c.as_ptr(), tampered_c.as_ptr(), errbuf4.as_mut_ptr().cast(), errbuf4.len())
        };
        assert_eq!(rc, 1, "tampered outputs must validate as 1");

        let _ = fs::remove_dir_all(&root);
    }
}
