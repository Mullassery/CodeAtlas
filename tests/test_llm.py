import codeatlas.llm as llm


def test_resolve_config_openai(monkeypatch):
    monkeypatch.setenv("OPENAI_API_KEY", "sk-test")
    monkeypatch.delenv("OPENAI_BASE_URL", raising=False)
    monkeypatch.delenv("OPENROUTER_API_KEY", raising=False)
    monkeypatch.delenv("DEEPSEEK_API_KEY", raising=False)
    monkeypatch.delenv("OLLAMA_BASE_URL", raising=False)

    config = llm.resolve_config()
    assert config is not None
    assert config.provider == "openai"
    assert config.model == "gpt-4o"


def test_resolve_config_none(monkeypatch):
    for var in ("OPENAI_API_KEY", "OPENROUTER_API_KEY", "DEEPSEEK_API_KEY", "OLLAMA_BASE_URL"):
        monkeypatch.delenv(var, raising=False)
    assert llm.resolve_config() is None


def test_enrich_renames_components_and_relations(monkeypatch):
    def fake_chat(config, messages, **kwargs):
        return (
            '{"components":['
            '{"name":"App Runner","description":"Runs the app"},'
            '{"name":"Core Engine","description":"Core logic"}'
            "]}"
        )

    monkeypatch.setattr(llm, "chat_completion", fake_chat)

    analysis = {
        "components": [
            {"name": "app", "description": "Owns 1 symbols."},
            {"name": "core", "description": "Owns 1 symbols."},
        ],
        "components_relations": [{"src_name": "core", "dst_name": "app"}],
    }

    config = llm.LLMConfig("openai", "https://api.openai.com/v1", "sk-test", "gpt-4o")
    llm.enrich(analysis, config=config)

    assert analysis["components"][0]["name"] == "App Runner"
    assert analysis["components"][0]["description"] == "Runs the app"
    assert analysis["components_relations"][0]["src_name"] == "Core Engine"
    assert analysis["components_relations"][0]["dst_name"] == "App Runner"


def test_enrich_noop_without_config():
    analysis = {"components": [{"name": "app"}]}
    llm.enrich(analysis, config=None)
    assert analysis["components"][0]["name"] == "app"


def test_llm_error_mapping():
    assert llm._llm_error_for_status(401, b"{}").kind == "auth"
    assert llm._llm_error_for_status(403, b"{}").kind == "auth"
    assert llm._llm_error_for_status(402, b"{}").kind == "quota"
    assert llm._llm_error_for_status(429, b"{}").kind == "quota"
    assert llm._llm_error_for_status(500, b"{}").kind == "llm"
