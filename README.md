# @tabnas/jsonic-cli

Command-line interface for [`@tabnas/jsonic`](https://github.com/tabnas/jsonic) —
a JSON parser that isn't strict. Installs the `jsonic` command, which parses
relaxed-JSON source (from arguments, files or STDIN) and prints standard JSON.

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`@tabnas/jsonic-cli`), providing the `jsonic` command. |

See [`ts/README.md`](ts/README.md) for usage.

> The BNF / grammar-conversion CLI lives in [`@tabnas/bnf`](https://github.com/tabnas/abnf)
> as the `tabnas-bnf` command.

## Quick start

```sh
npm install -g @tabnas/jsonic-cli
jsonic a:1
# => {"a":1}
```

## License

MIT. Copyright (c) Richard Rodger.
