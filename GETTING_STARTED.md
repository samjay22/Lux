# Getting Started with Lux

## Current Implementation Status

🎉 **Lux is now a fully functional programming language!**

✅ **Phase 1: Project Setup & Error Handling** - Complete
✅ **Phase 2: Lexer (Tokenization)** - Complete
✅ **Phase 3: Parser (AST Generation)** - Complete
✅ **Phase 6: Interpreter (Runtime Execution)** - Complete
⏳ **Phase 4: Type System** - Not yet implemented
⏳ **Phase 5: Semantic Analysis** - Not yet implemented
⏳ **Phase 7: Async Runtime** - Not yet implemented

## What You Can Do Right Now

### 1. Build the Project

```bash
cargo build --release
```

The binary will be at `target/release/lux`

### 2. Run Your First Program

```bash
# Run the showcase example
./target/release/lux examples/showcase.lux

# Run a simple hello world
./target/release/lux examples/hello_simple.lux

# Run the fibonacci example
./target/release/lux examples/fib_test.lux
```

### 3. View Lexer Output (Tokenization)

You can see how the lexer tokenizes Lux code:

```bash
# Show tokens from a file
./target/release/lux --tokens examples/hello.lux

# Or use the short flag
./target/release/lux -t examples/fibonacci.lux
```

**Example output:**
```
Tokens for 'examples/hello.lux':
============================================================
   0: Keyword(Fn)        | "fn"
   1: Identifier         | "main"
   2: LeftParen          | "("
   3: RightParen         | ")"
   4: LeftBrace          | "{"
   5: Keyword(Local)     | "local"
   6: Identifier         | "greeting"
   7: Colon              | ":"
   8: Keyword(String)    | "string"
   9: Assign             | "="
  10: Literal(String("Hello, Lux!")) | "\"Hello, Lux!\""
  11: Identifier         | "print"
  12: LeftParen          | "("
  13: Identifier         | "greeting"
  14: RightParen         | ")"
  15: RightBrace         | "}"
  16: Eof                | ""
============================================================
Total tokens: 17
```

### 3. Test the Lexer with Different Examples

Try tokenizing different example files:

```bash
# Basic hello world
./target/release/lux -t examples/hello.lux

# Fibonacci with loops
./target/release/lux -t examples/fibonacci.lux

# Type system demo
./target/release/lux -t examples/types_demo.lux

# Metatable features (Lua-style)
./target/release/lux -t examples/metatables.lux

# Async example (planned feature)
./target/release/lux -t examples/async_example.lux
```

### 4. Write Your Own Lux Code

Create a new `.lux` file and test the lexer:

```lux
// test.lux
local x: int = 42
local name := "Alice"

fn greet(person: string) {
    print("Hello, " + person)
}
```

Then tokenize it:
```bash
./target/release/lux -t test.lux
```

### 5. Interactive REPL (Limited)

You can start the REPL, but it only tokenizes input:

```bash
./target/release/lux
```

### 6. Run Tests

See all the lexer tests pass:

```bash
cargo test
```

### 7. View Help

```bash
./target/release/lux --help
```

## Language Features You Can Test (Lexer Level)

The lexer recognizes all these language features:

### Keywords
- **Variables**: `local`, `const`
- **Functions**: `fn`, `return`
- **Control Flow**: `if`, `else`, `while`, `for`, `break`, `continue`
- **Types**: `int`, `float`, `string`, `bool`, `nil`, `table`
- **Booleans**: `true`, `false`
- **Metatables**: `setmetatable`, `getmetatable`
- **Async**: `async`, `await`, `spawn`
- **Logical**: `and`, `or`, `not`

### Operators
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Assignment**: `=`, `:=`
- **Length**: `#` (Lua-style)
- **Arrow**: `->` (function return type)

### Literals
- **Integers**: `42`, `0`, `123456`
- **Floats**: `3.14`, `0.5`, `123.456`
- **Strings**: `"hello"`, `"world"` (with escape sequences: `\n`, `\t`, `\"`, etc.)

### Comments
- **Single-line**: `// comment`
- **Multi-line**: `/* comment */` (with nesting support)

### Special Identifiers
- **Metamethods**: `__index`, `__add`, `__call`, etc. (recognized as identifiers)

## What's Next?

To make the language actually executable, we need to implement:

1. **Parser** - Convert tokens into an Abstract Syntax Tree (AST)
2. **Type Checker** - Validate types and perform type inference
3. **Semantic Analyzer** - Check variable scoping, control flow, etc.
4. **Interpreter** - Execute the validated AST
5. **Async Runtime** - Handle concurrent task execution

## Contributing

Want to help implement the next phase? Check out the task list in the project!

## Examples to Explore

All examples are in the `examples/` directory:

- `hello.lux` - Simple hello world
- `fibonacci.lux` - Fibonacci with loops and type inference
- `types_demo.lux` - Type system demonstration
- `metatables.lux` - Lua-style metatables (operator overloading, inheritance)
- `async_example.lux` - Async/await syntax (planned feature)

## Questions?

Check the main [README.md](README.md) for full language documentation.

