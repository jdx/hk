import globals from "globals";
import js from "@eslint/js";


/** @type {import('eslint').Linter.Config[]} */
export default [
  {languageOptions: { globals: globals.node }},
  js.configs.recommended,
  {rules:{
    semi: ["error", "always"],
  }}
];
