# RFS Monitor Compiler (Class A)

The language-agnostic PSL-to-monitor compiler: turns property/assumption
formulas into a `MonitorPlan` that every backend renders with identical
semantics.

<a id="dev:monitor-compiler-overview"></a>
## Overview

TBD (task 11.3): the Phase 3 property subset, formula compilation
(`src/monitor.rs::compile_formula`), the `MonitorPlan` node tree
(`src/monitor.rs::MonitorPlan`), and how backends render it - plus the PSL
translation details and the unsupported-formula fallback behavior.
