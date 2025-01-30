# Chapter 1

Some text

```cpp,godbolt
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

More text

```cpp,
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

```cpp
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

```,
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

```,godbolt
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

