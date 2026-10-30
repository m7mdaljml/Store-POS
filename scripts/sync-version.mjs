import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const pkg = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
);
const version = pkg.version;

const confUrl = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const confPath = fileURLToPath(confUrl);
const conf = readFileSync(confUrl, "utf8");
const updated = conf.replace(
  /("version"\s*:\s*")[^"]*(")/,
  (_, start, end) => `${start}${version}${end}`,
);

if (updated !== conf) {
  writeFileSync(confUrl, updated);
}

try {
  execFileSync("git", ["add", confPath], { cwd: root, stdio: "ignore" });
} catch {
  // not a git repo or git is unavailable
}

console.log(`Synced app version to ${version}`);
