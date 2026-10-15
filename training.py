#!/usr/bin/env python3
"""RFS SFT + GRPO training for Qwen3.8-27B (Unsloth).

Trains Qwen3.8-27B on the RFS training set (`all_rfs_training.jsonl`) in two
stages, following the official Unsloth Qwen3.8 recipes
(https://unsloth.ai/docs/models/qwen3.8/train and the Kaggle-Qwen3.8_(27B)
Conversational / GRPO notebooks):

  1. SFT  -- supervised fine-tuning (QLoRA). Each JSONL row
             {"messages": [{"role":"user",...}, {"role":"assistant",...}]}
             is converted into a `text` column rendered with the Qwen chat
             template; loss is masked to assistant responses only via
             unsloth.chat_templates.train_on_responses_only.
  2. GRPO -- group-relative policy optimization on top of the SFT LoRA
             adapter, with pure-Python rule-based rewards:
               * think_channel        : < think> ... < /think> channel structure
               * rfs_structure        : structural validity of ```rfs snippets
               * reference_similarity : word-bigram overlap with the gold answer

Dataset conversion (applied automatically by this script):
  SFT rows  : messages -> text   (tokenizer.apply_chat_template, tokenize=False)
  GRPO rows : messages -> prompt (user turn only) + reference (gold assistant);
              rows whose rendered prompt exceeds --grpo-max-prompt-tokens are
              dropped so a completion budget remains inside max_seq_length.

Usage:
  python training.py --stage both                  # SFT then GRPO (default)
  python training.py --stage sft --sft-epochs 3
  python training.py --stage grpo --grpo-base outputs/rfs_qwen38/sft_lora
  python training.py --dry-run                     # validate data + reward self-test, no GPU

Hardware notes (from the official notebooks):
  * QLoRA in 4-bit needs two >=14 GB GPUs (Kaggle T4 x2) or one >=40 GB GPU;
    do NOT pass device_map - Unsloth's "sequential" default fills the cards.
  * Do not set fp16/bf16: this architecture trains on a float32 autocast path.
  * GRPO loads with text_only=True and fast_inference=False (validated path).

Requirements: unsloth, trl (GRPOConfig/GRPOTrainer), datasets, transformers v5,
bitsandbytes. See run_sft()/run_grpo() for exact call signatures.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import zlib
from pathlib import Path

THINK_OPEN = "< think>"
THINK_CLOSE = "< /think>"
DEFAULT_MODEL = "unsloth/Qwen3.8-27B-unsloth-bnb-4bit"
REASONING_EFFORT = "low"  # low | medium | xhigh (Qwen3.8 chat-template kwarg)

# ---------------------------------------------------------------------------
# Pure helpers (no torch/unsloth imports): data loading, channel splitting,
# RFS structural checks, similarity. These are exercised by --dry-run.
# ---------------------------------------------------------------------------


def load_training_rows(path: Path) -> list[dict]:
    """Read and validate all_rfs_training.jsonl (mirrors Rust validate_training_line)."""
    rows: list[dict] = []
    errors: list[str] = []
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as e:
                errors.append(f"line {lineno}: invalid JSON ({e})")
                continue
            messages = value.get("messages")
            if not isinstance(messages, list) or len(messages) != 2:
                errors.append(f"line {lineno}: expected exactly 2 messages")
                continue
            user, assistant = messages[0], messages[1]
            if user.get("role") != "user" or assistant.get("role") != "assistant":
                errors.append(f"line {lineno}: roles must be user then assistant")
                continue
            u_content = user.get("content")
            a_content = assistant.get("content")
            if not isinstance(u_content, str) or not u_content.strip():
                errors.append(f"line {lineno}: empty user content")
                continue
            if (
                not isinstance(a_content, str)
                or not a_content.startswith(THINK_OPEN)
                or THINK_CLOSE not in a_content
            ):
                errors.append(
                    f"line {lineno}: assistant must use '{THINK_OPEN} ... {THINK_CLOSE}' format"
                )
                continue
            rows.append({"messages": messages})
    if errors:
        for e in errors[:10]:
            print(f"data error: {e}", file=sys.stderr)
        raise SystemExit(f"{len(errors)} invalid row(s) in {path}; aborting before GPU work")
    return rows


def split_channels(text: str | None) -> tuple[str, str]:
    """Return (reasoning, answer); the visible answer follows the last < /think>."""
    if text is None:
        return "", ""
    idx = text.rfind(THINK_CLOSE)
    if idx == -1:
        # The model never closed its reasoning channel: no final answer exists.
        return text.replace(THINK_OPEN, " ").strip(), ""
    reasoning = text[:idx].replace(THINK_OPEN, " ").strip()
    answer = text[idx + len(THINK_CLOSE):]
    for marker in ("< |im_start|>", "< |im_end|>", "< |endoftext|>"):
        answer = answer.split(marker)[0]
    return reasoning.strip(), answer.strip()


def extract_rfs_blocks(text: str) -> list[str]:
    """Extract fenced ```rfs blocks (same scanning rules as the Rust snippet extractor)."""
    blocks: list[str] = []
    rest = text
    while True:
        start = rest.find("```rfs")
        if start == -1:
            break
        after = rest[start + len("```rfs"):].lstrip("\r\n")
        end = after.find("```")
        if end == -1:
            break
        snippet = after[:end].strip()
        if snippet:
            blocks.append(snippet)
        rest = after[end + len("```"):]
    return blocks


def rfs_structure_score(snippet: str) -> float:
    """Heuristic structural validity of an RFS snippet, in [0, 1].

    1.0 = balanced braces + module header + at least two core section keywords;
    0.5 = balanced braces + module header only; 0.0 = malformed.
    """
    if snippet.count("{") != snippet.count("}"):
        return 0.0
    if not re.search(r"\bmodule\s+\w+", snippet):
        return 0.0
    markers = sum(
        re.search(pattern, snippet) is not None
        for pattern in (r"\benum\b", r"\binitial\b", r"\bevent\b", r"\btransition\b")
    )
    return 1.0 if markers >= 2 else 0.5


def bigram_jaccard(a: str, b: str) -> float:
    """Jaccard similarity over word bigrams (lower-cased whitespace tokens)."""

    def ngrams(text: str) -> set[str]:
        words = re.findall(r"[a-z0-9_]+", text.lower())
        if len(words) < 2:
            return set(words)
        # The second sequence is intentionally one shorter (word bigrams).
        return {f"{x} {y}" for x, y in zip(words, words[1:], strict=False)}

    sa, sb = ngrams(a), ngrams(b)
    if not sa or not sb:
        return 0.0
    return len(sa & sb) / len(sa | sb)


def _completion_text(completion) -> str:
    """Robustly extract completion text (TRL passes str or multimodal content lists)."""
    if not isinstance(completion, list):
        return completion if isinstance(completion, str) else str(completion)
    if not completion:
        return ""
    content = completion[0].get("content", "")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "".join(
            item.get("text", "")
            for item in content
            if isinstance(item, dict) and item.get("type") == "text"
        )
    return str(content)


# ---------------------------------------------------------------------------
# GRPO reward functions. TRL calls each with (prompts, completions, **columns);
# every function returns one float score per completion.
# ---------------------------------------------------------------------------


def think_channel(completions, **kwargs) -> list[float]:
    """Reward a closed < think> ... < /think> channel with a non-empty visible answer."""
    scores = []
    for completion in completions:
        text = _completion_text(completion)
        reasoning, answer = split_channels(text)
        if THINK_CLOSE not in text:
            scores.append(-1.0)  # never closed the reasoning channel
        elif not answer:
            scores.append(0.0)   # closed, but no visible answer (truncation)
        else:
            scores.append(1.5 if reasoning else 1.0)
    return scores


def rfs_structure(prompts, completions, **kwargs) -> list[float]:
    """Reward well-formed ```rfs snippets; neutral (0.0) when none are present."""
    scores = []
    for completion in completions:
        blocks = extract_rfs_blocks(_completion_text(completion))
        scores.append(max((rfs_structure_score(b) for b in blocks), default=0.0))
    return scores


def reference_similarity(prompts, completions, reference, **kwargs) -> list[float]:
    """Reward overlap between the visible answer and the gold assistant answer."""
    scores = []
    for completion, gold in zip(completions, reference, strict=True):
        text = _completion_text(completion)
        _, answer = split_channels(text)
        scores.append(bigram_jaccard(answer or text, gold))
    return scores


# ---------------------------------------------------------------------------
# Dataset conversion (needs a tokenizer; imported lazily by the run_* stages)
# ---------------------------------------------------------------------------


def prepare_sft_dataset(rows: list[dict], tokenizer):
    """Convert rows to an HF dataset with a `text` column (Qwen chat template)."""
    from datasets import Dataset

    def render(batch_messages: list[list[dict]]) -> dict:
        return {
            "text": [
                tokenizer.apply_chat_template(
                    messages, tokenize=False, add_generation_prompt=False
                )
                for messages in batch_messages
            ]
        }

    dataset = Dataset.from_list([{"messages": r["messages"]} for r in rows])
    return dataset.map(render, batched=True)


def _rendered_prompt_tokens(messages: list[dict], tokenizer) -> int:
    """Token count of the rendered generation prompt (incl. think scaffolding)."""
    text = tokenizer.apply_chat_template(
        messages,
        tokenize=False,
        add_generation_prompt=True,
        reasoning_effort=REASONING_EFFORT,
    )
    ids = tokenizer(text=text, add_special_tokens=False)["input_ids"]
    if len(ids) and isinstance(ids[0], (list, tuple)):
        ids = ids[0]
    return len(ids)


def prepare_grpo_dataset(rows: list[dict], tokenizer, max_prompt_tokens: int):
    """Convert rows to prompt/reference columns; drop prompts that are too long."""
    from datasets import Dataset

    kept: list[dict] = []
    dropped = 0
    for row in rows:
        prompt = [{"role": "user", "content": row["messages"][0]["content"]}]
        if _rendered_prompt_tokens(prompt, tokenizer) > max_prompt_tokens:
            dropped += 1
            continue
        kept.append(
            {
                "prompt": prompt,
                "reference": row["messages"][1]["content"],
            }
        )
    if not kept:
        raise RuntimeError(
            f"no GRPO rows left after dropping prompts > {max_prompt_tokens} tokens; "
            "raise --grpo-max-prompt-tokens or --max-seq-length"
        )
    print(
        f"GRPO dataset: {len(kept)} rows kept, {dropped} dropped "
        f"(prompt budget {max_prompt_tokens})"
    )
    return Dataset.from_list(kept)


def deterministic_split(rows: list[dict], eval_ratio: float) -> tuple[list[dict], list[dict]]:
    """Deterministic train/eval split by CRC32 of the user prompt (stable across runs)."""
    if eval_ratio <= 0 or len(rows) < 10:
        return rows, []
    n_eval = max(1, int(len(rows) * eval_ratio))

    def key(row: dict) -> int:
        return zlib.crc32(row["messages"][0]["content"].encode("utf-8"))

    ordered = sorted(rows, key=key)
    return ordered[n_eval:], ordered[:n_eval]


def self_test_rewards() -> None:
    """CPU-only sanity checks for the reward functions (mirrors the GRPO notebook)."""
    good = "step by step reasoning." + THINK_CLOSE + "\nFinal answer text here."
    unclosed = "I am still thinking about it and never closed the channel."
    snippet_good = (
        "module m {\n"
        "  enum State { A }\n"
        "  initial A\n"
        "  event Go\n"
        "  transition A on Go { next A; }\n"
        "}\n"
    )
    snippet_bad = "module m {\n  enum State { A\n"  # unbalanced braces
    rfs_good = "reasoning." + THINK_CLOSE + "\n```rfs\n" + snippet_good + "```\ndone"
    rfs_bad = "reasoning." + THINK_CLOSE + "\n```rfs\n" + snippet_bad + "```\ndone"

    assert think_channel([good]) == [1.5], "closed channel with answer must score 1.5"
    assert think_channel([unclosed]) == [-1.0], "unclosed channel must score -1.0"
    assert think_channel([THINK_CLOSE]) == [0.0], "closed without answer must score 0.0"

    assert rfs_structure([None], [rfs_good]) == [1.0], "well-formed snippet must score 1.0"
    assert rfs_structure([None], [rfs_bad]) == [0.0], "unbalanced braces must score 0.0"
    assert rfs_structure([None], ["no code at all"]) == [0.0], "no snippet must be neutral"

    sim = reference_similarity([None], ["answer alpha beta"], ["gold alpha beta gamma"])
    assert 0.0 < sim[0] <= 1.0, f"similarity out of range: {sim}"
    assert bigram_jaccard("a b c", "a b c") == 1.0, "identical texts must score 1.0"
    print("reward self-test: OK")


# ---------------------------------------------------------------------------
# Stage 1: SFT (QLoRA) - official Qwen3.8 conversational recipe
# ---------------------------------------------------------------------------


def run_sft(args: argparse.Namespace, rows: list[dict]) -> Path:
    from trl import SFTConfig, SFTTrainer
    from unsloth import FastModel
    from unsloth.chat_templates import train_on_responses_only

    model, tokenizer = FastModel.from_pretrained(
        model_name=args.model,
        max_seq_length=args.max_seq_length,
        load_in_4bit=True,
        full_finetuning=False,
        offload_embedding=args.offload_embedding,
    )
    model = FastModel.get_peft_model(
        model,
        finetune_vision_layers=False,      # text-only RFS data
        finetune_language_layers=True,
        finetune_attention_modules=True,   # reaches the gated-delta linear-attn modules
        finetune_mlp_modules=True,
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        lora_dropout=0,
        bias="none",
        use_gradient_checkpointing="unsloth",
        random_state=args.seed,
    )

    train_rows, eval_rows = deterministic_split(rows, args.eval_ratio)
    extra = f", {len(eval_rows)} eval rows" if eval_rows else ""
    print(f"SFT dataset: {len(train_rows)} train rows{extra}")
    train_dataset = prepare_sft_dataset(train_rows, tokenizer)
    eval_dataset = prepare_sft_dataset(eval_rows, tokenizer) if eval_rows else None

    sft_args = SFTConfig(
        dataset_text_field="text",
        max_seq_length=args.max_seq_length,
        per_device_train_batch_size=1,  # 248k-token vocab: keep batch 1, use grad accum
        gradient_accumulation_steps=args.grad_accum,
        warmup_steps=10,
        learning_rate=args.learning_rate,
        logging_steps=1,
        optim="adamw_8bit",
        weight_decay=0.001,
        lr_scheduler_type="linear",
        seed=args.seed,
        output_dir=str(Path(args.output_dir) / "sft"),
        report_to="none",
    )
    if args.sft_max_steps is not None:
        sft_args.max_steps = args.sft_max_steps
    else:
        sft_args.num_train_epochs = args.sft_epochs

    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=train_dataset,
        eval_dataset=eval_dataset,
        args=sft_args,
    )
    # Train only on assistant responses; mask the user prompt portion.
    trainer = train_on_responses_only(trainer)
    trainer.train()

    adapter_dir = Path(args.output_dir) / "sft_lora"
    model.save_pretrained(adapter_dir)
    tokenizer.save_pretrained(adapter_dir)
    print(f"SFT LoRA adapter saved to {adapter_dir}")
    return adapter_dir


# ---------------------------------------------------------------------------
# Stage 2: GRPO - official Qwen3.8 RL path (text_only, no fast vLLM inference)
# ---------------------------------------------------------------------------


def run_grpo(args: argparse.Namespace, rows: list[dict], base_model: str) -> Path:
    from trl import GRPOConfig, GRPOTrainer
    from unsloth import FastModel

    model, tokenizer = FastModel.from_pretrained(
        model_name=base_model,
        max_seq_length=args.max_seq_length,
        load_in_4bit=True,
        fast_inference=False,  # Qwen3.8 RL docs: keep the validated generation path
        text_only=True,        # skip vision/audio towers (text-only task)
    )
    model.config.architectures = model.config.architectures or ["Qwen3_5ForCausalLM"]
    model.generation_config.max_length = None

    dataset = prepare_grpo_dataset(rows, tokenizer, args.grpo_max_prompt_tokens)

    model = FastModel.get_peft_model(
        model,
        finetune_vision_layers=False,
        finetune_language_layers=True,
        finetune_attention_modules=True,
        finetune_mlp_modules=True,
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        lora_dropout=0,
        bias="none",
        use_gradient_checkpointing="unsloth",
        random_state=args.seed,
    )

    grpo_args = GRPOConfig(
        temperature=1.0,
        learning_rate=args.grpo_learning_rate,
        adam_beta1=0.9,
        adam_beta2=0.99,
        weight_decay=0.1,
        warmup_steps=10,
        lr_scheduler_type="cosine",
        optim="adamw_8bit",
        max_grad_norm=0.1,
        logging_steps=1,
        log_completions=args.log_completions,
        per_device_train_batch_size=args.num_generations,  # batch == group size
        gradient_accumulation_steps=1,
        num_generations=args.num_generations,
        chat_template_kwargs={"reasoning_effort": REASONING_EFFORT},
        max_completion_length=args.grpo_max_completion_length,
        mask_truncated_completions=False,  # keep capped completions so they get penalized
        max_steps=args.grpo_max_steps,
        save_steps=args.grpo_save_steps,
        report_to="none",
        output_dir=str(Path(args.output_dir) / "grpo"),
        importance_sampling_level="sequence",
        loss_type="grpo",
    )

    trainer = GRPOTrainer(
        model=model,
        processing_class=tokenizer,
        reward_funcs=[think_channel, rfs_structure, reference_similarity],
        args=grpo_args,
        train_dataset=dataset,
    )
    trainer.train()

    adapter_dir = Path(args.output_dir) / "grpo_lora"
    model.save_pretrained(adapter_dir)
    tokenizer.save_pretrained(adapter_dir)
    print(f"GRPO LoRA adapter saved to {adapter_dir}")
    return adapter_dir


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--stage", choices=("sft", "grpo", "both"), default="both")
    parser.add_argument("--dry-run", action="store_true",
                        help="validate the dataset and self-test rewards; no GPU work")
    parser.add_argument("--data", type=Path, default=None,
                        help="path to all_rfs_training.jsonl (default: next to this script)")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--output-dir", type=Path, default=Path("outputs/rfs_qwen38"))
    parser.add_argument("--max-seq-length", type=int, default=2048)
    parser.add_argument("--seed", type=int, default=3407)
    # SFT
    parser.add_argument("--sft-epochs", type=float, default=3.0)
    parser.add_argument("--sft-max-steps", type=int, default=None)
    parser.add_argument("--grad-accum", type=int, default=4)
    parser.add_argument("--learning-rate", type=float, default=2e-4)
    parser.add_argument("--eval-ratio", type=float, default=0.1)
    parser.add_argument("--offload-embedding", action="store_true", default=True)
    # LoRA (both stages)
    parser.add_argument("--lora-r", type=int, default=16)
    parser.add_argument("--lora-alpha", type=int, default=16)
    # GRPO
    parser.add_argument("--grpo-base", default="auto",
                        help="base for GRPO: 'auto' = SFT adapter from this run when "
                             "--stage includes sft, else --model; or an explicit adapter dir")
    parser.add_argument("--num-generations", type=int, default=2)
    parser.add_argument("--grpo-max-prompt-tokens", type=int, default=1024)
    parser.add_argument("--grpo-max-completion-length", type=int, default=1024)
    parser.add_argument("--grpo-max-steps", type=int, default=100)
    parser.add_argument("--grpo-save-steps", type=int, default=50)
    parser.add_argument("--grpo-learning-rate", type=float, default=5e-6)
    parser.add_argument("--log-completions", action="store_true")
    return parser.parse_args(argv)


def main() -> None:
    args = parse_args()
    data_path = args.data or (Path(__file__).resolve().parent / "all_rfs_training.jsonl")
    rows = load_training_rows(data_path)
    total_chars = sum(
        len(m["content"]) for row in rows for m in row["messages"]
    )
    print(f"loaded {len(rows)} training rows from {data_path} ({total_chars} chars)")

    if args.dry_run:
        self_test_rewards()
        train_rows, eval_rows = deterministic_split(rows, args.eval_ratio)
        print(f"deterministic split: {len(train_rows)} train / {len(eval_rows)} eval")
        print("dry-run complete; no GPU work performed")
        return

    sft_dir = run_sft(args, rows) if args.stage in ("sft", "both") else None

    if args.stage in ("grpo", "both"):
        if args.grpo_base == "auto":
            base = str(sft_dir) if sft_dir is not None else args.model
        else:
            base = args.grpo_base
        run_grpo(args, rows, base)


if __name__ == "__main__":
    main()
