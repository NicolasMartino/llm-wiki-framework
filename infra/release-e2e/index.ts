import * as docker from "@pulumi/docker";
import * as pulumi from "@pulumi/pulumi";

const stackName = pulumi.getStack();
const config = new pulumi.Config();

const configuredNetworkName = config.get("networkName");
const configuredDockerImage = config.get("dockerImage");
const configuredDockerPlatform = config.get("dockerPlatform");

export const containerNamePrefix = `llm-wiki-release-e2e-${stackName}`;
export const dockerImage = configuredDockerImage ?? "debian:bookworm-slim";
export const dockerPlatform = configuredDockerPlatform ?? "linux/arm64";
export const artifactMountRoot = "/artifact";
export const homeMountRoot = "/home/e2e";
export const runMountRoot = "/work/run";

const network = new docker.Network("release-e2e-net", {
  name: configuredNetworkName ?? `${containerNamePrefix}-net`,
  driver: "bridge",
  internal: true,
  attachable: true
});

export const networkName = network.name;
