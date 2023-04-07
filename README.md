# Arlekin client

## Dependencies
- Rust 1.68.0

## Development
### Installation
To compile and run the browser version you need to have `Rust` tools installed (https://www.rust-lang.org/tools/install).

Then you need to install WebAssembly using the following command:
```
rustup target add wasm32-unknown-unknown
```
And then, you need to install Trunk using the following command:
```
cargo install --locked trunk
```
### Compiling and running
Next in the downloaded repository, go to the `frontend` folder and run the following command:
```
trunk serve
```
And then, without finishing the first command call in the folder `backend` command:
```
cargo run
```
Once everything compiles, you should see the Arlekin client at: http://localhost:8080/.

> The `trunk serve` command will automatically compile the page when it detects changes in the `frontent` folder.<br>
Warning: Remember to reload cache your browser (CTRL+F5).

## Contributtion
The git workflow strictly applies. [Read it here.](docs/GitWorkflow.md)

In addition, do not forget to format the Rust code and detect errors and fix them with the compiler and clippy before you post a pull request.

Formatting ([installation guide](https://github.com/rust-lang/rustfmt#on-the-stable-toolchain)):
```
cargo fmt
```

Clippy ([installation guide](https://github.com/rust-lang/rust-clippy#step-2-install-clippy)):
```
cargo clippy --target wasm32-unknown-unknown
```
