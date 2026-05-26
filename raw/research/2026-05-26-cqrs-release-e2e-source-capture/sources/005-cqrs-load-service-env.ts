import * as pulumi from "@pulumi/pulumi";
import * as fs from "fs";
import * as path from "path";

/**
 * Stack-to-environment mapping.
 * Pulumi stacks use names like "cqrs"; profile files use environment names like "dev".
 * The committed dev/docker profiles currently contain cqrs-* Docker DNS names,
 * so M1 supports only the historical cqrs dev stack plus e2e.
 */
const STACK_ENV_MAP: Record<string, string> = {
  cqrs: "dev",
  e2e: "e2e",
};

function resolveEnvironment(): string {
  const stack = pulumi.getStack();
  const env = STACK_ENV_MAP[stack];
  if (!env) {
    throw new Error(
      `No environment mapping for stack '${stack}'. Known stacks: ${Object.keys(STACK_ENV_MAP).join(", ")}`,
    );
  }
  return env;
}

/**
 * Load a service profile from config/runtime and return an envs array for Docker containers.
 *
 * Static values are read from the profile file. Dynamic overrides (e.g., image digests,
 * APP_ENV) are merged on top — overrides take precedence and can add new keys.
 *
 * @param service   - Service name matching the profile file (e.g., "exercise-api")
 * @param overrides - Dynamic values computed by Pulumi at deploy time
 * @returns `pulumi.Input<string>[]` suitable for docker.Container `envs`
 */
export function loadServiceEnv(
  service: string,
  overrides?: Record<string, pulumi.Input<string>>,
): pulumi.Input<string>[] {
  const env = resolveEnvironment();
  const profilePath = path.resolve(__dirname, `../../config/runtime/${env}/docker/${service}.env`);

  if (!fs.existsSync(profilePath)) {
    throw new Error(`Profile not found: ${profilePath}`);
  }

  const content = fs.readFileSync(profilePath, "utf-8");
  const entries = new Map<string, pulumi.Input<string>>();

  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const eqIndex = trimmed.indexOf("=");
    if (eqIndex < 0) continue;

    const key = trimmed.substring(0, eqIndex);
    const value = trimmed.substring(eqIndex + 1);
    entries.set(key, value);
  }

  // Validate required keys
  const requiredPath = path.resolve(__dirname, `../../config/runtime/${service}.required`);
  if (fs.existsSync(requiredPath)) {
    const requiredContent = fs.readFileSync(requiredPath, "utf-8");
    const missing: string[] = [];
    for (const line of requiredContent.split("\n")) {
      const key = line.trim();
      if (!key) continue;
      if (!entries.has(key) && !(overrides && key in overrides)) {
        missing.push(key);
      }
    }
    if (missing.length > 0) {
      throw new Error(`Profile ${profilePath} is missing required keys: ${missing.join(", ")}`);
    }
  }

  // Merge dynamic overrides
  if (overrides) {
    for (const [key, value] of Object.entries(overrides)) {
      entries.set(key, value);
    }
  }

  // Convert to KEY=value array
  const result: pulumi.Input<string>[] = [];
  for (const [key, value] of entries) {
    if (typeof value === "string") {
      result.push(`${key}=${value}`);
    } else {
      // pulumi.Output<string> — use interpolate to build the KEY=value string
      result.push(pulumi.interpolate`${key}=${value}`);
    }
  }

  return result;
}
