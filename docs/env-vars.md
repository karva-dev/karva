<!-- WARNING: This file is auto-generated (cargo run -p karva_dev generate-all). Update the doc comments on the env-var structs in 'crates/karva_static/src/lib.rs' if you want to change anything here. -->

# Environment Variables

This page lists every environment variable that Karva reads from the environment, plus the variables the worker exposes to running tests.

## Read by Karva

Variables Karva reads from the environment to influence its own behavior.

### `RAYON_NUM_THREADS`

This is a standard Rayon environment variable.

### `KARVA_MAX_PARALLELISM`

This is a standard Karva environment variable.

### `KARVA_CONFIG_FILE`

This is a standard Karva environment variable.

### `KARVA_SNAPSHOT_UPDATE`

When set to "1" or "true", snapshot assertions write directly to `.snap`
instead of creating `.snap.new` pending files.

## Set by the worker on tests

Variables the Karva worker writes into the test process before each attempt, so running test code can introspect its own retry state.

### `KARVA_ATTEMPT`

The 1-indexed attempt number for the currently running test. Always
set; `"1"` when no retries are configured.

### `KARVA_TOTAL_ATTEMPTS`

The total number of attempts allowed for the currently running test
(`retries + 1`). Always set.

