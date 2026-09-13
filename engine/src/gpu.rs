// SPDX-License-Identifier: AGPL-3.0-or-later
// GPU tier: one WGSL kernel, bounded streaming, any adapter (vendor-neutral).
// The host read is parallel: rayon chunked CSV reads feeding per-chunk dispatches,
// so the 1B end-to-end is read-bound across threads instead of one.
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use wgpu::util::DeviceExt;

const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> acct: array<i32>;
@group(0) @binding(1) var<storage, read> amt: array<u32>;
@group(0) @binding(2) var<storage, read> is_dep: array<u32>;
@group(0) @binding(3) var<storage, read_write> deposits: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> withdrawals: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> counts: array<atomic<u32>>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.y * 16776960u + gid.x;
    if (i >= arrayLength(&acct)) { return; }
    let a = acct[i];
    atomicAdd(&deposits[a], is_dep[i] * amt[i]);
    atomicAdd(&withdrawals[a], (1u - is_dep[i]) * amt[i]);
    atomicAdd(&counts[a], 1u);
}
"#;

const ROWS_PER_CHUNK: usize = 8_000_000;

fn parse_i64(b: &[u8]) -> i64 {
    let mut v: i64 = 0;
    for &c in b { v = v * 10 + (c - b'0') as i64; }
    v
}

fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, src: &wgpu::Buffer, len: usize) -> Result<Vec<u32>> {
    let size = (len * 4) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging"), size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("copy") });
    enc.copy_buffer_to_buffer(src, 0, &staging, 0, size);
    queue.submit(Some(enc.finish()));
    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| { tx.send(r).unwrap(); });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().map_err(|e| anyhow!("map failed: {e:?}"))?;
    let data = slice.get_mapped_range();
    let out: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging.unmap();
    Ok(out)
}

fn dispatch_chunk(
    device: &wgpu::Device, queue: &wgpu::Queue, pipeline: &wgpu::ComputePipeline,
    bgl: &wgpu::BindGroupLayout, dep_buf: &wgpu::Buffer, wd_buf: &wgpu::Buffer, cnt_buf: &wgpu::Buffer,
    acct: &[i32], amt: &[u32], is_dep: &[u32],
) {
    let acct_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("acct"), contents: bytemuck::cast_slice(acct),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let amt_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("amt"), contents: bytemuck::cast_slice(amt),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let is_dep_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("is_dep"), contents: bytemuck::cast_slice(is_dep),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bg"), layout: bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: acct_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: amt_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: is_dep_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: dep_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wd_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: cnt_buf.as_entire_binding() },
        ],
    });
    let cn = acct.len() as u32;
    let needed = cn.div_ceil(256);
    let x_dim = needed.min(65535).max(1);
    let y_dim = needed.div_ceil(65535).max(1);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("e") });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(x_dim, y_dim, 1);
    }
    queue.submit(Some(encoder.finish()));
}

pub fn run(input_dir: &Path, output_dir: &Path, adapter_idx: usize) -> Result<()> {
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

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapters = instance.enumerate_adapters(wgpu::Backends::all());
    if adapters.is_empty() { return Err(anyhow!("no wgpu adapters")); }
    let adapter = adapters.get(adapter_idx).ok_or_else(|| anyhow!("adapter {adapter_idx} out of range"))?;
    println!("adapter[{}]: {} ({:?})", adapter_idx, adapter.get_info().name, adapter.get_info().backend);
    let limits = adapter.limits();
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor { required_limits: limits.clone(), ..Default::default() }, None))?;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("groupby"), source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("groupby-pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pl"), bind_group_layouts: &[&bgl], push_constant_ranges: &[],
        })),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let agg_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    let dep_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("deposits"), size: (n_acct * 4) as u64, usage: agg_usage, mapped_at_creation: false,
    });
    let wd_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("withdrawals"), size: (n_acct * 4) as u64, usage: agg_usage, mapped_at_creation: false,
    });
    let cnt_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("counts"), size: (n_acct * 4) as u64, usage: agg_usage, mapped_at_creation: false,
    });

    let t0 = Instant::now();
    let txn_path = input_dir.join("transactions.csv");
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8).min(8);
    let starts = crate::cpu::chunk_starts(&txn_path, n_threads)?;
    let n = AtomicUsize::new(0);
    starts.par_iter().enumerate().for_each(|(i, &start)| {
        let end = starts.get(i + 1).copied().unwrap_or(u64::MAX);
        let mut f = File::open(&txn_path).unwrap();
        // every chunk begins on a data row: chunk_starts lands chunk 0 just
        // after the header newline and later chunks just after a row newline
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
                dispatch_chunk(&device, &queue, &pipeline, &bgl, &dep_buf, &wd_buf, &cnt_buf, &acct, &amt, &is_dep);
                acct.clear(); amt.clear(); is_dep.clear();
            }
        }
        if !acct.is_empty() {
            dispatch_chunk(&device, &queue, &pipeline, &bgl, &dep_buf, &wd_buf, &cnt_buf, &acct, &amt, &is_dep);
        }
        n.fetch_add(local, Ordering::Relaxed);
    });
    let n = n.load(Ordering::Relaxed);
    device.poll(wgpu::Maintain::Wait);
    println!("gpu: {} threads, read+dispatch {} rows in {:.2}s", n_threads, n, t0.elapsed().as_secs_f64());

    crate::key::gate(n as u64).map_err(|e| anyhow!(e))?;

    let deposits = read_back(&device, &queue, &dep_buf, n_acct)?;
    let withdrawals = read_back(&device, &queue, &wd_buf, n_acct)?;
    let counts = read_back(&device, &queue, &cnt_buf, n_acct)?;

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
        let e = starting[i] + d - w;
        writeln!(f, "{},{}", i, e)?;
        writeln!(g, "{},{},{},{},{}", i, d, w, c, e)?;
        td += d; tw += w; tc += c; te += e;
    }
    writeln!(g, "-1,{},{},{},{}", td, tw, tc, te)?;
    println!("gpu: wrote outputs in {:.2}s", t2.elapsed().as_secs_f64());
    Ok(())
}
