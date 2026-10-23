// Biome formats and lints this project (CES-58). ESLint runs only the three rules of
// eslint-plugin-jsdoc that compare a JSDoc comment with the code it documents (CES-123).
import tsParser from "@typescript-eslint/parser";
import jsdoc from "eslint-plugin-jsdoc";

export default [
  {
    files: ["**/*.{js,jsx,mjs,cjs,ts,tsx,mts,cts}"],
    languageOptions: { parser: tsParser },
    plugins: { jsdoc },
    // The block tags of TSDoc that eslint-plugin-jsdoc does not know or renames.
    settings: {
      jsdoc: { tagNamePreference: { typeParam: "typeParam", virtual: "virtual" } },
    },
    rules: {
      "jsdoc/check-param-names": "error",
      "jsdoc/check-tag-names": ["error", { definedTags: ["defaultValue", "label"] }],
      "jsdoc/require-returns-check": "error",
    },
  },
];
