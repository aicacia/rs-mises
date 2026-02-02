# Summary

Manage Identities (**Group**, **Device**, **User**, **Persona**, **Service**, **Application**) and Objects (**Key**, **Request**, **Policy**, **Resource**) and their relationships to provide scoped access to resources. All state is stored in the graph.

# Identities

- Types: **Group**, **Device**, **User**, **Persona**, **Service**, **Application**.
- **Group** - a collection or namespace of Identities. The initial _master Group_ is the root of all Identities, relationships, and Objects; its key is the master key.
- **Device** - Devices are either replicated (have a `root` field referencing an Identity ID) or non-replicated devices (no `root`) modeling devices such as IoT endpoints.
  - Replicated devices hold either a **full graph** (rooted at the master Group) or a **subset graph** rooted at a specific Identity (e.g., a User). Subset graphs include only nodes and edges reachable from their `root`; syncing is limited to that subset.
- **Persona** - an aliased indirection for a **User**.
- **Service** - a reusable unit of functionality that performs some task(s).
- **Application** - Represents a deployed client or runtime instance. Applications are the authenticating entities (tokens map to them) and are linked to their owning `Service` via graph relationship `OWNS`. Use instances for client credentials, runtime authorization, and delegation to the parent `Service`.
- Attributes: **Internal** / **External** — these are orthogonal attributes that apply to any Identity type.
  - **Internal** — locally owned identities with an active key (represented by a `HAS_KEY` edge to a `Key` node whose private key is derivable).
  - **External** — identities whose public keys are not derived from the master key; they must be explicitly authorized before joining the graph or forming relationships.

# Authentication

- Use OpenID Connect (OIDC) JWTs for authentication and authorization. Tokens MUST be validated (signature, expiry, issuer, audience) before mapping to graph Identities.
- Required JWT claims: `iss`, `aud`, `sub`, `exp`, `jti`, `scope` and a delegation claim `acting_for` when acting for another Identity.

  JWT Identity mapping (priority):
  1. `acting_for` → delegated Identity (when issued token on behalf of another identity)
  2. `sub` → subject Identity

- **Groups cannot authenticate directly.** A **Device**, **User**, or **Application** with ownership or permission may act on a Group's behalf only when delegation is explicit in the token (for example, an `acting_for` claim).
- **Device** authentication MUST be performed on-device (not only over the network); acceptable means include hardware-backed keys, TPM/secure enclave attestation, or platform attestation APIs. High-assurance operations (e.g., master key import/export, provisioning devices into the master Group) MUST require device attestation or equivalent proof-of-possession.
- **User** authentication supports Username/Password for web flows; multi-factor authentication is recommended for elevated actions.
- **Personas** do not authenticate; a **User** authenticates on their behalf.
- **Applications** authenticate using OIDC client credentials (client_id/client_secret, JWTs, etc.). An `acting_for` claim may express actions on behalf of another Identity.

# Keys

- Keys are derived using BIP32 hierarchical deterministic derivation from a master key.
- The master key MUST be encrypted at rest (passphrase or hardware keystore) and may be generated or imported.
- Derived keys support export, backup, versioning, and rotation.
  - **Identity keys** Non-hardened child derivation to enable public-key-based rotation and delegation. Hardened derivation for keys where public-child derivation must be prevented (e.g., certain root keys).

# Resources

- A Resource is owned by an Identity and identified by a type (primary use case: `file-system`). Resources are accessible only via the Gateway API.
- Resources are modular and extensible via plugins that implement the resource interface and register types and permissions.
- Each Resource type defines a fixed set of allowed permissions (e.g., `file-system` supports `readonly` and `readwrite`).
- Resources are responsible only for their internal logic; global authorization and policy decisions are performed by the central system.

# Policies

- **Policies** define notifications and constraints (re-approval, quorum, retention/erasure). Policies are applied through `MEMBER_OF`. Group policies apply to nested members unless explicitly overridden. Cycles are allowed.
- Policy evaluation is deterministic. Precedence (highest → lowest):
  1. Any deny (explicit or inherited)
  2. Explicit allow attached directly to the target
  3. Inherited allow
- Deny always wins: a deny rule (whether explicit or inherited) overrides any allow.
- If multiple rules of the same precedence apply, prefer the most-specific rule, then the nearest node in the membership path.

# Requests

- **Requests** are how Identities ask the system to change state. Any Identity (or one acting on behalf of another Identity) may initiate a request. Every request MUST use a validated JWT that maps to a graph Identity directly or with delegation.

- Request types:
  - **Relationship** - create or modify edges (e.g., `MEMBER_OF`, `OWNS`). Requires authorization and approvers determined by owners and applicable policies.
  - **Resource** - create resources or grant access. Requests separate **ownership** (who OWNS) from **scope** (how the Resource is namespaced). They provide a descriptive use hint.

- Ownership (`ownership`) can be `identity` (implicit approver decides), explicitly send a list of identity ID(s), or the initiating Identity `requestor`.

- Scope (`scope`): `owner`, `requestor`, or `owner+requestor`.

- `create_if_missing` (default: `true`) controls creation when no matching Resource exists.

- On approval: resolve or create the Resource for the requested scope, create/update `OWNS` edges.

## Relationship/Edge Types (subject → object)

- `MEMBER_OF` - Identity (User | Device | Service | Group) → Group - membership; transitive (cycles allowed).
- `REVOKED_BY` - Key → Identity - when key is manually revoked by a user.
- `HAS_KEY` - Identity → Key - identity key possession (active or historical).
- `OWNS` - Identity → Object or Identity - ownership; owners are primary approvers unless overridden.
