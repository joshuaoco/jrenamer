use anyhow::{anyhow, bail, Context, Error, Result};
use clap::{App, Arg, ArgMatches};
use file_info::FileInfo;
use std::{collections::HashMap, ffi::OsStr, io::{self, Write}, path::Path, process::{Command, Stdio}};
#[macro_use] extern crate lazy_static;
mod file_info;
mod file;
use file::File;


fn path_exists<'a, T: AsRef<Path> + ?Sized>(path: &'a T) -> Result<&'a Path> {
    let path = path.as_ref();
    if path.exists() {
        Ok(path)
    } else {
        Err(anyhow!("File {} doesn't exist", path.to_string_lossy()))
    }
}


fn main() -> Result<()> {
    let matches = get_matches();

    // First we resolve the paths into their file info format
    let mut files = get_files(&matches)?;

    // Then we resolve scripts, checking if they exist and reporting if they don't
    let scripts = get_scripts(&matches);

    // Run the scripts against the files, building up fragments as we go
    for f in files.iter_mut()  {
        if f.exists(){

            f.add_file_info_to_fragments();

            for s in scripts.iter() {
                f.run_script(s)?;
            }

            println!("File: {}", f.path_provided.to_string_lossy());
            println!("Fragments from scripts available: {:?}",  f.fragments);

        //TODO Get user input for filename
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(n) => {
                    println!("{} bytes read", n);
                    println!("{}", input);
                }
                Err(error) => println!("error: {}", error),
            }

        //TODO Parse user input and insert fragments where needed
            println!("{}", f.parse_fstring(&input));
        }

    }
    Ok(())
}




// TODO: Genericise

fn get_files<'a>(ms: &'a ArgMatches) -> Result<Vec<File<'a>>> {
    // First we resolve the paths into their file info format
    match ms.values_of("input") {
        Some(vals) => Ok(vals.map(File::from_path).collect::<Vec<File>>()),
        None => {
            bail!("You must supply at least one input item")
        }
    }
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
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .get_matches()
}
