## [2.1.2] - 2025-08-18

### 🚀 Features

- Add ungz method for gzip decompression (#96)
- Add file and environment utilities from mise (#99)
- Convert xx, clx, and ensembler to git subtrees
- Added support for process::cmd to read stdout/stderr by line

### 🐛 Bug Fixes

- Bug with stashing unstaged files (#160)
- Bug with <clx:flex> tags appearing when they should not (#161)

### ⚙️ Miscellaneous Tasks

- Pin homedir to 0.3.5 for MSRV compatibility
- Cargo build
## [2.1.1] - 2025-05-15

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 2.1.1
## [2.1.0] - 2025-04-25

### 🚀 Features

- Added duct cmd expression
- *(process)* Added arg/args

### 🧪 Testing

- Harden tests

### ⚙️ Miscellaneous Tasks

- Bump deps
- Remove -x
- Added hk
- Added pkl
- Release xx version 2.1.0
## [2.0.5] - 2025-02-17

### 🐛 Bug Fixes

- Clone options (#72)
- Add stub make_executable for windows

### ⚙️ Miscellaneous Tasks

- Updated deps
- Fix cargo includes
- Release xx version 2.0.5
## [2.0.4] - 2025-02-01

### 🚀 Features

- Add branch support on clone (#71)

### 🐛 Bug Fixes

- *(deps)* Update rust crate bzip2 to 0.5 (#62)

### ⚙️ Miscellaneous Tasks

- Lint issue
- Updated deps
- Cargo up
- Cargo up
- Always save cache
- Release hook info
- Set cargo include
- Release xx version 2.0.4
## [2.0.3] - 2024-12-12

### 🚀 Features

- Find_up

### ⚙️ Miscellaneous Tasks

- Release xx version 2.0.3
## [2.0.2] - 2024-12-10

### 🚀 Features

- Reexport fs::file

### ⚙️ Miscellaneous Tasks

- Add hash as dependency of fslock
- Rename fslock struct
- Rename fslock struct
- Must_use
- Release xx version 2.0.1
- Release xx version 2.0.2
## [2.0.0] - 2024-12-06

### 🚀 Features

- Fslock

### ⚙️ Miscellaneous Tasks

- Upgraded miette
- Release xx version 2.0.0
## [1.1.9] - 2024-11-11

### 🐛 Bug Fixes

- *(deps)* Update rust crate flate2 to v1.0.33 (#44)
- *(deps)* Update rust crate reqwest to v0.12.7 (#45)
- *(deps)* Update rust crate filetime to v0.2.25 (#47)
- *(deps)* Update rust crate flate2 to v1.0.34 (#48)
- *(deps)* Update rust crate tar to v0.4.42 (#49)
- *(deps)* Update rust crate homedir to v0.3.4 (#50)
- *(deps)* Update rust crate reqwest to v0.12.8 (#51)

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 1.1.9
## [1.1.8] - 2024-08-19

### 🐛 Bug Fixes

- Statically link xz

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.8
## [1.1.7] - 2024-08-18

### 🐛 Bug Fixes

- Windows compat

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.7
## [1.1.6] - 2024-08-18

### 🐛 Bug Fixes

- *(deps)* Update rust crate zip to v2.1.2 (#18)
- *(deps)* Update rust crate tokio to v1.38.0 (#19)
- *(deps)* Update rust crate regex to v1.10.5 (#20)
- *(deps)* Update rust crate tar to v0.4.41 (#21)
- *(deps)* Update rust crate zip to v2.1.3 (#22)
- *(deps)* Update rust crate reqwest to v0.12.5 (#24)
- *(deps)* Update rust crate log to v0.4.22 (#26)
- *(deps)* Update rust crate thiserror to v1.0.62 (#30)
- *(deps)* Update rust crate thiserror to v1.0.63 (#32)
- *(deps)* Update rust crate tokio to v1.38.1 (#33)
- *(deps)* Update rust crate zip to v2.1.5 (#34)
- *(deps)* Update rust crate tokio to v1.39.2 (#37)
- *(deps)* Update rust crate flate2 to v1.0.31 (#39)
- *(deps)* Update rust crate regex to v1.10.6 (#40)
- *(deps)* Update rust crate filetime to v0.2.24 (#41)
- *(deps)* Update rust crate zip to v2.1.6 (#42)
- Chmod zip unarchiving (#43)

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.6
## [1.1.5] - 2024-05-25

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 1.1.5
## [1.1.4] - 2024-05-25

### 🐛 Bug Fixes

- *(git)* Make clone static

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.4
## [1.1.3] - 2024-05-25

### 🐛 Bug Fixes

- *(git)* Make clone static

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.3
## [1.1.2] - 2024-05-25

### 🚀 Features

- *(file)* Added mv

### 🐛 Bug Fixes

- *(deps)* Update rust crate thiserror to v1.0.61 (#15)
- *(file)* Accept generic content for write()
- *(http)* Create dir for download
- *(file)* Create dir before moving

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.2
## [1.1.1] - 2024-05-14

### 🚀 Features

- *(hash)* Added ensure_checksum_sha512

### 🐛 Bug Fixes

- Hash generics

### ⚙️ Miscellaneous Tasks

- *(hash)* Debug logging
- Release xx version 1.1.1
## [1.1.0] - 2024-05-14

### 🚀 Features

- Added hash functions

### 🐛 Bug Fixes

- *(deps)* Update rust crate thiserror to v1.0.60 (#13)

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.0
## [1.0.2] - 2024-05-13

### 🐛 Bug Fixes

- Use async reqwest

### ⚙️ Miscellaneous Tasks

- Release xx version 1.0.2
## [1.0.1] - 2024-05-13

### 🚀 Features

- Added rustls feature

### ⚙️ Miscellaneous Tasks

- Release xx version 1.0.1
## [1.0.0] - 2024-05-13

### 🚀 Features

- Added file::remove_dir_all
- Http

### 🧪 Testing

- Enable logging in unit tests
- Enable trace logging
- Show coverage results in action output

### ⚙️ Miscellaneous Tasks

- Added coverage (#12)
- Release xx version 1.0.0
## [0.5.1] - 2024-05-12

### 🐛 Bug Fixes

- Lib.rs

### 📚 Documentation

- Added CHANGELOG

### ⚙️ Miscellaneous Tasks

- Release xx version 0.5.1
## [0.5.0] - 2024-05-12

### 🚀 Features

- Glob function

### ⚙️ Miscellaneous Tasks

- Release xx version 0.5.0
## [0.4.0] - 2024-05-11

### 🚀 Features

- Archive functions

### ⚙️ Miscellaneous Tasks

- Release xx version 0.4.0
## [0.3.0] - 2024-04-25

### 🚀 Features

- Added git module

### ⚙️ Miscellaneous Tasks

- Release xx version 0.3.0
## [0.2.5] - 2024-02-10

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.5
## [0.2.4] - 2024-02-10

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.4
## [0.2.3] - 2024-02-09

### 🐛 Bug Fixes

- *(deps)* Update rust crate miette to v6

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.3
## [0.2.2] - 2024-01-14

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.2
## [0.2.1] - 2024-01-14

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.1
## [0.2.0] - 2024-01-13
