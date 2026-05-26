// Import infrastructure modules
import { network } from "./src/network";
import {
  postgresContainer,
  appDbUrl,
  usersDbUrl,
  workoutsDbUrl,
  exercisesDbUrl,
} from "./src/postgres";
import { kafkaContainer, kafkaBootstrapServers } from "./src/kafka";
import { keycloakContainer, keycloakUrl, keycloakIssuer, keycloakRealm } from "./src/keycloak";
import { keycloakConfigContainer, keycloakConfigured } from "./src/keycloak-config";
import { kafkaUiContainer, kafkaUiUrl } from "./src/kafka-ui";
import { mailpitContainer, mailpitUrl } from "./src/mailpit";
import { prometheusContainer, prometheusUrl, prometheusApiUrl } from "./src/prometheus";
import { userApiContainer, userApiUrl } from "./src/user-api";
import { workoutApiContainer, workoutApiUrl } from "./src/workout-api";
import { exerciseApiContainer, exerciseApiUrl } from "./src/exercise-api";
import { elasticsearchContainer, elasticsearchUrl } from "./src/elasticsearch";
import { fullstackContainer, fullstackContainerNames, fullstackUrl } from "./src/fullstack";
import { nginxContainer, nginxUrl } from "./src/nginx";
import { topicProvisioning, topicsCreated } from "./src/kafka-topics";

// Export network for use by other modules
export const networkName = network.name;

// Phase 1: Infrastructure Foundation
export { postgresContainer } from "./src/postgres";
export { kafkaContainer } from "./src/kafka";
export { elasticsearchContainer } from "./src/elasticsearch";

// Phase 10: Topic Provisioning (runs after Kafka is healthy)
export { topicProvisioning };

// Phase 2: Identity & Auth
export { keycloakContainer } from "./src/keycloak";
export { keycloakConfigContainer } from "./src/keycloak-config";

// Phase 3: Developer UX
export { kafkaUiContainer } from "./src/kafka-ui";
export { mailpitContainer } from "./src/mailpit";
export { prometheusContainer } from "./src/prometheus";

// Services
export { userApiContainer } from "./src/user-api";
export { workoutApiContainer } from "./src/workout-api";
export { exerciseApiContainer } from "./src/exercise-api";
export { fullstackContainer } from "./src/fullstack";
export { nginxContainer } from "./src/nginx";

// Stack outputs
export const stackOutputs = {
  network: networkName,
  postgres: postgresContainer.name,
  kafka: kafkaContainer.name,
  keycloak: keycloakContainer.name,
  keycloakConfig: keycloakConfigContainer.name,
  kafkaUi: kafkaUiContainer.name,
  mailpit: mailpitContainer.name,
  prometheus: prometheusContainer.name,
  userApi: userApiContainer?.name,
  workoutApi: workoutApiContainer?.name,
  exerciseApi: exerciseApiContainer?.name,
  elasticsearch: elasticsearchContainer.name,
  // Fullstack is optional (may be undefined if not enabled)
  fullstack: fullstackContainer?.name,
  fullstackContainers: fullstackContainerNames,
  // Nginx is e2e-only (undefined in non-e2e stacks)
  nginx: nginxContainer?.name,
  // URLs
  kafkaBootstrapServers,
  keycloakUrl,
  keycloakIssuer,
  keycloakRealm,
  keycloakConfigured,
  kafkaUiUrl,
  mailpitUrl,
  prometheusUrl,
  prometheusApiUrl,
  userApiUrl,
  workoutApiUrl,
  exerciseApiUrl,
  elasticsearchUrl,
  fullstackUrl,
  nginxUrl,
  databases: {
    app: appDbUrl,
    users: usersDbUrl,
    workouts: workoutsDbUrl,
    exercises: exercisesDbUrl,
  },
  topicsCreated,
};
