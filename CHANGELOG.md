# Changelog

## [1.15.2](https://github.com/jdx/hk/compare/v1.15.1..v1.15.2) - 2025-09-20

### 📚 Documentation

- remove references to alternative config formats by [@jdx](https://github.com/jdx) in [5e497b6](https://github.com/jdx/hk/commit/5e497b6a9babdddeb78ed0de58a5167c541b6128)

## [1.15.1](https://github.com/jdx/hk/compare/v1.15.0..v1.15.1) - 2025-09-19

### 🐛 Bug Fixes

- Skip any hunks that begin before the current position to avoid partial re-application by [@jdx](https://github.com/jdx) in [8325a66](https://github.com/jdx/hk/commit/8325a66d7b6b85b672c8780b6f5a0e10af5dc0ce)

## [1.15.0](https://github.com/jdx/hk/compare/v1.14.0..v1.15.0) - 2025-09-19

### 🚀 Features

- **(docs)** add custom VitePress theme with unique branding by [@jdx](https://github.com/jdx) in [#287](https://github.com/jdx/hk/pull/287)

### 🐛 Bug Fixes

- handle fixes and unstaged hunks in the same file by [@jdx](https://github.com/jdx) in [#283](https://github.com/jdx/hk/pull/283)

### 📦️ Dependency Updates

- pin dependencies by [@renovate[bot]](https://github.com/renovate[bot]) in [#288](https://github.com/jdx/hk/pull/288)
- update actions/checkout action to v5 by [@renovate[bot]](https://github.com/renovate[bot]) in [#289](https://github.com/jdx/hk/pull/289)
- update actions/download-artifact action to v5 by [@renovate[bot]](https://github.com/renovate[bot]) in [#290](https://github.com/jdx/hk/pull/290)
- update actions/upload-pages-artifact action to v4 by [@renovate[bot]](https://github.com/renovate[bot]) in [#291](https://github.com/jdx/hk/pull/291)

## [1.14.0](https://github.com/jdx/hk/compare/v1.13.7..v1.14.0) - 2025-09-18

### 🚀 Features

- centralized settings registry with codegen by [@jdx](https://github.com/jdx) in [#284](https://github.com/jdx/hk/pull/284)

### 🔍 Other Changes

- removed snapshot file by [@bhanuprasad14](https://github.com/bhanuprasad14) in [0952af5](https://github.com/jdx/hk/commit/0952af5dc4516a309d93da74155935c48eed81d1)
- Extract Pkl error handling into separate method by [@jdx](https://github.com/jdx) in [e62cb24](https://github.com/jdx/hk/commit/e62cb2490c6917fe70836b543e532348cd3722f4)

## [1.13.7](https://github.com/jdx/hk/compare/v1.13.6..v1.13.7) - 2025-09-17

### 🔍 Other Changes

- include all Pkl package files (zip, sha256, and raw) in artifact and release by [@jdx](https://github.com/jdx) in [fbea8ff](https://github.com/jdx/hk/commit/fbea8ff4591e9ed6f63abd0cff3fd640f9e860d3)

## [1.13.6](https://github.com/jdx/hk/compare/v1.13.5..v1.13.6) - 2025-09-17

### 🔍 Other Changes

- fix build-pkl upload path to use mise dist (docs/public/pkl) by [@jdx](https://github.com/jdx) in [6745830](https://github.com/jdx/hk/commit/6745830a9f5f4bb949216b682ba501119aa4262f)

## [1.13.5](https://github.com/jdx/hk/compare/v1.13.4..v1.13.5) - 2025-09-17

### 🔍 Other Changes

- fix pkl release building by [@jdx](https://github.com/jdx) in [#278](https://github.com/jdx/hk/pull/278)
- enable cache for release-plz by [@jdx](https://github.com/jdx) in [85195fa](https://github.com/jdx/hk/commit/85195fadfcaa50137d3807b165d924bcabbc6d0a)

## [1.13.4](https://github.com/jdx/hk/compare/v1.13.3..v1.13.4) - 2025-09-17

### 🐛 Bug Fixes

- explicitly install pkl and run package command directly by [@jdx](https://github.com/jdx) in [#276](https://github.com/jdx/hk/pull/276)

## [1.13.3](https://github.com/jdx/hk/compare/v1.13.2..v1.13.3) - 2025-09-17

### 🔍 Other Changes

- diagnose pkl package failure in release workflow by [@jdx](https://github.com/jdx) in [#274](https://github.com/jdx/hk/pull/274)

## [1.13.2](https://github.com/jdx/hk/compare/v1.13.1..v1.13.2) - 2025-09-17

### 🐛 Bug Fixes

- use simple directory path for pkl artifact upload by [@jdx](https://github.com/jdx) in [#272](https://github.com/jdx/hk/pull/272)

## [1.13.1](https://github.com/jdx/hk/compare/v1.13.0..v1.13.1) - 2025-09-17

### 🐛 Bug Fixes

- correct pkl package artifact path in release workflow by [@jdx](https://github.com/jdx) in [#271](https://github.com/jdx/hk/pull/271)

## [1.13.0](https://github.com/jdx/hk/compare/v1.12.1..v1.13.0) - 2025-09-17

### 🚀 Features

- add tracing and performance diagnostics support by [@jdx](https://github.com/jdx) in [3045a38](https://github.com/jdx/hk/commit/3045a3826e4b1c4213dbccadb7dc1ce442a814fa)
- add tracing and performance diagnostics support by [@jdx](https://github.com/jdx) in [#259](https://github.com/jdx/hk/pull/259)
- comprehensive configuration unification with proper precedence and union semantics by [@jdx](https://github.com/jdx) in [#266](https://github.com/jdx/hk/pull/266)
- centralized skip configuration for steps and hooks by [@jdx](https://github.com/jdx) in [#268](https://github.com/jdx/hk/pull/268)

### 🐛 Bug Fixes

- pre-commit partial staging: honor env stash, alias patch-file, robust partial stash by [@jdx](https://github.com/jdx) in [#267](https://github.com/jdx/hk/pull/267)

### 📚 Documentation

- add comprehensive schema reference and built-in linters documentation by [@jdx](https://github.com/jdx) in [#261](https://github.com/jdx/hk/pull/261)
- add comprehensive logging and debugging guide by [@jdx](https://github.com/jdx) in [#263](https://github.com/jdx/hk/pull/263)
- add glossary and update sidebar navigation by [@jdx](https://github.com/jdx) in [#264](https://github.com/jdx/hk/pull/264)
- document configuration unification and skip configuration features by [@jdx](https://github.com/jdx) in [#269](https://github.com/jdx/hk/pull/269)

### 🔍 Other Changes

- **(ci)** include Pkl artifacts in releases; support manual version input by [@jdx](https://github.com/jdx) in [#265](https://github.com/jdx/hk/pull/265)
- mise.lock by [@bhanuprasad14](https://github.com/bhanuprasad14) in [5a5a5d0](https://github.com/jdx/hk/commit/5a5a5d03727a72a260383f67eb7696857894d664)

### New Contributors

- @bhanuprasad14 made their first contribution

## [1.12.1](https://github.com/jdx/hk/compare/v1.12.0..v1.12.1) - 2025-09-13

### 🐛 Bug Fixes

- include pkl packages in releases by [@jdx](https://github.com/jdx) in [#227](https://github.com/jdx/hk/pull/227)
- improve stashing error handling and robustness by [@jdx](https://github.com/jdx) in [#229](https://github.com/jdx/hk/pull/229)
- only show fix help message when running in check mode by [@jdx](https://github.com/jdx) in [#230](https://github.com/jdx/hk/pull/230)

## [1.12.0](https://github.com/jdx/hk/compare/v1.11.2..v1.12.0) - 2025-09-07

### 🚀 Features

- display fix instructions when running under check by [@jdx](https://github.com/jdx) in [#223](https://github.com/jdx/hk/pull/223)

## [1.11.2](https://github.com/jdx/hk/compare/v1.11.1..v1.11.2) - 2025-09-07

### 🔍 Other Changes

- remove gh release create from release-plz by [@jdx](https://github.com/jdx) in [ae69d4a](https://github.com/jdx/hk/commit/ae69d4acc39f95752a32907c53e89476da5366bc)

## [1.11.1](https://github.com/jdx/hk/compare/v1.11.0..v1.11.1) - 2025-09-07

### 🐛 Bug Fixes

- release with gh by [@jdx](https://github.com/jdx) in [7d10b8d](https://github.com/jdx/hk/commit/7d10b8d8c356b5207bbadd57ebeccdfaeca55d71)

## [1.11.0](https://github.com/jdx/hk/compare/v1.10.7..v1.11.0) - 2025-09-06

### 🚀 Features

- **(test)** support before/after commands in hk test by [@jdx](https://github.com/jdx) in [#214](https://github.com/jdx/hk/pull/214)
- support fail_fast in hk.pkl (env > config) by [@jdx](https://github.com/jdx) in [#212](https://github.com/jdx/hk/pull/212)

### 🐛 Bug Fixes

- remove period from missin-profile message by [@jdx](https://github.com/jdx) in [5f525a4](https://github.com/jdx/hk/commit/5f525a46e0815c0179f0a49d5bd25656a1e7f0f3)
- add pre-commit task to mise.toml generation by [@jdx](https://github.com/jdx) in [#217](https://github.com/jdx/hk/pull/217)
- prefer unstaged over fixer changes; keep untracked in patch-file by [@jdx](https://github.com/jdx) in [#219](https://github.com/jdx/hk/pull/219)

### 📚 Documentation

- Add missing code block end marker by [@karouf](https://github.com/karouf) in [#213](https://github.com/jdx/hk/pull/213)

### 🔍 Other Changes

- add pkl linter in Builtins by [@aheritier](https://github.com/aheritier) in [#210](https://github.com/jdx/hk/pull/210)
- set gh token by [@jdx](https://github.com/jdx) in [#215](https://github.com/jdx/hk/pull/215)
- support immutable releases by [@jdx](https://github.com/jdx) in [#216](https://github.com/jdx/hk/pull/216)
- Add Claude Code GitHub Workflow by [@jdx](https://github.com/jdx) in [#218](https://github.com/jdx/hk/pull/218)
- bump xx by [@jdx](https://github.com/jdx) in [82dba87](https://github.com/jdx/hk/commit/82dba87d3fc3fcbe25b290570529ea72c7acb31e)
- lint by [@jdx](https://github.com/jdx) in [912e9c8](https://github.com/jdx/hk/commit/912e9c8c028caf966c949d81064a22616b33325c)

### New Contributors

- @karouf made their first contribution in [#213](https://github.com/jdx/hk/pull/213)
- @aheritier made their first contribution in [#210](https://github.com/jdx/hk/pull/210)

## [1.10.7](https://github.com/jdx/hk/compare/v1.10.6..v1.10.7) - 2025-08-27

### 🐛 Bug Fixes

- show `--from-ref` instructions for missing profile warning by [@jdx](https://github.com/jdx) in [#204](https://github.com/jdx/hk/pull/204)

### 📚 Documentation

- document default_branch by [@jdx](https://github.com/jdx) in [#206](https://github.com/jdx/hk/pull/206)

## [1.10.6](https://github.com/jdx/hk/compare/v1.10.5..v1.10.6) - 2025-08-26

### 🔍 Other Changes

- **(cache)** run init after loading cached config to apply warnings/settings\n\nFixes missing profile summary not showing on cached runs. Also ensures env/settings side-effects from config are applied consistently whether read fresh or from cache. by [@jdx](https://github.com/jdx) in [3b08788](https://github.com/jdx/hk/commit/3b08788956718d82e1f612e83cf36b6059e03365)

## [1.10.5](https://github.com/jdx/hk/compare/v1.10.4..v1.10.5) - 2025-08-26

### 🔍 Other Changes

- bump clx by [@jdx](https://github.com/jdx) in [a4e4f5c](https://github.com/jdx/hk/commit/a4e4f5c166d5c9c24d2c158cd7822be8cc4bc7f1)

## [1.10.4](https://github.com/jdx/hk/compare/v1.10.3..v1.10.4) - 2025-08-24

### 🔍 Other Changes

- bump clx by [@jdx](https://github.com/jdx) in [3dccbc4](https://github.com/jdx/hk/commit/3dccbc4484c46c9eb6feff20d4365297b642a832)

## [1.10.3](https://github.com/jdx/hk/compare/v1.10.2..v1.10.3) - 2025-08-24

### 🐛 Bug Fixes

- **(docs)** add v-pre directive to inline code for proper rendering by [@smasato](https://github.com/smasato) in [#200](https://github.com/jdx/hk/pull/200)
- **(step)** include root variant for '**/' stage globs (stage maintainers.yml at repo root) by [@jdx](https://github.com/jdx) in [#198](https://github.com/jdx/hk/pull/198)

### 🔍 Other Changes

- fix msrv job by [@jdx](https://github.com/jdx) in [5590e22](https://github.com/jdx/hk/commit/5590e2293f84907a142bd34334dbd0164bb050b6)

## [1.10.2](https://github.com/jdx/hk/compare/v1.10.1..v1.10.2) - 2025-08-22

### 🐛 Bug Fixes

- use correct expr environment with builtins by [@jdx](https://github.com/jdx) in [2284b33](https://github.com/jdx/hk/commit/2284b334200b5cb17dd06536b84755bf85778021)

### 📚 Documentation

- correct expr examples for git staged file syntax by [@jdx](https://github.com/jdx) in [f1fa1cf](https://github.com/jdx/hk/commit/f1fa1cfebec13cd03b1fb867777af477d433a3fe)

### 🔍 Other Changes

- remove embedded subtrees in favor of submodules by [@jdx](https://github.com/jdx) in [186480d](https://github.com/jdx/hk/commit/186480d317441cce2294a9960932939112dc11d6)
- fetch submodules by [@jdx](https://github.com/jdx) in [397e0d3](https://github.com/jdx/hk/commit/397e0d3ae78375be506963e6fe60900af4c23374)
- fixing release-plz by [@jdx](https://github.com/jdx) in [86a2d79](https://github.com/jdx/hk/commit/86a2d7942f6fa19222e2f6b8311adb8cfab74f20)

## [1.10.1](https://github.com/jdx/hk/compare/v1.10.0..v1.10.1) - 2025-08-22

### 🐛 Bug Fixes

- **(clx)** avoid Unicode slicing panic in truncate_text and previews by using char-safe prefix by [@jdx](https://github.com/jdx) in [3369b59](https://github.com/jdx/hk/commit/3369b591910d2bbdc36f8055021f469c307563ca)

## [1.10.0](https://github.com/jdx/hk/compare/v1.9.2..v1.10.0) - 2025-08-22

### 🚀 Features

- **(progress)** flex-width progress bar by [@jdx](https://github.com/jdx) in [#187](https://github.com/jdx/hk/pull/187)
- expose git status to expr conditions and Tera templates by [@jdx](https://github.com/jdx) in [#191](https://github.com/jdx/hk/pull/191)

### 🐛 Bug Fixes

- handle tera render errors without panic; improve progress UI stability by [@jdx](https://github.com/jdx) in [#190](https://github.com/jdx/hk/pull/190)

### 🔍 Other Changes

- ignore staged entries when worktree deleted (AD); add Bats repro by [@jdx](https://github.com/jdx) in [#188](https://github.com/jdx/hk/pull/188)

## [1.9.2](https://github.com/jdx/hk/compare/v1.9.1..v1.9.2) - 2025-08-19

### 🐛 Bug Fixes

- progress bar with children jobs by [@jdx](https://github.com/jdx) in [#185](https://github.com/jdx/hk/pull/185)
- bug with <clx:flex> tags appearing when they should not by [@jdx](https://github.com/jdx) in [a149229](https://github.com/jdx/hk/commit/a14922982add3abdcfcaee3d47ed2bac6c059bd0)

### 📚 Documentation

- add demo by [@jdx](https://github.com/jdx) in [282a62e](https://github.com/jdx/hk/commit/282a62ead207b294ed13e8b4dbe753f6b69b80f7)

### 🔍 Other Changes

- subtree-sync by [@jdx](https://github.com/jdx) in [b37605f](https://github.com/jdx/hk/commit/b37605fc0239fd9dbc8537fe5db55f7a7ba719ab)
- subtree-sync by [@jdx](https://github.com/jdx) in [d0fc8d3](https://github.com/jdx/hk/commit/d0fc8d34626672952b4162c2ab5186e0a94076bd)
- subtree by [@jdx](https://github.com/jdx) in [be0df56](https://github.com/jdx/hk/commit/be0df564a5147f4e26f2f06dff88579386b13708)

### New Contributors

- @mise-en-dev made their first contribution

## [1.9.1](https://github.com/jdx/hk/compare/v1.9.0..v1.9.1) - 2025-08-19

### 🐛 Bug Fixes

- **(test)** make sandbox opt-in via {{tmp}}; clean fixtures on success; fix builtins to use {{tmp}}; docs: update examples by [@jdx](https://github.com/jdx) in [#182](https://github.com/jdx/hk/pull/182)

### 🔍 Other Changes

- subtree-sync by [@jdx](https://github.com/jdx) in [#184](https://github.com/jdx/hk/pull/184)

## [1.9.0](https://github.com/jdx/hk/compare/v1.8.0..v1.9.0) - 2025-08-19

### 🚀 Features

- **(step)** per-step output summaries (stderr|stdout|combined|hide) by [@jdx](https://github.com/jdx) in [#180](https://github.com/jdx/hk/pull/180)
- add `warning` config to hk.pkl by [@jdx](https://github.com/jdx) in [#162](https://github.com/jdx/hk/pull/162)
- add `{{workspace_files}}` by [@jdx](https://github.com/jdx) in [#177](https://github.com/jdx/hk/pull/177)
- added `hk test` by [@jdx](https://github.com/jdx) in [#178](https://github.com/jdx/hk/pull/178)

### 🐛 Bug Fixes

- make NoFilesToProcess/ConditionFalse take precedence over ProfileNotEnabled by [@jdx](https://github.com/jdx) in [#179](https://github.com/jdx/hk/pull/179)

### 📚 Documentation

- add demo by [@jdx](https://github.com/jdx) in [#181](https://github.com/jdx/hk/pull/181)

## [1.8.0](https://github.com/jdx/hk/compare/v1.7.1..v1.8.0) - 2025-08-18

### 🚀 Features

- **(file)** added mv by [@jdx](https://github.com/jdx) in [794f384](https://github.com/jdx/hk/commit/794f3840e24aa275e7255714bf99125d8bcd91e0)
- **(hash)** added ensure_checksum_sha512 by [@jdx](https://github.com/jdx) in [951573c](https://github.com/jdx/hk/commit/951573ccdfe0c233bc6d6f6b1bb8b17f9f7f2613)
- **(process)** added arg/args by [@jdx](https://github.com/jdx) in [95888e4](https://github.com/jdx/hk/commit/95888e47ef4057c01b0bcdf91d7f712df476db58)
- added support for process::cmd to read stdout/stderr by line by [@jdx](https://github.com/jdx) in [2a39337](https://github.com/jdx/hk/commit/2a3933705a7d99b83004c82b95b14fd4b1daddb5)
- added git module by [@jdx](https://github.com/jdx) in [573040d](https://github.com/jdx/hk/commit/573040d313508959fecef1bdfbf6f2d23651ca2d)
- archive functions by [@jdx](https://github.com/jdx) in [24a95ff](https://github.com/jdx/hk/commit/24a95ff3c00a58d7c5f22356758c40c503080ccf)
- glob function by [@jdx](https://github.com/jdx) in [0a9af8e](https://github.com/jdx/hk/commit/0a9af8eff1e92316a4aea0d73a583cf712427024)
- added file::remove_dir_all by [@jdx](https://github.com/jdx) in [61cad67](https://github.com/jdx/hk/commit/61cad67a06b7fe944acab95abcd786239b9c4b3b)
- http by [@jdx](https://github.com/jdx) in [6065472](https://github.com/jdx/hk/commit/6065472a87d396753a31368f682bbae179cbff44)
- added rustls feature by [@jdx](https://github.com/jdx) in [d611055](https://github.com/jdx/hk/commit/d611055a9bd3297b49cb6b74f1304db1ac1df8d3)
- added hash functions by [@jdx](https://github.com/jdx) in [1c272c4](https://github.com/jdx/hk/commit/1c272c40e19f1e655b5ae83063ce062e28bead98)
- fslock by [@jdx](https://github.com/jdx) in [f4d012a](https://github.com/jdx/hk/commit/f4d012abcd2940ed05726b1f74254da2671c535b)
- reexport fs::file by [@jdx](https://github.com/jdx) in [e694919](https://github.com/jdx/hk/commit/e6949197abf8862efc574dcdee37ea594b929212)
- find_up by [@jdx](https://github.com/jdx) in [c096b24](https://github.com/jdx/hk/commit/c096b24ed5e6adb981a42d910d5392df3b3af295)
- add branch support on clone by [@acesyde](https://github.com/acesyde) in [1e7feb1](https://github.com/jdx/hk/commit/1e7feb1051608b013f1e417399bb49934212b40e)
- added duct cmd expression by [@jdx](https://github.com/jdx) in [d9bbf36](https://github.com/jdx/hk/commit/d9bbf3695144b368be8a23df89622f706b81a34d)
- add ungz method for gzip decompression by [@aniaan](https://github.com/aniaan) in [1e0f797](https://github.com/jdx/hk/commit/1e0f7976d7790f59c22f80714c3a44fee8eeb88c)
- add file and environment utilities from mise by [@jdx](https://github.com/jdx) in [afb8132](https://github.com/jdx/hk/commit/afb8132067c4a748dc74c7f832b95b223e53e125)
- convert xx, clx, and ensembler to git subtrees by [@jdx](https://github.com/jdx) in [8bb6d0b](https://github.com/jdx/hk/commit/8bb6d0be2e676bcd888d19630bbceb9abe3b4ca6)
- added support for process::cmd to read stdout/stderr by line by [@jdx](https://github.com/jdx) in [138e27d](https://github.com/jdx/hk/commit/138e27d7535c672310d154481a727186a08a55ec)
- always show elapsed time by [@jdx](https://github.com/jdx) in [dda0cff](https://github.com/jdx/hk/commit/dda0cffe738a8fc1d3f7fa1fc7b2b140d7af350b)
- added new progress job struct by [@jdx](https://github.com/jdx) in [3f3acdd](https://github.com/jdx/hk/commit/3f3acddbc06752c2046db223b6110dabd2d4b172)
- added body prop by [@jdx](https://github.com/jdx) in [6b1c164](https://github.com/jdx/hk/commit/6b1c1645cfda9f9708c95b7e6f7b9c2d6a4614ff)
- progress bar by [@jdx](https://github.com/jdx) in [5648144](https://github.com/jdx/hk/commit/5648144a41baa050d30bfd6458b00e078a676fc7)
- warn status by [@jdx](https://github.com/jdx) in [0236f0d](https://github.com/jdx/hk/commit/0236f0da975eb8f2196a0d764eff29ee5f9d01dc)
- show arrow on progress bar by [@jdx](https://github.com/jdx) in [b650cb8](https://github.com/jdx/hk/commit/b650cb8eaf815f19a7e6957e0b14be01b2bca541)
- add configurable skip message visibility by [@jdx](https://github.com/jdx) in [c934cbb](https://github.com/jdx/hk/commit/c934cbbc6c475c154f659f0c33fcdd052ba71b08)

### 🐛 Bug Fixes

- **(file)** accept generic content for write() by [@jdx](https://github.com/jdx) in [fa392c7](https://github.com/jdx/hk/commit/fa392c75e70cddc45efa124ba00f0dcac90b3d5f)
- **(file)** create dir before moving by [@jdx](https://github.com/jdx) in [3af89a7](https://github.com/jdx/hk/commit/3af89a7631c09a1355a915df63e388dccb376e0a)
- **(git)** make clone static by [@jdx](https://github.com/jdx) in [c86e7ef](https://github.com/jdx/hk/commit/c86e7ef0f537000a892bbb7f253008ce816e4a8f)
- **(git)** make clone static by [@jdx](https://github.com/jdx) in [db92996](https://github.com/jdx/hk/commit/db92996a4a1de1d530d953a18bb3eec8f2845c7e)
- **(http)** create dir for download by [@jdx](https://github.com/jdx) in [a01a89b](https://github.com/jdx/hk/commit/a01a89b86fa88b586b38ee35e41278fba0d78260)
- bug with stashing unstaged files by [@jdx](https://github.com/jdx) in [#160](https://github.com/jdx/hk/pull/160)
- bug with <clx:flex> tags appearing when they should not by [@jdx](https://github.com/jdx) in [#161](https://github.com/jdx/hk/pull/161)
- lib.rs by [@jdx](https://github.com/jdx) in [3989dbc](https://github.com/jdx/hk/commit/3989dbcee21a08969199a2ebdceae9607366f291)
- use async reqwest by [@jdx](https://github.com/jdx) in [fd1487a](https://github.com/jdx/hk/commit/fd1487aba47d8c6821b87f70d2297475641c57eb)
- hash generics by [@jdx](https://github.com/jdx) in [e1130b1](https://github.com/jdx/hk/commit/e1130b10e7fff7c49740ed35af1d0355e05f91ec)
- chmod zip unarchiving by [@jdx](https://github.com/jdx) in [6001ae2](https://github.com/jdx/hk/commit/6001ae2774570a5c2699a86a4eb7abccdcae0465)
- windows compat by [@jdx](https://github.com/jdx) in [92e1771](https://github.com/jdx/hk/commit/92e1771b8806983e7ef6f2e5afd8be8904380ca1)
- statically link xz by [@jdx](https://github.com/jdx) in [36aa947](https://github.com/jdx/hk/commit/36aa9479a6a4081db475a4ca364b3dbe96f5e128)
- clone options by [@acesyde](https://github.com/acesyde) in [462e4c7](https://github.com/jdx/hk/commit/462e4c766da63af5c398749df665dbd06450e39d)
- add stub make_executable for windows by [@jdx](https://github.com/jdx) in [5bbf842](https://github.com/jdx/hk/commit/5bbf84266030f81060cde35a39095fb7e2c0d5c6)
- make cli name customizable by [@jdx](https://github.com/jdx) in [80e57fa](https://github.com/jdx/hk/commit/80e57fa213622a4f53249c3036b0cc4c26f25830)
- indent with newlines by [@jdx](https://github.com/jdx) in [fb73cfb](https://github.com/jdx/hk/commit/fb73cfb858626d6641a634bc434f526d62cddd71)
- newline width calculation by [@jdx](https://github.com/jdx) in [ba1af54](https://github.com/jdx/hk/commit/ba1af54ab895c0f8837281d9c6a9edb0c1ee56ce)
- immediately render on done by [@jdx](https://github.com/jdx) in [638b42b](https://github.com/jdx/hk/commit/638b42b098fce71a1f5aa61b7721fcefed7c387f)
- use colors when tty by [@jdx](https://github.com/jdx) in [f1087c0](https://github.com/jdx/hk/commit/f1087c0e6097e41ed937bee739fa3c8f951d99bd)
- prevent waiting endlessly by [@jdx](https://github.com/jdx) in [1d407ea](https://github.com/jdx/hk/commit/1d407eadc576c0404c09bcdb5d2283309bb4016e)
- bug with flush by [@jdx](https://github.com/jdx) in [1502af5](https://github.com/jdx/hk/commit/1502af550cd81887990a27521b244ae647cf8453)
- remove newline when no output by [@jdx](https://github.com/jdx) in [7a6677c](https://github.com/jdx/hk/commit/7a6677c7cb5854d6a95b191c0eb1e633572fdd0c)
- bug by [@jdx](https://github.com/jdx) in [1196444](https://github.com/jdx/hk/commit/1196444ed276d7b54e9f01c7d7ff974d0bcee846)
- ui by [@jdx](https://github.com/jdx) in [082c80d](https://github.com/jdx/hk/commit/082c80da90b2337dd81806128dd9b6da9daaead1)
- truncate by [@jdx](https://github.com/jdx) in [113d944](https://github.com/jdx/hk/commit/113d94427351b9403f99a27928526baafc3aa41a)
- ellipsis by [@jdx](https://github.com/jdx) in [a7d14b1](https://github.com/jdx/hk/commit/a7d14b150b174dae653a3aa6ff73c4b3ac7141f8)
- typo by [@jdx](https://github.com/jdx) in [9f773ca](https://github.com/jdx/hk/commit/9f773caccf7eff18aa1d226c9b3e8b5edbabc3bf)
- allow setting body and tweaked default interval by [@jdx](https://github.com/jdx) in [a2c9acb](https://github.com/jdx/hk/commit/a2c9acbb218a001181fc922b77689a4316efe85d)
- hanging on exit by [@jdx](https://github.com/jdx) in [daaa54c](https://github.com/jdx/hk/commit/daaa54cf02d1900d6df7486d9013aca7a69b1167)
- prevent overflows by [@jdx](https://github.com/jdx) in [a65700e](https://github.com/jdx/hk/commit/a65700e2133a3345f5da24e975139256aa960234)
- bug by [@jdx](https://github.com/jdx) in [95b8509](https://github.com/jdx/hk/commit/95b8509d08d12728c1467bdf3ea46b84589dac9b)
- bug with flush by [@jdx](https://github.com/jdx) in [09c385d](https://github.com/jdx/hk/commit/09c385d7878677660033090c2d57c87a00e1d48f)
- issue with flex truncation by [@jdx](https://github.com/jdx) in [9d513db](https://github.com/jdx/hk/commit/9d513dbb5de446bac5cddb879b5b212071256084)
- clx not stopping correctly by [@jdx](https://github.com/jdx) in [787f18f](https://github.com/jdx/hk/commit/787f18ff5cbc3f09ad83eb768eaac927324ba73a)
- bug with stashing unstaged files by [@jdx](https://github.com/jdx) in [94bef2b](https://github.com/jdx/hk/commit/94bef2b4c9424f5f9dd385f7676f5bb960b11eb0)
- bug with <clx:flex> tags appearing when they should not by [@jdx](https://github.com/jdx) in [1026ba0](https://github.com/jdx/hk/commit/1026ba00b16e5d0b8cf3e5eba50245eb32316e4e)

### 🚜 Refactor

- refactor by [@jdx](https://github.com/jdx) in [49c7391](https://github.com/jdx/hk/commit/49c7391fc59a215a09b33073b2625c5e2a6abdd7)
- refactor by [@jdx](https://github.com/jdx) in [7511fd5](https://github.com/jdx/hk/commit/7511fd5501dfc1ae5bd99e20653078daa57fe6c7)

### 📚 Documentation

- added CHANGELOG by [@jdx](https://github.com/jdx) in [7686b61](https://github.com/jdx/hk/commit/7686b616f0c988157da93d6781280def3086bc3c)

### 🧪 Testing

- enable logging in unit tests by [@jdx](https://github.com/jdx) in [8816a36](https://github.com/jdx/hk/commit/8816a36e23517998a02a04498694083aa9cdb52b)
- enable trace logging by [@jdx](https://github.com/jdx) in [c822c89](https://github.com/jdx/hk/commit/c822c89102fc0da975902a5ac016e0b29916c202)
- show coverage results in action output by [@jdx](https://github.com/jdx) in [e5cbbf6](https://github.com/jdx/hk/commit/e5cbbf6175e1b157c29e8af64eaaeba179aaa684)
- harden tests by [@jdx](https://github.com/jdx) in [c06063e](https://github.com/jdx/hk/commit/c06063e1aecc1719b87d44aee0bff9715dd9dd2a)

### 🔍 Other Changes

- **(hash)** debug logging by [@jdx](https://github.com/jdx) in [15ee85f](https://github.com/jdx/hk/commit/15ee85f9735bd27d6e0eb83b4d05fb24bd6074b1)
- add some trace logs to prevent flex tags by [@jdx](https://github.com/jdx) in [950bfe1](https://github.com/jdx/hk/commit/950bfe14329e6c1f4433544e8a436b364fcc175e)
- init by [@jdx](https://github.com/jdx) in [3394ee3](https://github.com/jdx/hk/commit/3394ee3eedcdc6c0d5f8fb512f42f4b9bdb0f621)
- added renovate by [@jdx](https://github.com/jdx) in [9e082bc](https://github.com/jdx/hk/commit/9e082bcde2ead25b42bf7739742ba5e432e8f5ad)
- added renovate by [@jdx](https://github.com/jdx) in [7be5bd8](https://github.com/jdx/hk/commit/7be5bd867c974f36c55f646e79264d5c0ce6ac88)
- wip by [@jdx](https://github.com/jdx) in [ee4b38a](https://github.com/jdx/hk/commit/ee4b38aa3e6e79992886bb4c6f5ed1c13749c684)
- added test mutex by [@jdx](https://github.com/jdx) in [159916e](https://github.com/jdx/hk/commit/159916e4a91b542f1d9fc3d080c1166846a01c50)
- added test mutex by [@jdx](https://github.com/jdx) in [5c4038a](https://github.com/jdx/hk/commit/5c4038a31823d5d6b34c90b9d9bc83c857b4323e)
- wip by [@jdx](https://github.com/jdx) in [85ccbd7](https://github.com/jdx/hk/commit/85ccbd76948f6812bfa48c9512e03f3399036f3e)
- wip by [@jdx](https://github.com/jdx) in [1e3871d](https://github.com/jdx/hk/commit/1e3871d434314e4bb81078fef98e66c7387166c4)
- wip by [@jdx](https://github.com/jdx) in [8990ca9](https://github.com/jdx/hk/commit/8990ca9dfeca1a2827c592b4858aaee1936039d0)
- wip by [@jdx](https://github.com/jdx) in [09f315a](https://github.com/jdx/hk/commit/09f315ae744c8ba314914a9f30decfd675b6b418)
- wip by [@jdx](https://github.com/jdx) in [bd50579](https://github.com/jdx/hk/commit/bd50579bb05f7d01c37faad32bf268ca5976e024)
- added regex by [@jdx](https://github.com/jdx) in [02951f5](https://github.com/jdx/hk/commit/02951f5f3409ee9cbb05142849b5c9b0691c66aa)
- keep miette@5 for now by [@jdx](https://github.com/jdx) in [f4b8cf0](https://github.com/jdx/hk/commit/f4b8cf088c0d154e14c96dcf77d7cc805e147c50)
- added process by [@jdx](https://github.com/jdx) in [45ec479](https://github.com/jdx/hk/commit/45ec4799dfc5bd198d4c824accc19aa89c89cfc9)
- wip by [@jdx](https://github.com/jdx) in [a328fd3](https://github.com/jdx/hk/commit/a328fd333f0676d7b1cf218d74c4934c9834d7a5)
- wip by [@jdx](https://github.com/jdx) in [54a1e7c](https://github.com/jdx/hk/commit/54a1e7cf72afd2982cd6806752658d91f1e44d9c)
- wip by [@jdx](https://github.com/jdx) in [4962f3c](https://github.com/jdx/hk/commit/4962f3c26ad86e77c9b9770edbbc7b2d9c3849d9)
- wip by [@jdx](https://github.com/jdx) in [f144cb8](https://github.com/jdx/hk/commit/f144cb886a18e3d4f76fc21c8c0279dba6262e25)
- wip by [@jdx](https://github.com/jdx) in [4e1c886](https://github.com/jdx/hk/commit/4e1c8867d81d13ea8b47d34b69ae1c3614867bf6)
- Merge pull request #5 from jdx/renovate/lock-file-maintenance by [@jdx](https://github.com/jdx) in [9453405](https://github.com/jdx/hk/commit/9453405db24e79f91890087ed5a74fe5fbf7267c)
- Merge pull request #9 from jdx/git by [@jdx](https://github.com/jdx) in [21e4030](https://github.com/jdx/hk/commit/21e403007c64e5b84c657188f86e2e9c23fbc2b4)
- added coverage by [@jdx](https://github.com/jdx) in [b717528](https://github.com/jdx/hk/commit/b7175281fd9f7da22afc800586d13773a056a5de)
- updated deps by [@jdx](https://github.com/jdx) in [f9a6669](https://github.com/jdx/hk/commit/f9a66699c83a11cc335e6045c39c2e33f92f7b90)
- updated deps by [@jdx](https://github.com/jdx) in [4af5ffa](https://github.com/jdx/hk/commit/4af5fface15b3d6a1ec3a51530b173e894b3c437)
- upgraded miette by [@jdx](https://github.com/jdx) in [fac2714](https://github.com/jdx/hk/commit/fac2714d643fbb1f365aa987c9e2027f323c8190)
- add hash as dependency of fslock by [@jdx](https://github.com/jdx) in [6ddc9f8](https://github.com/jdx/hk/commit/6ddc9f8354dcc2acaf3d5984520b33035e18b04f)
- rename fslock struct by [@jdx](https://github.com/jdx) in [da4efd3](https://github.com/jdx/hk/commit/da4efd3924f819a97a754d5041ef0595dc0c1128)
- rename fslock struct by [@jdx](https://github.com/jdx) in [be0c21a](https://github.com/jdx/hk/commit/be0c21a0a94560068af2cdc6a17e459ab3a9cb7e)
- must_use by [@jdx](https://github.com/jdx) in [3e189d8](https://github.com/jdx/hk/commit/3e189d835e99a97894c2214e10c2d9a61dc2a0af)
- lint issue by [@jdx](https://github.com/jdx) in [a1034e8](https://github.com/jdx/hk/commit/a1034e8effd4f1f529a83196672068e3bc1339b5)
- updated deps by [@jdx](https://github.com/jdx) in [37bb396](https://github.com/jdx/hk/commit/37bb396c2ba60eca3a46763e1330be209cc83540)
- Create LICENSE by [@jdx](https://github.com/jdx) in [39ef7b7](https://github.com/jdx/hk/commit/39ef7b7e4219d675fac481c6be86427ec21ed155)
- cargo up by [@jdx](https://github.com/jdx) in [8b642d8](https://github.com/jdx/hk/commit/8b642d8d8f247a3bb50c35700ae38a5d2b221655)
- cargo up by [@jdx](https://github.com/jdx) in [52f46ae](https://github.com/jdx/hk/commit/52f46ae76323b8336ade3f9bc55f52694952eeb9)
- always save cache by [@jdx](https://github.com/jdx) in [6914022](https://github.com/jdx/hk/commit/6914022392c4b8966659c7e982c014d05d61523d)
- set cargo include by [@jdx](https://github.com/jdx) in [9054498](https://github.com/jdx/hk/commit/9054498e198219d901ae4339debb83acc1513ddd)
- updated deps by [@jdx](https://github.com/jdx) in [b999c45](https://github.com/jdx/hk/commit/b999c45d06a83f2931841434a0e19577ac60e22c)
- fix cargo includes by [@jdx](https://github.com/jdx) in [0aa2b11](https://github.com/jdx/hk/commit/0aa2b1103214e6e2712faeac318e35b43240ef28)
- bump deps by [@jdx](https://github.com/jdx) in [7d09dee](https://github.com/jdx/hk/commit/7d09dee1112bdf7e9a19f33478c5c48e6a845121)
- remove -x by [@jdx](https://github.com/jdx) in [6170a27](https://github.com/jdx/hk/commit/6170a27a296a6bf97c7c11e85ae7e2b743e6e94c)
- added hk by [@jdx](https://github.com/jdx) in [3de8165](https://github.com/jdx/hk/commit/3de8165dc58f22bce0d03106642a079ebf20d310)
- added pkl by [@jdx](https://github.com/jdx) in [7da29a1](https://github.com/jdx/hk/commit/7da29a19b5c7757acfa2fa8cdc61fe370b10c081)
- updated deps by [@jdx](https://github.com/jdx) in [26d1e42](https://github.com/jdx/hk/commit/26d1e42f589d9d9436b51159472fcbb226b2d610)
- Update hk to v1.2.2 and fix clippy warnings by [@jdx](https://github.com/jdx) in [c69ddf5](https://github.com/jdx/hk/commit/c69ddf5a0a06ee47ff6859f4ea89582d7519b5d3)
- Enhance README with comprehensive documentation by [@jdx](https://github.com/jdx) in [96136de](https://github.com/jdx/hk/commit/96136dee9bf96498b44c45b9c9ccc7f2811c35f6)
- pin homedir to 0.3.5 for MSRV compatibility by [@jdx](https://github.com/jdx) in [d684d9c](https://github.com/jdx/hk/commit/d684d9c1f7c2fc9771797c6e316c77f28439b32a)
- cargo up by [@jdx](https://github.com/jdx) in [1254b65](https://github.com/jdx/hk/commit/1254b651730db5cf6e61d3d0b6ceebe86e8e5f8b)
- Merge remote-tracking branch 'xx/main' by [@jdx](https://github.com/jdx) in [7a08655](https://github.com/jdx/hk/commit/7a0865560bbceb900f3f2d034a59572215eca2a3)
- init by [@jdx](https://github.com/jdx) in [2846add](https://github.com/jdx/hk/commit/2846adda2803851e1c4f9d8dcc569b1ff35cdc8a)
- fix arc by [@jdx](https://github.com/jdx) in [0fbef92](https://github.com/jdx/hk/commit/0fbef92597b248e2c9c3363dc4cd532b0f86f546)
- downgrade edition by [@jdx](https://github.com/jdx) in [af773e2](https://github.com/jdx/hk/commit/af773e2195a8f12ee6ca5ccf3f6aaab69762676a)
- fix compilation on windows by [@jdx](https://github.com/jdx) in [d019567](https://github.com/jdx/hk/commit/d019567935f73106c51e389ab89cbb4bc6f7d8d7)
- v2 api by [@jdx](https://github.com/jdx) in [e456c39](https://github.com/jdx/hk/commit/e456c398d376548396896105cd6c22372fda4e42)
- added builder by [@jdx](https://github.com/jdx) in [7f56868](https://github.com/jdx/hk/commit/7f5686814e21dbbe9af79dfd8a5aa0542ac17dfc)
- wip by [@jdx](https://github.com/jdx) in [deed786](https://github.com/jdx/hk/commit/deed786648aa82c0deb18398c3f1cc7ac905b0ec)
- wip by [@jdx](https://github.com/jdx) in [09c1bbb](https://github.com/jdx/hk/commit/09c1bbb6c97fd10e9602d5e16a6b8f6c7a1225c8)
- wip by [@jdx](https://github.com/jdx) in [c5490e5](https://github.com/jdx/hk/commit/c5490e57d51bc82c334b26b3d2a4dac89bff8587)
- wip by [@jdx](https://github.com/jdx) in [24f1d78](https://github.com/jdx/hk/commit/24f1d78cffa923d307e5c9767be271c971afe8fb)
- wip by [@jdx](https://github.com/jdx) in [20cf15f](https://github.com/jdx/hk/commit/20cf15f5a549e83e2752c587aa52b4d15b617a57)
- wip by [@jdx](https://github.com/jdx) in [7a113c0](https://github.com/jdx/hk/commit/7a113c0ac92172094bfd58c695146f4485b79a53)
- tweak ui with pending by [@jdx](https://github.com/jdx) in [88f38e0](https://github.com/jdx/hk/commit/88f38e0b98be1afd47784d26bf9a56cffd38716f)
- updated deps by [@jdx](https://github.com/jdx) in [e75148f](https://github.com/jdx/hk/commit/e75148f994db5691a2f7dab70d9095f55f4af374)
- cargo fmt by [@jdx](https://github.com/jdx) in [32dd2ee](https://github.com/jdx/hk/commit/32dd2ee5aece134fdf1f0e5db8970d0a6355b8ef)
- ci by [@jdx](https://github.com/jdx) in [b5728c4](https://github.com/jdx/hk/commit/b5728c447d6cd743e0c0ad2c4961e25118eb267e)
- msrv by [@jdx](https://github.com/jdx) in [e8aa4d1](https://github.com/jdx/hk/commit/e8aa4d16a2ad8a43c169f29437c2f75735c8dcc2)
- bump deps by [@jdx](https://github.com/jdx) in [ce3e04f](https://github.com/jdx/hk/commit/ce3e04f2070d71d19c5d45850082c3f2785bca5f)
- add some trace logs to prevent flex tags by [@jdx](https://github.com/jdx) in [e183500](https://github.com/jdx/hk/commit/e1835004e75b83ea254478a0afd8f39faeb7d709)
- Merge remote-tracking branch 'clx/main' by [@jdx](https://github.com/jdx) in [80638a0](https://github.com/jdx/hk/commit/80638a0b732541aace88766b32c16169821f5f2a)
- cargo build by [@jdx](https://github.com/jdx) in [f4eb5a3](https://github.com/jdx/hk/commit/f4eb5a3a254a39c85fded92acf7fa8e4326e45e5)
- pull subtrees by [@jdx](https://github.com/jdx) in [abe3cc1](https://github.com/jdx/hk/commit/abe3cc15180e4d13c17d81566bd23e01dbdc4c7f)

### 📦️ Dependency Updates

- update actions/checkout action to v4 by [@renovate[bot]](https://github.com/renovate[bot]) in [6a0fa0f](https://github.com/jdx/hk/commit/6a0fa0f1d9ec5f3d4d18dd38eeb487519b81952f)
- update rust crate miette to v6 by [@renovate[bot]](https://github.com/renovate[bot]) in [1f70021](https://github.com/jdx/hk/commit/1f70021a717f6d859bfc8ca9e0147a85b9f41b69)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [310bfb5](https://github.com/jdx/hk/commit/310bfb5c3121f92659afc22d1c0dec8b4867bb61)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [bd0ea32](https://github.com/jdx/hk/commit/bd0ea3231a32a7edb1e80e83f6490909a4affd49)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [2be2f99](https://github.com/jdx/hk/commit/2be2f996659c786d9c2441c944de1d924afc1ba3)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [cd92e4d](https://github.com/jdx/hk/commit/cd92e4dd3e36b0153380c01c6f3b9b886d4956fb)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [ed553a2](https://github.com/jdx/hk/commit/ed553a2b1db8cce9c64e2889147c55cc0d5a32b8)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [f85ab52](https://github.com/jdx/hk/commit/f85ab52ad70c3ea196ae18f21550187f6a36dd36)
- update rust crate thiserror to v1.0.60 by [@renovate[bot]](https://github.com/renovate[bot]) in [050eb7a](https://github.com/jdx/hk/commit/050eb7a131cb7c2c2eb9b71f999ab5166350f8e7)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [55f3903](https://github.com/jdx/hk/commit/55f3903c1e98b98607992d5adc9d4a969b33ad92)
- update rust crate thiserror to v1.0.61 by [@renovate[bot]](https://github.com/renovate[bot]) in [34de9af](https://github.com/jdx/hk/commit/34de9af29d9b7f65c398cfd4d0e993a5dfddba09)
- update rust crate insta to v1.39.0 by [@renovate[bot]](https://github.com/renovate[bot]) in [efff934](https://github.com/jdx/hk/commit/efff934023c71d79abf138055f6d69b477a32e27)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [9fbbb25](https://github.com/jdx/hk/commit/9fbbb258d7e0f3aaabc12da1dc0352cbd250a467)
- update rust crate zip to v2.1.2 by [@renovate[bot]](https://github.com/renovate[bot]) in [8486a44](https://github.com/jdx/hk/commit/8486a44c5af5d2ea976953aa44b792c1377aec52)
- update rust crate tokio to v1.38.0 by [@renovate[bot]](https://github.com/renovate[bot]) in [2a35642](https://github.com/jdx/hk/commit/2a35642a6e09a609cb48356b88cbecd12775c60d)
- update rust crate regex to v1.10.5 by [@renovate[bot]](https://github.com/renovate[bot]) in [b8f4a98](https://github.com/jdx/hk/commit/b8f4a98070270f01036e90faa4f05256a641fe3e)
- update rust crate tar to v0.4.41 by [@renovate[bot]](https://github.com/renovate[bot]) in [6ec1ed3](https://github.com/jdx/hk/commit/6ec1ed361d5c391bce45cda8d540c2daaf2bccd6)
- update rust crate zip to v2.1.3 by [@renovate[bot]](https://github.com/renovate[bot]) in [1a914b1](https://github.com/jdx/hk/commit/1a914b13f3d87ad64314659a5278f021e5b8d2bb)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [480b146](https://github.com/jdx/hk/commit/480b146e480360cfe2cf06bf3d1bc106ef933edc)
- update rust crate reqwest to v0.12.5 by [@renovate[bot]](https://github.com/renovate[bot]) in [d587a95](https://github.com/jdx/hk/commit/d587a954fc97fbb9349c130632c4ab6b5c564b86)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [d8015c8](https://github.com/jdx/hk/commit/d8015c8787d009de1b6d50f931970daf87d05165)
- update rust crate log to v0.4.22 by [@renovate[bot]](https://github.com/renovate[bot]) in [09db21e](https://github.com/jdx/hk/commit/09db21e7fdee794e19bcddbc4db959d9219800e7)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [ae2042d](https://github.com/jdx/hk/commit/ae2042d9b6a7ca81a0795f253d3fd4c0a2b1b5dd)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [483615b](https://github.com/jdx/hk/commit/483615b1509d7baa9852b485033687f233f2c6f8)
- update rust crate thiserror to v1.0.62 by [@renovate[bot]](https://github.com/renovate[bot]) in [114479f](https://github.com/jdx/hk/commit/114479f5cc25d9ac3ecb89612679a500a4b87015)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [ef7d03e](https://github.com/jdx/hk/commit/ef7d03e59a2f734f43c9f77344a2e4a493a695f1)
- update rust crate thiserror to v1.0.63 by [@renovate[bot]](https://github.com/renovate[bot]) in [864e4eb](https://github.com/jdx/hk/commit/864e4ebd062300528d0245c2c0eb9cdbd1eafa67)
- update rust crate tokio to v1.38.1 by [@renovate[bot]](https://github.com/renovate[bot]) in [861fc6e](https://github.com/jdx/hk/commit/861fc6e8d81063a0062873b43d87c27461b99269)
- update rust crate zip to v2.1.5 by [@renovate[bot]](https://github.com/renovate[bot]) in [7705d6d](https://github.com/jdx/hk/commit/7705d6de94c074b952719c220f4838414b31ba6c)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [559a5b9](https://github.com/jdx/hk/commit/559a5b95e7a1cfd8eaf64dcd458a6ed9a2dd368f)
- update rust crate env_logger to v0.11.5 by [@renovate[bot]](https://github.com/renovate[bot]) in [0fab118](https://github.com/jdx/hk/commit/0fab1185cd663768a262f4ed2b42364c5c3c66f3)
- update rust crate tokio to v1.39.2 by [@renovate[bot]](https://github.com/renovate[bot]) in [e87b0a9](https://github.com/jdx/hk/commit/e87b0a96c00a7de0be7249d109a74f85a507b6ec)
- lock file maintenance by [@renovate[bot]](https://github.com/renovate[bot]) in [016724c](https://github.com/jdx/hk/commit/016724c80145f73c618faa61dcfddc62e6d1d243)
- update rust crate flate2 to v1.0.31 by [@renovate[bot]](https://github.com/renovate[bot]) in [31f950f](https://github.com/jdx/hk/commit/31f950f8dd3d675921749c8479ffd513a300e368)
- update rust crate regex to v1.10.6 by [@renovate[bot]](https://github.com/renovate[bot]) in [19c16e4](https://github.com/jdx/hk/commit/19c16e4e3b65eb25c6a3318a3cd82acbf1326930)
- update rust crate filetime to v0.2.24 by [@renovate[bot]](https://github.com/renovate[bot]) in [1cc311d](https://github.com/jdx/hk/commit/1cc311decd62a257468ffcaea881e559e06053a4)
- update rust crate zip to v2.1.6 by [@renovate[bot]](https://github.com/renovate[bot]) in [da4dc9e](https://github.com/jdx/hk/commit/da4dc9e1b1be22291df431e8bf86cba2003c25e3)
- update rust crate flate2 to v1.0.33 by [@renovate[bot]](https://github.com/renovate[bot]) in [e0307db](https://github.com/jdx/hk/commit/e0307dbd75dfd860196fc36468eb4f1fac33e70c)
- update rust crate reqwest to v0.12.7 by [@renovate[bot]](https://github.com/renovate[bot]) in [efb508e](https://github.com/jdx/hk/commit/efb508e915081d9f126bff5070b29271ea2fe985)
- update rust crate pretty_assertions to v1.4.1 by [@renovate[bot]](https://github.com/renovate[bot]) in [443d59e](https://github.com/jdx/hk/commit/443d59ee805676bc03cd85d7e6e1bd0cfc602c2d)
- update rust crate filetime to v0.2.25 by [@renovate[bot]](https://github.com/renovate[bot]) in [6703634](https://github.com/jdx/hk/commit/67036349270bcefd64f4d2ee7b9415250871d1b2)
- update rust crate flate2 to v1.0.34 by [@renovate[bot]](https://github.com/renovate[bot]) in [ee64839](https://github.com/jdx/hk/commit/ee6483983d1c279fed3d7b11c5c563732a4fcf35)
- update rust crate tar to v0.4.42 by [@renovate[bot]](https://github.com/renovate[bot]) in [b9143b9](https://github.com/jdx/hk/commit/b9143b97874c3062fa9c3cf009481f85981ad287)
- update rust crate homedir to v0.3.4 by [@renovate[bot]](https://github.com/renovate[bot]) in [ebc99c5](https://github.com/jdx/hk/commit/ebc99c5b77f91572cb498d37ea52419006edc181)
- update rust crate reqwest to v0.12.8 by [@renovate[bot]](https://github.com/renovate[bot]) in [d460622](https://github.com/jdx/hk/commit/d46062296275cdfd11717b3cbdb7c297bcc3c2e4)
- update rust crate bzip2 to 0.5 by [@renovate[bot]](https://github.com/renovate[bot]) in [7c6d784](https://github.com/jdx/hk/commit/7c6d7843d94a018b51db0e5d12c16a5a77c30bdf)

### New Contributors

- @aniaan made their first contribution
- @acesyde made their first contribution
- @renovate[bot] made their first contribution

## [1.7.1](https://github.com/jdx/hk/compare/v1.7.0..v1.7.1) - 2025-08-18

### 🐛 Bug Fixes

- clx not stopping correctly by [@jdx](https://github.com/jdx) in [#158](https://github.com/jdx/hk/pull/158)
- clean up output for missing-profile warnings by [@jdx](https://github.com/jdx) in [#157](https://github.com/jdx/hk/pull/157)

### 🔍 Other Changes

- fix duplicate Builtins.pkl files by [@jdx](https://github.com/jdx) in [2785dc6](https://github.com/jdx/hk/commit/2785dc634c155cbec4e5d120fa12beaaf320aa68)

## [1.7.0](https://github.com/jdx/hk/compare/v1.6.0..v1.7.0) - 2025-08-17

### 🚀 Features

- make stderr display in progress bars configurable by [@jdx](https://github.com/jdx) in [#155](https://github.com/jdx/hk/pull/155)
- cancellations by [@jdx](https://github.com/jdx) in [700d252](https://github.com/jdx/hk/commit/700d2522453537db949b513ba01738e74bd880e6)
- convert xx, clx, and ensembler to git subtrees by [@jdx](https://github.com/jdx) in [3a7b6be](https://github.com/jdx/hk/commit/3a7b6be02a26289fb12c37eb9f57057278713e7e)
- make stderr display in progress bars configurable by [@jdx](https://github.com/jdx) in [b3e7b2e](https://github.com/jdx/hk/commit/b3e7b2e06bf68d40bec8ec81053c3676b7e39767)

### 🐛 Bug Fixes

- properly display aborted steps when fail-fast triggers by [@jdx](https://github.com/jdx) in [#154](https://github.com/jdx/hk/pull/154)
- stdout/stderr output by [@jdx](https://github.com/jdx) in [e10f19d](https://github.com/jdx/hk/commit/e10f19dd69e93c2e704cccf94c38060d66342df1)
- return result information in error by [@jdx](https://github.com/jdx) in [5ae9b15](https://github.com/jdx/hk/commit/5ae9b151869a0976da5455ee0487c78fc153b1bc)
- clx v2 by [@jdx](https://github.com/jdx) in [0eefaa2](https://github.com/jdx/hk/commit/0eefaa21944f5175bed94d2dc7f31cfccd95831f)
- clx v2 by [@jdx](https://github.com/jdx) in [511732b](https://github.com/jdx/hk/commit/511732b0fabfc0ead026509a7caf1579600f6c05)
- added combined_output to result by [@jdx](https://github.com/jdx) in [c5ab404](https://github.com/jdx/hk/commit/c5ab404092c36f9c30a42f096f23622c0c2e7cdd)
- errexit trimming by [@jdx](https://github.com/jdx) in [d3942bf](https://github.com/jdx/hk/commit/d3942bf5f6c81dbb77e1f9fd6122f922b0c5d785)
- ensure stdin/out/err gets flushed by [@jdx](https://github.com/jdx) in [43fcea0](https://github.com/jdx/hk/commit/43fcea0058d9254f05d2dde3668a2e959065cd29)

### 🧪 Testing

- fix example by [@jdx](https://github.com/jdx) in [d5b8475](https://github.com/jdx/hk/commit/d5b8475c4713d6466f9ef288b826f7bd8de55886)

### 🔍 Other Changes

- init by [@jdx](https://github.com/jdx) in [7cf57f9](https://github.com/jdx/hk/commit/7cf57f9898103c47db37df6b5a2acd71055f61c4)
- async by [@jdx](https://github.com/jdx) in [79a0b11](https://github.com/jdx/hk/commit/79a0b118861201e06249186340ee1a34ce901215)
- move ui stuff to clx crate by [@jdx](https://github.com/jdx) in [474658a](https://github.com/jdx/hk/commit/474658adda35db9df515e6c210e1177e8037b3e5)
- remove clx exports by [@jdx](https://github.com/jdx) in [dfd2aed](https://github.com/jdx/hk/commit/dfd2aedf1788c9cbcc81ab89834cf0831f942573)
- wip by [@jdx](https://github.com/jdx) in [1a4d885](https://github.com/jdx/hk/commit/1a4d885c7c9119f4f4e8e6ad98d82a11b2b031c5)
- fix compilation on windows by [@jdx](https://github.com/jdx) in [ea22c95](https://github.com/jdx/hk/commit/ea22c95b54427cc9bb94450036903f64f2aa143e)
- clx v2 by [@jdx](https://github.com/jdx) in [befab5a](https://github.com/jdx/hk/commit/befab5ab29d60832e08dd30305fd4997c33ebe0f)
- clx v2 by [@jdx](https://github.com/jdx) in [293a5b2](https://github.com/jdx/hk/commit/293a5b28929af56bae93267285f08f62ab697a91)
- use message by [@jdx](https://github.com/jdx) in [56ec6a9](https://github.com/jdx/hk/commit/56ec6a961c948dbff09602727d5078061108c2af)
- wip by [@jdx](https://github.com/jdx) in [f77cdd4](https://github.com/jdx/hk/commit/f77cdd47a7bec5f380874d23f750c8ad219f1f00)
- wip by [@jdx](https://github.com/jdx) in [588cb64](https://github.com/jdx/hk/commit/588cb64f46822b5fd476c6af877b35c00c0af641)
- stdout by [@jdx](https://github.com/jdx) in [f255e1c](https://github.com/jdx/hk/commit/f255e1c26cec4e452dc07f0c0992d952212ec6d3)
- fix example by [@jdx](https://github.com/jdx) in [24e2be9](https://github.com/jdx/hk/commit/24e2be9c258a93fcdcb2e41dfa415e01bd756b63)
- box up ScriptFailed by [@jdx](https://github.com/jdx) in [768d991](https://github.com/jdx/hk/commit/768d99128375eae1afd3992d515f77dc0c5ffbbc)
- ci by [@jdx](https://github.com/jdx) in [47b9515](https://github.com/jdx/hk/commit/47b9515a9628df99360a862080f55363387c3767)
- set cargo includes by [@jdx](https://github.com/jdx) in [80ef27b](https://github.com/jdx/hk/commit/80ef27be0135ec82faddb5f2b056f178643ecfb9)
- bump deps by [@jdx](https://github.com/jdx) in [587eb15](https://github.com/jdx/hk/commit/587eb15c1027cf757834e27b410c861edcdc57eb)
- Merge remote-tracking branch 'ensembler/main' by [@jdx](https://github.com/jdx) in [340d0de](https://github.com/jdx/hk/commit/340d0de673297c0003ab842e49d148ef994b7b0b)

## [1.6.0](https://github.com/jdx/hk/compare/v1.5.0..v1.6.0) - 2025-08-16

### 🚀 Features

- add interactive field to JSON timing report by [@jdx](https://github.com/jdx) in [#143](https://github.com/jdx/hk/pull/143)
- added report property to hooks by [@jdx](https://github.com/jdx) in [#145](https://github.com/jdx/hk/pull/145)
- display message when steps are skipped by [@jdx](https://github.com/jdx) in [#146](https://github.com/jdx/hk/pull/146)
- convert xx, clx, and ensembler to git subtrees by [@jdx](https://github.com/jdx) in [aa87ee3](https://github.com/jdx/hk/commit/aa87ee35b1d81277212253070865f06fc06b0a87)
- add configurable skip message visibility by [@jdx](https://github.com/jdx) in [#151](https://github.com/jdx/hk/pull/151)
- allow hiding warnings by [@jdx](https://github.com/jdx) in [#153](https://github.com/jdx/hk/pull/153)

### 🐛 Bug Fixes

- libgit2 issue with merging stash diffs by [@jdx](https://github.com/jdx) in [8e1de78](https://github.com/jdx/hk/commit/8e1de7808b0c46b8d0d3ed99bb99be3ac26f672a)
- bug with truncation of progress by [@jdx](https://github.com/jdx) in [5c81a0c](https://github.com/jdx/hk/commit/5c81a0c2161ad4f622889167bae16f7e25230d59)
- update release-plz script for git subtrees by [@jdx](https://github.com/jdx) in [b55134c](https://github.com/jdx/hk/commit/b55134c8c5081b57f93b049f59f6e85a32fb9a39)
- vendor libgit2 by [@jdx](https://github.com/jdx) in [251ff96](https://github.com/jdx/hk/commit/251ff96959cb0934504ec6dc7c31a05e427e793f)

### 🔍 Other Changes

- **(mise)** restructure tasks, add dev alias, and parallelize bats tests by [@jdx](https://github.com/jdx) in [#150](https://github.com/jdx/hk/pull/150)
- update cargo deps by [@jdx](https://github.com/jdx) in [#148](https://github.com/jdx/hk/pull/148)
- update cargo deps by [@jdx](https://github.com/jdx) in [#149](https://github.com/jdx/hk/pull/149)
- mise bump by [@jdx](https://github.com/jdx) in [#147](https://github.com/jdx/hk/pull/147)
- mise.lock by [@jdx](https://github.com/jdx) in [b14839b](https://github.com/jdx/hk/commit/b14839b5fbe33d02fe6799fc6fdef1973a080ed7)
- add CLAUDE.md by [@jdx](https://github.com/jdx) in [f2a5083](https://github.com/jdx/hk/commit/f2a5083ec49f4575719cc28b9a9d835f8ac75e3c)
- remove test fixture by [@jdx](https://github.com/jdx) in [564612d](https://github.com/jdx/hk/commit/564612df078990dbc88c9f0e70629cb1d58bff67)
- Squashed 'xx/' content from commit afb8132 by [@jdx](https://github.com/jdx) in [73d78f8](https://github.com/jdx/hk/commit/73d78f808aba44b2a11cf9603e37da97943fa90e)
- Squashed 'clx/' content from commit e9f476e by [@jdx](https://github.com/jdx) in [f51c062](https://github.com/jdx/hk/commit/f51c06227c297e7040078121804e19574f780b62)
- Squashed 'ensembler/' content from commit 25e12c4 by [@jdx](https://github.com/jdx) in [34433b1](https://github.com/jdx/hk/commit/34433b193d847baf0746ec12e60b85b550c0c4d8)
- pin homedir to 0.3.5 for MSRV compatibility by [@jdx](https://github.com/jdx) in [f7e1f42](https://github.com/jdx/hk/commit/f7e1f42d6b86215201a7f06b23014fff4dbe5856)
- fix release-plz with subtrees by [@jdx](https://github.com/jdx) in [965753e](https://github.com/jdx/hk/commit/965753e98ff3aad5bfcab1a92aab515cca900a69)
- fix release-plz with subtrees by [@jdx](https://github.com/jdx) in [21475e3](https://github.com/jdx/hk/commit/21475e3f3ce659b637d5982dd69158adb274b2d8)

## [1.5.0](https://github.com/jdx/hk/compare/v1.4.0..v1.5.0) - 2025-08-14

### 🚀 Features

- json5 support for prettier builtin by [@tpansino](https://github.com/tpansino) in [#136](https://github.com/jdx/hk/pull/136)

### 🐛 Bug Fixes

- elixir steps named properly by [@paradox460](https://github.com/paradox460) in [#140](https://github.com/jdx/hk/pull/140)
- pkl builtin by [@tpansino](https://github.com/tpansino) in [#138](https://github.com/jdx/hk/pull/138)
- make glob, exclude, and stage relative to dir by [@jdx](https://github.com/jdx) in [#142](https://github.com/jdx/hk/pull/142)

### 🔍 Other Changes

- Prettier *.yml and *.md files by [@tpansino](https://github.com/tpansino) in [#141](https://github.com/jdx/hk/pull/141)

### New Contributors

- @tpansino made their first contribution in [#141](https://github.com/jdx/hk/pull/141)

## [1.4.0](https://github.com/jdx/hk/compare/v1.3.0..v1.4.0) - 2025-08-12

### 🚀 Features

- timing json report by [@jdx](https://github.com/jdx) in [#134](https://github.com/jdx/hk/pull/134)

### 🔍 Other Changes

- render during release-plz by [@jdx](https://github.com/jdx) in [a19fff8](https://github.com/jdx/hk/commit/a19fff8a706ba53397b7e234ab3274c1ccad4bd1)
- add usage to mise.toml by [@jdx](https://github.com/jdx) in [844df64](https://github.com/jdx/hk/commit/844df647e63467cab3191bdde92fa592ed0637c9)

## [1.3.0](https://github.com/jdx/hk/compare/v1.2.2..v1.3.0) - 2025-08-12

### 🚀 Features

- **(stash)** prefer unstaged contents on conflict; add tests by [@jdx](https://github.com/jdx) in [#133](https://github.com/jdx/hk/pull/133)
- Pkl Builtin by [@paradox460](https://github.com/paradox460) in [#127](https://github.com/jdx/hk/pull/127)
- Elixir builtins by [@paradox460](https://github.com/paradox460) in [#128](https://github.com/jdx/hk/pull/128)

### 🐛 Bug Fixes

- unsupported stage syntax in builtins by [@atty303](https://github.com/atty303) in [#132](https://github.com/jdx/hk/pull/132)

### New Contributors

- @atty303 made their first contribution in [#132](https://github.com/jdx/hk/pull/132)
- @paradox460 made their first contribution in [#128](https://github.com/jdx/hk/pull/128)

## [1.2.2](https://github.com/jdx/hk/compare/v1.2.1..v1.2.2) - 2025-07-21

### 🐛 Bug Fixes

- resolve file path issues when steps have different dir settings by [@jdx](https://github.com/jdx) in [#126](https://github.com/jdx/hk/pull/126)

### 🧪 Testing

- refactored tests into individual files by [@jdx](https://github.com/jdx) in [666c921](https://github.com/jdx/hk/commit/666c9217e957723264238ac60f86fd8cb2384727)

## [1.2.1](https://github.com/jdx/hk/compare/v1.2.0..v1.2.1) - 2025-07-09

### 🐛 Bug Fixes

- Mismatched stylelint pkl export #121 by [@bashandbone](https://github.com/bashandbone) in [#122](https://github.com/jdx/hk/pull/122)
- lint issues by [@jdx](https://github.com/jdx) in [92117b7](https://github.com/jdx/hk/commit/92117b7c52b36ac3fd42334389dea55d497355fb)
- scope files by `workspace_indicator` by [@babariviere](https://github.com/babariviere) in [#123](https://github.com/jdx/hk/pull/123)

### 📚 Documentation

- fix pkl syntax in configuration.md by [@smasato](https://github.com/smasato) in [#119](https://github.com/jdx/hk/pull/119)
- added CONTRIBUTING.md by [@jdx](https://github.com/jdx) in [4b43c1e](https://github.com/jdx/hk/commit/4b43c1ea7319d63fd7f0c73d579ca9bba27e3522)

### 🔍 Other Changes

- added release-plz action by [@jdx](https://github.com/jdx) in [761f8ad](https://github.com/jdx/hk/commit/761f8ad46b66690017da64744ab07e20433cb209)
- added cargo-edit by [@jdx](https://github.com/jdx) in [05aca24](https://github.com/jdx/hk/commit/05aca2440e70d9435dfa18f9c233ece899a41c6d)

### New Contributors

- @babariviere made their first contribution in [#123](https://github.com/jdx/hk/pull/123)
- @smasato made their first contribution in [#119](https://github.com/jdx/hk/pull/119)
- @bashandbone made their first contribution in [#122](https://github.com/jdx/hk/pull/122)

## [1.2.0](https://github.com/jdx/hk/compare/v1.1.2..v1.2.0) - 2025-06-13

### 🚀 Features

- added --no-progress flag by [@jdx](https://github.com/jdx) in [ffef92f](https://github.com/jdx/hk/commit/ffef92f27afac702bf37423286ad9a75b4cc0fc7)
- Add support for .hkrc.pkl user configuration file by [@ckilpatrick-stay-hydrated-call-your-mom](https://github.com/ckilpatrick-stay-hydrated-call-your-mom) in [#117](https://github.com/jdx/hk/pull/117)

### 🐛 Bug Fixes

- disable clippy progress bar by [@jdx](https://github.com/jdx) in [bc10730](https://github.com/jdx/hk/commit/bc1073049de367d8d2cd94af107c0ea33e7ce508)
- skip file enumeration in pre-push when deleting remote branches by [@markjaquith](https://github.com/markjaquith) in [#110](https://github.com/jdx/hk/pull/110)

### 📚 Documentation

- replace references to deprecated `hk generate` with `hk init` by [@betaboon](https://github.com/betaboon) in [#116](https://github.com/jdx/hk/pull/116)

### 🔍 Other Changes

- only build and deploy docs on the main jdx/hk repo by [@markjaquith](https://github.com/markjaquith) in [#111](https://github.com/jdx/hk/pull/111)
- Update pkl_introduction.md by [@jdx](https://github.com/jdx) in [7dbdeda](https://github.com/jdx/hk/commit/7dbdedad36fd5ae0b32f083758335247b172ee05)

### New Contributors

- @ckilpatrick-stay-hydrated-call-your-mom made their first contribution in [#117](https://github.com/jdx/hk/pull/117)
- @betaboon made their first contribution in [#116](https://github.com/jdx/hk/pull/116)

## [1.1.2](https://github.com/jdx/hk/compare/v1.1.1..v1.1.2) - 2025-05-25

### 🐛 Bug Fixes

- filename escaping by [@jdx](https://github.com/jdx) in [#103](https://github.com/jdx/hk/pull/103)
- use git merge-base to find common ancestor by [@jdx](https://github.com/jdx) in [#108](https://github.com/jdx/hk/pull/108)
- batch process shfmt by [@jdx](https://github.com/jdx) in [a802506](https://github.com/jdx/hk/commit/a802506631db1304388b8ffda3166a632a39a49a)
- shfmt check_list_files by [@jdx](https://github.com/jdx) in [e8aca63](https://github.com/jdx/hk/commit/e8aca6309d459a92aadf299798551cb2334f3c8b)
- Add missing node dependency to mise.toml by [@matiashf](https://github.com/matiashf) in [#104](https://github.com/jdx/hk/pull/104)

### 🧪 Testing

- test for deleted files by [@jdx](https://github.com/jdx) in [4a854ca](https://github.com/jdx/hk/commit/4a854ca44b3905d2d5ceaaf3ec6c0c9ee67a8c87)

### 🔍 Other Changes

- check out full repo for changelog generation by [@jdx](https://github.com/jdx) in [bdef823](https://github.com/jdx/hk/commit/bdef823cbea14837fc03f8a8b42b5d99d2b71dda)
- bump deps by [@jdx](https://github.com/jdx) in [e857656](https://github.com/jdx/hk/commit/e8576567b4e8199796befe47972cf60b2948049c)
- bump deps by [@jdx](https://github.com/jdx) in [1e0d870](https://github.com/jdx/hk/commit/1e0d87085b96f5497225c9ea5c33a50acaaf140f)
- Update shellcheck.pkl by [@jdx](https://github.com/jdx) in [e67de40](https://github.com/jdx/hk/commit/e67de4099539ad25a283effb74e6fa09898a9d5b)

### New Contributors

- @matiashf made their first contribution in [#104](https://github.com/jdx/hk/pull/104)

## [1.1.1](https://github.com/jdx/hk/compare/v1.1.0..v1.1.1) - 2025-05-16

### 🐛 Bug Fixes

- _duplicate type json cache warning by [@jdx](https://github.com/jdx) in [#99](https://github.com/jdx/hk/pull/99)
- ensure unstaged/untracked changed are used with --all by [@jdx](https://github.com/jdx) in [#100](https://github.com/jdx/hk/pull/100)

### 🔍 Other Changes

- clippy by [@jdx](https://github.com/jdx) in [5823c5a](https://github.com/jdx/hk/commit/5823c5ac76cb6e8b71d5312c212f2b8a7e9ef04c)
- clippy by [@jdx](https://github.com/jdx) in [a5df810](https://github.com/jdx/hk/commit/a5df81085304e2fb25fac2f6261fe74fda5e12c1)

## [1.1.0](https://github.com/jdx/hk/compare/v1.0.0..v1.1.0) - 2025-05-14

### 🚀 Features

- added builtins command by [@jdx](https://github.com/jdx) in [#89](https://github.com/jdx/hk/pull/89)
- add HK_STASH_UNTRACKED option by [@jdx](https://github.com/jdx) in [e47e309](https://github.com/jdx/hk/commit/e47e309cdfd24fc104fa100d8e6d0c0a0b7df8fa)
- add shell config by [@jdx](https://github.com/jdx) in [#92](https://github.com/jdx/hk/pull/92)

### 🐛 Bug Fixes

- use --reject flag with `git apply` by [@jdx](https://github.com/jdx) in [1cca76d](https://github.com/jdx/hk/commit/1cca76dad3c2fdac0ada4590883844fd2ae660b2)
- consistently default to git for stashing by [@jdx](https://github.com/jdx) in [494db6c](https://github.com/jdx/hk/commit/494db6ce0d3f42343bc9cceaf608ae28ff8b7f1e)

### 📚 Documentation

- tweak by [@jdx](https://github.com/jdx) in [4f23f61](https://github.com/jdx/hk/commit/4f23f6106e5435a2be413f0d909749bf45d26589)
- sidebar by [@jdx](https://github.com/jdx) in [48555c9](https://github.com/jdx/hk/commit/48555c96ef5870d7285b39df8104adf3f0951c1c)
- mention other hooks are supported by [@jdx](https://github.com/jdx) in [a39ce48](https://github.com/jdx/hk/commit/a39ce48ce0f3dc9ddbc52f438937614b53b62406)
- explain workspace_indicator better by [@jdx](https://github.com/jdx) in [006ee7f](https://github.com/jdx/hk/commit/006ee7f06d8269016472de1c8b14a9d0466f0b2f)

### 🔍 Other Changes

- brew autobump by [@jdx](https://github.com/jdx) in [bb70806](https://github.com/jdx/hk/commit/bb708067620bbe6ada6204171690f0057f1c152f)
- fix git cliff generation by [@jdx](https://github.com/jdx) in [b244f29](https://github.com/jdx/hk/commit/b244f292e89ff328df60c43fab20a83cf4ebc565)
- fix git cliff generation by [@jdx](https://github.com/jdx) in [56f8e65](https://github.com/jdx/hk/commit/56f8e65b7ea57d9f0baaa8b61e14a8b9cfc1eed1)
- clean up cross building by [@jdx](https://github.com/jdx) in [d752e6e](https://github.com/jdx/hk/commit/d752e6e30074397e2ca20e70c1c5ffed4c9e6537)
- Make `hk run` show `run` help instead of crashing by [@markjaquith](https://github.com/markjaquith) in [#95](https://github.com/jdx/hk/pull/95)
- added semantic-pr-lintt by [@jdx](https://github.com/jdx) in [#96](https://github.com/jdx/hk/pull/96)
- define build.rs by [@jdx](https://github.com/jdx) in [bf28f52](https://github.com/jdx/hk/commit/bf28f529499e033e71bc5f63305ca107a1473b53)
- define build.rs by [@jdx](https://github.com/jdx) in [91576c8](https://github.com/jdx/hk/commit/91576c823884bfe9dca4f4e8d5692a4e7115c9f1)

### New Contributors

- @markjaquith made their first contribution in [#95](https://github.com/jdx/hk/pull/95)

## [1.0.0](https://github.com/jdx/hk/compare/v0.8.5..v1.0.0) - 2025-04-26

### 🚀 Features

- groups by [@jdx](https://github.com/jdx) in [c445773](https://github.com/jdx/hk/commit/c44577332aa24e481845833038877c01295520a4)
- newlines builtin by [@jdx](https://github.com/jdx) in [5ec47a1](https://github.com/jdx/hk/commit/5ec47a1e45f84845962328d892332cad2d9a6dd7)

### 🐛 Bug Fixes

- git tweaks by [@jdx](https://github.com/jdx) in [e4ee0ec](https://github.com/jdx/hk/commit/e4ee0ec6c24ca9b301d4e740e9cb443d7fa1885e)

### 📚 Documentation

- about improvements by [@jdx](https://github.com/jdx) in [1793831](https://github.com/jdx/hk/commit/1793831d33fc8e6b878d5d8316bb03bf0c85ffe3)
- docs by [@jdx](https://github.com/jdx) in [b334019](https://github.com/jdx/hk/commit/b3340195f4e527c36a48fcd0b863567a149f4fbb)
- remove logo by [@jdx](https://github.com/jdx) in [e3ce01b](https://github.com/jdx/hk/commit/e3ce01b8404ebf26d77732d0859fb66e6a808b96)
- hk init by [@jdx](https://github.com/jdx) in [94c5557](https://github.com/jdx/hk/commit/94c555717048314178deb9eeed46492c3635d6ab)
- configuration by [@jdx](https://github.com/jdx) in [1624bbd](https://github.com/jdx/hk/commit/1624bbdab0f561412e38db75703d4fdc69c285ec)
- configuration by [@jdx](https://github.com/jdx) in [3935756](https://github.com/jdx/hk/commit/3935756f84e506f496bcd1902da97dc0bb3f2498)

### 🔍 Other Changes

- --plan stub by [@jdx](https://github.com/jdx) in [5d4184c](https://github.com/jdx/hk/commit/5d4184cbee22b4ff9587e69b07f0a03d2ff65f63)
- implemented more of --plan by [@jdx](https://github.com/jdx) in [5074e0d](https://github.com/jdx/hk/commit/5074e0dc27378ea48b60a7a5b47b103e1042e636)
- add release notes by [@jdx](https://github.com/jdx) in [6e6fc22](https://github.com/jdx/hk/commit/6e6fc2203b7ee2d709ce3f22c9067f744908b6b0)
- add brew bump by [@jdx](https://github.com/jdx) in [3e5bce8](https://github.com/jdx/hk/commit/3e5bce85e3e960ec739083a22216013ba5a81a29)

## [0.8.5](https://github.com/jdx/hk/compare/v0.8.4..v0.8.5) - 2025-04-23

### 🐛 Bug Fixes

- non-libgit2 restore fix by [@jdx](https://github.com/jdx) in [600fe48](https://github.com/jdx/hk/commit/600fe488f5f2795573cfd55838cc540f8610cb64)

## [0.8.4](https://github.com/jdx/hk/compare/v0.8.3..v0.8.4) - 2025-04-23

### 🚀 Features

- added `HK_FAIL_FAST` by [@jdx](https://github.com/jdx) in [4ba0047](https://github.com/jdx/hk/commit/4ba00473efa924e99221cb9c17ae7b176e55bfe9)
- allow --from-ref without --to-ref by [@jdx](https://github.com/jdx) in [be42e50](https://github.com/jdx/hk/commit/be42e500b976deefedfa2c222070a5b41e1d7b9d)

### 🐛 Bug Fixes

- correct run/check syntax by [@jdx](https://github.com/jdx) in [18e4a2c](https://github.com/jdx/hk/commit/18e4a2c73c507521c9c5b3919af0f072ade8743f)
- simplify init syntax by [@jdx](https://github.com/jdx) in [89987ab](https://github.com/jdx/hk/commit/89987ab1471934b782361a3efc29c8e280839879)
- set stage property on builtins by [@jdx](https://github.com/jdx) in [bfc94a9](https://github.com/jdx/hk/commit/bfc94a9d8f9f109fe6bd8b6489ad35b53571adfb)
- canonicalize warning by [@jdx](https://github.com/jdx) in [8fa9095](https://github.com/jdx/hk/commit/8fa9095ce4fd2add90d94e494265048d93fef85d)

### 📚 Documentation

- docs by [@jdx](https://github.com/jdx) in [f6fe107](https://github.com/jdx/hk/commit/f6fe1076c9223629386a41422bc87395920e6a64)
- docs by [@jdx](https://github.com/jdx) in [b22cedf](https://github.com/jdx/hk/commit/b22cedf27128737f3c749727e4e9ec6d29ff868c)
- docs by [@jdx](https://github.com/jdx) in [2d909dc](https://github.com/jdx/hk/commit/2d909dc2fbdc87d27f7dece402825451523ef2bf)

## [0.8.3](https://github.com/jdx/hk/compare/v0.8.2..v0.8.3) - 2025-04-22

### 🔍 Other Changes

- enable cross for building by [@jdx](https://github.com/jdx) in [69fbac1](https://github.com/jdx/hk/commit/69fbac1ae892b26aea967f38192f65cf43ae53c3)
- setup GHA for dry run releases by [@jdx](https://github.com/jdx) in [3660565](https://github.com/jdx/hk/commit/366056564d5648e409a505915fe9545ac4cabd04)
- default pkl version in release dry run by [@jdx](https://github.com/jdx) in [e65881b](https://github.com/jdx/hk/commit/e65881b30459cba9aff7aba27cbe24678ba44aba)
- use cross for linux-arm64 by [@jdx](https://github.com/jdx) in [12acb99](https://github.com/jdx/hk/commit/12acb99869813d001b6e50526811e044cefc6499)

## [0.8.2](https://github.com/jdx/hk/compare/v0.8.1..v0.8.2) - 2025-04-18

### 🐛 Bug Fixes

- bug by [@jdx](https://github.com/jdx) in [7a94c4a](https://github.com/jdx/hk/commit/7a94c4ab456ec25a0916ad9217f41f0d7758c89f)

## [0.8.1](https://github.com/jdx/hk/compare/v0.8.0..v0.8.1) - 2025-04-18

### 🐛 Bug Fixes

- progress bar completion by [@jdx](https://github.com/jdx) in [f250f04](https://github.com/jdx/hk/commit/f250f04fd7a52ecf5757c1070e904947fde922a6)

### 📚 Documentation

- cli docs and completions by [@jdx](https://github.com/jdx) in [0ca531d](https://github.com/jdx/hk/commit/0ca531da41f772397a17e93b4f633010d46c6d22)

### 🔍 Other Changes

- bump clx by [@jdx](https://github.com/jdx) in [34664a8](https://github.com/jdx/hk/commit/34664a812d0686aaf49e7670e051b290d4683976)

## [0.8.0](https://github.com/jdx/hk/compare/v0.7.5..v0.8.0) - 2025-04-17

### 🚀 Features

- simplify steps to only have 1 type by [@jdx](https://github.com/jdx) in [#74](https://github.com/jdx/hk/pull/74)
- pkl updates by [@jdx](https://github.com/jdx) in [#77](https://github.com/jdx/hk/pull/77)
- allow adding files mid-run by [@jdx](https://github.com/jdx) in [#83](https://github.com/jdx/hk/pull/83)
- cond by [@jdx](https://github.com/jdx) in [#84](https://github.com/jdx/hk/pull/84)

### 🐛 Bug Fixes

- make hk work with `git commit -am` by [@jdx](https://github.com/jdx) in [#76](https://github.com/jdx/hk/pull/76)
- hide group unless they have name by [@jdx](https://github.com/jdx) in [5eb9c8a](https://github.com/jdx/hk/commit/5eb9c8ab25ab26fe1bac3d7209c20c647d81f2c8)
- staging new files by [@jdx](https://github.com/jdx) in [#85](https://github.com/jdx/hk/pull/85)
- things by [@jdx](https://github.com/jdx) in [4dd7947](https://github.com/jdx/hk/commit/4dd7947f8e882dccf1c8d68dd928a10c18a255d3)
- added "hide" property by [@jdx](https://github.com/jdx) in [a98a2c4](https://github.com/jdx/hk/commit/a98a2c4ef98ccbd24413abaa3017d84905e22a8a)

### 🚜 Refactor

- LinterStep -> Step by [@jdx](https://github.com/jdx) in [#80](https://github.com/jdx/hk/pull/80)
- move hook to hook.rs by [@jdx](https://github.com/jdx) in [#81](https://github.com/jdx/hk/pull/81)
- stash_method by [@jdx](https://github.com/jdx) in [1e92a9b](https://github.com/jdx/hk/commit/1e92a9b4120a15f3979410f717e452b0472b2711)
- hook_ctx by [@jdx](https://github.com/jdx) in [56936d9](https://github.com/jdx/hk/commit/56936d97331b64ccf6a8168e2a028fefba94a9aa)
- use CancellationToken by [@jdx](https://github.com/jdx) in [64a8866](https://github.com/jdx/hk/commit/64a8866b314e45b821592b111cd85b33a5793542)
- build_step_jobs by [@jdx](https://github.com/jdx) in [0b1aafe](https://github.com/jdx/hk/commit/0b1aafe0ddbd22b151305956467469acbea73922)
- file listing by [@jdx](https://github.com/jdx) in [486d0dd](https://github.com/jdx/hk/commit/486d0dd210a85ad8260bb6ccf969d60a48bab22f)
- file adding by [@jdx](https://github.com/jdx) in [ffa9be7](https://github.com/jdx/hk/commit/ffa9be71c829a98932cfb0584e3e3b6a39e0bac3)

### 📚 Documentation

- add pkl intro by [@jdx](https://github.com/jdx) in [fb3eccc](https://github.com/jdx/hk/commit/fb3eccc474d5f7f087c412ad781f3a800fb3d91a)

### ⚡ Performance

- fetch unstaged/staged file lists in parallel by [@jdx](https://github.com/jdx) in [0f6fa56](https://github.com/jdx/hk/commit/0f6fa56031f797a1881a97898e70c3b9f4755091)

### 🧪 Testing

- ensure depends works by [@jdx](https://github.com/jdx) in [68c725d](https://github.com/jdx/hk/commit/68c725dc2886386ce5b18024d19463514e73f417)

### 🔍 Other Changes

- Include `.tfvars` files in Terraform builtin by [@thomasleese](https://github.com/thomasleese) in [#75](https://github.com/jdx/hk/pull/75)
- use ubuntu-latest in GHA by [@jdx](https://github.com/jdx) in [7fea380](https://github.com/jdx/hk/commit/7fea380ea585f02d4051cacca063f933ba849e93)

### New Contributors

- @thomasleese made their first contribution in [#75](https://github.com/jdx/hk/pull/75)

## [0.7.5](https://github.com/jdx/hk/compare/v0.7.4..v0.7.5) - 2025-04-11

### 🐛 Bug Fixes

- bugs by [@jdx](https://github.com/jdx) in [6b9fc78](https://github.com/jdx/hk/commit/6b9fc78ed033d7a6874f1e0c801af049c0ac9b4d)
- apply patch on ctrl-c by [@jdx](https://github.com/jdx) in [4bad9a0](https://github.com/jdx/hk/commit/4bad9a0cb147f660e62abad26020065d987d4e9f)

## [0.7.4](https://github.com/jdx/hk/compare/v0.7.3..v0.7.4) - 2025-04-11

### 🐛 Bug Fixes

- many fixes by [@jdx](https://github.com/jdx) in [7503e09](https://github.com/jdx/hk/commit/7503e09cda70d5916ac82ebf70b10a2388dc09e6)

### 🔍 Other Changes

- update deps by [@jdx](https://github.com/jdx) in [e1bce6d](https://github.com/jdx/hk/commit/e1bce6d7adbbdc062a99472cbe15222a3eb192fb)

## [0.7.3](https://github.com/jdx/hk/compare/v0.7.2..v0.7.3) - 2025-04-10

### 🐛 Bug Fixes

- added env var for HK_STASH by [@jdx](https://github.com/jdx) in [417e683](https://github.com/jdx/hk/commit/417e68300803492dcb8015239ace7bad1a04fb01)

## [0.7.2](https://github.com/jdx/hk/compare/v0.7.1..v0.7.2) - 2025-04-10

### 🐛 Bug Fixes

- bug when depends not running by [@jdx](https://github.com/jdx) in [249cda3](https://github.com/jdx/hk/commit/249cda3718349ca7f49dc9a4f64fae6c847df9de)
- bugs by [@jdx](https://github.com/jdx) in [c6f8f5a](https://github.com/jdx/hk/commit/c6f8f5a66cf0b36b73d732b994eeb90d378f3f03)
- many things by [@jdx](https://github.com/jdx) in [df4c426](https://github.com/jdx/hk/commit/df4c42605d0e0f2526573ad8b307ab938179bc9c)

### 🔍 Other Changes

- Update go_fmt.pkl by [@jdx](https://github.com/jdx) in [b376ffc](https://github.com/jdx/hk/commit/b376ffc308a7345b99447d0bf7fa85c24c858a96)
- Update shfmt.pkl by [@jdx](https://github.com/jdx) in [a45b08c](https://github.com/jdx/hk/commit/a45b08cbcbdd7d06b2239c6b780475ef32b2c4e4)
- Update terraform.pkl by [@jdx](https://github.com/jdx) in [8357957](https://github.com/jdx/hk/commit/835795774dd3dd7871f495bb6c069cfea31b1619)

## [0.7.1](https://github.com/jdx/hk/compare/v0.7.0..v0.7.1) - 2025-04-09

### 🚀 Features

- interactive option by [@jdx](https://github.com/jdx) in [63dd3fd](https://github.com/jdx/hk/commit/63dd3fd733f7b7f7fffe085601acaa54e94e151e)
- exclude by [@jdx](https://github.com/jdx) in [6e68927](https://github.com/jdx/hk/commit/6e689271fcd7655de04c5532ce4a1fbb586453c1)
- allow disabling libgit2 by [@jdx](https://github.com/jdx) in [#67](https://github.com/jdx/hk/pull/67)

### 📚 Documentation

- clarify LinterStep by [@jdx](https://github.com/jdx) in [58ad9e9](https://github.com/jdx/hk/commit/58ad9e9f660aa8da6fb7a8b4265d1c3b73e9fc64)

### 🔍 Other Changes

- updated deps by [@jdx](https://github.com/jdx) in [5b8f1a3](https://github.com/jdx/hk/commit/5b8f1a3cbd2bdcc9caef1844f5af636ec6f6f631)

## [0.7.0](https://github.com/jdx/hk/compare/v0.6.5..v0.7.0) - 2025-04-04

### 🚀 Features

- new pkl structure by [@jdx](https://github.com/jdx) in [#56](https://github.com/jdx/hk/pull/56)

## [0.6.5](https://github.com/jdx/hk/compare/v0.6.4..v0.6.5) - 2025-03-30

### 🚀 Features

- show pending groups by [@jdx](https://github.com/jdx) in [497fa7e](https://github.com/jdx/hk/commit/497fa7e3af17805e645533a44b51786ea35df6ab)
- progress bar by [@jdx](https://github.com/jdx) in [4e91410](https://github.com/jdx/hk/commit/4e914109f11cdaf8a7f4bf69c146dc2b13afcba6)
- show progress of git actions by [@jdx](https://github.com/jdx) in [01f66dc](https://github.com/jdx/hk/commit/01f66dcf47c2e375d665dc312a589d11df783ece)
- show progress of git stashing by [@jdx](https://github.com/jdx) in [56f1353](https://github.com/jdx/hk/commit/56f135303bb14848397065b2980feaa141b4e72c)

### 🐛 Bug Fixes

- tests by [@jdx](https://github.com/jdx) in [3f97453](https://github.com/jdx/hk/commit/3f97453a4c6e9b8c2cbc8c912803295973293e7f)
- truncation by [@jdx](https://github.com/jdx) in [57c49c2](https://github.com/jdx/hk/commit/57c49c209a6e3f355de2c8b4e400f5fc1917cab6)
- use repo root as cwd by [@jdx](https://github.com/jdx) in [0e7b1a7](https://github.com/jdx/hk/commit/0e7b1a721d14d57bbd9255f6e74f1fef0c9257d0)
- correct generated hk.pkl by [@jdx](https://github.com/jdx) in [1dd67e4](https://github.com/jdx/hk/commit/1dd67e4f18a771b0d4250571960cfc06189685e4)
- set errexit by [@jdx](https://github.com/jdx) in [3c45fb7](https://github.com/jdx/hk/commit/3c45fb77e0bf42b3e25a402fe4504042c1cc669b)
- set errexit by [@jdx](https://github.com/jdx) in [b7635c3](https://github.com/jdx/hk/commit/b7635c314ceda4a1bb3fe1d66cf5121a2d8864f1)
- set errexit by [@jdx](https://github.com/jdx) in [eaf7dd0](https://github.com/jdx/hk/commit/eaf7dd0d2dc6cf83f21b8efe528b8fa5563667e7)
- remove test code from actionlint by [@jdx](https://github.com/jdx) in [db07406](https://github.com/jdx/hk/commit/db074062e47d0446605256c73b7d9ceeed689931)

### 🧪 Testing

- tweak by [@jdx](https://github.com/jdx) in [035adca](https://github.com/jdx/hk/commit/035adca93646c2e5a0c4fd09f627cbf05debb6f1)

### 🔍 Other Changes

- bump deps by [@jdx](https://github.com/jdx) in [625df04](https://github.com/jdx/hk/commit/625df04ca2b2c7b95555200ba4ff7384640a3523)
- bump deps by [@jdx](https://github.com/jdx) in [4d257f9](https://github.com/jdx/hk/commit/4d257f96e7a8d8c4f87be0591e505db646d555e7)

## [0.6.4](https://github.com/jdx/hk/compare/v0.6.3..v0.6.4) - 2025-03-29

### 🐛 Bug Fixes

- clean up output when empty by [@jdx](https://github.com/jdx) in [0e01b92](https://github.com/jdx/hk/commit/0e01b92aa668f391a0b3256e0d7635c4b2b6e26e)
- more output tweaks by [@jdx](https://github.com/jdx) in [6bfaae8](https://github.com/jdx/hk/commit/6bfaae830dc59e93b392f3d6141f9f5dc277601b)
- show output file on error by [@jdx](https://github.com/jdx) in [c413a03](https://github.com/jdx/hk/commit/c413a03d1e9b5c45c2fd0d7f6c5e47a2ad847bb6)

### 🔍 Other Changes

- wip by [@jdx](https://github.com/jdx) in [f2cd324](https://github.com/jdx/hk/commit/f2cd32465e473f13f1c54e567efd4f3b6a730fed)

## [0.6.3](https://github.com/jdx/hk/compare/v0.6.2..v0.6.3) - 2025-03-29

### 🚀 Features

- clx v2 by [@jdx](https://github.com/jdx) in [#45](https://github.com/jdx/hk/pull/45)

## [0.6.2](https://github.com/jdx/hk/compare/v0.6.1..v0.6.2) - 2025-03-24

### 🚀 Features

- allow specifying any git hooks by [@jdx](https://github.com/jdx) in [#42](https://github.com/jdx/hk/pull/42)

### 🐛 Bug Fixes

- glob after dir by [@jdx](https://github.com/jdx) in [dd26b0a](https://github.com/jdx/hk/commit/dd26b0a497c357a61da997ea131bf25a6d18f97a)

### 🚜 Refactor

- move failed mutex to ctx by [@jdx](https://github.com/jdx) in [1d9074b](https://github.com/jdx/hk/commit/1d9074b9a3d0ffcb21c7b0c6e94b40ff9c4d533b)
- move tctx into ctx by [@jdx](https://github.com/jdx) in [d7b6bbd](https://github.com/jdx/hk/commit/d7b6bbd6fa8008d05a5199a0ebd8d074cb9b843c)
- move semaphore to ctx by [@jdx](https://github.com/jdx) in [1399920](https://github.com/jdx/hk/commit/1399920df0b9fe38cdaaf25550c6c28008a89188)
- remove lint by [@jdx](https://github.com/jdx) in [331507c](https://github.com/jdx/hk/commit/331507c055b9575aa71ba28ea67f4072d57730f5)
- move step to ctx by [@jdx](https://github.com/jdx) in [3300747](https://github.com/jdx/hk/commit/3300747f984a656c39f6e1fa585cc3a02ec3c2c4)
- remove files_in_contention from run_step by [@jdx](https://github.com/jdx) in [9ce5c44](https://github.com/jdx/hk/commit/9ce5c44f5be8759bfeb0a4c7dba3edce0751ab30)
- break step classes into separate files by [@jdx](https://github.com/jdx) in [f47b38d](https://github.com/jdx/hk/commit/f47b38d56ddc6f77989ef3f28356051af1419cef)
- remove unnecessary file_locks mutex by [@jdx](https://github.com/jdx) in [c7bf181](https://github.com/jdx/hk/commit/c7bf1810739599ea5ee696c72321cbc33c45a7a6)
- added queue by [@jdx](https://github.com/jdx) in [#41](https://github.com/jdx/hk/pull/41)

## [0.6.1](https://github.com/jdx/hk/compare/v0.6.0..v0.6.1) - 2025-03-22

### 🚀 Features

- commit-msg hook by [@jdx](https://github.com/jdx) in [aa55aee](https://github.com/jdx/hk/commit/aa55aeec29e0c71db8ff49d9cebd55872d76d32a)

### 🐛 Bug Fixes

- make files relative to dir instead of repo root by [@jdx](https://github.com/jdx) in [8626a46](https://github.com/jdx/hk/commit/8626a468d1bd08a6a4bb467fb6142d701ad2f116)

## [0.6.0](https://github.com/jdx/hk/compare/v0.5.1..v0.6.0) - 2025-03-21

### 🚀 Features

- prepare-commit-msg by [@jdx](https://github.com/jdx) in [#37](https://github.com/jdx/hk/pull/37)

### 🔍 Other Changes

- added mise deps by [@jdx](https://github.com/jdx) in [d73a13c](https://github.com/jdx/hk/commit/d73a13c6bfc9de2a4180523351ca19a67fceb01a)

## [0.5.1](https://github.com/jdx/hk/compare/v0.5.0..v0.5.1) - 2025-03-20

### 🚀 Features

- added --force flag to generate by [@jdx](https://github.com/jdx) in [09b63ff](https://github.com/jdx/hk/commit/09b63ff1a220cfbc804624270c31fa60155dc102)

### 🐛 Bug Fixes

- disable check_first for cargo-check/clippy by [@jdx](https://github.com/jdx) in [5546426](https://github.com/jdx/hk/commit/5546426fc3809ab6a11c2e7280eed5e585fb8ac3)
- make pre-push hook function correctly by [@jdx](https://github.com/jdx) in [#35](https://github.com/jdx/hk/pull/35)

### 🔍 Other Changes

- Update about.md by [@jdx](https://github.com/jdx) in [aba3525](https://github.com/jdx/hk/commit/aba3525bc9eb27260ed19c377091de35a5c5b90a)
- Update about.md by [@jdx](https://github.com/jdx) in [c320d35](https://github.com/jdx/hk/commit/c320d35065fea507fdfd8d1d835e230c4430d18d)
- Update about.md by [@jdx](https://github.com/jdx) in [84a3d96](https://github.com/jdx/hk/commit/84a3d96a90aa730ca9147ede22947645a1d9a229)
- remove `rustup up` by [@jdx](https://github.com/jdx) in [#36](https://github.com/jdx/hk/pull/36)
- added `mise run release` task by [@jdx](https://github.com/jdx) in [ea39789](https://github.com/jdx/hk/commit/ea39789c6604f74bd9cb0963911bf3537ea6c419)

## [0.5.0](https://github.com/jdx/hk/compare/v0.4.6..v0.5.0) - 2025-02-25

### 🚀 Features

- --from-ref/--to-ref by [@jdx](https://github.com/jdx) in [de47fa4](https://github.com/jdx/hk/commit/de47fa4d107d4edb580d7e6cbc744999f29bd06e)

### 📚 Documentation

- data-loss bug has been resolved by [@jdx](https://github.com/jdx) in [bc4390e](https://github.com/jdx/hk/commit/bc4390ea4f75215f65ed57ffbd78dd0e25f203dd)
- update all docs by [@jdx](https://github.com/jdx) in [d42f97b](https://github.com/jdx/hk/commit/d42f97b2e87dc0efeb58d99d5bbb1c0295c48e16)

### 🔍 Other Changes

- Update README.md by [@jdx](https://github.com/jdx) in [a56321d](https://github.com/jdx/hk/commit/a56321d68ae945cf7bd1ec6f801d25dce21c8867)

## [0.4.6](https://github.com/jdx/hk/compare/v0.4.5..v0.4.6) - 2025-02-24

### 🚀 Features

- batch by [@jdx](https://github.com/jdx) in [9f0e3f6](https://github.com/jdx/hk/commit/9f0e3f6c8277c73e58f4f3ea621a75bae0e2f522)

## [0.4.5](https://github.com/jdx/hk/compare/v0.4.4..v0.4.5) - 2025-02-23

### 🚀 Features

- added env field to step/linter by [@jdx](https://github.com/jdx) in [ee02aa0](https://github.com/jdx/hk/commit/ee02aa0475df486a9d0188cbc84198209f038f40)
- filter check_first with list of files by [@jdx](https://github.com/jdx) in [d97ac4f](https://github.com/jdx/hk/commit/d97ac4fd9fba1f5c21b0d1e245095fa3ae263b7f)

### 🐛 Bug Fixes

- use `--force` when popping unstaged changes by [@jdx](https://github.com/jdx) in [bed8692](https://github.com/jdx/hk/commit/bed8692a005e23f117139dfb2eccf73ddef0b460)
- workspace_indicator with cargo-fmt by [@jdx](https://github.com/jdx) in [6832fd3](https://github.com/jdx/hk/commit/6832fd3f98055378e4b0b7169eb4aa0b0700cd5b)
- show warning if missing fix files by [@jdx](https://github.com/jdx) in [62051eb](https://github.com/jdx/hk/commit/62051ebab721d9f040f4fcb26a3dd38f09071f1a)

### 🔍 Other Changes

- cargo up by [@jdx](https://github.com/jdx) in [d1fd40b](https://github.com/jdx/hk/commit/d1fd40b8d5cd4f765ce16f8104d24e9d5dbd9a77)

## [0.4.4](https://github.com/jdx/hk/compare/v0.4.3..v0.4.4) - 2025-02-22

### 🚀 Features

- support eslint by [@jdx](https://github.com/jdx) in [34ec22d](https://github.com/jdx/hk/commit/34ec22dd7f9193e55a6025ad69cbd7b2b5afbadf)

### 🐛 Bug Fixes

- use List instead of Listing by [@jdx](https://github.com/jdx) in [48edd5d](https://github.com/jdx/hk/commit/48edd5d77021d280b29b11995509a278a7d0ec7b)

### 📚 Documentation

- update example by [@jdx](https://github.com/jdx) in [84cb727](https://github.com/jdx/hk/commit/84cb7277d9e2da1895b65f271320f0d7566a0a7f)
- update benchmark.png by [@jdx](https://github.com/jdx) in [8fbfbec](https://github.com/jdx/hk/commit/8fbfbec1d6131b071aea7f836e00e31006e63037)

### 🧪 Testing

- fix check_first test by [@jdx](https://github.com/jdx) in [92c25c6](https://github.com/jdx/hk/commit/92c25c603c446cd0c1be6a3fef7aa83641cdfc37)

### 🔍 Other Changes

- rustfmt by [@jdx](https://github.com/jdx) in [4aa7bfb](https://github.com/jdx/hk/commit/4aa7bfb2f0b5408cdc981f952587f48da16a76e8)
- build cli for benchmark by [@jdx](https://github.com/jdx) in [cd4c016](https://github.com/jdx/hk/commit/cd4c0163b51744347a38f09d90d02f142063a05c)
- macos code signing by [@jdx](https://github.com/jdx) in [782b290](https://github.com/jdx/hk/commit/782b2900e712cf82eed347b672e9a9e09117b46f)

## [0.4.3](https://github.com/jdx/hk/compare/v0.4.2..v0.4.3) - 2025-02-21

### 🚀 Features

- cache config by [@jdx](https://github.com/jdx) in [6791c00](https://github.com/jdx/hk/commit/6791c00a7da09b0929ccb258f8b86ce7cd892602)
- check_diff and check_list_files added (but do nothing extra yet) by [@jdx](https://github.com/jdx) in [4aea0a8](https://github.com/jdx/hk/commit/4aea0a85d2c2b45da365a4ebff8676aa03a07719)
- stub out new pkl fields by [@jdx](https://github.com/jdx) in [5a598c3](https://github.com/jdx/hk/commit/5a598c3c69b9cffe2215454eb57b6d9b0209f313)
- workspace_indicator by [@jdx](https://github.com/jdx) in [#21](https://github.com/jdx/hk/pull/21)

### 🐛 Bug Fixes

- improve output a bit by [@jdx](https://github.com/jdx) in [9f4b534](https://github.com/jdx/hk/commit/9f4b5346d58ba695254f49215f3bd172fde0f72f)

### 📚 Documentation

- add more stuff to the example by [@jdx](https://github.com/jdx) in [614f47b](https://github.com/jdx/hk/commit/614f47b9d301257128eaf495e821bfe5f623cc24)
- add more stuff to the example by [@jdx](https://github.com/jdx) in [7ca1c36](https://github.com/jdx/hk/commit/7ca1c363c1bc2e0722216bc74d8111ba26d7e50b)

### 🧪 Testing

- fix tests by [@jdx](https://github.com/jdx) in [7eb11ca](https://github.com/jdx/hk/commit/7eb11ca0f3219efc28f641c720696ae25ae86d6a)

### 🔍 Other Changes

- fix release job by [@jdx](https://github.com/jdx) in [1604dbc](https://github.com/jdx/hk/commit/1604dbc801d4f2c9574eabdec5d7076f7f478841)
- update rust by [@jdx](https://github.com/jdx) in [630f9f7](https://github.com/jdx/hk/commit/630f9f72fc5ad6208c10da5a200eca93fcd2cbc6)
- update rust by [@jdx](https://github.com/jdx) in [0ad4c71](https://github.com/jdx/hk/commit/0ad4c714fae80e3729de2de2800d0740fd38d702)

## [0.4.2](https://github.com/jdx/hk/compare/v0.4.1..v0.4.2) - 2025-02-21

### 🐛 Bug Fixes

- use real stashing by [@jdx](https://github.com/jdx) in [#22](https://github.com/jdx/hk/pull/22)

### 🚜 Refactor

- paving the way for batching steps by [@jdx](https://github.com/jdx) in [f8c4ff3](https://github.com/jdx/hk/commit/f8c4ff368ebd9be348f5bc8aee4da8ab40c1f892)

### 📚 Documentation

- favicon by [@jdx](https://github.com/jdx) in [2fb8aa0](https://github.com/jdx/hk/commit/2fb8aa0b7db244b13c7ddb102baba08b22b9fa97)

### 🔍 Other Changes

- fix draft release by [@jdx](https://github.com/jdx) in [3d79627](https://github.com/jdx/hk/commit/3d796276daa63e62af74765f190d6078d965fc3f)
- Delete pkl/builtins/prettier_package_json.pkl by [@jdx](https://github.com/jdx) in [44dc0f0](https://github.com/jdx/hk/commit/44dc0f002fe3e1fa9fd4e6aa6b590fbdcb2d60cb)
- make compatible with lowest semver by [@jdx](https://github.com/jdx) in [444e6e2](https://github.com/jdx/hk/commit/444e6e2910ed85275e4b79c4fe6bf087bc446fdf)
- Update README.md by [@jdx](https://github.com/jdx) in [694cea7](https://github.com/jdx/hk/commit/694cea77aa990d806f0c60ad7bc33554f9e0b472)
- Update README.md by [@jdx](https://github.com/jdx) in [9e527f4](https://github.com/jdx/hk/commit/9e527f49d9d651f7b45d0c80800d536790fff02f)
- Update README.md by [@jdx](https://github.com/jdx) in [cc09b3d](https://github.com/jdx/hk/commit/cc09b3ddc30e8e958bfc071a9110eb07955216ad)
- Update getting_started.md by [@jdx](https://github.com/jdx) in [3a878d5](https://github.com/jdx/hk/commit/3a878d53d4165be85ca3e35157667bfc16223ba6)

## [0.4.1](https://github.com/jdx/hk/compare/v0.4.0..v0.4.1) - 2025-02-20

### 🐛 Bug Fixes

- check step by [@jdx](https://github.com/jdx) in [7935824](https://github.com/jdx/hk/commit/7935824c6875977510b41931680f51d0ca09803a)

### 🔍 Other Changes

- draft release by [@jdx](https://github.com/jdx) in [8437542](https://github.com/jdx/hk/commit/843754268115439d8c972f8b7e08536aee9d2d88)

## [0.4.0](https://github.com/jdx/hk/compare/v0.3.3..v0.4.0) - 2025-02-20

### 🚀 Features

- new schema by [@jdx](https://github.com/jdx) in [#15](https://github.com/jdx/hk/pull/15)

### 🔍 Other Changes

- Update README.md by [@jdx](https://github.com/jdx) in [8ac0c21](https://github.com/jdx/hk/commit/8ac0c2161ff3fb1ff9a27d7ec1a12d1b08422a69)

## [0.3.3](https://github.com/jdx/hk/compare/v0.3.2..v0.3.3) - 2025-02-19

### 🔍 Other Changes

- bump version on releases in docs by [@jdx](https://github.com/jdx) in [33a1e5a](https://github.com/jdx/hk/commit/33a1e5a8095ebbed55f7f1b57bbf219b11e0f0a3)
- bump version on releases in docs by [@jdx](https://github.com/jdx) in [ca0c739](https://github.com/jdx/hk/commit/ca0c739faa0530a31c17630a1bf0642536bfc1e1)
- bump version on releases in docs by [@jdx](https://github.com/jdx) in [2d99a45](https://github.com/jdx/hk/commit/2d99a450c7601ffb578c823ed12ee4422be169b2)

## [0.3.2](https://github.com/jdx/hk/compare/v0.3.1..v0.3.2) - 2025-02-19

### 🔍 Other Changes

- fix pkl packageZipUrl by [@jdx](https://github.com/jdx) in [42daa33](https://github.com/jdx/hk/commit/42daa33cbb07df402bf4e527e38b8dae5ed8dfa7)

## [0.3.1](https://github.com/jdx/hk/compare/v0.3.0..v0.3.1) - 2025-02-19

### 🔍 Other Changes

- fix pkl to work as module by [@jdx](https://github.com/jdx) in [5cc993c](https://github.com/jdx/hk/commit/5cc993ce259f2951bb40aaa06539e2ed26c86199)
- fix pkl to work as module by [@jdx](https://github.com/jdx) in [4a97788](https://github.com/jdx/hk/commit/4a977880cf250d3b0e530910a16df00da162485a)

## [0.3.0](https://github.com/jdx/hk/compare/v0.2.4..v0.3.0) - 2025-02-19

### 🐛 Bug Fixes

- check_first logic by [@jdx](https://github.com/jdx) in [#11](https://github.com/jdx/hk/pull/11)
- only add changed files by [@jdx](https://github.com/jdx) in [218e254](https://github.com/jdx/hk/commit/218e2541ff942f3e5c695cfba0675e932489cbee)
- skip adding if no files by [@jdx](https://github.com/jdx) in [1313311](https://github.com/jdx/hk/commit/13133117a2c9c75278aa70a43c71b102c8443b8f)

### 🔍 Other Changes

- fix windows lint issue by [@jdx](https://github.com/jdx) in [8c4dade](https://github.com/jdx/hk/commit/8c4dade0e4f208e4489a5bc1c334347c192608b3)
- build pkl only on releases by [@jdx](https://github.com/jdx) in [0924566](https://github.com/jdx/hk/commit/09245666b9021c952be319ea23a8747a535dd4aa)
- fix CI by [@jdx](https://github.com/jdx) in [c9fa55b](https://github.com/jdx/hk/commit/c9fa55b74a527fbadaee413688b75953faf16470)
- Create renovate.json by [@jdx](https://github.com/jdx) in [a5bfbe8](https://github.com/jdx/hk/commit/a5bfbe835a39eeafa230f28df52513fad67af774)
- Update README.md by [@jdx](https://github.com/jdx) in [64d8f27](https://github.com/jdx/hk/commit/64d8f276cccb9f6b499d4d45d64e39b84906887b)
- package pkl into project by [@jdx](https://github.com/jdx) in [44adb46](https://github.com/jdx/hk/commit/44adb46a7856d30900f3cbcbd34678827411078e)
- added PklProject.deps.json by [@jdx](https://github.com/jdx) in [b392f84](https://github.com/jdx/hk/commit/b392f84fc88165b81fe47ce153d0cf38262eb2bf)
- move min_hk_version to base pkl by [@jdx](https://github.com/jdx) in [ff35c94](https://github.com/jdx/hk/commit/ff35c94aaeda5a4a091d9ac44289e1b5ff605b9a)
- stop building pkl to v0 by [@jdx](https://github.com/jdx) in [a62c874](https://github.com/jdx/hk/commit/a62c8748562f78af2a33e5a59e56d764bc4a7056)
- prettier on commands.json by [@jdx](https://github.com/jdx) in [6591de2](https://github.com/jdx/hk/commit/6591de2e9fefc69279a28bdf878d7f4d231af0eb)
- prettier on commands.json by [@jdx](https://github.com/jdx) in [216c4d0](https://github.com/jdx/hk/commit/216c4d0673ca0916f6be5b6eb9f55bbd2f1f409d)

## [0.2.4](https://github.com/jdx/hk/compare/v0.2.3..v0.2.4) - 2025-02-18

### 🚀 Features

- added depends/check_first/stomp configs by [@jdx](https://github.com/jdx) in [73480d0](https://github.com/jdx/hk/commit/73480d02ae058121ba6cc34bc3bb85a1b997280a)
- make depends work by [@jdx](https://github.com/jdx) in [8ac584d](https://github.com/jdx/hk/commit/8ac584d38a513c8b35dbd6bb66ff5c6224a1b2ab)

### 📚 Documentation

- improve by [@jdx](https://github.com/jdx) in [58a8744](https://github.com/jdx/hk/commit/58a874416ac75dc80c14c0a7c12f2f293813a68b)
- syntax by [@jdx](https://github.com/jdx) in [2a5abea](https://github.com/jdx/hk/commit/2a5abea382161cbda21db344c8db73f4370fb5fc)
- describe cli more by [@jdx](https://github.com/jdx) in [5ec008c](https://github.com/jdx/hk/commit/5ec008ca703f788fba8f9d4187115880a58e2093)

### 🔍 Other Changes

- added local hk wrapper by [@jdx](https://github.com/jdx) in [bff8b53](https://github.com/jdx/hk/commit/bff8b53c15d0dde5d2a72258282a71e368df1705)

## [0.2.3](https://github.com/jdx/hk/compare/v0.2.2..v0.2.3) - 2025-02-17

### 🚀 Features

- added HK_FILE to use a different config filename by [@jdx](https://github.com/jdx) in [51f1326](https://github.com/jdx/hk/commit/51f1326494ec18abbba10e59dcf3e19f839936a8)

### 🐛 Bug Fixes

- show better error message if pkl is missing by [@jdx](https://github.com/jdx) in [4b71530](https://github.com/jdx/hk/commit/4b715305f7603cf57a7cc5eaa771f3fc91ad6b5c)

### 📚 Documentation

- stronger message about WIP by [@jdx](https://github.com/jdx) in [c650bfa](https://github.com/jdx/hk/commit/c650bfa0d9a6892b7b56726830d59c9b137fd7bb)
- hn note by [@jdx](https://github.com/jdx) in [6143924](https://github.com/jdx/hk/commit/61439240b99d1d79a950b23d57515b1d824d2a2c)
- hn note by [@jdx](https://github.com/jdx) in [7630bf1](https://github.com/jdx/hk/commit/7630bf1943f0c44ea3fbd11eafbbdc6fec895db4)
- benchmark in readme by [@jdx](https://github.com/jdx) in [f75d7de](https://github.com/jdx/hk/commit/f75d7de9f1350be824251da9c78c246e039f6915)

### 🔍 Other Changes

- fix changelog generation version by [@jdx](https://github.com/jdx) in [18bc316](https://github.com/jdx/hk/commit/18bc3164c5f2ca3c3abc9a42eae1d93d81e79a33)
- updated http url by [@jdx](https://github.com/jdx) in [875b25c](https://github.com/jdx/hk/commit/875b25c068492eed78829d9d1a9d0aa6b9dd9ca6)
- benchmark by [@jdx](https://github.com/jdx) in [#6](https://github.com/jdx/hk/pull/6)
- benchmark by [@jdx](https://github.com/jdx) in [#7](https://github.com/jdx/hk/pull/7)

## [0.2.2](https://github.com/jdx/hk/compare/v0.2.1..v0.2.2) - 2025-02-17

### 🚀 Features

- HK_SKIP_HOOKS by [@jdx](https://github.com/jdx) in [5a07907](https://github.com/jdx/hk/commit/5a079075b6c841269ed3c12821eee545a5911849)
- added a bunch of AI barf by [@jdx](https://github.com/jdx) in [bdac3cd](https://github.com/jdx/hk/commit/bdac3cdb7af01efbdd158e522c7692a563249cac)

## [0.2.1](https://github.com/jdx/hk/compare/v0.2.0..v0.2.1) - 2025-02-17

### 🚀 Features

- grouping steps by [@jdx](https://github.com/jdx) in [a0dda64](https://github.com/jdx/hk/commit/a0dda64c057a37bf66c64e2f0ff6613248786247)
- init alias by [@jdx](https://github.com/jdx) in [6cd7390](https://github.com/jdx/hk/commit/6cd7390d3106e1aac904c5c89a14faf11f798de5)

### 🐛 Bug Fixes

- --check logic by [@jdx](https://github.com/jdx) in [0a0f66c](https://github.com/jdx/hk/commit/0a0f66c1f1278222551bed72765a41b6cd7ac26b)
- --check logic by [@jdx](https://github.com/jdx) in [e204037](https://github.com/jdx/hk/commit/e204037c5cd73485e2809aceee7904d9186e6803)

### 📚 Documentation

- docs: by [@jdx](https://github.com/jdx) in [25af7d8](https://github.com/jdx/hk/commit/25af7d8cd9838fced725914f6061f8740d944094)
- added pkl syntax by [@jdx](https://github.com/jdx) in [a71f3ab](https://github.com/jdx/hk/commit/a71f3abb4b98427d03a3e2ee037d876e7ce9def1)

### 🔍 Other Changes

- do not prettify commands.json by [@jdx](https://github.com/jdx) in [a0d2d0e](https://github.com/jdx/hk/commit/a0d2d0e82897a72956f7dbd6f244eb3e290e52e2)
- added actionlint to CI by [@jdx](https://github.com/jdx) in [3f42d91](https://github.com/jdx/hk/commit/3f42d91c6fe6e79e296d5a2014368e7bb4e4b0e8)
- wip by [@jdx](https://github.com/jdx) in [7e3bf67](https://github.com/jdx/hk/commit/7e3bf67c748f0d956f63e3048ee8c7603acf42a6)
- goat counter by [@jdx](https://github.com/jdx) in [d47a456](https://github.com/jdx/hk/commit/d47a456ab8a2e549a61964e152116fe74d333952)
- GA by [@jdx](https://github.com/jdx) in [035b1d8](https://github.com/jdx/hk/commit/035b1d869fdaf59e1b313dde0e6b5c628f4d0583)
- created flocks by [@jdx](https://github.com/jdx) in [343ab06](https://github.com/jdx/hk/commit/343ab06baebb3d1b580d1281045edbcfb1f6a913)
- disabled beta toolchain on CI by [@jdx](https://github.com/jdx) in [41e2028](https://github.com/jdx/hk/commit/41e20288d5c1b8f06183b7622ef01f1f3d99ea29)

## [0.2.0](https://github.com/jdx/hk/compare/v0.1.9..v0.2.0) - 2025-02-17

### 🚀 Features

- **breaking** use check/check_all instead of run/run_all by [@jdx](https://github.com/jdx) in [5a6555c](https://github.com/jdx/hk/commit/5a6555ce015d19083b2fdb526875b7133583efd4)

### 🐛 Bug Fixes

- tidy up step output by [@jdx](https://github.com/jdx) in [4196721](https://github.com/jdx/hk/commit/4196721a7e7f59d61062a1b4747fcc730370fc2f)
- recurse directories to find hk.pkl by [@jdx](https://github.com/jdx) in [d53b8f5](https://github.com/jdx/hk/commit/d53b8f56a66dad20fd15ce3a019554e13369b165)
- only stage changes if globs are defined by [@jdx](https://github.com/jdx) in [61b3514](https://github.com/jdx/hk/commit/61b351440154c8f45dbc90b72d148d26ab2924c4)
- only warn if failed adding staged files by [@jdx](https://github.com/jdx) in [7a2f92e](https://github.com/jdx/hk/commit/7a2f92e7287c217b086a85fd1fe501ab3edfeaf1)

### 🔍 Other Changes

- init by [@jdx](https://github.com/jdx) in [0a4e57c](https://github.com/jdx/hk/commit/0a4e57cf0f597ae8495b5ad250c9afff5948ad29)
- remove unused black tool by [@jdx](https://github.com/jdx) in [844a0a4](https://github.com/jdx/hk/commit/844a0a424ee362e861528525c6800bdbc046dd28)
- Update configuration.md by [@jdx](https://github.com/jdx) in [64178c6](https://github.com/jdx/hk/commit/64178c6cfe0c4820f088867e651b21ebfac5c7b6)
- switch to rpkl upstream by [@jdx](https://github.com/jdx) in [3d7e219](https://github.com/jdx/hk/commit/3d7e219e7f8686e9c3ebe0ba64a2490a4ae235e7)

<!-- generated by git-cliff -->
