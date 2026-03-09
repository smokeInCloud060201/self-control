---
description: How to conduct a code review before committing changes.
---

# Code Review Workflow

Follow these steps before committing any code to ensure it meets the project's standards for quality, architecture, and security.

1. **Self-Review**: Compare your changes against the `review.md` skill and the overall project `architecture.md` and `coding.md` standards.
2. **Context Check**: Ensure that the changes align with the Domain-Driven Design (DDD) principles defined for the modularized kiosk-machine.
3. **Security Audit**: Verify that no sensitive information (e.g., NRIC, passwords, keys) is being logged. Use the `CommonLogger` masking capabilities if necessary.
4. **Test Verification**: Run `bun test` and ensure all tests pass. If you've added new features, verify that unit and integration tests are included as per `testing.md`.
5. **Lint and Format**: Ensure the code is properly formatted and passes all linting rules.
6. **Final Sign-off**: Once all steps are completed and the code meets the Principal SE / Architect standards, you can proceed to create a commit.

// turbo
7. **Commit**: Use the `commits.md` skill to format your commit message correctly.
