import * as docker from "@pulumi/docker";
import * as pulumi from "@pulumi/pulumi";
import { network } from "./network";
import { prefixedContainerName } from "./container-names";
import { postgresContainer } from "./postgres";
import { kafkaContainer } from "./kafka";
import { keycloakContainer } from "./keycloak";
import { keycloakConfigContainer } from "./keycloak-config";
import { topicProvisioning } from "./kafka-topics";
import { userApiContainer } from "./user-api";
import { workoutApiContainer } from "./workout-api";
import { exerciseApiContainer } from "./exercise-api";
import { ports, enableFullstack } from "./ports";
import { loadServiceEnv } from "./load-service-env";
import * as path from "path";

const stackName = pulumi.getStack();
const useIngressProxy = stackName === "e2e";

/**
 * Fullstack Dioxus Server Container (Optional)
 *
 * The unified frontend + BFF server for SSR, hydration, and API handling.
 *
 * Automatically enabled for e2e stack (browser tests need it).
 * For other stacks: pulumi config set enableFullstack true
 */

const config = new pulumi.Config();

// Application environment (controls Swagger UI visibility)
const appEnv = config.get("appEnv") ?? "local";

// SECURITY: Keycloak client secret from Pulumi secrets
const keycloakClientSecret = config.getSecret("keycloakClientSecret") ?? "repforge-api-secret";

// Docker build platform
const dockerPlatformOverride = config.get("dockerPlatform");
const dockerPlatformDetected = process.arch === "arm64" ? "linux/arm64" : "linux/amd64";
const dockerPlatform = dockerPlatformOverride ?? dockerPlatformDetected;

// Build the Docker image using the fullstack Dockerfile (only if enabled)
// SOURCE_HASH forces Pulumi to trigger a Docker build on every `pulumi up`,
// ensuring source code changes are always picked up. Docker layer caching
// still applies for dependency stages (before ARG SOURCE_HASH in Dockerfile).
export const fullstackImage = enableFullstack
  ? new docker.Image(
      "fullstack-image",
      {
        imageName: "repforge/fullstack:latest",
        build: {
          context: path.resolve(__dirname, "../../code"),
          dockerfile: path.resolve(__dirname, "../../code/Dockerfile.fullstack"),
          platform: dockerPlatform,
          builderVersion: docker.BuilderVersion.BuilderBuildKit,
          args: {
            SOURCE_HASH: Date.now().toString(),
            REPFORGE_E2E_PROBE_ENABLED: useIngressProxy ? "1" : "0",
            DIOXUS_BUILD_PROFILE: useIngressProxy ? "debug" : "release",
          },
        },
        skipPush: true,
      },
      {
        dependsOn: [network],
      },
    )
  : undefined;

function replicaResourceNames(): string[] {
  if (useIngressProxy) {
    return ["fullstack-a", "fullstack-b"];
  }
  return ["fullstack"];
}

function networkAliasesFor(resourceName: string): string[] | undefined {
  if (!useIngressProxy) {
    return undefined;
  }
  // Keep "fullstack" alias for compatibility with configs/scripts that still
  // reference the legacy single-instance host name.
  if (resourceName === "fullstack-a") {
    return ["fullstack-a", "fullstack"];
  }
  return [resourceName];
}

const commonDependsOnRaw = [
  postgresContainer,
  kafkaContainer,
  keycloakContainer,
  keycloakConfigContainer,
  topicProvisioning,
  userApiContainer,
  workoutApiContainer,
  exerciseApiContainer,
];

const commonDependsOn = commonDependsOnRaw.filter(
  (resource): resource is Exclude<(typeof commonDependsOnRaw)[number], undefined> =>
    resource !== undefined,
);

export const fullstackContainers: docker.Container[] =
  enableFullstack && fullstackImage
    ? replicaResourceNames().map(
        (resourceName) =>
          new docker.Container(
            resourceName,
            {
              name: prefixedContainerName(resourceName),
              // Use the stable local tag for container startup. When skipPush is true,
              // repoDigest can resolve to a local image ID that the Docker provider then
              // mistakenly treats as a remote image name during replacement.
              image: fullstackImage.imageName,

              networksAdvanced: [
                {
                  name: network.name,
                  ...(useIngressProxy ? { aliases: networkAliasesFor(resourceName) } : {}),
                },
              ],

              envs: loadServiceEnv("fullstack", {
                APP_KEYCLOAK_CLIENT_SECRET: pulumi.interpolate`${keycloakClientSecret}`,
                APP_INSTANCE_ID: resourceName,
                APP_ENV: appEnv,
                REPFORGE_E2E_PROBE_ENABLED: useIngressProxy ? "1" : "0",
                APP_IMAGE_DIGEST: pulumi.interpolate`${fullstackImage.repoDigest}`,
              }),

              ...(useIngressProxy
                ? {}
                : {
                    ports: [
                      {
                        internal: 7081,
                        external: ports.fullstack,
                      },
                    ],
                  }),

              // Health check
              healthcheck: {
                tests: [
                  "CMD",
                  "wget",
                  "--no-verbose",
                  "--tries=1",
                  "--spider",
                  "http://localhost:7081/health",
                ],
                interval: "10s",
                timeout: "5s",
                retries: 5,
                startPeriod: "60s",
              },

              restart: "unless-stopped",
            },
            {
              dependsOn: commonDependsOn,
            },
          ),
      )
    : [];

// Backward-compatible single-container export for callers that expect one.
export const fullstackContainer =
  fullstackContainers.length > 0 ? fullstackContainers[0] : undefined;

export const fullstackContainerNames = fullstackContainers.map((container) => container.name);

// Export the URL for accessing fullstack (undefined if not enabled)
export const fullstackUrl = enableFullstack ? `http://localhost:${ports.fullstack}` : undefined;
