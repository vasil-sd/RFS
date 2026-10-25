> For the complete documentation index, see [llms.txt](https://unsloth.ai/docs/llms.txt). Markdown versions of documentation pages are available by appending `.md` to page URLs; this page is available as [Markdown](https://unsloth.ai/docs/models/qwen3.8/train.md).

# Qwen3.8 Fine-tuning Guide

Learn how to fine-tune Qwen3.8-27B with Unsloth.

Qwen3.8-27B can now be fine-tuned and trained with reinforcement learning (RL) via [Unsloth](https://github.com/unslothai/unsloth). It is a dense 27B unified vision-language model with native text, image and video support, thinking controls and a 262K context window.

* Unsloth trains Qwen3.8 **\~1.5x faster** with **\~50% less VRAM** than FA2 setups (no accuracy loss)
* Fine-tune Qwen3.8-27B via our **free** **Kaggle notebooks**:

| [**Conversational**](https://www.kaggle.com/notebooks/welcome?src=https://github.com/unslothai/notebooks/blob/main/nb/Kaggle-Qwen3.8_\(27B\)-Conversational.ipynb\&accelerator=nvidiaTeslaT4) (you can enable Vision) | [**RL GRPO**](https://www.kaggle.com/notebooks/welcome?src=https://github.com/unslothai/notebooks/blob/main/nb/Kaggle-Muse_Glimmer_\(30B\)-GRPO.ipynb\&accelerator=nvidiaTeslaT4) |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |

* **QLoRA works with 24GB** and LoRA needs >36GB
* You can also train Qwen3.8 with [reinforcement learning](#reinforcement-learning-rl) (RL) free or on 24GB VRAM.
* **Full fine-tuning (FFT)** works as well, but it will use 4x more VRAM.
* Unsloth utilizes **Flash Linear Attention kernels** for efficient Qwen3.8 training
* Use [`Qwen3.8-27B-unsloth-bnb-4bit`](https://huggingface.co/unsloth/Qwen3.8-27B-unsloth-bnb-4bit) for 4-bit QLoRA, then **export** to NVFP4, FP8, GGUF etc.
* If you want to preserve reasoning ability, mix reasoning-style examples with direct answers and keep at least 75% reasoning data.

Please use the latest Transformers v5. Qwen3.8 uses the `qwen3_5` architecture. The first run can take longer while the Gated DeltaNet kernels compile.

{% hint style="success" %}
For free Qwen3.8-27B fine-tuning via [Unsloth](https://github.com/unslothai/unsloth), we provide multiple Kaggle notebooks that offer **30 hours of free GPU usage with 2× Tesla T4 GPUs**. Kaggle is a Google product, similar to Google Colab, and provides a convenient way to run fine-tuning workflows without needing your own GPU hardware.
{% endhint %}

### 🦥 Unsloth Guide

{% columns %}
{% column %}
Qwen3.8 can be run and fine-tuned in [Unsloth](/docs/new/studio.md) Desktop, our new open-source web UI for local AI.

With Unsloth Studio, you can run models locally on **MacOS, Windows**, Linux and train NVIDIA GPUs. Intel, MLX and AMD training support coming this month.
{% endcolumn %}

{% column %}

<div data-with-frame="true"><figure><img src="https://3215535692-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FxhOjnexMCB3dmuQFQ2Zq%2Fuploads%2FD7qpbGJLY5YMv0E0QgrB%2Ftrainingrun_LLM-unsloth.gif?alt=media&amp;token=feb766a7-1a7b-4f47-b5f4-b38e3a0d1470" alt=""><figcaption></figcaption></figure></div>
{% endcolumn %}
{% endcolumns %}

{% stepper %}
{% step %}

#### Install Unsloth

The easiest way to get started is by downloading the [Unsloth Desktop app](/docs/desktop.md). Works on [macOS](/docs/get-started/install/mac.md), [Windows](/docs/get-started/install/windows-installation.md), and [Linux](/docs/get-started/install/linux.md).

<a href="https://unsloth.ai/download" class="button primary" data-icon="down-to-bracket">Download Unsloth</a>

* <i class="fa-apple">:apple:</i> [Download for macOS](https://unsloth.ai/download/mac)
* <i class="fa-windows">:windows:</i> [Download for Windows](https://unsloth.ai/download/windows)
* <i class="fa-linux">:linux:</i> [Download for Linux](https://unsloth.ai/download/linux)

Or, if you prefer to install manually:

MacOS, Linux, WSL:

```bash
curl -fsSL https://unsloth.ai/install.sh | sh
```

Windows PowerShell:

```bash
irm https://unsloth.ai/install.ps1 | iex
```

{% endstep %}

{% step %}

#### Train Qwen3.8

Go to the Train tab then search for Qwen3.8-27B in the search bar and select your desired model and dataset. Next, adjust your hyperparameters, context length as desired.

<figure><img src="https://3215535692-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2FxhOjnexMCB3dmuQFQ2Zq%2Fuploads%2FD7qpbGJLY5YMv0E0QgrB%2Ftrainingrun_LLM-unsloth.gif?alt=media&amp;token=feb766a7-1a7b-4f47-b5f4-b38e3a0d1470" alt=""><figcaption></figcaption></figure>
{% endstep %}

{% step %}

#### Monitor training progress

After you click start training, you will be able to monitor and observe the training progress of the model. The training loss should be steadily decreasing.\
Once done, the model will be automatically saved.
{% endstep %}

{% step %}

#### Export your fine-tuned model

Once done, Unsloth Studio allows you to export the model to GGUF, safetensor etc formats.
{% endstep %}

{% step %}

#### Compare fine-tuned model vs original model

Click on `Compare Mode` to compare the LoRA adapter and the original model.
{% endstep %}
{% endstepper %}

### SFT recipe

Below is a minimal SFT recipe for text-only fine-tuning. Your dataset needs a `text` column already rendered with the Qwen chat template.

```python
from unsloth import FastModel
from datasets import load_dataset
from trl import SFTTrainer, SFTConfig

max_seq_length = 2048

dataset = load_dataset(
    "json",
    data_files = "train.jsonl",
    split = "train",
)

model, tokenizer = FastModel.from_pretrained(
    model_name = "unsloth/Qwen3.8-27B-unsloth-bnb-4bit",
    max_seq_length = max_seq_length,
    load_in_4bit = True,
    full_finetuning = False,
    offload_embedding = True,
)

model = FastModel.get_peft_model(
    model,
    finetune_vision_layers     = False,
    finetune_language_layers   = True,
    finetune_attention_modules = True,
    finetune_mlp_modules       = True,

    r = 16,
    lora_alpha = 16,
    lora_dropout = 0,
    bias = "none",
    use_gradient_checkpointing = "unsloth",
    random_state = 3407,
    use_rslora = False,
    loftq_config = None,
)

trainer = SFTTrainer(
    model = model,
    tokenizer = tokenizer,
    train_dataset = dataset,
    args = SFTConfig(
        dataset_text_field = "text",
        max_seq_length = max_seq_length,
        per_device_train_batch_size = 1,
        gradient_accumulation_steps = 4,
        warmup_steps = 10,
        max_steps = 100,
        learning_rate = 2e-4,
        logging_steps = 1,
        optim = "adamw_8bit",
        output_dir = "outputs_qwen38",
        seed = 3407,
        dataset_num_proc = 1,
        report_to = "none",
    ),
)

trainer.train()
```

`offload_embedding=True` is optional and reduces resident VRAM by keeping the large, untied input embedding in RAM. Unsloth disables it automatically on unsupported platforms.

If you OOM, reduce `max_seq_length` and keep batch size 1 with `"unsloth"` gradient checkpointing.

### Vision fine-tuning

Qwen3.8 supports native image and video inputs. For multimodal training, enable the vision layers and use a conversational vision dataset with `UnslothVisionDataCollator`.

```python
model = FastModel.get_peft_model(
    model,
    finetune_vision_layers     = True,
    finetune_language_layers   = True,
    finetune_attention_modules = True,
    finetune_mlp_modules       = True,

    r = 16,
    lora_alpha = 16,
    lora_dropout = 0,
    bias = "none",
    use_gradient_checkpointing = "unsloth",
    random_state = 3407,
    use_rslora = False,
    loftq_config = None,
)
```

Set `finetune_vision_layers=False` when your dataset is text-only.

### Reinforcement Learning (RL)

Qwen3.8 uses the same `qwen3_5` architecture, so use the Qwen3.5 Unsloth RL path and disable fast vLLM inference:

```python
from unsloth import FastModel

model, tokenizer = FastModel.from_pretrained(
    model_name = "unsloth/Qwen3.8-27B-unsloth-bnb-4bit",
    max_seq_length = 2048,
    load_in_4bit = True,
    fast_inference = False,
)
```

### Saving / export fine-tuned model

Use the same chat template and EOS token during deployment that you used during training.

**Save to GGUF**

```python
model.save_pretrained_gguf(
    "qwen38_gguf",
    tokenizer,
    quantization_method = "q4_k_m",
)

# model.push_to_hub_gguf(
#     "hf_username/qwen38_gguf",
#     tokenizer,
#     quantization_method = "q4_k_m",
# )
```

**Save to vLLM**

```python
model.save_pretrained_merged(
    "qwen38_finetuned",
    tokenizer,
    save_method = "merged_16bit",
)
```

To save only the LoRA adapters:

```python
model.save_pretrained("qwen38_lora")
tokenizer.save_pretrained("qwen38_lora")
```


---

# Agent Instructions
This documentation is published with GitBook. GitBook is the documentation platform designed so that both humans and AI agents can read, navigate, and reason over technical content effectively. Learn more at gitbook.com.

## Querying This Documentation
If you need additional information that is not directly available in this page, you can query the documentation by asking a question.

Perform an HTTP GET request on the following URL with the `ask` and `goal` query parameters:

```
GET https://unsloth.ai/docs/models/qwen3.8/train.md?ask=<question>&goal=<user_goal>
```

`ask` is the immediate question: it should be specific, self-contained, and written in natural language.
`goal` is what the user is ultimately trying to achieve, the reason they need the answer. Sharing it helps GitBook give you a better, more relevant answer. A goal is most helpful when it describes the outcome the user wants rather than restating the question. For example, with `ask=how do I create an API token`, a goal like `automate deployments from our CI pipeline` lets GitBook tailor the answer to that use case.

The response will contain a direct answer to the question and relevant excerpts and sources from the documentation.

Use this mechanism when the answer is not explicitly present in the current page, you need clarification or additional context, or you want to retrieve related documentation sections.
