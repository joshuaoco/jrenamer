use anyhow::{anyhow, Context, Result};
use regex::{Captures, Regex};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fmt,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::SystemTime,
};

use crate::file_info::FileInfo;
use serde::ser::{self, Serialize, SerializeStruct};

// This is the struct which stores all the files as they're passed from the commandline
// It can contain files which don't exist (represented by Err in file_info), and thus is to be used for reporting errors per file
#[derive(Debug)]
pub struct File<'a> {
    pub path_provided: &'a OsStr,
    pub file_info: Result<FileInfo<'a>>,
    pub fragments: Fragments,
}

impl Serialize for File<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.file_info.is_err() {
            return Err(ser::Error::custom(
                "Can't serialize a file that doesnt exist",
            ));
        }

        let mut state = serializer.serialize_struct("File", 2)?;
        state.serialize_field("file_info", self.file_info.as_ref().unwrap())?;
        state.serialize_field("fragments", &self.fragments)?;
        state.end()
    }
}

impl File<'_> {
    pub fn from_path<T: AsRef<Path> + ?Sized>(path: &T) -> File {
        let path_provided = path.as_ref().as_os_str();
        let file_info = FileInfo::from_path(path);
        let fragments = Fragments(HashMap::new());
        File {
            path_provided,
            file_info,
            fragments,
        }
    }

    /// Runs the Python script at ~script~, fragments are passed as json to the stdin of the script
    /// executable and the script should respond with a json comprising fragment names with their contents
    /// these are then appended to the Files fragments set
    pub fn run_script(&mut self, script: &Path) -> Result<()> {
        let mut child = Command::new("python")
            .arg(script)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context(anyhow!(
                "Couldn't spawn child process for {}",
                script.to_string_lossy()
            ))?;

        // Send current fragments and file info through stdin to process

        child
            .stdin
            .as_mut()
            .context(anyhow!(
                "Couldn't get stdin for {}",
                script.to_string_lossy()
            ))?
            .write_all(serde_json::to_string(&self.fragments)?.as_bytes())?;

        // Wait for process to finish and capture its output
        // TODO Don't wait forever
        let output = child.wait_with_output()?;

        if output.status.success() {
            let raw_output = String::from_utf8(output.stdout)?; //TODO Can we not do this
            self.add_to_frags(&raw_output);
        } else {
            let err = String::from_utf8(output.stderr)?;
            println!("Error from script: {}", err);
        }

        Ok(())
    }

    pub fn exists(&self) -> bool {
        self.file_info.is_ok()
    }

    /// Adds s to Files fragments assuming its a well formed string:string JSON object
    fn add_to_frags(&mut self, s: &str) {
        match serde_json::from_str::<HashMap<String, String>>(s) {
            Ok(frags) => self.fragments.0.extend(frags),
            Err(e) => eprintln!("Error with script return json: {}", e),
        }
    }

    /// Parses the inherent file information for a File and adds it to that files Fragments
    /// This will always include the filename and relative path, but all other information is actually fallible
    ///
    /// It is at this point that the file information elements are converted to Strings from OsStrs,
    /// as such there can be loss of information in non UTF-8 convertable text.
    /// For now it silently converts and doesn't report any issues, since they're relatively rare.
    pub fn add_file_info_to_fragments(&mut self) {
        if let Ok(fi) = &self.file_info {
            self.fragments.0.insert(
                "filename".to_string(),
                fi.filename.to_string_lossy().to_string(),
            );

            if let Some(extension_os) = fi.extension {
                self.fragments.0.insert(
                    "extension".to_string(),
                    extension_os.to_string_lossy().to_string(),
                );
            }

            self.fragments
                .0
                .insert("path".to_string(), fi.path.to_string_lossy().to_string());

            if let Some(abs_path) = &fi.absolute_path {
                self.fragments.0.insert(
                    "absolute_path".to_string(),
                    abs_path.to_string_lossy().to_string(),
                );
            }

            // Helper function to get the seconds from a SystemTime if present,
            // used since there's many times we want to extract the seconds from
            let s_if_there = |t: &Option<SystemTime>, key: &str| {
                if let Some(t) = t {
                    if let Ok(n) = t.duration_since(SystemTime::UNIX_EPOCH) {
                        //TODO: Need to deal with times being set before unix epoch?

                        return Some((key.to_string(), n.as_secs().to_string()));
                    }
                }
                None
            };

            if let Some((k, v)) = s_if_there(&fi.accessed, "accessed") {
                self.fragments.0.insert(k, v);
            }

            if let Some((k, v)) = s_if_there(&fi.created, "created") {
                self.fragments.0.insert(k, v);
            }

            if let Some((k, v)) = s_if_there(&fi.modified, "modified") {
                self.fragments.0.insert(k, v);
            }

            self.fragments
                .0
                .insert("filesize".to_string(), fi.filesize.to_string());
        }
    }

    pub fn parse_fstring(&self, fstring: &str) -> String {
        //TODO: Make this into a lazy static
        let re: Regex = Regex::new(r"\#(.*?)\#").unwrap();

        re.replace_all(fstring, |caps: &Captures| {
            // Handle the escaped ## case
            if caps[1].is_empty() {
                return "#".to_string();
            };

            if let Some(v) = self.fragments.0.get(&caps[1]) {
                v.to_string()
            } else {
                caps[0].to_string()
            }
        })
        .into_owned()
    }
}

// The pair of variable name (as its used in rename string) and its value as a string
#[derive(Debug, serde::Serialize)]
pub struct Fragments(HashMap<String, String>);

impl fmt::Display for Fragments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0
            .iter()
            .try_for_each(|(k, v)| writeln!(f, "{}: {}", k, v))
    }
}

#[cfg(test)]
mod tests {
    use super::File;

    fn example_file() -> File<'static> {
        File::from_path("Cargo.toml")
    }

    #[test]
    fn test_parse_fstring() {
        let mut file = example_file();
        file.fragments
            .0
            .insert("date".to_string(), "12345".to_string());
        file.fragments
            .0
            .insert("scariness".to_string(), "very_scary".to_string());

        assert_eq!(
            file.parse_fstring("#date#_my_file_is#scariness#_and_only_cost_##_dolars.txt"),
            "12345_my_file_isvery_scary_and_only_cost_#_dolars.txt"
        )
    }

    #[test]
    fn test_fstring_alternation() {
        let mut file = example_file();
        file.fragments
            .0
            .insert("date".to_string(), "12345".to_string());

        assert_eq!(
            file.parse_fstring(
                "(#date#|fail)_my_file_is(#scariness#|not scary)_and_only_cost_##_dolars.txt"
            ),
            "12345_my_file_isnot scary_and_only_cost_#_dolars.txt"
        )
    }
}
