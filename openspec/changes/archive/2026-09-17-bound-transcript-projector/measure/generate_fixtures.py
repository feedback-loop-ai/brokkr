#!/usr/bin/env python3
"""Generate the #277 DSH measurement fixtures outside the frozen corpus.

Deterministic: every fixture is a pure function of the constants below, so the
same generator reproduces the same bytes and digests on any host. Members are
one character at one-millisecond gaps with consecutive nonoverlapping
sequences, valid positions and no assemblies or dedicated ids: the ordinary
streaming shape, not an attack.
"""

import json
import os

OUT = os.environ.get("MEASURE_OUT", "measure")
ROWS_16 = 16
MEMBERS = 16_384
TOTAL = ROWS_16 * MEMBERS  # 262,144


def line(value):
    return json.dumps(value, separators=(",", ":")) + "\n"


def write(name, text):
    with open(os.path.join(OUT, name), "w", encoding="utf-8") as f:
        f.write(text)


def packed(seq0, time0, members, args=False):
    data = {
        "turn": 1,
        "step": 1,
        "index": 0,
        "dt": [1] * (members - 1),
        "texts" if not args else "args": ["x" if args else "a"] * members,
    }
    if args:
        data["id"] = "c1"
    kind = "tool-call-chunks" if args else "text-chunks"
    return line({"type": kind, "seq0": seq0, "time0": time0, "data": data})


def main():
    os.makedirs(OUT, exist_ok=True)
    header = line({"type": "session", "version": 0})

    # case A: sixteen packed text rows of 16,384 one-character members.
    text = header
    for row in range(ROWS_16):
        text += packed(1 + row * MEMBERS, 1000 + row * 1000, MEMBERS)
    write("case_a_16x16384.jsonl", text)

    # case B: the same 262,144 members in one packed row.
    write("case_b_1x262144.jsonl", header + packed(1, 1000, TOTAL))

    # case tiny-calls: 10,000 absent-data ordinary tool calls.
    write(
        "case_tiny_calls.jsonl",
        header
        + "".join(
            line({"type": "tool/call", "seq": seq, "time": 1000, "data": {}})
            for seq in range(1, 10_001)
        ),
    )

    # case blockless: 10,000 absent-data blockless rows between two visible
    # text-delta chunks.
    text = header
    text += line(
        {
            "type": "assistant/chunk",
            "seq": 1,
            "time": 1,
            "data": {"turn": 1, "step": 1, "chunk": {"type": "text-delta", "text": "start"}},
        }
    )
    for seq in range(2, 10_002):
        kind = "user/message" if seq % 2 == 0 else "tool/result"
        text += line({"type": kind, "seq": seq, "data": {}})
    text += line(
        {
            "type": "assistant/chunk",
            "seq": 10_002,
            "time": 2,
            "data": {"turn": 1, "step": 1, "chunk": {"type": "text-delta", "text": "end"}},
        }
    )
    write("case_blockless.jsonl", text)

    # case quiet-packed: sixteen packed tool-argument rows (recognized quiet).
    text = header
    for row in range(ROWS_16):
        text += packed(1 + row * MEMBERS, 1000 + row * 1000, MEMBERS, args=True)
    write("case_quiet_packed.jsonl", text)

    # case block-dense: one message turn carrying 50,000 blocks.
    write(
        "case_block_dense.jsonl",
        header
        + line(
            {
                "type": "assistant/message",
                "seq": 1,
                "time": 1000,
                "data": {
                    "turn": 1,
                    "step": 1,
                    "message": {
                        "content": [{"type": "text", "text": "b"}] * 50_000
                    },
                },
            }
        ),
    )

    # Claude and Codex tiny-row residual probes: 10,000 one-character turns.
    write(
        "case_claude_tiny.jsonl",
        "".join(
            line({"type": "assistant", "message": {"role": "assistant", "content": "a"}})
            for _ in range(10_000)
        ),
    )
    write(
        "case_codex_tiny.jsonl",
        "".join(
            line(
                {
                    "timestamp": "t",
                    "type": "response_item",
                    "payload": {
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": "a"}],
                    },
                }
            )
            for _ in range(10_000)
        ),
    )


if __name__ == "__main__":
    main()
