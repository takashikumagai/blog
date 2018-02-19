## Coding for Cross-Platform: Part 1 - Concatenating Pathnames

*Feb. 19, 2018*

Windows and the rest of the platforms use different separators for pathnames,
specfically, Windows uses '/' (backslash) whereas the others use '/' (forward slash)
To support both types of platforms, each language provides its own way of concatenating paths

| Language | Good | Bad |
| ------------- | ------------- | ------------- |
| C++ | filesystem::path a, b; auto c = a / b; | std::sring a, b, c; c = a + '/' + b;|
| Java | Path a, b, c; c = Paths.get(a,b); | String a, b, c; c = a + "/" + b; |
| Python | a = 'mydir'<br>b = 'file.txt'<br>c = os.path.join(a,b) | a = 'mydir'<br>b = 'file.txt'<br>c = a + '/' + b |
