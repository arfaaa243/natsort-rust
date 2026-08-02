# Original test suite — not yet populated

This directory is meant to hold the **unmodified** `natsort` test files
(`test_natsorted.py`, `test_natsorted_convenience.py`) fetched from
https://github.com/SethMMorton/natsort, pinned to the commit used at
kickoff, alongside their SHA-256 hashes in `original_tests.sha256`.

They are not here yet because this pass had no network access to fetch
them. Before submission:

```bash
# from repo root
mkdir -p tests/original
curl -L -o tests/original/test_natsorted.py \
  https://raw.githubusercontent.com/SethMMorton/natsort/main/tests/test_natsorted.py
curl -L -o tests/original/test_natsorted_convenience.py \
  https://raw.githubusercontent.com/SethMMorton/natsort/main/tests/test_natsorted_convenience.py
sha256sum tests/original/*.py > tests/original/original_tests.sha256
```

Do not hand-edit the fetched files. If a test fails against the port and
can't reasonably be made to pass in the time remaining, leave it failing
and document why in ../DECISIONS.md — do not delete or alter the assertion
to make it green.
