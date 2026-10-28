# RFS IR + JSON Read/Write (Class A)

The IR type system and the hand-rolled JSON stack that serializes it: the
pinned wire-format contract at the heart of the toolchain.

<a id="dev:ir-json-rw-overview"></a>
## Overview

TBD (task 8.3): the `Ir` document model (`src/ir.rs::Ir`, `IR_VERSION`), the
hand-rolled JSON reader/writer (`src/json_rw.rs::Json`,
`src/json_rw.rs::parse`) and why it is dependency-free, the draft-07 schema
emitter + sync walker (`src/json_schema.rs::SchemaDoc`), and the spec hash for
artifact headers (`src/spec_hash.rs::spec_hash16`).
