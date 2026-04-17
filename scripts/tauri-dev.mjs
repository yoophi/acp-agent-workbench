import { spawn } from "node:child_process";
import net from "node:net";

const DEFAULT_PORT = 1420;
const MAX_PORT_ATTEMPTS = 100;

function parsePort(value, fallback) {
  if (!value) {
    return fallback;
  }

  const port = Number.parseInt(value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`Invalid port: ${value}`);
  }

  return port;
}

function canListen(host, port) {
  return new Promise((resolve) => {
    const server = net.createServer();

    server.once("error", () => {
      resolve(false);
    });

    server.once("listening", () => {
      server.close(() => {
        resolve(true);
      });
    });

    server.listen(port, host);
  });
}

async function findAvailablePort(host, startPort) {
  for (let offset = 0; offset < MAX_PORT_ATTEMPTS; offset += 1) {
    const port = startPort + offset;
    if (port > 65535) {
      break;
    }

    if (await canListen(host, port)) {
      return port;
    }
  }

  throw new Error(
    `No available development port found from ${startPort} to ${Math.min(
      startPort + MAX_PORT_ATTEMPTS - 1,
      65535,
    )}`,
  );
}

function createTauriDevConfig(host, port) {
  const urlHost = host === "0.0.0.0" ? "127.0.0.1" : host;

  return {
    build: {
      beforeDevCommand: `npm run dev -- --host ${host} --port ${port} --strictPort`,
      devUrl: `http://${urlHost}:${port}`,
    },
  };
}

const passthroughArgs = process.argv.slice(2);
const printConfig = passthroughArgs.includes("--print-config");
const tauriArgs = passthroughArgs.filter((arg) => arg !== "--print-config");
const host = process.env.TAURI_DEV_HOST ?? "127.0.0.1";
const requestedPort = parsePort(
  process.env.TAURI_DEV_PORT ?? process.env.VITE_DEV_SERVER_PORT,
  DEFAULT_PORT,
);
const port = await findAvailablePort(host, requestedPort);
const config = createTauriDevConfig(host, port);

if (printConfig) {
  console.log(JSON.stringify(config, null, 2));
  process.exit(0);
}

console.log(`Starting Tauri dev app at ${config.build.devUrl}`);

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const child = spawn(
  npmCommand,
  ["run", "tauri", "--", "dev", "--config", JSON.stringify(config), ...tauriArgs],
  {
    stdio: "inherit",
    env: {
      ...process.env,
      TAURI_DEV_HOST: host,
      TAURI_DEV_PORT: String(port),
      VITE_DEV_SERVER_PORT: String(port),
    },
  },
);

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 0);
});
