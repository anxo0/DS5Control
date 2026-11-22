# Map of the move to Rust

`estado.py` draws `estado.svg`, a map of squares showing how much of the C# code (`DS5Control/`) has its Rust version. Since 0.3.0 the program runs entirely on Rust, so the map is complete.

- At the top, badges: the percentage moved and the `dato` lines of `mudanza.txt`.
- Below, one block per part (mapping engine, profiles and settings, physical controller, virtual controller and HidHide, the app). Inside, one rectangle per `.cs` file, sized by its lines and filled with small squares: green when moved, grey when pending. Hovering shows the path, the lines and the percentage.
- At the bottom, dimmed, what was left behind on purpose (not counted in the percentage).

## Running it

```
python tools/estado-rust/estado.py
```

It only needs Python 3. It writes `tools/estado-rust/estado.svg` (or the path given as an argument) and prints a summary per part.

## `mudanza.txt`

Each line is `<pattern> <part> <state>`:

- `pattern`: path inside `DS5Control/` with `/`; `*` is a wildcard (it also crosses folders). The first matching line wins.
- `part`: `mapeo` (mapping engine), `perfiles` (profiles and settings), `mando` (physical controller), `virtual`, `app` or `fuera` (not moved).
- `state`: `hecho` (done, 100), `pendiente` (pending, 0) or a number from 0 to 100.

`dato <name> <value>` lines are badges; `dato states a+b+c` adds the numbers and shows them as states identical to C#. Empty lines and lines starting with `#` are ignored.
