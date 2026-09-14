// SPDX-License-Identifier: AGPL-3.0-or-later
// CUDA tier: cudarc host plus one atomic groupby kernel. Replaces the
// RAPIDS container for the reference workload: same binary, same parallel
// chunked host read, same byte-exact gate. Producers parse file chunks in
// parallel; one consumer thread feeds the device in bounded 8M-row batches.
use anyhow::{anyhow, Result};
use cudarc::driver::{CudaContext, LaunchConfig, PushKernelArg};
use cudarc::nvrtc::Ptx;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::time::Instant;

const PTX: &str = include_str!("../kernels/groupby.ptx");
const ROWS_PER_CHUNK: usize = 8_000_000;

type Batch = (Vec<i32>, Vec<u32>, Vec<u32>);

fn parse_i64(b: &[u8]) -> i64 {
    let mut v: i64 = 0;
    for &c in b { v = v * 10 + (c - b'0') as i64; }
    v
}

pub fn run(input_dir: &Path, output_dir: &Path, _adapter: usize) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let mut bal_rdr = csv::ReaderBuilder::new().has_headers(true)
        .from_path(input_dir.join("balances.csv"))?;
    let mut starting: Vec<i64> = Vec::new();
    for rec in bal_rdr.records() {
        let rec = rec?;
        let id = parse_i64(rec.get(0).unwrap().as_bytes()) as usize;
        let b = parse_i64(rec.get(1).unwrap().as_bytes());
        if starting.len() <= id { starting.resize(id + 1, 0); }
        starting[id] = b;
    }
    let n_acct = starting.len();

    let ctx = CudaContext::new(0).map_err(|e| anyhow!("cuda device 0: {e:?}"))?;
    let stream = ctx.default_stream();
    let module = ctx.load_module(Ptx::from_src(PTX)).map_err(|e| anyhow!("ptx load: {e:?}"))?;
    let kern = module.load_function("groupby").map_err(|e| anyhow!("groupby load: {e:?}"))?;

    // deposits/withdrawals are u64: CUDA has native 64-bit atomicAdd, so
    // unlike the wgpu tier this needs no hi/lo split (see kernels/groupby.cu).
    let mut dep_dev = stream.alloc_zeros::<u64>(n_acct).map_err(|e| anyhow!("alloc: {e:?}"))?;
    let mut wd_dev = stream.alloc_zeros::<u64>(n_acct).map_err(|e| anyhow!("alloc: {e:?}"))?;
    let mut cnt_dev = stream.alloc_zeros::<u32>(n_acct).map_err(|e| anyhow!("alloc: {e:?}"))?;

    let t0 = Instant::now();
    let txn_path = input_dir.join("transactions.csv");
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).min(8);
    let starts = crate::cpu::chunk_starts(&txn_path, n_threads)?;
    let (tx, rx) = sync_channel::<Batch>(n_threads);
    let total = std::sync::Arc::new(AtomicUsize::new(0));
    let total_in = total.clone();

    let producer = std::thread::spawn(move || {
        starts.par_iter().enumerate().for_each(|(i, &start)| {
            let end = starts.get(i + 1).copied().unwrap_or(u64::MAX);
            let mut f = File::open(&txn_path).unwrap();
            f.seek(SeekFrom::Start(start)).unwrap();
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(f.take(end - start));
            let mut acct: Vec<i32> = Vec::with_capacity(ROWS_PER_CHUNK);
            let mut amt: Vec<u32> = Vec::with_capacity(ROWS_PER_CHUNK);
            let mut is_dep: Vec<u32> = Vec::with_capacity(ROWS_PER_CHUNK);
            let mut local = 0usize;
            for rec in rdr.records() {
                let rec = match rec { Ok(r) => r, Err(_) => continue };
                acct.push(parse_i64(rec.get(0).unwrap().as_bytes()) as i32);
                amt.push(parse_i64(rec.get(1).unwrap().as_bytes()) as u32);
                is_dep.push((rec.get(2).unwrap().as_bytes()[0] == b'D') as u32);
                local += 1;
                if acct.len() == ROWS_PER_CHUNK {
                    if tx.send((acct, amt, is_dep)).is_err() { return; }
                    acct = Vec::with_capacity(ROWS_PER_CHUNK);
                    amt = Vec::with_capacity(ROWS_PER_CHUNK);
                    is_dep = Vec::with_capacity(ROWS_PER_CHUNK);
                }
            }
            if !acct.is_empty() { let _ = tx.send((acct, amt, is_dep)); }
            total_in.fetch_add(local, Ordering::Relaxed);
        });
    });

    for (acct, amt, is_dep) in rx {
        let n = acct.len();
        let acct_dev = stream.memcpy_stod(&acct).map_err(|e| anyhow!("htod: {e:?}"))?;
        let amt_dev = stream.memcpy_stod(&amt).map_err(|e| anyhow!("htod: {e:?}"))?;
        let is_dep_dev = stream.memcpy_stod(&is_dep).map_err(|e| anyhow!("htod: {e:?}"))?;
        let cfg = LaunchConfig::for_num_elems(n as u32);
        let mut args = stream.launch_builder(&kern);
        args.arg(&acct_dev);
        args.arg(&amt_dev);
        args.arg(&is_dep_dev);
        args.arg(&mut dep_dev);
        args.arg(&mut wd_dev);
        args.arg(&mut cnt_dev);
        let n64 = n as i64;
        args.arg(&n64);
        unsafe { args.launch(cfg) }.map_err(|e| anyhow!("launch: {e:?}"))?;
    }
    producer.join().map_err(|_| anyhow!("producer panicked"))?;
    let n = total.load(Ordering::Relaxed);
    crate::key::announce_licence();
    eprintln!("cuda: {} threads, read+dispatch {} rows in {:.2}s", n_threads, n, t0.elapsed().as_secs_f64());

    let deposits = stream.memcpy_dtov(&dep_dev).map_err(|e| anyhow!("dtoh: {e:?}"))?;
    let withdrawals = stream.memcpy_dtov(&wd_dev).map_err(|e| anyhow!("dtoh: {e:?}"))?;
    let counts = stream.memcpy_dtov(&cnt_dev).map_err(|e| anyhow!("dtoh: {e:?}"))?;

    let t2 = Instant::now();
    let mut f = BufWriter::new(File::create(output_dir.join("final_balances.csv"))?);
    writeln!(f, "ACCOUNT_ID,BALANCE_CENTS")?;
    let mut g = BufWriter::new(File::create(output_dir.join("summary_report.csv"))?);
    writeln!(g, "ACCOUNT_ID,TOTAL_DEPOSITS,TOTAL_WITHDRAWALS,TXN_COUNT,ENDING_BALANCE")?;
    let (mut td, mut tw, mut tc, mut te) = (0i64, 0i64, 0i64, 0i64);
    for i in 0..n_acct {
        let d = deposits[i] as i64;
        let w = withdrawals[i] as i64;
        let c = counts[i] as i64;
        // deposits/withdrawals are already u64 off the device (see the
        // alloc above); `as i64` here is just the same narrowing every
        // other tier does for money that comfortably fits i64::MAX.
        let e = starting[i] + d - w;
        writeln!(f, "{},{}", i, e)?;
        writeln!(g, "{},{},{},{},{}", i, d, w, c, e)?;
        td += d; tw += w; tc += c; te += e;
    }
    writeln!(g, "-1,{},{},{},{}", td, tw, tc, te)?;
    eprintln!("cuda: wrote outputs in {:.2}s", t2.elapsed().as_secs_f64());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptx_embeds_the_groupby_kernel() {
        assert!(PTX.contains("groupby"), "PTX should name the kernel");
    }

    #[test]
    fn parse_i64_reads_plain_decimal() {
        assert_eq!(parse_i64(b"12345"), 12345);
        assert_eq!(parse_i64(b"0"), 0);
    }
}
