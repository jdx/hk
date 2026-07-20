import globals from "globals";


/** @type {import('eslint').Linter.Config[]} */
export default [
  {files: ["**/*.{js,mjs,cjs}"]},
  {ignores: [".vitepress/**/*"]},
  {languageOptions: { globals: globals.browser }},
];
