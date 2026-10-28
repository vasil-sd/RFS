# RFS Workspace Crates (Class A)

The cargo workspace members outside the root `rfs` crate: the build-script
helper, the protocol-adapter layer, and the example crates.

<a id="dev:workspace-crates-overview"></a>
## Overview

TBD (task 12.2): the three workspace members - `crates/rfs-build` (the
`build.rs` helper API that drives `.rfs` compilation from dependent
crates), `crates/rfs-adapters` (the `ProtocolFsm` trait + async/callback/
channel/no_std-minimal adapters), and the example crates under
`crates/examples/` (`async_handler`, `sync_handler`, `fuzz_rust`) - with
their dependency directions on the root crate.
