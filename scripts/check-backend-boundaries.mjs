#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const backendSrcRoot = path.join(root, "src-tauri", "src");
const runSelfTest = process.argv.includes("--self-test");
const rustExtension = ".rs";

const layers = new Set(["domain", "ports", "application", "adapters"]);
const forbiddenImports = {
  domain: new Set(["ports", "application", "adapters"]),
  ports: new Set(["application", "adapters"]),
  application: new Set(["adapters"]),
  adapters: new Set(),
};

const violations = [];

if (runSelfTest) {
  runBoundarySelfTest();
} else {
  for (const file of walk(backendSrcRoot)) {
    const fromLayer = getLayer(file);
    if (!fromLayer) continue;

    const relativeFile = toPosix(path.relative(root, file));
    const source = fs.readFileSync(file, "utf8");
    const importLayers = collectUseImportLayers(source);
    for (const importedLayer of importLayers) {
      if (!forbiddenImports[fromLayer].has(importedLayer)) continue;
      violations.push({
        file: relativeFile,
        message: `${fromLayer} cannot import from crate::${importedLayer}. Allowed direction is adapters -> application -> ports -> domain.`,
      });
    }
  }
}

if (violations.length > 0) {
  console.error("Backend boundary check failed:\n");
  for (const violation of violations) {
    console.error(`- ${violation.file}`);
    console.error(`  ${violation.message}`);
  }
  process.exit(1);
}

console.log("Backend boundary check passed.");

function runBoundarySelfTest() {
  const cases = [
    {
      name: "domain cannot import ports",
      fromLayer: "domain",
      source: "use crate::ports::session_registry::SessionRegistry;",
      expected: ["ports"],
    },
    {
      name: "ports can import domain",
      fromLayer: "ports",
      source: "use crate::domain::events::RunEvent;",
      expected: [],
    },
    {
      name: "application cannot import grouped adapters",
      fromLayer: "application",
      source: "use crate::{adapters::session_registry::AppState, ports::event_sink::RunEventSink};",
      expected: ["adapters"],
    },
    {
      name: "adapters can import application and ports",
      fromLayer: "adapters",
      source: "use crate::{application::start_agent_run::StartAgentRunUseCase, ports::{event_sink::RunEventSink, session_registry::SessionRegistry}};",
      expected: [],
    },
    {
      name: "line and block comments are ignored",
      fromLayer: "ports",
      source: `
        // use crate::adapters::fs::LocalGoalFileReader;
        /*
          use crate::application::list_agents::ListAgentsUseCase;
        */
        use crate::domain::agent::AgentDescriptor;
      `,
      expected: [],
    },
    {
      name: "restricted visibility use statements are checked",
      fromLayer: "application",
      source: "pub(crate) use crate::adapters::session_registry::AppState;",
      expected: ["adapters"],
    },
  ];

  for (const testCase of cases) {
    const before = violations.length;
    const actual = collectUseImportLayers(testCase.source)
      .filter((importedLayer) => forbiddenImports[testCase.fromLayer].has(importedLayer))
      .sort();
    const expected = testCase.expected.toSorted();

    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      throw new Error(
        `Self-test failed for "${testCase.name}": expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}.`,
      );
    }
    violations.splice(before);
  }

  console.log("Backend boundary self-test passed.");
}

function* walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      yield* walk(fullPath);
    } else if (path.extname(entry.name) === rustExtension) {
      yield fullPath;
    }
  }
}

function getLayer(file) {
  const relative = toPosix(path.relative(backendSrcRoot, file));
  const [layer] = relative.split("/");
  return layers.has(layer) ? layer : undefined;
}

function collectUseImportLayers(source) {
  const cleanedSource = stripRustComments(source);
  const importedLayers = new Set();
  const useStatements = /\b(?:pub(?:\s*\([^)]*\))?\s+)?use\s+([\s\S]*?);/g;

  for (const match of cleanedSource.matchAll(useStatements)) {
    const useTree = match[1];
    collectDirectCrateImports(useTree, importedLayers);
    collectGroupedCrateImports(useTree, importedLayers);
  }

  return [...importedLayers];
}

function collectDirectCrateImports(useTree, importedLayers) {
  const directCrateImport = /crate\s*::\s*(domain|ports|application|adapters)\b/g;
  for (const match of useTree.matchAll(directCrateImport)) {
    importedLayers.add(match[1]);
  }
}

function collectGroupedCrateImports(useTree, importedLayers) {
  const groupedRoot = /crate\s*::\s*\{([\s\S]*)\}/g;
  for (const match of useTree.matchAll(groupedRoot)) {
    const groupBody = match[1];
    const rootLayerImport = /(?:^|[,{])\s*(domain|ports|application|adapters)\b/g;
    for (const layerMatch of groupBody.matchAll(rootLayerImport)) {
      importedLayers.add(layerMatch[1]);
    }
  }
}

function stripRustComments(source) {
  let output = "";
  let i = 0;
  let blockDepth = 0;
  let inLineComment = false;

  while (i < source.length) {
    const current = source[i];
    const next = source[i + 1];

    if (inLineComment) {
      if (current === "\n") {
        inLineComment = false;
        output += current;
      }
      i += 1;
      continue;
    }

    if (blockDepth > 0) {
      if (current === "/" && next === "*") {
        blockDepth += 1;
        i += 2;
      } else if (current === "*" && next === "/") {
        blockDepth -= 1;
        i += 2;
      } else {
        if (current === "\n") output += current;
        i += 1;
      }
      continue;
    }

    if (current === "/" && next === "/") {
      inLineComment = true;
      i += 2;
      continue;
    }

    if (current === "/" && next === "*") {
      blockDepth = 1;
      i += 2;
      continue;
    }

    output += current;
    i += 1;
  }

  return output;
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}
