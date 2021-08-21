# step-machine

Run your CLI program as state machines with persistence and recovery abilities. When such a
program breaks you'll have opportunity to change the external world (create a missing folder,
change a file permissions or something) and continue the program from the step it was
interrupted on.

## Usage

Let's toss two coins and make sure they both landed on the same side. We express the behaviour
as two states of our machine. Step logic is implemented in `State::next()` methods which
return the next state or `None` for the last step (the full code is in `examples/coin.rs`).
```rust
#[derive(Debug, Serialize, Deserialize, From)]
enum Machine {
    FirstToss(FirstToss),
    SecondToss(SecondToss),
}

#[derive(Debug, Serialize, Deserialize)]
struct FirstToss;
impl State<Machine> for FirstToss {
    type Error = anyhow::Error;

    fn next(self) -> StepResult {
        let coin = Coin::toss();
        println!("First coin: {:?}", coin);
        Ok(Some(SecondToss { first_coin: coin }.into()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SecondToss {
    first_coin: Coin,
}
impl State<Machine> for SecondToss {
    type Error = anyhow::Error;

    fn next(self) -> StepResult {
        let second_coin = Coin::toss();
        println!("Second coin: {:?}", second_coin);
        ensure!(second_coin == self.first_coin, "Coins landed differently");
        println!("Coins match");
        Ok(None)
    }
}
```

Then we start our machine like this:
```rust
let init_state = Machine::FirstToss(FirstToss);
let mut engine = Engine::<Machine>::new(init_state)?.restore()?;
engine.drop_error();
engine.run()?;
```
We initialize the `Engine` with the first step. Then we restore the previous state if the
process was interrupted (e.g. by an error). Then we drop a possible error and run all the steps
to completion.

Let's run it now:
```sh
$ cargo run --example coin
First coin: Heads
Second coin: Tails
Error: Coins landed differently
```

We weren't lucky this time and the program resulted in an error. Let's run it again:
```sh
$ cargo run --example coin
Second coin: Heads
Coins match
```

Notice that, thanks to the `restore()`, our machine run from the step it was interrupted,
knowing about the first coin landed on heads.

License: MIT OR Apache-2.0
