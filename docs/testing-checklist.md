# Testing checklist before release

All are to be done inside developer VM with 100MB files and 1GB USBs.

```bash
scripts/gen_test_files.sh 100000000 test.img
```

We should turn these into E2E tests eventually but for now this has to be done manually.

## Happy path

- [ ] Write normal file, with hash validation and invalidation
- [ ] Write compressed file, with hash validation and invalidation

## Sad path

- [ ] Cancel an operation during setup wizard
- [ ] Cancel an operation right before confirmation
- [ ] Write to a read-only file
