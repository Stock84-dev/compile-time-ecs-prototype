## Legacy
This is the code from the previous version of the bactester.
It code uses nightly but that will slowly change into stable.

## Engine
All code related to backtesting of trading strategies

## Inception
High performance ECS library. Designed for writing trading strategies with high abstraction without any cost.

## Esl - Engine standard library
Crate that is used for writing custom trading strategies. It is designed to be used on the GPU and is #![no_std] compatible.

## Ergnomics
Utility crate.

# Workspace
Working directory for researchers.

## Conventions
Similar crates are inside the same folder and their name starts with the folder name. Each crate shouldn't have `lib.rs` or `main.rs`. Use the crate name for the file and change it in `Cargo.toml`. That way the files can be searched faster. Same thing for `mod.rs`, use directory name and place it one step above.
