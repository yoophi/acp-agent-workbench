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
    {
      name: "restricted visibility use statements are checked",
      layer: "application",
      source: "pub(crate) use crate::adapters::session_registry::AppState;",
      expected: 1,
    },
    {
      name: "line and block comments are ignored",
      layer: "ports",
      source: `
        // use crate::adapters::fs::LocalGoalFileReader;
        /*
          let _ = crate::application::list_agents::ListAgentsUseCase;
        */
        use crate::domain::agent::AgentDescriptor;
      `,
      expected: 0,
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
  const sourceWithoutComments = stripRustComments(source);
  const forbiddenLayers = layerRules.get(layer) ?? new Set();
  for (const forbidden of forbiddenLayers) {
    if (referencesCrateLayer(sourceWithoutComments, forbidden)) {
      violations.push({
        file,
        message: `${layer}/ cannot depend on crate::${forbidden}. Allowed direction is domain -> ports -> application -> adapters.`,
      });
    }
  }
}

function referencesCrateLayer(source, layer) {
  return (
    new RegExp(String.raw`\bcrate::${layer}\s*(?:::|\{|\b)`).test(source) ||
    collectCrateUseBodies(source).some((body) => groupedUseReferencesLayer(body, layer))
  );
}

function collectCrateUseBodies(source) {
  const bodies = [];
  const useCrate = /\b(?:pub(?:\s*\([^)]*\))?\s+)?use\s+crate::([\s\S]*?);/g;
  for (const match of source.matchAll(useCrate)) {
    bodies.push(match[1]);
  }
  return bodies;
}

function groupedUseReferencesLayer(body, layer) {
  const trimmed = body.trim();
  if (!trimmed.startsWith("{")) return false;
  return new RegExp(String.raw`(?:^|[,{]\s*)${layer}\s*(?:::|\{|\b)`).test(trimmed);
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
