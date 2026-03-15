# Examples

These are concise, concrete scenarios that demonstrate common flows. For rules, constraints, and definitions see `docs/design.md`.

### Request lifecycle (relationship)

**Scenario:** `user:A` requests to add `device:B` to `group:root`.

Steps:

1. Request: `user:A` submits a relationship request `{ relationship_type: MEMBER_OF, subject: device:B, object: group:root }` with `scope=owner` and `ownership=identity`.
   - The request is authenticated with a validated JWT mapped to a graph Identity (priority: `acting_for` → `sub`).
   - `scope=owner` means only owners of `group:root` may approve the request.
   - `ownership=identity` indicates the request is tied to the subject identity (`device:B`); approvers are still resolved via owners/policies rather than simply being the requester.
2. Approval: eligible approvers are resolved from current owners of `group:root` and request approvals are collected until quorum is met.
3. Apply: the approved request creates the `MEMBER_OF` edge and the request transitions to `Applied`.

### Approval denial (no quorum)

**Scenario:** `user:A` requests to add `device:B` to `group:root`, but not enough approvers respond.

Steps:

1.  Request: `user:A` submits the same relationship request as above.
2.  Approval: eligible approvers are resolved, but fewer than the required quorum of approvals are received (e.g., approvers ignore, reject, or don’t respond).
3.  Deny: the request transitions to `Denied` (or `Rejected` depending on policy), and no graph edge is created.

### Application file-system resource

**Scenario:** `app:A` requests a `file-system` with `scope=owner+requestor`.

Steps:

1.  Request: `app:A` submits a request carrying a validated JWT (signature, `iss`, `aud`, `sub`, `exp`, `jti`, `scope`) and payload `{ type: file-system, scope: owner+requestor, actions: [...] }`. The JWT MUST be validated and map to a graph Identity (priority: `acting_for` → `sub`).
2.  Approval: if policy requires approval the request becomes `PENDING_APPROVAL`. Approvers are determined by object owners and applicable policies (owners and `APPROVER_FOR`); the approver UI should default to the approver's active Identity and may select which Identity will `OWNS` the resource.

- If policy allows auto-issuance for the requested scope, the request can transition directly toward issuance without human approval.

3.  Issuance: on approval the chosen Identity gets an `OWNS` edge for the resource and a grant-scoped key is allocated for access.
4.  Grant/key creation: issuance atomically creates a grant-scoped `Key` node (with `kid`), adds `HAS_KEY`, and creates approval edges marking issuance.
5.  Activation: activation moves the grant to `ACTIVE` and issues access tokens signed by the grant key (tokens reference `kid`); token validation must include `kid`, `jti`, `exp`, `iss`, and `aud`.
6.  Revocation/expiry: revocation or expiry transitions the grant to `REVOKED`/`EXPIRED`, removes or blacklists keys as appropriate, invalidates tokens, and emits auditable changefeed events.
