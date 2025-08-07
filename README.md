# mdbook-godbolt

A preprocessor for mdbook to add runnable code snippets via Godbolt, similar to Rust
*playground* snippets.

## Usage

You can make a code snippet executable by adding the `godbolt` *attribute* to the opening
code fence. This will wrap the resulting HTML element in a `<div>` with the `godbolt`
class. There must be a language specified, both for godbolt and so your codeblock gets
syntax highlighting from the main book processor. Adding `godbolt` does not do anything
to make the code runnable as you at a minimun need to specify the compiler code. This is
done by specify the `godbolt-compiler:<code>` option.

Every attribute must be seperated by a comma with no spaces. A lookup table for compiler
codes can be found below.

### Example

```markdown
```cpp,godbolt,godbolt-compiler:g151,godbolt-flags:-std=c++17
$#include <cmath>
$#include <iostream>
$
auto magnitude(auto const x, auto const y, auto const z) -> double {
    return std::sqrt(x * x + y * y + z * z);
}

auto main() -> int {
    auto const x = 2.;
    auto const y = 3.;
    auto const z = 5.;

    std::cout << "The magnitude of the vector is "
              << magnitude(x, y, z)
              << "units.\n";
$
$    return 0;
}
```
```

## How it Works

- [ ] TODO

## Compiler Code Table

- [ ] TODO

