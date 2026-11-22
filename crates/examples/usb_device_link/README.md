# usb_device_link - a USB device and link state machine

## What it demonstrates
- A USB device & link state machine: Default -> Addressed -> Configured, suspend/resume with the prior configuration restored, bus reset (through link retraining when suspended), and data flow only in the configured state. RFS features: event parameters with address/config payloads, an enum-typed resume-memory variable, and two safety properties.
- The automaton is drawn from the real USB device state model (USB 2.0/3.x device states + link power management); it is distinct from the existing `usb_control_transfer` teaching spec, which models one transfer rather than the device/link lifecycle.

## The machine
```mermaid
stateDiagram-v2
    [*] --> Default
    Default --> Addressed : SetAddr(a != 0)
    Addressed --> Configured : Configure(c)
    Addressed --> Suspended : SuspendReq
    Configured --> Suspended : SuspendReq
    Suspended --> Configured : ResumeReq (was Configured)
    Suspended --> Addressed : ResumeReq (was Addressed)
    Configured --> Configured : DataIn\nemit SendData
    Default --> Default : ResetBus\nemit SendResetAck
    Suspended --> Recovery : ResetBus\nemit SendResetAck
    Recovery --> Default : LinkUp
```
(ResetBus from the other states returns directly to Default.)

## RFS modes demonstrated
### Specification & analysis (`rfs validate`, `rfs info`)
The spec parses and validates; `rfs info` reports the 5 states, 7 events, 2 outputs,
the three variables (incl. the enum-typed `suspended_from`), and the two monitored
properties. Rationale: the analysis gate that runs before any codegen/MC.

### Model checking - in-process (`rfs mc-ref`, exact)
`pass.rfs` model-checks to **PASS complete** (4320 states): `no_data_while_suspended`
holds - data tokens flow only in the configured state - and `default_has_no_address`
holds: a device in Default holds no address. The negative variant `fail.rfs` removes
the state guard, so a data token flows while suspended - a pinned **property-violation**
CE on `no_data_while_suspended` at step 3 (DataIn in Suspended emits SendData);
`fixed.rfs` restores the guard and re-runs to PASS.

### Model checking - generated binaries (rust + c)
Both backends' MC binaries are generated, compiled, and run in exact mode; their reports
are **byte-identical** to each other and to mc-ref. The failing variant's CE is identical
across all three engines.

### Production code generation (rust + c)
`rfs codegen rust production` and `rfs codegen c production` both compile the FSM
warning-free - the generated-C compiler-agnosticism bar. Rationale: a real device/link
machine compiles to shippable code in both backends.

## Files layout
| File | Role |
|---|---|
| `pass.rfs` | the valid device & link machine |
| `fail.rfs` | state guard removed (data-while-suspended defect) |
| `fixed.rfs` | the corrected spec (= pass.rfs) |
| `run.sh` | drives every demonstrated RFS mode |
| `captures/` | pinned outputs (text + JSON CEs) |

## Build / run / test
Prerequisite: the rfs toolchain built (`cargo build`) or `RFS_BIN` set; rustc + gcc available.
```sh
./run.sh
```
`cargo test --test example_projects_test usb_device_link` re-runs it and pins the excerpts.

## Expected results
- pass.rfs: PASS complete (4320 states); both properties hold.
- fail.rfs: FAIL, `reason=property-violation label=no_data_while_suspended step=3` - SendData emitted in Suspended.
- fixed.rfs: PASS complete.
