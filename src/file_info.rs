use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use serde::Serialize;

/// Main struct that is used for creating names
//TODO: Improve and stabalize file_info format to scripts
#[derive(Serialize, Debug)]
pub struct FileInfo<'a> {
    pub filename: &'a OsStr,
    pub extension: Option<&'a OsStr>,
    pub path: &'a Path,
    pub absolute_path: Option<PathBuf>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub filesize: u64,
}

impl FileInfo<'_> {
    pub fn from_path<'a, T: AsRef<Path> + ?Sized>(path: &'a T) -> Result<FileInfo<'a>> {
        let path = path.as_ref();

        let filename = path.file_name().context("File has no filename")?;
        let extension = path.extension();
        let absolute_path = path.canonicalize().ok();

        let md = path.metadata().with_context(|| {
            format!(
                "File \"{}\" probably doesn't exist",
                filename.to_string_lossy()
            )
        })?;

        let accessed = md.accessed().ok();
        let created = md.created().ok();
        let modified = md.modified().ok();

        let filesize = md.len();

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
