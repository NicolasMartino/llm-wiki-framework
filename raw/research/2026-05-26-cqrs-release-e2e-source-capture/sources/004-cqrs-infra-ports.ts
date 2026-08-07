import * as pulumi from "@pulumi/pulumi";

// ---------------------------------------------------------------------------
// Stack profile: defines defaults per environment.
// Add a new entry to `profiles` to support a new stack.
// ---------------------------------------------------------------------------

interface StackProfile {
  ports: {
    postgres: number;
    keycloak: number;
    kafka: number;
    kafkaUi: number;
    mailpitSmtp: number;
    mailpitWeb: number;
    fullstack: number;
    userApi: number;
    workoutApi: number;
    exerciseApi: number;
    elasticsearch: number;
    prometheus: number;
  };
  enableFullstack: boolean;
}

const profiles: Record<string, StackProfile> = {
  dev: {
    ports: {
      postgres: 7060,
      keycloak: 7051,
      kafka: 9092,
      kafkaUi: 7052,
      mailpitSmtp: 7025,
      mailpitWeb: 7057,
      fullstack: 7081,
      userApi: 7061,
      workoutApi: 7062,
      exerciseApi: 7063,
      elasticsearch: 7200,
      prometheus: 7090,
    },
    enableFullstack: false,
  },
  e2e: {
    ports: {
      postgres: 9060,
      keycloak: 9051,
      kafka: 19092,
      kafkaUi: 9052,
      mailpitSmtp: 9025,
      mailpitWeb: 9057,
      fullstack: 9081,
      userApi: 9061,
      workoutApi: 9062,
      exerciseApi: 9063,
      elasticsearch: 9200,
      prometheus: 9090,
    },
    enableFullstack: true,
  },
};

// "cqrs" stack uses the same profile as "dev"
profiles["cqrs"] = profiles["dev"];

// ---------------------------------------------------------------------------
// Resolve: stack profile → config overrides → final values
// ---------------------------------------------------------------------------

const stackName = pulumi.getStack();
const profile = profiles[stackName] ?? profiles["dev"];
const config = new pulumi.Config();

/** Resolved port configuration. Config overrides take precedence over profile defaults. */
export const ports = Object.fromEntries(
  Object.entries(profile.ports).map(([key, defaultValue]) => {
    const configKey = `${key}Port`; // e.g. "postgresPort"
    return [key, config.getNumber(configKey) ?? defaultValue];
  }),
) as StackProfile["ports"];

/** Whether the fullstack container should be deployed. */
export const enableFullstack = config.getBoolean("enableFullstack") ?? profile.enableFullstack;

/** Service container toggles. Keep enabled by default; local dev commands can disable them. */
export const enableUserApi = config.getBoolean("enableUserApi") ?? true;
export const enableWorkoutApi = config.getBoolean("enableWorkoutApi") ?? true;
export const enableExerciseApi = config.getBoolean("enableExerciseApi") ?? true;
