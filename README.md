# hk

A git hook manager and project linting tool with an emphasis on performance. Compared to other
git hook managers, hk has tighter integration with linters and is able to make use of read/write
file locks in order to maximize concurrency while also preventing race conditions.

See docs: https://hk.jdx.dev/

## Demo

![hk demo](docs/public/hk-demo.gif)

## CI

<p>
  <a href="https://namespace.so">
    <img src="docs/public/namespace-logo.svg" alt="Namespace" width="64">
  </a>
</p>

Thanks to [Namespace](https://namespace.so) for providing CI for hk.
