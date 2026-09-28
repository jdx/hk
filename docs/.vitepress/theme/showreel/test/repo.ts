// The checkout the tests run in. The runner bundles each test into a
// temporary directory, so the path comes from the working directory: docs/
// under `aube run`, or anywhere else inside the checkout.

import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const REPO = (() => {
  for (let dir = resolve(process.cwd()); ; dir = dirname(dir)) {
    if (existsSync(join(dir, "pkl/Config.pkl")) && existsSync(join(dir, "docs/.vitepress/theme/showreel"))) return dir;
    if (dirname(dir) === dir) throw new Error(`no hk checkout at or above ${process.cwd()}`);
  }
})();

/** The showreel's source directory. */
export const SHOWREEL = join(REPO, "docs/.vitepress/theme/showreel");
