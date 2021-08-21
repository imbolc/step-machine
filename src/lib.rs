use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use std::io;
use std::path::PathBuf;
use store::Store;

mod store;

type StdResult<T, E> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Step(String),
}

pub type Result<T> = StdResult<T, Error>;

/// Represents state of a state machine M
pub trait State<M: State<M>> {
    type Error: fmt::Debug;

    /// Runs the current step and returns the next machine state or `None` if everything is done
    fn next(self) -> StdResult<Option<M>, Self::Error>;
}

/// Machine state with metadata to store
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Step<M> {
    /// Current state of the machine
    pub state: M,
    /// An error if any
    pub error: Option<String>,
}

impl<M> Step<M> {
    fn new(state: M) -> Self {
        Self {
            state: state,
            error: None,
        }
    }
}

#[derive(Debug)]
pub struct Engine<M> {
    store: Store,
    step: Step<M>,
}

impl<M> Engine<M>
where
    M: fmt::Debug + Serialize + DeserializeOwned + State<M>,
{
    /// Creates an Engine using initial state
    pub fn new(state: M) -> Result<Self> {
        let store: Store = Store::new()?;
        let step = Step::new(state);
        Ok(Self { store, step })
    }

    /// Use another store path
    pub fn with_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        self.store.path = path;
        self
    }

    /// Restores an Engine from the previous run
    pub fn restore(mut self) -> Result<Self> {
        if let Some(step) = self.store.load()? {
            self.step = step;
        }
        Ok(self)
    }

    /// Runs all steps to completion
    pub fn run(mut self) -> Result<()> {
        if let Some(e) = self.step.error.as_ref() {
            return Err(crate::Error::Step(format!(
                "Previous run resulted in an error: {} on step: {:?}",
                e, self.step.state
            )));
        }

        loop {
            log::info!("Running step: {:?}", &self.step.state);
            let state_backup = serde_json::to_string(&self.step.state)?;
            match self.step.state.next() {
                Ok(state) => {
                    if let Some(state) = state {
                        self.step = Step::new(state); // TODO
                        self.save()?;
                    } else {
                        log::info!("Finished successfully");
                        self.store.clean()?;
                        break;
                    }
                }
                Err(e) => {
                    self.step.state = serde_json::from_str(&state_backup)?;
                    let err_str = format!("{:?}", e);
                    self.step.error = Some(err_str.clone());
                    self.save()?;
                    return Err(crate::Error::Step(err_str));
                }
            }
        }
        Ok(())
    }

    /// Drops the previous error
    pub fn drop_error(&mut self) {
        self.step.error = None;
    }

    fn save(&self) -> Result<()> {
        self.store.save(&self.step)?;
        Ok(())
    }
}
