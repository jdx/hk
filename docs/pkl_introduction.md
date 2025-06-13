# Introduction to pkl

hk uses [pkl](https://pkl-lang.org/) for configuration. As this is a new configuration language, this doc gives an overview of how to write
it and work with it for hk configuration.

## Dependencies

You'll need the pkl cli to use hk. This is because the rust library currently shells out to the pkl cli to parse the configuration. I'm sure someday we'll have a native pkl parser in rust (maybe you could write it?) but for now, you'll need pkl.

These are easily installed with mise though:

```sh
mise use -g pkl
```

## Why pkl?

* Schema validation is built into the language so your IDE can display errors not just with pkl syntax, but ensure that the types are correct
* pkl can import other pkl files from the file system or HTTP URLs—so hk doesn't need its own logic around "importing" files
* You can create/amend shared objects which can really help clean up your config. It even has things like functions and string templates for advanced use cases.
* pkl is comprehensive enough—but static—that I found I didn't need a plugin system for hk. I had looked at wasm and lua for plugins, but by using (cached) pkl files this helps hk stay much faster than it would be otherwise.

## Downsides?

* Editor/syntax highlighting support is young—though being a project driven by Apple I suspect this will improve quicker than most languages
* Some of the behavior with the "amends" line and how `hk.pkl` files are used in hk I wish was a little more streamlined—but this is more of an issue with hk than pkl.
* It's more complex than simple formats like yaml or toml and there is more to learn, however:
  * AI tools make this much easier since you can just ask cursor or whoever to help you write pkl
  * It's complex because it has a lot of features that don't exist in simple formats
* Some of the quirks of pkl I can't say I'm a fan of:
  * `List(a, b, c)` instead of `[a, b, c]`
  * `default` behavior is quite confusing
  * amending is a little weird

I have looked at many other esoteric languages for a long time now for hk and other projects though. IMO schema validation being built in
is an absolute killer feature that on its own is worth the tradeoffs. If you find yourself bristling at pkl, just remember that by using
features in pkl that means a lot of features didn't need to be implemented in hk—so you'll just be learning pkl features instead of hk features.

pkl itself is also young and being improved so I am optimistic they may add some syntax sugar that would address some of these problems—or
at least what I see as problems.

## Testing pkl config

While I strongly encourage setting up your editor with a pkl extension to view errors inside the editor, you can also use the pkl cli to evaluate pkl files which is a great way to see what pkl is outputting without needing to run it through hk:

```sh
$ pkl eval hk.pkl
hooks {
  ["pre-commit"] {
    fix = true
    steps {
      ["prelint"] {
        command = "lint"
        args = ["--fix"]
      }
    }
  }
}
```

Especially if you're doing dynamic configuration things I would strongly recommend doing this.

## Basic syntax

While of course pkl provides a [full reference](https://pkl-lang.org/main/current/language-reference/index.html), here I'll just show the pkl
concepts we use in hk.

### Basic Types

```pkl
my_string = "hello"
my_number = 1
my_boolean = true
list_of_strings = List("a", "b", "c")
```

### Mapping

Mappings are key-value pairs:

```pkl
my_mapping = new Mapping<String, String> {
  ["key"] = "value"
}
```

### Listings/Lists

Lists are for basic ordered collections:

```pkl
my_list = List("a", "b", "c")
```

Listings are for more complex ordered collections:

```pkl
my_listing = new Listing<Step> {
  new LinterStep {
    check = "make lint"
  }
  new LinterStep {
    check = "make format"
  }
}
```

### Local variables

hk will complain if you attempt to export variables that it doesn't expect, so you'll likely need to use the `local` keyword to create local variables:

```pkl
local my_step = new LinterStep {
  check = "make lint"
}
```

### Classes

You typically won't define your own class with an hk config, but you will instantiate the ones provided by [Config.pkl](https://github.com/jdx/hk/blob/main/pkl/Config.pkl):

```pkl
local my_step = new LinterStep {
  check = "make lint"
}
```

### Amending objects

If you want to use shared object but amend it with modifications, you do that with this syntax:

```pkl
local make_lint = new LinterStep {
  check = "make lint"
}
local linters = new Mapping<String, Step> {
  ["make-lint"] = (make_lint) {
    dir = "proj_a"
  }
  ["make-lint"] = (make_lint) {
    dir = "proj_b"
  }
}
```

Essentially this is the same as:

```pkl
local linters = new Mapping<String, Step> {
  ["make-lint"] = new LinterStep {
    check = "make lint"
    dir = "proj_a"
  }
  ["make-lint"] = new LinterStep {
    check = "make lint"
    dir = "proj_b"
  }
}
```

### Comments

```pkl
// This is a comment
/*
This is a multi-line comment
*/
/// This is a doc comment (not used by hk at least today)
```

### Amends

Every `hk.pkl` should start with this line which essentially schema validates the config and provides base classes:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
```

### Imports

Share code between files by importing:

```pkl
import "./extra.pkl"
# do something with `extra`

import "https://example.com/remote.pkl"
# do something with `remote`
```

## Caching

hk will cache the output of parsing each `hk.pkl` file until it is modified. For now, I would discouraging using features like env vars inside of `hk.pkl` files as the cache will not be invalidated if the env vars change. Perhaps this could be fixed somehow.
