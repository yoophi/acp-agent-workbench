#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const srcRoot = path.join(root, "src");
const runSelfTest = process.argv.includes("--self-test");
const sourceExtensions = new Set([".ts", ".tsx"]);
const layerOrder = new Map([
  ["shared", 0],
  ["entities", 1],
  ["features", 2],
  ["widgets", 3],
  ["pages", 4],
  ["app", 5],
]);

const publicApiOnlyLayers = new Set(["entities", "features", "widgets"]);

const violations = [];

if (runSelfTest) {
  runBoundarySelfTest();
} else {
  for (const file of walk(srcRoot)) {
    const relativeFile = toPosix(path.relative(root, file));
    const source = fs.readFileSync(file, "utf8");
    const froms = collectImportSpecifiers(source);
    for (const specifier of froms) {
      if (!isLocalImport(specifier)) continue;
      const imported = resolveImport(file, specifier);
      if (!imported) continue;
      const fromModule = getModuleInfo(file);
      const toModule = getModuleInfo(imported);
      if (!fromModule || !toModule) continue;

      validateLayerDirection(relativeFile, specifier, fromModule, toModule);
      validatePublicApi(relativeFile, specifier, fromModule, toModule, imported);
    }
  }
}

if (violations.length > 0) {
  console.error("FSD boundary check failed:\n");
  for (const violation of violations) {
    console.error(`- ${violation.file}`);
    console.error(`  import ${JSON.stringify(violation.specifier)}`);
    console.error(`  ${violation.message}`);
  }
  process.exit(1);
}

console.log("FSD boundary check passed.");

function runBoundarySelfTest() {
  runSpecifierParserSelfTest();

  const cases = [
    {
      name: "shared cannot import entities",
      fromFile: path.join(srcRoot, "shared", "api", "tauri.ts"),
      imported: path.join(srcRoot, "entities", "message", "index.ts"),
      specifier: "../../entities/message",
      expected: 1,
    },
    {
      name: "features cannot import widgets",
      fromFile: path.join(srcRoot, "features", "agent-run", "api.ts"),
      imported: path.join(srcRoot, "widgets", "event-stream", "index.ts"),
      specifier: "../../widgets/event-stream",
      expected: 1,
    },
    {
      name: "widgets must use feature public API",
      fromFile: path.join(srcRoot, "widgets", "event-stream", "EventStream.tsx"),
      imported: path.join(srcRoot, "features", "permission-response", "usePermissionResponse.ts"),
      specifier: "../../features/permission-response/usePermissionResponse",
      expected: 1,
    },
    {
      name: "widgets can import feature public API",
      fromFile: path.join(srcRoot, "widgets", "event-stream", "EventStream.tsx"),
      imported: path.join(srcRoot, "features", "permission-response", "index.ts"),
      specifier: "../../features/permission-response",
      expected: 0,
    },
  ];

  for (const testCase of cases) {
    const before = violations.length;
    validateImport(
      path.relative(root, testCase.fromFile),
      testCase.specifier,
      testCase.fromFile,
      testCase.imported,
    );
    const added = violations.length - before;
    if (added !== testCase.expected) {
      throw new Error(
        `Self-test failed for "${testCase.name}": expected ${testCase.expected} violation(s), got ${added}.`,
      );
    }
    violations.splice(before, added);
  }

  console.log("FSD boundary self-test passed.");
}

function runSpecifierParserSelfTest() {
  const source = `
    import React from "react";
    import { eventGroups } from "../../entities/message";
    import type { TimelineItem } from "../../entities/message";
    import "../../app/styles.css";
    const lazy = import("../../widgets/event-stream");
    export { useAgentRun } from "../../features/agent-run";
    export type { AgentDescriptor } from "../../entities/agent";
    export * from "../../shared/ui";
  `;
  const actual = collectImportSpecifiers(source);
  const expected = [
    "react",
    "../../entities/message",
    "../../entities/message",
    "../../app/styles.css",
    "../../widgets/event-stream",
    "../../features/agent-run",
    "../../entities/agent",
    "../../shared/ui",
  ];

  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `Self-test failed for import specifier parsing: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}.`,
    );
  }
}

function validateImport(relativeFile, specifier, fromFile, imported) {
  const fromModule = getModuleInfo(fromFile);
  const toModule = getModuleInfo(imported);
  if (!fromModule || !toModule) return;
  validateLayerDirection(toPosix(relativeFile), specifier, fromModule, toModule);
  validatePublicApi(toPosix(relativeFile), specifier, fromModule, toModule, imported);
}

function* walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      yield* walk(fullPath);
    } else if (sourceExtensions.has(path.extname(entry.name))) {
      yield fullPath;
    }
  }
}

function collectImportSpecifiers(source) {
  const specifiers = [];
  const importFrom = /^\s*import\s+(?!["'(])(?:type\s+)?[\s\S]*?\s+from\s+["']([^"']+)["']/gm;
  const sideEffectImport = /^\s*import\s+["']([^"']+)["']/gm;
  const dynamicImport = /import\s*\(\s*["']([^"']+)["']\s*\)/g;
  const reExportFrom = /^\s*export\s+(?:type\s+)?(?:\*|\{[\s\S]*?\})\s+from\s+["']([^"']+)["']/gm;

  for (const match of source.matchAll(importFrom)) {
    specifiers.push(match[1]);
  }
  for (const match of source.matchAll(sideEffectImport)) {
    specifiers.push(match[1]);
  }
  for (const match of source.matchAll(dynamicImport)) {
    specifiers.push(match[1]);
  }
  for (const match of source.matchAll(reExportFrom)) {
    specifiers.push(match[1]);
  }

  return specifiers;
}

function isLocalImport(specifier) {
  return specifier.startsWith(".");
}

function resolveImport(fromFile, specifier) {
  const base = path.resolve(path.dirname(fromFile), specifier);
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    path.join(base, "index.ts"),
    path.join(base, "index.tsx"),
  ];
  return candidates.find((candidate) => fs.existsSync(candidate) && fs.statSync(candidate).isFile());
}

function getModuleInfo(file) {
  const relative = toPosix(path.relative(srcRoot, file));
  const segments = relative.split("/");
  const layer = segments[0];
  if (!layerOrder.has(layer)) return undefined;

  return {
    layer,
    order: layerOrder.get(layer),
    slice: getSliceName(layer, segments),
    relative,
  };
}

function getSliceName(layer, segments) {
  if (layer === "app") return "app";
  if (layer === "shared") return segments[1] ?? "shared";
  return segments[1] ?? layer;
}

function validateLayerDirection(file, specifier, fromModule, toModule) {
  if (fromModule.layer === toModule.layer) return;
  if (fromModule.order >= toModule.order) return;

  violations.push({
    file,
    specifier,
    message: `${fromModule.layer} cannot import upward from ${toModule.layer}. Allowed direction is app -> pages -> widgets -> features -> entities -> shared.`,
  });
}

function validatePublicApi(file, specifier, fromModule, toModule, imported) {
  if (fromModule.layer === toModule.layer && fromModule.slice === toModule.slice) return;
  if (!publicApiOnlyLayers.has(toModule.layer)) return;
  if (isSlicePublicApi(imported, toModule)) return;

  violations.push({
    file,
    specifier,
    message: `Cross-slice imports from ${toModule.layer}/${toModule.slice} must use its public index.ts API.`,
  });
}

function isSlicePublicApi(imported, moduleInfo) {
  const expected = path.join(srcRoot, moduleInfo.layer, moduleInfo.slice, "index.ts");
  return path.normalize(imported) === path.normalize(expected);
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}
