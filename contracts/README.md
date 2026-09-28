# GIW desktop desired-state contract

The public desired-state data contract has two independently authored peer authorities:

1. `contracts/typespec/main.tsp`
2. `schema/giw-desktop.schema.json` (JSON Schema Draft 2020-12)

Neither is generated from the other. `@oresoftware/typespec-json-schema-validator` generates a temporary TypeSpec witness only as comparison evidence and executes both authorities over the same deterministic instance/probe corpus.

`tools/validator` separately enforces runtime security semantics that are intentionally **not** a third shape authority, including:

- duplicate service-name rejection;
- literal `env` key naming policy;
- secret-bearing literal environment-name rejection (secrets must use runtime passthrough/encrypted boundaries);
- TCP port admission beyond the JSON regular-expression shape.

This separation is deliberate. Data-shape facts must converge in TypeSpec and JSON Schema. Host/security policy that cannot be represented symmetrically in both languages remains executable Rust admission logic and must not be smuggled into only one schema authority.

Recorded parity fixtures live under `contracts/instances/<Declaration>/{valid,invalid}`. Generated TJSV schema witnesses and receipts belong under ignored `artifacts/` / `.typespec-json-schema-validator/` paths and are evidence, never authority.
