import {defineConfig} from 'eslint/config';
import solid from 'eslint-plugin-solid/configs/typescript';
import tseslint from 'typescript-eslint';

export default defineConfig(
  {files: ['src/**/*.{ts,tsx}'], extends: [tseslint.configs.recommended]},
  {files: ['src/**/*.tsx'], extends: [solid]}
);
