// SPDX-License-Identifier: AGPL-3.0-or-later
// CPU tier: parallel chunked CSV read (rayon) + exact i64 accumulation.
// Byte-exact against the COBOL reference at every scale; peak memory bounded
// by n_threads * n_acct * 24 bytes.
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Instant;

fn parse_i64(b: &[u8]) -> i64 {
    let mut v: i64 = 0;
    for &c in b { v = v * 10 + (c - b'0') as i64; }
    v
}

pub(crate) fn chunk_starts(path: &Path, n_threads: usize) -> Result<Vec<u64>> {
    let mut f = File::open(path)?;
    let size = f.metadata()?.len();
    let chunk = size / n_threads as u64;
    let mut starts = Vec::with_capacity(n_threads);
    for i in 0..n_threads {
        let mut pos = i as u64 * chunk;
        // advance to just after the next newline (chunk 0: after the header line)
        let mut buf = [0u8; 1];
        loop {
            if pos >= size { pos = size; break; }
            f.seek(SeekFrom::Start(pos))?;
            f.read_exact(&mut buf)?;
            if buf[0] == b'\n' { pos += 1; break; }
            pos += 1;
        }
        starts.push(pos.min(size));
    }
    Ok(starts)
}

pub fn run(input_dir: &Path, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let t0 = Instant::now();

    // balances define the account domain
    let mut rdr = csv::ReaderBuilder::new().has_headers(true)
        .from_path(input_dir.join("balances.csv"))?;
    let mut starting: Vec<i64> = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let id = parse_i64(rec.get(0).unwrap().as_bytes()) as usize;
        let b = parse_i64(rec.get(1).unwrap().as_bytes());
        if starting.len() <= id { starting.resize(id + 1, 0); }
        starting[id] = b;
    }
    let n_acct = starting.len();

    let txn_path = input_dir.join("transactions.csv");
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).min(8);
    let starts = chunk_starts(&txn_path, n_threads)?;

    let results: Vec<(Vec<i64>, Vec<i64>, Vec<i64>, usize)> = starts
        .par_iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = starts.get(i + 1).copied().unwrap_or(u64::MAX);
            let mut f = File::open(&txn_path).unwrap();
            f.seek(SeekFrom::Start(start)).unwrap();
            let limited = f.take(end - start);
            let mut rdr = csv::ReaderBuilder::new().has_headers(false).from_reader(limited);
            let mut deposits = vec![0i64; n_acct];
            let mut withdrawals = vec![0i64; n_acct];
            let mut counts = vec![0i64; n_acct];
            let mut local_n = 0usize;
            for rec in rdr.records() {
                let rec = match rec { Ok(r) => r, Err(_) => continue };
                let id = parse_i64(rec.get(0).unwrap().as_bytes()) as usize;
                let amt = parse_i64(rec.get(1).unwrap().as_bytes());
                if rec.get(2).unwrap().as_bytes()[0] == b'D' {
                    deposits[id] += amt;
                } else {
                    withdrawals[id] += amt;
                }
                counts[id] += 1;
                local_n += 1;
            }
            (deposits, withdrawals, counts, local_n)
        })
        .collect();

    let mut deposits = vec![0i64; n_acct];
    let mut withdrawals = vec![0i64; n_acct];
    let mut counts = vec![0i64; n_acct];
    let mut n = 0usize;
    for (d, w, c, ln) in results {
        n += ln;
        for i in 0..n_acct {
            deposits[i] += d[i];
            withdrawals[i] += w[i];
            counts[i] += c[i];
        }
    }
    let read_s = t0.elapsed().as_secs_f64();

    crate::key::announce_licence();

    let t2 = Instant::now();
    let mut f = BufWriter::new(File::create(output_dir.join("final_balances.csv"))?);
    writeln!(f, "ACCOUNT_ID,BALANCE_CENTS")?;
    let mut g = BufWriter::new(File::create(output_dir.join("summary_report.csv"))?);
    writeln!(g, "ACCOUNT_ID,TOTAL_DEPOSITS,TOTAL_WITHDRAWALS,TXN_COUNT,ENDING_BALANCE")?;
    let (mut td, mut tw, mut tc, mut te) = (0i64, 0i64, 0i64, 0i64);
    for i in 0..n_acct {
        let d = deposits[i];
        let w = withdrawals[i];
        let c = counts[i];
        let e = starting[i] + d - w;
        writeln!(f, "{},{}", i, e)?;
        writeln!(g, "{},{},{},{},{}", i, d, w, c, e)?;
        td += d; tw += w; tc += c; te += e;
    }
    writeln!(g, "-1,{},{},{},{}", td, tw, tc, te)?;
    if n == 0 { return Err(anyhow!("no rows parsed")); }
    eprintln!("cpu: {} threads, read+agg {:.2}s ({} rows), write {:.2}s", n_threads, read_s, n, t2.elapsed().as_secs_f64());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_starts_cover_file_and_land_after_newlines() {
        let dir = std::env::temp_dir().join("bylazora-chunk-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        let mut rows = String::from("id,amt,dir\n");
        for i in 0..1000 { rows.push_str(&format!("{i},123,D\n")); }
        std::fs::write(&p, &rows).unwrap();
        let starts = chunk_starts(&p, 4).unwrap();
        let data = rows.as_bytes();
        assert_eq!(starts.len(), 4);
        for (i, &s) in starts.iter().enumerate() {
            if i == 0 {
                assert_eq!(s, 11, "chunk 0 starts after the header newline");
            } else {
                assert_eq!(data[s as usize - 1], b'\n', "chunk {i} starts after a newline");
            }
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

