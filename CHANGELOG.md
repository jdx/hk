# Changelog

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

### New Contributors

- @jdx made their first contribution

<!-- generated by git-cliff -->
