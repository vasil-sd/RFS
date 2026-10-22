// Plain-C sync-style ARQ handler (Phase 5 task 12): the C mirror of the
// Rust sync_handler example (crates/examples/sync_handler) against the
// generated spec-43 sliding-window ARQ sender - same scripted peer, same
// expected outcome. The generated FSM source is included directly (single
// translation unit), exactly like the fuzz skeleton does it.
#include "arq_fsm.c"

#include <stdio.h>
#include <stdlib.h>

// Assumption handler: continue on violation (the scenario never violates).
static rfs_policy_t cont_handler(void* user_data, const char* label) {
    (void)user_data;
    (void)label;
    return RFS_CONTINUE;
}

static input_t tick_input(void) {
    input_t in;
    in.kind = INPUT_TICK;
    return in;
}

static input_t ack_input(uint8_t seq, uint32_t checksum) {
    input_t in;
    in.kind = INPUT_ACK_SEQ;
    in.payload.input_ack_seq.seq = seq;
    in.payload.input_ack_seq.checksum = checksum;
    return in;
}

int main(void) {
    fsm_t f;
    fsm_new(&f);

    uint8_t frames[16];
    int n_frames = 0;
    uint8_t acks[16];
    int n_acks = 0;
    int ticks = 0;
    int dropped = 0;
    int max_window = 0;

    // Wake the sender: Idle -> Active (no output).
    step_result_t r = fsm_step(&f, tick_input(), cont_handler, 0);
    ticks++;
    if (r.status != RFS_OK || r.n_outputs != 0) {
        fprintf(stderr, "wake tick failed\n");
        return 1;
    }

    // Run until all 8 frames are sent and acknowledged. Every third tick,
    // inject an out-of-band Ack that the protocol must drop.
    while (n_frames < 8 || n_acks < 8) {
        if (ticks % 3 == 0) {
            r = fsm_step(&f, ack_input(9, 0xDEADBEEFu), cont_handler, 0);
            if (r.status != RFS_OK || r.n_outputs != 0) {
                fprintf(stderr, "out-of-band ack not dropped\n");
                return 1;
            }
            dropped++;
        }
        r = fsm_step(&f, tick_input(), cont_handler, 0);
        ticks++;
        if (r.status != RFS_OK) {
            fprintf(stderr, "tick failed: %d\n", (int)r.status);
            return 1;
        }
        for (int i = 0; i < r.n_outputs; i++) {
            if (r.outputs[i].kind == OUTPUT_SENDFRAME_FRAME) {
                frames[n_frames++] = r.outputs[i].payload.output_sendframe_frame.seq;
            } else if (r.outputs[i].kind == OUTPUT_RECVACK_ACKED) {
                acks[n_acks++] = r.outputs[i].payload.output_recvack_acked.seq;
            }
        }
        int window = n_frames - n_acks;
        if (window > max_window) max_window = window;
    }

    // Completion tick: next_seq == 8 && acked == 8 -> Done (no output).
    r = fsm_step(&f, tick_input(), cont_handler, 0);
    ticks++;
    if (r.status != RFS_OK || r.n_outputs != 0) {
        fprintf(stderr, "completion tick failed\n");
        return 1;
    }

    // Quiescence: in Done, further ticks produce no output.
    r = fsm_step(&f, tick_input(), cont_handler, 0);
    ticks++;
    if (r.status != RFS_OK || r.n_outputs != 0) {
        fprintf(stderr, "quiescence tick failed\n");
        return 1;
    }

    // Verify the observable protocol invariants (same as the Rust example).
    for (int i = 0; i < 8; i++) {
        if (frames[i] != (uint8_t)i || acks[i] != (uint8_t)i) {
            fprintf(stderr, "sequence mismatch\n");
            return 1;
        }
    }
    if (max_window > 4) {
        fprintf(stderr, "window bound violated: %d\n", max_window);
        return 1;
    }
    if (dropped == 0) {
        fprintf(stderr, "expected dropped out-of-band acks\n");
        return 1;
    }

    // The data lines below are byte-identical to the Rust sync_handler's
    // (the header line is language-specific and not compared).
    printf("c_sync_handler: ARQ session complete (generated C FSM, plain gcc)\n");
    printf("  ticks issued:             %d\n", ticks);
    printf("  frames sent (in order):   [");
    for (int i = 0; i < n_frames; i++) printf("%s%d", i ? ", " : "", frames[i]);
    printf("]\n");
    printf("  acks received (in order): [");
    for (int i = 0; i < n_acks; i++) printf("%s%d", i ? ", " : "", acks[i]);
    printf("]\n");
    printf("  out-of-band acks dropped: %d\n", dropped);
    printf("  max window observed:      %d (bound 4)\n", max_window);
    return 0;
}
