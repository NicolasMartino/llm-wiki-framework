import * as docker from "@pulumi/docker";
import * as pulumi from "@pulumi/pulumi";
import * as path from "path";
import { network } from "./network";
import { prefixedContainerName } from "./container-names";
import { ports } from "./ports";
import { fullstackContainers } from "./fullstack";

const stackName = pulumi.getStack();
const useIngressProxy = stackName === "e2e";
const nginxConfigPath = path.resolve(__dirname, "../nginx/nginx.multi.conf");

/**
 * E2E-only Nginx ingress with shared /assets cache for parallel browser runs.
 */
export const nginxContainer =
  useIngressProxy && fullstackContainers.length > 0
    ? new docker.Container(
        "nginx",
        {
          name: prefixedContainerName("nginx"),
          image: "nginx:1.27-alpine",

          networksAdvanced: [
            {
              name: network.name,
              aliases: ["nginx"],
            },
          ],

          ports: [
            {
              internal: 80,
              external: ports.fullstack,
            },
          ],

          volumes: [
            {
              hostPath: nginxConfigPath,
              containerPath: "/etc/nginx/nginx.conf",
              readOnly: true,
            },
          ],

          healthcheck: {
            // Use IPv4 loopback explicitly; busybox wget in alpine can prefer ::1
            // while this config only listens on IPv4.
            tests: [
              "CMD",
              "wget",
              "--no-verbose",
              "--tries=1",
              "--spider",
              "http://127.0.0.1:80/nginx-health",
            ],
            interval: "10s",
            timeout: "5s",
            retries: 5,
            startPeriod: "10s",
          },

          restart: "unless-stopped",
        },
        {
          dependsOn: fullstackContainers,
          deleteBeforeReplace: true,
        },
      )
    : undefined;

export const nginxUrl = useIngressProxy
  ? pulumi.interpolate`http://localhost:${ports.fullstack}`
  : undefined;
