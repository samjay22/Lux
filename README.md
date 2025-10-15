```bash
# Build the project
cargo build --release

# Run a Lux program
./target/release/lux examples/hello_simple.lux

# Or use cargo run
cargo run --release examples/test_features.lux

# View tokenization output
./target/release/lux --tokens examples/fib_test.lux
Add 
# Start REPL
./target/release/lux
```

```

## Features

- ✅ **Lua-style Syntax** - Clean, simple syntax with `local` keyword
- ✅ **Static Type System** - Go-style type declarations with type inference (`:=`)
- ✅ **Async/Await** - True parallel execution with OS threads
- ✅ **Module System** - Import/export functionality for code organization
- ✅ **Standard Library** - Written entirely in Lux (in `lib/stdlib.lux`)
- ✅ **Semantic Analysis** - Written in Lux (in `tools/semantic_analyzer.lux`)
- ✅ **Higher-Order Functions** - Functions as first-class values
- ✅ **Metatables** - Lua-style metaprogramming

## Documentation

- [QUICK_REFERENCE.md](QUICK_REFERENCE.md) - Language syntax and features
- [MODULE_SYSTEM.md](MODULE_SYSTEM.md) - Import/export system
- [STDLIB.md](STDLIB.md) - Standard library documentation
- [TRUE_PARALLEL_ASYNC.md](TRUE_PARALLEL_ASYNC.md) - Async runtime
- [SEMANTIC_ANALYZER.md](SEMANTIC_ANALYZER.md) - Semantic analysis tool

## Status

DONE: Add documentation for the language syntax and features
DONE: Add documentation for the language standard library
DONE: Add Typing Enforcement
DONE: Add more examples
DONE: Semantic Analysis (written in Lux!)
DONE: Async Runtime (true parallel execution)
DONE: Module System (import/export)
DONE: Standard Library (written in Lux!)
TODO: Add tests for the language features
TODO: VS Code Plugin

```