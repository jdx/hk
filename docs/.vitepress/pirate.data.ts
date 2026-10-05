import { missingVariants } from "./pirate";

export interface PirateData {
  /** English pages with no pirate variant, as theme/shanty-mode.ts `fileKey`s; normally none. */
  missing: string[];
}

export declare const data: PirateData;

export default {
  watch: ["../pirate/**/*.md"],
  load: (): PirateData => ({ missing: missingVariants() }),
};
