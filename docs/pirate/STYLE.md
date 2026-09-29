# Writing the pirate pages

Sea shanty mode shows every page of the docs as the crew of _Bound for the
Main_ (the hk sea shanty, [lyrics](../shanty.md)) would tell it. Each English
page `docs/<page>.md` has a variant at `docs/pirate/<page>.md`. The switch in
the header moves a reader between the two, keeping their place, so a variant
has to be the same page underneath: the same sections, the same facts, the same
commands, in the same order. Only the words around them are the crew's.

This file is not a page; the site leaves it out.

## The voice

A shanty-singing ship's crew explaining their trade to a new hand: warm,
salty, proud of the ship, and exact about how the work is done. The joke is in
the mapping of git hooks onto a sailing ship, not in misspelling every word.

- **Readable first.** A reader skimming for one fact must still find it at a
  glance. Keep sentences short and plain underneath the flavour.
- **Light dialect.** Use `ye`, `yer`, `aye`, `'tis`, `matey`, `arr` and the like
  as seasoning: a touch in most paragraphs, never every clause. Don't drop
  every final g or write `o'` for every "of". No apostrophe soup.
- **Nautical metaphor, used consistently.** Use the lexicon below, so the same
  thing has the same name on every page.
- **No slurs, no violence played for laughs, no "wench"-style stereotypes.** The
  crew is everyone who sails with hk.
- **Lead with the crew hauling together.** hk's pitch is that fixers run in
  parallel, even on shared files, because locks keep them from colliding. Say
  what runs at once first; "two hands on one sail take turns" is the footnote,
  never the headline.
- **An occasional line from the song** is welcome as a flourish (a lead-in or a
  closing line), not in every section. See the lyrics for the lines.

## The lexicon

Use these words, and only these, for the things on the left. "Plainly" means
keep the English word; a crew gloss after it, once on a page, is fine.

| hk / git                                          | The crew's word                                                          |
| ------------------------------------------------- | ------------------------------------------------------------------------ |
| repository, project                               | the ship, the vessel                                                     |
| monorepo                                          | one ship, many crews (never a fleet)                                     |
| subproject (a directory with its own `hk.pkl`)    | a quarter of the ship with charts of its own                             |
| workspace (`workspace_indicator`)                 | a cabin, known by the marker on its door                                 |
| directory                                         | directory, plainly                                                       |
| commit                                            | a voyage; making a commit is setting sail                                |
| partial commit                                    | a partial voyage: setting sail with part of the cargo                    |
| commit message                                    | the name ye christen her with                                            |
| the main branch                                   | the main (as in "bound for the main")                                    |
| other branches, push, merge, rebase, checkout     | plainly                                                                  |
| files                                             | the cargo                                                                |
| the files a run works on (`--files0-from`)        | the manifest; excluding files strikes them off it                        |
| `--all`, every tracked file                       | the whole of the cargo                                                   |
| staging, staged changes                           | loading cargo aboard; cargo loaded aboard for this voyage                |
| unstaged changes                                  | cargo left on the dock                                                   |
| untracked files                                   | untracked files, plainly ("cargo Git doesn't track yet" as a gloss)      |
| stash / restore                                   | stow it in the hold / bring it up from the hold                          |
| git hook                                          | the bosun's pipe that calls all hands on deck; it sounds when it runs    |
| installing / uninstalling hooks                   | rigging the hooks / taking them down                                     |
| a global install (every repository)               | the whole fleet, every ship on the machine                               |
| agent and editor hooks, not Git's                 | hooks, plainly; never the bosun's pipe                                   |
| hk                                                | hk (never renamed); "the bosun" when a figure is needed                  |
| step                                              | a hand, one of the crew                                                  |
| the steps, together                               | the crew (as the song has it: "all muster the self-same crew")           |
| defining / removing a step                        | signing on a hand / signing a hand off                                   |
| group (`Group`)                                   | a gang of hands                                                          |
| job                                               | job, plainly ("one haul on the line" as a gloss)                         |
| one run of a hook or command                      | a run, or a passage                                                      |
| `--plan` output                                   | the passage plan                                                         |
| running steps in parallel                         | all hands haul at once                                                   |
| file locks                                        | lashings: "a lock on each file takes the strain"                         |
| skipping a step or hook                           | it sits this one out                                                     |
| enabling / disabling a profile, fixing, a feature | calling it up / standing it down                                         |
| linter                                            | a lookout                                                                |
| formatter, fixer                                  | a sailmaker, who mends the canvas                                        |
| partial fixer                                     | a sailmaker who can't mend every tear                                    |
| check                                             | inspecting the cargo                                                     |
| fix                                               | mending the canvas                                                       |
| patch, diff, `check_diff`, `check_list_files`     | plainly (a patch is sailmaker's work already)                            |
| diagnostics (SARIF, JUnit, normalized output)     | diagnostics, plainly ("what the lookouts sang out" as a gloss)           |
| step-defined tests                                | drills                                                                   |
| builtins                                          | the standing crew who come with the ship                                 |
| the list of builtins                              | the crew roster                                                          |
| `hk util` utilities                               | the ship's tool chest                                                    |
| customizing, amending, migrating                  | refitting                                                                |
| removed, deprecated                               | struck off                                                               |
| `hk.pkl`, hk's configuration                      | the ship's charts, the charts                                            |
| the root config of a monorepo                     | the master chart                                                         |
| `hk.local.pkl`, local overrides                   | yer own marks on the charts                                              |
| user configuration (`~/.config/hk/config.pkl`)    | yer sea chest, which goes with ye from ship to ship                      |
| runtime settings                                  | settings, plainly: the settings the ship sails by, the settings in force |
| configuration precedence                          | the chain of command: who outranks whom                                  |
| where a setting comes from                        | where it hails from                                                      |
| environment variables                             | standing orders                                                          |
| CLI commands                                      | the bosun's calls; one command is a call                                 |
| CLI flags                                         | flags, flown on a call                                                   |
| Pkl                                               | the chart-maker's language                                               |
| configuration examples                            | ships in bottles                                                         |
| hk's cache                                        | hk's locker                                                              |
| a network download (Pkl packages)                 | sending a boat ashore; offline, no boats go ashore                       |
| mise                                              | the quartermaster, who provisions the tools                              |
| installing a tool / an installed tool             | bringing it aboard / on hand                                             |
| CI                                                | the harbour-master                                                       |
| running hk by hand in a terminal                  | at the helm ("the hook, the helm and the harbour-master")                |
| a developer's own machine                         | yer own machine, plainly                                                 |
| logs, log levels, traces                          | the ship's log                                                           |
| a failing check, a blocked commit                 | a squall; the hook hauls her back to port                                |
| trouble in general, troubleshooting               | foul weather                                                             |
| coding agents                                     | clockwork hands                                                          |
| agent instructions (`hk agent instructions`)      | sailing instructions                                                     |
| MCP host, editor                                  | host, plainly                                                            |
| the MCP dashboard                                 | the view from the quarterdeck                                            |
| moving from another hook manager                  | changing ships; the other tools are hook managers, never ships           |
| getting started                                   | getting under way                                                        |
| contributors                                      | those who sign aboard                                                    |
| other developers, a team                          | yer shipmates                                                            |
| users, the reader                                 | ye, sailor, matey                                                        |

Names never change: hk, Git, Pkl, mise, GitHub, and every tool (prettier,
eslint, ruff, shellcheck …) keep their real names.

### One word, one meaning

These words are taken. Don't lend them to anything else:

- **Voyage** and **set sail** mean a commit, nothing more. Starting out is
  getting under way; a run is a run or a passage ("every time the crew hauls",
  not "every voyage"). "Weigh anchor" is only the sidebar's "Start here".
- **Hand**, on its own, is a step. People are sailors, shipmates, or those who
  sign aboard. "All you hands who would sign aboard" is the song's line and
  stays only as a quotation; "lend a hand" and "lay hands on" are idioms and
  fine.
- **Crew** is the steps working together, or the crew telling the page ("the
  crew's glossary"). Other developers are yer shipmates, not "the whole crew".
- **Sign on** is for steps; **sign aboard** is for people.
- **Hold** and **stow** mean stashing. Not the cache, not all the files, not
  something embedded or bundled. The ordinary verb ("the file holds the path")
  is fine.
- **Orders** are environment variables, the standing orders. On the
  environment variables page, "order" alone is short for one. Settings are
  settings, commands are calls, flags are flags, and shell lines are commands.
  In a heading, write "standing orders (environment variables)". The verb ("as
  its configuration orders") is fine.
- **The dock** is unstaged cargo only, never a developer's machine or a local
  run.
- **Fleet** means every repository on the machine. A monorepo is one ship: its
  subprojects are quarters, its workspaces cabins, its groups gangs. Other hook
  managers are not ships. The sponsors' "keep the fleet afloat" (open source at
  large) is fine.
- **Charts** are hk's configuration; no "articles". The fixed labels
  `## Further charts` and the sidebar's "Charts and tables" mean the reference
  pages, and go no further.
- **Deck** is where the hands work and the output shows ("all hands on deck",
  "clears it off the deck"), not a directory. Don't use **below decks**: a
  profile that isn't on is a watch off duty.
- **Stand down** turns something off; **sit this one out** skips a step or
  hook. No "shore leave".
- **Rig** is installing hooks or setting something up (a project, a host, a
  tool's configuration); a tool itself is brought aboard. **Refit** is changing
  what's already there (a builtin, an example, the move to v2). No "rigger".
- **Muster** gathers hands into a hook or group. Files are selected, not
  mustered.
- **Squall** is a failing check or a blocked commit, and "back to port" is where
  it sends her. Trouble in general is foul weather. A setting hails from its
  source, not from a port.
- **The ship's log** is logs only. Diagnostics and reports are diagnostics.
- **Quoted song lines** stay as sung, even where the lexicon differs
  ("harbor-master", "all you hands").

## What must not change

`node .vitepress/pirate-pages.mjs check <page>` enforces most of these, and the
docs build runs it for every variant that is up to date.

1. **Front matter.** Rewrite `title` and `description` in the crew's voice
   when the English page has them; keep the subject recognisable
   ("Hooks and stowing the hold", not just "Arr!"). Every other key (`outline`,
   `layout`, `sidebar` …) is copied unchanged. Add `sourceHash` with `stamp`
   (below); never write it by hand.
2. **Headings.** One `#` heading, like the English page. Every other heading
   keeps its level and order and carries the English heading's anchor:
   `## Stowin' the cargo {#stashing-and-partial-commits}`. `outline` lists the
   anchors. Links from other pages and the switch's "keep my place" both rely on
   them.
3. **Code blocks, byte for byte.** Copy every fenced block unchanged: language,
   meta and contents, comments included. Readers paste them. No pirate words
   inside code, ever.
4. **Links.** Keep every link target exactly as the English page writes it
   (`/hooks#file-selection`, `./fix.md`, `https://…`); the site moves links to
   pirate pages by itself. Pirate link text is fine. Add no links, drop none.
5. **Inline code.** Keep every inline code span from the English page, unchanged:
   commands, flags, keys, values, paths, variable names. The words around them
   are yours.
6. **Facts.** Every fact, default, number, version, caveat, warning, limitation
   and recommendation in the English page is in the variant, with the same
   strength. Don't soften a warning into a joke, and don't invent behaviour,
   flags, or tools. When the metaphor fights the meaning, the meaning wins: say
   it plainly.
7. **Containers, tables, lists.** The same containers (`::: tip` …) in the same
   order; translate a container's own title
   (`::: tip Make the linters available` → `::: tip Muster yer lookouts`), and
   the site supplies pirate titles for containers without one. Tables keep
   their rows and columns; translate prose cells and headers, keep cells that
   are code or values. Lists keep their items.
8. **Everything else.** Images, Vue components (`<ShantyVideo />` …), includes
   (`<!--@include: ../gen/pkl-config.md-->`, path relative to the variant) and
   raw HTML stay, with only their visible prose translated. Never write `{{` in
   prose: Vue reads it as an expression.

### Reference pages generated from the CLI

`docs/cli/**` is generated from hk's usage spec, and so are their variants:
`.vitepress/pirate-cli.mjs` builds `docs/pirate/cli/**` from the English pages
and the crew's words in `docs/pirate/cli.json`, and `mise run render:usage` runs
it right after it writes the English pages. Never edit a page under
`docs/pirate/cli/`: edit `cli.json`, then run `aube run pirate:cli` from `docs/`.

`cli.json` translates the pages line by line, keyed by each English line exactly
as the generated page has it:

- `lines` holds the words for every page, so a flag that several commands share
  reads the same on each of them;
- `pages` holds one page's own words (`"util.md": { … }`), which win over
  `lines`;
- `labels` swaps a prefix wherever it starts a line (`- **Usage:** `).

A heading's words go without an anchor; the script adds the English heading's.
Code blocks, commands and links come from the English page as they are, and the
words follow the rules above: keep every inline code span and link a line has.
A command page keeps the command as its `title` and `#` heading.

A line with no words in `cli.json` stays in English, and its page says how many
of its lines do; `aube run pirate:cli --missing` lists them. So a new or
reworded flag never leaves a page wrong or stale: the English words stand until
someone gives them the crew's. To keep a line in English on purpose, map it to
itself.

Use these labels so every command page reads alike (a heading shows its anchor
below, which the script adds; in `cli.json` give the words alone, or with that
same anchor):

| English                                                          | Pirate                                                                                                  |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `**Usage:**`                                                     | `**How to hail it:**`                                                                                   |
| `**Aliases:**`                                                   | `**Also answers to:**`                                                                                  |
| `**Effect:** read-only`                                          | `**Effect:** read-only (looks, never touches)`                                                          |
| `**Effect:** modifies state`                                     | `**Effect:** modifies state (it touches, as well as looks)`                                             |
| `**Effect:** destructive — may delete or irreversibly overwrite` | `**Effect:** destructive — may delete or irreversibly overwrite (what's cut away can't be hauled back)` |
| `**Choices:**`                                                   | `**Choose from:**`                                                                                      |
| `**Default:**`                                                   | `**Unless ye say otherwise:**`                                                                          |
| `**Version:**`                                                   | `**Version:**`, unchanged                                                                               |
| `-h --help` — Print help                                         | Print help, for when ye've lost yer bearings                                                            |
| `## Arguments`                                                   | `## Cargo it takes {#arguments}`                                                                        |
| `## Flags`                                                       | `## Flags to fly {#flags}`                                                                              |
| `## Subcommands`                                                 | `## Lesser calls {#subcommands}`                                                                        |
| `## Examples`                                                    | `## Tales from the deck {#examples}`                                                                    |
| `## Learn more`                                                  | `## Further charts {#learn-more}`                                                                       |

The labels that aren't CLI-only (`**Default:**`, `**Choices:**` …) read the
same on every reference page, `environment_variables.md` included. `**Type:**`
stays as it is.

The links under `## Further charts` use these words:

| English                                       | Pirate                                                           |
| --------------------------------------------- | ---------------------------------------------------------------- |
| `[Getting started](/getting_started)`         | `[Getting started: get under way](/getting_started)`             |
| `[Troubleshooting](/logging)`                 | `[Troubleshooting: the ship's log](/logging)`                    |
| `[All commands](/cli/)`                       | `[All the bosun's calls](/cli/)`                                 |
| `[Built-in linters and utilities](/builtins)` | `[Built-in linters and utilities: the standing crew](/builtins)` |
| `[Git hooks and stashing](/hooks)`            | `[Git hooks and stowing the hold](/hooks)`                       |
| `[Configuration guide](/configuration)`       | `[Configuration guide: the ship's charts](/configuration)`       |
| `[Coding agents](/agents)`                    | `[Coding agents: the clockwork hands](/agents)`                  |

A shared flag's line gives the English sentence first and the crew's gloss after
it.

### Configuration example pages

`docs/reference/examples/*` share their shape, so their variants share these
words:

| English                        | Pirate                                                       |
| ------------------------------ | ------------------------------------------------------------ |
| `**Prerequisites:**`           | `**Before ye sail:**`                                        |
| `## Configuration`             | `## The charts {#configuration}`                             |
| `## Try it`                    | `## Take her out {#try-it}`                                  |
| `## Adapt it`                  | `## Refit her for yer own ship {#adapt-it}`                  |
| ``… and save it as `hk.pkl`.`` | ``… and save it as `hk.pkl`, the charts yer ship sails by.`` |

### Generated reference left in English

Where a page includes generated reference (`<!--@include: ../gen/…-->`), say so
once, in these words: "The reference below is generated from …, so it stays in
plain English."

### The sidebar

The sidebar's words are `PIRATE_SIDEBAR_TEXT` in `.vitepress/sidebar.ts`. A
page's entry is its `#` heading ("Getting under way"), or, when the heading is
long, the heading's own crew phrase with the English subject in brackets ("The
harbour-master (CI)" for "Continuous integration: the harbour-master"). Never a
different phrase from the heading: change the heading and its entry together.

## When the English page changes

The generated CLI pages never go stale (see above). Every other variant records
the English page it was written from (`sourceHash`). When the English page
changes, the variant is stale: it keeps building and shows a
notice that links to the English page, and `status` lists it. The build never
fails over a stale variant, so English docs never wait for the crew.

To bring a variant up to date, or to write a new one:

```sh
node .vitepress/pirate-pages.mjs status          # stale, missing and orphaned variants
node .vitepress/pirate-pages.mjs outline hooks.md  # anchors, links and code a variant must keep
# write or update docs/pirate/hooks.md, then:
node .vitepress/pirate-pages.mjs stamp hooks.md
node .vitepress/pirate-pages.mjs check hooks.md
```

Run them from `docs/`. An English page with no variant still works in sea
shanty mode: it is shown in English, themed, with a note that says so.
