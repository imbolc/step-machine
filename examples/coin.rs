use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io;

use step_machine::{Engine, Error, JsonStore, Step, StepResult};

fn main() -> Result<(), Error> {
    env_logger::init();
    let store = JsonStore::new().map_err(|e| Error::Store(Box::new(e)))?;
    let engine = Engine::new(store, Box::new(FirstToss))?.restore()?;
    engine.drop_error()?.run()?;
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

#[derive(Debug, Serialize, Deserialize)]
struct FirstToss;

#[typetag::serde]
impl Step for FirstToss {
    fn run(self: Box<Self>) -> StepResult {
        let first_coin = Coin::toss();
        println!("First coin: {:?}", first_coin);
        Ok(Some(Box::new(SecondToss { first_coin })))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SecondToss {
    first_coin: Coin,
}

#[typetag::serde]
impl Step for SecondToss {
    fn run(self: Box<Self>) -> StepResult {
        let second_coin = Coin::toss();
        println!("Second coin: {:?}", second_coin);
        if second_coin == self.first_coin {
            println!("Coins match");
            Ok(None)
        } else {
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Coins landed differently",
            )))
        }
    }
}
