//! Run your CLI programs as state machines with persistence and recovery abilities. When such a
//! program breaks you'll have opportunity to change the external world (create a missing folder,
//! change a file permissions or something) and continue the program from the step it was
//! interrupted on.
//!
//! # Usage
//!
//! Let's toss two coins and make sure they both landed on the same side. We express the behaviour
//! as two states of our machine. Step logic is implemented in `State::next()` methods which
//! return the next state or `None` for the last step (the full code is in `examples/coin.rs`).
//! ```rust
//! #[derive(Debug, Serialize, Deserialize, From)]
//! enum Machine {
//!     FirstToss(FirstToss),
//!     SecondToss(SecondToss),
//! }
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct FirstToss;
//! impl FirstToss {
//!     fn next(self) -> StepResult {
//!         let first_coin = Coin::toss();
//!         println!("First coin: {:?}", first_coin);
//!         Ok(Some(SecondToss { first_coin }.into()))
//!     }
//! }
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct SecondToss {
//!     first_coin: Coin,
//! }
//! impl SecondToss {
//!     fn next(self) -> StepResult {
//!         let second_coin = Coin::toss();
//!         println!("Second coin: {:?}", second_coin);
//!         ensure!(second_coin == self.first_coin, "Coins landed differently");
//!         println!("Coins match");
//!         Ok(None)
//!     }
//! }
//! ```
//!
//! Then we start our machine like this:
//! ```rust
//! let init_state = FirstToss.into();
//! let mut engine = Engine::<Machine>::new(init_state)?.restore()?;
//! engine.drop_error()?;
//! engine.run()?;
//! ```
//! We initialize the `Engine` with the first step. Then we restore the previous state if the
//! process was interrupted (e.g. by an error). Then we drop a possible error and run all the steps
//! to completion.
//!
//! Let's run it now:
//! ```sh
//! $ cargo run --example coin
//! First coin: Heads
//! Second coin: Tails
//! Error: Coins landed differently
//! ```
//!
//! We weren't lucky this time and the program resulted in an error. Let's run it again:
//! ```sh
//! $ cargo run --example coin
//! Second coin: Heads
//! Coins match
//! ```
//!
//! Notice that, thanks to the `restore()`, our machine run from the step it was interrupted,
//! knowing about the first coin landed on heads.
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
        Self { state, error: None }
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
    pub fn drop_error(&mut self) -> Result<()> {
        self.step.error = None;
        self.save()
    }

    fn save(&self) -> Result<()> {
        self.store.save(&self.step)?;
        Ok(())
    }
}
