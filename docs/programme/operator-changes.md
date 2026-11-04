# Operator changes applied by the programme orchestrator (R-025)

Each row: time, system, change, how it was checked, how to undo it.

| Time | System | Change | Check | Undo |
| --- | --- | --- | --- | --- |
| 2026-11-04 09:18Z | local Mac | Created SSH key `~/.ssh/anima_ops_ed25519` (no passphrase, comment `anima-ops-20261104`, SHA256:E9HW7Ow77gBDPQ9EokBqi0yiSUCRkglYXcQaxiEwQ5c) for this run | `ssh-keygen -lf` | `rm ~/.ssh/anima_ops_ed25519*` |
| 2026-11-04 09:19Z | Hostinger API | Registered that public key (id 618466) and asked to attach it to VPS 1656033; the API returned action id 0 and SSH still refuses it, so it is not on the server | `ssh -i … root@31.220.53.226` denied | `hostinger vps public-keys delete 618466` |
