# Renaming numbered files

Renaming numbered files, such as offsetting numbers and padding them with zeros, is one of the things that comes up often when you manage files on your computer. Easy enough to do in Python/C++, etc. but usually we want to do these tasks directly from terminal without creating a file let alone compiling. Though the syntax can get fairly tricky, bash can accomplish certain patterns of renaming tasks if they are not too complicated. Here is the quick cheat sheet that I save for myself and, as you can see, decided to upload publicly.




### Scenario 1: Simple zero padding
```
$ for((i=9; i<=11; i++)); do mv from$i.jpg to$(printf %03d $i).jpg; done
```

Tip: replace ``mv`` with ``echo`` to see the preview first, then run with ``mv``

<code>$ for((i=9; i<=11; i++)); do **echo** from$i.jpg to$(printf %03d $i).jpg; done</code>

#### Example

- Input file names: from9.jpg, from10.jpg, from11.jpg
  - \> Output file names will be: to009.jpg to010.jpg to011.jpg

- Preview using ``echo``
```
from9.jpg to009.jpg
from10.jpg to010.jpg
from11.jpg to011.jpg
```

### Scenario 2: zero padding + shifting numbers
```
$ for((i=9; i<=11; i++)); do mv from$i.jpg to$(printf %03d $(expr $i - 8)).jpg; done
```

#### Example
- Input file names: from9.jpg, from10.jpg, from11.jpg
  - \> Output file names will be: to001.jpg to002.jpg to003.jpg

### How It Works

To break it down and explain the ingredients of the scripts above,
```
$()
```
evaluates/executes the command written in the parenthesis

```
$(printf %03d $i)
```
The printf command in bash formats the string and prints the string

```
$(expr $1 - 8)
```
The expr command performs simple arithmetic operations and prints the result




