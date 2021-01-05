use anyhow::{bail, Context, Result};
use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::Metadata,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// Main struct that is used for creating names
#[derive(Serialize, Debug)]
struct FileInfo<'a> {
    filename: &'a OsStr,
    extension: Option<&'a OsStr>,
    path: &'a Path,
    absolute_path: Option<PathBuf>,
    accessed: Option<SystemTime>,
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    filesize: Option<u64>,
}


impl FileInfo<'_> {
    fn from_path<'a, T: AsRef<Path> + ?Sized>(path: &'a T) -> Result<FileInfo<'a>> {
        let path = path.as_ref();

        let filename = path.file_name().context("File has no filename")?;
        let extension = path.extension();
        let absolute_path = path.canonicalize().ok();

        let md = path.metadata().ok();

        fn call_and_flatten<U, V>(
            meta: Option<&Metadata>,
            f: impl Fn(&Metadata) -> Result<U, V>,
        ) -> Option<U> {
            meta.map(|m| f(m).ok()).flatten()
        }

        let accessed = call_and_flatten(md.as_ref(), Metadata::accessed);
        let created = call_and_flatten(md.as_ref(), Metadata::created);
        let modified = call_and_flatten(md.as_ref(), Metadata::modified);

        let filesize = md.as_ref().map(Metadata::len);

        Ok(FileInfo {
            filename,
            extension,
            path,
            absolute_path,
            accessed,
            created,
            modified,
            filesize,
        })
    }
}

struct Fragments(HashMap<String, String>);

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
        Some(vals) => vals.filter_map(|v| FileInfo::from_path(v).ok()),
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
