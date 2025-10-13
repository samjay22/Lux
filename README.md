```bash
# Build the project
cargo build --release

# Run a Lux program
./target/release/lux examples/hello_simple.lux

# Or use cargo run
cargo run --release examples/test_features.lux

# View tokenization output
./target/release/lux --tokens examples/fib_test.lux

# Start REPL
./target/release/lux
```
