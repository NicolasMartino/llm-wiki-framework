import * as docker from "@pulumi/docker";
import * as pulumi from "@pulumi/pulumi";

// Get stack name for isolation (e.g., "cqrs" for dev, "e2e" for testing)
const stackName = pulumi.getStack();
export const containerNamePrefix = stackName;

/**
 * Docker network for RepForge infrastructure.
 *
 * Network name is stack-specific to allow multiple environments:
 * - cqrs stack → cqrs-net (dev environment)
 * - e2e stack → e2e-net (isolated test environment)
 *
 * All containers join this network for internal communication.
 * Only BFF and Keycloak are exposed to the host (plus dev-only UIs).
 */
export const network = new docker.Network("net", {
  name: `${stackName}-net`,
  driver: "bridge",
  // Enable inter-container communication
  internal: false,
  // Attach to default bridge for host access where needed
  attachable: true,
});

// Export network name for container configurations
export const networkName = network.name;
