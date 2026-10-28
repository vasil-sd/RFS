# RFS Validator (Class A)

Semantic validation and expression typing for RFS IR: the pass that turns a
syntactically valid parse into a well-typed, well-formed document.

<a id="dev:validator-overview"></a>
## Overview

TBD (task 8.2): validation architecture - entry point `src/validator.rs::validate_ir`,
the `ValidationError` model, expression typing (`ExprType`, the type environment),
and the individual validation passes; relationship to the parser (what is checked
where) and to codegen (which invariants codegen may rely on).
