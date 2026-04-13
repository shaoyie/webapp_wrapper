import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const configPath = path.join(repoRoot, "app-profiles", "apps.json");
const generatedIconsDir = path.join(repoRoot, "src-tauri", "generated-icons");
const profileDumpPath = path.join(repoRoot, "src-tauri", ".profile.json");

const args = process.argv.slice(2);

function pickArg(key) {
  const prefixed = `--${key}`;
  for (let i = 0; i < args.length; i += 1) {
    const current = args[i];
    if (current === prefixed) {
      return args[i + 1];
    }
    if (current.startsWith(`${prefixed}=`)) {
      return current.split("=")[1];
    }
  }
  return undefined;
}

const cliAppId = pickArg("app-id");
const envAppId = process.env.APP_ID;

const configRaw = await readFile(configPath, "utf8");
const config = JSON.parse(configRaw);
const profiles = config.profiles || {};
const defaultAppId = config.defaultAppId;

if (!defaultAppId || !profiles[defaultAppId]) {
  throw new Error("apps.json must define a valid defaultAppId");
}

const requestedId = cliAppId || envAppId;
let selectedId = requestedId && profiles[requestedId] ? requestedId : defaultAppId;

if (requestedId && !profiles[requestedId]) {
  console.warn(
    `[prepare-app] Unknown appId '${requestedId}', falling back to '${defaultAppId}'.`
  );
}

const profile = profiles[selectedId];

if (!profile?.iconDir) {
  throw new Error(`Profile '${selectedId}' is missing iconDir.`);
}

const iconSource = path.isAbsolute(profile.iconDir)
  ? profile.iconDir
  : path.join(repoRoot, profile.iconDir);

await rm(generatedIconsDir, { recursive: true, force: true });
await mkdir(generatedIconsDir, { recursive: true });
await cp(iconSource, generatedIconsDir, { recursive: true });

await writeFile(
  profileDumpPath,
  JSON.stringify(
    {
      appId: selectedId,
      profile,
      generatedAt: new Date().toISOString(),
    },
    null,
    2
  )
);

console.log(
  `[prepare-app] active appId: ${selectedId}, appName: ${profile.appName}, url: ${profile.appUrl}`
);
