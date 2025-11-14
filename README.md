# json2jsonl

This is an extremely basic tool, but it reads a JSON array and turns it into jsonl (one value per line). That's it.

All the real work is thanks to https://github.com/bambolelooo/large-json-array/ .

Usage:

```
Convert JSON array to JSONL

Usage: json2jsonl [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file (stdin if not provided)

Options:
  -o, --o <O>     Output file (stdout if not provided)
  -p, --progress  Progress bar
  -h, --help      Print help
```
