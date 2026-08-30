# Threat model

> **Stub.** Filled by issue #35.

Planned sections:

## What Egeria defends

Structural properties of workflows: that privileged effects are gated, that
untrusted data does not reach a privileged sink unvalidated, that capabilities are
scoped, that retries terminate.

## What Egeria does not defend

Agent behavior. A workflow can be structurally perfect while the agent inside a
node is manipulated by injected content. Formal graph correctness is necessary and
nowhere near sufficient — adaptive prompt-injection research is clear that static
prompt defenses are brittle, which is why trust boundaries belong in the IR and
enforcement belongs in the runtime.

Also outside the boundary: correctness of generated code, correctness of a human
approval, and the behavior of external services.

## Trust and classification model

Trust classes (trusted configuration, trusted user input, untrusted external
content, model-generated, validated) and data classifications (public, internal,
confidential, PII, secret), and the structural rules expressed over them.

## Untrusted inputs to Egeria itself

Workflow documents, SOP sources, generated Alloy models, and — in a future
registry — community submissions are all untrusted artifacts. Parsers must not
panic on hostile input; solver inputs and outputs are data, not instructions; and
registry evaluation must run in strong isolation with pinned dependencies and no
privileged credentials.

## Fail-closed compilation

Why a backend that cannot preserve a security property must reject rather than
emulate.
