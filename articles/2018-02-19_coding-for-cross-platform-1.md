## Coding for Cross-Platform: Part 1 - Concatenating Pathnames

*Feb. 19, 2018*

Windows and the rest of the platforms use different separators for pathnames,
specfically, Windows uses '/' (backslash) whereas the others use '/' (forward slash)
To support both types of platforms, each language provides its own way of concatenating paths

### C++

```cpp
#include <experimental/filesystem>

void f() {
    std::experimental::filesystem::path a = "mydir", b = "file.txt";
    auto concatenated = a / b;
}
```


### Java

```java
import java.nio.file.Path;
import java.nio.file.Paths;

class C {
    public void f() {
        Path a = Paths.get("mydir")
        Path b = Paths.get("file.txt)
        Path concatenated = Paths.get(a,b);
    }
}

```


### Python

```python
import os.path

a = 'mydir'
b = 'file.txt'
concatenated = os.path.join(a,b)
```
