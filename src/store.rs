use super::{Error, Step};
use std::env::current_exe;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Store {
    pub path: PathBuf,
}

// impl<M: State<M>> Store<M> {
impl Store {
    /// Creates a new store, using `<exe_stem>.json` file
    pub fn new() -> Result<Self, Error> {
        let mut path = exe_stem()?;
        path.set_extension("json");
        Ok(Self { path })
    }

    /// Saves the step
    pub fn save<M: serde::Serialize>(&self, step: &Step<M>) -> Result<(), Error> {
        let text = serde_json::to_string_pretty(&step)?;
        Ok(fs::write(&self.path, text)?)
    }

    /// Loads a step
    pub fn load<M: serde::de::DeserializeOwned>(&self) -> Result<Option<Step<M>>, Error> {
        let text = match fs::read_to_string(&self.path) {
            Ok(x) => Ok(x),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => Err(e),
        }?;
        Ok(serde_json::from_str(&text)?)
    }

    /// Cleans the store by removing the json file
    pub fn clean(&self) -> Result<(), Error> {
        fs::remove_file(&self.path)?;
        Ok(())
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
