# Lux Language Quick Reference

## Variables

```lux
// With type annotation
local x: int = 42
local name: string = "Alice"
local pi: float = 3.14159
local isReady: bool = true

// With type inference
local y := 100
local greeting := "Hello"
```

## Functions

```lux
// Function declaration
fn add(a: int, b: int) -> int {
    return a + b
}

// Async function (parsed but executed synchronously)
async fn compute(n: int) -> int {
    return n * 2
}

// Function with no return type
fn greet(name: string) {
    print("Hello, ")
    print(name)
}

// Recursive function
fn factorial(n: int) -> int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
```

## Function Expressions (Anonymous Functions)

```lux
// In a table
local ops: table = {
    add = fn(a: int, b: int) -> int {
        return a + b
    },
    multiply = fn(a: int, b: int) -> int {
        return a * b
    }
}

// Call it
local result := ops.add(5, 3)  // 8
```

## Control Flow

```lux
// If/else
if x > 10 {
    print("Large")
} else if x > 5 {
    print("Medium")
} else {
    print("Small")
}

// While loop
local i := 0
while i < 5 {
    print(i)
    i = i + 1
}

// For loop (C-style)
for local j := 0; j < 10; j = j + 1 {
    print(j)
}

// Break and continue
while true {
    if condition {
        break
    }
    if otherCondition {
        continue
    }
}
```

## Tables

```lux
// Table with named fields
local person: table = {
    name = "Alice",
    age = 30,
    city = "Wonderland"
}

// Access fields
print(person.name)
print(person.age)

// Array-style table
local numbers: table = {1, 2, 3, 4, 5}

// Length operator
local len := #numbers  // 5

// Mixed table
local mixed: table = {
    name = "Bob",
    1,
    2,
    3
}
```

## Metatables

```lux
// Create a table
local vec: table = {x = 10, y = 20}

// Create a metatable with metamethods
local meta: table = {
    __add = fn(a: table, b: table) -> table {
        return {x = a.x + b.x, y = a.y + b.y}
    },
    __tostring = fn(v: table) -> string {
        return "Vector"
    }
}

// Set the metatable
setmetatable(vec, meta)

// Get the metatable
local m := getmetatable(vec)
```

## Async/Await

```lux
// Define a function to spawn
fn worker(id: int, value: int) -> int {
    print("Worker starting")
    return value * 2
}

// Spawn a task (returns task ID)
local task := spawn worker(1, 10)

// Await the result
local result := await task  // 20

// Multiple tasks in parallel
local t1 := spawn worker(1, 5)
local t2 := spawn worker(2, 10)
local t3 := spawn worker(3, 15)

local r1 := await t1  // 10
local r2 := await t2  // 20
local r3 := await t3  // 30

// Nested spawning
fn compositeTask(id: int) -> int {
    local sub1 := spawn worker(id * 10, id)
    local sub2 := spawn worker(id * 20, id * 2)
    
    local r1 := await sub1
    local r2 := await sub2
    
    return r1 + r2
}

local task := spawn compositeTask(1)
local result := await task
```

## Operators

### Arithmetic
```lux
local a := 10 + 5   // Addition: 15
local b := 10 - 5   // Subtraction: 5
local c := 10 * 5   // Multiplication: 50
local d := 10 / 5   // Division: 2
local e := 10 % 3   // Modulo: 1
```

### Comparison
```lux
local eq := 5 == 5   // Equal: true
local ne := 5 != 3   // Not equal: true
local lt := 5 < 10   // Less than: true
local le := 5 <= 5   // Less or equal: true
local gt := 10 > 5   // Greater than: true
local ge := 10 >= 10 // Greater or equal: true
```

### Logical
```lux
local and := true and false  // Logical AND: false
local or := true or false    // Logical OR: true
local not := not true        // Logical NOT: false
```

### String Concatenation
```lux
local greeting := "Hello" + " " + "World"  // "Hello World"
```

### Length
```lux
local len := #"Hello"      // 5
local len := #{1, 2, 3}    // 3
```

## Built-in Functions

```lux
// Print to console
print("Hello, World!")
print(42)

// Set metatable
setmetatable(table, metatable)

// Get metatable
local meta := getmetatable(table)
```

## Types

```lux
int      // Integer
float    // Floating-point number
string   // String
bool     // Boolean (true/false)
nil      // Nil value
table    // Table (associative array)
```

## Comments

```lux
// Single-line comment

/*
 * Multi-line comment
 * Can be nested!
 */

/* Nested /* comments */ work! */
```

## Example Program

```lux
// Fibonacci with async/await
fn fib(n: int) -> int {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn fibWorker(n: int) -> int {
    print("Computing fib(")
    print(n)
    print(")")
    return fib(n)
}

// Spawn parallel tasks
local t1 := spawn fibWorker(10)
local t2 := spawn fibWorker(12)

// Await results
local r1 := await t1  // 55
local r2 := await t2  // 144

print("Results: ")
print(r1)
print(", ")
print(r2)
```

## Running Programs

```bash
# Run a program
./target/release/lux program.lux

# View tokens
./target/release/lux --tokens program.lux

# Build
cargo build --release

# Test
cargo test
```

## Tips

1. **Type Inference**: Use `:=` for automatic type inference
2. **Recursion**: Fully supported (tested with Fibonacci)
3. **Tables**: Can mix named fields and array elements
4. **Async**: Tasks execute immediately but provide async semantics
5. **Functions**: First-class values, can be stored in tables
6. **Scope**: Variables are scoped to their block
7. **Return**: Functions return `nil` if no explicit return

## Common Patterns

### Map-like operations
```lux
local data: table = {
    process = fn(x: int) -> int { return x * 2 },
    validate = fn(x: int) -> bool { return x > 0 }
}

local result := data.process(5)  // 10
```

### Parallel computation
```lux
local tasks: table = {}
local i := 0
while i < 10 {
    tasks[i] = spawn compute(i)
    i = i + 1
}

// Await all
i = 0
while i < 10 {
    local result := await tasks[i]
    print(result)
    i = i + 1
}
```

### Factory pattern
```lux
fn makeCounter() -> table {
    return {
        count = 0,
        increment = fn(self: table) -> int {
            self.count = self.count + 1
            return self.count
        }
    }
}

local counter := makeCounter()
```

