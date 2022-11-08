use clap::Parser;
use env_logger::Env;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use updvcspins::args::Args;
use updvcspins::errors::*;
use updvcspins::git;
use updvcspins::makepkg;
use updvcspins::makepkg::Source;

fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    fs::metadata(&args.pkgbuild)
        .with_context(|| anyhow!("Failed to access PKGBUILD at {:?}", args.pkgbuild))?;

    let vcspins = makepkg::list_pins(&args.pkgbuild).context("Failed to get pins from PKGBUILD")?;
    debug!("Found vcs pins: {:?}", vcspins);

    if vcspins.is_empty() {
        bail!("No vcs pins are configured (vcspins= is empty)");
    }

    let mut sources =
        makepkg::list_sources(&args.pkgbuild).context("Failed to get sources from PKGBUILD")?;

    let folder = args
        .pkgbuild
        .parent()
        .context("Failed to determine parent folder")?;

    let mut resolved_pins = HashMap::new();
    for pin in vcspins {
        debug!("Processing pin: {:?}", pin);
        let filename = pin.filename()?.to_string();

        match pin.take_source() {
            Source::File(_f) => bail!("File sources are not allowed in vcspins"),
            Source::Url(_f) => bail!("Url sources are not allowed in vcspins"),
            Source::Git(git) => {
                let repo_path = folder.join(&*filename);
                let resolved = git::run(git, &repo_path)?;
                resolved_pins.insert(filename, resolved);
            }
        }
    }

    let f = File::open(&args.pkgbuild)?;
    let r = BufReader::new(f);

    let mut out = String::new();
    let mut iter = r.lines();
    while let Some(line) = iter.next() {
        let line = line.context("Failed to decode line")?;
        trace!("Read line from PKGBUILD: {:?}", line);

        if line.starts_with("_commit") {
            let (key, first) = resolved_pins
                .iter()
                .next()
                .context("Can't use _commit if no vcspins= is set")?;
            debug!("Using repo for _commit: {:?}", key);
            writeln!(out, "_commit={}", first.commit_hash)?;
        } else if line.starts_with("_tag") {
            let (key, first) = resolved_pins
                .iter()
                .next()
                .context("Can't use _tag= if no vcspins= is set")?;
            debug!("Using repo for _tag=: {:?}", key);
            writeln!(out, "_tag={}", first.tag_hash)?;
        } else if line.starts_with("source=") {
            // skip original source array
            for line in iter.by_ref() {
                let line = line.context("Failed to decode line")?;
                if line.ends_with(')') {
                    break;
                }
            }
            // write new source array
            writeln!(out, "source=(")?;
            for input in &mut sources {
                // check if this is one of the repo's we updated our pin for
                let filename = input.filename()?;
                if let Some(pin) = resolved_pins.get(&*filename) {
                    let src = input.source_mut();
                    *src = pin.source.clone();
                    if let Source::Git(git) = src {
                        if args.pin_commit {
                            git.tag = None;
                            git.commit = Some(pin.commit_hash.clone());
                        } else {
                            git.tag = Some(pin.tag_hash.clone());
                        }
                    }
                }

                writeln!(out, "    \"{}\"", input)?;
            }
            writeln!(out, ")")?;
        } else {
            writeln!(out, "{}", line)?;
        }
    }

    if args.dry_run {
        debug!("Skipping write back because of dry run");
    } else {
        let path = args.output.unwrap_or(args.pkgbuild);
        debug!("Updating PKGBUILD...");
        fs::write(&path, &out).context("Failed to write to PKGBUILD")?;
    }

    Ok(())
}
