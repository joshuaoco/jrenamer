use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::Metadata,
    path::{Path, PathBuf},
    time::SystemTime,
};

mod file_info;




// The pair of variable name (as its used in rename string) and its value as a string
struct Fragments(HashMap<String, String>);

// This is the struct which stores all the files as they're passed from the commandline
// It can contain files which don't exist (represented by Err in file_info), and thus is to be used for reporting errors per file
struct Files<'a> {
    path_provided: &'a OsStr,
    file_info: Result<file_info::FileInfo<'a>>,
    fragments: Fragments

}

fn main() -> Result<()> {
    let matches = App::new("JRenamer")
        .version("unversioned")
        .author("Joshua O. <joshua@joshuao.com>")
        .about("Renames files scriptingly")
        .arg(
            Arg::with_name("input")
                .help("Names of the files to rename")
                .min_values(1),
        )
        .get_matches();

    let fragments = Fragments(HashMap::new());

    let finfos = match matches.values_of("input") {
        Some(vals) => vals.filter_map(|v| file_info::FileInfo::from_path(v).ok()),
        None => {
            bail!("You must supply at least one input item")
        }
    };




    //TODO: Make file info format neater, decide on OsStr
    for f in finfos.filter_map(|fi| serde_json::to_string(&fi).ok()) {
        println!("{}", f);
    }




    Ok(())
}
