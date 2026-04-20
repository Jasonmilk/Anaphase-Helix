## Description
<!-- Describe the changes introduced by this PR -->

## Type of change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Checklist
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] AI Coder Self-Check passed (if applicable)
  - [ ] CLI command is `def`, not `async def`
  - [ ] `asyncio.run()` used for async calls
  - [ ] `generate_trace_id()` + `set_trace_id()` called at start
  - [ ] Command start/complete logs present
  - [ ] No magic strings
  - [ ] Errors logged with `logger.error`
  - [ ] Dependencies updated in `pyproject.toml`
