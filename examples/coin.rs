use anyhow::{ensure, Result};
use derive_more::From;
use rand::Rng;
use serde::{Deserialize, Serialize};

use step_machine::{Engine, State};

type StepResult = Result<Option<Machine>>;

fn main() -> Result<()> {
    env_logger::init();
    let init_state = FirstToss.into();
    let mut engine = Engine::<Machine>::new(init_state)?.restore()?;
    engine.drop_error();
    engine.run()?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Coin {
    Heads,
    Tails,
}

impl Coin {
    fn toss() -> Self {
        if rand::thread_rng().gen::<bool>() {
            Coin::Heads
        } else {
            Coin::Tails
        }
    }
}

#[derive(Debug, Serialize, Deserialize, From)]
enum Machine {
    FirstToss(FirstToss),
    SecondToss(SecondToss),
}

impl State<Machine> for Machine {
    type Error = anyhow::Error;

    fn next(self) -> StepResult {
        match self {
            Machine::FirstToss(state) => state.next(),
            Machine::SecondToss(state) => state.next(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FirstToss;
impl FirstToss {
    fn next(self) -> StepResult {
        let first_coin = Coin::toss();
        println!("First coin: {:?}", first_coin);
        Ok(Some(SecondToss { first_coin }.into()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SecondToss {
    first_coin: Coin,
}
impl SecondToss {
    fn next(self) -> StepResult {
        let second_coin = Coin::toss();
        println!("Second coin: {:?}", second_coin);
        ensure!(second_coin == self.first_coin, "Coins landed differently");
        println!("Coins match");
        Ok(None)
    }
}
