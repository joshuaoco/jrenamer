use anyhow::{anyhow, bail, Result};
use clap::{App, Arg, ArgMatches};

use std::{fs, io, path::Path};

extern crate lazy_static;
mod file;
mod file_info;
use file::File;

fn main() -> Result<()> {
    let matches = get_matches();

    // First we resolve the paths into their file info format
    let mut files = get_files(&matches)?;

    // Then we resolve scripts, checking if they exist and reporting if they don't
    let scripts = get_scripts(&matches);

    // Operate on one file at a time
    // TODO: Parallelise optionally?
    for f in files.iter_mut() {
        if f.exists() {
            // Add the base stat-ish file info as fragments
            f.add_file_info_to_fragments();

            // Run each script in turn and continue to build up fragments
            for s in scripts.iter() {
                f.run_script(s)?;
            }

            // If the format string was specified with a flag, use that, else prompt for it
            let fstring = match matches.value_of("format") {
                Some(val) => val.to_string(),
                None => user_fstring(f)?,
            };

            let new_name = f.parse_fstring(&fstring);
            // TODO: Provide a dry run option which just displays what change would be made
            fs::rename(f.path_provided, new_name)?;
        }
    }
    Ok(())
}

// TODO: Genericise

fn get_files<'a>(ms: &'a ArgMatches) -> Result<Vec<File<'a>>> {
    match ms.values_of("input") {
        Some(vals) => Ok(vals.map(File::from_path).collect::<Vec<File>>()),
        None => {
            bail!("You must supply at least one input item")
        }
    }
}

fn user_fstring(f: &File) -> Result<String> {
    println!("File: {}", f.path_provided.to_string_lossy());
    println!("Fragments from scripts available: {:?}", f.fragments);

    //TODO Get user input for filename
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn get_scripts<'a>(ms: &'a ArgMatches) -> Vec<&'a Path> {
    match ms.values_of("script") {
        Some(vals) => vals
            .filter_map(|v| match path_exists(v) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("{}", e);
                    None
                }
            })
            .collect::<Vec<&Path>>(),

        None => Vec::new(),
    }
}

fn get_matches() -> ArgMatches<'static> {
    App::new("JRenamer")
        .version("unversioned")
        .author("Joshua O. <joshua@joshuao.com>")
        .about("Renames files scriptingly")
        .arg(
            Arg::with_name("input")
                .help("Names of the files to rename")
                .min_values(1),
        )
        .arg(
            Arg::with_name("script")
                .short("s")
                .long("script")
                .value_name("script")
                .multiple(true)
                .require_delimiter(true)
                .help("List of scripts to run")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .value_name("format_string")
                .multiple(false)
                .help("Use this string instead of prompting for input")
                .takes_value(true),
        )
        .get_matches()
}

fn path_exists<T: AsRef<Path> + ?Sized>(path: &T) -> Result<&Path> {
    let path = path.as_ref();
    if path.exists() {
        Ok(path)
    } else {
        Err(anyhow!("File {} doesn't exist", path.to_string_lossy()))
    }
}
