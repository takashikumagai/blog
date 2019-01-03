# Batch Renaming Numbered Files

Whether you are a programmer or not, the need for [batch renaming](https://en.wikipedia.org/wiki/Batch_renaming) follows you around wherever you go, as long as you manage files on your computer. In particular, renaming numbered files, such as offsetting numbers and padding them with zeros, is very common. Easy enough to do in Python/C++, etc. but usually we want to do these tasks directly from terminal without creating a file let alone compiling some code. Though the syntax can get a bit tricky, bash can accomplish certain patterns of renaming tasks if they are not too complicated. Here is the quick cheat sheet that I save for myself and, as you can see, decided to upload publicly.




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
evaluates/executes the command written in the parenthesis.

Next, the printf command in bash formats the string and prints the string to stdout. In short, the syntax is

<code>printf *(formatting string)* *(things you want to format, e.g. numbers)*</code>

So by writing it like
```
$(printf %03d $i)
```
we can store the formatted string (in this case a zero-padded number) to a variable. Note that although ``printf`` is well known as a C function, what we are using here is a linux command that resides in ``/usr/bin/``, just like ``ls`` or ``ping``. 

Finally, the expr command performs simple arithmetic operations and prints the result
```
$(expr $1 - 8)
```




