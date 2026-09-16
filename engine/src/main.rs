// SPDX-License-Identifier: AGPL-3.0-or-later
// bylazora-core: the migration engine in Rust.
mod mcp;

use anyhow::Result;
#[cfg(feature = "cuda")]
use bylazora::cuda;
use bylazora::{copybook, cpu, db2, gpu, key, migrate, validator};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Backend {
    Cpu,
    /// vendor-neutral wgpu, any adapter
    Gpu,
    /// NVIDIA only; needs a build with --features cuda
    Cuda,
}

#[derive(Parser)]
#[command(name = "bylazora-core", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Byte-compare two output directories; exit 0 only when identical
    Validate {
        #[arg(long)]
        ref_dir: PathBuf,
        #[arg(long)]
        other_dir: PathBuf,
    },
    /// Run a tier and write byte-exact outputs
    Bench {
        input_dir: PathBuf,
        output_dir: PathBuf,
        #[arg(long, value_enum, default_value = "cpu")]
        backend: Backend,
        /// wgpu adapter index for the gpu backend
        #[arg(long, default_value_t = 0)]
        adapter: usize,
    },
    /// Minimal MCP server over stdio (tools: validate, bench)
    Mcp,
    /// Manage the production licence key (offline; no network is ever used)
    Licence {
        #[command(subcommand)]
        action: LicenceAction,
    },
    /// Import a DB2 schema and DEL unload into canonical CSV
    Db2 {
        #[command(subcommand)]
        action: Db2Action,
    },
    /// Parse a COBOL copybook into its field schema
    Copybook {
        #[command(subcommand)]
        action: CopybookAction,
    },
    /// Scaffold and drive a gated migration workspace (new/reference/verify/status)
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
}

#[derive(Subcommand)]
enum CopybookAction {
    /// Parse a copybook source file and write its field schema as JSON
    Parse {
        #[arg(long)]
        src: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum LicenceAction {
    /// Install a key to the licence file (accepts only valid, unexpired keys)
    Set {
        /// The key from your order confirmation
        key: String,
        /// Override the licence file path (default: XDG config, or BYLAZORA_KEY_FILE)
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Show the installed key's coverage (org, tier, expiry)
    Show,
    /// Exit 0 only when a valid, unexpired key is installed
    Check,
    /// Remove the installed key file
    Clear {
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum MigrateAction {
    /// Scaffold a gated migration workspace for one job
    New {
        job: String,
        /// Parent directory for the workspace (default: current directory)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Declared input file, relative to the job's input/ directory
        #[arg(long = "input")]
        inputs: Vec<String>,
        /// Declared output file name
        #[arg(long = "output")]
        outputs: Vec<String>,
        /// COBOL copybook to render into the job schema
        #[arg(long)]
        copybook: Option<PathBuf>,
        /// Legacy COBOL source for the GnuCOBOL reference capture
        #[arg(long = "cobol-source")]
        cobol_source: Option<PathBuf>,
    },
    /// Seal the reference outputs (the byte-level contract)
    Reference {
        job: PathBuf,
        /// Copy the reference from this directory instead of running capture.sh
        #[arg(long)]
        from: Option<PathBuf>,
        /// Replace an existing seal
        #[arg(long)]
        force: bool,
    },
    /// Build, run and gate the target against the sealed reference
    Verify { job: PathBuf },
    /// Show the job contract and the latest verdict
    Status { job: PathBuf },
}

#[derive(Subcommand)]
enum Db2Action {
    /// Import a DB2 DEL unload using a schema into canonical CSV
    Import {
        /// DB2 schema JSON (columns: name, db2_type, precision, scale, nullable, key)
        #[arg(long)]
        schema: PathBuf,
        /// DB2 DEL unload file
        #[arg(long)]
        del: PathBuf,
        /// Output CSV path
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Validate { ref_dir, other_dir } => {
            let errs = validator::compare_outputs(&ref_dir, &other_dir)?;
            if errs.is_empty() {
                println!("IDENTICAL");
                Ok(())
            } else {
                for e in &errs { eprintln!("{e}"); }
                std::process::exit(1);
            }
        }
        Cmd::Bench { input_dir, output_dir, backend, adapter } => {
            match backend {
                Backend::Cpu => cpu::run(&input_dir, &output_dir),
                Backend::Gpu => gpu::run(&input_dir, &output_dir, adapter),
                #[cfg(feature = "cuda")]
                Backend::Cuda => cuda::run(&input_dir, &output_dir, adapter),
                #[cfg(not(feature = "cuda"))]
                Backend::Cuda => Err(anyhow::anyhow!(
                    "cuda backend not available: this binary was built without --features cuda (requires the CUDA toolkit)"
                )),
            }
        }
        Cmd::Mcp => mcp::run(),
        Cmd::Licence { action } => {
            let r = match action {
                LicenceAction::Set { key, file } => key::cmd_set(&key, file.as_ref()),
                LicenceAction::Show => key::cmd_show(),
                LicenceAction::Check => key::cmd_check().map(|_| "licence valid".to_string()),
                LicenceAction::Clear { file } => key::cmd_clear(file.as_ref()),
            };
            match r {
                Ok(m) => { println!("{m}"); Ok(()) }
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
        Cmd::Db2 { action } => match action {
            Db2Action::Import { schema, del, out } => {
                let json = std::fs::read_to_string(&schema)?;
                let cols = db2::parse_schema(&json).map_err(anyhow::Error::msg)?;
                let del_text = std::fs::read_to_string(&del)?;
                let rows = db2::import_del(&cols, &del_text, &out).map_err(anyhow::Error::msg)?;
                println!("imported {rows} rows to {}", out.display());
                Ok(())
            }
        },
        Cmd::Copybook { action } => match action {
            CopybookAction::Parse { src, out } => {
                let text = std::fs::read_to_string(&src)?;
                let fields = copybook::parse_copybook(&text).map_err(anyhow::Error::msg)?;
                let schema = copybook::field_schema(&fields);
                std::fs::write(&out, serde_json::to_string_pretty(&schema)?)?;
                println!("parsed {} fields to {}", fields.len(), out.display());
                Ok(())
            }
        },
        Cmd::Migrate { action } => {
            let r = match action {
                MigrateAction::New { job, dir, inputs, outputs, copybook, cobol_source } => {
                    migrate::cmd_new(&job, dir.as_deref(), &inputs, &outputs, copybook.as_deref(), cobol_source.as_deref())
                        .map(|_| "workspace scaffolded".to_string())
                }
                MigrateAction::Reference { job, from, force } => {
                    migrate::cmd_reference(&job, from.as_deref(), force).map(|_| "reference sealed".to_string())
                }
                MigrateAction::Verify { job } => migrate::cmd_verify(&job).map(|_| "IDENTICAL".to_string()),
                MigrateAction::Status { job } => migrate::cmd_status(&job),
            };
            match r {
                Ok(m) => { println!("{m}"); Ok(()) }
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
    }
}
