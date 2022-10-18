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
//!
//! #[typetag::serde]
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
//! #[typetag::serde]
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
//! let state = FirstToss.into();
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
pub use json_store::JsonStore;
use serde::{Deserialize, Serialize};
use std::{error, fmt, io};

pub mod json_store;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Store error: {0}")]
    Store(#[source] BoxedError),
    #[error("{0}")]
    Step(String),
}

/// A shourtcut for the `Step::next` result
pub type BoxedError = Box<dyn error::Error>;
pub type StepResult = Result<Option<Box<dyn Step>>, BoxedError>;
pub type BoxedStep = Box<dyn Step>;

/// A step of the machine should implement this trait
#[typetag::serde]
pub trait Step: fmt::Debug {
    /// The method is called by the engine and could optionaly return the next step
    fn run(self: Box<Self>) -> StepResult;
}

pub trait Store: fmt::Debug {
    type Error: error::Error + 'static;

    fn load(&self) -> Result<Option<State>, Self::Error>;
    fn save(&self, step: &State) -> Result<(), Self::Error>;
    fn clean(&self) -> Result<(), Self::Error>;
}

/// Machine state with metadata to store
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    /// Current state of the machine
    step: Box<dyn Step>,
    /// An error if any
    error: Option<String>,
}

#[derive(Debug)]
pub struct Engine<S: Store> {
    store: S,
    state: State,
}

impl Error {
    fn store(e: impl error::Error + 'static) -> Self {
        Self::Store(Box::new(e))
    }
}

impl State {
    fn new(step: BoxedStep, error: Option<String>) -> Self {
        Self { step, error }
    }
}

impl<S: Store> Engine<S> {
    /// Creates an Engine using initial state
    pub fn new(store: S, first_step: BoxedStep) -> Result<Self, Error> {
        let state = State::new(first_step, None);
        Ok(Self { store, state })
    }

    /// Restores an Engine from the previous run
    pub fn restore(mut self) -> Result<Self, Error> {
        if let Some(state) = self.store.load().map_err(Error::store)? {
            self.state = state;
        }
        Ok(self)
    }

    /// Drops the previous error
    pub fn drop_error(mut self) -> Result<Self, Error> {
        self.state.error = None;
        self.save()?;
        Ok(self)
    }

    /// Runs all steps to completion
    pub fn run(mut self) -> Result<(), Error> {
        if let Some(e) = self.state.error.as_ref() {
            return Err(crate::Error::Step(format!(
                "Previous run resulted in an error: {} on step: {:?}",
                e, self.state.step
            )));
        }

        loop {
            log::info!("Running step: {:?}", &self.state.step);
            let step_backup = serde_json::to_string(&self.state.step)?;
            match self.state.step.run() {
                Ok(Some(step)) => {
                    self.state.step = step;
                    self.save()?;
                }
                Ok(None) => {
                    log::info!("Finished successfully");
                    self.store.clean().map_err(Error::store)?;
                    break;
                }
                Err(e) => {
                    self.state.step = serde_json::from_str(&step_backup)?;
                    let err_str = error_chain(e);
                    self.state.error = Some(err_str.clone());
                    self.save()?;
                    return Err(Error::Step(err_str));
                }
            };
        }
        Ok(())
    }

    fn save(&self) -> Result<(), Error> {
        self.store.save(&self.state).map_err(Error::store)?;
        Ok(())
    }
}

/// A helper to format error with its source chain
pub fn error_chain(e: BoxedError) -> String {
    let mut s = e.to_string();
    let mut current = e.as_ref().source();
    if current.is_some() {
        s.push_str("\nCaused by:");
    }
    while let Some(cause) = current {
        s.push_str(&format!("\n\t{}", cause));
        current = cause.source();
    }
    s
}
