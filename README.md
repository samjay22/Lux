# Lux Programming Language

A custom programming language with Lua-like syntax, Go-like static typing, and built-in async/await support.

## 🚀 Quick Start

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

**Try it now:**
```lux
// hello.lux
local greeting: string = "Hello, Lux!"
print(greeting)

fn fibonacci(n: int) -> int {
    if n < 2 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

local result := fibonacci(10)
print(result)  // Prints: 55
```

## 🎯 Design Goals

- **Simple, Clean Syntax**: Inspired by Lua's minimalist approach
- **Static Typing**: Go-style explicit type declarations with type inference
- **First-Class Concurrency**: Built-in async/await and goroutine-style task spawning
- **Modular Architecture**: Clean separation of concerns for easy extension

## 🚀 Features

### Current Implementation (v0.1.0)

**🎉 Lux is now executable! You can run programs!**

- ✅ **Lexer/Tokenizer**: Complete lexical analysis with comprehensive token support
- ✅ **Parser**: Full recursive descent parser with AST generation
- ✅ **Interpreter**: Tree-walking interpreter with runtime execution
- ✅ **Error Handling**: Rich error messages with source location tracking
- ✅ **Comments**: Single-line (`//`) and multi-line (`/* */`) with nesting support
- ✅ **Literals**: Integers, floats, strings with escape sequences
- ✅ **Keywords**: Full keyword set for control flow, types, and async operations
- ✅ **Functions**: Function declarations, calls, and recursion
- ✅ **Control Flow**: if/else, while, for loops, break, continue, return
- ✅ **Tables**: Lua-style tables with fields and arrays
- ✅ **Operators**: Arithmetic, comparison, logical, and length (#) operators
- ✅ **Built-ins**: print, setmetatable, getmetatable

### Planned Features

- 🔄 **Type System**: Static type checking with inference (Phase 4)
- 🔄 **Semantic Analysis**: Variable resolution and scope checking (Phase 5)
- 🔄 **Async Runtime**: Task spawning and execution (Phase 7)
- 🔄 **Metatable Dispatch**: Full metamethod support for operator overloading

## 📖 Language Syntax

### Variables (Lua-style)

```lux
// Explicit type declaration with 'local' keyword
local x: int = 42
local name: string = "Alice"
local pi: float = 3.14159

// Type inference with :=
local y := 100        // inferred as int
local greeting := "Hello"  // inferred as string

// Constants
const MAX_SIZE: int = 1000
```

### Functions

```lux
// Function with explicit types
fn add(a: int, b: int) -> int {
    return a + b
}

// Function without return value
fn greet(name: string) {
    print("Hello, " + name)
}

// Function with type inference for locals
fn calculate(x: int) -> int {
    local result := x * 2  // type inferred
    return result
}
```

### Control Flow

```lux
// If-else statements
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// While loops
while count < 10 {
    count = count + 1
}

// For loops
for i := 0; i < 10; i = i + 1 {
    print(i)
}
```

### Async/Concurrency (Planned)

```lux
// Async function declaration
async fn fetchData(url: string) -> string {
    // async operations
    return data
}

// Spawning tasks (like Go's 'go' keyword)
fn main() {
    spawn fetchData("https://example.com")

    // Await for result
    local result = await fetchData("https://api.example.com")
    print(result)
}
```

### Types

**Primitive Types:**
- `int` - 64-bit signed integer
- `float` - 64-bit floating point
- `string` - UTF-8 string
- `bool` - Boolean (true/false)
- `nil` - Null/void type
- `table` - Lua-style tables (associative arrays)

**Compound Types (Planned):**
- `[int]` - Arrays (homogeneous)
- `{string: int}` - Typed tables
- `fn(int, int) -> int` - Function types

### Metatables (Lua-style)

Lux supports Lua-style metatables for powerful metaprogramming:

```lux
// Create a table with custom behavior
local vec: table = {x = 10, y = 20}

// Define a metatable with metamethods
local meta: table = {
    __add = fn(a: table, b: table) -> table {
        return {x = a.x + b.x, y = a.y + b.y}
    },

    __tostring = fn(v: table) -> string {
        return "Vector(" + v.x + ", " + v.y + ")"
    }
}

// Set the metatable
setmetatable(vec, meta)

// Now the table has custom behavior
local result := vec + vec  // Uses __add metamethod
print(result)              // Uses __tostring metamethod
```

**Supported Metamethods:**
- `__index` - Custom property access
- `__newindex` - Custom property assignment
- `__add`, `__sub`, `__mul`, `__div` - Arithmetic operators
- `__eq`, `__lt`, `__le` - Comparison operators
- `__call` - Make tables callable
- `__tostring` - String representation
- `__len` - Length operator (`#`)
- `__concat` - Concatenation operator

**Built-in Functions:**
- `setmetatable(table, metatable)` - Set a table's metatable
- `getmetatable(table)` - Get a table's metatable

## 🛠️ Installation & Usage

### Prerequisites

- Rust 1.70 or higher
- Cargo (comes with Rust)

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd custom-language

# Build the project
cargo build --release

# Run tests
cargo test

# Install the binary
cargo install --path .
```

### Running Lux Programs

```bash
# Run a Lux script file
lux script.lux

# Start the REPL (interactive mode)
lux

# In REPL
lux:1 > let x := 42
lux:2 > print(x)
lux:3 > exit
```

## 📁 Project Structure

```
lux-lang/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library root
│   ├── error/               # Error handling & diagnostics
│   │   ├── mod.rs
│   │   └── diagnostic.rs
│   ├── lexer/               # Lexical analysis
│   │   ├── mod.rs
│   │   ├── token.rs         # Token definitions
│   │   └── scanner.rs       # Lexer implementation
│   ├── parser/              # Parsing (in progress)
│   │   ├── mod.rs
│   │   ├── ast.rs           # AST definitions
│   │   └── parser.rs        # Parser implementation
│   ├── types/               # Type system (planned)
│   ├── semantic/            # Semantic analysis (planned)
│   ├── runtime/             # Interpreter (planned)
│   └── async_runtime/       # Async executor (planned)
├── tests/                   # Integration tests
├── examples/                # Example Lux programs
├── Cargo.toml
└── README.md
```

## 🧪 Testing

The project includes comprehensive unit tests for all implemented components:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test lexer::

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## 🏗️ Architecture

### Compilation Pipeline

```
Source Code
    ↓
┌─────────────┐
│   Lexer     │ → Tokens
└─────────────┘
    ↓
┌─────────────┐
│   Parser    │ → AST
└─────────────┘
    ↓
┌─────────────┐
│Type Checker │ → Typed AST
└─────────────┘
    ↓
┌─────────────┐
│  Semantic   │ → Validated AST
│  Analyzer   │
└─────────────┘
    ↓
┌─────────────┐
│ Interpreter │ → Execution
│ / Compiler  │
└─────────────┘
```

### Design Principles

1. **Modularity**: Each phase is independent and can be tested separately
2. **Error Recovery**: Parser can recover from errors to report multiple issues
3. **Rich Diagnostics**: Helpful error messages with source context
4. **Extensibility**: Easy to add new language features
5. **Performance**: Efficient implementation using Rust

## 🤝 Contributing

Contributions are welcome! Areas for contribution:

- Parser implementation
- Type system and inference
- Standard library functions
- Optimization passes
- Documentation and examples
- Bug fixes and tests

## 📝 License

MIT License - See LICENSE file for details

## 🗺️ Roadmap

### Phase 1: Foundation ✅
- [x] Project setup
- [x] Error handling infrastructure
- [x] Module structure

### Phase 2: Lexer ✅
- [x] Token definitions
- [x] Lexer implementation
- [x] Comprehensive tests

### Phase 3: Parser (In Progress)
- [ ] AST node definitions
- [ ] Expression parsing
- [ ] Statement parsing
- [ ] Parser tests

### Phase 4: Type System
- [ ] Type representations
- [ ] Type checker
- [ ] Type inference
- [ ] Type tests

### Phase 5: Semantic Analysis
- [ ] Symbol table
- [ ] Scope checking
- [ ] Variable resolution
- [ ] Semantic tests

### Phase 6: Runtime
- [ ] Runtime values
- [ ] Tree-walking interpreter
- [ ] Built-in functions
- [ ] Runtime tests

### Phase 7: Async Runtime
- [ ] Task spawning
- [ ] Async executor
- [ ] Await mechanism
- [ ] Concurrency tests

### Future Enhancements
- [ ] Bytecode compiler
- [ ] Virtual machine
- [ ] JIT compilation
- [ ] Standard library
- [ ] Package manager
- [ ] Language server (LSP)
- [ ] Debugger

## 📚 Resources

- [Language Design Document](docs/design.md) (coming soon)
- [API Documentation](https://docs.rs/lux-lang) (coming soon)
- [Tutorial](docs/tutorial.md) (coming soon)
- [Examples](examples/) (coming soon)

## 🙏 Acknowledgments

Inspired by:
- **Lua**: Simple, elegant syntax
- **Go**: Static typing and concurrency model
- **Rust**: Safety and performance
- **Crafting Interpreters** by Robert Nystrom

---

**Status**: 🚧 Active Development - Phase 2 Complete (Lexer)

**Version**: 0.1.0

**Author**: Samuel Taylor

