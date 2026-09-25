# histtop

Your shell history file is a flat, in-order log of everything you've typed.
There's no built-in way to see which commands you actually rely on versus
the one-off typos and experiments. The classic pipeline

```
history | sort | uniq -c | sort -rn
```

sort of works, but it breaks on zsh's extended history format (each line
prefixed with `: <timestamp>:<elapsed>;`), ignores bash's separate
`#<timestamp>` comment lines, and buries useful results under noise like a
hundred bare `ls` and `cd` entries.

`histtop` reads a bash or zsh history file directly, understands both
formats, and prints commands ranked by how often you actually ran them.

## Usage

```
histtop ~/.zsh_history
histtop ~/.bash_history --top 10
histtop ~/.zsh_history --min-words 2   # skip single-word commands like `ls`, `cd`
```

Example output:

```
    42  git status
    31  git commit -m
    18  cargo test
     9  ls -la
```

Run with no arguments and it picks a default history file based on `$SHELL`
(`~/.zsh_history` or `~/.bash_history`).

## Building

Standard library only, no external dependencies:

```
cargo build --release
```

## Status

Early. Commands split across lines with a trailing `\` are reassembled
into a single entry, but there's no way to exclude commands by pattern yet.
Working, but small.
