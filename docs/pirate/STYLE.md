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

| hk / git                         | The crew's word                                           |
| -------------------------------- | --------------------------------------------------------- |
| repository, project              | the ship, the vessel                                      |
| commit                           | a voyage; making a commit is setting sail                 |
| commit message                   | the name ye christen her with                             |
| the main branch                  | the main (as in "bound for the main")                     |
| git hook                         | the bosun's pipe that calls all hands on deck             |
| hk                               | hk (never renamed); "the bosun" when a figure is needed   |
| step                             | a hand, one of the crew                                   |
| linter                           | a lookout                                                 |
| formatter, fixer                 | a sailmaker, a rigger: they mend the canvas               |
| check                            | inspecting the cargo                                      |
| fix                              | mending the canvas                                        |
| files                            | the cargo                                                 |
| staged changes                   | cargo loaded aboard for this voyage                       |
| unstaged changes                 | cargo left on the dock, or stowed below                   |
| stash / restore                  | stow it in the hold / bring it up from the hold           |
| file locks                       | lashings: "a lock on each file takes the strain"          |
| running steps in parallel        | all hands haul at once                                    |
| `hk.pkl`                         | the ship's charts, the articles the crew sails by         |
| Pkl                              | the chart-maker's language                                |
| CI                               | the harbour-master                                        |
| profiles                         | watches (the slow watch, the CI watch)                    |
| builtins                         | the standing crew who come with the ship                  |
| mise                             | the quartermaster, who provisions the tools               |
| environment variables            | standing orders, or winds and currents                    |
| `--plan` output                  | the passage plan                                          |
| CLI commands                     | the bosun's calls, orders                                 |
| coding agents                    | clockwork hands                                           |
| logs                             | the ship's log                                            |
| a failing check, a blocked commit | a squall; the hook hauls her back to port                |
| contributors                     | those who sign aboard                                     |
| users, the reader                | ye, sailor, matey                                         |

Names never change: hk, Git, Pkl, mise, GitHub, and every tool (prettier,
eslint, ruff, shellcheck …) keep their real names.

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

`docs/cli/**` is generated from hk's usage spec. Their variants follow the same
rules and keep the command as their `title` and `#` heading. Use these labels
so every command page reads alike:

| English              | Pirate                                   |
| -------------------- | ---------------------------------------- |
| `**Usage:**`         | `**How to hail it:**`                    |
| `**Aliases:**`       | `**Also answers to:**`                   |
| `**Effect:** read-only` | `**Effect:** read-only (looks, never touches)` |
| `**Effect:** modifies state` | `**Effect:** modifies state (lays hands on the cargo)` |
| `**Choices:**`       | `**Choose from:**`                       |
| `**Default:**`       | `**Unless ye say otherwise:**`           |
| `## Arguments`       | `## Cargo it takes {#arguments}`         |
| `## Flags`           | `## Flags to fly {#flags}`               |
| `## Subcommands`     | `## Lesser calls {#subcommands}`         |
| `## Examples`        | `## Tales from the deck {#examples}`     |
| `## Learn more`      | `## Further charts {#learn-more}`        |

Replace the `<!-- @generated … -->` comment with
`<!-- Pirate variant of docs/cli/<page>.md; see docs/pirate/STYLE.md. -->`, and
keep `<!-- hk documentation examples -->` where the English page has it.

## When the English page changes

A variant records the English page it was written from (`sourceHash`). When
the English page changes, the variant is stale: it keeps building and shows a
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
