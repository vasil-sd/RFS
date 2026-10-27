//! Pinned FAIL-variant specs shared by the MC suites (Phase 7 task 7):
//! modified copies of the benchmark specs with a known violation. They are
//! test inputs, not part of the valid-example set (no new example files).
//! `benchmark_mc_test` (the owner) and `info_format_test` (ce-trace walk)
//! consume them from here so each variant's text has exactly ONE home -
//! duplicating the spec text across suites is the drift class this project
//! pins against.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn spec_source(name: &str) -> String {
    let p = root().join("examples/valid").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {:?}: {e}", p))
}

/// Read a valid example and replace exactly one anchored block with `new`.
pub fn mutate_spec(name: &str, old: &str, new: &str) -> String {
    let src = spec_source(name);
    assert_eq!(
        src.matches(old).count(),
        1,
        "anchor for the {name} FAIL variant drifted; re-pin it"
    );
    src.replacen(old, new, 1)
}

// ---------------------------------------------------------------------------
// Phase 4 FAIL variants (full inline copies of the benchmark specs).
// ---------------------------------------------------------------------------

/// `44_token_ring` with the Tick GRANT arm removed: a granted node's request
/// flag is never cleared, so the safety invariant `holder_no_request` is
/// violated at step 1. Safety takes precedence over liveness here - the
/// `req_granted_*` properties also fail on this path but are reported after.
pub const RING_NO_GRANT_SRC: &str = r#"
# FAIL VARIANT (benchmark_mc_test, task 9): the Tick grant arm is removed,
# so a granted node's request flag is never cleared -> holder_no_request violated.
#
# A single token circulates the ring; on every Tick it passes to the next
# node in ring order (bit<2> addition wraps 3 -> 0). A node with a pending
# request is granted when the token arrives at it: its flag clears and a
# Grant is emitted. Nodes may request only while they do not hold the token.
#
# Liveness "every request is eventually granted": nodes 1-3 request at
# startup (req init 14); req_granted_* check that each startup request is
# cleared, and flags are cleared ONLY on grant (safety invariant
# holder_no_request). The bounded property checks EVERY instance of node 1's
# requests (the environment may re-request at any time): the token needs at
# most 3 hops and env_tick bounds each hop to 5 samples.
#
# Abstraction stress: Request carries a bit<16> payload the protocol never
# inspects - input abstraction (D-P4-3) collapses it to one representative,
# so the state space is independent of its width. The token's carried frame
# ID (data: bit<8>) is a CONCRETE wide field: updated on every pass but never
# inspected, it multiplies the state space by 256 and shows the contrast.
module token_ring {
  enum State { Running }
  initial Running

  event Tick
  event Request(R(node: bit<2>, payload: bit<16>))

  output Grant(G(node: bit<2>, frame: bit<8>))
  output Pass(P(from: bit<2>, to: bit<2>))

  var holder: bit<2> init 0   # current token holder (ring position)
  var req: bit<8>    init 14  # pending request flags, one per node; only bits 0-3 used
  var data: bit<8>   init 0   # frame ID carried by the token, +1 each pass (concrete wide field)

  assume env_tick : always (bounded_eventually[5] (input_is(Tick))) ;

  # Safety: the current holder never has a pending request - flags are only
  # cleared on grant, at the granting step.
  prop holder_no_request : always (req & (1 << holder) == 0) ;

  # Liveness: every startup request is eventually granted (nodes 1-3).
  prop req_granted_1 : eventually (req & 2 == 0) ;
  prop req_granted_2 : eventually (req & 4 == 0) ;
  prop req_granted_3 : eventually (req & 8 == 0) ;

  # Liveness (bounded, every instance): node 1's request is granted within
  # 25 samples of being set.
  prop req_granted_1_bounded : always (req & 2 != 0 -> bounded_eventually[25] (req & 2 == 0)) ;

  # A node requests the token (only while it does not hold it). The payload
  # is never inspected - input abstraction collapses it to one value.
  transition Running on Request(R(node, payload)) where node != holder {
    next Running;
    set req = req | (1 << node);
  }

  # Token passes to the next node in ring order. If that node has a pending
  # request it is granted: flag clears and a Grant is emitted.
  transition Running on Tick {
    priority 1;
    next Running;
    emit Pass(P(holder, holder + 1));
    set holder = holder + 1;
    set data = data + 1;
  }
}
"#;

/// `45_elevator` with the Idle go-up arm removed: an up-call at a floor above
/// the current one can never be served, so the liveness property `up2_served`
/// is violated (lasso, cycle length 3 - the elevator idles at floor 0 forever
/// while the startup up-call at floor 2 stays pending). The safety invariant
/// `idle_no_local_call` still holds on the path.
pub const ELEV_NO_UP_SRC: &str = r#"
# FAIL VARIANT (benchmark_mc_test, task 9): the Idle go-up arm is removed,
# so an up-call above can never be served -> up2_served liveness violated.
#
# The elevator moves one floor per Tick. While moving it stops to serve any
# pending call at the current floor (both directions, one stop); it reverses
# direction only at the ends and waits (Idle) when no call is anywhere. A
# press at the floor the elevator is currently at is served immediately (one
# stop clears both call vectors there), so a press never leaves a pending
# local bit behind; calls pressed elsewhere are served on the next visit.
# Two calls wait at startup: an up-call at floor 2 and a down-call at floor 3.
#
# Liveness "every call is eventually served": up2_served / down3_served check
# the two startup calls (non-vacuous: both are pending at init). Per-instance
# bounded liveness (always (pending -> bounded_eventually[N] cleared)) lives in
# spec 44 instead: each such obligation adds ~N countdown states to every
# config where the bit is set, which blew this spec past the Phase 4 sizing
# target (1M+ states with two bound-80 obligations).
#
# Note: only ONE environment assumption (env_tick). Two independent
# bounded_eventually assumptions over disjoint event sets admit prefixes where
# both countdowns reach their deadline on the same sample - no single input can
# satisfy both, so every successor is pruned and exact MC reports a spurious
# deadlock. Kept out of this benchmark spec on purpose.
#
# Abstraction stress: CallUp/CallDown carry a bit<16> user_id the protocol
# never inspects - input abstraction (D-P4-3) collapses it to one
# representative, so the state space is independent of its width. The call
# vectors are CONCRETE wide fields (bit<8>, only bits 0-3 used): 16x16
# combinations multiply the state space.
module elevator {
  enum State { Idle, Up, Down }
  initial Idle

  event Tick
  event CallUp(F(floor: bit<2>, user_id: bit<16>))
  event CallDown(F(floor: bit<2>, user_id: bit<16>))

  output Serve(S(floor: bit<2>))

  var pos: bit<2>      init 0   # current floor (0 = bottom, 3 = top)
  var up_calls: bit<8> init 4   # pending up-calls, one bit per floor; only bits 0-3 used
  var down_calls: bit<8> init 8 # pending down-calls, one bit per floor; only bits 0-3 used

  assume env_tick : always (bounded_eventually[5] (input_is(Tick))) ;

  # Safety: the elevator never idles while a passenger waits at its floor
  # (holds at every sample: presses at the current floor are served in the
  # same step, and stops clear both call vectors there).
  prop idle_no_local_call : always (state == Idle -> up_calls & (1 << pos) == 0 && down_calls & (1 << pos) == 0) ;

  # Liveness: the two startup calls are eventually served.
  prop up2_served : eventually (up_calls & 4 == 0) ;
  prop down3_served : eventually (down_calls & 8 == 0) ;

  # Passengers press calls (the user_id is never inspected - input
  # abstraction collapses it to one value). A press at the floor the elevator
  # is currently at is served immediately: one stop clears BOTH call vectors
  # at that floor (and the pressing passenger boards) WITHOUT losing the
  # current state - staying in Up/Down is what makes up2_served / down3_served
  # hold against an environment that spams presses at the current floor
  # (any reset to Idle would let a press interrupt every ascent attempt). A
  # press elsewhere sets the pending bit; it is served on the next visit.
  transition Idle on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Idle on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Idle;
    set up_calls = up_calls | (1 << floor);
  }
  transition Up on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Up;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Up;
    set up_calls = up_calls | (1 << floor);
  }
  transition Down on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Down;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Down;
    set up_calls = up_calls | (1 << floor);
  }
  transition Idle on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Idle on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Idle;
    set down_calls = down_calls | (1 << floor);
  }
  transition Up on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Up;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Up;
    set down_calls = down_calls | (1 << floor);
  }
  transition Down on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Down;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Down;
    set down_calls = down_calls | (1 << floor);
  }

  # Moving up: stop to serve a call at the current floor (arrived with it
  # pending); otherwise keep going up. After serving the last demand the
  # elevator finishes its ascent to the top before reversing or waiting - a
  # documented simplification; at the top, reverse if a call waits below.
  transition Up on Tick where ((up_calls | down_calls) & (1 << pos)) != 0 {
    priority 3;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on Tick where pos < 3 {
    priority 2;
    next Up;
    set pos = pos + 1;
  }
  transition Up on Tick where pos == 3 && ((up_calls | down_calls) & ((1 << pos) - 1)) != 0 {
    priority 1;
    next Down;
  }
  transition Up on Tick {
    next Idle;
  }

  # Moving down: mirror image of the Up arms.
  transition Down on Tick where ((up_calls | down_calls) & (1 << pos)) != 0 {
    priority 3;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on Tick where pos > 0 {
    priority 2;
    next Down;
    set pos = pos - 1;
  }
  transition Down on Tick where pos == 0 && ((up_calls | down_calls) & ~((1 << (pos + 1)) - 1)) != 0 {
    priority 1;
    next Up;
  }
  transition Down on Tick {
    next Idle;
  }

  # Idle: head to the nearest demand (up first), or wait when no call is
  # anywhere. No serve arm here: state Idle never has a pending call at the
  # current floor (idle_no_local_call).
  # Call at some floor strictly above pos: floors >= pos minus the current
  # floor (built from `1 << pos` - `pos + 1` would wrap at the top floor).
  transition Idle on Tick where ((up_calls | down_calls) & ((1 << pos) - 1)) != 0 {
    priority 1;
    next Down;
  }
  transition Idle on Tick {
    next Idle;
  }
}
"#;

// ---------------------------------------------------------------------------
// Phase 6 task 6 FAIL variants. Unlike the Phase 4 variants above
// (full inline copies), these are derived from the REAL spec source at test
// time: the delta from the pinned example is exactly one line, so a variant
// cannot drift from the spec it documents. `mutate_spec` fails loudly if an
// anchor ever changes.
// ---------------------------------------------------------------------------

/// `47_http2_stream` with the Open RstStream arm retargeted off Closed: an
/// abort no longer lands in Closed, so `rst_goes_closed` is violated at step 2
/// (SendData opens the stream; RstStream leaves it). Safety takes precedence -
/// the bounded liveness property is never analyzed.
pub fn http2_rst_fail_src() -> String {
    mutate_spec(
        "47_http2_stream.rfs",
        "  transition Open on RstStream {\n    next Closed;\n    emit StreamReset;\n  }",
        "  transition Open on RstStream {\n    next HalfClosedLocal;\n    emit StreamReset;\n  }",
    )
}

/// `49_can_arbitration` with the Data AckErr band-entry arm's counter update
/// weakened (+1 instead of +4): after one full frame the first passive-band
/// entry lands at error_counter = 5, below the band floor 8, so
/// `passive_band` is violated at step 6. (A guard OVERLAP would not work: RFS
/// priorities fire DESCENDING - SEMANTICS 3.4 - so the priority-3 arm keeps
/// winning for counters 0..3 and no sub-band ErrorPassive entry is produced.)
pub fn can_passive_fail_src() -> String {
    mutate_spec(
        "49_can_arbitration.rfs",
        "  transition Data on AckErr where error_counter + 4 >= 8 && error_counter + 4 < 16 {\n    priority 2;\n    next ErrorPassive;\n    set error_counter = error_counter + 4;\n  }",
        "  transition Data on AckErr where error_counter + 4 >= 8 && error_counter + 4 < 16 {\n    priority 2;\n    next ErrorPassive;\n    set error_counter = error_counter + 1;\n  }",
    )
}
