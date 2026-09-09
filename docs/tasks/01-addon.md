# Task 01 — Phase 1 addon

## Exit criteria
- [x] TOC valid (Interface 120100, SV declared, optional deps only)
- [x] Core/Store/Session/GatherMate/Routes/Navigation/Bridge/UI implemented
- [x] Event-driven; no global OnUpdate (1 Hz refresh only while panel visible)
- [x] Versioned SavedVariables + migration + pruning caps
- [x] GO1 import codec with checksum (roundtrip + tamper tests)
- [x] Optional adapters degrade gracefully (proven by tests)
- [x] `scripts/check-lua.sh` PASS (8 files, Lua 5.1 dialect)
- [x] `tests/lua/run_tests.lua` 27/27 PASS
- [x] `fixtures/savedvariables_sample.lua` canonical sample
