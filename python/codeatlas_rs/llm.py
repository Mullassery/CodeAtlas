"""Optional LLM semantic enrichment.

When a provider is configured (via environment variables), this module names and
describes the top-level components using a single structured completion. When no
provider is configured, analysis proceeds with deterministic names (ADR-0006).

v1 supports OpenAI-compatible chat-completions endpoints (OpenAI, Ollama,
OpenRouter, DeepSeek, LiteLLM, LM Studio, etc.). Other provider wire protocols
are deferred.
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any

EXIT_AUTH_ERROR = 2
EXIT_QUOTA_EXHAUSTED = 3


class LLMError(Exception):
    def __init__(self, message: str, kind: str = "llm") -> None:
        self.kind = kind  # "auth" | "quota" | "llm"
        super().__init__(message)


@dataclass
class LLMConfig:
    provider: str
    base_url: str
    api_key: str | None
    model: str


def resolve_config(env: dict[str, str] | None = None) -> LLMConfig | None:
    env = os.environ if env is None else env
    model = env.get("AGENT_MODEL")

    if env.get("OPENAI_API_KEY"):
        return LLMConfig(
            "openai",
            env.get("OPENAI_BASE_URL", "https://api.openai.com/v1"),
            env["OPENAI_API_KEY"],
            model or "gpt-4o",
        )
    if env.get("OPENROUTER_API_KEY"):
        return LLMConfig(
            "openrouter",
            "https://openrouter.ai/api/v1",
            env["OPENROUTER_API_KEY"],
            model or "google/gemini-3.8-flash",
        )
    if env.get("DEEPSEEK_API_KEY"):
        return LLMConfig(
            "deepseek",
            "https://api.deepseek.com/v1",
            env["DEEPSEEK_API_KEY"],
            model or "deepseek-chat",
        )
    if env.get("OLLAMA_BASE_URL"):
        return LLMConfig(
            "ollama", env["OLLAMA_BASE_URL"], None, model or "qwen3:30b"
        )
    return None


def _chat_url(config: LLMConfig) -> str:
    base = config.base_url.rstrip("/")
    if config.provider == "ollama":
        return f"{base}/v1/chat/completions"
    return f"{base}/chat/completions"


def chat_completion(
    config: LLMConfig,
    messages: list[dict[str, str]],
    *,
    post: Any = None,
) -> str:
    """Send a chat completion and return the assistant message text."""
    body = {
        "model": config.model,
        "messages": messages,
        "response_format": {"type": "json_object"},
        "temperature": 0,
    }
    headers = {"Content-Type": "application/json"}
    if config.api_key:
        headers["Authorization"] = f"Bearer {config.api_key}"

    request = urllib.request.Request(
        _chat_url(config),
        data=json.dumps(body).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    try:
        if post is not None:
            status, payload = post(request)
        else:
            with urllib.request.urlopen(request, timeout=300) as response:
                status, payload = response.status, response.read()
    except urllib.error.HTTPError as exc:
        raise _llm_error_for_status(exc.code, exc.read()) from exc
    except urllib.error.URLError as exc:
        raise LLMError(f"LLM request failed: {exc}") from exc

    if status >= 400:
        raise _llm_error_for_status(status, payload)

    data = json.loads(payload)
    return data["choices"][0]["message"]["content"]


def _llm_error_for_status(status: int, payload: bytes) -> LLMError:
    if status in (401, 403):
        return LLMError("LLM provider rejected the API key.", kind="auth")
    if status == 402 or status == 429:
        return LLMError("LLM provider token or credit quota is exhausted.", kind="quota")
    return LLMError(f"LLM provider error (HTTP {status}): {payload[:200]!r}")


def _parse_json(content: str) -> dict[str, Any]:
    text = content.strip()
    if text.startswith("```"):
        text = text.strip("`")
        if text.startswith("json"):
            text = text[4:]
    start, end = text.find("{"), text.rfind("}")
    if start == -1 or end == -1:
        raise LLMError("LLM returned no JSON object.")
    return json.loads(text[start : end + 1])


def enrich(analysis: dict[str, Any], config: LLMConfig | None = None) -> dict[str, Any]:
    """Rename and describe top-level components in place, if a provider is set."""
    config = resolve_config() if config is None else config
    components = analysis.get("components", [])
    if config is None or not components:
        return analysis

    names = [c.get("name", "") for c in components]
    messages = [
        {
            "role": "system",
            "content": (
                "You name and describe software architecture components. "
                "Return a single JSON object."
            ),
        },
        {
            "role": "user",
            "content": (
                "Given these component folder names, propose concise, descriptive names and "
                "one-sentence descriptions, in the same order. "
                f"Return JSON: {{\"components\":[{{\"name\":\"...\",\"description\":\"...\"}}]}}. "
                f"Input: {json.dumps(names)}"
            ),
        },
    ]

    content = chat_completion(config, messages)
    try:
        result = _parse_json(content)
        proposed = result.get("components", [])
    except (LLMError, ValueError):
        return analysis

    old_to_new: dict[str, str] = {}
    for component, proposal in zip(components, proposed):
        new_name = proposal.get("name") if isinstance(proposal, dict) else None
        new_desc = proposal.get("description") if isinstance(proposal, dict) else None
        if isinstance(new_name, str) and new_name.strip():
            old_to_new[component.get("name", "")] = new_name.strip()
            component["name"] = new_name.strip()
        if isinstance(new_desc, str) and new_desc.strip():
            component["description"] = new_desc.strip()

    for relation in analysis.get("components_relations", []):
        src = relation.get("src_name")
        if src in old_to_new:
            relation["src_name"] = old_to_new[src]
        dst = relation.get("dst_name")
        if dst in old_to_new:
            relation["dst_name"] = old_to_new[dst]

    return analysis
