use super::{State, Store};
use std::env::current_exe;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("can't read file `{1}`")]
    ReadFile(#[source] io::Error, PathBuf),
    #[error("can't write file `{1}`")]
    WriteFile(#[source] io::Error, PathBuf),
    #[error("can't remove file `{1}`")]
    RemoveFile(#[source] io::Error, PathBuf),
    #[error("can't decode json: {1}")]
    Decode(#[source] serde_json::Error, String),
    #[error("can't encode state into json: {1:?}")]
    Encode(#[source] serde_json::Error, String),
    #[error("can't find executable steam")]
    ExeStem(#[source] io::Error),
}

#[derive(Debug)]
pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    /// Creates a new store, using `<exe_stem>.json` file
    pub fn new() -> Result<Self, Error> {
        let mut path = exe_stem().map_err(Error::ExeStem)?;
        path.set_extension("json");
        Ok(Self { path })
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = path.as_ref().into();
        self
    }
}

impl Store for JsonStore {
    type Error = Error;

    /// Loads a step
    fn load(&self) -> Result<Option<State>, Self::Error> {
        let json = match fs::read_to_string(&self.path) {
            Ok(x) => x,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Error::ReadFile(e, self.path.clone())),
        };
        serde_json::from_str(&json).map_err(|e| Error::Decode(e, json))
    }

    /// Saves the step
    fn save(&self, state: &State) -> Result<(), Self::Error> {
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| Error::Encode(e, format!("{:?}", state)))?;
        fs::write(&self.path, json).map_err(|e| Error::WriteFile(e, self.path.clone()))
    }

    /// Cleans the store by removing the json file
    fn clean(&self) -> Result<(), Self::Error> {
        fs::remove_file(&self.path).map_err(|e| Error::RemoveFile(e, self.path.clone()))
    }
}

fn exe_stem() -> io::Result<PathBuf> {
    let mut path = current_exe()?;
    let stem = path
        .file_stem()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no exe file stem"))?
        .to_owned();
    path.set_file_name(stem);
    Ok(path)
}
