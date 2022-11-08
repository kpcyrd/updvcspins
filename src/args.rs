// use clap::{ArgAction, Parser, Subcommand};
use clap::{ArgAction, Parser};
use std::path::PathBuf;
// use strum::VariantNames;

#[derive(Debug, Parser)]
pub struct Args {
    /// Turn debugging information on
    #[arg(short, long, global = true, action(ArgAction::Count))]
    pub verbose: u8,
    /// Path to PKGBUILD
    #[arg(short, long, default_value = "PKGBUILD")]
    pub pkgbuild: PathBuf,
    /// Attempt update but do not write to PKGBUILD
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    /// Write updated PKGBUILD to this path
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Pin commits instead of tag object hashes
    #[arg(long)]
    pub pin_commit: bool,
}
