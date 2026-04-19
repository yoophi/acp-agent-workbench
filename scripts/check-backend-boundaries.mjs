#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const backendSrc = path.join(root, "src-tauri", "src");
const runSelfTest = process.argv.includes("--self-test");
const sourceExtensions = new Set([".rs"]);
const layerRules = new Map([
  ["domain", new Set(["ports", "application", "adapters"])],
  ["ports", new Set(["application", "adapters"])],
  ["application", new Set(["adapters"])],
]);

const violations = [];

if (runSelfTest) {
  runBoundarySelfTest();
} else {
  for (const file of walk(backendSrc)) {
    const layer = getLayer(file);
    if (!layerRules.has(layer)) continue;
    validateSource(toPosix(path.relative(root, file)), layer, fs.readFileSync(file, "utf8"));
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
      layer: "domain",
      source: "use crate::ports::event_sink::RunEventSink;",
      expected: 1,
    },
    {
      name: "ports can import domain",
      layer: "ports",
      source: "use crate::domain::events::RunEvent;",
      expected: 0,
    },
    {
      name: "application cannot import grouped adapters",
      layer: "application",
      source: "use crate::{adapters::session_registry::AppState, ports::session_registry::SessionRegistry};",
      expected: 1,
    },
    {
      name: "application can import ports and domain",
      layer: "application",
      source: "use crate::{domain::run::AgentRun, ports::session_registry::SessionRegistry};",
      expected: 0,
    },
    {
      name: "non-use fully qualified adapter path is rejected",
      layer: "application",
      source: "let _ = crate::adapters::session_registry::AppState::default();",
      expected: 1,
    },
  ];

  for (const testCase of cases) {
    const before = violations.length;
    validateSource(`${testCase.layer}/sample.rs`, testCase.layer, testCase.source);
    const added = violations.length - before;
    if (added !== testCase.expected) {
      throw new Error(
        `Self-test failed for "${testCase.name}": expected ${testCase.expected} violation(s), got ${added}.`,
      );
    }
    violations.splice(before, added);
  }

  console.log("Backend boundary self-test passed.");
}

function validateSource(file, layer, source) {
  const forbiddenLayers = layerRules.get(layer) ?? new Set();
  for (const forbidden of forbiddenLayers) {
    if (referencesCrateLayer(source, forbidden)) {
      violations.push({
        file,
        message: `${layer}/ cannot depend on crate::${forbidden}. Allowed direction is domain -> ports -> application -> adapters.`,
      });
    }
  }
}

function referencesCrateLayer(source, layer) {
  return (
    new RegExp(String.raw`\bcrate::${layer}\s*(?:::|\{)`).test(source) ||
    collectCrateUseBodies(source).some((body) => groupedUseReferencesLayer(body, layer))
  );
}

function collectCrateUseBodies(source) {
  const bodies = [];
  const useCrate = /\buse\s+crate::([\s\S]*?);/g;
  for (const match of source.matchAll(useCrate)) {
    bodies.push(match[1]);
  }
  return bodies;
}

function groupedUseReferencesLayer(body, layer) {
  const trimmed = body.trim();
  if (!trimmed.startsWith("{")) return false;
  return new RegExp(String.raw`(?:^|[,{]\s*)${layer}\s*(?:::|\{)`).test(trimmed);
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

function getLayer(file) {
  const relative = toPosix(path.relative(backendSrc, file));
  return relative.split("/")[0];
}

function toPosix(value) {
  return value.split(path.sep).join("/");
}
