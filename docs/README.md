# Documentation

**Looking for the manual?** It is at [manual/](manual/), and it is published at
<https://toyoshim-i.github.io/Karakuri/manual/>. Everything else here is written for
whoever is building this, not for whoever is playing it.

## The map

| | |
|---|---|
| [roadmap.md](roadmap.md) | Where this is going, milestone by milestone, and which constraints a later milestone puts on earlier code. Start here to find out what is owed |
| [ir-spec.md](ir-spec.md) | The `.kir` language, the Set file format, the session stream and the metadata card. The reference the compiler is checked against |
| [architecture.md](architecture.md) | How the crates fit together |
| [manual.md](manual.md) | The command line and the keys — every flag, every key press, and what the store holds |
| [contributing.md](contributing.md) | How to work in this repository: running the tests, splitting the work, and where a record has to be hooked from |
| [plugins.md](plugins.md) | The plugin seam, and why output routing lives outside this repository |
| [adr/](adr/) | Every decision, with the alternative that lost. [INDEX.md](adr/INDEX.md) is the table |
| [principles/](principles/) | The rules that stand today, one file per rule. `ls` is the index, because each filename is the rule it states |

## Two kinds of document, and they are different on purpose

`principles/` **accumulates**, because a codebase learns and a rule earned by a mistake
should outlive the mistake. The manual's [seven rules](manual/index.html) **must not**,
because they are the mental model somebody holds before the panel makes sense, and a set of
rules that keeps growing is a set nobody holds. Adding one there means removing or merging
one.

The manual is HTML and everything else is Markdown, which is the same distinction said in
the file extension: one is a designed thing with readers who never open the repository, the
other is prose for people who are already in it.
