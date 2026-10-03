# Security policy

## Reporting a vulnerability

Please do not open a public issue or discussion for a suspected vulnerability.

Use the repository's **Security** tab on GitHub and choose **Report a vulnerability** to send a private report to the maintainers. If that option is not available, open a public issue that says only that you have a security report and asks for a private way to send it. Leave the details out of the issue.

Include the hk version (`hk version`), your operating system, and the steps to reproduce.

## What is in scope

hk runs the commands in a project's `hk.pkl`, so running hk in a repository you do not trust runs that repository's code. That is how hk works, not a vulnerability by itself. [Security model](https://hk.jdx.dev/security) describes what hk executes, what Pkl evaluation can read and fetch, how global hooks reach every repository, and what `--safe` does and does not guarantee. Read it before reporting behavior it already documents.
