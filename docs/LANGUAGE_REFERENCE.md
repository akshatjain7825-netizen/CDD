# Source Language Reference Manual (`wasmc`)

## 1. Lexical Grammar

### 1.1 Source Encoding
Source files are encoded in UTF-8 without byte-order marks.

### 1.2 Comments and Whitespace
- Line comments begin with `//` and continue to the end of the line.
- Block comments begin with `/*` and end with `*/`. Nested block comments are supported.
- Whitespace consists of spaces (`\x20`), tabs (`\t`), newlines (`\n`), and carriage returns (`\r`).

### 1.3 Identifiers
Identifiers start with an ASCII letter or underscore, followed by any number of letters, digits, or underscores:
```ebnf
Identifier = [a-zA-Z_] { [a-zA-Z0-9_] } ;
```

### 1.4 Keywords
```
fn, let, mut, if, else, while, for, return, tail,
break, continue, try, catch, throw,
import, export, struct, global,
void, bool, char, i32, i64, u32, u64, f32, f64,
true, false
```

### 1.5 Literals
- **Integer Literals:** Decimal (`1234`), hexadecimal (`0x1F`), binary (`0b1010`), octal (`0o77`). Optional type suffixes (`i32`, `i64`, `u32`, `u64`).
- **Float Literals:** IEEE-754 decimal floats (`3.14159`, `1e-4`, `2.5f32`, `2.5f64`).
- **Boolean Literals:** `true`, `false`.
- **Character Literals:** `'a'`, `'\n'`, `'\t'`, `'\\'`, `'\''`.
- **String Literals:** `"Hello, world!\n"`. Supported escapes: `\n`, `\r`, `\t`, `\\`, `\"`, `\0`, `\xHH`.

---

## 2. Type System

### 2.1 Primitive Value Types
| Type | Representation | Wasm Storage |
|---|---|---|
| `void` | Unit type (no value) | None |
| `bool` | 1-bit boolean | `i32` (0 or 1) |
| `char` | 8-bit ASCII character | `i32` |
| `i32` | 32-bit two's complement integer | `i32` |
| `u32` | 32-bit unsigned integer | `i32` |
| `i64` | 64-bit two's complement integer | `i64` |
| `u64` | 64-bit unsigned integer | `i64` |
| `f32` | 32-bit IEEE 754 float | `f32` |
| `f64` | 64-bit IEEE 754 float | `f64` |

### 2.2 Composite Types
- **Pointer Types:** `*T` represents a 32-bit linear memory address pointing to type `T`.
- **Array Types:** `[T; N]` represents a contiguous sequence of `N` elements of type `T`.
- **Struct Types:** Named aggregate records with typed fields:
  ```src
  struct Point {
      x: i32,
      y: i32,
  }
  ```
- **Function Pointer Types:** `fn(P1, P2) -> R` represents an indirect function reference into the WebAssembly table.

---

## 3. Declarations

### 3.1 Function Declarations
```src
[export] fn name(param1: Type1, param2: Type2) -> ReturnType {
    // statements
}
```
If `ReturnType` is omitted, it defaults to `void`.

### 3.2 Import Declarations
```src
import "module_name" "field_name" fn imported_fn(param1: Type1) -> ReturnType;
```

### 3.3 Global Declarations
```src
global [mut] name: Type = constant_expression;
```

### 3.4 Struct Declarations
```src
struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}
```

---

## 4. Statements

- **Variable Binding:**
  ```src
  let x: i32 = 10;
  let mut counter: i32 = 0;
  ```
- **Assignment:**
  ```src
  counter = counter + 1;
  counter += 5;
  ```
- **Control Flow:**
  - `if (condition) { ... } else { ... }`
  - `while (condition) { ... }`
  - `for (let mut i: i32 = 0; i < 10; i = i + 1) { ... }`
  - `break;` and `continue;`
  - `return [expression];`
  - `tail fn_call(...);` (Wasm Core 2.0 tail call `return_call`)
- **Exception Handling:**
  ```src
  try {
      throw 404;
  } catch (err) {
      // handle error
  }
  ```

---

## 5. Memory Semantics

- **Address-of Operator (`&`):** Applying `&` to a local variable triggers escape analysis in `wasmc-semantics`, causing the variable to be assigned `StorageClass::ShadowStack`. Its storage is allocated in linear memory inside the function's downward-growing stack frame.
- **Pointer Dereference (`*`):** Reading or writing to `*ptr` translates to aligned linear memory load/store instructions (`i32.load`, `i64.store`, `f64.load`, etc.).
