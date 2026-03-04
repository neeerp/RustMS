mod compare;
mod cosmic_reader;
mod model;
mod nx_reader;
mod report;

use anyhow::{Context, Result};
use clap::Parser;
use compare::CompareOptions;
use model::ValidationReport;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "map-data-validator",
    about = "Compares NX portal data against Cosmic XML portal data"
)]
struct Args {
    /// Path to Map.nx
    #[arg(long)]
    nx_map: PathBuf,

    /// Path to Cosmic Map root (e.g. .../Map.wz/Map)
    #[arg(long)]
    cosmic_map_root: PathBuf,

    /// Optional comma-separated map ids to validate (e.g. 100000000,100000001)
    #[arg(long, value_delimiter = ',')]
    maps: Vec<i32>,

    /// Promote soft mismatches to failures
    #[arg(long, default_value_t = false)]
    strict: bool,

    /// Optional path to write JSON report
    #[arg(long)]
    report: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    match run(args) {
        Ok(report) => {
            print_summary(&report);
            if report.summary.should_fail {
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(2);
        }
    }
}

fn run(args: Args) -> Result<ValidationReport> {
    let target_maps = if args.maps.is_empty() {
        None
    } else {
        Some(args.maps.into_iter().collect::<BTreeSet<_>>())
    };

    let nx_maps = nx_reader::read_maps(&args.nx_map, target_maps.as_ref())
        .with_context(|| format!("failed to read NX maps from {}", args.nx_map.display()))?;
    let cosmic_maps = cosmic_reader::read_maps(&args.cosmic_map_root, target_maps.as_ref())
        .with_context(|| {
            format!(
                "failed to read Cosmic maps from {}",
                args.cosmic_map_root.display()
            )
        })?;

    let report = compare::compare_maps(
        &nx_maps,
        &cosmic_maps,
        &CompareOptions {
            target_maps,
            strict: args.strict,
        },
    );

    if let Some(path) = args.report.as_ref() {
        report::write_json(path, &report)
            .with_context(|| format!("failed to write report to {}", path.display()))?;
    }

    Ok(report)
}

fn print_summary(report: &ValidationReport) {
    println!("Map Data Validation Summary");
    println!("  maps compared: {}", report.summary.maps_compared);
    println!("  exact maps: {}", report.summary.exact_maps);
    println!(
        "  maps with hard mismatches: {}",
        report.summary.maps_with_hard
    );
    println!(
        "  maps with soft mismatches: {}",
        report.summary.maps_with_soft
    );
    println!(
        "  maps with warnings: {}",
        report.summary.maps_with_warnings
    );
    println!("  hard mismatch count: {}", report.summary.hard_count);
    println!("  soft mismatch count: {}", report.summary.soft_count);
    println!("  warning count: {}", report.summary.warning_count);
    println!("  should fail: {}", report.summary.should_fail);
}
