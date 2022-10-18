[![version-badge][]][crate-url]
[![docs-badge][]][docs-url]
[![license-badge][]][crate-url]

# step-machine

Run your CLI programs as state machines with persistence and recovery abilities. When such a
program breaks you'll have opportunity to change the external world (create a missing folder,
change a file permissions or something) and continue the program from the step it was
interrupted on.

## Usage

Let's toss two coins and make sure they both landed on the same side. We express the behaviour
as two states of our machine. Step logic is implemented in `State::next()` methods which
return the next state or `None` for the last step (the full code is in `examples/coin.rs`).
```rust

#[typetag::serde]
#[derive(Debug, Serialize, Deserialize)]
struct FirstToss;
impl FirstToss {
    fn next(self) -> StepResult {
        let first_coin = Coin::toss();
        println!("First coin: {:?}", first_coin);
        Ok(Some(SecondToss { first_coin }.into()))
    }
}

#[typetag::serde]
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
```

Then we start our machine like this:
```rust
let state = FirstToss.into();
let mut engine = Engine::<Machine>::new(init_state)?.restore()?;
engine.drop_error()?;
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

[version-badge]: https://img.shields.io/crates/v/step-machine.svg
[docs-badge]: https://docs.rs/step-machine/badge.svg
[license-badge]: https://img.shields.io/crates/l/step-machine.svg
[crate-url]: https://crates.io/crates/step-machine
[docs-url]: https://docs.rs/step-machine
