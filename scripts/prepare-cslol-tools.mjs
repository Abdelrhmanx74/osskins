import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import AdmZip from "adm-zip";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const repoRoot = path.resolve(__dirname, "..");
const resourcesDir = path.join(repoRoot, "src-tauri", "resources");
const archivePath = path.join(resourcesDir, "cslol-tools.zip");
const toolsDir = path.join(resourcesDir, "cslol-tools");
const markerPath = path.join(toolsDir, ".prepared-from-archive.json");

const expectedFiles = [
  "cslol-diag.exe",
  "cslol-dll.dll",
  "hashes.game.txt",
  "mod-tools.exe",
  "wad-extract-multi.bat",
  "wad-extract.exe",
  "wad-make-multi.bat",
  "wad-make.exe",
  "wxy-extract-multi.bat",
].map((name) => path.join(toolsDir, name));

async function pathExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function readJsonIfExists(filePath) {
  if (!(await pathExists(filePath))) return null;
  try {
    const raw = await fs.readFile(filePath, "utf8");
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

async function validateExtracted() {
  const missing = [];
  for (const filePath of expectedFiles) {
    if (!(await pathExists(filePath))) missing.push(path.basename(filePath));
  }
  if (missing.length > 0) {
    throw new Error(
      `cslol-tools extracted but missing files: ${missing.join(", ")}`
    );
  }
}

async function main() {
  await fs.mkdir(resourcesDir, { recursive: true });

  const archiveExists = await pathExists(archivePath);
  if (!archiveExists) {
    throw new Error(
      `Missing archive at ${archivePath}. Expected resources/cslol-tools.zip to be present.`
    );
  }

  const archiveStat = await fs.stat(archivePath);
  const marker = await readJsonIfExists(markerPath);

  const markerMatchesArchive =
    marker &&
    marker.archiveSize === archiveStat.size &&
    marker.archiveMtimeMs === archiveStat.mtimeMs;

  if (markerMatchesArchive) {
    await validateExtracted();
    process.stdout.write("cslol-tools already prepared (up-to-date)\n");
    return;
  }

  await fs.rm(toolsDir, { recursive: true, force: true });

  const zip = new AdmZip(archivePath);
  zip.extractAllTo(resourcesDir, true);

  await validateExtracted();

  await fs.writeFile(
    markerPath,
    JSON.stringify(
      {
        archiveSize: archiveStat.size,
        archiveMtimeMs: archiveStat.mtimeMs,
        preparedAt: new Date().toISOString(),
      },
      null,
      2
    ) + "\n",
    "utf8"
  );

  process.stdout.write("cslol-tools prepared\n");
}

await main();
